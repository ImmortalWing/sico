fn main() {
    use wasm_encoder::{
        BlockType, CodeSection, Function, FunctionSection, Instruction, Module, TypeSection,
        ValType,
    };
    let mut types = TypeSection::new();
    types.ty().function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    let mut functions = FunctionSection::new();
    functions.function(0);
    let variant = std::env::args().nth(1).unwrap_or_default();
    let mut body = Function::new(vec![]);
    let seq: &[Instruction] = match variant.as_str() {
        "a" => &[
            Instruction::Loop(BlockType::Empty),
            Instruction::Br(0),
            Instruction::End,
            Instruction::End,
        ],
        "b" => &[
            Instruction::Loop(BlockType::Empty),
            Instruction::Br(0),
            Instruction::End,
            Instruction::I32Const(0),
            Instruction::End,
        ],
        "c" => &[
            Instruction::Loop(BlockType::Empty),
            Instruction::Br(0),
            Instruction::End,
            Instruction::Unreachable,
            Instruction::End,
        ],
        _ => &[Instruction::I32Const(0), Instruction::End],
    };
    for instruction in seq {
        body.instruction(instruction);
    }
    let mut code = CodeSection::new();
    code.function(&body);
    let mut memories = wasm_encoder::MemorySection::new();
    memories.memory(wasm_encoder::MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&code);
    let bytes = module.finish();
    match wasmparser::Validator::new().validate_all(&bytes) {
        Ok(_) => println!("VALID"),
        Err(e) => println!("INVALID: {e}"),
    }
}
