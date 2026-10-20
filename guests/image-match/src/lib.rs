//! `sico:user/image-match@1` template-match guest core (M24 gate 3 roster).
//!
//! Production guest crate per ADR-0019 (Rust-compiled package core): compiled
//! to `wasm32-unknown-unknown` and committed as a digest-pinned core `.wasm`;
//! `sico_codegen_wasm::package_builder::image_match_package` embeds those bytes
//! and wraps them with `build_package_component`. The crate lives outside the
//! root workspace, is pure (import-free, no ambient FS/net/screen), and is
//! dependency-free (integer SAD only — no `png`, unlike the sibling codec).
//!
//! ABI contract copied from `guests/image-codec` (measured, not assumed):
//! - `realloc(ptr, old_size, align, new_size) -> ptr` (canonical ABI);
//! - parameters flatten to `(i32 ptr, i32 len)` per `Bytes`;
//! - a `Result` return is **one `i32`**: a pointer to a 12-byte return area the
//!   callee allocated, holding the discriminant byte at offset 0 and the
//!   payload `(ptr, len)` at offsets 4 and 8;
//! - returned buffers are intentionally leaked (no post-return by convention).
//!
//! Surface (RFC-0043 §Summary `image-match@1`, as frozen by [`A1`] for the
//! result and [`A2`] for the input):
//! `template(bmp: Bytes, tpl: Bytes) -> Result[Bytes, Text]`.
//!
//! Input framing (A2 self-describing packed header, little-endian):
//! `width u32 · height u32 · format u8 (0=bgra8,1=grey8) · reserved u8[3]=0 ·
//! pixels (width*height*bytes_per_pixel)`. Result `ok` arm (A1): a single
//! `region` = 16 bytes `x u32 · y u32 · width u32 · height u32`; `error` arm:
//! an RFC-0041 `image-error` case name as `Text`.
//!
//! The check ORDER below is part of the frozen contract: the independent
//! oracle (`prototypes/image-match-probe/tools/make-fixtures.py`) mirrors it
//! exactly, so the first violated rule selects the single returned case.

use std::alloc::{alloc as sys_alloc, realloc as sys_realloc, Layout};

/// RFC-0041 D4 allocation ceiling: a bitmap pixel payload is at most 2^28 bytes.
const MAX_PIXEL_BYTES: u64 = 1 << 28;
/// A2 header length: `width(4) + height(4) + format(1) + reserved(3)`.
const HEADER_LEN: usize = 12;

fn trap() -> ! {
    core::arch::wasm32::unreachable()
}

/// Canonical ABI allocator entry point. The lifter only ever calls it with
/// `ptr == 0`; the non-zero branch is implemented for completeness and traps on
/// an inconsistent `old_size` rather than guessing a layout.
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

/// A decoded A2 input bitmap borrowed from the caller's buffer.
struct Bitmap<'a> {
    width: u64,
    height: u64,
    format: u8,
    pixels: &'a [u8],
}

/// Bytes-per-pixel for a supported A2 format tag; `None` for unknown formats.
fn bytes_per_pixel(format: u8) -> Option<u64> {
    match format {
        0 => Some(4), // bgra8
        1 => Some(1), // grey8
        _ => None,
    }
}

/// Parses one A2 self-describing packed `Bytes` into a [`Bitmap`], refusing with
/// the first violated RFC-0041 `image-error` case in the frozen order:
/// too-short/reserved-nonzero/zero-axis/length-mismatch → `stride-mismatch`;
/// format ∉ {0,1} → `unsupported-format`; `w*h*bpp` overflow →
/// `dimension-overflow`; payload > 2^28 → `allocation-limit`.
fn decode_bitmap(b: &[u8]) -> Result<Bitmap<'_>, &'static str> {
    if b.len() < HEADER_LEN {
        return Err("stride-mismatch");
    }
    let width = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64;
    let height = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as u64;
    let format = b[8];
    if b[9] != 0 || b[10] != 0 || b[11] != 0 {
        return Err("stride-mismatch");
    }
    let Some(bpp) = bytes_per_pixel(format) else {
        return Err("unsupported-format");
    };
    if width == 0 || height == 0 {
        return Err("stride-mismatch");
    }
    let Some(pixel_bytes) = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(bpp))
    else {
        return Err("dimension-overflow");
    };
    if pixel_bytes > MAX_PIXEL_BYTES {
        return Err("allocation-limit");
    }
    if b.len() != HEADER_LEN + pixel_bytes as usize {
        return Err("stride-mismatch");
    }
    Ok(Bitmap {
        width,
        height,
        format,
        pixels: &b[HEADER_LEN..],
    })
}

