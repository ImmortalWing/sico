use super::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap_or(1)).unwrap_or(0)
    }

    fn below_u32(&mut self, bound: u32) -> u32 {
        u32::try_from(self.below(bound as usize)).unwrap_or(0)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next() & 0xff) as u8).collect()
    }

    fn text(&mut self, len: usize) -> String {
        let mut output = String::new();
        while output.len() < len {
            let candidate = match self.below(6) {
                0 => char::from_u32(0x61 + self.below_u32(26)),
                1 => char::from_u32(0x4e2d + self.below_u32(0x100)),
                2 => char::from_u32(0x1f600 + self.below_u32(0x50)),
                3 => Some('\u{0}'),
                4 => Some(' '),
                _ => Some('%'),
            };
            if let Some(candidate) = candidate {
                output.push(candidate);
            }
        }
        output
    }

    fn input(&mut self) -> ScriptInput {
        let count = self.below(120);
        let arguments = (0..count)
            .map(|_| {
                let len = self.below(96);
                self.text(len)
            })
            .collect();
        let stdin_len = self.below(4096);
        let stdin = self.bytes(stdin_len);
        ScriptInput { arguments, stdin }
    }

    fn result(&mut self) -> ScriptResult {
        if self.below(4) == 0 {
            ScriptResult::Error(ScriptError {
                code: match self.below(4) {
                    0 => ScriptErrorCode::InvalidInput,
                    1 => ScriptErrorCode::ResourceLimit,
                    2 => ScriptErrorCode::DomainError,
                    _ => ScriptErrorCode::Cancelled,
                },
                message: {
                    let len = self.below(128);
                    self.text(len)
                },
            })
        } else {
            let stdout_len = self.below(4096);
            let stderr_len = self.below(1024);
            ScriptResult::Output(ScriptOutput {
                stdout: self.bytes(stdout_len),
                stderr: self.bytes(stderr_len),
                exit_code: i64::from(self.below_u32(120)),
            })
        }
    }
}

#[test]
fn seeded_input_and_result_roundtrips_are_byte_and_value_exact() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut arena = Arena::new(2048, ARENA_LIMIT);
    for _ in 0..10_000 {
        let input = rng.input();
        let params = lower_input(&mut arena, &input).unwrap();
        assert_eq!(lift_input(arena.memory(), params).unwrap(), input);

        let result = rng.result();
        let area_ptr = lower_result(&mut arena, &result).unwrap();
        assert_eq!(lift_result(arena.memory(), area_ptr).unwrap(), result);
        arena.reset();
    }
}

#[test]
fn m8_bounds_are_enforced_before_writing() {
    let mut arena = Arena::new(2048, ARENA_LIMIT);
    let input = |arguments: Vec<String>, stdin: Vec<u8>| ScriptInput { arguments, stdin };

    let at_count = input(vec![String::new(); MAX_ARGUMENTS], Vec::new());
    assert!(lower_input(&mut arena, &at_count).is_ok());
    arena.reset();
    let over_count = input(vec![String::new(); MAX_ARGUMENTS + 1], Vec::new());
    assert_eq!(
        lower_input(&mut arena, &over_count),
        Err(AbiError::ArgumentCount)
    );
    arena.reset();

    let at_argument = input(vec!["x".repeat(MAX_ARGUMENT_BYTES)], Vec::new());
    assert!(lower_input(&mut arena, &at_argument).is_ok());
    arena.reset();
    let over_argument = input(vec!["x".repeat(MAX_ARGUMENT_BYTES + 1)], Vec::new());
    assert_eq!(
        lower_input(&mut arena, &over_argument),
        Err(AbiError::ArgumentBytes)
    );
    arena.reset();

    let at_total = input(
        vec!["y".repeat(1024); MAX_TOTAL_ARGUMENT_BYTES / 1024],
        Vec::new(),
    );
    assert!(lower_input(&mut arena, &at_total).is_ok());
    arena.reset();
    let mut over_total_arguments = vec!["y".repeat(1024); MAX_TOTAL_ARGUMENT_BYTES / 1024 - 1];
    over_total_arguments.push("y".repeat(1025));
    let over_total = input(over_total_arguments, Vec::new());
    assert_eq!(
        lower_input(&mut arena, &over_total),
        Err(AbiError::TotalArgumentBytes)
    );
    arena.reset();

    let at_stdin = input(Vec::new(), vec![0; MAX_CHANNEL_BYTES]);
    assert!(lower_input(&mut arena, &at_stdin).is_ok());
    arena.reset();
    let over_stdin = input(Vec::new(), vec![0; MAX_CHANNEL_BYTES + 1]);
    assert_eq!(
        lower_input(&mut arena, &over_stdin),
        Err(AbiError::ChannelBytes)
    );
    arena.reset();

    let at_message = ScriptResult::Error(ScriptError {
        code: ScriptErrorCode::DomainError,
        message: "z".repeat(MAX_ERROR_MESSAGE_BYTES),
    });
    assert!(lower_result(&mut arena, &at_message).is_ok());
    arena.reset();
    let over_message = ScriptResult::Error(ScriptError {
        code: ScriptErrorCode::DomainError,
        message: "z".repeat(MAX_ERROR_MESSAGE_BYTES + 1),
    });
    assert_eq!(
        lower_result(&mut arena, &over_message),
        Err(AbiError::ErrorMessageBytes)
    );
}

