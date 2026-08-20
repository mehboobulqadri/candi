// SPDX-License-Identifier: AGPL-3.0

#[cfg(feature = "mupdf-backend")]
mod mupdf;

#[cfg(feature = "mupdf-backend")]
pub use mupdf::MupdfBackend;
