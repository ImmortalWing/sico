//! Emitted JSON helpers for the STEP-0083 bounded `sico.json` contract:
//! whitespace skipping, strict string/number scanners, a depth-bounded
//! recursive validator, a top-level key finder and a string quoter. All
//! scanners return 0 on malformed input; depth is capped at 32.

use std::collections::BTreeMap;

use wasm_encoder::{BlockType, Function, Instruction, MemArg, ValType};

const MAX_JSON_DEPTH: i32 = 32;

fn mem(offset: u32, align: u32) -> MemArg {
    MemArg {
        offset: u64::from(offset),
        align,
        memory_index: 0,
    }
}

macro_rules! ops {
    ($body:ident; $($instruction:expr),+ $(,)?) => {
        $( $body.instruction(&$instruction); )+
    };
}

fn helper(helpers: &BTreeMap<&'static str, u32>, name: &'static str) -> u32 {
    helpers[name]
}

/// `(ptr) = ws(ptr, end)`: skip space/tab/LF/CR.
pub(crate) fn emit_ws() -> Function {
    let mut body = Function::new(vec![(1, ValType::I32)]);
    ops!(body;
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::LocalGet(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(0), Instruction::I32Load8U(mem(0, 0)), Instruction::LocalTee(2),
        Instruction::I32Const(32), Instruction::I32Eq,
        Instruction::LocalGet(2), Instruction::I32Const(9), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(2), Instruction::I32Const(10), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(2), Instruction::I32Const(13), Instruction::I32Eq, Instruction::I32Or,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(0),
        Instruction::Br(1),
        Instruction::End,
        Instruction::LocalGet(0), Instruction::Return, Instruction::End,
        Instruction::Unreachable, Instruction::End,
    );
    body
}

/// `(ptr|0) = string(ptr, end)`: strict string scanner with escape
/// validation; returns the position after the closing quote or 0.
#[allow(clippy::too_many_lines)]
pub(crate) fn emit_string() -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // locals: p(2), b(3)
    let hex = |body: &mut Function, offset: u32| {
        ops!(body;
            Instruction::LocalGet(2), Instruction::I32Load8U(mem(offset, 0)),
            Instruction::LocalTee(3),
            Instruction::I32Const(48), Instruction::I32Sub, Instruction::I32Const(10), Instruction::I32LtU,
            Instruction::LocalGet(3), Instruction::I32Const(32), Instruction::I32Or,
            Instruction::I32Const(97), Instruction::I32Sub, Instruction::I32Const(6), Instruction::I32LtU,
            Instruction::I32Or, Instruction::I32Eqz,
            Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        );
    };
    ops!(body;
        Instruction::LocalGet(0), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)), Instruction::LocalTee(3),
        Instruction::I32Const(0x22), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::Return,
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(0x5C), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)), Instruction::LocalTee(3),
        Instruction::I32Const(0x75), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(4), Instruction::I32Add,
        Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
    );
    hex(&mut body, 1);
    hex(&mut body, 2);
    hex(&mut body, 3);
    hex(&mut body, 4);
    ops!(body;
        Instruction::LocalGet(2), Instruction::I32Const(5), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::Else,
        Instruction::LocalGet(3), Instruction::I32Const(0x22), Instruction::I32Eq,
        Instruction::LocalGet(3), Instruction::I32Const(0x5C), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(0x2F), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(0x62), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(0x66), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(0x6E), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(0x72), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(0x74), Instruction::I32Eq, Instruction::I32Or,
        Instruction::I32Eqz,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Const(2), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::End,
        Instruction::Else,
        Instruction::LocalGet(3), Instruction::I32Const(0x20), Instruction::I32LtU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::End,
        Instruction::Br(0), Instruction::End,
        Instruction::Unreachable, Instruction::End,
    );
    body
}

