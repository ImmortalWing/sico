use std::{fs, path::Path};

use sico_ir::{
    BlockId, CoreLowerError, Operation, Terminator, Type, VerifyErrorKind, canonical_json,
    lower_core, verify,
};
use sico_source::{SourceFile, SourceId};

const SUPPORTED: &[&str] = &[
    "CAP-002",
    "MATCH-001",
    "MATCH-002",
    "MATCH-003",
    "NOM-001",
    "NOM-002",
    "NOM-003",
    "NUM-001",
    "NUM-002",
    "NUM-003",
    "NUM-004",
    "RESULT-001",
];

#[test]
fn twelve_core_cases_lower_deterministically_and_remaining_valid_cases_refuse() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    collect_sico(&repository.join("syntax-candidates/b"), &mut paths);
    paths.sort();
    let mut lowered = 0;
    let mut snapshots = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let text = fs::read_to_string(path).unwrap();
        if !text.contains("// expect: accept") {
            continue;
        }
        let case = metadata(&text, "case").to_owned();
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(index).unwrap()),
            path.display().to_string(),
            text,
        )
        .unwrap();
        let result = lower_core(&source);
        if !SUPPORTED.contains(&case.as_str()) {
            assert!(
                matches!(result, Ok(_) | Err(CoreLowerError::Unsupported { .. })),
                "unexpected non-capability result for {case}: {result:?}"
            );
            continue;
        }
        match result {
            Ok(module) => {
                lowered += 1;
                assert!(verify(&module).is_empty());
                assert_eq!(module, lower_core(&source).unwrap());
                assert_eq!(
                    canonical_json(&module).unwrap(),
                    canonical_json(&module).unwrap()
                );
                snapshots.push(format!("{case}={}", shape(&module)));
            }
            Err(CoreLowerError::Unsupported { feature, range }) => {
                panic!("unexpected refusal for {case}: {feature} at {range:?}");
            }
            Err(error) => panic!("{case}: {error:?}"),
        }
    }
    assert_eq!(lowered, 12);
    snapshots.sort();
    let actual = format!("{}\n", snapshots.join("\n"));
    if std::env::var_os("SICO_DUMP_CORE_IR").is_some() {
        print!("{actual}");
    } else {
        assert_eq!(actual, include_str!("../../../tests/ir/core-lowering.snap"));
    }
}

#[test]
fn expression_operands_and_constructor_fields_lower_left_to_right_once() {
    let text = "record Pair:\n  field left: Int\n  field right: Int\nend record\n\nfunction main() returns Pair:\n  return Pair(left: 1 + 2, right: 3 + 4)\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "order.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    let operations: Vec<_> = module.functions[0].blocks[0]
        .instructions
        .iter()
        .map(|instruction| &instruction.operation)
        .collect();
    assert!(matches!(operations[0], Operation::ConstInt(value) if value == "1"));
    assert!(matches!(operations[1], Operation::ConstInt(value) if value == "2"));
    assert!(matches!(operations[2], Operation::AddInt { .. }));
    assert!(matches!(operations[3], Operation::ConstInt(value) if value == "3"));
    assert!(matches!(operations[4], Operation::ConstInt(value) if value == "4"));
    assert!(matches!(operations[5], Operation::AddInt { .. }));
    assert!(matches!(
        operations[6],
        Operation::Construct { name, fields }
            if name == "Pair"
                && fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>()
                    == ["left", "right"]
    ));

    let mut duplicate = module;
    let Operation::Construct { fields, .. } =
        &mut duplicate.functions[0].blocks[0].instructions[6].operation
    else {
        panic!("expected construct")
    };
    let duplicate_name = fields[0].name.clone();
    fields[1].name = duplicate_name;
    assert!(
        verify(&duplicate)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::TypeMismatch)
    );
}

