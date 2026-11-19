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
    #[cfg(target_arch = "wasm32")]
    {
        core::arch::wasm32::unreachable()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Native test builds have no wasm32 arch module; a valid input never
        // reaches `trap`, so panic is the native stand-in for `unreachable`.
        panic!("trap reached on non-wasm32 target")
    }
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

/// `decode-jpeg(data: Bytes) -> Result[Bytes, Text]`.
#[export_name = "decode-jpeg"]
pub extern "C" fn decode_jpeg(source_ptr: i32, source_len: i32) -> i32 {
    if source_ptr < 0 || source_len < 0 {
        trap();
    }
    let input =
        unsafe { std::slice::from_raw_parts(source_ptr as *const u8, source_len as usize) };
    match decode_jpeg_inner(input) {
        Ok(payload) => ok_area(&payload),
        Err(case) => err_area(case),
    }
}

/// `encode-jpeg(pixels: Bytes, width: U64, height: U64) -> Result[Bytes, Text]`.
#[export_name = "encode-jpeg"]
pub extern "C" fn encode_jpeg(
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
    match encode_jpeg_inner(pixels, width as u64, height as u64) {
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
    for pixel in pixels.as_chunks::<4>().0 {
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

// ---------------------------------------------------------------------------
// JPEG (RFC-0050 §2 as amended by Amendment A2, accepted 2026-11-06)
// ---------------------------------------------------------------------------

/// JPEG quality frozen by RFC-0050 §2 (standard IS tables).
const JPEG_QUALITY: u8 = 90;
/// Maximum JPEG dimension accepted by the encoder (`u16` width/height).
const MAX_JPEG_DIMENSION: u64 = 65_535;
/// JPEG start-of-image marker (`FFD8`).
const JPEG_SOI: [u8; 2] = [0xFF, 0xD8];

/// The structural facts the JPEG scan establishes before any pixel work.
struct JpegScan {
    width: u64,
    height: u64,
    /// 1 = greyscale, 3 = YCbCr.
    components: u8,
    /// The `APPn` segment identifiers seen, in order.
    app_markers: Vec<u8>,
    /// True when an `APP0` segment carried the JFIF signature.
    has_jfif: bool,
}

/// Reads a length-prefixed segment header at `offset`; returns the payload
/// start and the segment end (both exclusive/inclusive as named).
fn jpeg_segment(input: &[u8], offset: usize) -> Result<(usize, usize), &'static str> {
    if offset + 2 > input.len() {
        return Err("truncated");
    }
    let length = usize::from(u16::from_be_bytes([input[offset], input[offset + 1]]));
    if length < 2 {
        return Err("corrupt-stream");
    }
    let end = offset + length;
    if end > input.len() {
        return Err("truncated");
    }
    Ok((offset + 2, end))
}

/// Parses a baseline (`SOF0`) frame header and enforces the RFC-0050 A2
/// accepted decode profiles before any pixel allocation.
fn parse_sof0(input: &[u8], offset: usize) -> Result<(JpegScan, usize), &'static str> {
    let (start, end) = jpeg_segment(input, offset)?;
    if start + 6 > end {
        return Err("corrupt-stream");
    }
    let precision = input[start];
    if precision != 8 {
        return Err("unsupported-profile");
    }
    let height = u64::from(u16::from_be_bytes([input[start + 1], input[start + 2]]));
    let width = u64::from(u16::from_be_bytes([input[start + 3], input[start + 4]]));
    let count = input[start + 5];
    if count != 1 && count != 3 {
        return Err("unsupported-profile");
    }
    if start + 6 + usize::from(count) * 3 > end {
        return Err("corrupt-stream");
    }
    let mut luma = (0u8, 0u8);
    for index in 0..usize::from(count) {
        let base = start + 6 + index * 3;
        let sampling = input[base + 1];
        let horizontal = sampling >> 4;
        let vertical = sampling & 0x0F;
        if horizontal == 0 || vertical == 0 {
            return Err("unsupported-profile");
        }
        if index == 0 {
            luma = (horizontal, vertical);
        } else if horizontal != 1 || vertical != 1 {
            return Err("unsupported-profile");
        }
    }
    if count == 1 {
        if luma != (1, 1) {
            return Err("unsupported-profile");
        }
    } else {
        // Only 4:4:4 / 4:2:2 / 4:2:0 are accepted (RFC-0050 A2 §4).
        match luma {
            (1, 1) | (2, 1) | (2, 2) => {}
            _ => return Err("unsupported-profile"),
        }
    }
    if width == 0 || height == 0 {
        return Err("oversized");
    }
    let pixels = width.checked_mul(height).ok_or("oversized")?;
    if pixels > MAX_PIXELS {
        return Err("oversized");
    }
    Ok((
        JpegScan {
            width,
            height,
            components: count,
            app_markers: Vec::new(),
            has_jfif: false,
        },
        end,
    ))
}

/// Skips entropy-coded data, honouring byte stuffing (`FF00`) and restart
/// markers (`FFD0`–`FFD7`); returns the offset of the next real marker.
fn skip_jpeg_entropy(input: &[u8], start: usize) -> Result<usize, &'static str> {
    let mut offset = start;
    while offset < input.len() {
        if input[offset] != 0xFF {
            offset += 1;
            continue;
        }
        let mut marker = offset + 1;
        while marker < input.len() && input[marker] == 0xFF {
            marker += 1;
        }
        if marker >= input.len() {
            return Err("truncated");
        }
        match input[marker] {
            0x00 => offset = marker + 1,
            0xD0..=0xD7 => offset = marker + 1,
            _ => return Ok(offset),
        }
    }
    Err("truncated")
}