/// `(ptr|0) = number(ptr, end)`: strict JSON number scanner.
#[allow(clippy::too_many_lines)]
pub(crate) fn emit_number() -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // locals: p(2), d(3)
    let is_digit = |body: &mut Function| {
        ops!(body;
            Instruction::LocalTee(3),
            Instruction::I32Const(48), Instruction::I32Sub, Instruction::I32Const(10), Instruction::I32LtU,
        );
    };
    let digits = |body: &mut Function| {
        ops!(body;
            Instruction::Block(BlockType::Empty),
            Instruction::Loop(BlockType::Empty),
            Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU, Instruction::BrIf(1),
            Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)),
        );
        is_digit(body);
        ops!(body;
            Instruction::I32Eqz, Instruction::BrIf(1),
            Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
            Instruction::Br(0), Instruction::End, Instruction::End,
        );
    };
    ops!(body;
        Instruction::LocalGet(0), Instruction::LocalSet(2),
        // optional minus
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x2D), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::End, Instruction::End,
        // integer part
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)), Instruction::LocalTee(3),
        Instruction::I32Const(0x30), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::Else,
        Instruction::LocalGet(3), Instruction::I32Const(0x31), Instruction::I32Sub,
        Instruction::I32Const(9), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
    );
    digits(&mut body);
    ops!(body;
        Instruction::Else,
        Instruction::I32Const(0), Instruction::Return,
        Instruction::End, Instruction::End,
        // fraction
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x2E), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)),
    );
    is_digit(&mut body);
    ops!(body;
        Instruction::I32Eqz,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
    );
    digits(&mut body);
    ops!(body;
        Instruction::End, Instruction::End,
        // exponent
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x20), Instruction::I32Or, Instruction::I32Const(0x65), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)), Instruction::LocalTee(3),
        Instruction::I32Const(0x2B), Instruction::I32Eq,
        Instruction::LocalGet(3), Instruction::I32Const(0x2D), Instruction::I32Eq, Instruction::I32Or,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Load8U(mem(0, 0)),
    );
    is_digit(&mut body);
    ops!(body;
        Instruction::I32Eqz,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
    );
    digits(&mut body);
    ops!(body;
        Instruction::End, Instruction::End,
        Instruction::LocalGet(2), Instruction::End,
    );
    body
}

/// `(ptr|0) = scan(ptr, end, depth)`: depth-bounded recursive JSON
/// validator; returns the position after the value or 0 for malformed input.
#[allow(clippy::too_many_lines)]
pub(crate) fn emit_scan(alloc: u32, helpers: &BTreeMap<&'static str, u32>) -> Function {
    let _ = alloc;
    let ws = helper(helpers, "sico.json.ws");
    let string = helper(helpers, "sico.json.string");
    let number = helper(helpers, "sico.json.number");
    let scan = helper(helpers, "sico.json.scan");
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // locals: p(3), b(4)
    let literal = |body: &mut Function, extra: i32, bytes: &[(u32, u32)]| {
        ops!(body;
            Instruction::LocalGet(3), Instruction::I32Const(extra), Instruction::I32Add,
            Instruction::LocalGet(1), Instruction::I32GtU,
            Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        );
        for (offset, byte) in bytes {
            ops!(body;
                Instruction::LocalGet(3), Instruction::I32Load8U(mem(*offset, 0)),
                Instruction::I32Const(i32::try_from(*byte).unwrap_or(0)), Instruction::I32Ne,
                Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
            );
        }
        ops!(body;
            Instruction::LocalGet(3), Instruction::I32Const(extra), Instruction::I32Add, Instruction::Return,
        );
    };
    ops!(body;
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::Call(ws), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Const(MAX_JSON_DEPTH), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Load8U(mem(0, 0)), Instruction::LocalSet(4),
        // string
        Instruction::LocalGet(4), Instruction::I32Const(0x22), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(string), Instruction::Return,
        Instruction::End,
        // object
        Instruction::LocalGet(4), Instruction::I32Const(0x7B), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(ws), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x7D), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::Return,
        Instruction::End, Instruction::End,
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(ws), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x22), Instruction::I32Ne,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(string), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(ws), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x3A), Instruction::I32Ne,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::Call(scan), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(ws), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Load8U(mem(0, 0)), Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::I32Const(0x2C), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::Br(1),
        Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(0x7D), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::Return,
        Instruction::End,
        Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::Unreachable, Instruction::End,
        // array
        Instruction::LocalGet(4), Instruction::I32Const(0x5B), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(ws), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x5D), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::Return,
        Instruction::End, Instruction::End,
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::Call(scan), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(ws), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Load8U(mem(0, 0)), Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::I32Const(0x2C), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::Br(1),
        Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(0x5D), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::Return,
        Instruction::End,
        Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::Unreachable, Instruction::End,
        // literals
        Instruction::LocalGet(4), Instruction::I32Const(0x74), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
    );
    literal(&mut body, 4, &[(1, 0x72), (2, 0x75), (3, 0x65)]);
    ops!(body;
        Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(0x66), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
    );
    literal(&mut body, 5, &[(1, 0x61), (2, 0x6C), (3, 0x73), (4, 0x65)]);
    ops!(body;
        Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(0x6E), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
    );
    literal(&mut body, 4, &[(1, 0x75), (2, 0x6C), (3, 0x6C)]);
    ops!(body;
        Instruction::End,
        // number
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::Call(number), Instruction::Return,
        Instruction::End,
    );
    body
}