#[test]
fn fixed_width_source_lowers_to_verified_typed_ir() {
    let text = "function signed_add(left: I64, right: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(left, right)\nend function\n\nfunction unsigned_sub(left: U64, right: U64) returns Result[U64, NumericError]:\n  return U64.checked_sub(left, right)\nend function\n\nfunction signed_less(left: I64, right: I64) returns Bool:\n  return I64.less_than(left, right)\nend function\n\nfunction unsigned_equal(left: U64, right: U64) returns Bool:\n  return U64.equal(left, right)\nend function\n\nfunction signed_literal() returns I64:\n  return I64.literal(-9223372036854775808)\nend function\n\nfunction unsigned_literal() returns U64:\n  return U64.literal(18446744073709551615)\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "fixed.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty());
    assert_eq!(module, lower_core(&source).unwrap());
    assert_eq!(module.functions[0].parameters[0].ty, Type::I64);
    assert!(matches!(
        module.functions[0].blocks[0].instructions[0].operation,
        Operation::CheckedAdd { .. }
    ));
    assert_eq!(
        module.functions[0].return_type,
        Type::Result {
            ok: Box::new(Type::I64),
            error: Box::new(Type::Named("NumericError".into())),
        }
    );
    assert!(matches!(
        module.functions[1].blocks[0].instructions[0].operation,
        Operation::CheckedSub { .. }
    ));
    assert!(matches!(
        module.functions[2].blocks[0].instructions[0].operation,
        Operation::LessFixed { .. }
    ));
    assert!(matches!(
        module.functions[3].blocks[0].instructions[0].operation,
        Operation::EqualFixed { .. }
    ));
    assert!(matches!(
        module.functions[4].blocks[0].instructions[0].operation,
        Operation::ConstI64(i64::MIN)
    ));
    assert!(matches!(
        module.functions[5].blocks[0].instructions[0].operation,
        Operation::ConstU64(u64::MAX)
    ));
}

#[test]
fn computed_checked_result_match_payloads_lower_to_verified_ir() {
    let text = "function add_or_zero(left: I64, right: I64) returns I64:\n  match I64.checked_add(left, right):\n    case ok(value):\n      return value\n    case error(reason):\n      return I64.literal(0)\n  end match\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "checked-match.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty());
    let function = &module.functions[0];
    assert_eq!(function.blocks.len(), 3);
    assert_eq!(function.locals.len(), 1);
    assert!(matches!(
        function.blocks[0].instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::CheckedAdd { .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::WriteLocal { local: 0, .. },
                ..
            }
        ]
    ));
    for arm in &function.blocks[1..] {
        assert!(matches!(
            arm.instructions
                .first()
                .map(|instruction| &instruction.operation),
            Some(Operation::ReadLocal { local: 0 })
        ));
        assert!(matches!(
            arm.instructions
                .get(1)
                .map(|instruction| &instruction.operation),
            Some(Operation::Project { .. })
        ));
    }
}

#[test]
fn straight_line_fixed_let_bindings_lower_to_ssa_values() {
    let text = "function constants() returns U64:\n  let signed = I64.literal(7)\n  let result = U64.literal(9)\n  return result\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "fixed-lets.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty());
    let block = &module.functions[0].blocks[0];
    assert!(matches!(
        block.instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::ConstI64(7),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ConstU64(9),
                ..
            }
        ]
    ));
    assert_eq!(
        block.terminator,
        Terminator::Return(Some(sico_ir::ValueId(1)))
    );
}

#[test]
fn straight_line_let_bindings_feed_fixed_and_checked_operations() {
    let text = "function mask() returns I64:\n  let left = I64.literal(7)\n  let right = I64.literal(3)\n  return I64.bit_and(left, right)\nend function\n\nfunction checked() returns Result[U64, NumericError]:\n  let left = U64.literal(8)\n  return U64.checked_div(left, U64.literal(2))\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "let-operands.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty());
    assert!(matches!(
        module.functions[0].blocks[0].instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::ConstI64(7),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ConstI64(3),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::BitAnd { .. },
                ..
            }
        ]
    ));
    assert!(matches!(
        module.functions[1].blocks[0].instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::ConstU64(8),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ConstU64(2),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::CheckedDiv { .. },
                ..
            }
        ]
    ));
}

