//! Emitted helper functions for the STEP-0083 Script standard-library
//! intrinsics. Every helper works on canonical `(ptr, len)` pairs inside the
//! single bounded arena; all size arithmetic flows through the checked
//! `$alloc` and hardware bounds checks fail closed.

use wasm_encoder::{BlockType, Function, Instruction, MemArg, ValType};

use sico_ir::{CollectionElement, CollectionIntrinsic, CollectionOperation};

/// Core signature of one emitted helper.
pub(crate) fn helper_signature(name: &str) -> (Vec<ValType>, Vec<ValType>) {
    if let Some(intrinsic) = sico_ir::collection_intrinsic(name) {
        return collection_helper_signature(intrinsic);
    }
    let i32s = |count: usize| vec![ValType::I32; count];
    match name {
        "sico.bytes.concat" | "sico.text.concat" | "sico.list.append" | "sico.text.join"
        | "sico.json.find" | "sico.text.format" | "sico.list.min" | "sico.list.max" => {
            (i32s(4), i32s(2))
        }
        "sico.json.number"
        | "sico.json.ws"
        | "sico.json.string"
        | "sico.bytes.utf8_decode"
        | "sico.text.length"
        | "sico.text.leading_spaces" => (i32s(2), i32s(1)),
        "sico.json.quote"
        | "sico.text.split_lines"
        | "sico.text.split_words"
        | "sico.text.trim"
        | "sico.list.sort" => (i32s(2), i32s(2)),
        "sico.json.scan" => (vec![ValType::I32, ValType::I32, ValType::I32], i32s(1)),

        "sico.text.contains"
        | "sico.text.starts_with"
        | "sico.text.ends_with"
        | "sico.bytes.equal" => (i32s(4), i32s(1)),
        "sico.u64.to_text" | "sico.i64.to_text" => (vec![ValType::I64], i32s(2)),

        // RFC-0045 stdlib batch 2 (STEP-0174).
        "sico.text.compare" => (i32s(4), vec![ValType::I64]),
        "sico.text.char_loc" => (i32s(3), i32s(2)),

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
    if let Some(intrinsic) = sico_ir::collection_intrinsic(name) {
        return emit_collection_helper(intrinsic, alloc);
    }
    // `list.min`/`list.max` share one body shape with a boolean flip, so
    // they are dispatched here instead of as duplicate match arms.
    if name == "sico.list.min" {
        return emit_list_extreme(helpers, true);
    }
    if name == "sico.list.max" {
        return emit_list_extreme(helpers, false);
    }
    match name {
        "sico.bytes.concat" | "sico.text.concat" => emit_concat(alloc),
        "sico.bytes.utf8_decode" => emit_utf8_validate(),
        "sico.text.length" => emit_text_length(),
        "sico.text.trim" => emit_trim(),
        "sico.text.contains" => emit_contains(),
        "sico.text.starts_with" => emit_starts_with(),
        "sico.text.ends_with" => emit_ends_with(),
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
        "sico.bytes.equal" => emit_bytes_equal(),
        "sico.text.compare" => emit_text_compare(),
        "sico.text.char_loc" => emit_char_loc(),
        "sico.text.format" => emit_format(alloc),
        "sico.list.sort" => emit_list_sort(alloc, helpers),
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
    let mut body = Function::new(vec![(5, ValType::I32)]);
    // locals: base(4), table(5), capacity(6), next count(7), old base(8).
    //
    // List values expose only `(table, count)`.  A table created here carries
    // a private 16-byte header immediately before the visible pointer:
    // magic, used version, capacity, magic2.  Appending the latest version may
    // fill an unobservable spare slot in place.  Appending an older alias
    // (`count != used`) takes the copy path, which preserves persistent-list
    // semantics while making the common linear builder path amortized O(n).
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(7),
        // Reuse only a table carrying both private header magics, and only
        // when the caller holds its latest logical version.
        Instruction::LocalGet(0), Instruction::I32Const(16), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::I32Const(16), Instruction::I32Sub,
        Instruction::LocalSet(8),
        Instruction::LocalGet(8), Instruction::I32Load(mem(0, 2)),
        Instruction::I32Const(0x5349_434f), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(8), Instruction::I32Load(mem(12, 2)),
        Instruction::I32Const(0x4c49_5354), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(8), Instruction::I32Load(mem(4, 2)),
        Instruction::LocalGet(1), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(8), Instruction::I32Load(mem(8, 2)),
        Instruction::LocalGet(1), Instruction::I32GtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(2), Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(3), Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(8), Instruction::LocalGet(7),
        Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(0), Instruction::LocalGet(7), Instruction::Return,
        Instruction::End, Instruction::End, Instruction::End,
        Instruction::End, Instruction::End,
        // Allocate a geometrically sized private table for a foreign table,
        // a full table, or an append from a stale alias.
        Instruction::I32Const(1), Instruction::LocalSet(6),
        Instruction::Block(BlockType::Empty), Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::LocalGet(7), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Shl,
        Instruction::LocalSet(6), Instruction::Br(0),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(6), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Const(16), Instruction::I32Add,
        Instruction::Call(alloc), Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::I32Const(0x5349_434f),
        Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(4), Instruction::LocalGet(7),
        Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(4), Instruction::LocalGet(6),
        Instruction::I32Store(mem(8, 2)),
        Instruction::LocalGet(4), Instruction::I32Const(0x4c49_5354),
        Instruction::I32Store(mem(12, 2)),
        Instruction::LocalGet(4), Instruction::I32Const(16), Instruction::I32Add,
        Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(0), Instruction::LocalGet(1),
        Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(5), Instruction::LocalGet(1), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Add, Instruction::LocalGet(2), Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(5), Instruction::LocalGet(1), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Add, Instruction::LocalGet(3), Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(5), Instruction::LocalGet(7),
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
        Instruction::LocalGet(1),
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
        Instruction::Br(1),
        Instruction::End,
        // line = [line_start, ptr+i), strip a trailing CR
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::LocalGet(6), Instruction::I32Sub, Instruction::LocalSet(7),
        Instruction::LocalGet(7),
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
        Instruction::LocalGet(7),
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
        // whitespace: close the open token if any (the branch fires when
        // in_token != 0)
        Instruction::LocalGet(7),
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
        // final token (write when the input ended inside a token)
        Instruction::LocalGet(7),
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

// ---------------------------------------------------------------------------
// STEP-0131 insertion-ordered Map/Set helpers.
//
// Runtime representation: a map/set value is the `(entries pointer, count)`
// pair, exactly like a `List[Text]` table. Each entry is one 8-byte key slot
// plus, for maps, one 8-byte value slot:
//
//   map entry  [key: 8 bytes][value: 8 bytes]
//   set entry  [key: 8 bytes]
//
// Key/value slot encodings: `Text`/`Bytes` are canonical `(ptr, len)` i32
// pairs; `Bool`/`I64`/`U64` are one i64. All operations are functional
// (copy-on-write): every mutation allocates a fresh entries table, so values
// stay valid and iteration order is first-insertion order by construction —
// a re-put keeps the original entry position and only overwrites the value
// slot inside the fresh copy. A set is a map without value slots; adding an
// existing key keeps the original table contents.
// ---------------------------------------------------------------------------

/// Byte width of one key slot (every element form is 8 bytes).
fn collection_slot_width(_element: CollectionElement) -> i32 {
    8
}

/// Byte width of one full entry (key + optional value).
fn collection_entry_width(intrinsic: CollectionIntrinsic) -> i32 {
    let mut width = collection_slot_width(intrinsic.key);
    if intrinsic.value.is_some() {
        width += collection_slot_width(intrinsic.key);
    }
    width
}

/// Whether this element is stored as a canonical `(ptr, len)` i32 pair.
fn collection_is_pair(element: CollectionElement) -> bool {
    matches!(element, CollectionElement::Text | CollectionElement::Bytes)
}

/// Core signature of one monomorphized collection helper.
fn collection_helper_signature(intrinsic: CollectionIntrinsic) -> (Vec<ValType>, Vec<ValType>) {
    use CollectionOperation::{
        ListAppend, ListEmpty, ListGet, ListLength, ListMax, ListMin, ListSort, MapEmpty, MapGet,
        MapHas, MapKeys, MapLength, MapPut, MapValues, SetAdd, SetEmpty, SetHas, SetLength,
        SetToList,
    };
    let key_params = || {
        if collection_is_pair(intrinsic.key) {
            vec![ValType::I32, ValType::I32]
        } else {
            vec![ValType::I64]
        }
    };
    match intrinsic.operation {
        MapEmpty | SetEmpty | ListEmpty => (vec![], vec![ValType::I32, ValType::I32]),
        MapPut => {
            let mut params = vec![ValType::I32, ValType::I32];
            params.extend(key_params());
            match intrinsic.value {
                Some(value) if collection_is_pair(value) => {
                    params.extend([ValType::I32, ValType::I32]);
                }
                Some(_) => params.push(ValType::I64),
                None => {}
            }
            (params, vec![ValType::I32, ValType::I32])
        }
        MapGet => {
            let mut params = vec![ValType::I32, ValType::I32];
            params.extend(key_params());
            (params, vec![ValType::I32, ValType::I64])
        }
        MapHas | SetHas => {
            let mut params = vec![ValType::I32, ValType::I32];
            params.extend(key_params());
            (params, vec![ValType::I32])
        }
        MapLength | SetLength | MapKeys | MapValues | SetToList | ListLength | ListSort => (
            vec![ValType::I32, ValType::I32],
            vec![ValType::I32, ValType::I32],
        ),
        // RFC-0046 D4 numeric list monomorphs: a list is the same
        // `(table i32, count i32)` pair regardless of element; scalar
        // elements ride one i64 slot.
        ListGet => (
            vec![ValType::I32, ValType::I32, ValType::I64],
            vec![ValType::I32, ValType::I64],
        ),
        ListAppend => (
            vec![ValType::I32, ValType::I32, ValType::I64],
            vec![ValType::I32, ValType::I32],
        ),
        ListMin | ListMax => (
            vec![ValType::I32, ValType::I32, ValType::I64],
            vec![ValType::I64],
        ),
        SetAdd => {
            let mut params = vec![ValType::I32, ValType::I32];
            params.extend(key_params());
            (params, vec![ValType::I32, ValType::I32])
        }
    }
}

/// Emits the monomorphized helper body for one collection intrinsic.
/// Parameter slots: table(0), count(1), key(2..), then the value for
/// `map.put`; scratch locals start after the widest parameter list (slot 6).
fn emit_collection_helper(intrinsic: CollectionIntrinsic, alloc: u32) -> Function {
    match intrinsic.operation {
        CollectionOperation::MapEmpty
        | CollectionOperation::SetEmpty
        | CollectionOperation::ListEmpty => emit_collection_empty(),
        CollectionOperation::MapPut => emit_map_put(intrinsic, alloc),
        CollectionOperation::MapGet => emit_map_get(intrinsic),
        CollectionOperation::MapHas | CollectionOperation::SetHas => emit_map_has(intrinsic),
        CollectionOperation::SetAdd => emit_set_add(intrinsic, alloc),
        CollectionOperation::MapLength
        | CollectionOperation::SetLength
        | CollectionOperation::ListLength => emit_collection_length(),
        CollectionOperation::MapKeys => emit_map_keys(alloc),
        CollectionOperation::MapValues => emit_map_values(alloc),
        CollectionOperation::SetToList => emit_set_to_list(),
        // RFC-0046 D4 (STEP-0175): numeric list monomorphs.
        CollectionOperation::ListGet => emit_list_get_scalar(),
        CollectionOperation::ListAppend => emit_list_append_scalar(alloc),
        CollectionOperation::ListSort => {
            emit_list_sort_scalar(alloc, intrinsic.key == CollectionElement::I64)
        }
        CollectionOperation::ListMin => {
            emit_list_extreme_scalar(true, intrinsic.key == CollectionElement::I64)
        }
        CollectionOperation::ListMax => {
            emit_list_extreme_scalar(false, intrinsic.key == CollectionElement::I64)
        }
    }
}

/// `() -> (0, 0)`: the empty map/set is the null table with count 0.
fn emit_collection_empty() -> Function {
    let mut body = Function::new(vec![]);
    ops!(body;
        Instruction::I32Const(0), Instruction::I32Const(0), Instruction::End,
    );
    body
}

/// `(table, count)` passthrough for `length`.
fn emit_collection_length() -> Function {
    let mut body = Function::new(vec![]);
    ops!(body;
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::End,
    );
    body
}

/// Number of flat parameters one element contributes (a `Text`/`Bytes`
/// pair is two i32s; a scalar is one i64).
fn element_param_count(element: CollectionElement) -> u32 {
    if collection_is_pair(element) { 2 } else { 1 }
}

/// Linear scan: pushes the index of the first entry whose key equals the
/// argument, or `count` when absent. `index` is the scratch local holding
/// the scan position; parameters are table(0), count(1), key(2..).
fn emit_find_entry(body: &mut Function, intrinsic: CollectionIntrinsic, index: u32) {
    let entry_width = collection_entry_width(intrinsic);
    let key = intrinsic.key;
    // verdict lives in local 9; the byte-loop cursor in local 8 (pair keys).
    ops!(body;
        Instruction::I32Const(0), Instruction::LocalSet(index),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(index), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
    );
    if collection_is_pair(key) {
        // Content equality: equal length and equal bytes. Pointer identity
        // would be a semantic trap — dynamic Text built at runtime never
        // shares addresses even when its contents match. The verdict is
        // computed into local 9 with flat, stack-neutral If arms; label
        // depths here (innermost first): byte-If(0), byte-Loop(1),
        // byte-Block(2), len-If(3), scan-Loop(4), scan-Block(5).
        ops!(body;
            Instruction::I32Const(1), Instruction::LocalSet(9),
            Instruction::LocalGet(3),
            Instruction::LocalGet(0), Instruction::LocalGet(index), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
            Instruction::I32Ne,
            Instruction::If(BlockType::Empty),
            Instruction::I32Const(0), Instruction::LocalSet(9),
            Instruction::Else,
            Instruction::I32Const(0), Instruction::LocalSet(8),
            Instruction::Block(BlockType::Empty),
            Instruction::Loop(BlockType::Empty),
            Instruction::LocalGet(8), Instruction::LocalGet(3), Instruction::I32GeU,
            Instruction::BrIf(1),
            Instruction::LocalGet(2), Instruction::LocalGet(8), Instruction::I32Add,
            Instruction::I32Load8U(mem(0, 0)),
            Instruction::LocalGet(0), Instruction::LocalGet(index), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I32Load(mem(0, 2)), Instruction::LocalGet(8), Instruction::I32Add,
            Instruction::I32Load8U(mem(0, 0)),
            Instruction::I32Ne,
            Instruction::If(BlockType::Empty),
            Instruction::I32Const(0), Instruction::LocalSet(9),
            Instruction::Br(2),
            Instruction::End,
            Instruction::LocalGet(8), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(8),
            Instruction::Br(0),
            Instruction::End, Instruction::End,
            Instruction::End,
        );
    } else {
        ops!(body;
            Instruction::LocalGet(0), Instruction::LocalGet(index), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I64Load(mem(0, 3)),
            Instruction::LocalGet(2), Instruction::I64Eq,
            Instruction::LocalSet(9),
        );
    }
    // A verdict of 1 exits the scan (index = match position); 0 continues.
    // Depths from here: scan-Loop(0), scan-Block(1).
    ops!(body;
        Instruction::LocalGet(9),
        Instruction::BrIf(1),
        Instruction::LocalGet(index), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(index),
        Instruction::Br(0),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(index),
    );
}

/// `(out_table, out_count) = put(table, count, key..., value...)`:
/// copy-on-put. Existing key: copy all entries, overwrite the value slot in
/// the copy at the original index (first-insertion order preserved). New
/// key: copy `count` entries then append one entry. Locals after params:
/// new, found, scan.
fn emit_map_put(intrinsic: CollectionIntrinsic, alloc: u32) -> Function {
    let entry_width = collection_entry_width(intrinsic);
    let key = intrinsic.key;
    let value = intrinsic.value.expect("map.put carries a value element");
    let params = 2 + element_param_count(key) + element_param_count(value);
    let new = params;
    let found = params + 1;
    let index = params + 2;
    let scratch = params + 3;
    let value_base = 2 + element_param_count(key);
    let locals_needed = scratch + 10;
    let mut body = Function::new(vec![(locals_needed, ValType::I32)]);
    // Always allocate room for one MORE entry than the source: a fresh map
    // (count 0) must not hand out a zero-size block whose contents a later
    // arena allocation would overwrite.
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::I32Const(entry_width), Instruction::I32Mul,
        Instruction::Call(alloc), Instruction::LocalSet(new),
        Instruction::LocalGet(new), Instruction::LocalGet(0), Instruction::LocalGet(1),
        Instruction::I32Const(entry_width), Instruction::I32Mul,
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
    );
    emit_find_entry(&mut body, intrinsic, index);
    ops!(body;
        Instruction::LocalSet(found),
        // found < count: overwrite the value slot inside the fresh copy
        Instruction::LocalGet(found), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(new), Instruction::LocalGet(found), Instruction::I32Const(entry_width), Instruction::I32Mul,
        Instruction::I32Add, Instruction::I32Const(8), Instruction::I32Add,
    );
    if collection_is_pair(value) {
        ops!(body;
            Instruction::LocalGet(value_base), Instruction::I32Store(mem(0, 2)),
            Instruction::LocalGet(new), Instruction::LocalGet(found), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I32Const(12), Instruction::I32Add,
            Instruction::LocalGet(value_base + 1), Instruction::I32Store(mem(0, 2)),
        );
    } else {
        ops!(body;
            Instruction::LocalGet(value_base), Instruction::I64Store(mem(0, 3)),
        );
    }
    ops!(body;
        Instruction::Else,
        // new key: write the appended entry past the copied prefix
        Instruction::LocalGet(new), Instruction::LocalGet(1), Instruction::I32Const(entry_width), Instruction::I32Mul,
        Instruction::I32Add,
    );
    if collection_is_pair(key) {
        ops!(body;
            Instruction::LocalGet(2), Instruction::I32Store(mem(0, 2)),
            Instruction::LocalGet(new), Instruction::LocalGet(1), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I32Const(4), Instruction::I32Add,
            Instruction::LocalGet(3), Instruction::I32Store(mem(0, 2)),
        );
    } else {
        ops!(body;
            Instruction::LocalGet(2), Instruction::I64Store(mem(0, 3)),
        );
    }
    if collection_is_pair(value) {
        ops!(body;
            Instruction::LocalGet(new), Instruction::LocalGet(1), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I32Const(8), Instruction::I32Add,
            Instruction::LocalGet(value_base), Instruction::I32Store(mem(0, 2)),
            Instruction::LocalGet(new), Instruction::LocalGet(1), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I32Const(12), Instruction::I32Add,
            Instruction::LocalGet(value_base + 1), Instruction::I32Store(mem(0, 2)),
        );
    } else {
        ops!(body;
            Instruction::LocalGet(new), Instruction::LocalGet(1), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I32Const(8), Instruction::I32Add,
            Instruction::LocalGet(value_base), Instruction::I64Store(mem(0, 3)),
        );
    }
    ops!(body;
        Instruction::End,
        // count stays on an overwrite and grows by one on an append; the
        // selection goes through a scratch local because both If arms must
        // be stack-neutral.
        Instruction::LocalGet(found), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(1), Instruction::LocalSet(scratch),
        Instruction::Else,
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(scratch),
        Instruction::End,
        Instruction::LocalGet(new), Instruction::LocalGet(scratch),
        Instruction::End,
    );
    body
}

/// `(out_table, out_count) = add(set, count, key...)`: copy-on-add; an
/// existing key returns the fresh copy with the unchanged count. Locals
/// after params: new, found, scan.
fn emit_set_add(intrinsic: CollectionIntrinsic, alloc: u32) -> Function {
    let entry_width = collection_entry_width(intrinsic);
    let key = intrinsic.key;
    let params = 2 + element_param_count(key);
    let new = params;
    let found = params + 1;
    let index = params + 2;
    let scratch = params + 3;
    let locals_needed = scratch + 10;
    let mut body = Function::new(vec![(locals_needed, ValType::I32)]);
    // As with put: always allocate room for one extra entry so an empty set
    // never starts from a zero-size block.
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::I32Const(entry_width), Instruction::I32Mul,
        Instruction::Call(alloc), Instruction::LocalSet(new),
        Instruction::LocalGet(new), Instruction::LocalGet(0), Instruction::LocalGet(1),
        Instruction::I32Const(entry_width), Instruction::I32Mul,
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
    );
    emit_find_entry(&mut body, intrinsic, index);
    ops!(body;
        Instruction::LocalSet(found),
        Instruction::LocalGet(found), Instruction::LocalGet(1), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(new), Instruction::LocalGet(1), Instruction::I32Const(entry_width), Instruction::I32Mul,
        Instruction::I32Add,
    );
    if collection_is_pair(key) {
        ops!(body;
            Instruction::LocalGet(2), Instruction::I32Store(mem(0, 2)),
            Instruction::LocalGet(new), Instruction::LocalGet(1), Instruction::I32Const(entry_width), Instruction::I32Mul,
            Instruction::I32Add, Instruction::I32Const(4), Instruction::I32Add,
            Instruction::LocalGet(3), Instruction::I32Store(mem(0, 2)),
        );
    } else {
        ops!(body;
            Instruction::LocalGet(2), Instruction::I64Store(mem(0, 3)),
        );
    }
    ops!(body;
        Instruction::End,
        // append grows the count; an existing key keeps it (stack-neutral
        // arms via the scratch local).
        Instruction::LocalGet(found), Instruction::LocalGet(1), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(scratch),
        Instruction::Else,
        Instruction::LocalGet(1), Instruction::LocalSet(scratch),
        Instruction::End,
        Instruction::LocalGet(new), Instruction::LocalGet(scratch),
        Instruction::End,
    );
    body
}

/// `(found) = has(table, count, key...)`. Scratch local after params: scan.
fn emit_map_has(intrinsic: CollectionIntrinsic) -> Function {
    let params = 2 + element_param_count(intrinsic.key);
    let index = params;
    let mut body = Function::new(vec![(index + 9, ValType::I32)]);
    emit_find_entry(&mut body, intrinsic, index);
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32LtU, Instruction::End,
    );
    body
}

