// SPDX-License-Identifier: AGPL-3.0

//! Versioned sidecar (`{pdf}.candi.toml`): schema v1 stores the reading
//! position, schema v2 stores the full reading session (zoom, theme,
//! bookmarks) and migrates v1 files on load.
//!
//! Concurrent writers use last-write-wins; locking is deferred to v0.2 (Spike 4).

#[cfg(unix)]
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Serialize, Serializer};

/// Highest schema version understood by [`load`] (v1 reading position).
const POSITION_SCHEMA: u32 = 1;
/// Schema version written by [`save_session`] and read by [`load_session`].
const SESSION_SCHEMA: u32 = 2;

/// Theme applied to fresh sessions and v1 migrations.
const DEFAULT_THEME: &str = "Light";

/// Lowest and highest supported zoom percent, shared with the GUI's
/// quantizer so sidecar values and live zoom stay consistent.
pub const MIN_ZOOM_PERCENT: u16 = 25;
pub const MAX_ZOOM_PERCENT: u16 = 800;

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

/// Zoom preference persisted with a v2 session.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ZoomMode {
    FitWidth,
    Percent(u16),
}

impl Serialize for ZoomMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::FitWidth => serializer.serialize_str("fit-width"),
            Self::Percent(percent) => serializer.serialize_u16(*percent),
        }
    }
}

/// A user bookmark pointing at a 0-based page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bookmark {
    pub page: usize,
    pub created_at: String,
}

/// Full reading session (schema v2). Fractions are 0-based: `scroll_frac` is
/// the vertical position within the current page as a fraction of its height.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionState {
    pub page: usize,
    pub scroll_frac: f64,
    pub zoom: ZoomMode,
    pub theme: String,
    pub bookmarks: Vec<Bookmark>,
}

impl SessionState {
    /// Fresh session starting at the first page of a document.
    pub fn new(page_count: usize) -> Self {
        Self::fresh().clamp_to(page_count)
    }

    fn fresh() -> Self {
        Self {
            page: 0,
            scroll_frac: 0.0,
            zoom: ZoomMode::FitWidth,
            theme: DEFAULT_THEME.to_owned(),
            bookmarks: Vec::new(),
        }
    }

