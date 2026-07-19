//! Independent host-side Canonical ABI lift/lower for the frozen Script v0
//! boundary (`docs/development/script-canonical-abi-layout-v0.md`).
//!
//! These helpers reimplement the exact byte layout without any engine
//! dependency so malformed memory, arithmetic overflow, misalignment, invalid
//! discriminants and invalid UTF-8 are refused with typed errors before any
//! trust is placed in guest- or host-produced buffers.

#![forbid(unsafe_code)]

/// RFC-0029 Script v0 bounds.
pub const MAX_ARGUMENTS: usize = 1_024;
pub const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
pub const MAX_TOTAL_ARGUMENT_BYTES: usize = 1024 * 1024;
pub const MAX_CHANNEL_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ERROR_MESSAGE_BYTES: usize = 64 * 1024;
pub const ARENA_LIMIT: usize = 64 * 1024 * 1024;

/// Canonical ABI result area for `result<script-output, script-error>`.
pub const RESULT_SIZE: usize = 32;
pub const RESULT_ALIGN: usize = 8;
pub const RESULT_PAYLOAD_OFFSET: usize = 8;

/// Script v0 `script-input` value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptInput {
    pub arguments: Vec<String>,
    pub stdin: Vec<u8>,
}

/// Script v0 `script-output` value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i64,
}

/// Script v0 `script-error-code` discriminant in WIT declaration order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScriptErrorCode {
    InvalidInput,
    ResourceLimit,
    DomainError,
    Cancelled,
}

impl ScriptErrorCode {
    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::InvalidInput),
            1 => Some(Self::ResourceLimit),
            2 => Some(Self::DomainError),
            3 => Some(Self::Cancelled),
            _ => None,
        }
    }

    fn tag(self) -> u8 {
        match self {
            Self::InvalidInput => 0,
            Self::ResourceLimit => 1,
            Self::DomainError => 2,
            Self::Cancelled => 3,
        }
    }
}

/// Script v0 `script-error` value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptError {
    pub code: ScriptErrorCode,
    pub message: String,
}

/// Script v0 `result<script-output, script-error>` value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScriptResult {
    Output(ScriptOutput),
    Error(ScriptError),
}

/// Typed boundary refusal. Every variant is a fail-closed outcome; none of
/// them permits a partial read or write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiError {
    ArgumentCount,
    ArgumentBytes,
    TotalArgumentBytes,
    ChannelBytes,
    ErrorMessageBytes,
    ArenaExhausted,
    ArithmeticOverflow,
    MisalignedPointer,
    OutOfBounds,
    InvalidTag,
    InvalidUtf8,
}

/// One bounded arena modelling the guest linear-memory discipline: a bump
/// pointer over a checked ceiling, reclaimed per call by [`Arena::reset`].
#[derive(Clone, Debug)]
pub struct Arena {
    memory: Vec<u8>,
    base: usize,
    heap: usize,
    limit: usize,
}

impl Arena {
    /// Creates an arena whose bump pointer starts at `base` and may never
    /// cross `limit` (at most [`ARENA_LIMIT`]).
    #[must_use]
    pub fn new(base: usize, limit: usize) -> Self {
        let limit = limit.min(ARENA_LIMIT);
        Self {
            memory: Vec::new(),
            base,
            heap: base,
            limit,
        }
    }

    /// Allocates `size` bytes aligned to `align` with checked arithmetic.
    ///
    /// # Errors
    ///
    /// Returns [`AbiError::ArithmeticOverflow`] on overflowing pointer
    /// arithmetic and [`AbiError::ArenaExhausted`] when the ceiling would be
    /// crossed; no allocation happens in either case.
    pub fn alloc(&mut self, align: usize, size: usize) -> Result<usize, AbiError> {
        debug_assert!(align.is_power_of_two());
        let pointer = self
            .heap
            .checked_add(align - 1)
            .ok_or(AbiError::ArithmeticOverflow)?
            & !(align - 1);
        let end = pointer
            .checked_add(size)
            .ok_or(AbiError::ArithmeticOverflow)?;
        if end > self.limit {
            return Err(AbiError::ArenaExhausted);
        }
        if end > self.memory.len() {
            self.memory.resize(end, 0);
        }
        self.heap = end;
        Ok(pointer)
    }

