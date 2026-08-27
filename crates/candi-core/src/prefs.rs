// SPDX-License-Identifier: AGPL-3.0

//! App-level config (`config.toml` under the XDG config dir): the last
//! chosen theme and the recent-documents list. Reads are tolerant — a
//! missing, unparsable, or future-schema file yields defaults — and writes
//! are atomic (tmp + rename), mirroring the [`crate::state`] sidecar
//! patterns. GUI-only today; the TUI has no config.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::state::{DEFAULT_THEME, format_rfc3339_utc, write_file_atomically};

/// Schema version written by [`store`] and understood by [`load`].
pub const CONFIG_SCHEMA: u32 = 1;

/// Maximum number of recent documents retained.
const MAX_RECENTS: usize = 10;

/// App configuration: appearance choice plus recent documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefs {
    /// Last chosen theme name (built-in or custom; unknown names fall back
    /// to [`DEFAULT_THEME`] at load time).
    pub theme: String,
    /// Recent documents, most recent first, capped at [`MAX_RECENTS`].
    pub recents: Vec<Recent>,
}

/// One recent document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recent {
    pub path: PathBuf,
    /// RFC3339 UTC timestamp of the last open.
    pub last_opened: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_owned(),
            recents: Vec::new(),
        }
    }
}

impl Prefs {
    /// Record a successful open: the path is canonicalized (best effort),
    /// deduped against the existing list, moved to the front, and stamped.
    pub fn record_open(&mut self, path: &Path) {
        let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.recents.retain(|recent| recent.path.as_path() != path);
        self.recents.insert(
            0,
            Recent {
                path,
                last_opened: format_rfc3339_utc(SystemTime::now()),
            },
        );
        self.recents.truncate(MAX_RECENTS);
    }
}

/// `$XDG_CONFIG_HOME/candi/config.toml`, else `$HOME/.config/candi/config.toml`.
/// `None` when neither variable is set (Linux-only per project constraints).
pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.toml"))
}

/// The Candi configuration directory: `$XDG_CONFIG_HOME/candi`, else
/// `$HOME/.config/candi`; `None` when neither variable is set.
pub fn config_dir() -> Option<PathBuf> {
    resolve_config_dir(
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
}

/// Config directory from the XDG/Home environment; empty values count as
/// unset, matching shell convention.
fn resolve_config_dir(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    if let Some(dir) = xdg.filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir).join("candi"));
    }
    let home = home.filter(|home| !home.is_empty())?;
    Some(PathBuf::from(home).join(".config").join("candi"))
}

/// Load the config at `path`, falling back to [`Prefs::default`] when it is
/// missing, corrupt, or written by a newer schema.
pub fn load(path: &Path) -> Prefs {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Prefs::default(),
        Err(err) => {
            warn(format!("reading config: {err}"));
            return Prefs::default();
        }
    };
    match toml::from_str::<ConfigFile>(&contents) {
        Ok(file) if file.schema_version <= CONFIG_SCHEMA => Prefs {
            theme: file
                .appearance
                .map(|a| a.theme)
                .filter(|theme| !theme.is_empty())
                .unwrap_or_else(|| DEFAULT_THEME.to_owned()),
            recents: file
                .recents
                .into_iter()
                .take(MAX_RECENTS)
                .filter_map(|recent| {
                    if recent.path.is_empty() || recent.last_opened.is_empty() {
                        warn(
                            "config recents entry missing path or timestamp — dropping".to_owned(),
                        );
                        None
                    } else {
                        Some(Recent {
                            path: PathBuf::from(recent.path),
                            last_opened: recent.last_opened,
                        })
                    }
                })
                .collect(),
        },
        Ok(file) => {
            warn(format!(
                "config schema_version {} is newer than {} — using defaults",
                file.schema_version, CONFIG_SCHEMA
            ));
            Prefs::default()
        }
        Err(err) => {
            warn(format!("corrupt config, using defaults: {err}"));
            Prefs::default()
        }
    }
}

/// Atomically write the config to `path`, creating parent directories as
/// needed. A failed write surfaces as `Err` — callers warn, never panic.
pub fn store(path: &Path, prefs: &Prefs) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let file = ConfigFile {
        schema_version: CONFIG_SCHEMA,
        appearance: Some(AppearanceSection {
            theme: prefs.theme.clone(),
        }),
        recents: prefs
            .recents
            .iter()
            .map(|recent| RecentSection {
                path: recent.path.to_string_lossy().into_owned(),
                last_opened: recent.last_opened.clone(),
            })
            .collect(),
    };
    let body = toml::to_string_pretty(&file)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    write_file_atomically(path, &body)
}

fn warn(message: String) {
    eprintln!("candi: {message}");
}

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    schema_version: u32,
    appearance: Option<AppearanceSection>,
    #[serde(default)]
    recents: Vec<RecentSection>,
}

#[derive(Serialize, Deserialize)]
struct AppearanceSection {
    theme: String,
}

#[derive(Serialize, Deserialize)]
struct RecentSection {
    path: String,
    last_opened: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_recents_entries_are_dropped_with_a_warning() {
        let dir = std::env::temp_dir().join(format!("candi-prefs-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(
            &path,
            "schema_version = 1\n\
             \n\
             [[recents]]\n\
             path = \"/tmp/book.pdf\"\n\
             last_opened = \"2026-08-01T00:00:00Z\"\n\
             \n\
             [[recents]]\n\
             path = \"\"\n\
             last_opened = \"2026-08-02T00:00:00Z\"\n",
        )
        .unwrap();
        let prefs = load(&path);
        assert_eq!(prefs.recents.len(), 1, "the malformed entry is dropped");
        assert_eq!(prefs.recents[0].path, PathBuf::from("/tmp/book.pdf"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_dir_prefers_xdg_then_home_then_none() {
        assert_eq!(
            resolve_config_dir(Some("/xdg"), Some("/home")),
            Some(PathBuf::from("/xdg/candi"))
        );
        assert_eq!(
            resolve_config_dir(None, Some("/home")),
            Some(PathBuf::from("/home/.config/candi"))
        );
        assert_eq!(resolve_config_dir(None, None), None);
        assert_eq!(
            resolve_config_dir(Some(""), Some("/home")),
            Some(PathBuf::from("/home/.config/candi"))
        );
        assert_eq!(resolve_config_dir(None, Some("")), None);
    }
}