/// Walks the JPEG marker structure and enforces the RFC-0050 A2 accepted
/// profiles (baseline SOF0 only, 8-bit, 1 or 3 components, 4:4:4 / 4:2:2 /
/// 4:2:0, no trailing data).
fn scan_jpeg(input: &[u8]) -> Result<JpegScan, &'static str> {
    if input.len() < 2 || input[0..2] != JPEG_SOI {
        return Err("bad-signature");
    }
    let mut offset = 2usize;
    let mut scan: Option<JpegScan> = None;
    let mut saw_scan = false;
    loop {
        if offset >= input.len() {
            return Err("truncated");
        }
        if input[offset] != 0xFF {
            return Err("corrupt-stream");
        }
        while offset < input.len() && input[offset] == 0xFF {
            offset += 1;
        }
        if offset >= input.len() {
            return Err("truncated");
        }
        let marker = input[offset];
        offset += 1;
        match marker {
            // EOI: the stream ends here; anything after it is trailing data.
            0xD9 => {
                if !saw_scan {
                    return Err("corrupt-stream");
                }
                if offset != input.len() {
                    return Err("trailing-data");
                }
                return scan.ok_or("corrupt-stream");
            }
            // Start of scan: the entropy-coded segment follows the header.
            0xDA => {
                if scan.is_none() {
                    return Err("corrupt-stream");
                }
                let (start, end) = jpeg_segment(input, offset)?;
                if start + 1 > end {
                    return Err("corrupt-stream");
                }
                offset = skip_jpeg_entropy(input, end)?;
                saw_scan = true;
            }
            // Baseline frame header.
            0xC0 => {
                if scan.is_some() {
                    return Err("corrupt-stream");
                }
                let (parsed, end) = parse_sof0(input, offset)?;
                scan = Some(parsed);
                offset = end;
            }
            // Every other start-of-frame process is refused (progressive,
            // lossless, hierarchical, arithmetic, extended sequential).
            0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD | 0xCE
            | 0xCF => return Err("unsupported-profile"),
            // Application segments (EXIF/ICC/JFIF are ignored, not applied).
            0xE0..=0xEF => {
                let (start, end) = jpeg_segment(input, offset)?;
                if let Some(current) = scan.as_mut() {
                    current.app_markers.push(marker - 0xE0);
                    if marker == 0xE0 && start + 5 <= end && &input[start..start + 5] == b"JFIF\0" {
                        current.has_jfif = true;
                    }
                }
                offset = end;
            }
            // Standalone markers carry no payload.
            0x01 | 0xD0..=0xD7 => {}
            // Everything else is a length-prefixed segment.
            _ => {
                let (_start, end) = jpeg_segment(input, offset)?;
                offset = end;
            }
        }
    }
}

/// Maps a `jpeg-decoder` failure onto the RFC-0050 §3 closed case set.
fn classify_jpeg_error(error: jpeg_decoder::Error) -> &'static str {
    match error {
        jpeg_decoder::Error::Unsupported(_) => "unsupported-profile",
        jpeg_decoder::Error::Format(_) => "corrupt-stream",
        jpeg_decoder::Error::Io(io) => {
            if io.kind() == std::io::ErrorKind::UnexpectedEof {
                "truncated"
            } else {
                "corrupt-stream"
            }
        }
        jpeg_decoder::Error::Internal(_) => "corrupt-stream",
    }
}