    /// `cabi_post_run` semantics: reset the bump pointer to the arena base.
    /// Idempotent; a repeated reset has no effect.
    pub fn reset(&mut self) {
        self.heap = self.base;
    }

    #[must_use]
    pub fn memory(&self) -> &[u8] {
        &self.memory
    }

    #[must_use]
    pub fn heap_high_water(&self) -> usize {
        self.heap
    }

    fn write(&mut self, pointer: usize, bytes: &[u8]) -> Result<(), AbiError> {
        let end = pointer
            .checked_add(bytes.len())
            .ok_or(AbiError::ArithmeticOverflow)?;
        if end > self.memory.len() {
            return Err(AbiError::OutOfBounds);
        }
        self.memory[pointer..end].copy_from_slice(bytes);
        Ok(())
    }
}

/// Flattened Canonical ABI parameters of `run`: `(arguments.ptr,
/// arguments.len, stdin.ptr, stdin.len)`.
pub type FlatParams = (u32, u32, u32, u32);

/// Lowers a `script-input` into the arena and returns the flattened `run`
/// parameters. M8 bounds are enforced before any byte is written.
///
/// # Errors
///
/// Returns a typed [`AbiError`] when an M8 bound is exceeded or arena
/// arithmetic overflows; the arena is left without the new allocation.
pub fn lower_input(arena: &mut Arena, input: &ScriptInput) -> Result<FlatParams, AbiError> {
    if input.arguments.len() > MAX_ARGUMENTS {
        return Err(AbiError::ArgumentCount);
    }
    let mut total_argument_bytes = 0_usize;
    for argument in &input.arguments {
        if argument.len() > MAX_ARGUMENT_BYTES {
            return Err(AbiError::ArgumentBytes);
        }
        total_argument_bytes = total_argument_bytes
            .checked_add(argument.len())
            .ok_or(AbiError::ArithmeticOverflow)?;
    }
    if total_argument_bytes > MAX_TOTAL_ARGUMENT_BYTES {
        return Err(AbiError::TotalArgumentBytes);
    }
    if input.stdin.len() > MAX_CHANNEL_BYTES {
        return Err(AbiError::ChannelBytes);
    }

    let element_count = input.arguments.len();
    let table_size = element_count
        .checked_mul(8)
        .ok_or(AbiError::ArithmeticOverflow)?;
    let table = arena.alloc(4, table_size)?;
    for (index, argument) in input.arguments.iter().enumerate() {
        let pointer = arena.alloc(1, argument.len())?;
        arena.write(pointer, argument.as_bytes())?;
        let entry = table + index * 8;
        arena.write(
            entry,
            &u32::try_from(pointer)
                .map_err(|_| AbiError::ArithmeticOverflow)?
                .to_le_bytes(),
        )?;
        arena.write(
            entry + 4,
            &u32::try_from(argument.len())
                .map_err(|_| AbiError::ArithmeticOverflow)?
                .to_le_bytes(),
        )?;
    }
    let stdin = arena.alloc(1, input.stdin.len())?;
    arena.write(stdin, &input.stdin)?;
    Ok((
        u32::try_from(table).map_err(|_| AbiError::ArithmeticOverflow)?,
        u32::try_from(element_count).map_err(|_| AbiError::ArithmeticOverflow)?,
        u32::try_from(stdin).map_err(|_| AbiError::ArithmeticOverflow)?,
        u32::try_from(input.stdin.len()).map_err(|_| AbiError::ArithmeticOverflow)?,
    ))
}