/// `(tag, payload) = get(table, count, key...)`: found -> tag 0 and the
/// value slot; missing -> tag 1 with the overflow discriminant 0. Locals
/// after params: tag, payload, scan.
fn emit_map_get(intrinsic: CollectionIntrinsic) -> Function {
    let entry_width = collection_entry_width(intrinsic);
    let value = intrinsic.value.expect("map.get carries a value element");
    let params = 2 + element_param_count(intrinsic.key);
    let tag = params;
    let payload = params + 1;
    let index = params + 2;
    // tag and scan index are i32; only the payload is i64. Local groups
    // are declared in index order: tag, payload, index, then i32 scratch.
    let scratch_count = index + 10 - tag - 2;
    let mut body = Function::new(vec![
        (1, ValType::I32),
        (1, ValType::I64),
        (scratch_count, ValType::I32),
    ]);
    emit_find_entry(&mut body, intrinsic, index);
    ops!(body;
        Instruction::LocalSet(index),
        Instruction::LocalGet(index), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::LocalSet(tag),
        Instruction::LocalGet(0), Instruction::LocalGet(index), Instruction::I32Const(entry_width), Instruction::I32Mul,
        Instruction::I32Add, Instruction::I32Const(8), Instruction::I32Add,
    );
    if collection_is_pair(value) {
        ops!(body;
            Instruction::I32Load(mem(0, 2)), Instruction::I64ExtendI32U, Instruction::LocalSet(payload),
        );
    } else {
        ops!(body;
            Instruction::I64Load(mem(0, 3)), Instruction::LocalSet(payload),
        );
    }
    ops!(body;
        Instruction::Else,
        Instruction::I32Const(1), Instruction::LocalSet(tag),
        Instruction::I64Const(0), Instruction::LocalSet(payload),
        Instruction::End,
        Instruction::LocalGet(tag), Instruction::LocalGet(payload), Instruction::End,
    );
    body
}