#[test]
fn straight_line_let_bindings_feed_user_calls() {
    let text = "function pick(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction main() returns I64:\n  let left = I64.literal(5)\n  let right = I64.literal(6)\n  return pick(right, left)\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "let-call.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty());
    let instructions = &module.functions[1].blocks[0].instructions;
    assert!(matches!(
        instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::ConstI64(5),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ConstI64(6),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::Call { arguments, .. },
                ..
            }
        ] if arguments == &[sico_ir::ValueId(1), sico_ir::ValueId(0)]
    ));
}

#[test]
fn straight_line_alias_lets_reuse_existing_ssa_values() {
    let text = "function alias(value: I64) returns I64:\n  let copy = value\n  return copy\nend function\n\nfunction chain() returns I64:\n  let base = I64.literal(6)\n  let copy = base\n  return I64.bit_or(copy, I64.literal(1))\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "alias-lets.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty());
    assert!(module.functions[0].blocks[0].instructions.is_empty());
    assert_eq!(
        module.functions[0].blocks[0].terminator,
        Terminator::Return(Some(sico_ir::ValueId(0)))
    );
    assert!(matches!(
        module.functions[1].blocks[0].instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::ConstI64(6),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ConstI64(1),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::BitOr { .. },
                ..
            }
        ]
    ));
}

#[test]
fn straight_line_let_bindings_lower_operations_to_ssa_values() {
    let op_text = "function mask(a: I64, b: I64) returns I64:\n  let masked = I64.bit_and(a, b)\n  return masked\nend function\n";
    let op_source = SourceFile::from_text(SourceId::new(0), "let-op.sico", op_text).unwrap();
    let op_module = lower_core(&op_source).unwrap();
    assert!(verify(&op_module).is_empty());
    assert!(matches!(
        op_module.functions[0].blocks[0].instructions.as_slice(),
        [sico_ir::Instruction {
            operation: Operation::BitAnd { .. },
            ..
        }]
    ));
    assert_eq!(
        op_module.functions[0].blocks[0].terminator,
        Terminator::Return(Some(sico_ir::ValueId(2)))
    );

    let checked_text = "function kept(a: I64, b: I64) returns Result[I64, NumericError]:\n  let added = I64.checked_add(a, b)\n  return added\nend function\n";
    let checked_source =
        SourceFile::from_text(SourceId::new(0), "let-checked.sico", checked_text).unwrap();
    let checked_module = lower_core(&checked_source).unwrap();
    assert!(verify(&checked_module).is_empty());
    assert!(matches!(
        checked_module.functions[0].blocks[0]
            .instructions
            .as_slice(),
        [sico_ir::Instruction {
            operation: Operation::CheckedAdd { .. },
            ..
        }]
    ));

    let mismatch_text = "function bad(a: I64, b: I64) returns I64:\n  let added = I64.checked_add(a, b)\n  return added\nend function\n";
    let mismatch_source =
        SourceFile::from_text(SourceId::new(0), "let-mismatch.sico", mismatch_text).unwrap();
    assert!(lower_core(&mismatch_source).is_err());
}