/// Lifts a `script-input` back out of guest-shaped memory, validating every
/// pointer, length, alignment and UTF-8 byte sequence.
///
/// # Errors
///
/// Returns a typed [`AbiError`] for malformed memory: misalignment,
/// out-of-bounds or overflowing pointers, invalid UTF-8 and bound violations.
pub fn lift_input(memory: &[u8], params: FlatParams) -> Result<ScriptInput, AbiError> {
    let (table, count, stdin_pointer, stdin_length) = params;
    let table = usize::try_from(table).map_err(|_| AbiError::ArithmeticOverflow)?;
    let count = usize::try_from(count).map_err(|_| AbiError::ArithmeticOverflow)?;
    if count > MAX_ARGUMENTS {
        return Err(AbiError::ArgumentCount);
    }
    if table % 4 != 0 {
        return Err(AbiError::MisalignedPointer);
    }
    let mut arguments = Vec::with_capacity(count);
    let mut total_argument_bytes = 0_usize;
    for index in 0..count {
        let entry = table
            .checked_add(index.checked_mul(8).ok_or(AbiError::ArithmeticOverflow)?)
            .ok_or(AbiError::ArithmeticOverflow)?;
        let pointer = read_u32(memory, entry)? as usize;
        let length = read_u32(memory, entry + 4)? as usize;
        if length > MAX_ARGUMENT_BYTES {
            return Err(AbiError::ArgumentBytes);
        }
        total_argument_bytes = total_argument_bytes
            .checked_add(length)
            .ok_or(AbiError::ArithmeticOverflow)?;
        if total_argument_bytes > MAX_TOTAL_ARGUMENT_BYTES {
            return Err(AbiError::TotalArgumentBytes);
        }
        let bytes = read_bytes(memory, pointer, length)?;
        arguments.push(String::from_utf8(bytes.to_vec()).map_err(|_| AbiError::InvalidUtf8)?);
    }
    let stdin_length = usize::try_from(stdin_length).map_err(|_| AbiError::ArithmeticOverflow)?;
    if stdin_length > MAX_CHANNEL_BYTES {
        return Err(AbiError::ChannelBytes);
    }
    let stdin = read_bytes(
        memory,
        usize::try_from(stdin_pointer).map_err(|_| AbiError::ArithmeticOverflow)?,
        stdin_length,
    )?
    .to_vec();
    Ok(ScriptInput { arguments, stdin })
}

/// Lowers a `script-result` into the arena following the guest lowering
/// contract: a 32-byte area, tag at offset 0, payload at offset 8, and every
/// payload byte copied into the arena. Returns the area pointer.
///
/// # Errors
///
/// Returns a typed [`AbiError`] when a channel/message bound is exceeded or
/// arena arithmetic overflows.
pub fn lower_result(arena: &mut Arena, result: &ScriptResult) -> Result<u32, AbiError> {
    let result_area = match result {
        ScriptResult::Output(output) => {
            if output.stdout.len() > MAX_CHANNEL_BYTES || output.stderr.len() > MAX_CHANNEL_BYTES {
                return Err(AbiError::ChannelBytes);
            }
            let ret_area = arena.alloc(RESULT_ALIGN, RESULT_SIZE)?;
            arena.write(ret_area, &[0])?;
            let stdout = write_list(arena, &output.stdout)?;
            let stderr = write_list(arena, &output.stderr)?;
            arena.write(ret_area + RESULT_PAYLOAD_OFFSET, &stdout)?;
            arena.write(ret_area + RESULT_PAYLOAD_OFFSET + 8, &stderr)?;
            arena.write(
                ret_area + RESULT_PAYLOAD_OFFSET + 16,
                &output.exit_code.to_le_bytes(),
            )?;
            ret_area
        }
        ScriptResult::Error(error) => {
            if error.message.len() > MAX_ERROR_MESSAGE_BYTES {
                return Err(AbiError::ErrorMessageBytes);
            }
            let ret_area = arena.alloc(RESULT_ALIGN, RESULT_SIZE)?;
            arena.write(ret_area, &[1])?;
            arena.write(ret_area + RESULT_PAYLOAD_OFFSET, &[error.code.tag()])?;
            let message = write_list(arena, error.message.as_bytes())?;
            arena.write(ret_area + RESULT_PAYLOAD_OFFSET + 4, &message)?;
            ret_area
        }
    };
    u32::try_from(result_area).map_err(|_| AbiError::ArithmeticOverflow)
}