/// `(out_table, out_count) = keys(table, count)`: a map entry is 16 bytes
/// (key slot + value slot) while a `List[Text]` table strides 8, so the key
/// pairs are compacted into a fresh table. Returning the entries table
/// directly would read the value slot of entry `i` as key `i + 1`. A
/// zero-count map allocates a zero-size block that is never stored to (the
/// copy loop exits immediately), matching the proven-empty split tables.
fn emit_map_keys(alloc: u32) -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // locals: i(2), new(3); entries stride 16, list slots stride 8.
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::Call(alloc), Instruction::LocalSet(3),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(3), Instruction::LocalGet(2), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Const(4),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(0, 2)),
        Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(3), Instruction::LocalGet(2), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Const(4),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
        Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(2),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::End,
    );
    body
}

/// `(out_table, out_count) = values(table, count)`: mirrors `keys`, but
/// copies the value slot (entries offset +8). The copy is two 32-bit loads
/// and stores, which is byte-identical for both slot widths: a `Text` value
/// slot is a `(ptr, len)` pair, a fixed-width value slot is one little-endian
/// 8-byte scalar — and the RFC-0046 D4 numeric list tables stride the same
/// 8-byte slots, so `map.values[Text,I64]` needs no separate emitter.
fn emit_map_values(alloc: u32) -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // locals: i(2), new(3); entries stride 16, list slots stride 8.
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::Call(alloc), Instruction::LocalSet(3),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(3), Instruction::LocalGet(2), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Const(4),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Const(8), Instruction::I32Add,
        Instruction::I32Load(mem(0, 2)),
        Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(3), Instruction::LocalGet(2), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32Const(4),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Const(8), Instruction::I32Add,
        Instruction::I32Load(mem(4, 2)),
        Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(2), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(2),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::End,
    );
    body
}

