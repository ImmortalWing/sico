//! `sico:user/image-codec@1` PNG guest core (M24 #2).
//!
//! Production guest crate per ADR-0019 (Rust-compiled package core, accepted
//! by owner re-adjudication 2026-09-30) and RFC-0050 as amended by A1. It is
//! compiled to `wasm32-unknown-unknown` and committed as a digest-pinned core
//! `.wasm`; `sico_codegen_wasm::package_builder::image_codec_package` embeds
//! those bytes and wraps them with `build_package_component`. The crate lives
//! outside the root workspace, is pure (import-free, no ambient FS/net/screen),
//! and its outputs are bounded by RFC-0050 §3 limits.
//!
//! ABI contract copied from `package_builder.rs` (measured, not assumed):
//! - `realloc(ptr, old_size, align, new_size) -> ptr` (canonical ABI);
//! - parameters flatten to `(i32 ptr, i32 len)` per `Bytes` and one `i64`
//!   per `U64`;
//! - an aggregate/`Result` return is **one `i32`**: the pointer to a
//!   12-byte return area the callee allocated itself, holding the
//!   discriminant as a single byte at offset 0 and the payload `(ptr, len)`
//!   at offsets 4 and 8 (`emit_csv_error` is the in-repo precedent);
//! - returned buffers are intentionally leaked: the package convention has
//!   no post-return, so allocations outlive the call.
//!
//! Surface shape follows the RFC-0050 §1 flattening required by the v0
//! user-WIT value set (`interface_value_offender`, RFC-0039 §2.4 A7):
//! records and enums are refused there, so the decoded image rides
//! `Bytes` as a 16-byte little-endian header (width u64, height u64)
//! followed by RFC-0041 BGRA8 pixels, and the error case rides a case-name
//! `Text`.

use std::alloc::{alloc as sys_alloc, realloc as sys_realloc, Layout};
use std::io::Cursor;

/// Declared pixel-count bound (RFC-0050 §3): `width * height <= 2^28`.
const MAX_PIXELS: u64 = 1 << 28;
/// PNG signature (RFC 2083 §3.1).
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
/// Byte length of the flattened decode payload header (width + height).
const IMAGE_HEADER_LEN: usize = 16;

fn trap() -> ! {
    core::arch::wasm32::unreachable()
}

/// Canonical ABI allocator entry point. The lifter only ever calls it with
/// `ptr == 0`; the non-zero branch is implemented for completeness and
/// traps on an inconsistent `old_size` rather than guessing a layout.
#[no_mangle]
pub extern "C" fn realloc(ptr: i32, old_size: i32, align: i32, new_size: i32) -> i32 {
    if align <= 0 || new_size < 0 || old_size < 0 || (align as usize).count_ones() != 1 {
        trap();
    }
    let align = align as usize;
    let new_size = new_size as usize;
    if ptr == 0 {
        if new_size == 0 {
            return i32::try_from(align).unwrap_or(1);
        }
        let layout = Layout::from_size_align(new_size, align).unwrap_or_else(|_| trap());
        let allocated = unsafe { sys_alloc(layout) };
        if allocated.is_null() {
            trap();
        }
        return allocated as i32;
    }
    if new_size == 0 {
        return ptr;
    }
    let old_size = old_size as usize;
    if old_size == 0 {
        trap();
    }
    let layout = Layout::from_size_align(old_size, align).unwrap_or_else(|_| trap());
    let resized = unsafe { sys_realloc(ptr as *mut u8, layout, new_size) };
    if resized.is_null() {
        trap();
    }
    resized as i32
}

/// Copies `bytes` into a leaked guest allocation and returns `(ptr, len)`.
fn alloc_bytes(bytes: &[u8]) -> (i32, i32) {
    if bytes.is_empty() {
        return (0, 0);
    }
    let layout = Layout::from_size_align(bytes.len(), 1).unwrap_or_else(|_| trap());
    let allocated = unsafe { sys_alloc(layout) };
    if allocated.is_null() {
        trap();
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), allocated, bytes.len()) };
    (
        allocated as i32,
        i32::try_from(bytes.len()).unwrap_or_else(|_| trap()),
    )
}

/// Writes the 12-byte canonical return area and returns its pointer.
fn return_area(discriminant: u8, payload: (i32, i32)) -> i32 {
    let layout = Layout::from_size_align(12, 4).unwrap_or_else(|_| trap());
    let area = unsafe { sys_alloc(layout) };
    if area.is_null() {
        trap();
    }
    unsafe {
        *area = discriminant;
        let slots = area.add(4) as *mut i32;
        *slots = payload.0;
        *slots.add(1) = payload.1;
    }
    area as i32
}

fn ok_area(bytes: &[u8]) -> i32 {
    return_area(0, alloc_bytes(bytes))
}

fn err_area(case: &str) -> i32 {
    return_area(1, alloc_bytes(case.as_bytes()))
}