/// Maps a `jpeg-encoder` failure onto the RFC-0050 §3 closed case set.
fn classify_jpeg_encode_error(error: jpeg_encoder::EncodingError) -> &'static str {
    match error {
        jpeg_encoder::EncodingError::IoError(io) => {
            if io.kind() == std::io::ErrorKind::UnexpectedEof {
                "truncated"
            } else {
                "corrupt-stream"
            }
        }
        jpeg_encoder::EncodingError::Write(_) => "corrupt-stream",
        jpeg_encoder::EncodingError::BadImageData { .. } => "encode-buffer-mismatch",
        jpeg_encoder::EncodingError::ZeroImageDimensions { .. } => "oversized",
        jpeg_encoder::EncodingError::InvalidAppSegment(_)
        | jpeg_encoder::EncodingError::AppSegmentTooLarge(_)
        | jpeg_encoder::EncodingError::IccTooLarge(_) => "corrupt-stream",
    }
}

/// Decodes a baseline JPEG into the RFC-0050 A1 flattened payload shape
/// (16-byte little-endian `width`/`height` header + RFC-0041 BGRA8 pixels).
fn decode_jpeg_inner(input: &[u8]) -> Result<Vec<u8>, &'static str> {
    let scan = scan_jpeg(input)?;
    let width = scan.width;
    let height = scan.height;
    let pixels = width.checked_mul(height).ok_or("oversized")?;

    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(input));
    // D6: keep the decode buffer ceiling strictly bounded by the crate budget
    // (`u32::MAX`) while allowing MCU-padding headroom so a tiny subsampled
    // baseline JPEG is never refused. `jpeg-decoder` enforces the limit as
    // `components × width × height`; `saturating_mul(8)` already covers that,
    // and `+4096` adds margin for any future padding accounting without
    // loosening the hard `u32::MAX` bound.
    let budget = pixels
        .saturating_mul(8)
        .saturating_add(4096)
        .min(u64::from(u32::MAX)) as usize;
    decoder.set_max_decoding_buffer_size(budget.max(1));
    decoder.read_info().map_err(classify_jpeg_error)?;
    let info = decoder.info().ok_or("corrupt-stream")?;
    if u64::from(info.width) != width || u64::from(info.height) != height {
        return Err("corrupt-stream");
    }
    let channels = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => 1usize,
        jpeg_decoder::PixelFormat::RGB24 => 3usize,
        _ => return Err("unsupported-profile"),
    };
    let decoded = decoder.decode().map_err(classify_jpeg_error)?;
    let expected = usize::try_from(pixels.checked_mul(channels as u64).ok_or("oversized")?)
        .unwrap_or_else(|_| trap());
    if decoded.len() != expected {
        return Err("corrupt-stream");
    }

    let mut payload = Vec::with_capacity(
        IMAGE_HEADER_LEN
            + usize::try_from(
                width.checked_mul(height)
                    .and_then(|count| count.checked_mul(4))
                    .unwrap_or_else(|| trap()),
            )
            .unwrap_or_else(|_| trap()),
    );
    payload.extend_from_slice(&width.to_le_bytes());
    payload.extend_from_slice(&height.to_le_bytes());
    if channels == 1 {
        for grey in decoded {
            payload.push(grey);
            payload.push(grey);
            payload.push(grey);
            payload.push(255u8);
        }
    } else {
        for pixel in decoded.as_chunks::<3>().0 {
            payload.push(pixel[2]);
            payload.push(pixel[1]);
            payload.push(pixel[0]);
            payload.push(255u8);
        }
    }
    Ok(payload)
}

/// Composites one channel over an opaque white background
/// (`round((c·a + 255·(255−a)) / 255)`, RFC-0050 A2 §2).
fn composite_over_white(channel: u8, alpha: u8) -> u8 {
    let alpha = u32::from(alpha);
    let value = u32::from(channel) * alpha + 255 * (255 - alpha) + 127;
    u8::try_from(value / 255).unwrap_or(255)
}

