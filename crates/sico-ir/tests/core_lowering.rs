use std::{fs, path::Path};

use sico_ir::{
    BlockId, CoreLowerError, EnumVariant, Instruction, Operation, Terminator, Type, ValueId,
    VerifyErrorKind, canonical_json, lower_core, verify,
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
        if path
            .to_string_lossy()
            .contains("zero-indent-recovery-declaration")
        {
            // IND-003 is a parse-level reject (MISSING_FUNCTION_CLOSE, E1001): it has no AST/HIR.
            assert!(matches!(
                lower_core(&source).unwrap_err(),
                CoreLowerError::Frontend(_)
            ));
            continue;
        }
        let CoreLowerError::Semantic(diagnostics) = lower_core(&source).unwrap_err() else {
            panic!("{case} reached IR support dispatch")
        };
        // IND-004/IND-005 (column-0 body lines) report the oracle code first but may carry follow-on diagnostics.
        if !case.starts_with("IND-") {
            assert_eq!(diagnostics.len(), 1, "{case}");
        }
        assert_eq!(diagnostics[0].code, expected[case.as_str()], "{case}");
    }
    assert_eq!(rejected, 38);
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

#[test]
fn option_some_none_constructors_and_arms_lower_to_verified_ir() {
    // RFC-0056 D4: `some(x)`/`none` mirror the ok/error machinery; the
    // producer/consumer here stays at IR level because the Script v0
    // function-signature boundary refuses Option-typed signatures (same
    // non-goal as user enums) — the guest-facing producer is the ui@1
    // `poll_event` intrinsic (G3-R1).
    let construct = "function probe() returns Option[I64]:\n  return some(I64.literal(3))\nend function\n\nfunction empty() returns Option[I64]:\n  return none\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "opt.sico", construct).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty(), "{:?}", verify(&module));
    let some_variant = module.functions[0].blocks[0]
        .instructions
        .iter()
        .find_map(|instruction| match &instruction.operation {
            Operation::Variant { name, payload } if name == "some" => Some(payload.clone()),
            _ => None,
        })
        .expect("some variant");
    assert_eq!(some_variant.len(), 1);
    assert!(matches!(
        module.functions[1].blocks[0].instructions.iter().find_map(
            |instruction| match &instruction.operation {
                Operation::Variant { name, .. } if name == "none" => Some(()),
                _ => None,
            }
        ),
        Some(())
    ));

    let matched = "function value(opt: Option[I64]) returns I64:\n  match opt:\n    case some(v):\n      return v\n    case none:\n      return I64.literal(0)\n  end match\nend function\n";
    let source = SourceFile::from_text(SourceId::new(1), "opt-match.sico", matched).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty(), "{:?}", verify(&module));
    let project = module.functions[0]
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match &instruction.operation {
            Operation::Project { field, .. } => Some(field.clone()),
            _ => None,
        })
        .expect("some payload projection");
    assert_eq!(project, "some");
}

#[test]
fn option_constructor_refusal_boundaries_have_stable_identities() {
    // Pipeline-level identities: return-position misuse is gated by
    // semantics (E2001) now that `some`/`none` infer proper Option types;
    // the lower `unconstrained some` refusal stands where semantics has no
    // expectation to check — the RFC-0049 let-bound match subject.
    for (source, expected) in [
        (
            "function probe() returns I64:\n  return some(I64.literal(3))\nend function\n",
            "E2001",
        ),
        (
            "function probe() returns I64:\n  return none\nend function\n",
            "E2001",
        ),
        (
            "function probe() returns Text:\n  let x = match some(\"hi\"):\n    case some(t):\n      t\n    case none:\n      \"empty\"\n  end match\n  return x\nend function\n",
            "unconstrained some",
        ),
    ] {
        let source = SourceFile::from_text(SourceId::new(0), "refusal.sico", source).unwrap();
        let error = lower_core(&source).expect_err("unconstrained constructor must refuse");
        let text = format!("{error:?}");
        assert!(
            text.contains(expected),
            "expected identity containing {expected:?}, got {text}"
        );
    }
}

