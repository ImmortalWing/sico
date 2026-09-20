//! M22 regression: the self-host link unit must lower inside the frozen IR
//! verifier limits.  STEP-0249 repaired the `control_while_function_ir`
//! 256-local overflow by extracting `while_bytes_at_rhs_packed`; this test
//! pins every function of the assembled parser link unit to the verifier
//! bounds so the frontier cannot silently regress into a fail-closed build.

use sico_ir::{ModuleImport, lower_core_modules};
use sico_source::{SourceFile, SourceId};

const DRIVER_SOURCE: &str = include_str!("../../../selfhost/parser_driver.sico");
const PARSER_SOURCE: &str = include_str!("../../../selfhost/parser.sico");

#[test]
fn selfhost_link_unit_stays_inside_ir_verifier_limits() {
    let entry = SourceFile::from_text(
        SourceId::new(0),
        "parser_driver.sico",
        DRIVER_SOURCE,
    )
    .expect("driver source is bounded");
    let parser = SourceFile::from_text(SourceId::new(1), "parser.sico", PARSER_SOURCE)
        .expect("parser source is bounded");
    let imports = vec![ModuleImport {
        name: "parser",
        source: &parser,
    }];
    let module = lower_core_modules(&entry, &imports, &[]).expect("link unit lowers");

    assert!(
        module.functions.len() <= sico_ir::MAX_FUNCTIONS,
        "function count {} exceeds {}",
        module.functions.len(),
        sico_ir::MAX_FUNCTIONS
    );
    for (index, function) in module.functions.iter().enumerate() {
        let instructions: usize = function.blocks.iter().map(|b| b.instructions.len()).sum();
        assert!(
            function.locals.len() <= sico_ir::MAX_LOCALS_PER_FUNCTION,
            "function[{index}] carries {} locals, limit {}",
            function.locals.len(),
            sico_ir::MAX_LOCALS_PER_FUNCTION
        );
        assert!(
            function.blocks.len() <= sico_ir::MAX_BLOCKS_PER_FUNCTION,
            "function[{index}] carries {} blocks",
            function.blocks.len()
        );
        assert!(
            instructions <= sico_ir::MAX_INSTRUCTIONS_PER_FUNCTION,
            "function[{index}] carries {instructions} instructions"
        );
    }
    // The repair target itself: report the tightest function so the next
    // frontier extension knows how much headroom remains.
    let busiest = module
        .functions
        .iter()
        .map(|function| function.locals.len())
        .max()
        .unwrap_or(0);
    assert!(
        busiest + 16 <= sico_ir::MAX_LOCALS_PER_FUNCTION,
        "busiest function holds {busiest} locals; frontier work needs headroom"
    );
}
