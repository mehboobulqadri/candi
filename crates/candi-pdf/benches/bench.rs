// SPDX-License-Identifier: AGPL-3.0
// Benchmark harness binary — driven by bench/run.sh, one process per backend/doc.
// Methodology: open_ms before backend open, best-of-2 for page/search/nav,
// process RSS (VmRSS baseline -> VmHWM peak -> delta), nonzero exit on errors.

use std::env;
use std::process;
use std::time::Instant;

use candi_pdf::{BackendKind, Document, Error, open};

mod measure {
    use std::time::Instant;

    pub fn elapsed_ms(start: Instant, end: Instant) -> f64 {
        end.duration_since(start).as_secs_f64() * 1000.0
    }

    fn status_field(name: &str) -> Option<u64> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|l| l.starts_with(name))?;
        line.split_whitespace().nth(1)?.parse().ok()
    }

    pub fn rss_mb() -> u64 {
        status_field("VmRSS:").unwrap_or(0) / 1024
    }

    pub fn peak_rss_mb() -> u64 {
        status_field("VmHWM:").unwrap_or(0) / 1024
    }
}

const SEARCH_QUERY: &str = "the";

const BUDGET_STARTUP_MS: f64 = 300.0;
const BUDGET_OPEN_MS: f64 = 150.0;
const BUDGET_PAGE_MS: f64 = 20.0;
const BUDGET_SEARCH_MS: f64 = 300.0;
const BUDGET_NAV_MS: f64 = 50.0;
const BUDGET_PEAK_RSS_MB: u64 = 200;

#[derive(Clone, Copy, Debug)]
enum ExpectedOpenError {
    Encrypted,
    Malformed,
    NoTextLayer,
}

fn parse_backend(s: &str) -> Result<BackendKind, String> {
    match s {
        "mupdf" => Ok(BackendKind::Mupdf),
        "pdfium" => Ok(BackendKind::Pdfium),
        _ => Err(format!("unknown backend: {s}")),
    }
}

fn expected_open_error(label: &str) -> Option<ExpectedOpenError> {
    match label {
        "dummy-encrypted" => Some(ExpectedOpenError::Encrypted),
        "broken" => Some(ExpectedOpenError::Malformed),
        "image-only" => Some(ExpectedOpenError::NoTextLayer),
        _ => None,
    }
}

fn matches_expected(err: &Error, expected: ExpectedOpenError) -> bool {
    matches!(
        (err, expected),
        (Error::Encrypted(_), ExpectedOpenError::Encrypted)
            | (Error::Malformed(_), ExpectedOpenError::Malformed)
            | (Error::NoTextLayer, ExpectedOpenError::NoTextLayer)
    )
}

fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn fmt_ms(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.1}s", ms / 1000.0)
    } else {
        format!("{ms:.0}ms")
    }
}

#[derive(Clone, Copy)]
struct TimedRun {
    page_ms_mean: f64,
    search_ms: f64,
    nav_ms: f64,
}

struct SearchNavRun {
    search_ms: f64,
    nav_ms: f64,
}

fn page_pass_ms(doc: &dyn Document) -> Result<f64, Error> {
    let pages = doc.page_count();
    if pages == 0 {
        return Ok(0.0);
    }
    let mut total = 0.0;
    for page in 0..pages {
        let t0 = Instant::now();
        let _ = doc.page_text(page)?;
        total += measure::elapsed_ms(t0, Instant::now());
    }
    Ok(total / pages as f64)
}

fn search_first_ms(doc: &dyn Document) -> Result<f64, Error> {
    let pages = doc.page_count();
    let t0 = Instant::now();
    let query = SEARCH_QUERY;
    for page in 0..pages {
        let text = doc.page_text(page)?;
        if text.to_ascii_lowercase().contains(query) {
            break;
        }
    }
    Ok(measure::elapsed_ms(t0, Instant::now()))
}