#[test]
fn payload_binding_match_arms_lower_to_variant_projections() {
    let text = "enum Notice:\n  case Message(text: Text)\n  case Stop\nend enum\n\nfunction text_of(notice: Notice) returns Text:\n  match notice:\n    case Notice.Message(t):\n      return t\n    case Notice.Stop:\n      return \"stopped\"\n  end match\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "bind.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty(), "{:?}", verify(&module));
    let function = module
        .functions
        .iter()
        .find(|function| function.name == "text_of")
        .expect("text_of lowers");
    let match_block = function
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Terminator::Match { .. }))
        .expect("match block");
    let Terminator::Match { arms, .. } = &match_block.terminator else {
        unreachable!()
    };
    assert_eq!(arms.len(), 2);
    let bound = function
        .blocks
        .iter()
        .find(|block| {
            block
                .instructions
                .iter()
                .any(|instruction| matches!(instruction.operation, Operation::Project { .. }))
        })
        .expect("binding arm block");
    let project = bound
        .instructions
        .iter()
        .find_map(|instruction| match &instruction.operation {
            Operation::Project { base, field } => Some((*base, field.clone())),
            _ => None,
        })
        .expect("payload projection");
    assert_eq!(project.1, "Message");
    let project_instruction = bound
        .instructions
        .iter()
        .find(|instruction| matches!(instruction.operation, Operation::Project { .. }))
        .expect("projection instruction");
    assert_eq!(project_instruction.ty, Type::String);
    // The arm body returns the projected binding value.
    assert!(matches!(
        bound.terminator,
        Terminator::Return(Some(value)) if value == project_instruction.result
    ));
}

#[test]
fn multi_payload_binding_arms_use_indexed_projection_fields() {
    let text = "enum PairBox:\n  case Pair(left: U64, right: U64)\n  case Empty\nend enum\n\nfunction first(box: PairBox) returns U64:\n  match box:\n    case PairBox.Pair(a, b):\n      return a\n    case PairBox.Empty:\n      return U64.literal(0)\n  end match\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "multi.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty(), "{:?}", verify(&module));
    let function = module
        .functions
        .iter()
        .find(|function| function.name == "first")
        .expect("first lowers");
    let fields: Vec<_> = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.operation {
            Operation::Project { field, .. } => Some(field.clone()),
            _ => None,
        })
        .collect();
    // Both slots are named bindings, so both project — including the arm
    // body-unused `b` (the ok/error precedent emits its projection
    // unconditionally too; determinism is what matters).
    assert_eq!(fields, ["Pair.0", "Pair.1"]);
}

#[test]
fn payload_binding_refusal_boundaries_have_stable_identities() {
    // Arity mismatch and payload-less-case bindings refuse at lowering;
    // unqualified case names are gated earlier by semantics (E3001 sees a
    // one-segment pattern as a whole enum). Identities are pipeline-level:
    // whichever layer gates first owns the frozen text (RFC-0056 §6).
    for (source, expected) in [
        (
            "enum PairBox:\n  case Pair(left: U64, right: U64)\n  case Empty\nend enum\n\nfunction first(box: PairBox) returns U64:\n  match box:\n    case PairBox.Pair(a):\n      return a\n    case PairBox.Empty:\n      return U64.literal(0)\n  end match\nend function\n",
            "match payload binding arity",
        ),
        (
            "enum Notice:\n  case Message(text: Text)\n  case Stop\nend enum\n\nfunction text_of(notice: Notice) returns Text:\n  match notice:\n    case Message(t):\n      return t\n    case Notice.Stop:\n      return \"stopped\"\n  end match\nend function\n",
            "E3001",
        ),
        (
            "enum Notice:\n  case Message(text: Text)\n  case Stop\nend enum\n\nfunction text_of(notice: Notice) returns Text:\n  match notice:\n    case Notice.Stop(t):\n      return t\n    case Notice.Message(_):\n      return \"m\"\n  end match\nend function\n",
            "match payload binding arity",
        ),
    ] {
        let source = SourceFile::from_text(SourceId::new(0), "refusal.sico", source).unwrap();
        let error = lower_core(&source).expect_err("binding arm must refuse");
        let text = format!("{error:?}");
        assert!(
            text.contains(expected),
            "expected identity containing {expected:?}, got {text}"
        );
    }
}