/// `(out_table, out_count) = to_list(table, count)`: a set entry IS one
/// 8-byte key slot, so the entries table already has the `List[Text]`
/// layout and the pair is returned unchanged — zero-copy is exact, not an
/// approximation. The key material lives in the same arena as the set, and
/// copy-on-write keeps the shared table immutable after creation.
fn emit_set_to_list() -> Function {
    let mut body = Function::new(vec![]);
    ops!(body;
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::End,
    );
    body
}

// ---------------------------------------------------------------------------
// RFC-0045 stdlib batch 2 (STEP-0174).
// ---------------------------------------------------------------------------

/// `(equal) = bytes.equal(a, a_len, b, b_len)`: length gate then a byte loop.
fn emit_bytes_equal() -> Function {
    let mut body = Function::new(vec![(1, ValType::I32)]);
    // local: i(4)
    ops!(body;
        Instruction::LocalGet(1), Instruction::LocalGet(3), Instruction::I32Ne,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::Return,
        Instruction::End,
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(1), Instruction::Return, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(4), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Ne,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(4), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(4),
        Instruction::Br(0), Instruction::End, Instruction::Unreachable, Instruction::End,
    );
    body
}

/// `(order) = text.compare(a, a_len, b, b_len)`: byte-order lexicographic,
/// `-1`/`0`/`+1` as i64. Shared by `list.sort`/`min`/`max` via cross-helper
/// calls, so its registration is forced through `helper_dependencies`.
fn emit_text_compare() -> Function {
    let mut body = Function::new(vec![(2, ValType::I32), (1, ValType::I64)]);
    // locals: min_len(4), i(5). Label depths from the loop body:
    // 0 = Loop (continue), 1 = Block (done).
    ops!(body;
        // min_len = min(a_len, b_len)
        Instruction::LocalGet(1), Instruction::LocalGet(3),
        Instruction::LocalGet(1), Instruction::LocalGet(3), Instruction::I32LeU,
        Instruction::Select,
        Instruction::LocalSet(4),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::LocalGet(4), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(5), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::LocalGet(2), Instruction::LocalGet(5), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32LtU,
        Instruction::If(BlockType::Empty), Instruction::I64Const(-1), Instruction::Return, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(5), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::LocalGet(2), Instruction::LocalGet(5), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32GtU,
        Instruction::If(BlockType::Empty), Instruction::I64Const(1), Instruction::Return, Instruction::End,
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::Br(0), Instruction::End, Instruction::End,
        // equal prefix: the shorter text sorts first
        Instruction::LocalGet(1), Instruction::LocalGet(3), Instruction::I32LtU,
        Instruction::If(BlockType::Empty), Instruction::I64Const(-1), Instruction::Return, Instruction::End,
        Instruction::LocalGet(1), Instruction::LocalGet(3), Instruction::I32GtU,
        Instruction::If(BlockType::Empty), Instruction::I64Const(1), Instruction::Return, Instruction::End,
        Instruction::I64Const(0), Instruction::End,
    );
    body
}