fn nav_ms(doc: &dyn Document) -> Result<f64, Error> {
    let pages = doc.page_count();
    let mut anchor = None;
    for page in 0..pages {
        let text = doc.page_text(page)?;
        if text.to_ascii_lowercase().contains(SEARCH_QUERY) {
            anchor = Some(page);
            break;
        }
    }
    let Some(anchor) = anchor else {
        return Ok(0.0);
    };

    let mut samples = Vec::new();
    if anchor > 0 {
        let t0 = Instant::now();
        let _ = doc.page_text(anchor - 1)?;
        samples.push(measure::elapsed_ms(t0, Instant::now()));
    }
    if anchor + 1 < pages {
        let t0 = Instant::now();
        let _ = doc.page_text(anchor + 1)?;
        samples.push(measure::elapsed_ms(t0, Instant::now()));
    }
    if samples.is_empty() {
        Ok(0.0)
    } else {
        Ok(samples.iter().sum::<f64>() / samples.len() as f64)
    }
}

fn search_nav_run(doc: &dyn Document) -> Result<SearchNavRun, Error> {
    Ok(SearchNavRun {
        search_ms: search_first_ms(doc)?,
        nav_ms: nav_ms(doc)?,
    })
}

fn best_of_two_pages<F>(mut f: F) -> Result<f64, Error>
where
    F: FnMut() -> Result<f64, Error>,
{
    let run1 = f()?;
    let run2 = f()?;
    Ok(run1.min(run2))
}

fn best_of_two_search_nav<F>(mut f: F) -> Result<SearchNavRun, Error>
where
    F: FnMut() -> Result<SearchNavRun, Error>,
{
    let run1 = f()?;
    let run2 = f()?;
    Ok(SearchNavRun {
        search_ms: run1.search_ms.min(run2.search_ms),
        nav_ms: run1.nav_ms.min(run2.nav_ms),
    })
}

fn within_budget(
    label: &str,
    backend: &str,
    open_ms: f64,
    startup_ms: f64,
    timed: &TimedRun,
    peak_mb: u64,
) -> bool {
    let mut ok = true;
    let mut fail = |metric: &str, value: &str, limit: &str| {
        eprintln!("{label} ({backend}): BUDGET MISS {metric}={value} (limit {limit})");
        ok = false;
    };

    if open_ms > BUDGET_OPEN_MS {
        fail(
            "open_ms",
            &fmt_ms(open_ms),
            &format!("<= {BUDGET_OPEN_MS:.0}ms"),
        );
    }
    if startup_ms > BUDGET_STARTUP_MS {
        fail(
            "startup_ms",
            &fmt_ms(startup_ms),
            &format!("< {BUDGET_STARTUP_MS:.0}ms"),
        );
    }
    if timed.page_ms_mean > BUDGET_PAGE_MS {
        fail(
            "page_ms_mean",
            &fmt_ms(timed.page_ms_mean),
            &format!("< {BUDGET_PAGE_MS:.0}ms"),
        );
    }
    if timed.search_ms > BUDGET_SEARCH_MS {
        fail(
            "search_ms",
            &fmt_ms(timed.search_ms),
            &format!("< {BUDGET_SEARCH_MS:.0}ms"),
        );
    }
    if timed.nav_ms > BUDGET_NAV_MS {
        fail(
            "nav_ms",
            &fmt_ms(timed.nav_ms),
            &format!("< {BUDGET_NAV_MS:.0}ms"),
        );
    }
    if peak_mb > BUDGET_PEAK_RSS_MB {
        fail(
            "peak_rss_mb",
            &format!("{peak_mb} MB"),
            &format!("< {BUDGET_PEAK_RSS_MB} MB"),
        );
    }
    ok
}

fn print_error_row(backend: &str, label: &str, baseline: u64, peak: u64, delta: u64, kind: &str) {
    println!(
        "{backend:<8} {label:<18} {kind:<9} {startup:>10} {page:>9} {search:>9} {nav:>7} {base:>11} {peak:>12} {delta:>11}",
        startup = "ok",
        page = "-",
        search = "-",
        nav = "-",
        base = format!("{baseline} MB"),
        peak = format!("{peak} MB"),
        delta = format!("{delta} MB"),
    );
}

struct ProcessMetrics {
    open_ms: f64,
    startup_ms: f64,
    timed: TimedRun,
    baseline: u64,
    peak: u64,
    delta: u64,
}

