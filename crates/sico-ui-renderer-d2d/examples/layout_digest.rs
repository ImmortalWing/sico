//! Prints the layout + hit digests of the frozen corpus.
//!
//! Run it after a deliberate change to the layout rules, the token table or
//! the corpus scenes to see the new digests, then update
//! `tests/layout_corpus.rs` and the published evidence file:
//!
//! ```text
//! cargo run -p sico-ui-renderer-d2d --example layout_digest
//! ```
//!
//! The scenes and probe points come from
//! [`sico_ui_renderer_d2d::layout::corpus`], the single source of truth shared
//! with `tests/layout_corpus.rs`, so the regeneration tool and the frozen
//! assertions can never drift.

use sico_ui_renderer_d2d::layout::{compute, corpus};

fn main() {
    for scene in corpus::scenes() {
        let layout = compute(&scene.tree, scene.width, scene.height).expect("layout");
        let probes = corpus::probes(&layout);
        println!(
            "{} {}x{} layout {} hit {}",
            scene.name,
            scene.width,
            scene.height,
            layout.digest(),
            layout.hit_digest(&probes)
        );
    }
}
