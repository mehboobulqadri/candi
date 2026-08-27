// SPDX-License-Identifier: AGPL-3.0

//! Expected outline tree for the pinned arXiv fixture, shared by the
//! per-backend test targets.

use candi_pdf::TocItem;

/// Captured from the pinned arXiv fixture (1706.03762): MuPDF and PDFium
/// agree on every entry, nesting level, 1-based page, and destination top —
/// points from the page's top edge, measured from both engines (they emit
/// bit-identical f32 values).
pub fn attention_outline() -> Vec<TocItem> {
    let leaf = |title: &str, page: usize, top: f32| TocItem {
        title: title.into(),
        page,
        dest_top: Some(top),
        children: Vec::new(),
    };
    vec![
        leaf("Introduction", 2, 72.0),
        leaf("Background", 2, 355.006),
        TocItem {
            title: "Model Architecture".into(),
            page: 2,
            dest_top: Some(634.41003),
            children: vec![
                leaf("Encoder and Decoder Stacks", 3, 476.406),
                TocItem {
                    title: "Attention".into(),
                    page: 3,
                    dest_top: Some(674.881),
                    children: vec![
                        leaf("Scaled Dot-Product Attention", 4, 344.471),
                        leaf("Multi-Head Attention", 4, 625.08704),
                        leaf("Applications of Attention in our Model", 5, 273.687),
                    ],
                },
                leaf("Position-wise Feed-Forward Networks", 5, 490.775),
                leaf("Embeddings and Softmax", 5, 642.427),
                leaf("Positional Encoding", 6, 211.474),
            ],
        },
        leaf("Why Self-Attention", 6, 488.792),
        TocItem {
            title: "Training".into(),
            page: 7,
            dest_top: Some(289.762),
            children: vec![
                leaf("Training Data and Batching", 7, 346.142),
                leaf("Hardware and Schedule", 7, 459.279),
                leaf("Optimizer", 7, 550.597),
                leaf("Regularization", 7, 684.645),
            ],
        },
        TocItem {
            title: "Results".into(),
            page: 8,
            dest_top: Some(363.191),
            children: vec![
                leaf("Machine Translation", 8, 388.669),
                leaf("Model Variations", 8, 651.041),
                leaf("English Constituency Parsing", 9, 551.848),
            ],
        },
        leaf("Conclusion", 10, 356.727),
    ]
}