fn print_metrics_row(backend: &str, label: &str, metrics: &ProcessMetrics) {
    println!(
        "{backend:<8} {label:<18} {open:>7} {startup:>10} {page:>9} {search:>9} {nav:>7} {base:>11} {peak:>12} {delta:>11}",
        open = group(metrics.open_ms.round() as u64),
        startup = group(metrics.startup_ms.round() as u64),
        page = group(metrics.timed.page_ms_mean.round() as u64),
        search = group(metrics.timed.search_ms.round() as u64),
        nav = group(metrics.timed.nav_ms.round() as u64),
        base = format!("{} MB", metrics.baseline),
        peak = format!("{} MB", metrics.peak),
        delta = format!("{} MB", metrics.delta),
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        return;
    }
    if args.len() != 4 {
        eprintln!("usage: bench <backend> <label> <pdf>");
        process::exit(2);
    }

    let backend_name = &args[1];
    let label = &args[2];
    let path = &args[3];
    let enforce_budget = env::var("BENCH_CHECK_BUDGET")
        .map(|v| v != "0" && !v.is_empty())
        .unwrap_or(true);

    let kind = match parse_backend(backend_name) {
        Ok(kind) => kind,
        Err(e) => {
            eprintln!("{label}: ERROR backend: {e}");
            process::exit(1);
        }
    };

    let baseline = measure::rss_mb();
    let process_start = Instant::now();

    let t_open = Instant::now();
    let open_result = open(kind, path, None);
    let open_ms = measure::elapsed_ms(t_open, Instant::now());

    if let Some(expected) = expected_open_error(label) {
        match open_result {
            Err(ref err) if matches_expected(err, expected) => {
                let peak = measure::peak_rss_mb();
                let delta = peak.saturating_sub(baseline);
                eprintln!("{label} ({backend_name}): expected open error: {err}");
                print_error_row(
                    backend_name,
                    label,
                    baseline,
                    peak,
                    delta,
                    match expected {
                        ExpectedOpenError::Encrypted => "Encrypted",
                        ExpectedOpenError::Malformed => "Malformed",
                        ExpectedOpenError::NoTextLayer => "NoTextLayer",
                    },
                );
                return;
            }
            Err(err) => {
                eprintln!("{label} ({backend_name}): ERROR open (unexpected): {err}");
                process::exit(1);
            }
            Ok(_) => {
                eprintln!(
                    "{label} ({backend_name}): ERROR open succeeded but expected {expected:?}"
                );
                process::exit(1);
            }
        }
    }

    let doc = match open_result {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("{label} ({backend_name}): ERROR open: {err}");
            process::exit(1);
        }
    };

    if let Err(err) = doc.page_text(0) {
        eprintln!("{label} ({backend_name}): ERROR page_text(0): {err}");
        process::exit(1);
    }
    let startup_ms = measure::elapsed_ms(process_start, Instant::now());

    eprintln!(
        "{label} ({backend_name}): open={} startup={}",
        fmt_ms(open_ms),
        fmt_ms(startup_ms),
    );

    let search_nav = match best_of_two_search_nav(|| search_nav_run(doc.as_ref())) {
        Ok(run) => run,
        Err(err) => {
            eprintln!("{label} ({backend_name}): ERROR search/nav: {err}");
            process::exit(1);
        }
    };

    let peak = measure::peak_rss_mb();
    let delta = peak.saturating_sub(baseline);

    let page_ms_mean = match best_of_two_pages(|| page_pass_ms(doc.as_ref())) {
        Ok(ms) => ms,
        Err(err) => {
            eprintln!("{label} ({backend_name}): ERROR page pass: {err}");
            process::exit(1);
        }
    };

    let timed = TimedRun {
        page_ms_mean,
        search_ms: search_nav.search_ms,
        nav_ms: search_nav.nav_ms,
    };

    eprintln!(
        "{label} ({backend_name}): best-of-2 page_ms={} search_ms={} nav_ms={} peak={peak} MB",
        fmt_ms(timed.page_ms_mean),
        fmt_ms(timed.search_ms),
        fmt_ms(timed.nav_ms),
    );

    print_metrics_row(
        backend_name,
        label,
        &ProcessMetrics {
            open_ms,
            startup_ms,
            timed,
            baseline,
            peak,
            delta,
        },
    );

    if enforce_budget && !within_budget(label, backend_name, open_ms, startup_ms, &timed, peak) {
        process::exit(1);
    }
}
