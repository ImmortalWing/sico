//! Emitted helper functions for the STEP-0083 Script standard-library
//! intrinsics. Every helper works on canonical `(ptr, len)` pairs inside the
//! single bounded arena; all size arithmetic flows through the checked
//! `$alloc` and hardware bounds checks fail closed.

use wasm_encoder::{BlockType, Function, Instruction, MemArg, ValType};

/// Core signature of one emitted helper.
pub(crate) fn helper_signature(name: &str) -> (Vec<ValType>, Vec<ValType>) {
    let i32s = |count: usize| vec![ValType::I32; count];
    match name {
        "sico.bytes.concat" | "sico.text.concat" | "sico.list.append" | "sico.text.join" => {
            (i32s(4), i32s(2))
        }
        "sico.json.find" => (i32s(4), i32s(2)),
        "sico.json.number"
        | "sico.json.ws"
        | "sico.json.string"
        | "sico.bytes.utf8_decode"
        | "sico.text.length" => (i32s(2), i32s(1)),
        "sico.json.quote"
        | "sico.text.split_lines"
        | "sico.text.split_words"
        | "sico.text.trim" => (i32s(2), i32s(2)),
        "sico.json.scan" => (vec![ValType::I32, ValType::I32, ValType::I32], i32s(1)),

        "sico.text.contains" | "sico.text.starts_with" => (i32s(4), i32s(1)),
        "sico.u64.to_text" | "sico.i64.to_text" => (vec![ValType::I64], i32s(2)),

        _ => unreachable!("helper signatures exist for every helper name"),
    }
}

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

/// Emits the helper body for `name`; `alloc` is the checked arena allocator
/// and `helpers` maps helper names to their core function indices for
/// cross-helper calls.
#[allow(clippy::too_many_lines)]
pub(crate) fn emit_helper(
    name: &str,
    alloc: u32,
    helpers: &std::collections::BTreeMap<&'static str, u32>,
) -> Function {
    match name {
        "sico.bytes.concat" | "sico.text.concat" => emit_concat(alloc),
        "sico.bytes.utf8_decode" => emit_utf8_validate(),
        "sico.text.length" => emit_text_length(),
        "sico.text.trim" => emit_trim(),
        "sico.text.contains" => emit_contains(),
        "sico.text.starts_with" => emit_starts_with(),
        "sico.u64.to_text" => emit_u64_to_text(alloc),
        "sico.i64.to_text" => emit_i64_to_text(alloc),
        "sico.list.append" => emit_list_append(alloc),
        "sico.json.find" => crate::json::emit_find(alloc, helpers),
        "sico.json.number" => crate::json::emit_number(),
        "sico.json.quote" => crate::json::emit_quote(alloc),
        "sico.json.scan" => crate::json::emit_scan(alloc, helpers),
        "sico.json.string" => crate::json::emit_string(),
        "sico.json.ws" => crate::json::emit_ws(),
        "sico.text.join" => emit_join(alloc),
        "sico.text.split_lines" => emit_split_lines(alloc),
        "sico.text.split_words" => emit_split_words(alloc),
        _ => unreachable!("helpers exist for every helper name"),
    }
}

/// `(out, total) = concat(a, b)`.
fn emit_concat(alloc: u32) -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    ops!(body;
        Instruction::LocalGet(1), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::LocalTee(5), Instruction::Call(alloc), Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::LocalGet(0), Instruction::LocalGet(1),
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::I32Add,
        Instruction::LocalGet(2), Instruction::LocalGet(3),
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::End,
    );
    body
}

