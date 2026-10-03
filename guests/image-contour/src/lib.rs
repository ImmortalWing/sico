//! `sico:user/image-contour@1` connected-region guest core (M24 gate 3 roster).
//!
//! Production guest crate per ADR-0019 (Rust-compiled package core): compiled
//! to `wasm32-unknown-unknown` and committed as a digest-pinned core `.wasm`;
//! `sico_codegen_wasm::package_builder::image_contour_package` embeds those
//! bytes and wraps them with `build_package_component`. The crate lives outside
//! the root workspace, is pure (import-free, no ambient FS/net/screen), and is
//! dependency-free (integer connected-component labeling only).
//!
//! ABI contract copied from `guests/image-codec` / `guests/image-match` /
//! `guests/image-grid` (measured, not assumed): `realloc` canonical allocator;
//! each `Bytes` parameter flattens to `(i32 ptr, i32 len)` and each `U64` to one
//! `i64`; a `Result` return is one `i32` pointer to a self-allocated 12-byte
//! return area (discriminant at offset 0, payload `(ptr, len)` at offsets 4/8);
//! returned buffers are intentionally leaked (no post-return by convention).
//!
//! Surface (RFC-0043 §Summary `image-contour@1`, input frozen by [`A2`], result
//! list encoded by [`A1`]):
//! `find(bmp: Bytes, min-area: U64) -> Result[Bytes, Text]`.
//!
//! FROZEN v0 semantics (RFC-0043 §A4 delegates the op's exact behaviour to the
//! determinism rules + the content-asserting corpus; this is that definition):
//! - input must be a grey8 A2 bitmap; a bgra8 input is `unsupported-format`
//!   (§A4 runs these rasters on the grey8/threshold family only);
//! - foreground = pixel value `>= 128` (fixed v0 threshold, matching the
//!   `image-vision@1` occupancy-mask convention);
//! - a region is a maximal 4-connected (left/right/up/down) foreground
//!   component; the closed external region's bounding box
//!   `region { min_x, min_y, w = max_x-min_x+1, h = max_y-min_y+1 }` is emitted
//!   (integer-only; a hole inside a ring does not split it);
//! - a component is emitted iff its foreground pixel `area >= min-area` (the
//!   `>=` boundary is inclusive; `min-area` is a filter, not a parameter-accept
//!   gate, so a too-large `min-area` yields an empty result rather than a
//!   refusal);
//! - ordering is deterministic by the row-major seed (the first foreground pixel
//!   of each component in row-major order), i.e. ties resolved by lowest `(y, x)`;
//! - `list<region>` is A1-encoded as `count u32 LE` then `count x 16 B`
//!   (`x, y, width, height` u32 LE); no qualifying region yields a `count = 0`
//!   blob as **ok** (A1 §2 empty result; §A4 admits empty lists for degenerate
//!   inputs).

use std::alloc::{alloc as sys_alloc, realloc as sys_realloc, Layout};

/// RFC-0041 D4 allocation ceiling: a bitmap pixel payload is at most 2^28 bytes.
const MAX_PIXEL_BYTES: u64 = 1 << 28;
/// A2 header length: `width(4) + height(4) + format(1) + reserved(3)`.
const HEADER_LEN: usize = 12;
/// Fixed v0 foreground threshold for the grey8/threshold raster family.
const FOREGROUND_MIN: u8 = 128;

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
/// the first violated RFC-0041 `image-error` case in the frozen order (identical
/// to `guests/image-match` / `guests/image-grid`, so all three roster cores
/// reject malformed inputs alike).
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
    let Some(pixel_bytes) = width.checked_mul(height).and_then(|n| n.checked_mul(bpp)) else {
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

/// `find(bmp: Bytes, min-area: U64) -> Result[Bytes, Text]`.
#[export_name = "find"]
pub extern "C" fn find(bmp_ptr: i32, bmp_len: i32, min_area: i64) -> i32 {
    if bmp_ptr < 0 || bmp_len < 0 || min_area < 0 {
        trap();
    }
    let bmp_bytes = unsafe { std::slice::from_raw_parts(bmp_ptr as *const u8, bmp_len as usize) };
    match find_inner(bmp_bytes, min_area as u64) {
        Ok(blob) => ok_area(&blob),
        Err(case) => err_area(case),
    }
}

/// Decodes the A2 input, then emits the A1 `list<region>` blob of the closed
/// external regions whose area passes `min-area`.
fn find_inner(bmp_bytes: &[u8], min_area: u64) -> Result<Vec<u8>, &'static str> {
    let bmp = decode_bitmap(bmp_bytes)?;
    if bmp.format != 1 {
        // grey8-only op: a valid A2 bgra8 input is out of this op's contract
        // scope (§A4 grey8/threshold family) -> unsupported-format.
        return Err("unsupported-format");
    }
    let w = bmp.width as usize;
    let h = bmp.height as usize;
    let px = bmp.pixels; // grey8: bpp == 1, so len == w * h (bounded by 2^28)
    let total = w * h;
    let mut visited = vec![false; total];
    let mut regions: Vec<[u32; 4]> = Vec::new();
    // Row-major seed scan: the first unvisited foreground pixel always opens its
    // own component, so components are emitted in lowest-(y,x) seed order.
    for seed in 0..total {
        if px[seed] < FOREGROUND_MIN || visited[seed] {
            continue;
        }
        let sx = seed % w;
        let sy = seed / w;
        let (mut minx, mut maxx, mut miny, mut maxy) = (sx as u32, sx as u32, sy as u32, sy as u32);
        let mut area: u64 = 0;
        // Explicit-stack 4-connected flood fill (mark on push: no revisits, no
        // recursion). Bounding box and area are set properties, so the pop order
        // cannot affect the emitted region: the result is deterministic.
        let mut stack = vec![seed];
        visited[seed] = true;
        while let Some(idx) = stack.pop() {
            area += 1;
            let x = idx % w;
            let y = idx / w;
            if (x as u32) < minx {
                minx = x as u32;
            }
            if (x as u32) > maxx {
                maxx = x as u32;
            }
            if (y as u32) < miny {
                miny = y as u32;
            }
            if (y as u32) > maxy {
                maxy = y as u32;
            }
            if x > 0 {
                let n = idx - 1;
                if !visited[n] && px[n] >= FOREGROUND_MIN {
                    visited[n] = true;
                    stack.push(n);
                }
            }
            if x + 1 < w {
                let n = idx + 1;
                if !visited[n] && px[n] >= FOREGROUND_MIN {
                    visited[n] = true;
                    stack.push(n);
                }
            }
            if y > 0 {
                let n = idx - w;
                if !visited[n] && px[n] >= FOREGROUND_MIN {
                    visited[n] = true;
                    stack.push(n);
                }
            }
            if y + 1 < h {
                let n = idx + w;
                if !visited[n] && px[n] >= FOREGROUND_MIN {
                    visited[n] = true;
                    stack.push(n);
                }
            }
        }
        if area >= min_area {
            regions.push([minx, miny, maxx - minx + 1, maxy - miny + 1]);
        }
    }
    Ok(list_blob(&regions))
}

/// A1 result encoding for `list<region>`: `count u32 LE` then each region's
/// `x, y, width, height` as four `u32` LE.
fn list_blob(regions: &[[u32; 4]]) -> Vec<u8> {
    let count = u32::try_from(regions.len()).unwrap_or_else(|_| trap());
    let mut out = Vec::with_capacity(4 + 16 * regions.len());
    out.extend_from_slice(&count.to_le_bytes());
    for region in regions {
        for value in region {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}
