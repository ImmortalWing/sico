//! The frozen layout + hit-test corpus (STEP-0592).
//!
//! Every digest below is the SHA-256 of one computed layout (the per-node
//! rectangles) and of the hit results for a fixed probe list, in the canonical
//! versioned wire form of [`Layout::digest`] and [`Layout::hit_digest`]. The
//! layout is deterministic by construction — every measurement is a frozen
//! token (`crate::tokens`), no DirectWrite glyph metrics, no DPI, no time, no
//! environment value — so the same tree + client size hashes the same way on
//! every machine and in every process.
//!
//! The scenes and probe points come from [`layout::corpus`], the single source
//! of truth shared with `examples/layout_digest.rs` (the regeneration tool),
//! so the two can never drift. The shapes mirror the frozen frame corpus
//! scenes (`frame_corpus.rs`) so the layout corpus and the frame corpus stay
//! aligned.
//!
//! The corpus is platform-neutral: the layout and hit-testing are pure
//! computation with no Win32 dependency, so these assertions run everywhere.
//! A digest change means the layout rules, the token table or the corpus
//! scenes changed, and re-freezing is a deliberate, audited act.

// The casts below are bounded (client side ≤ 16 384) and behaviour-neutral.
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use sico_ui_renderer_d2d::UiBackendError;
use sico_ui_renderer_d2d::layout::{Hit, compute, corpus};

/// The frozen digests of the corpus, in corpus order: `(layout, hit)`.
const CORPUS: [(&str, u32, u32, &str, &str); 4] = [
    (
        "empty",
        200,
        120,
        "2f90b55b902998af5d46020abc066dfe69cbb131bfd18ea33292979095716f04",
        "a7a471a580609be848816f8aa952caf0fcde37a7d7e19813964eaad2516b5730",
    ),
    (
        "text",
        320,
        160,
        "97a1ffb47e765b02c7f640d46882d7ff63df533a8fe8f18cec99f54caab7d816",
        "42585f1da3dd1ba3942602499a1d6f9a6eec19f4dcd1acc4e7ef7616d7d7e63c",
    ),
    (
        "controls",
        360,
        280,
        "27fe362d88a8242e53105f49ae007c93df06fbabe9766b751276fb9cb9eb6500",
        "87be33ba52be676ae6bcd1435e185ab94ff3a383794e73a0d2b23ea26f4bd4a0",
    ),
    (
        "row",
        320,
        120,
        "9da940cb69d5bdb55d43a909a9773621290eeb438ac020b7588bb6dcc2512274",
        "133db61be41ff487736da7f09eb9fa58181405c48458ea9ee933ef5350c60532",
    ),
];

#[test]
fn layout_corpus_digests_are_frozen() {
    let scenes = corpus::scenes();
    assert_eq!(scenes.len(), CORPUS.len(), "corpus scene count changed");
    for (scene, (name, width, height, layout_digest, hit_digest)) in
        scenes.iter().zip(CORPUS.iter())
    {
        assert_eq!(scene.name, *name, "corpus order changed");
        assert_eq!(scene.width, *width, "{name}: scene width");
        assert_eq!(scene.height, *height, "{name}: scene height");
        let layout = compute(&scene.tree, scene.width, scene.height).expect("layout");
        assert_eq!(
            layout.digest(),
            *layout_digest,
            "{name}: layout digest changed"
        );
        let probes = corpus::probes(&layout);
        assert_eq!(
            layout.hit_digest(&probes),
            *hit_digest,
            "{name}: hit digest changed"
        );
    }
}

#[test]
fn the_same_tree_lays_out_the_same_way_in_this_process() {
    // Two independent recomputations in one process must agree; the frozen
    // const above already pins the value across processes.
    for scene in corpus::scenes() {
        let first = compute(&scene.tree, scene.width, scene.height).expect("layout");
        let second = compute(&scene.tree, scene.width, scene.height).expect("layout");
        assert_eq!(
            first.digest(),
            second.digest(),
            "{}: recomputation differs",
            scene.name
        );
    }
}

#[test]
fn every_leaf_under_its_own_center_hits_itself() {
    // The deepest-node rule: the centre of a leaf's rectangle must resolve to
    // that leaf, never to an ancestor. Containers are skipped because their
    // centre may be covered by a child (the deepest node there is the child).
    for scene in corpus::scenes() {
        let layout = compute(&scene.tree, scene.width, scene.height).expect("layout");
        for (handle, node) in scene.tree.iter() {
            if node.kind.accepts_children() {
                continue;
            }
            let Some(rect) = layout.rect(handle) else {
                panic!("{}: node {handle} has no rect", scene.name);
            };
            let Some(kind) = layout.kind(handle) else {
                panic!("{}: node {handle} has no kind", scene.name);
            };
            let (x, y) = rect.center();
            let hit = layout.hit(x as i32, y as i32);
            assert_eq!(
                hit,
                Hit::Node(handle),
                "{}: node {handle} ({kind:?}) centre must hit itself",
                scene.name
            );
        }
    }
}

#[test]
fn a_point_outside_the_client_is_an_explicit_miss() {
    for scene in corpus::scenes() {
        let layout = compute(&scene.tree, scene.width, scene.height).expect("layout");
        assert_eq!(
            layout.hit(-1, -1),
            Hit::Miss,
            "{}: top-left outside",
            scene.name
        );
        assert_eq!(
            layout.hit(scene.width as i32, scene.height as i32),
            Hit::Miss,
            "{}: bottom-right edge outside",
            scene.name
        );
        assert_eq!(
            layout.hit(-5, 50),
            Hit::Miss,
            "{}: left outside",
            scene.name
        );
    }
}

#[test]
fn a_layout_without_a_window_is_refused() {
    let scene = corpus::scenes().remove(0);
    let empty = sico_ui_renderer_d2d::tree::UiTree::new();
    assert_eq!(
        compute(&empty, scene.width, scene.height),
        Err(UiBackendError::NoWindow)
    );
}

#[test]
fn a_zero_client_size_is_refused() {
    let scene = corpus::scenes().remove(0);
    assert_eq!(
        compute(&scene.tree, 0, scene.height),
        Err(UiBackendError::InvalidArgument {
            which: "client-size"
        })
    );
    assert_eq!(
        compute(&scene.tree, scene.width, 0),
        Err(UiBackendError::InvalidArgument {
            which: "client-size"
        })
    );
}
