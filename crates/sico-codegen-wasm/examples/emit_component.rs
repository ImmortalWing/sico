use std::{env, fs, path::PathBuf};

use sico_codegen_wasm::compile_component;
use sico_source::{SourceFile, SourceId};

fn main() {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: emit_component <output.component.wasm>");
    let source = SourceFile::from_text(
        SourceId::new(0),
        "answer.sico",
        "function main() returns Int:\n  return 40 + 2\nend function\n",
    )
    .expect("probe source must be valid");
    let ir = sico_ir::lower_core(&source).expect("probe source must lower");
    let component = compile_component(&ir).expect("probe IR must componentize");
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("artifact directory must be creatable");
    }
    fs::write(&output, &component).expect("Component artifact must be writable");
    println!(
        "COMPONENT_EMITTED bytes={} path={}",
        component.len(),
        output.display()
    );
}
