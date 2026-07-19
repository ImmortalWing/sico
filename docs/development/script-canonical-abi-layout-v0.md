# Script Canonical ABI layout v0

> - status: frozen for STEP-0079
> - date: 2026-07-17
> - source of truth: [`wit/script-profile-v0/world.wit`](../../wit/script-profile-v0/world.wit) interpreted through the WebAssembly Component Model Canonical ABI
> - implementation: `crates/sico-codegen-wasm/src/canonical.rs` derives this table from the embedded WIT text; this document is the human-readable freeze and any disagreement is a defect

## 1. Scope

This table fixes the Core Wasm and Canonical ABI representation of every aggregate shape accepted by the Script v0 boundary `sico:script/program@0.1.0.run: func(input: script-input) -> result<script-output, script-error>`. It covers Text (`string`), Bytes (`list<u8>`), `list<string>`, the Script records, the error enum and `result` ownership and cleanup. It does not cover nested generics, resources or any type not present in the Script WIT.

## 2. Primitive and composite layout

All multi-byte scalars are little-endian. `ptr`/`len` pairs are unsigned 32-bit offsets into the single guest linear memory.

| WIT type | Core representation | size | align | flattened |
|---|---|---:|---:|---|
| `string` (Text) | `(ptr i32, len i32)` to strict UTF-8 bytes | 8 | 4 | 2 × i32 |
| `list<u8>` (Bytes) | `(ptr i32, len i32)` to arbitrary octets | 8 | 4 | 2 × i32 |
| `list<string>` | `(ptr i32, len i32)` to `len` contiguous `(ptr, len)` entries, each 8 bytes aligned 4 | 8 | 4 | 2 × i32 |
| `s64` | one `i64` | 8 | 8 | 1 × i64 |
| `enum` with 4 cases | one byte discriminant, values in declaration order | 1 | 1 | 1 × i32 |

Discriminant size rule: at most 256 cases take one byte, at most 65,536 take two, otherwise four.

## 3. Script records

Fields are laid out in declaration order; each field offset is aligned up to the field alignment. Record alignment is the maximum field alignment and the record size is aligned up to it.

`script-input` (align 4, size 16):

| field | type | offset | size |
|---|---|---:|---:|
| `arguments` | `list<string>` | 0 | 8 |
| `stdin` | `list<u8>` | 8 | 8 |

`script-output` (align 8, size 24):

| field | type | offset | size |
|---|---|---:|---:|
| `stdout` | `list<u8>` | 0 | 8 |
| `stderr` | `list<u8>` | 8 | 8 |
| `exit-code` | `s64` | 16 | 8 |

`script-error-code` discriminants: `invalid-input` = 0, `resource-limit` = 1, `domain-error` = 2, `cancelled` = 3.

`script-error` (align 4, size 12):

| field | type | offset | size |
|---|---|---:|---:|
| `code` | `script-error-code` | 0 | 1 |
| `message` | `string` | 4 | 8 |

## 4. Result and function flattening

`result<script-output, script-error>`: one-byte discriminant at offset 0 (`ok` = 0, `err` = 1), payload at offset 8 (aligned to the maximum payload alignment 8), payload size `max(24, 12) = 24`, total size 32, alignment 8.

`run` parameter `script-input` flattens to four i32 values `(arguments.ptr, arguments.len, stdin.ptr, stdin.len)`, at most the Canonical ABI flat-parameter limit. The flattened result exceeds the single flat-result limit, so the core `run` returns one `i32` pointer to a 32-byte result area inside guest memory.

## 5. Ownership and cleanup

- One Wasm instance owns exactly one bounded arena. Arena base follows the static data segment; the arena ceiling is the declared memory maximum of 64 MiB (1,024 pages). Allocation uses checked 32-bit arithmetic and traps on overflow or ceiling crossing; it never silently wraps.
- Host-to-guest lowering allocates list/string payloads through the guest-exported `cabi_realloc(old_ptr, old_size, align, new_size) -> ptr`. Ownership of lowered parameter buffers transfers to the guest for the duration of the call.
- Guest-to-host lifting reads the 32-byte result area and every referenced payload before `cabi_post_run` returns. Result payload bytes are copied by the guest into its own arena during lowering; result buffers never alias caller-supplied parameter storage.
- `cabi_post_run` deterministically resets the arena bump pointer to the arena base. Repeated calls on one instance therefore reuse the same arena and do not grow memory. Cleanup is idempotent: a second reset has no effect. Dropping the Store releases the entire instance allocation; there is no general GC and no cross-instance reference.
- Text is validated at every untrusted boundary. Guest-produced `string` values (only `script-error.message` in v0) are validated as strict UTF-8 by the Canonical ABI lift and by the independent host helper; invalid bytes fail closed. `Bytes` never undergoes Text validation.

## 6. Bounds enforced at the boundary

From RFC-0029: at most 1,024 arguments; at most 64 KiB UTF-8 bytes per argument; at most 1 MiB of UTF-8 bytes across all arguments; at most 8 MiB each for stdin, stdout and stderr; at most 64 KiB in `script-error.message`; guest linear memory at most 64 MiB. Pointer, length, alignment, multiplication and addition are checked before any memory access; violations fail closed (host helper: typed refusal; engine lift: trap; guest allocation: trap).

## 7. Guest visible exports

The generated Program core module exports `memory`, `cabi_realloc`, `run` and `cabi_post_run`. The Component imports only the types-only instance `sico:script/types@0.1.0` and exports `run` with canonical options `utf8`, `memory`, `realloc` and `post-return`.