#[test]
fn malformed_memory_fails_closed() {
    let mut arena = Arena::new(2048, ARENA_LIMIT);
    let result = ScriptResult::Error(ScriptError {
        code: ScriptErrorCode::DomainError,
        message: "honest failure".into(),
    });
    let area_ptr = lower_result(&mut arena, &result).unwrap() as usize;
    let memory = arena.memory().to_vec();

    // Invalid result discriminant.
    let mut mutated = memory.clone();
    mutated[area_ptr] = 2;
    assert_eq!(
        lift_result(&mutated, u32::try_from(area_ptr).unwrap_or(0)),
        Err(AbiError::InvalidTag)
    );
    let mut mutated = memory.clone();
    mutated[area_ptr] = 255;
    assert_eq!(
        lift_result(&mutated, u32::try_from(area_ptr).unwrap_or(0)),
        Err(AbiError::InvalidTag)
    );

    // Invalid enum discriminant inside the error payload.
    let mut mutated = memory.clone();
    mutated[area_ptr + RESULT_PAYLOAD_OFFSET] = 4;
    assert_eq!(
        lift_result(&mutated, u32::try_from(area_ptr).unwrap_or(0)),
        Err(AbiError::InvalidTag)
    );

    // Misaligned result area pointer.
    assert_eq!(
        lift_result(&memory, u32::try_from(area_ptr + 1).unwrap_or(0)),
        Err(AbiError::MisalignedPointer)
    );

    // Out-of-bounds and overflowing message pointers.
    let mut mutated = memory.clone();
    let payload = area_ptr + RESULT_PAYLOAD_OFFSET + 4;
    mutated[payload..payload + 4].copy_from_slice(
        &u32::try_from(memory.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    assert_eq!(
        lift_result(&mutated, u32::try_from(area_ptr).unwrap_or(0)),
        Err(AbiError::OutOfBounds)
    );
    let mut mutated = memory.clone();
    mutated[payload..payload + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        lift_result(&mutated, u32::try_from(area_ptr).unwrap_or(0)),
        Err(AbiError::OutOfBounds)
    );
    let mut mutated = memory.clone();
    mutated[payload + 4..payload + 8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        lift_result(&mutated, u32::try_from(area_ptr).unwrap_or(0)),
        Err(AbiError::ErrorMessageBytes | AbiError::ArithmeticOverflow | AbiError::OutOfBounds)
    ));

    // Invalid UTF-8 in the error message.
    let mut mutated = memory;
    let message_pointer =
        u32::from_le_bytes(mutated[payload..payload + 4].try_into().unwrap()) as usize;
    mutated[message_pointer] = 0xff;
    assert_eq!(
        lift_result(&mutated, u32::try_from(area_ptr).unwrap_or(0)),
        Err(AbiError::InvalidUtf8)
    );

    // Truncated buffer.
    let truncated = &arena.memory()[..area_ptr + 4];
    assert_eq!(
        lift_result(truncated, u32::try_from(area_ptr).unwrap_or(0)),
        Err(AbiError::OutOfBounds)
    );

    // Misaligned and out-of-bounds argument tables.
    let input = ScriptInput {
        arguments: vec!["one".into(), "two".into()],
        stdin: vec![1, 2, 3],
    };
    let params = lower_input(&mut arena, &input).unwrap();
    let memory = arena.memory().to_vec();
    assert_eq!(
        lift_input(&memory, (params.0 + 1, params.1, params.2, params.3)),
        Err(AbiError::MisalignedPointer)
    );
    assert!(matches!(
        lift_input(&memory, (u32::MAX - 7, params.1, params.2, params.3)),
        Err(AbiError::ArithmeticOverflow | AbiError::OutOfBounds)
    ));
    assert!(matches!(
        lift_input(&memory, (params.0, params.1 + 1, params.2, params.3)),
        Err(AbiError::ArgumentBytes | AbiError::OutOfBounds)
    ));
    let mut mutated = memory.clone();
    mutated[params.0 as usize] = 0xff;
    mutated[params.0 as usize + 1] = 0xff;
    mutated[params.0 as usize + 2] = 0xff;
    mutated[params.0 as usize + 3] = 0xff;
    assert!(matches!(
        lift_input(&mutated, params),
        Err(AbiError::ArithmeticOverflow | AbiError::OutOfBounds)
    ));
}

#[test]
fn invalid_utf8_in_argument_fails_closed() {
    let mut arena = Arena::new(2048, ARENA_LIMIT);
    let input = ScriptInput {
        arguments: vec!["valid".into()],
        stdin: Vec::new(),
    };
    let params = lower_input(&mut arena, &input).unwrap();
    let mut memory = arena.memory().to_vec();
    let pointer = u32::from_le_bytes(
        memory[params.0 as usize..params.0 as usize + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    memory[pointer] = 0xfe;
    assert_eq!(lift_input(&memory, params), Err(AbiError::InvalidUtf8));
}

#[test]
fn cleanup_is_deterministic_idempotent_and_bounded() {
    let mut rng = Rng(0xdead_beef_cafe_f00d);
    let mut arena = Arena::new(2048, ARENA_LIMIT);
    let input = rng.input();
    let first = lower_input(&mut arena, &input).unwrap();
    let first_image = arena.memory().to_vec();
    arena.reset();
    arena.reset();
    assert_eq!(arena.heap_high_water(), 2048);
    let second = lower_input(&mut arena, &input).unwrap();
    assert_eq!(first, second);
    assert_eq!(first_image, arena.memory());

    // Repeated call cycles never grow memory beyond the declared arena policy:
    // same-shape calls reuse the arena with an identical high-water mark and
    // the backing memory never grows after the first cycle.
    arena.reset();
    let call_input = rng.input();
    let call_result = rng.result();
    let mut expected_high_water = None;
    let mut expected_memory_len = None;
    for _ in 0..1_000 {
        let params = lower_input(&mut arena, &call_input).unwrap();
        assert_eq!(lift_input(arena.memory(), params).unwrap(), call_input);
        let area_ptr = lower_result(&mut arena, &call_result).unwrap();
        assert_eq!(lift_result(arena.memory(), area_ptr).unwrap(), call_result);
        let mark = arena.heap_high_water();
        if let Some(expected) = expected_high_water {
            assert_eq!(mark, expected);
            assert_eq!(Some(arena.memory().len()), expected_memory_len);
        } else {
            expected_high_water = Some(mark);
            expected_memory_len = Some(arena.memory().len());
        }
        arena.reset();
    }
}

#[test]
fn arena_arithmetic_is_checked() {
    let mut arena = Arena::new(2048, 4096);
    assert!(arena.alloc(8, 2048).is_ok());
    assert_eq!(arena.alloc(8, 1), Err(AbiError::ArenaExhausted));
    let mut arena = Arena::new(usize::MAX - 4, ARENA_LIMIT);
    assert_eq!(arena.alloc(8, 8), Err(AbiError::ArithmeticOverflow));
}