/// `template(bmp: Bytes, tpl: Bytes) -> Result[Bytes, Text]` — SAD template
/// match. Returns the located `region` (the template window at the best match,
/// `width`/`height` = template dims) as a 16-byte little-endian blob, or an
/// `image-error` case name.
#[export_name = "template"]
pub extern "C" fn template(bmp_ptr: i32, bmp_len: i32, tpl_ptr: i32, tpl_len: i32) -> i32 {
    if bmp_ptr < 0 || bmp_len < 0 || tpl_ptr < 0 || tpl_len < 0 {
        trap();
    }
    let bmp_bytes = unsafe { std::slice::from_raw_parts(bmp_ptr as *const u8, bmp_len as usize) };
    let tpl_bytes = unsafe { std::slice::from_raw_parts(tpl_ptr as *const u8, tpl_len as usize) };
    match template_inner(bmp_bytes, tpl_bytes) {
        Ok(region) => ok_area(&region),
        Err(case) => err_area(case),
    }
}

/// Decodes both operands, then runs the deterministic integer SAD scan.
fn template_inner(bmp_bytes: &[u8], tpl_bytes: &[u8]) -> Result<[u8; 16], &'static str> {
    let bmp = decode_bitmap(bmp_bytes)?;
    let tpl = decode_bitmap(tpl_bytes)?;
    if bmp.format != tpl.format {
        return Err("unsupported-format");
    }
    if tpl.width > bmp.width || tpl.height > bmp.height {
        return Err("region-out-of-bounds");
    }
    let bpp = bytes_per_pixel(bmp.format).unwrap_or_else(|| trap());
    let (bw, bh) = (bmp.width, bmp.height);
    let (tw, th) = (tpl.width, tpl.height);
    // Row-major scan (y outer, x inner); strict `<` keeps the first minimum,
    // i.e. ties break by lowest (y, x) per RFC-0043 §A4.
    let mut best_x = 0u64;
    let mut best_y = 0u64;
    let mut best_sad = u64::MAX;
    for y in 0..=(bh - th) {
        for x in 0..=(bw - tw) {
            let mut sad = 0u64;
            for dy in 0..th {
                let b_base = ((y + dy) * bw + x) * bpp;
                let t_base = dy * tw * bpp;
                for dx in 0..tw {
                    let bi = (b_base + dx * bpp) as usize;
                    let ti = (t_base + dx * bpp) as usize;
                    for c in 0..bpp as usize {
                        let d = i64::from(bmp.pixels[bi + c]) - i64::from(tpl.pixels[ti + c]);
                        sad += d.unsigned_abs();
                    }
                }
            }
            if sad < best_sad {
                best_sad = sad;
                best_x = x;
                best_y = y;
            }
        }
    }
    Ok(encode_region(best_x, best_y, tw, th))
}

/// A1 result encoding: `region { x, y, width, height }` as four `u32` LE.
fn encode_region(x: u64, y: u64, width: u64, height: u64) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&(x as u32).to_le_bytes());
    out[4..8].copy_from_slice(&(y as u32).to_le_bytes());
    out[8..12].copy_from_slice(&(width as u32).to_le_bytes());
    out[12..16].copy_from_slice(&(height as u32).to_le_bytes());
    out
}