    /// Clamp out-of-range values to the document, mirroring [`crate::ViewState`]
    /// rules: pages clamp to the last index (empty documents stay on page 0),
    /// `scroll_frac` clamps to `[0.0, 1.0]` (non-finite becomes 0.0), and
    /// bookmarks pointing past the document are dropped.
    pub fn clamp_to(mut self, page_count: usize) -> Self {
        self.page = self.page.min(page_count.saturating_sub(1));
        self.scroll_frac = if self.scroll_frac.is_finite() {
            self.scroll_frac.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.bookmarks.retain(|bookmark| bookmark.page < page_count);
        self
    }

    /// Add a bookmark for `page`, or remove it if one exists.
    pub fn toggle_bookmark(&mut self, page: usize) {
        if let Some(index) = self.bookmarks.iter().position(|b| b.page == page) {
            self.bookmarks.remove(index);
        } else {
            self.add_bookmark(page);
        }
    }

    /// Add a bookmark for `page`; existing bookmarks for the same page are
    /// kept as-is.
    pub fn add_bookmark(&mut self, page: usize) {
        if self.bookmarks.iter().any(|b| b.page == page) {
            return;
        }
        self.bookmarks.push(Bookmark {
            page,
            created_at: format_rfc3339_utc(SystemTime::now()),
        });
    }

    /// Drop any bookmark for `page`; missing bookmarks are ignored.
    pub fn remove_bookmark(&mut self, page: usize) {
        self.bookmarks.retain(|b| b.page != page);
    }
}

/// Outcome of [`load_session`]: missing, valid data, or recoverable corruption.
#[derive(Debug)]
pub enum SessionLoad {
    Missing,
    Loaded(SessionState),
    Corrupt(String),
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
    /// The sidecar declares a schema version above [`POSITION_SCHEMA`] /
    /// [`SESSION_SCHEMA`]; `found` is the version as written.
    UnsupportedSchema {
        found: i64,
    },
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

/// Load the full reading session from the PDF's sidecar file.
///
/// Schema v2 files are read as-is; schema v1 files are migrated (page kept,
/// everything else at defaults). Missing files yield [`SessionLoad::Missing`].
/// The returned page/scroll values are **not** clamped to the document —
/// callers with a page count should apply [`SessionState::clamp_to`].
pub fn load_session(pdf: &Path) -> Result<SessionLoad, Error> {
    let path = sidecar_path(pdf);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(SessionLoad::Missing),
        Err(err) => return Err(Error::Io(err)),
    };
    parse_session_sidecar(&contents)
}

/// Atomically persist reading position next to the PDF (never modifies the PDF).
///
/// Sets `updated_at` to the current UTC time on every save.
pub fn save(pdf: &Path, position: &Position) -> Result<(), Error> {
    let position = Position::new(
        position.page(),
        position.scroll(),
        format_rfc3339_utc(SystemTime::now()),
    );
    let body = serialize_sidecar(&position).map_err(Error::Io)?;
    write_atomically(&sidecar_path(pdf), &body)
}

/// Atomically persist a reading session next to the PDF (never modifies the PDF).
///
/// Sets the top-level `updated_at` to the current UTC time on every save;
/// bookmark timestamps are preserved as stored.
pub fn save_session(pdf: &Path, session: &SessionState) -> Result<(), Error> {
    let body =
        serialize_session(session, &format_rfc3339_utc(SystemTime::now())).map_err(Error::Io)?;
    write_atomically(&sidecar_path(pdf), &body)
}

fn write_atomically(sidecar: &Path, body: &str) -> Result<(), Error> {
    let mut temp = sidecar.as_os_str().to_os_string();
    temp.push(".tmp");
    let temp_path = PathBuf::from(temp);

    let written = write_temp(&temp_path, body);
    if written.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    written?;

    if let Err(err) = fs::rename(&temp_path, sidecar) {
        let _ = fs::remove_file(&temp_path);
        return Err(Error::Io(err));
    }

    if let Some(parent) = sidecar.parent() {
        sync_dir(parent)?;
    }

    Ok(())
}

fn write_temp(temp_path: &Path, body: &str) -> Result<(), Error> {
    let mut file = File::create(temp_path).map_err(Error::Io)?;
    file.write_all(body.as_bytes()).map_err(Error::Io)?;
    file.sync_all().map_err(Error::Io)
}

/// Internal parse failure: recoverable corruption or a hard error.
enum Failure {
    Corrupt(String),
    Hard(Error),
}

fn parse_toml(contents: &str) -> Result<toml::Value, String> {
    toml::from_str(contents).map_err(|err| err.to_string())
}

/// Validate and return `schema_version`, rejecting anything above `max`.
fn schema_version_of(value: &toml::Value, max: u32) -> Result<u32, Failure> {
    match value.get("schema_version") {
        Some(toml::Value::Integer(version)) if *version >= 0 => {
            if *version > i64::from(max) {
                return Err(Failure::Hard(Error::UnsupportedSchema { found: *version }));
            }
            Ok(*version as u32)
        }
        Some(toml::Value::Integer(_)) => Err(Failure::Corrupt("negative schema_version".into())),
        Some(_) => Err(Failure::Corrupt("invalid schema_version type".into())),
        None => Err(Failure::Corrupt("missing schema_version".into())),
    }
}

fn uint_field(field: Option<&toml::Value>, name: &str) -> Result<usize, String> {
    match field {
        Some(toml::Value::Integer(value)) if *value >= 0 => Ok(*value as usize),
        Some(toml::Value::Integer(_)) => Err(format!("negative {name}")),
        Some(_) => Err(format!("invalid {name} type")),
        None => Err(format!("missing {name}")),
    }
}

fn text_field(field: Option<&toml::Value>, name: &str) -> Result<String, String> {
    match field {
        Some(toml::Value::String(text)) if !text.is_empty() => Ok(text.clone()),
        Some(toml::Value::String(_)) => Err(format!("empty {name}")),
        Some(_) => Err(format!("invalid {name} type")),
        None => Err(format!("missing {name}")),
    }
}

/// Extract the v1 `[reading]` position; unknown keys are ignored.
fn reading_position(value: &toml::Value) -> Result<Position, String> {
    let reading = value
        .get("reading")
        .ok_or_else(|| "missing [reading]".to_string())?;
    let page = uint_field(reading.get("page"), "page")?;
    let scroll = uint_field(reading.get("scroll"), "scroll")?;
    let updated_at = text_field(reading.get("updated_at"), "updated_at")?;
    Ok(Position::new(page, scroll, updated_at))
}

fn parse_sidecar(contents: &str) -> Result<Load, Error> {
    let value = match parse_toml(contents) {
        Ok(value) => value,
        Err(message) => return Ok(Load::Corrupt(message)),
    };

    let schema_version = match schema_version_of(&value, POSITION_SCHEMA) {
        Ok(schema_version) => schema_version,
        Err(Failure::Corrupt(message)) => return Ok(Load::Corrupt(message)),
        Err(Failure::Hard(err)) => return Err(err),
    };

    if schema_version == 0 {
        return Ok(Load::Corrupt(format!(
            "unsupported schema_version {schema_version}"
        )));
    }

    match reading_position(&value) {
        Ok(position) => Ok(Load::Loaded(position)),
        Err(message) => Ok(Load::Corrupt(message)),
    }
}

fn parse_session_sidecar(contents: &str) -> Result<SessionLoad, Error> {
    let value = match parse_toml(contents) {
        Ok(value) => value,
        Err(message) => return Ok(SessionLoad::Corrupt(message)),
    };

    let schema_version = match schema_version_of(&value, SESSION_SCHEMA) {
        Ok(schema_version) => schema_version,
        Err(Failure::Corrupt(message)) => return Ok(SessionLoad::Corrupt(message)),
        Err(Failure::Hard(err)) => return Err(err),
    };

    if schema_version == 0 {
        return Ok(SessionLoad::Corrupt(format!(
            "unsupported schema_version {schema_version}"
        )));
    }

    if schema_version == 1 {
        let migrated = match reading_position(&value) {
            Ok(position) => SessionState {
                page: position.page(),
                ..SessionState::fresh()
            },
            Err(message) => return Ok(SessionLoad::Corrupt(message)),
        };
        return Ok(SessionLoad::Loaded(migrated));
    }

    match session_fields(&value) {
        Ok(session) => Ok(SessionLoad::Loaded(session)),
        Err(message) => Ok(SessionLoad::Corrupt(message)),
    }
}

/// Extract the v2 session fields; unknown keys are ignored.
fn session_fields(value: &toml::Value) -> Result<SessionState, String> {
    text_field(value.get("updated_at"), "updated_at")?;

    let reading = value
        .get("reading")
        .ok_or_else(|| "missing [reading]".to_string())?;
    let page = uint_field(reading.get("page"), "page")?;
    let scroll_frac = frac_field(reading.get("scroll_frac"))?;
    let zoom = zoom_field(reading.get("zoom"))?;
    let theme = text_field(reading.get("theme"), "theme")?;

    let bookmarks = match value.get("bookmarks") {
        Some(toml::Value::Array(entries)) => entries
            .iter()
            .map(bookmark)
            .collect::<Result<Vec<_>, String>>()?,
        Some(_) => return Err("invalid bookmarks type".into()),
        None => Vec::new(),
    };

    Ok(SessionState {
        page,
        scroll_frac,
        zoom,
        theme,
        bookmarks,
    })
}

fn frac_field(field: Option<&toml::Value>) -> Result<f64, String> {
    match field {
        Some(toml::Value::Float(frac)) if frac.is_finite() => Ok(*frac),
        Some(toml::Value::Float(_)) => Err("non-finite scroll_frac".into()),
        Some(toml::Value::Integer(frac)) => Ok(*frac as f64),
        Some(_) => Err("invalid scroll_frac type".into()),
        None => Err("missing scroll_frac".into()),
    }
}

fn zoom_field(field: Option<&toml::Value>) -> Result<ZoomMode, String> {
    match field {
        Some(toml::Value::String(mode)) if mode == "fit-width" => Ok(ZoomMode::FitWidth),
        Some(toml::Value::String(_)) => Err("unknown zoom mode".into()),
        // Absurd values clamp into the supported range, consistent with the
        // GUI's quantizer bounds.
        Some(toml::Value::Integer(percent)) if *percent >= 0 => {
            let percent =
                (*percent).clamp(i64::from(MIN_ZOOM_PERCENT), i64::from(MAX_ZOOM_PERCENT));
            Ok(ZoomMode::Percent(percent as u16))
        }
        Some(toml::Value::Integer(_)) => Err("negative zoom".into()),
        Some(_) => Err("invalid zoom type".into()),
        None => Err("missing zoom".into()),
    }
}

fn bookmark(value: &toml::Value) -> Result<Bookmark, String> {
    let page = uint_field(value.get("page"), "bookmark page")?;
    let created_at = text_field(value.get("created_at"), "bookmark created_at")?;
    Ok(Bookmark { page, created_at })
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
        schema_version: POSITION_SCHEMA,
        reading: ReadingSection {
            page: position.page(),
            scroll: position.scroll(),
            updated_at: position.updated_at(),
        },
    };
    toml::to_string_pretty(&file).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

#[derive(Serialize)]
struct SessionFile<'a> {
    schema_version: u32,
    updated_at: &'a str,
    reading: SessionReading<'a>,
    bookmarks: Vec<BookmarkSection<'a>>,
}

#[derive(Serialize)]
struct SessionReading<'a> {
    page: usize,
    scroll_frac: f64,
    zoom: &'a ZoomMode,
    theme: &'a str,
}

#[derive(Serialize)]
struct BookmarkSection<'a> {
    page: usize,
    created_at: &'a str,
}

fn serialize_session(session: &SessionState, updated_at: &str) -> io::Result<String> {
    let file = SessionFile {
        schema_version: SESSION_SCHEMA,
        updated_at,
        reading: SessionReading {
            page: session.page,
            scroll_frac: session.scroll_frac,
            zoom: &session.zoom,
            theme: &session.theme,
        },
        bookmarks: session
            .bookmarks
            .iter()
            .map(|bookmark| BookmarkSection {
                page: bookmark.page,
                created_at: &bookmark.created_at,
            })
            .collect(),
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