/// `decode-png(data: Bytes) -> Result[Bytes, Text]`.
#[export_name = "decode-png"]
pub extern "C" fn decode_png(source_ptr: i32, source_len: i32) -> i32 {
    if source_ptr < 0 || source_len < 0 {
        trap();
    }
    let input =
        unsafe { std::slice::from_raw_parts(source_ptr as *const u8, source_len as usize) };
    match decode_png_inner(input) {
        Ok(payload) => ok_area(&payload),
        Err(case) => err_area(case),
    }
}

/// `encode-png(pixels: Bytes, width: U64, height: U64) -> Result[Bytes, Text]`.
#[export_name = "encode-png"]
pub extern "C" fn encode_png(
    pixels_ptr: i32,
    pixels_len: i32,
    width: i64,
    height: i64,
) -> i32 {
    if pixels_ptr < 0 || pixels_len < 0 || width < 0 || height < 0 {
        trap();
    }
    let pixels =
        unsafe { std::slice::from_raw_parts(pixels_ptr as *const u8, pixels_len as usize) };
    match encode_png_inner(pixels, width as u64, height as u64) {
        Ok(bytes) => ok_area(&bytes),
        Err(case) => err_area(case),
    }
}

/// IHDR fields read before any pixel allocation (RFC-0050 §3).
struct PngHeader {
    width: u64,
    height: u64,
    bit_depth: u8,
    color_type: u8,
    interlace: u8,
}

/// Walks the chunk list: validates signatures/lengths/CRC coverage, reads
/// IHDR, and refuses trailing data after IEND.
fn scan_png(input: &[u8]) -> Result<PngHeader, &'static str> {
    if input.len() < 8 || input[..8] != PNG_SIGNATURE {
        return Err("bad-signature");
    }
    let mut offset = 8usize;
    let mut header: Option<PngHeader> = None;
    let mut ended = false;
    while offset < input.len() {
        if ended {
            return Err("trailing-data");
        }
        if offset + 12 > input.len() {
            return Err("truncated");
        }
        let length = u32::from_be_bytes([
            input[offset],
            input[offset + 1],
            input[offset + 2],
            input[offset + 3],
        ]) as usize;
        let kind = &input[offset + 4..offset + 8];
        if !kind
            .iter()
            .all(|byte| byte.is_ascii_alphabetic())
        {
            return Err("corrupt-stream");
        }
        let data_start = offset + 8;
        let data_end = data_start.checked_add(length).ok_or("oversized")?;
        if data_end + 4 > input.len() {
            return Err("truncated");
        }
        let crc_offset = data_end;
        let expected = u32::from_be_bytes([
            input[crc_offset],
            input[crc_offset + 1],
            input[crc_offset + 2],
            input[crc_offset + 3],
        ]);
        if crc32(&input[offset + 4..data_end]) != expected {
            return Err("corrupt-stream");
        }
        if kind == b"IHDR" {
            if header.is_some() {
                return Err("corrupt-stream");
            }
            if length != 13 {
                return Err("corrupt-stream");
            }
            let data = &input[data_start..data_end];
            let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as u64;
            let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as u64;
            header = Some(PngHeader {
                width,
                height,
                bit_depth: data[8],
                color_type: data[9],
                interlace: data[12],
            });
        } else if kind == b"IEND" {
            ended = true;
        } else if header.is_none() {
            return Err("corrupt-stream");
        }
        offset = crc_offset + 4;
    }
    if !ended {
        return Err("truncated");
    }
    header.ok_or("corrupt-stream")
}

/// CRC-32 (ISO 3309 / ITU-T V.42), the PNG chunk checksum.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn decode_png_inner(input: &[u8]) -> Result<Vec<u8>, &'static str> {
    let header = scan_png(input)?;
    if header.interlace != 0 {
        return Err("unsupported-profile");
    }
    if header.bit_depth != 8 {
        return Err("unsupported-profile");
    }
    // v0 accepted color types: greyscale-8 (0), truecolor-RGB-8 (2),
    // greyscale+alpha-8 (4), truecolor+alpha-RGB-8 (6). Palette (3) refused.
    if !matches!(header.color_type, 0 | 2 | 4 | 6) {
        return Err("unsupported-profile");
    }
    let pixels = header
        .width
        .checked_mul(header.height)
        .ok_or("oversized")?;
    if pixels == 0 || pixels > MAX_PIXELS {
        return Err("oversized");
    }
    let channels = match header.color_type {
        0 => 1usize,
        2 => 3,
        4 => 2,
        _ => 4,
    };

    let mut decoder = png::Decoder::new(Cursor::new(input));
    let budget = pixels
        .saturating_mul(4)
        .min(u64::from(u32::MAX))
        .max(1) as usize;
    decoder.set_limits(png::Limits { bytes: budget });
    let mut reader = decoder.read_info().map_err(classify_decode_error)?;
    let info = reader.info();
    if u64::from(info.width) != header.width || u64::from(info.height) != header.height {
        return Err("corrupt-stream");
    }
    if info.color_type != expected_color_type(header.color_type) {
        return Err("unsupported-profile");
    }
    let channels = usize::try_from(channels).unwrap_or_else(|_| trap());
    let expected: usize = pixels
        .checked_mul(channels as u64)
        .ok_or("oversized")?
        .try_into()
        .unwrap_or_else(|_| trap());
    let mut decoded = vec![0u8; expected];
    let row_info = reader
        .next_frame(&mut decoded)
        .map_err(classify_decode_error)?;
    if row_info.buffer_size() != expected {
        return Err("corrupt-stream");
    }

    let width = usize::try_from(header.width).unwrap_or_else(|_| trap());
    let height = usize::try_from(header.height).unwrap_or_else(|_| trap());
    let mut payload = Vec::with_capacity(
        IMAGE_HEADER_LEN
            + width
                .checked_mul(height)
                .and_then(|count| count.checked_mul(4))
                .unwrap_or_else(|| trap()),
    );
    payload.extend_from_slice(&header.width.to_le_bytes());
    payload.extend_from_slice(&header.height.to_le_bytes());
    for row in 0..height {
        for column in 0..width {
            let source = (row * width + column) * channels;
            let (b, g, r, a) = match channels {
                1 => {
                    let grey = decoded[source];
                    (grey, grey, grey, 255u8)
                }
                2 => {
                    let grey = decoded[source];
                    (grey, grey, grey, decoded[source + 1])
                }
                3 => (decoded[source + 2], decoded[source + 1], decoded[source], 255u8),
                _ => (decoded[source + 2], decoded[source + 1], decoded[source], decoded[source + 3]),
            };
            payload.push(b);
            payload.push(g);
            payload.push(r);
            payload.push(a);
        }
    }
    Ok(payload)
}

