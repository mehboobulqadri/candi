// SPDX-License-Identifier: AGPL-3.0

//! Versioned reading-position sidecar (`{pdf}.candi.toml`).
//!
//! Concurrent writers use last-write-wins; locking is deferred to v0.2 (Spike 4).

#[cfg(unix)]
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Persisted reading position (schema v1). Page indices are **0-based**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    page: usize,
    scroll: usize,
    updated_at: String,
}

impl Position {
    pub fn new(page: usize, scroll: usize, updated_at: impl Into<String>) -> Self {
        Self {
            page,
            scroll,
            updated_at: updated_at.into(),
        }
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn updated_at(&self) -> &str {
        &self.updated_at
    }
}

/// Outcome of [`load`]: missing, valid data, or recoverable corruption.
#[derive(Debug)]
pub enum Load {
    Missing,
    Loaded(Position),
    Corrupt(String),
}

/// Hard failures from sidecar I/O or unsupported future schema versions.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    UnsupportedSchema { found: u32 },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::UnsupportedSchema { found } => {
                write!(f, "unsupported sidecar schema version {found}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::UnsupportedSchema { .. } => None,
        }
    }
}

/// Sidecar path for a PDF: `{pdf}.candi.toml` in the same directory.
pub fn sidecar_path(pdf: &Path) -> PathBuf {
    let mut path = pdf.as_os_str().to_os_string();
    path.push(".candi.toml");
    PathBuf::from(path)
}

/// Load reading position from the PDF's sidecar file.
pub fn load(pdf: &Path) -> Result<Load, Error> {
    let path = sidecar_path(pdf);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Load::Missing),
        Err(err) => return Err(Error::Io(err)),
    };
    parse_sidecar(&contents)
}

/// Atomically persist reading position next to the PDF (never modifies the PDF).
///
/// Sets `updated_at` to the current UTC time on every save.
pub fn save(pdf: &Path, position: &Position) -> Result<(), Error> {
    let sidecar = sidecar_path(pdf);
    let position = Position::new(
        position.page(),
        position.scroll(),
        format_rfc3339_utc(SystemTime::now()),
    );
    let body = serialize_sidecar(&position).map_err(Error::Io)?;

    let mut temp = sidecar.as_os_str().to_os_string();
    temp.push(".tmp");
    let temp_path = PathBuf::from(temp);

    {
        let mut file = File::create(&temp_path).map_err(Error::Io)?;
        file.write_all(body.as_bytes()).map_err(Error::Io)?;
        file.sync_all().map_err(Error::Io)?;
    }

    if let Err(err) = fs::rename(&temp_path, &sidecar) {
        let _ = fs::remove_file(&temp_path);
        return Err(Error::Io(err));
    }

    if let Some(parent) = sidecar.parent() {
        sync_dir(parent)?;
    }

    Ok(())
}

fn parse_sidecar(contents: &str) -> Result<Load, Error> {
    let value: toml::Value = match toml::from_str(contents) {
        Ok(value) => value,
        Err(err) => return Ok(Load::Corrupt(err.to_string())),
    };

    let schema_version = match value.get("schema_version") {
        Some(toml::Value::Integer(version)) if *version >= 0 => *version as u32,
        Some(toml::Value::Integer(_)) => {
            return Ok(Load::Corrupt("negative schema_version".into()));
        }
        Some(_) => return Ok(Load::Corrupt("invalid schema_version type".into())),
        None => return Ok(Load::Corrupt("missing schema_version".into())),
    };

    if schema_version > 1 {
        return Err(Error::UnsupportedSchema {
            found: schema_version,
        });
    }
    if schema_version != 1 {
        return Ok(Load::Corrupt(format!(
            "unsupported schema_version {schema_version}"
        )));
    }

    let reading = match value.get("reading") {
        Some(reading) => reading,
        None => return Ok(Load::Corrupt("missing [reading]".into())),
    };

    let page = match reading.get("page") {
        Some(toml::Value::Integer(page)) if *page >= 0 => *page as usize,
        Some(toml::Value::Integer(_)) => return Ok(Load::Corrupt("negative page".into())),
        Some(_) => return Ok(Load::Corrupt("invalid page type".into())),
        None => return Ok(Load::Corrupt("missing page".into())),
    };

    let scroll = match reading.get("scroll") {
        Some(toml::Value::Integer(scroll)) if *scroll >= 0 => *scroll as usize,
        Some(toml::Value::Integer(_)) => return Ok(Load::Corrupt("negative scroll".into())),
        Some(_) => return Ok(Load::Corrupt("invalid scroll type".into())),
        None => return Ok(Load::Corrupt("missing scroll".into())),
    };

    let updated_at = match reading.get("updated_at") {
        Some(toml::Value::String(updated_at)) if !updated_at.is_empty() => updated_at.clone(),
        Some(toml::Value::String(_)) => return Ok(Load::Corrupt("empty updated_at".into())),
        Some(_) => return Ok(Load::Corrupt("invalid updated_at type".into())),
        None => return Ok(Load::Corrupt("missing updated_at".into())),
    };

    Ok(Load::Loaded(Position::new(page, scroll, updated_at)))
}

#[derive(Serialize)]
struct SidecarFile<'a> {
    schema_version: u32,
    reading: ReadingSection<'a>,
}

#[derive(Serialize)]
struct ReadingSection<'a> {
    page: usize,
    scroll: usize,
    updated_at: &'a str,
}

fn serialize_sidecar(position: &Position) -> io::Result<String> {
    let file = SidecarFile {
        schema_version: 1,
        reading: ReadingSection {
            page: position.page(),
            scroll: position.scroll(),
            updated_at: position.updated_at(),
        },
    };
    toml::to_string_pretty(&file).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn format_rfc3339_utc(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let (year, month, day) = unix_days_to_ymd((seconds / 86_400) as i64);
    let time_of_day = seconds % 86_400;
    let hour = time_of_day / 3_600;
    let minute = (time_of_day % 3_600) / 60;
    let second = time_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn unix_days_to_ymd(mut days: i64) -> (i32, u32, u32) {
    days += 719_468;
    let era = (if days >= 0 { days } else { days - 146_096 }) / 146_097;
    let day_of_era = (days - era * 146_097) as u32;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i32 + (era * 400) as i32;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = if month_prime < 10 { year } else { year + 1 };
    (year, month, day)
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<(), Error> {
    OpenOptions::new()
        .read(true)
        .open(dir)
        .map_err(Error::Io)?
        .sync_all()
        .map_err(Error::Io)
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> Result<(), Error> {
    Ok(())
}
