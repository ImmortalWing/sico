//! Temporary diagnostic: dump the Rust canonical IR for a prefix source.

use sico_ir::{Terminator, canonical_json, lower_core};
use sico_source::{SourceFile, SourceId};

fn term_name(t: &Terminator) -> String {
    match t {
        Terminator::Return(v) => match v {
            Some(value) => format!("return {value:?}"),
            None => "return void".to_owned(),
        },
        Terminator::Jump(b) => format!("jump {}", b.0),
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => format!(
            "branch cond={:?} then={} else={}",
            condition, then_block.0, else_block.0
        ),
        Terminator::Match { .. } => "match".to_owned(),
        Terminator::Unreachable => "unreachable".to_owned(),
    }
}

#[test]
fn dump_oracle_ir() {
    let Ok(path) = std::env::var("ORACLE_SOURCE") else {
        // Passive without opt-in so the committed suite stays green; set
        // ORACLE_SOURCE (and optionally ORACLE_OUT / ORACLE_FN) to dump.
        return;
    };
    let text = std::fs::read_to_string(&path).expect("source readable");
    let source =
        SourceFile::from_text(SourceId::new(0), "selfhost-input.sico", &text).expect("bounded");
    let module = lower_core(&source).expect("Rust oracle lowers fixture");
    let json = canonical_json(&module).expect("canonical IR");
    let out = std::env::var("ORACLE_OUT").unwrap_or_else(|_| "oracle.json".to_owned());
    std::fs::write(out, json.as_bytes()).expect("write oracle");
    println!("functions={}", module.functions.len());
    let focus: Option<usize> = std::env::var("ORACLE_FN").ok().and_then(|v| v.parse().ok());
    for (index, function) in module.functions.iter().enumerate() {
        let summary: Vec<String> = function
            .blocks
            .iter()
            .map(|b| format!("{}:{}", b.id.0, short_name(&b.terminator)))
            .collect();
        println!(
            "fn[{index}] locals={} blocks={} entry={}",
            function.locals.len(),
            summary.join(" "),
            function.entry.0
        );
        if focus != Some(index) {
            continue;
        }
        for local in &function.locals {
            println!("  local {} {:?}", local.name, local.ty);
        }
        for block in &function.blocks {
            println!("  block {} range={:?}", block.id.0, block.range);
            for instruction in &block.instructions {
                let data = match &instruction.operation {
                    sico_ir::Operation::ConstInt(value) => format!("ConstInt({value})"),
                    op => format!("{:?}", op),
                };
                println!("    {} {:?} {}", instruction.result.0, instruction.ty, data);
            }
            println!("    -> {}", term_name(&block.terminator));
        }
    }
}

fn short_name(t: &Terminator) -> &'static str {
    match t {
        Terminator::Return(_) => "return",
        Terminator::Jump(_) => "jump",
        Terminator::Branch { .. } => "branch",
        Terminator::Match { .. } => "match",
        Terminator::Unreachable => "unreachable",
    }
}
