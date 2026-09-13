//! RFC-0039 (STEP-0143): merged lowering of an entry source plus module
//! imports — globally unique function ids, qualified cross-module calls,
//! and byte-identical single-file behavior.

use sico_ir::{ModuleImport, lower_core, lower_core_modules};
use sico_source::{SourceFile, SourceId};

fn source(id: u32, name: &str, text: &str) -> SourceFile {
    SourceFile::from_text(SourceId::new(id), name.to_owned(), text.to_owned()).unwrap()
}

const ENTRY: &str = "function main() returns Int:\n  return math.double(21)\nend function\n";
const MATH: &str =
    "module math\n\nfunction double(x: Int) returns Int:\n  return x\nend function\n";

#[test]
fn cross_module_calls_lower_and_verify_with_unique_ids() {
    let entry = source(0, "main.sico", ENTRY);
    let math = source(1, "math.sico", MATH);
    let module = lower_core_modules(
        &entry,
        &[ModuleImport {
            name: "math",
            source: &math,
        }],
        &[],
    )
    .expect("merged module lowers");

    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["main", "math.double"]);

    let ids: Vec<u32> = module.functions.iter().map(|f| f.id.0).collect();
    assert_eq!(ids, [1, 2]);

    // The entry's call targets the imported function's global id.
    let call_targets: Vec<u32> = module
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| {
            block
                .instructions
                .iter()
                .filter_map(|instruction| match &instruction.operation {
                    sico_ir::Operation::Call { function, .. } => Some(function.0),
                    _ => None,
                })
        })
        .collect();
    assert_eq!(call_targets, [2]);
}

#[test]
fn merged_lowering_is_deterministic() {
    let entry = source(0, "main.sico", ENTRY);
    let math = source(1, "math.sico", MATH);
    let imports = [ModuleImport {
        name: "math",
        source: &math,
    }];
    let first = lower_core_modules(&entry, &imports, &[]).unwrap();
    let second = lower_core_modules(&entry, &imports, &[]).unwrap();
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

#[test]
fn single_file_lowering_is_unchanged_by_the_module_path() {
    let entry = source(
        0,
        "solo.sico",
        "function main() returns Int:\n  return 7\nend function\n",
    );
    let direct = lower_core(&entry).unwrap();
    let via_modules = lower_core_modules(&entry, &[], &[]).unwrap();
    assert_eq!(
        serde_json::to_string(&direct).unwrap(),
        serde_json::to_string(&via_modules).unwrap()
    );
}