#[test]
fn straight_line_let_bindings_lower_calls_to_ssa_values() {
    let call_text = "function add(a: I64, b: I64) returns I64:\n  return a\nend function\n\nfunction main() returns I64:\n  let total = add(I64.literal(3), I64.literal(4))\n  return total\nend function\n";
    let call_source =
        SourceFile::from_text(SourceId::new(0), "let-call-rhs.sico", call_text).unwrap();
    let call_module = lower_core(&call_source).unwrap();
    assert!(verify(&call_module).is_empty());
    assert!(matches!(
        call_module.functions[1].blocks[0].instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::ConstI64(3),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ConstI64(4),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::Call { arguments, .. },
                ..
            }
        ] if arguments == &[sico_ir::ValueId(0), sico_ir::ValueId(1)]
    ));
    assert_eq!(
        call_module.functions[1].blocks[0].terminator,
        Terminator::Return(Some(sico_ir::ValueId(2)))
    );

    let let_args_text = "function pick(left: I64, right: I64) returns I64:\n  return left\nend function\n\nfunction main(v: I64) returns I64:\n  let other = I64.literal(9)\n  let got = pick(v, other)\n  return got\nend function\n";
    let let_args_source =
        SourceFile::from_text(SourceId::new(0), "let-call-args.sico", let_args_text).unwrap();
    let let_args_module = lower_core(&let_args_source).unwrap();
    assert!(verify(&let_args_module).is_empty());
    assert!(matches!(
        let_args_module.functions[1].blocks[0].instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::ConstI64(9),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::Call { arguments, .. },
                ..
            }
        ] if arguments == &[sico_ir::ValueId(0), sico_ir::ValueId(1)]
    ));

    let result_call_text = "function make(a: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(a, I64.literal(1))\nend function\n\nfunction pass(a: I64) returns Result[I64, NumericError]:\n  let got = make(a)\n  return got\nend function\n";
    let result_call_source =
        SourceFile::from_text(SourceId::new(0), "let-result-call.sico", result_call_text).unwrap();
    let result_call_module = lower_core(&result_call_source).unwrap();
    assert!(verify(&result_call_module).is_empty());
    assert!(matches!(
        result_call_module.functions[1].blocks[0]
            .instructions
            .as_slice(),
        [sico_ir::Instruction {
            operation: Operation::Call { .. },
            ..
        }]
    ));
}

