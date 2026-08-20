// SPDX-License-Identifier: AGPL-3.0

//! MuPDF parity gate — shared suite, no per-backend table special-casing.

#![cfg(feature = "mupdf-backend")]

include!("parity/mupdf.rs");