/// `(offset, char_len) = text.char_loc(ptr, len, char_index)`; `char_len`
/// 0 is the out-of-range sentinel (`Text` chars are never zero bytes).
/// Text values are canonically valid UTF-8 in-language, so lead-byte
/// classification needs no continuation validation here.
fn emit_char_loc() -> Function {
    let mut body = Function::new(vec![(4, ValType::I32)]);
    // locals: offset(3), count(4), byte(5), advance(6)
    let decode_advance = |body: &mut Function| {
        // byte in local 5; writes the advance into local 6.
        ops!(body;
            Instruction::LocalGet(5), Instruction::I32Const(0x80), Instruction::I32LtU,
            Instruction::If(BlockType::Empty),
            Instruction::I32Const(1), Instruction::LocalSet(6),
            Instruction::Else,
            Instruction::LocalGet(5), Instruction::I32Const(0xE0), Instruction::I32LtU,
            Instruction::If(BlockType::Empty),
            Instruction::I32Const(2), Instruction::LocalSet(6),
            Instruction::Else,
            Instruction::LocalGet(5), Instruction::I32Const(0xF0), Instruction::I32LtU,
            Instruction::If(BlockType::Empty),
            Instruction::I32Const(3), Instruction::LocalSet(6),
            Instruction::Else,
            Instruction::I32Const(4), Instruction::LocalSet(6),
            Instruction::End, Instruction::End, Instruction::End,
        );
    };
    ops!(body;
        Instruction::I32Const(0), Instruction::LocalSet(3),
        Instruction::I32Const(0), Instruction::LocalSet(4),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        // exhausted: index is beyond the char count
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(0), Instruction::I32Const(0), Instruction::Return,
        Instruction::End,
        // boundary reached: report (offset, char_len at offset)
        Instruction::LocalGet(4), Instruction::LocalGet(2), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::LocalSet(5),
    );
    decode_advance(&mut body);
    ops!(body;
        Instruction::LocalGet(3), Instruction::LocalGet(6), Instruction::Return,
        Instruction::End,
        // advance to the next boundary
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::LocalSet(5),
    );
    decode_advance(&mut body);
    ops!(body;
        Instruction::LocalGet(3), Instruction::LocalGet(6), Instruction::I32Add,
        Instruction::LocalSet(3),
        Instruction::LocalGet(4), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(4),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::Unreachable, Instruction::End,
    );
    body
}