/// Encodes RFC-0041 BGRA8 pixels as a baseline (SOF0) Huffman JPEG at the
/// frozen quality (90) and 4:4:4 sampling, then re-scans its own output to
/// assert the frozen structure.
fn encode_jpeg_inner(pixels: &[u8], width: u64, height: u64) -> Result<Vec<u8>, &'static str> {
    if width == 0 || height == 0 {
        return Err("oversized");
    }
    if width > MAX_JPEG_DIMENSION || height > MAX_JPEG_DIMENSION {
        return Err("oversized");
    }
    let count = width.checked_mul(height).ok_or("oversized")?;
    if count > MAX_PIXELS {
        return Err("oversized");
    }
    let expected = usize::try_from(count.saturating_mul(4)).unwrap_or_else(|_| trap());
    if pixels.len() != expected {
        return Err("encode-buffer-mismatch");
    }
    let mut rgb = Vec::with_capacity(
        usize::try_from(count.saturating_mul(3)).unwrap_or_else(|_| trap()),
    );
    for pixel in pixels.as_chunks::<4>().0 {
        let (blue, green, red, alpha) = (pixel[0], pixel[1], pixel[2], pixel[3]);
        rgb.push(composite_over_white(red, alpha));
        rgb.push(composite_over_white(green, alpha));
        rgb.push(composite_over_white(blue, alpha));
    }

    let mut output: Vec<u8> = Vec::new();
    {
        let mut encoder = jpeg_encoder::Encoder::new(&mut output, JPEG_QUALITY);
        encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_4_4);
        let width_u16 = u16::try_from(width).unwrap_or_else(|_| trap());
        let height_u16 = u16::try_from(height).unwrap_or_else(|_| trap());
        encoder
            .encode(
                &rgb,
                width_u16,
                height_u16,
                jpeg_encoder::ColorType::Rgb,
            )
            .map_err(classify_jpeg_encode_error)?;
    }

    // The frozen chunk/marker set is part of the contract; verify it here so
    // a crate upgrade cannot silently change the emitted structure.
    let scan = scan_jpeg(&output)?;
    if scan.width != width || scan.height != height || scan.components != 3 {
        return Err("corrupt-stream");
    }
    if !scan.app_markers.is_empty()
        && !(scan.app_markers.len() == 1 && scan.app_markers[0] == 0 && scan.has_jfif)
    {
        return Err("corrupt-stream");
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: a tiny baseline JPEG with 4:2:0 chroma subsampling (the
    /// smallest case the decode-buffer ceiling must accommodate) round-trips
    /// successfully and is never rejected as "oversized". `jpeg-decoder`
    /// enforces its buffer limit as `components × width × height`, so the
    /// tightest budget for a subsampled image is exercised here; the
    /// MCU-padding allowance in `decode_jpeg_inner` keeps the ceiling above
    /// that requirement while staying capped at the crate's `u32::MAX` bound.
    #[test]
    fn tiny_420_jpeg_round_trip_is_not_oversized() {
        let grey = 128u8;
        for (width, height) in [(1u64, 1u64), (3u64, 3u64)] {
            let count = width * height;
            let mut rgba = Vec::with_capacity((count * 4) as usize);
            for _ in 0..count {
                rgba.extend_from_slice(&[grey, grey, grey, 255u8]);
            }
            // Encode as a 4:2:0 baseline JPEG with `jpeg-encoder`.
            let mut rgb = Vec::with_capacity((count * 3) as usize);
            for pixel in rgba.as_chunks::<4>().0 {
                rgb.push(pixel[2]);
                rgb.push(pixel[1]);
                rgb.push(pixel[0]);
            }
            let mut jpeg: Vec<u8> = Vec::new();
            {
                let mut encoder = jpeg_encoder::Encoder::new(&mut jpeg, JPEG_QUALITY);
                encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_2_0);
                encoder
                    .encode(
                        &rgb,
                        width as u16,
                        height as u16,
                        jpeg_encoder::ColorType::Rgb,
                    )
                    .expect("encode tiny 4:2:0 baseline JPEG");
            }
            // The scan must actually be 4:2:0 (luma (2,2)) to exercise the fix.
            let scan = scan_jpeg(&jpeg).expect("scan tiny 4:2:0 JPEG");
            assert_eq!(scan.width, width);
            assert_eq!(scan.height, height);
            assert_eq!(scan.components, 3);

            let payload = decode_jpeg_inner(&jpeg)
                .expect("tiny 4:2:0 JPEG must not be rejected as oversized");
            assert_eq!(&payload[0..8], &width.to_le_bytes()[..]);
            assert_eq!(&payload[8..16], &height.to_le_bytes()[..]);
            assert_eq!(payload.len(), 16 + (count * 4) as usize);
            // Lossy round-trip of a solid grey block stays within ±8/channel.
            for pixel in payload[16..].as_chunks::<4>().0 {
                assert_eq!(pixel[3], 255);
                for channel in &pixel[0..3] {
                    assert!((i32::from(*channel) - i32::from(grey)).abs() <= 8);
                }
            }
        }
    }
}