/// Strict UTF-8 validation loop; returns 1 for valid, 0 for invalid.
#[allow(clippy::too_many_lines)]
fn emit_utf8_validate() -> Function {
    let mut body = Function::new(vec![(5, ValType::I32)]);
    // locals: result(2), i(3), byte(4), second(5), advance(6)
    // result starts as valid; any violation writes 0 and leaves the loop.
    ops!(body;
        Instruction::I32Const(1), Instruction::LocalSet(2),
        Instruction::Block(BlockType::Empty),   // $done (depth 3 from inner Ifs)
        Instruction::Loop(BlockType::Empty),    // $l
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::LocalSet(4),
        Instruction::Block(BlockType::Empty),   // $adv
        // ASCII
        Instruction::LocalGet(4), Instruction::I32Const(0x80), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(1), Instruction::LocalSet(6),
        Instruction::Br(1),
        Instruction::End,
        // 2-byte lead 0xC2..=0xDF
        Instruction::LocalGet(4), Instruction::I32Const(0xE0), Instruction::I32And,
        Instruction::I32Const(0xC0), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::I32Const(0xC2), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(1, 0)), Instruction::I32Const(0xC0),
        Instruction::I32And, Instruction::I32Const(0x80), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::I32Const(2), Instruction::LocalSet(6),
        Instruction::Br(1),
        Instruction::End,
        // 3-byte lead 0xE0..=0xEF
        Instruction::LocalGet(4), Instruction::I32Const(0xF0), Instruction::I32And,
        Instruction::I32Const(0xE0), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(2), Instruction::I32Add,
        Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(1, 0)), Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::I32Const(0xC0), Instruction::I32And,
        Instruction::I32Const(0x80), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(0xE0), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::I32Const(0xA0), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(0xED), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::I32Const(0xA0), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(2, 0)), Instruction::I32Const(0xC0),
        Instruction::I32And, Instruction::I32Const(0x80), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::I32Const(3), Instruction::LocalSet(6),
        Instruction::Br(1),
        Instruction::End,
        // 4-byte lead 0xF0..=0xF4
        Instruction::LocalGet(4), Instruction::I32Const(0xF8), Instruction::I32And,
        Instruction::I32Const(0xF0), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::I32Const(0xF4), Instruction::I32GtU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(3), Instruction::I32Add,
        Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(1, 0)), Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::I32Const(0xC0), Instruction::I32And,
        Instruction::I32Const(0x80), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(0xF0), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::I32Const(0x90), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(0xF4), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::I32Const(0x90), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(2, 0)), Instruction::I32Const(0xC0),
        Instruction::I32And, Instruction::I32Const(0x80), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(3, 0)), Instruction::I32Const(0xC0),
        Instruction::I32And, Instruction::I32Const(0x80), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2), Instruction::Br(4),
        Instruction::End,
        Instruction::I32Const(4), Instruction::LocalSet(6),
        Instruction::Br(1),
        Instruction::End,
        // any other lead byte is invalid
        Instruction::I32Const(0), Instruction::LocalSet(2),
        Instruction::Br(2),
        Instruction::End, // $adv
        Instruction::LocalGet(3), Instruction::LocalGet(6), Instruction::I32Add,
        Instruction::LocalSet(3),
        Instruction::Br(0),
        Instruction::End, // $l
        Instruction::End, // $done
        Instruction::LocalGet(2),
        Instruction::End,
    );
    body
}

/// Counts bytes that are not UTF-8 continuation bytes.
fn emit_text_length() -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // locals: i(2), count(3)
    ops!(body;
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::LocalGet(3), Instruction::Return, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::I32Const(0xC0),
        Instruction::I32And, Instruction::I32Const(0x80), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::End,
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::Br(0), Instruction::End, Instruction::Unreachable, Instruction::End,
    );
    body
}

