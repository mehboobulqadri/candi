// SPDX-License-Identifier: AGPL-3.0

//! Expected outline tree for the pinned arXiv fixture, shared by the
//! per-backend test targets.

use candi_pdf::TocItem;

/// Captured from the pinned arXiv fixture (1706.03762): MuPDF and PDFium
/// agree on every entry, nesting level, and 1-based page.
pub fn attention_outline() -> Vec<TocItem> {
    let leaf = |title: &str, page: usize| TocItem {
        title: title.into(),
        page,
        children: Vec::new(),
    };
    vec![
        leaf("Introduction", 2),
        leaf("Background", 2),
        TocItem {
            title: "Model Architecture".into(),
            page: 2,
            children: vec![
                leaf("Encoder and Decoder Stacks", 3),
                TocItem {
                    title: "Attention".into(),
                    page: 3,
                    children: vec![
                        leaf("Scaled Dot-Product Attention", 4),
                        leaf("Multi-Head Attention", 4),
                        leaf("Applications of Attention in our Model", 5),
                    ],
                },
                leaf("Position-wise Feed-Forward Networks", 5),
                leaf("Embeddings and Softmax", 5),
                leaf("Positional Encoding", 6),
            ],
        },
        leaf("Why Self-Attention", 6),
        TocItem {
            title: "Training".into(),
            page: 7,
            children: vec![
                leaf("Training Data and Batching", 7),
                leaf("Hardware and Schedule", 7),
                leaf("Optimizer", 7),
                leaf("Regularization", 7),
            ],
        },
        TocItem {
            title: "Results".into(),
            page: 8,
            children: vec![
                leaf("Machine Translation", 8),
                leaf("Model Variations", 8),
                leaf("English Constituency Parsing", 9),
            ],
        },
        leaf("Conclusion", 10),
    ]
}