/// `(ptr, len) = find(j_ptr, j_len, k_ptr, k_len)`: first top-level key
/// match in an object; `(0, 0)` when absent or malformed.
#[allow(clippy::too_many_lines)]
pub(crate) fn emit_find(alloc: u32, helpers: &BTreeMap<&'static str, u32>) -> Function {
    let _ = alloc;
    let ws = helper(helpers, "sico.json.ws");
    let string = helper(helpers, "sico.json.string");
    let scan = helper(helpers, "sico.json.scan");
    let mut body = Function::new(vec![(10, ValType::I32)]);
    // locals: end(4), p(5), key_start(6), key_end(7), v_start(8), v_end(9),
    // i(10), match(11), out_ptr(12), out_len(13)
    ops!(body;
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::LocalGet(0), Instruction::LocalGet(4), Instruction::Call(ws), Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(5), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x7B), Instruction::I32Ne,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::Call(ws), Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x7D), Instruction::I32Eq,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::End,
        Instruction::Block(BlockType::Empty), // $done
        Instruction::Loop(BlockType::Empty),  // $members
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::Call(ws), Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::I32GeU, Instruction::BrIf(1),
        Instruction::LocalGet(5), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x22), Instruction::I32Ne, Instruction::BrIf(1),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(6),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::Call(string), Instruction::LocalSet(7),
        Instruction::LocalGet(7), Instruction::I32Eqz, Instruction::BrIf(1),
        Instruction::LocalGet(7), Instruction::LocalSet(5),
        Instruction::LocalGet(7), Instruction::I32Const(-1), Instruction::I32Add, Instruction::LocalSet(7),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::Call(ws), Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::I32GeU, Instruction::BrIf(1),
        Instruction::LocalGet(5), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x3A), Instruction::I32Ne, Instruction::BrIf(1),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::Call(ws), Instruction::LocalSet(8),
        Instruction::LocalGet(8), Instruction::LocalGet(4), Instruction::I32Const(0),
        Instruction::Call(scan), Instruction::LocalSet(9),
        Instruction::LocalGet(9), Instruction::I32Eqz, Instruction::BrIf(1),
        // key match?
        Instruction::LocalGet(7), Instruction::LocalGet(6), Instruction::I32Sub,
        Instruction::LocalGet(3), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(1), Instruction::LocalSet(11),
        Instruction::I32Const(0), Instruction::LocalSet(10),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(10), Instruction::LocalGet(3), Instruction::I32GeU, Instruction::BrIf(1),
        Instruction::LocalGet(6), Instruction::LocalGet(10), Instruction::I32Add, Instruction::I32Load8U(mem(0, 0)),
        Instruction::LocalGet(2), Instruction::LocalGet(10), Instruction::I32Add, Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(11),
        Instruction::Br(2),
        Instruction::End,
        Instruction::LocalGet(10), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(10),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(11),
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(8), Instruction::LocalSet(12),
        Instruction::LocalGet(9), Instruction::LocalGet(8), Instruction::I32Sub, Instruction::LocalSet(13),
        Instruction::Br(3),
        Instruction::End, Instruction::End,
        // next member or close
        Instruction::LocalGet(9), Instruction::LocalGet(4), Instruction::Call(ws), Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::I32GeU, Instruction::BrIf(1),
        Instruction::LocalGet(5), Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Const(0x2C), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::Br(1),
        Instruction::End,
        Instruction::Br(1),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(12), Instruction::LocalGet(13), Instruction::End,
    );
    body
}

