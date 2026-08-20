// SPDX-License-Identifier: AGPL-3.0
// Benchmark harness binary — driven by bench/run.sh, one process per doc.
// Methodology replicates the spike probe (spikes/pdf-backend/): open_ms timed
// before the file read, best-of-2 extraction runs per process, process-level
// RSS (VmRSS baseline -> VmHWM peak -> delta), nonzero exit on error paths.

use std::env;
use std::process;
use std::time::Instant;

mod measure {
    use std::time::Instant;

    pub fn elapsed_ms(start: Instant, end: Instant) -> f64 {
        end.duration_since(start).as_secs_f64() * 1000.0
    }

    // RSS via /proc/self/status is Linux-only (spike methodology). The Windows
    // job (v0.1 tag) needs a fallback — noted here, not built now.
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

mod adapter {
    use std::fs;

    // Backend measurement seam — candi-pdf's backend trait (slice 01/01) and
    // the real extraction benchmarks (slice 01/05) plug in here. Until then,
    // open measures the file read (matching the spike's cross-backend-
    // comparable open_ms, which includes the read) and extraction reports
    // NotWired, printed as "-" in the output table.
    pub struct Doc {
        _bytes: Vec<u8>,
    }

    pub struct Extraction {
        // Kept in the spike's shape for slice 01/05; unused until then.
        #[allow(dead_code)]
        pub text: String,
        pub per_page_chars: Vec<usize>,
    }

    pub enum Error {
        /// Extraction not implemented yet (slice 01/05) — not a failure.
        NotWired,
        // Real backend errors land here in slice 01/05.
        #[allow(dead_code)]
        Failed(String),
    }

    pub fn open(path: &str) -> Result<Doc, String> {
        let bytes = fs::read(path).map_err(|e| format!("cannot open {path}: {e}"))?;
        Ok(Doc { _bytes: bytes })
    }

    impl Doc {
        pub fn extract_all(&self) -> Result<Extraction, Error> {
            Err(Error::NotWired)
        }
    }
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

fn fmt_rate(per_s: f64) -> String {
    if per_s >= 1_000_000.0 {
        format!("{:.2}M", per_s / 1_000_000.0)
    } else if per_s >= 1_000.0 {
        format!("{:.0}K", per_s / 1_000.0)
    } else {
        format!("{per_s:.0}")
    }
}

fn fmt_ms(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.1}s", ms / 1000.0)
    } else {
        format!("{ms:.0}ms")
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // `cargo test --all-targets` / `cargo bench` run this binary as a test
    // binary with no args: nothing to measure, pass like an empty test suite.
    if args.len() == 1 {
        return;
    }
    if args.len() != 3 {
        eprintln!("usage: bench <label> <pdf>");
        process::exit(2);
    }
    let label = &args[1];
    let path = &args[2];

    let baseline = measure::rss_mb();

    let t0 = Instant::now();
    let doc = match adapter::open(path) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("{label}: ERROR open: {e}");
            process::exit(1);
        }
    };
    let open_ms = measure::elapsed_ms(t0, Instant::now());

    let mut best_extract_ms: Option<f64> = None;
    let mut best_chars: Option<usize> = None;
    let mut failed = false;
    for run in 1..=2 {
        let t1 = Instant::now();
        match doc.extract_all() {
            Err(adapter::Error::NotWired) => {
                eprintln!("{label}: run {run}: extraction not wired (slice 01/05)");
                break;
            }
            Err(adapter::Error::Failed(e)) => {
                eprintln!("{label}: run {run}: ERROR extract: {e}");
                failed = true;
            }
            Ok(ext) => {
                let ms = measure::elapsed_ms(t1, Instant::now());
                let chars: usize = ext.per_page_chars.iter().sum();
                eprintln!(
                    "{label}: run {run}: extract={} chars={chars} chars/s={:.0}",
                    fmt_ms(ms),
                    chars as f64 / ms.max(1.0) * 1000.0,
                );
                if best_extract_ms.is_none_or(|best| ms < best) {
                    best_extract_ms = Some(ms);
                    best_chars = Some(chars);
                }
            }
        }
    }

    let peak = measure::peak_rss_mb();
    let delta = peak - baseline;

    let (extract_col, chars_col, rate_col) = match (best_chars, best_extract_ms) {
        (Some(chars), Some(ms)) => (
            group(ms.round() as u64),
            group(chars as u64),
            fmt_rate(chars as f64 / ms.max(1.0) * 1000.0),
        ),
        _ => ("-".to_string(), "-".to_string(), "-".to_string()),
    };
    let open_col = group(open_ms.round() as u64);

    println!(
        "{label:<18} {open:>7} {extract:>10} {chars:>9} {rate:>8} {base:>11} {peak:>12} {delta:>11}",
        open = open_col,
        extract = extract_col,
        chars = chars_col,
        rate = rate_col,
        base = format!("{baseline} MB"),
        peak = format!("{peak} MB"),
        delta = format!("{delta} MB"),
    );

    if failed {
        process::exit(1);
    }
}