#[test]
fn straight_line_set_reassignment_lowers_to_local_cells() {
    let op_text = "function bump(a: I64, b: I64) returns I64:\n  let total = I64.literal(0)\n  set total = I64.bit_or(a, b)\n  return total\nend function\n";
    let op_source = SourceFile::from_text(SourceId::new(0), "set-op.sico", op_text).unwrap();
    let op_module = lower_core(&op_source).unwrap();
    assert!(verify(&op_module).is_empty());
    let block = &op_module.functions[0].blocks[0];
    assert_eq!(op_module.functions[0].blocks.len(), 1);
    assert_eq!(op_module.functions[0].locals.len(), 1);
    assert_eq!(op_module.functions[0].locals[0].name, "total");
    assert_eq!(op_module.functions[0].locals[0].ty, sico_ir::Type::I64);
    assert!(matches!(
        block.instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::ConstI64(0),
                ..
            },
            sico_ir::Instruction {
                operation: Operation::WriteLocal { local: 0, .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::BitOr { .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::WriteLocal { local: 0, .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ReadLocal { local: 0, .. },
                ..
            }
        ]
    ));
    assert_eq!(
        block.terminator,
        Terminator::Return(Some(sico_ir::ValueId(6)))
    );

    let cells_text = "function keep(a: I64) returns I64:\n  let copy = a\n  set copy = I64.bit_xor(copy, a)\n  let flag = copy\n  return flag\nend function\n";
    let cells_source =
        SourceFile::from_text(SourceId::new(0), "set-cells.sico", cells_text).unwrap();
    let cells_module = lower_core(&cells_source).unwrap();
    assert!(verify(&cells_module).is_empty());
    assert_eq!(cells_module.functions[0].locals.len(), 2);
    assert_eq!(cells_module.functions[0].locals[0].name, "copy");
    assert_eq!(cells_module.functions[0].locals[1].name, "flag");
    assert!(matches!(
        cells_module.functions[0].blocks[0].instructions.as_slice(),
        [
            sico_ir::Instruction {
                operation: Operation::WriteLocal { .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ReadLocal { .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::BitXor { .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::WriteLocal { .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ReadLocal { .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::WriteLocal { .. },
                ..
            },
            sico_ir::Instruction {
                operation: Operation::ReadLocal { .. },
                ..
            }
        ]
    ));

    let result_text = "function kept(a: I64, b: I64) returns Result[I64, NumericError]:\n  let saved = I64.checked_add(a, b)\n  set saved = I64.checked_sub(b, a)\n  return saved\nend function\n";
    let result_source =
        SourceFile::from_text(SourceId::new(0), "set-result.sico", result_text).unwrap();
    let result_module = lower_core(&result_source).unwrap();
    assert!(verify(&result_module).is_empty());
    assert_eq!(
        result_module.functions[0].locals[0].ty,
        sico_ir::Type::Result {
            ok: Box::new(sico_ir::Type::I64),
            error: Box::new(sico_ir::Type::Named("NumericError".into())),
        }
    );

    let missing_text = "function bump() returns I64:\n  set total = I64.literal(1)\n  return total\nend function\n";
    let missing_source =
        SourceFile::from_text(SourceId::new(0), "set-missing.sico", missing_text).unwrap();
    assert!(lower_core(&missing_source).is_err());

    let mismatch_text = "function bump() returns I64:\n  let x = I64.literal(3)\n  set x = true\n  return x\nend function\n";
    let mismatch_source =
        SourceFile::from_text(SourceId::new(0), "set-mismatch.sico", mismatch_text).unwrap();
    assert!(lower_core(&mismatch_source).is_err());
}

#[test]
fn general_cfg_if_regions_lower_to_branch_terminators() {
    let open_text = "function pick(a: I64, b: I64) returns I64:\n  if I64.equal(a, b):\n    return a\n  end if\n  return b\nend function\n";
    let open_source = SourceFile::from_text(SourceId::new(0), "if-open.sico", open_text).unwrap();
    let open_module = lower_core(&open_source).unwrap();
    assert!(verify(&open_module).is_empty());
    assert_eq!(open_module.functions[0].blocks.len(), 4);
    assert!(matches!(
        open_module.functions[0].blocks[0].terminator,
        Terminator::Branch {
            condition: sico_ir::ValueId(2),
            then_block: sico_ir::BlockId(1),
            else_block: sico_ir::BlockId(3),
        }
    ));
    assert_eq!(
        open_module.functions[0].blocks[1].terminator,
        Terminator::Return(Some(sico_ir::ValueId(0)))
    );
    assert_eq!(
        open_module.functions[0].blocks[2].terminator,
        Terminator::Return(Some(sico_ir::ValueId(1)))
    );
    assert_eq!(
        open_module.functions[0].blocks[3].terminator,
        Terminator::Jump(sico_ir::BlockId(2))
    );

    let else_text = "function choose(a: I64, b: I64, flag: Bool) returns I64:\n  if flag:\n    return a\n  else:\n    return b\n  end if\nend function\n";
    let else_source = SourceFile::from_text(SourceId::new(0), "if-else.sico", else_text).unwrap();
    let else_module = lower_core(&else_source).unwrap();
    assert!(verify(&else_module).is_empty());
    assert_eq!(else_module.functions[0].blocks.len(), 4);
    assert_eq!(
        else_module.functions[0].blocks[1].terminator,
        Terminator::Return(Some(sico_ir::ValueId(0)))
    );
    assert_eq!(
        else_module.functions[0].blocks[2].terminator,
        Terminator::Return(Some(sico_ir::ValueId(1)))
    );
    assert_eq!(
        else_module.functions[0].blocks[3].terminator,
        Terminator::Unreachable
    );

    let nested_text = "function grade(a: I64) returns I64:\n  let tag = I64.literal(0)\n  if I64.equal(a, I64.literal(0)):\n    set tag = I64.literal(1)\n  else:\n    if I64.less_than(a, I64.literal(0)):\n      set tag = I64.literal(2)\n    end if\n  end if\n  return tag\nend function\n";
    let nested_source =
        SourceFile::from_text(SourceId::new(0), "if-nested.sico", nested_text).unwrap();
    let nested_module = lower_core(&nested_source).unwrap();
    assert!(verify(&nested_module).is_empty());
    assert_eq!(nested_module.functions[0].blocks.len(), 7);
    let outer_else = match &nested_module.functions[0].blocks[0].terminator {
        Terminator::Branch { else_block, .. } => else_block.0,
        other => panic!("expected branch, got {other:?}"),
    };
    assert!(matches!(
        nested_module.functions[0].blocks[outer_else as usize].terminator,
        Terminator::Branch { .. }
    ));

    let non_bool_text = "function pick(a: I64, b: I64) returns I64:\n  if a:\n    return a\n  end if\n  return b\nend function\n";
    let non_bool_source =
        SourceFile::from_text(SourceId::new(0), "if-non-bool.sico", non_bool_text).unwrap();
    assert!(lower_core(&non_bool_source).is_err());
}

#[test]
fn general_cfg_while_loops_lower_to_header_branches_and_back_edges() {
    let climb_text = "function climb(a: I64) returns I64:\n  let x = I64.literal(0)\n  while I64.less_than(x, a):\n    set x = I64.bit_or(x, I64.literal(1))\n  end while\n  return x\nend function\n";
    let climb_source =
        SourceFile::from_text(SourceId::new(0), "while-climb.sico", climb_text).unwrap();
    let climb_module = lower_core(&climb_source).unwrap();
    assert!(verify(&climb_module).is_empty());
    assert_eq!(climb_module.functions[0].blocks.len(), 4);
    let climb_branch = match &climb_module.functions[0].blocks[1].terminator {
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => (*condition, *then_block, *else_block),
        other => panic!("expected branch, got {other:?}"),
    };
    // The header block carries the condition and branches body -> after.
    assert!(matches!(climb_branch.0, sico_ir::ValueId(4)));
    assert_eq!(climb_branch.1, sico_ir::BlockId(2));
    assert_eq!(climb_branch.2, sico_ir::BlockId(3));
    // The body's last instruction is the write; the block jumps back to the
    // header (the back edge).
    let climb_body = &climb_module.functions[0].blocks[2];
    assert!(matches!(
        climb_body.instructions.last().map(|i| &i.operation),
        Some(Operation::WriteLocal { .. })
    ));
    assert_eq!(climb_body.terminator, Terminator::Jump(sico_ir::BlockId(1)));
    assert_eq!(
        climb_module.functions[0].blocks[3].terminator,
        Terminator::Return(Some(sico_ir::ValueId(9)))
    );

    let break_text = "function stop(a: I64) returns I64:\n  let x = I64.literal(0)\n  while I64.less_than(x, a):\n    break\n  end while\n  return x\nend function\n";
    let break_source =
        SourceFile::from_text(SourceId::new(0), "while-break.sico", break_text).unwrap();
    let break_module = lower_core(&break_source).unwrap();
    assert!(verify(&break_module).is_empty());
    // break jumps to the after block and suppresses the body back edge.
    assert_eq!(
        break_module.functions[0].blocks[2].terminator,
        Terminator::Jump(sico_ir::BlockId(3))
    );

    let nested_text = "function grid(a: I64) returns I64:\n  let x = I64.literal(0)\n  while I64.less_than(x, a):\n    let y = I64.literal(0)\n    while I64.less_than(y, a):\n      set y = I64.bit_or(y, I64.literal(1))\n    end while\n    set x = I64.bit_or(x, I64.literal(1))\n  end while\n  return x\nend function\n";
    let nested_source =
        SourceFile::from_text(SourceId::new(0), "while-nested.sico", nested_text).unwrap();
    let nested_module = lower_core(&nested_source).unwrap();
    assert!(verify(&nested_module).is_empty());
    assert_eq!(nested_module.functions[0].blocks.len(), 7);
    assert!(matches!(
        nested_module.functions[0].blocks[0].terminator,
        Terminator::Jump(_)
    ));
    let outer_header = match &nested_module.functions[0].blocks[0].terminator {
        Terminator::Jump(target) => target.0,
        other => panic!("expected jump, got {other:?}"),
    };
    assert!(matches!(
        nested_module.functions[0].blocks[outer_header as usize].terminator,
        Terminator::Branch { .. }
    ));

    let stray_break_text = "function leave(a: I64) returns I64:\n  let x = I64.literal(1)\n  break\n  return x\nend function\n";
    let stray_break_source =
        SourceFile::from_text(SourceId::new(0), "while-stray-break.sico", stray_break_text)
            .unwrap();
    assert!(lower_core(&stray_break_source).is_err());

    let non_bool_text = "function count(a: I64) returns I64:\n  let x = I64.literal(0)\n  while a:\n    set x = I64.bit_or(x, I64.literal(1))\n  end while\n  return x\nend function\n";
    let non_bool_source =
        SourceFile::from_text(SourceId::new(0), "while-non-bool.sico", non_bool_text).unwrap();
    assert!(lower_core(&non_bool_source).is_err());
}

#[test]
fn verifier_rejects_fixed_width_operand_and_result_mutations() {
    let source = SourceFile::from_text(
        SourceId::new(0),
        "fixed.sico",
        "function add(left: I64, right: I64) returns Result[I64, NumericError]:\n  return I64.checked_add(left, right)\nend function\n",
    )
    .unwrap();
    let module = lower_core(&source).unwrap();

    let mut mixed = module.clone();
    mixed.functions[0].parameters[1].ty = Type::U64;
    assert!(
        verify(&mixed)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::TypeMismatch)
    );

    let mut wrong_result = module;
    wrong_result.functions[0].blocks[0].instructions[0].ty = Type::Result {
        ok: Box::new(Type::U64),
        error: Box::new(Type::Named("NumericError".into())),
    };
    assert!(
        verify(&wrong_result)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::TypeMismatch)
    );
}

#[test]
fn semantic_error_wins_before_backend_support_classification() {
    let source = SourceFile::from_text(
        SourceId::new(0),
        "invalid-async.sico",
        "async function bad() returns Int:\n  return \"wrong\"\nend function\n",
    )
    .unwrap();
    let CoreLowerError::Semantic(diagnostics) = lower_core(&source).unwrap_err() else {
        panic!("semantic gate must run first")
    };
    assert_eq!(diagnostics[0].code, "E2001");
}

#[test]
fn all_thirty_three_invalid_cases_stop_before_ir_support_dispatch() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let map: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repository.join("diagnostics/semantic-case-map.json")).unwrap(),
    )
    .unwrap();
    let expected: std::collections::BTreeMap<_, _> = map["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            (
                case["case"].as_str().unwrap().to_owned(),
                case["code"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let mut paths = Vec::new();
    collect_sico(&repository.join("syntax-candidates/b"), &mut paths);
    paths.sort();
    let mut rejected = 0;
    for (index, path) in paths.iter().enumerate() {
        let text = fs::read_to_string(path).unwrap();
        if !text.contains("// expect: reject") {
            continue;
        }
        rejected += 1;
        let case = metadata(&text, "case").to_owned();
        let source = SourceFile::from_text(
            SourceId::new(u32::try_from(index).unwrap()),
            path.display().to_string(),
            text,
        )
        .unwrap();
        let CoreLowerError::Semantic(diagnostics) = lower_core(&source).unwrap_err() else {
            panic!("{case} reached IR support dispatch")
        };
        assert_eq!(diagnostics.len(), 1, "{case}");
        assert_eq!(diagnostics[0].code, expected[case.as_str()], "{case}");
    }
    assert_eq!(rejected, 33);
}

#[test]
fn intrinsic_try_and_match_mutations_are_rejected_independently() {
    let intrinsic_source = SourceFile::from_text(
        SourceId::new(0),
        "intrinsic.sico",
        "function main() returns Float64:\n  return Float64.from_int(1)\nend function\n",
    )
    .unwrap();
    let mut intrinsic = lower_core(&intrinsic_source).unwrap();
    let Operation::Intrinsic { name, .. } =
        &mut intrinsic.functions[0].blocks[0].instructions[1].operation
    else {
        panic!("expected intrinsic")
    };
    *name = "unknown".into();
    assert!(
        verify(&intrinsic)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::UnknownTarget)
    );

    let match_source = SourceFile::from_text(
        SourceId::new(1),
        "match.sico",
        "enum Flag:\n  case On\n  case Off\nend enum\n\nfunction value(flag: Flag) returns Bool:\n  match flag:\n    case Flag.On:\n      return true\n    case Flag.Off:\n      return false\n  end match\nend function\n",
    )
    .unwrap();
    let mut matched = lower_core(&match_source).unwrap();
    let Terminator::Match { arms, .. } = &mut matched.functions[0].blocks[0].terminator else {
        panic!("expected match")
    };
    arms[0].target = BlockId(99);
    assert!(
        verify(&matched)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::UnknownTarget)
    );

    let mut wrong_pattern = lower_core(&match_source).unwrap();
    let Terminator::Match { arms, .. } = &mut wrong_pattern.functions[0].blocks[0].terminator
    else {
        panic!("expected match")
    };
    arms[0].patterns[0] = sico_ir::Pattern::Bool(true);
    assert!(
        verify(&wrong_pattern)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::TypeMismatch)
    );

    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let result_path =
        repository.join("syntax-candidates/b/result-mapping/valid/same-error-try.sico");
    let result_source = SourceFile::from_text(
        SourceId::new(2),
        result_path.display().to_string(),
        fs::read_to_string(result_path).unwrap(),
    )
    .unwrap();
    let mut result = lower_core(&result_source).unwrap();
    let try_instruction = result.functions[1].blocks[0]
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction.operation, Operation::Try(_)))
        .unwrap();
    try_instruction.ty = Type::Bool;
    assert!(
        verify(&result)
            .iter()
            .any(|error| error.kind == VerifyErrorKind::TypeMismatch)
    );
}