fn expected_color_type(raw: u8) -> png::ColorType {
    match raw {
        0 => png::ColorType::Grayscale,
        2 => png::ColorType::Rgb,
        4 => png::ColorType::GrayscaleAlpha,
        _ => png::ColorType::Rgba,
    }
}

fn classify_decode_error(error: png::DecodingError) -> &'static str {
    use png::DecodingError;
    match error {
        DecodingError::IoError(io) => {
            if io.kind() == std::io::ErrorKind::UnexpectedEof {
                "truncated"
            } else {
                "corrupt-stream"
            }
        }
        DecodingError::LimitsExceeded => "oversized",
        DecodingError::Format(_) => "corrupt-stream",
        DecodingError::Parameter(_) => "corrupt-stream",
    }
}

fn classify_encode_error(error: png::EncodingError) -> &'static str {
    use png::EncodingError;
    match error {
        EncodingError::IoError(io) => {
            if io.kind() == std::io::ErrorKind::UnexpectedEof {
                "truncated"
            } else {
                "corrupt-stream"
            }
        }
        EncodingError::Format(_) => "corrupt-stream",
        EncodingError::Parameter(_) => "corrupt-stream",
        EncodingError::LimitsExceeded => "oversized",
    }
}

fn encode_png_inner(pixels: &[u8], width: u64, height: u64) -> Result<Vec<u8>, &'static str> {
    if width == 0 || height == 0 {
        return Err("oversized");
    }
    let count = width.checked_mul(height).ok_or("oversized")?;
    if count > MAX_PIXELS {
        return Err("oversized");
    }
    let expected = usize::try_from(count.saturating_mul(4)).unwrap_or_else(|_| trap());
    if pixels.len() != expected {
        // RFC-0050 §3 closed CodecError set, as completed by Amendment A1
        // (owner re-adjudication 2026-09-30): a pixel-buffer length that does
        // not equal `width * height * 4` is `encode-buffer-mismatch`, distinct
        // from a malformed input stream (`corrupt-stream`).
        return Err("encode-buffer-mismatch");
    }
    let mut rgba = Vec::with_capacity(expected);
    for pixel in pixels.chunks_exact(4) {
        rgba.push(pixel[2]);
        rgba.push(pixel[1]);
        rgba.push(pixel[0]);
        rgba.push(pixel[3]);
    }
    let mut output = Cursor::new(Vec::new());
    {
        let width_u32 = u32::try_from(width).unwrap_or_else(|_| trap());
        let height_u32 = u32::try_from(height).unwrap_or_else(|_| trap());
        let mut encoder = png::Encoder::new(&mut output, width_u32, height_u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // RFC-0050 §2 frozen encoder settings: every scanline Paeth,
        // zlib level 9, RGBA-8 non-interlaced, no ancillary chunks.
        // `png` 0.18 exposes the exact deflate level through
        // `DeflateCompression::Level`, so "level 9" stays literal.
        encoder.set_deflate_compression(png::DeflateCompression::Level(9));
        encoder.set_filter(png::Filter::Paeth);
        let mut writer = encoder.write_header().map_err(classify_encode_error)?;
        writer
            .write_image_data(&rgba)
            .map_err(classify_encode_error)?;
    }
    let bytes = output.into_inner();
    // The frozen chunk set is part of the contract; verify it here so a
    // crate upgrade cannot silently add ancillary chunks.
    scan_png(&bytes)?;
    let mut offset = 8usize;
    while offset + 12 <= bytes.len() {
        let length =
            u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
                as usize;
        let kind = &bytes[offset + 4..offset + 8];
        if !matches!(kind, b"IHDR" | b"IDAT" | b"IEND") {
            return Err("corrupt-stream");
        }
        offset += 12 + length;
    }
    Ok(bytes)
}