/// `(out, written) = text.format(template, template_len, args, arg_count)`:
/// `{N}` inserts args[N]; `{{`/`}}` are literal-brace escapes; any other
/// brace sequence — unknown index, garbage, unterminated — is copied
/// verbatim (RFC-0045 D3, specified deterministic behavior).
///
/// Single pass into an upper-bound allocation (`template_len` plus every
/// argument length; placeholders never emit more than their argument, and
/// escapes never emit more than their two source bytes). The bounded arena
/// is reclaimed per call, so the slack costs address space only.
#[allow(clippy::too_many_lines)]
fn emit_format(alloc: u32) -> Function {
    let mut body = Function::new(vec![(10, ValType::I32)]);
    // params: template(0), template_len(1), args table(2), arg count(3)
    // locals: out(4), bound(5), i(6), j(7), n(8), byte(9), arg_ptr(10),
    //         arg_len(11), dst(12), seen_digit(13)
    //
    // Scan label depths: directly in the loop body 0 = Loop (continue),
    // 1 = Block (done); inside the brace-classification If 0 = If,
    // 1 = Loop, 2 = Block.
    ops!(body;
        // bound = template_len + sum of argument lengths
        Instruction::LocalGet(1), Instruction::LocalSet(5),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::LocalGet(3), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(5),
        Instruction::LocalGet(2), Instruction::LocalGet(7), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
        Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::LocalGet(7), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(7),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(5), Instruction::Call(alloc), Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::LocalSet(12),
        Instruction::I32Const(0), Instruction::LocalSet(6),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(6), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::LocalSet(9),
        // opening brace?
        Instruction::LocalGet(9), Instruction::I32Const(123), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        // peek t[i+1] guarded: one i32 leaves each arm
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Result(ValType::I32)),
        Instruction::LocalGet(0), Instruction::LocalGet(6), Instruction::I32Const(1),
        Instruction::I32Add, Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::Else,
        Instruction::I32Const(0),
        Instruction::End,
        Instruction::LocalSet(8),
        // `{{` escape
        Instruction::LocalGet(8), Instruction::I32Const(123), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(12), Instruction::I32Const(123), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(12), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(12),
        Instruction::LocalGet(6), Instruction::I32Const(2), Instruction::I32Add,
        Instruction::LocalSet(6),
        Instruction::Br(2),
        Instruction::End,
        // `{N}` placeholder parse: branch-free verdict into local 13
        Instruction::I32Const(0), Instruction::LocalSet(8),
        Instruction::I32Const(0), Instruction::LocalSet(13),
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(7),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(7), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::LocalSet(9),
        Instruction::LocalGet(9), Instruction::I32Const(48), Instruction::I32LtU,
        Instruction::LocalGet(9), Instruction::I32Const(57), Instruction::I32GtU,
        Instruction::I32Or,
        Instruction::BrIf(1),
        Instruction::LocalGet(8), Instruction::I32Const(100_000), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(-1), Instruction::LocalSet(8),
        Instruction::Br(2),
        Instruction::End,
        Instruction::LocalGet(8), Instruction::I32Const(10), Instruction::I32Mul,
        Instruction::LocalGet(9), Instruction::I32Const(48), Instruction::I32Sub,
        Instruction::I32Add, Instruction::LocalSet(8),
        Instruction::I32Const(1), Instruction::LocalSet(13),
        Instruction::LocalGet(7), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(7),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(13),
        Instruction::LocalGet(7), Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::I32And,
        Instruction::LocalGet(0), Instruction::LocalGet(7), Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)), Instruction::I32Const(125), Instruction::I32Eq,
        Instruction::I32And,
        Instruction::LocalGet(8), Instruction::I32Const(-1), Instruction::I32Ne,
        Instruction::I32And,
        Instruction::LocalGet(8), Instruction::LocalGet(3), Instruction::I32LtU,
        Instruction::I32And,
        Instruction::LocalSet(13),
        Instruction::LocalGet(13),
        Instruction::If(BlockType::Empty),
        // copy the referenced argument
        Instruction::LocalGet(2), Instruction::LocalGet(8), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(0, 2)),
        Instruction::LocalSet(10),
        Instruction::LocalGet(2), Instruction::LocalGet(8), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
        Instruction::LocalSet(11),
        Instruction::LocalGet(12), Instruction::LocalGet(10), Instruction::LocalGet(11),
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(12), Instruction::LocalGet(11), Instruction::I32Add,
        Instruction::LocalSet(12),
        Instruction::LocalGet(7), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(6),
        Instruction::Br(2),
        Instruction::End,
        // verbatim `{`
        Instruction::LocalGet(12), Instruction::I32Const(123), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(12), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(12),
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(6),
        Instruction::Br(1),
        Instruction::End,
        // closing brace escape?
        Instruction::LocalGet(9), Instruction::I32Const(125), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalGet(1), Instruction::I32LtU,
        Instruction::If(BlockType::Result(ValType::I32)),
        Instruction::LocalGet(0), Instruction::LocalGet(6), Instruction::I32Const(1),
        Instruction::I32Add, Instruction::I32Add,
        Instruction::I32Load8U(mem(0, 0)),
        Instruction::Else,
        Instruction::I32Const(0),
        Instruction::End,
        Instruction::LocalSet(8),
        Instruction::LocalGet(8), Instruction::I32Const(125), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(12), Instruction::I32Const(125), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(12), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(12),
        Instruction::LocalGet(6), Instruction::I32Const(2), Instruction::I32Add,
        Instruction::LocalSet(6),
        Instruction::Br(2),
        Instruction::End,
        Instruction::End,
        // plain literal byte
        Instruction::LocalGet(12), Instruction::LocalGet(9), Instruction::I32Store8(mem(0, 0)),
        Instruction::LocalGet(12), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(12),
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(6),
        Instruction::Br(0), Instruction::End, Instruction::End,
        // (out, written) — written is the true emitted length
        Instruction::LocalGet(4), Instruction::LocalGet(12), Instruction::LocalGet(4),
        Instruction::I32Sub, Instruction::End,
    );
    body
}
/// `(table, count) = list.sort(table, count)`: stable insertion sort into a
/// fresh table, byte-order ascending via the shared `sico.text.compare`
/// helper (registered through `helper_dependencies`).
#[allow(clippy::too_many_lines)]
fn emit_list_sort(alloc: u32, helpers: &std::collections::BTreeMap<&'static str, u32>) -> Function {
    let compare = helpers["sico.text.compare"];
    let mut body = Function::new(vec![(9, ValType::I32), (1, ValType::I64)]);
    // params: table(0), count(1)
    // i32 locals: new(2), i(3), j(4), key_ptr(5), key_len(6), prev_ptr(7),
    //             prev_len(8), tmp_ptr(9), tmp_len(10)
    // i64 local: order(11)
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::Call(alloc), Instruction::LocalSet(2),
        Instruction::LocalGet(2), Instruction::LocalGet(0), Instruction::LocalGet(1),
        Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(1), Instruction::I32Const(2), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::Return,
        Instruction::End,
        Instruction::I32Const(1), Instruction::LocalSet(3),
        // outer: for i in 1..count
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(2), Instruction::LocalGet(3), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(0, 2)),
        Instruction::LocalSet(5),
        Instruction::LocalGet(2), Instruction::LocalGet(3), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
        Instruction::LocalSet(6),
        Instruction::LocalGet(3), Instruction::LocalSet(4),
        // inner: shift larger predecessors right
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::I32Eqz,
        Instruction::BrIf(1),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(1),
        Instruction::I32Sub, Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Add, Instruction::I32Load(mem(0, 2)),
        Instruction::LocalSet(7),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(1),
        Instruction::I32Sub, Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
        Instruction::LocalSet(8),
        Instruction::LocalGet(7), Instruction::LocalGet(8),
        Instruction::LocalGet(5), Instruction::LocalGet(6),
        Instruction::Call(compare),
        Instruction::LocalSet(11),
        // order > 0 -> shift; otherwise the insertion point is found
        Instruction::LocalGet(11), Instruction::I64Const(0), Instruction::I64GtS,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(0, 2)),
        Instruction::LocalSet(9),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
        Instruction::LocalSet(10),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(7), Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(8), Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(4), Instruction::I32Const(-1), Instruction::I32Add,
        Instruction::LocalSet(4),
        // continue the inner scan: from inside the If the inner Loop is
        // depth 1 (the If itself is depth 0)
        Instruction::Br(1),
        Instruction::End,
        Instruction::Br(1),
        Instruction::End, Instruction::End,
        // slot[j] = key
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(5), Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(6), Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(3),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::End,
    );
    body
}