fn shape(module: &sico_ir::Module) -> String {
    module
        .functions
        .iter()
        .map(|function| {
            let blocks = function
                .blocks
                .iter()
                .map(|block| {
                    let operations = block
                        .instructions
                        .iter()
                        .map(|instruction| match &instruction.operation {
                            Operation::ConstInt(_) => "const-int",
                            Operation::ConstI64(_) => "const-i64",
                            Operation::ConstU64(_) => "const-u64",
                            Operation::ConstBool(_) => "const-bool",
                            Operation::ConstString(_) => "const-string",
                            Operation::ConstBytes(_) => "const-bytes",
                            Operation::Copy(_) => "copy",
                            Operation::ReadLocal { .. } => "read-local",
                            Operation::WriteLocal { .. } => "write-local",
                            Operation::AddInt { .. } => "add-int",
                            Operation::CheckedAdd { .. } => "checked-add",
                            Operation::CheckedSub { .. } => "checked-sub",
                            Operation::CheckedMul { .. } => "checked-mul",
                            Operation::CheckedDiv { .. } => "checked-div",
                            Operation::EqualFixed { .. } => "equal-fixed",
                            Operation::BitAnd { .. } => "bit-and",
                            Operation::BitOr { .. } => "bit-or",
                            Operation::BitXor { .. } => "bit-xor",
                            Operation::Shl { .. } => "shl",
                            Operation::Shr { .. } => "shr",
                            Operation::LessFixed { .. } => "less-fixed",
                            Operation::Call { .. } => "call",
                            Operation::Intrinsic { .. } => "intrinsic",
                            Operation::Construct { .. } => "construct",
                            Operation::Project { .. } => "project",
                            Operation::Variant { .. } => "variant",
                            Operation::EffectCall { .. } => "effect-call",
                            Operation::ResourceCall { .. } => "resource-call",
                            Operation::ResourceMove(_) => "resource-move",
                            Operation::ResourceBorrow(_) => "resource-borrow",
                            Operation::ResourceDrop(_) => "resource-drop",
                            Operation::RevisionCheck { .. } => "revision-check",
                            Operation::Try(_) => "try",
                            Operation::Await(_) => "await",
                            Operation::StreamNext(_) => "stream-next",
                            Operation::TaskScopeOpen { .. } => "task-scope-open",
                            Operation::TaskScopeClose { .. } => "task-scope-close",
                            Operation::Spawn { .. } => "spawn",
                            Operation::TaskCollect { .. } => "task-collect",
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let terminator = match block.terminator {
                        Terminator::Return(_) => "return",
                        Terminator::Jump(_) => "jump",
                        Terminator::Branch { .. } => "branch",
                        Terminator::Match { .. } => "match",
                        Terminator::Unreachable => "unreachable",
                    };
                    format!("b{}[{operations}]>{terminator}", block.id.0)
                })
                .collect::<Vec<_>>()
                .join(";");
            format!("f{}:{}({blocks})", function.id.0, function.name)
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn metadata<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("// {key}: ")))
        .unwrap()
}

fn collect_sico(root: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_sico(&path, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "sico")
        {
            output.push(path);
        }
    }
}