/// Trims ASCII whitespace (space, tab, LF, CR) from both ends.
#[allow(clippy::too_many_lines)]
fn emit_trim() -> Function {
    let mut body = Function::new(vec![(3, ValType::I32)]);
    // locals: start(2), end(3), byte(4)
    let is_space = |body: &mut Function| {
        ops!(body;
            Instruction::LocalTee(4),
            Instruction::I32Const(32), Instruction::I32Eq,
            Instruction::LocalGet(4), Instruction::I32Const(9), Instruction::I32GeU,
            Instruction::LocalGet(4), Instruction::I32Const(13), Instruction::I32LeU,
            Instruction::I32And, Instruction::I32Or,
        );
    };
    ops!(body;
        Instruction::LocalGet(1), Instruction::LocalSet(3),
        // forward scan
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(3), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
    );
    is_space(&mut body);
    ops!(body;
        Instruction::I32Eqz, Instruction::BrIf(1),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::Br(0), Instruction::End, Instruction::End,
        // backward scan
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(2), Instruction::I32LeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Const(-1), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
    );
    is_space(&mut body);
    ops!(body;
        Instruction::I32Eqz, Instruction::BrIf(1),
        Instruction::LocalGet(3), Instruction::I32Const(-1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::LocalGet(3), Instruction::LocalGet(2), Instruction::I32Sub,
        Instruction::End,
    );
    body
}

/// Naive substring search; empty needle always matches.
#[allow(clippy::too_many_lines)]
fn emit_contains() -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // locals: i(4), j(5)
    ops!(body;
        Instruction::LocalGet(3), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty), Instruction::I32Const(1), Instruction::Return, Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GtU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::LocalGet(3), Instruction::I32Sub,
        Instruction::I32GtU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::I32Const(0), Instruction::LocalSet(5),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::LocalGet(3), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(1), Instruction::Return, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(4), Instruction::I32Add, Instruction::LocalGet(5),
        Instruction::I32Add, Instruction::I32Load8U(mem(0, 0)),
        Instruction::LocalGet(2), Instruction::LocalGet(5), Instruction::I32Add, Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Ne, Instruction::BrIf(1),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::Br(0), Instruction::End, Instruction::Unreachable, Instruction::End,
    );
    body
}

/// Prefix comparison.
fn emit_starts_with() -> Function {
    let mut body = Function::new(vec![(1, ValType::I32)]);
    // locals: i(4)
    ops!(body;
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GtU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::LocalGet(3), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(1), Instruction::Return, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(4), Instruction::I32Add, Instruction::I32Load8U(mem(0, 0)),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Add, Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Ne,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::Br(0), Instruction::End, Instruction::Unreachable, Instruction::End,
    );
    body
}