#[test]
fn result_payload_binding_lowering_is_unchanged_by_the_generalization() {
    // Regression anchor: the ok/error path must emit byte-identical IR to
    // the pre-RFC-0056 shape (frozen corpus safety).
    let text = "function half(value: U64) returns U64:\n  match U64.checked_div(value, U64.literal(2)):\n    case ok(h):\n      return h\n    case error(_):\n      return U64.literal(0)\n  end match\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "half.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty(), "{:?}", verify(&module));
    let fields: Vec<_> = module.functions[0]
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.operation {
            Operation::Project { field, .. } => Some(field.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(fields, ["ok"]);
}

#[test]
fn payload_enum_declarations_enter_the_module_contract() {
    let text = "enum Notice:\n  case Message(text: Text)\n  case Stop\nend enum\n\nfunction main() returns Bool:\n  return true\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "enum.sico", text).unwrap();
    let module = lower_core(&source).unwrap();
    assert!(verify(&module).is_empty());
    assert_eq!(
        module.enums["Notice"],
        vec![
            EnumVariant {
                name: "Message".into(),
                payload: vec![Type::String],
            },
            EnumVariant {
                name: "Stop".into(),
                payload: vec![],
            },
        ]
    );
    // Enum-free programs keep the frozen serialization shape: the serde
    // default skips the empty table, so the canonical JSON is byte-identical
    // to a pre-RFC-0056 module (the RFC-0047 D4 `records` precedent).
    let plain_text = "function main() returns Bool:\n  return true\nend function\n";
    let plain = SourceFile::from_text(SourceId::new(1), "plain.sico", plain_text).unwrap();
    let plain = lower_core(&plain).unwrap();
    assert!(!canonical_json(&plain).unwrap().contains("\"enums\""));
}

#[test]
fn verifier_checks_enum_payload_projection_against_the_enum_table() {
    // The D5 host-surface shape, exercised at IR level: arity-1 uses the
    // bare case name, arity-N uses "<case>.<index>", and every projection
    // type is checked against the declared payload.
    let text = "enum ui_event:\n  case click(id: U64)\n  case select(handle: U64, index: U64)\n  case closed\nend enum\n\nfunction mk() returns ui_event:\n  return ui_event.click(U64.literal(7))\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "event.sico", text).unwrap();
    let mut module = lower_core(&source).unwrap();
    let mk = module
        .functions
        .iter_mut()
        .find(|function| function.name == "mk")
        .expect("mk lowers");
    let variant = mk.blocks[0]
        .instructions
        .iter()
        .find(|instruction| matches!(instruction.operation, Operation::Variant { .. }))
        .expect("variant construct")
        .clone();
    let mk_id = mk.id;
    // Canonical value ids: parameters first (none here), then instructions in
    // order — the projection takes the next id after the variant.
    let next = u32::try_from(mk.blocks[0].instructions.len()).unwrap();
    let project = |field: &str, ty: Type| Instruction {
        result: ValueId(next),
        ty,
        operation: Operation::Project {
            base: variant.result,
            field: field.to_owned(),
        },
        range: variant.range,
    };
    let mut checked = module.clone();
    {
        let mk = checked
            .functions
            .iter_mut()
            .find(|f| f.id == mk_id)
            .unwrap();
        mk.return_type = Type::U64;
        mk.blocks[0].instructions.push(project("click", Type::U64));
        mk.blocks[0].terminator = Terminator::Return(Some(ValueId(next)));
    }
    assert!(
        verify(&checked).is_empty(),
        "click payload projection: {:?}",
        verify(&checked)
    );

    for (field, ty) in [("select.0", Type::U64), ("select.1", Type::U64)] {
        let mut variant_check = module.clone();
        let mk = variant_check
            .functions
            .iter_mut()
            .find(|f| f.id == mk_id)
            .unwrap();
        mk.return_type = ty.clone();
        mk.blocks[0].instructions.push(project(field, ty));
        mk.blocks[0].terminator = Terminator::Return(Some(ValueId(next)));
        assert!(
            verify(&variant_check).is_empty(),
            "select payload projection {field}: {:?}",
            verify(&variant_check)
        );
    }

    // Refusals: unknown case, indexed access on an arity-1 payload, payload
    // index out of range, payload-less case, and wrong projection type.
    for (field, return_ty) in [
        ("Missing", Type::U64),
        ("click.0", Type::U64),
        ("select.2", Type::U64),
        ("closed", Type::Unit),
        ("click", Type::String),
        ("select.0x", Type::U64),
    ] {
        let mut rejected = module.clone();
        {
            let mk = rejected
                .functions
                .iter_mut()
                .find(|f| f.id == mk_id)
                .unwrap();
            mk.return_type = return_ty.clone();
            mk.blocks[0].instructions.push(project(field, return_ty));
            mk.blocks[0].terminator = Terminator::Return(Some(ValueId(next)));
        }
        assert!(
            verify(&rejected)
                .iter()
                .any(|error| error.kind == VerifyErrorKind::TypeMismatch),
            "expected TypeMismatch for {field}"
        );
    }
}

#[test]
fn record_field_projection_stays_unvalidated_outside_the_enum_table() {
    // Regression guard for the Project verifier arm: a Named base that is
    // not a declared enum (here: a record) keeps the pre-RFC-0056
    // name-unchecked behavior.
    let text = "record Pair:\n  field left: Int\n  field right: Int\nend record\n\nfunction main() returns Pair:\n  return Pair(left: 1, right: 2)\nend function\n";
    let source = SourceFile::from_text(SourceId::new(0), "pair.sico", text).unwrap();
    let mut module = lower_core(&source).unwrap();
    let construct = module.functions[0].blocks[0].instructions[2].clone();
    module.functions[0].blocks[0]
        .instructions
        .push(Instruction {
            result: ValueId(3),
            ty: Type::Int,
            operation: Operation::Project {
                base: construct.result,
                field: "left".to_owned(),
            },
            range: construct.range,
        });
    module.functions[0].blocks[0].terminator = Terminator::Return(Some(ValueId(3)));
    module.functions[0].return_type = Type::Int;
    assert!(
        verify(&module).is_empty(),
        "record projection must stay accepted: {:?}",
        verify(&module)
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