/// `(ptr, len) = quote(ptr, len)`: JSON string literal with escapes;
/// non-ASCII bytes pass through unchanged (valid JSON).
#[allow(clippy::too_many_lines)]
pub(crate) fn emit_quote(alloc: u32) -> Function {
    let mut body = Function::new(vec![(5, ValType::I32)]);
    // locals: i(2), b(3), out_len(4), out(5), dst(6)
    ops!(body;
        Instruction::I32Const(2), Instruction::LocalSet(4),
        // length pass
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU, Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::LocalTee(3),
        Instruction::I32Const(0x22), Instruction::I32Eq,
        Instruction::LocalGet(3), Instruction::I32Const(0x5C), Instruction::I32Eq, Instruction::I32Or,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::I32Const(2), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::Else,
        Instruction::LocalGet(3), Instruction::I32Const(0x20), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(8), Instruction::I32Eq,
        Instruction::LocalGet(3), Instruction::I32Const(9), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(10), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(12), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(13), Instruction::I32Eq, Instruction::I32Or,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::I32Const(2), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::Else,
        Instruction::LocalGet(4), Instruction::I32Const(6), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::End,
        Instruction::Else,
        Instruction::LocalGet(4), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::Br(0), Instruction::End, Instruction::End,
        // write pass
        Instruction::LocalGet(4), Instruction::Call(alloc), Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalSet(6),
        Instruction::LocalGet(6), Instruction::I32Const(0x22), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(6),
        Instruction::I32Const(0), Instruction::LocalSet(2),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU, Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::LocalTee(3),
        Instruction::I32Const(0x22), Instruction::I32Eq,
        Instruction::LocalGet(3), Instruction::I32Const(0x5C), Instruction::I32Eq, Instruction::I32Or,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(0x5C), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(6), Instruction::LocalGet(3), Instruction::I32Store8(mem(1, 0)),
        Instruction::LocalGet(6), Instruction::I32Const(2), Instruction::I32Add, Instruction::LocalSet(6),
        Instruction::Else,
        Instruction::LocalGet(3), Instruction::I32Const(0x20), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(0x5C), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(3), Instruction::I32Const(8), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(0x62), Instruction::I32Store8(mem(1, 0)),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(9), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(0x74), Instruction::I32Store8(mem(1, 0)),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(10), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(0x6E), Instruction::I32Store8(mem(1, 0)),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(12), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(0x66), Instruction::I32Store8(mem(1, 0)),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(13), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(0x72), Instruction::I32Store8(mem(1, 0)),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(8), Instruction::I32Eq,
        Instruction::LocalGet(3), Instruction::I32Const(9), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(10), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(12), Instruction::I32Eq, Instruction::I32Or,
        Instruction::LocalGet(3), Instruction::I32Const(13), Instruction::I32Eq, Instruction::I32Or,
        Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(0x75), Instruction::I32Store8(mem(1, 0)),
        Instruction::LocalGet(6), Instruction::I32Const(0x30), Instruction::I32Store8(mem(2, 0)),
        Instruction::LocalGet(6), Instruction::I32Const(0x30), Instruction::I32Store8(mem(3, 0)),
        Instruction::LocalGet(3), Instruction::I32Const(4), Instruction::I32ShrU,
        Instruction::I32Const(48), Instruction::I32Add,
        Instruction::LocalGet(6), Instruction::I32Store8(mem(4, 0)),
        Instruction::LocalGet(3), Instruction::I32Const(15), Instruction::I32And, Instruction::LocalTee(3),
        Instruction::I32Const(10), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(48), Instruction::I32Add,
        Instruction::Else,
        Instruction::LocalGet(3), Instruction::I32Const(55), Instruction::I32Add,
        Instruction::End,
        Instruction::LocalGet(6), Instruction::I32Store8(mem(5, 0)),
        Instruction::LocalGet(6), Instruction::I32Const(6), Instruction::I32Add, Instruction::LocalSet(6),
        Instruction::Else,
        Instruction::LocalGet(6), Instruction::I32Const(2), Instruction::I32Add, Instruction::LocalSet(6),
        Instruction::End,
        Instruction::End,
        Instruction::Else,
        Instruction::LocalGet(6), Instruction::LocalGet(3), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(6),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(6), Instruction::I32Const(0x22), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::End,
    );
    body
}
