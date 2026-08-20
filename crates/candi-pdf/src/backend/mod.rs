// SPDX-License-Identifier: AGPL-3.0

#[cfg(feature = "mupdf-backend")]
mod mupdf;

#[cfg(feature = "pdfium-backend")]
mod pdfium;

#[cfg(feature = "mupdf-backend")]
pub use mupdf::MupdfBackend;

#[cfg(feature = "pdfium-backend")]
pub use pdfium::PdfiumBackend;