fn write_list(arena: &mut Arena, bytes: &[u8]) -> Result<[u8; 8], AbiError> {
    let pointer = arena.alloc(1, bytes.len())?;
    arena.write(pointer, bytes)?;
    let mut pair = [0_u8; 8];
    pair[..4].copy_from_slice(
        &u32::try_from(pointer)
            .map_err(|_| AbiError::ArithmeticOverflow)?
            .to_le_bytes(),
    );
    pair[4..].copy_from_slice(
        &u32::try_from(bytes.len())
            .map_err(|_| AbiError::ArithmeticOverflow)?
            .to_le_bytes(),
    );
    Ok(pair)
}

/// Lifts a `script-result` from guest-shaped memory. Every discriminant,
/// pointer, length, alignment and UTF-8 sequence is validated before use.
///
/// # Errors
///
/// Returns a typed [`AbiError`] for malformed memory: invalid discriminants,
/// misalignment, out-of-bounds or overflowing pointers, invalid UTF-8 and
/// bound violations.
pub fn lift_result(memory: &[u8], area: u32) -> Result<ScriptResult, AbiError> {
    let area = usize::try_from(area).map_err(|_| AbiError::ArithmeticOverflow)?;
    if area % RESULT_ALIGN != 0 {
        return Err(AbiError::MisalignedPointer);
    }
    read_bytes(memory, area, RESULT_SIZE)?;
    let payload = area + RESULT_PAYLOAD_OFFSET;
    match read_u8(memory, area)? {
        0 => {
            let stdout_pointer = read_u32(memory, payload)? as usize;
            let stdout_length = read_u32(memory, payload + 4)? as usize;
            let stderr_pointer = read_u32(memory, payload + 8)? as usize;
            let stderr_length = read_u32(memory, payload + 12)? as usize;
            if stdout_length > MAX_CHANNEL_BYTES || stderr_length > MAX_CHANNEL_BYTES {
                return Err(AbiError::ChannelBytes);
            }
            let exit_code = read_i64(memory, payload + 16)?;
            Ok(ScriptResult::Output(ScriptOutput {
                stdout: read_bytes(memory, stdout_pointer, stdout_length)?.to_vec(),
                stderr: read_bytes(memory, stderr_pointer, stderr_length)?.to_vec(),
                exit_code,
            }))
        }
        1 => {
            let code =
                ScriptErrorCode::from_tag(read_u8(memory, payload)?).ok_or(AbiError::InvalidTag)?;
            let message_pointer = read_u32(memory, payload + 4)? as usize;
            let message_length = read_u32(memory, payload + 8)? as usize;
            if message_length > MAX_ERROR_MESSAGE_BYTES {
                return Err(AbiError::ErrorMessageBytes);
            }
            let message =
                String::from_utf8(read_bytes(memory, message_pointer, message_length)?.to_vec())
                    .map_err(|_| AbiError::InvalidUtf8)?;
            Ok(ScriptResult::Error(ScriptError { code, message }))
        }
        _ => Err(AbiError::InvalidTag),
    }
}

fn read_u8(memory: &[u8], offset: usize) -> Result<u8, AbiError> {
    memory.get(offset).copied().ok_or(AbiError::OutOfBounds)
}

fn read_u32(memory: &[u8], offset: usize) -> Result<u32, AbiError> {
    let bytes = read_bytes(memory, offset, 4)?;
    Ok(u32::from_le_bytes(
        bytes.try_into().map_err(|_| AbiError::OutOfBounds)?,
    ))
}

fn read_i64(memory: &[u8], offset: usize) -> Result<i64, AbiError> {
    let bytes = read_bytes(memory, offset, 8)?;
    Ok(i64::from_le_bytes(
        bytes.try_into().map_err(|_| AbiError::OutOfBounds)?,
    ))
}

fn read_bytes(memory: &[u8], pointer: usize, length: usize) -> Result<&[u8], AbiError> {
    let end = pointer
        .checked_add(length)
        .ok_or(AbiError::ArithmeticOverflow)?;
    memory.get(pointer..end).ok_or(AbiError::OutOfBounds)
}

#[cfg(test)]
mod tests;
