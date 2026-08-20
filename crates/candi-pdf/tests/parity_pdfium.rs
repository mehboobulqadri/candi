// SPDX-License-Identifier: AGPL-3.0

//! PDFium parity gate — shared suite, no per-backend table special-casing.

#![cfg(feature = "pdfium-backend")]

include!("parity/pdfium.rs");
