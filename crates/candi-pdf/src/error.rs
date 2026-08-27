// SPDX-License-Identifier: AGPL-3.0

//! Error taxonomy for the document-backend layer.
//!
//! Messages are for humans only — never matched on. Code branches on the kind
//! (via `matches!` or `match`), never on message content.

use std::fmt;

/// Errors produced by opening and reading documents.
///
/// Backends map native error codes to these kinds; anything unrecognized
/// becomes [`Error::Other`]. `Encrypted` vs `WrongPassword` is decided by
/// whether a password was supplied to `open()`.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The file does not exist.
    NotFound(String),
    /// The file exists but the process lacks read access.
    PermissionDenied(String),
    /// The document is encrypted.
    Encrypted(String),
    /// A password was supplied but rejected.
    WrongPassword(String),
    /// Document-level: no extractable text layer (image-only or scanned).
    NoTextLayer,
    /// The file is not a parseable PDF.
    Malformed(String),
    /// Not supported by this build or backend.
    Unsupported(String),
    /// Anything that does not fit the kinds above.
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(m) => write!(f, "file not found: {m}"),
            Error::PermissionDenied(m) => write!(f, "permission denied: {m}"),
            Error::Encrypted(m) => write!(f, "encrypted document: {m}"),
            Error::WrongPassword(m) => write!(f, "wrong password: {m}"),
            Error::NoTextLayer => {
                write!(f, "no extractable text layer (image-only or scanned)")
            }
            Error::Malformed(m) => write!(f, "malformed document: {m}"),
            Error::Unsupported(m) => write!(f, "unsupported: {m}"),
            Error::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}