/// Decimal formatting for U64 into the arena.
fn emit_u64_to_text(alloc: u32) -> Function {
    let mut body = Function::new(vec![(4, ValType::I32), (1, ValType::I64)]);
    // locals: scratch(1), i(2), len(3), out(4), value(5 i64)
    ops!(body;
        Instruction::I32Const(24), Instruction::Call(alloc), Instruction::LocalSet(1),
        Instruction::LocalGet(0), Instruction::I64Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(1), Instruction::I32Const(48), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::Return,
        Instruction::End,
        Instruction::I32Const(24), Instruction::LocalSet(2),
        Instruction::LocalGet(0), Instruction::LocalSet(5),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::I64Eqz, Instruction::BrIf(1),
        Instruction::LocalGet(2), Instruction::I32Const(-1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::LocalGet(1), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::LocalGet(5), Instruction::I64Const(10), Instruction::I64RemU,
        Instruction::I32WrapI64, Instruction::I32Const(48), Instruction::I32Add,
        Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(5), Instruction::I64Const(10), Instruction::I64DivU, Instruction::LocalSet(5),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::I32Const(24), Instruction::LocalGet(2), Instruction::I32Sub, Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::Call(alloc), Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::LocalGet(3),
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(4), Instruction::LocalGet(3), Instruction::End,
    );
    body
}

/// Decimal formatting for I64 with sign handling (magnitude via bit tricks,
/// so `i64::MIN` is correct).
fn emit_i64_to_text(alloc: u32) -> Function {
    let mut body = Function::new(vec![(4, ValType::I32), (1, ValType::I64)]);
    // locals: scratch(1), i(2), len(3), out(4), magnitude(5 i64)
    ops!(body;
        Instruction::I32Const(24), Instruction::Call(alloc), Instruction::LocalSet(1),
        Instruction::LocalGet(0), Instruction::I64Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(1), Instruction::I32Const(48), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::Return,
        Instruction::End,
        Instruction::LocalGet(0), Instruction::I64Const(0), Instruction::I64LtS,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::I64Const(-1), Instruction::I64Xor,
        Instruction::I64Const(1), Instruction::I64Add, Instruction::LocalSet(5),
        Instruction::Else,
        Instruction::LocalGet(0), Instruction::LocalSet(5),
        Instruction::End,
        Instruction::I32Const(24), Instruction::LocalSet(2),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::I64Eqz, Instruction::BrIf(1),
        Instruction::LocalGet(2), Instruction::I32Const(-1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::LocalGet(1), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::LocalGet(5), Instruction::I64Const(10), Instruction::I64RemU,
        Instruction::I32WrapI64, Instruction::I32Const(48), Instruction::I32Add,
        Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(5), Instruction::I64Const(10), Instruction::I64DivU, Instruction::LocalSet(5),
        Instruction::Br(0), Instruction::End, Instruction::End,
        // negative: place the sign right before the digits
        Instruction::LocalGet(0), Instruction::I64Const(0), Instruction::I64LtS,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(-1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::LocalGet(1), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::I32Const(45), Instruction::I32Store8(mem(0, 0)),
        Instruction::End,
        Instruction::I32Const(24), Instruction::LocalGet(2), Instruction::I32Sub, Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::Call(alloc), Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::LocalGet(2), Instruction::I32Add,
        Instruction::LocalGet(3),
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(4), Instruction::LocalGet(3), Instruction::End,
    );
    body
}

/// `(table, count) = append(table, count, element ptr, element len)`.
fn emit_list_append(alloc: u32) -> Function {
    let mut body = Function::new(vec![(1, ValType::I32)]);
    // locals: new table(4)
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::I32Const(3), Instruction::I32Shl, Instruction::Call(alloc), Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::LocalGet(0), Instruction::LocalGet(1),
        Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Add, Instruction::LocalGet(2), Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Add, Instruction::LocalGet(3), Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::End,
    );
    body
}

/// `(out, total) = join(table, count, separator)`: total length pass, then a
/// copy pass interleaving the separator.
fn emit_join(alloc: u32) -> Function {
    let mut body = Function::new(vec![(6, ValType::I32)]);
    // locals: total(4), i(5), out(6), dst(7), elem_ptr(8), elem_len(9)
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::Call(alloc), Instruction::LocalSet(6),
        Instruction::LocalGet(6), Instruction::I32Const(0), Instruction::Return,
        Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32Const(1),
        Instruction::I32Sub, Instruction::I32Mul, Instruction::LocalSet(4),
        // length pass
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(4), Instruction::LocalGet(0), Instruction::LocalGet(5),
        Instruction::I32Const(3), Instruction::I32Shl, Instruction::I32Add,
        Instruction::I32Load(mem(4, 2)), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(4), Instruction::Call(alloc), Instruction::LocalSet(6),
        Instruction::LocalGet(6), Instruction::LocalSet(7),
        Instruction::I32Const(0), Instruction::LocalSet(5),
        // copy pass
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(0, 2)),
        Instruction::LocalSet(8),
        Instruction::LocalGet(0), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
        Instruction::LocalSet(9),
        Instruction::LocalGet(7), Instruction::LocalGet(8), Instruction::LocalGet(9),
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(7), Instruction::LocalGet(9), Instruction::I32Add, Instruction::LocalSet(7),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::LocalGet(2), Instruction::LocalGet(3),
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(7), Instruction::LocalGet(3), Instruction::I32Add, Instruction::LocalSet(7),
        Instruction::End,
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(6), Instruction::LocalGet(4), Instruction::End,
    );
    body
}