/// `(ptr, len) = list.min|max(table, count, default_ptr, default_len)`:
/// the extreme element, or the default when the list is empty. Total
/// function by RFC-0045 D2 — no error ceremony.
fn emit_list_extreme(
    helpers: &std::collections::BTreeMap<&'static str, u32>,
    is_min: bool,
) -> Function {
    let compare = helpers["sico.text.compare"];
    let mut body = Function::new(vec![(5, ValType::I32), (1, ValType::I64)]);
    // params: table(0), count(1), default_ptr(2), default_len(3)
    // locals: best_ptr(4), best_len(5), i(6), cur_ptr(7), cur_len(8); order(9 i64)
    ops!(body;
        // empty list: return the default argument
        Instruction::LocalGet(1), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(3), Instruction::Return,
        Instruction::End,
        Instruction::LocalGet(0), Instruction::I32Load(mem(0, 2)),
        Instruction::LocalSet(4),
        Instruction::LocalGet(0), Instruction::I32Load(mem(4, 2)),
        Instruction::LocalSet(5),
        Instruction::I32Const(1), Instruction::LocalSet(6),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(6), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(6), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(0, 2)),
        Instruction::LocalSet(7),
        Instruction::LocalGet(0), Instruction::LocalGet(6), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I32Load(mem(4, 2)),
        Instruction::LocalSet(8),
        Instruction::LocalGet(4), Instruction::LocalGet(5),
        Instruction::LocalGet(7), Instruction::LocalGet(8),
        Instruction::Call(compare),
        Instruction::LocalSet(9),
        // min: replace the best when order > 0; max: when order < 0
    );
    if is_min {
        ops!(body;
            Instruction::LocalGet(9), Instruction::I64Const(0), Instruction::I64GtS,
        );
    } else {
        ops!(body;
            Instruction::LocalGet(9), Instruction::I64Const(0), Instruction::I64LtS,
        );
    }
    ops!(body;
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::LocalSet(4),
        Instruction::LocalGet(8), Instruction::LocalSet(5),
        Instruction::End,
        Instruction::LocalGet(6), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(6),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(4), Instruction::LocalGet(5), Instruction::End,
    );
    body
}

// ---------------------------------------------------------------------------
// RFC-0046 D4 numeric list monomorphs (STEP-0175). A `List[I64]`/`List[U64]`
// table strides 8 bytes per slot holding the little-endian value itself —
// the same slot width map/set scalar entries use, so `map.values[Text,I64]`
// copies are byte-layout-identical. Ordering comparisons are signed for
// `I64`, unsigned for `U64` (numeric order, matching `I64.less_than`).
// ---------------------------------------------------------------------------

/// `(tag, value) = list.get(table, count, index)`: the packed two-slot
/// `Result[I64|U64, NumericError]` shape (tag 0 + payload, or tag 1 + 0).
fn emit_list_get_scalar() -> Function {
    let mut body = Function::new(vec![(1, ValType::I32), (1, ValType::I64)]);
    // params: table(0), count(1), index(2 i64); locals: tag(3), value(4)
    ops!(body;
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::I64ExtendI32U,
        Instruction::I64GeU,
        Instruction::If(BlockType::Empty),
        Instruction::I32Const(1), Instruction::LocalSet(3),
        Instruction::I64Const(0), Instruction::LocalSet(4),
        Instruction::Else,
        Instruction::I32Const(0), Instruction::LocalSet(3),
        Instruction::LocalGet(0), Instruction::LocalGet(2), Instruction::I32WrapI64,
        Instruction::I32Const(3), Instruction::I32Shl, Instruction::I32Add,
        Instruction::I64Load(mem(0, 3)), Instruction::LocalSet(4),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::LocalGet(4), Instruction::End,
    );
    body
}

/// `(table, count) = list.append(table, count, value)`: copy-on-write into a
/// geometrically sized table, like the Text pair-slot version.
fn emit_list_append_scalar(alloc: u32) -> Function {
    let mut body = Function::new(vec![(5, ValType::I32)]);
    // params: table(0), count(1), value(2 i64)
    // locals: base(3), table(4), capacity(5), next count(6), old base(7)
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(6),
        Instruction::LocalGet(0), Instruction::I32Const(16), Instruction::I32GeU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::I32Const(16), Instruction::I32Sub,
        Instruction::LocalSet(7),
        Instruction::LocalGet(7), Instruction::I32Load(mem(0, 2)),
        Instruction::I32Const(0x5349_4c4e), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::I32Load(mem(12, 2)),
        Instruction::I32Const(0x4c49_5354), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::I32Load(mem(4, 2)),
        Instruction::LocalGet(1), Instruction::I32Eq,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(7), Instruction::I32Load(mem(8, 2)),
        Instruction::LocalGet(1), Instruction::I32GtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(0), Instruction::LocalGet(1), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(2), Instruction::I64Store(mem(0, 3)),
        Instruction::LocalGet(7), Instruction::LocalGet(6),
        Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(0), Instruction::LocalGet(6), Instruction::Return,
        Instruction::End, Instruction::End, Instruction::End,
        Instruction::End, Instruction::End,
        Instruction::I32Const(1), Instruction::LocalSet(5),
        Instruction::Block(BlockType::Empty), Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::LocalGet(6), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Shl,
        Instruction::LocalSet(5), Instruction::Br(0),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(5), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Const(16), Instruction::I32Add,
        Instruction::Call(alloc), Instruction::LocalSet(3),
        Instruction::LocalGet(3), Instruction::I32Const(0x5349_4c4e),
        Instruction::I32Store(mem(0, 2)),
        Instruction::LocalGet(3), Instruction::LocalGet(6),
        Instruction::I32Store(mem(4, 2)),
        Instruction::LocalGet(3), Instruction::LocalGet(5),
        Instruction::I32Store(mem(8, 2)),
        Instruction::LocalGet(3), Instruction::I32Const(0x4c49_5354),
        Instruction::I32Store(mem(12, 2)),
        Instruction::LocalGet(3), Instruction::I32Const(16), Instruction::I32Add,
        Instruction::LocalSet(4),
        Instruction::LocalGet(4), Instruction::LocalGet(0), Instruction::LocalGet(1),
        Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(4), Instruction::LocalGet(1), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(2), Instruction::I64Store(mem(0, 3)),
        Instruction::LocalGet(4), Instruction::LocalGet(6),
        Instruction::End,
    );
    body
}

