//! STEP-0497: the in-process runner's guest memory ceiling must cover the
//! guest linear memory every Script component declares.
//!
//! The runner enforces the ceiling through wasmtime's store limits. A
//! ceiling below the declared minimum makes wasmtime unable to satisfy the
//! minimum at instantiation, so the launch dies with an opaque "forcing trap
//! when growing memory to … bytes" trap instead of a typed refusal. The
//! declaration is fixed (the minimum equals the maximum), so the ceiling has
//! to clear it on every path.
//!
//! This guard is a pure constant comparison: it runs on every machine and
//! needs no runtime binary.

use sico_codegen_wasm::SCRIPT_MEMORY_BYTES;
use sico_runner::RunnerLimits;

#[test]
fn the_runner_ceiling_covers_the_declared_component_memory() {
    let ceiling = RunnerLimits::default().memory_bytes;
    assert!(
        u64::try_from(ceiling).unwrap_or(u64::MAX) >= SCRIPT_MEMORY_BYTES,
        "the in-process runner enforces a {ceiling} B guest memory ceiling but a script component \
         declares {SCRIPT_MEMORY_BYTES} B: wasmtime cannot satisfy the declared minimum and every \
         script instantiation traps"
    );
}