/// Splits on LF, strips a trailing CR per line, and drops the empty tail
/// after a final LF.
#[allow(clippy::too_many_lines)]
fn emit_split_lines(alloc: u32) -> Function {
    let mut body = Function::new(vec![(7, ValType::I32)]);
    // locals: count(2), i(3), table(4), out_index(5), line_start(6), line_len(7), byte(8)
    ops!(body;
        // count = (len > 0) ? 1 : 0, plus one per LF, minus one for a final LF
        Instruction::LocalGet(1), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(2),
        Instruction::Else,
        Instruction::I32Const(1), Instruction::LocalSet(2),
        Instruction::End,
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::I32Const(10), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(1), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::I32Add,
        Instruction::I32Const(-1), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::I32Const(10), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(-1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::End, Instruction::End,
        // table = alloc(count * 8)
        Instruction::LocalGet(2), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::Call(alloc), Instruction::LocalSet(4),
        Instruction::I32Const(0), Instruction::LocalSet(3),
        Instruction::LocalGet(0), Instruction::LocalSet(6),
        // fill pass
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::I32Const(10), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::Br(0),
        Instruction::End,
        // line = [line_start, ptr+i), strip a trailing CR
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::LocalGet(6), Instruction::I32Sub, Instruction::LocalSet(7),
        Instruction::LocalGet(7), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::LocalGet(7), Instruction::I32Add,
        Instruction::I32Const(-1), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::I32Const(13), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::I32Const(-1), Instruction::I32Add, Instruction::LocalSet(7),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::LocalGet(6),
        Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::LocalGet(7),
        Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(6),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::Br(0), Instruction::End, Instruction::End,
        // tail line (never empty: a final LF was excluded above)
        Instruction::LocalGet(6), Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::I32Add,
        Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::I32Add,
        Instruction::LocalGet(6), Instruction::I32Sub, Instruction::LocalSet(7),
        Instruction::LocalGet(7), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::LocalGet(7), Instruction::I32Add,
        Instruction::I32Const(-1), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::I32Const(13), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::I32Const(-1), Instruction::I32Add, Instruction::LocalSet(7),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::LocalGet(6),
        Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::LocalGet(7),
        Instruction::I32Store(mem(4, 2)),
        Instruction::End,
        Instruction::LocalGet(4), Instruction::LocalGet(2), Instruction::End,
    );
    body
}

/// Splits on ASCII-whitespace runs; empty tokens are never produced.
#[allow(clippy::too_many_lines)]
fn emit_split_words(alloc: u32) -> Function {
    let mut body = Function::new(vec![(7, ValType::I32)]);
    // locals: count(2), i(3), table(4), out_index(5), start(6), in_token(7), byte(8)
    let is_space = |body: &mut Function| {
        ops!(body;
            Instruction::LocalTee(8),
            Instruction::I32Const(32), Instruction::I32Eq,
            Instruction::LocalGet(8), Instruction::I32Const(9), Instruction::I32GeU,
            Instruction::LocalGet(8), Instruction::I32Const(13), Instruction::I32LeU,
            Instruction::I32And, Instruction::I32Or,
        );
    };
    ops!(body;
        // pass 1: count tokens
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
    );
    is_space(&mut body);
    ops!(body;
        Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(2),
        Instruction::I32Const(1), Instruction::LocalSet(7),
        Instruction::End,
        Instruction::Else,
        Instruction::I32Const(0), Instruction::LocalSet(7),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::Br(0), Instruction::End, Instruction::End,
        // table = alloc(count * 8)
        Instruction::LocalGet(2), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::Call(alloc), Instruction::LocalSet(4),
        Instruction::I32Const(0), Instruction::LocalSet(3),
        Instruction::I32Const(0), Instruction::LocalSet(7),
        // pass 2: record tokens
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
    );
    is_space(&mut body);
    ops!(body;
        Instruction::If(BlockType::Empty),
        // whitespace: close the open token if any
        Instruction::LocalGet(7), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(0), Instruction::LocalGet(6), Instruction::I32Add,
        Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(3), Instruction::LocalGet(6), Instruction::I32Sub,
        Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::I32Const(0), Instruction::LocalSet(7),
        Instruction::End,
        Instruction::Else,
        Instruction::LocalGet(7), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalSet(6),
        Instruction::I32Const(1), Instruction::LocalSet(7),
        Instruction::End,
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(3),
        Instruction::Br(0), Instruction::End, Instruction::End,
        // final token
        Instruction::LocalGet(7), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(0), Instruction::LocalGet(6), Instruction::I32Add,
        Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(1), Instruction::LocalGet(6), Instruction::I32Sub,
        Instruction::I32Store(mem(4, 2)),
        Instruction::End,
        Instruction::LocalGet(4), Instruction::LocalGet(2), Instruction::End,
    );
    body
}