/// `(table, count) = list.sort(table, count)`: stable insertion sort into a
/// fresh table, numeric ascending (signed for I64, unsigned for U64).
#[allow(clippy::too_many_lines)]
fn emit_list_sort_scalar(alloc: u32, signed: bool) -> Function {
    let gt = if signed {
        Instruction::I64GtS
    } else {
        Instruction::I64GtU
    };
    let mut body = Function::new(vec![(3, ValType::I32), (2, ValType::I64)]);
    // params: table(0), count(1)
    // i32 locals: new(2), i(3), j(4); i64 locals: key(5), prev(6)
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::Call(alloc), Instruction::LocalSet(2),
        Instruction::LocalGet(2), Instruction::LocalGet(0), Instruction::LocalGet(1),
        Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::MemoryCopy { dst_mem: 0, src_mem: 0 },
        Instruction::LocalGet(1), Instruction::I32Const(2), Instruction::I32LtU,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::Return,
        Instruction::End,
        Instruction::I32Const(1), Instruction::LocalSet(3),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(2), Instruction::LocalGet(3), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I64Load(mem(0, 3)),
        Instruction::LocalSet(5),
        Instruction::LocalGet(3), Instruction::LocalSet(4),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(4), Instruction::I32Eqz,
        Instruction::BrIf(1),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(1),
        Instruction::I32Sub, Instruction::I32Const(3), Instruction::I32Shl,
        Instruction::I32Add, Instruction::I64Load(mem(0, 3)),
        Instruction::LocalSet(6),
        Instruction::LocalGet(6), Instruction::LocalGet(5),
    );
    ops!(body; gt);
    ops!(body;
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(6), Instruction::I64Store(mem(0, 3)),
        Instruction::LocalGet(4), Instruction::I32Const(-1), Instruction::I32Add,
        Instruction::LocalSet(4),
        // continue the inner scan: from inside the If the inner Loop is
        // depth 1 (the If itself is depth 0)
        Instruction::Br(1),
        Instruction::End,
        Instruction::Br(1),
        Instruction::End, Instruction::End,
        Instruction::LocalGet(2), Instruction::LocalGet(4), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add,
        Instruction::LocalGet(5), Instruction::I64Store(mem(0, 3)),
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(3),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(2), Instruction::LocalGet(1), Instruction::End,
    );
    body
}

/// `value = list.min|max(table, count, default)`: the extreme element, or
/// the default when the list is empty (same total-function shape as the
/// Text version, RFC-0045 D2).
fn emit_list_extreme_scalar(is_min: bool, signed: bool) -> Function {
    let replace = match (is_min, signed) {
        (true, true) => Instruction::I64LtS,
        (true, false) => Instruction::I64LtU,
        (false, true) => Instruction::I64GtS,
        (false, false) => Instruction::I64GtU,
    };
    let mut body = Function::new(vec![(1, ValType::I32), (2, ValType::I64)]);
    // params: table(0), count(1), default(2 i64)
    // locals: i(3 i32), best(4 i64), cur(5 i64)
    ops!(body;
        Instruction::LocalGet(1), Instruction::I32Eqz,
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(2), Instruction::Return,
        Instruction::End,
        Instruction::LocalGet(0), Instruction::I64Load(mem(0, 3)),
        Instruction::LocalSet(4),
        Instruction::I32Const(1), Instruction::LocalSet(3),
        Instruction::Block(BlockType::Empty),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GeU,
        Instruction::BrIf(1),
        Instruction::LocalGet(0), Instruction::LocalGet(3), Instruction::I32Const(3),
        Instruction::I32Shl, Instruction::I32Add, Instruction::I64Load(mem(0, 3)),
        Instruction::LocalSet(5),
        Instruction::LocalGet(5), Instruction::LocalGet(4),
    );
    ops!(body; replace);
    ops!(body;
        Instruction::If(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::LocalSet(4),
        Instruction::End,
        Instruction::LocalGet(3), Instruction::I32Const(1), Instruction::I32Add,
        Instruction::LocalSet(3),
        Instruction::Br(0), Instruction::End, Instruction::End,
        Instruction::LocalGet(4), Instruction::End,
    );
    body
}

/// Suffix comparison: `needle_len > hay_len` -> 0; else compare from the end.
/// Mirrors `emit_starts_with` with the cursor walking backwards.
fn emit_ends_with() -> Function {
    let mut body = Function::new(vec![(2, ValType::I32)]);
    // params: hay(0), hay_len(1), needle(2), needle_len(3)
    // locals: hay_off(4) = hay_len - needle_len, i(5) = 0
    ops!(body;
        Instruction::LocalGet(3), Instruction::LocalGet(1), Instruction::I32GtU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(1), Instruction::LocalGet(3), Instruction::I32Sub, Instruction::LocalSet(4),
        Instruction::Loop(BlockType::Empty),
        Instruction::LocalGet(5), Instruction::LocalGet(3), Instruction::I32GeU,
        Instruction::If(BlockType::Empty), Instruction::I32Const(1), Instruction::Return, Instruction::End,
        Instruction::LocalGet(0), Instruction::LocalGet(4), Instruction::I32Add, Instruction::LocalGet(5), Instruction::I32Add, Instruction::I32Load8U(mem(0, 0)),
        Instruction::LocalGet(2), Instruction::LocalGet(5), Instruction::I32Add, Instruction::I32Load8U(mem(0, 0)),
        Instruction::I32Ne,
        Instruction::If(BlockType::Empty), Instruction::I32Const(0), Instruction::Return, Instruction::End,
        Instruction::LocalGet(5), Instruction::I32Const(1), Instruction::I32Add, Instruction::LocalSet(5),
        Instruction::Br(0), Instruction::End, Instruction::Unreachable, Instruction::End,
    );
    body
}
