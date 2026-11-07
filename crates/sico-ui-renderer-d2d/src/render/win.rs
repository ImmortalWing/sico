//! The Direct2D renderer.
//!
//! This module is one of the two unsafe modules in the crate (ADR-0018 §1:
//! the smallest isolated adapter owns all platform-specific unsafe code; the
//! shared core stays unsafe-free). The other is [`crate::shell`], which owns
//! the OS window; this module owns the offscreen frame.
//!
//! The v0 rendering path is offscreen and headless: a GDI DIB is created
//! with an explicit BGRA channel layout, bound to a Direct2D DC render
//! target, and read back after `EndDraw`. That is what makes a frame a pure
//! function of the UI tree plus the frozen token table, and therefore
//! hashable byte-stably (ADR-0018 §3).
//!
//! Two facts were measured on the evidence host and are load-bearing:
//!
//! - the DIB must be a `BITMAPV5HEADER` with `BI_BITFIELDS` and explicit
//!   BGRA masks; a plain `BI_RGB` 32-bpp DIB is rejected;
//! - the render target must be `B8G8R8A8_UNORM` / `PREMULTIPLIED`; the
//!   `UNKNOWN` pixel format is rejected with `D2D_ERR_UNSUPPORTED_PIXEL_FORMAT`.

// The layout math casts small bounded values (client size ≤ 16 384, image
// dimensions and progress fractions well under 2^32) to `f32`/`u32`; the
// values are exactly representable or the truncation is impossible in range,
// so the pedantic cast lints are noise here. These allows are behaviour-
// neutral and do not change the frozen frame corpus.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::collections::BTreeMap;

use windows::Win32::Foundation::{HANDLE, RECT};
use windows::Win32::Graphics::Direct2D::{
    Common::{
        D2D_POINT_2F, D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F,
        D2D1_PIXEL_FORMAT,
    },
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
    D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_ROUNDED_RECT, D2D1CreateFactory, ID2D1Factory, ID2D1RenderTarget, ID2D1SolidColorBrush,
    ID2D1StrokeStyle,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory,
    IDWriteFactory, IDWriteFontCollection, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    BI_BITFIELDS, BITMAPINFO, BITMAPV5HEADER, CreateCompatibleDC, CreateDIBSection, DIB_USAGE,
    DeleteDC, DeleteObject, HDC, HGDIOBJ, SelectObject,
};
use windows::core::{ComInterface, PCWSTR};

use crate::error::UiBackendError;
use crate::frame::Frame;
use crate::tokens::{self, TextRole};
use crate::tree::{NodeKind, UiTree};

/// `LCS_sRGB` — the logical colour space of the DIB.
const LCS_SRGB: u32 = 0x7352_4742;

/// The laid-out rectangle of one node.
#[derive(Clone, Copy, Debug)]
struct LayoutBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl LayoutBox {
    fn to_rect(self) -> D2D_RECT_F {
        D2D_RECT_F {
            left: self.x,
            top: self.y,
            right: self.x + self.width,
            bottom: self.y + self.height,
        }
    }
}

/// The four DirectWrite text formats of the frozen type ramp.
struct TextFormats {
    dwrite: IDWriteFactory,
    caption: IDWriteTextFormat,
    body: IDWriteTextFormat,
    subtitle: IDWriteTextFormat,
    title: IDWriteTextFormat,
}

impl TextFormats {
    fn new() -> Result<Self, UiBackendError> {
        // SAFETY: DirectWrite's shared factory is a process-wide singleton
        // and every call below is a documented COM entry point.
        unsafe {
            let dwrite = DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_SHARED)
                .map_err(|error| render_error("DWriteCreateFactory", &error))?;
            Ok(Self {
                caption: Self::format(&dwrite, TextRole::Caption)?,
                body: Self::format(&dwrite, TextRole::Body)?,
                subtitle: Self::format(&dwrite, TextRole::Subtitle)?,
                title: Self::format(&dwrite, TextRole::Title)?,
                dwrite,
            })
        }
    }

    unsafe fn format(
        dwrite: &IDWriteFactory,
        role: TextRole,
    ) -> Result<IDWriteTextFormat, UiBackendError> {
        let family: Vec<u16> = tokens::typography::FONT_FAMILY
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let locale: Vec<u16> = tokens::typography::LOCALE
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: the factory is live, the strings are NUL-terminated UTF-16
        // buffers that outlive the call, and the returned format is owned.
        let format = unsafe {
            dwrite
                .CreateTextFormat(
                    PCWSTR::from_raw(family.as_ptr()),
                    None::<&IDWriteFontCollection>,
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    role.size(),
                    PCWSTR::from_raw(locale.as_ptr()),
                )
                .map_err(|error| render_error("CreateTextFormat", &error))?
        };
        // SAFETY: the format is live.
        unsafe {
            let _ = format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
            let _ = format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
        }
        Ok(format)
    }

    fn format_for(&self, role: TextRole) -> &IDWriteTextFormat {
        match role {
            TextRole::Caption => &self.caption,
            TextRole::Body => &self.body,
            TextRole::Subtitle => &self.subtitle,
            TextRole::Title => &self.title,
        }
    }

    /// Measures one text run in logical pixels.
    fn measure(&self, role: TextRole, text: &str) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, role.line_height());
        }
        let utf16: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: the layout is created from a live text format and a live
        // UTF-16 buffer; the metrics are filled in place.
        unsafe {
            self.dwrite
                .CreateTextLayout(
                    &utf16,
                    self.format_for(role),
                    tokens::geometry::TEXT_MAX_WIDTH,
                    tokens::geometry::TEXT_MAX_WIDTH,
                )
                .ok()
                .and_then(|layout| {
                    let mut metrics =
                        windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_METRICS::default();
                    layout.GetMetrics(&raw mut metrics).ok()?;
                    Some((metrics.width, metrics.height))
                })
                .unwrap_or((0.0, role.line_height()))
        }
    }
}

fn render_error(stage: &str, error: &windows::core::Error) -> UiBackendError {
    UiBackendError::RenderFailed {
        detail: format!("{stage}: {}", error.code().0),
    }
}

fn color(rgb: u32) -> D2D1_COLOR_F {
    let (red, green, blue, alpha) = tokens::to_color_f(rgb);
    D2D1_COLOR_F {
        r: red,
        g: green,
        b: blue,
        a: alpha,
    }
}

/// The BGRA DIB description the Direct2D DC render target accepts.
#[repr(C)]
struct BitmapInfoV5 {
    header: BITMAPV5HEADER,
    masks: [u32; 3],
}

impl BitmapInfoV5 {
    fn new(width: i32, height: i32) -> Self {
        let mut info = Self {
            header: BITMAPV5HEADER::default(),
            masks: [0x00FF_0000, 0x0000_FF00, 0x0000_00FF],
        };
        info.header.bV5Size = std::mem::size_of::<BITMAPV5HEADER>() as u32;
        info.header.bV5Width = width;
        info.header.bV5Height = -height;
        info.header.bV5Planes = 1;
        info.header.bV5BitCount = 32;
        info.header.bV5Compression = BI_BITFIELDS;
        info.header.bV5RedMask = 0x00FF_0000;
        info.header.bV5GreenMask = 0x0000_FF00;
        info.header.bV5BlueMask = 0x0000_00FF;
        info.header.bV5AlphaMask = 0xFF00_0000;
        info.header.bV5CSType = LCS_SRGB;
        info
    }
}

/// Renders one frame of the tree.
pub(crate) fn render_frame(
    tree: &UiTree,
    width: u32,
    height: u32,
) -> Result<Frame, UiBackendError> {
    let formats = TextFormats::new()?;
    // SAFETY: every call below is a documented Windows entry point; the DIB,
    // the memory DC and the COM objects are released before returning.
    unsafe {
        let factory = D2D1CreateFactory::<ID2D1Factory>(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
            .map_err(|error| render_error("D2D1CreateFactory", &error))?;

        let dc = CreateCompatibleDC(HDC(0));
        let info = BitmapInfoV5::new(width.cast_signed(), height.cast_signed());
        let bmi: *const BITMAPINFO = (&raw const info.header).cast::<BITMAPINFO>();
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(dc, bmi, DIB_USAGE(0), &raw mut bits, HANDLE(0), 0)
            .map_err(|error| render_error("CreateDIBSection", &error))?;
        let previous = SelectObject(dc, HGDIOBJ(bitmap.0));

        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let target = factory
            .CreateDCRenderTarget(&raw const properties)
            .map_err(|error| render_error("CreateDCRenderTarget", &error))?;
        let render_target: ID2D1RenderTarget = target
            .cast()
            .map_err(|error| render_error("ID2D1RenderTarget cast", &error))?;

        let bounds = RECT {
            left: 0,
            top: 0,
            right: width.cast_signed(),
            bottom: height.cast_signed(),
        };
        target
            .BindDC(dc, &raw const bounds)
            .map_err(|error| render_error("BindDC", &error))?;
        render_target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        render_target.BeginDraw();
        render_target.Clear(Some(&color(tokens::color::BACKGROUND)));

        let boxes = layout(tree, &formats, width as f32, height as f32);
        if let Err(error) = draw_tree(tree, &render_target, &boxes, &formats) {
            let _ = render_target.EndDraw(None, None);
            SelectObject(dc, previous);
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(dc);
            return Err(error);
        }
        render_target
            .EndDraw(None, None)
            .map_err(|error| render_error("EndDraw", &error))?;

        let byte_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|area| area.checked_mul(4))
            .ok_or(UiBackendError::RenderFailed {
                detail: "frame size overflow".to_owned(),
            })?;
        let pixels = std::slice::from_raw_parts(bits.cast::<u8>(), byte_len).to_vec();

        SelectObject(dc, previous);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);

        Frame::new(width, height, pixels).map_err(|detail| UiBackendError::RenderFailed { detail })
    }
}

/// Lays the tree out with the frozen geometry rules.
fn layout(
    tree: &UiTree,
    formats: &TextFormats,
    width: f32,
    height: f32,
) -> BTreeMap<u64, LayoutBox> {
    let mut boxes = BTreeMap::new();
    if let Some(root) = tree.root() {
        place(
            tree, formats, root, 0.0, 0.0, width, height, false, &mut boxes,
        );
    }
    boxes
}

/// Places one node and its subtree, returning the node's own box.
///
/// `stretch_width` is set for the children of a vertical container: text and
/// progress nodes take the full width of their column while controls keep
/// their intrinsic width. Children of a horizontal container never stretch.
#[allow(clippy::too_many_arguments)]
fn place(
    tree: &UiTree,
    formats: &TextFormats,
    handle: u64,
    x: f32,
    y: f32,
    available_width: f32,
    available_height: f32,
    stretch_width: bool,
    boxes: &mut BTreeMap<u64, LayoutBox>,
) -> LayoutBox {
    let Some(node) = tree.node(handle) else {
        return LayoutBox {
            x,
            y,
            width: 0.0,
            height: 0.0,
        };
    };
    let (natural_width, natural_height) = natural_size(formats, node);
    let stretches = stretch_width && matches!(node.kind, NodeKind::Text | NodeKind::Progress);
    let own = LayoutBox {
        x,
        y,
        width: if stretches {
            available_width
        } else {
            natural_width
        },
        height: natural_height,
    };
    boxes.insert(handle, own);

    match node.kind {
        NodeKind::Root => {
            let padding = tokens::geometry::WINDOW_PADDING;
            let content_x = x + padding;
            let content_y = y + padding;
            let content_width = (available_width - padding * 2.0).max(0.0);
            let content_height = (available_height - padding * 2.0).max(0.0);
            let mut cursor = content_y;
            for child in &node.children {
                let child_box = place(
                    tree,
                    formats,
                    *child,
                    content_x,
                    cursor,
                    content_width,
                    content_height,
                    true,
                    boxes,
                );
                cursor += child_box.height + tokens::geometry::SPACING;
            }
        }
        NodeKind::Column => {
            let mut cursor = y;
            for child in &node.children {
                let child_box = place(
                    tree,
                    formats,
                    *child,
                    x,
                    cursor,
                    available_width,
                    available_height,
                    true,
                    boxes,
                );
                cursor += child_box.height + tokens::geometry::SPACING;
            }
        }
        NodeKind::Row => {
            let mut cursor = x;
            for child in &node.children {
                let child_box = place(
                    tree,
                    formats,
                    *child,
                    cursor,
                    y,
                    available_width,
                    available_height,
                    false,
                    boxes,
                );
                cursor += child_box.width + tokens::geometry::SPACING;
            }
        }
        NodeKind::Text
        | NodeKind::Button
        | NodeKind::Dropdown
        | NodeKind::ImagePreview
        | NodeKind::Progress => {}
    }
    own
}

/// The intrinsic size of a node before it is placed.
fn natural_size(formats: &TextFormats, node: &crate::tree::UiNode) -> (f32, f32) {
    match node.kind {
        NodeKind::Root | NodeKind::Column | NodeKind::Row => (0.0, 0.0),
        NodeKind::Text => {
            let (width, _) = formats.measure(TextRole::Body, &node.text);
            (width, tokens::typography::BODY_LINE_HEIGHT)
        }
        NodeKind::Button => {
            let (width, _) = formats.measure(TextRole::Body, &node.text);
            (
                (width + tokens::geometry::CONTROL_PADDING_H * 2.0)
                    .max(tokens::geometry::MIN_BUTTON_WIDTH),
                tokens::geometry::CONTROL_HEIGHT,
            )
        }
        NodeKind::Dropdown => {
            let (width, _) = formats.measure(TextRole::Body, &node.text);
            (
                (width
                    + tokens::geometry::DROPDOWN_CHEVRON_WIDTH
                    + tokens::geometry::CONTROL_PADDING_H * 2.0)
                    .max(tokens::geometry::MIN_BUTTON_WIDTH),
                tokens::geometry::CONTROL_HEIGHT,
            )
        }
        NodeKind::ImagePreview => {
            let image = node.image.as_ref();
            (
                image.map_or(0.0, |image| image.width as f32),
                image.map_or(0.0, |image| image.height as f32),
            )
        }
        NodeKind::Progress => (0.0, tokens::geometry::PROGRESS_HEIGHT),
    }
}

/// Draws every node of the tree.
#[allow(clippy::too_many_lines)]
fn draw_tree(
    tree: &UiTree,
    render_target: &ID2D1RenderTarget,
    boxes: &BTreeMap<u64, LayoutBox>,
    formats: &TextFormats,
) -> Result<(), UiBackendError> {
    for (handle, node) in tree.iter() {
        let Some(box_) = boxes.get(&handle) else {
            continue;
        };
        match node.kind {
            NodeKind::Root | NodeKind::Column | NodeKind::Row => {}
            NodeKind::Text => {
                let brush = solid_brush(render_target, tokens::color::TEXT_PRIMARY)?;
                draw_text(
                    render_target,
                    formats,
                    &node.text,
                    TextRole::Body,
                    *box_,
                    &brush,
                    false,
                );
            }
            NodeKind::Button => {
                let fill = solid_brush(render_target, tokens::color::ACCENT)?;
                // SAFETY: the rounded rectangle and the brush are live for
                // the duration of the call.
                unsafe {
                    render_target.FillRoundedRectangle(
                        &rounded(*box_, tokens::geometry::CORNER_RADIUS),
                        &fill,
                    );
                }
                let ink = solid_brush(render_target, tokens::color::TEXT_ON_ACCENT)?;
                draw_text(
                    render_target,
                    formats,
                    &node.text,
                    TextRole::Body,
                    *box_,
                    &ink,
                    true,
                );
            }
            NodeKind::Dropdown => {
                let fill = solid_brush(render_target, tokens::color::CONTROL_FILL)?;
                let border = solid_brush(render_target, tokens::color::CONTROL_BORDER)?;
                // SAFETY: as above.
                unsafe {
                    render_target.FillRoundedRectangle(
                        &rounded(*box_, tokens::geometry::CORNER_RADIUS),
                        &fill,
                    );
                    render_target.DrawRoundedRectangle(
                        &rounded(*box_, tokens::geometry::CORNER_RADIUS),
                        &border,
                        tokens::geometry::BORDER_WIDTH,
                        None::<&ID2D1StrokeStyle>,
                    );
                }
                let ink = solid_brush(render_target, tokens::color::TEXT_PRIMARY)?;
                let text_box = LayoutBox {
                    x: box_.x + tokens::geometry::CONTROL_PADDING_H,
                    y: box_.y,
                    width: (box_.width - tokens::geometry::DROPDOWN_CHEVRON_WIDTH).max(0.0),
                    height: box_.height,
                };
                draw_text(
                    render_target,
                    formats,
                    &node.text,
                    TextRole::Body,
                    text_box,
                    &ink,
                    false,
                );
                let chevron = solid_brush(render_target, tokens::color::TEXT_SECONDARY)?;
                draw_chevron(render_target, *box_, &chevron);
            }
            NodeKind::ImagePreview => {
                let fill = solid_brush(render_target, tokens::color::IMAGE_PLACEHOLDER)?;
                let border = solid_brush(render_target, tokens::color::CONTROL_BORDER)?;
                // SAFETY: as above.
                unsafe {
                    render_target.FillRectangle(&box_.to_rect(), &fill);
                    render_target.DrawRectangle(
                        &box_.to_rect(),
                        &border,
                        tokens::geometry::BORDER_WIDTH,
                        None::<&ID2D1StrokeStyle>,
                    );
                }
                if let Some(image) = &node.image
                    && !image.pixels.is_empty()
                {
                    draw_image(render_target, *box_, image)?;
                }
            }
            NodeKind::Progress => {
                let track = solid_brush(render_target, tokens::color::PROGRESS_TRACK)?;
                let fill = solid_brush(render_target, tokens::color::ACCENT)?;
                let (done, of) = node.progress.unwrap_or((0, 1));
                let ratio = if of == 0 {
                    0.0
                } else {
                    (done as f32 / of as f32).clamp(0.0, 1.0)
                };
                // SAFETY: as above.
                unsafe {
                    render_target.FillRoundedRectangle(
                        &rounded(*box_, tokens::geometry::PROGRESS_RADIUS),
                        &track,
                    );
                    if ratio > 0.0 {
                        let filled = LayoutBox {
                            width: box_.width * ratio,
                            ..*box_
                        };
                        render_target.FillRoundedRectangle(
                            &rounded(filled, tokens::geometry::PROGRESS_RADIUS),
                            &fill,
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn rounded(box_: LayoutBox, radius: f32) -> D2D1_ROUNDED_RECT {
    D2D1_ROUNDED_RECT {
        rect: box_.to_rect(),
        radiusX: radius,
        radiusY: radius,
    }
}

fn solid_brush(
    render_target: &ID2D1RenderTarget,
    rgb: u32,
) -> Result<ID2D1SolidColorBrush, UiBackendError> {
    // SAFETY: the render target is live and the colour is a plain value.
    unsafe {
        render_target
            .CreateSolidColorBrush(&color(rgb), None)
            .map_err(|error| render_error("CreateSolidColorBrush", &error))
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    render_target: &ID2D1RenderTarget,
    formats: &TextFormats,
    text: &str,
    role: TextRole,
    box_: LayoutBox,
    brush: &ID2D1SolidColorBrush,
    centered: bool,
) {
    if text.is_empty() {
        return;
    }
    let format = formats.format_for(role);
    let aligned = format.clone();
    // SAFETY: the cloned format is a live DirectWrite text format.
    unsafe {
        let _ = aligned.SetTextAlignment(if centered {
            DWRITE_TEXT_ALIGNMENT_CENTER
        } else {
            DWRITE_TEXT_ALIGNMENT_LEADING
        });
        let utf16: Vec<u16> = text.encode_utf16().collect();
        render_target.DrawText(
            &utf16,
            &aligned,
            &box_.to_rect(),
            brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
}

fn draw_chevron(render_target: &ID2D1RenderTarget, box_: LayoutBox, brush: &ID2D1SolidColorBrush) {
    let center_x = box_.x + box_.width
        - tokens::geometry::CONTROL_PADDING_H
        - tokens::geometry::DROPDOWN_CHEVRON_WIDTH / 2.0;
    let center_y = box_.y + box_.height / 2.0;
    let half = tokens::geometry::DROPDOWN_CHEVRON_WIDTH / 4.0;
    // SAFETY: both points are plain values and the brush is live.
    unsafe {
        render_target.DrawLine(
            D2D_POINT_2F {
                x: center_x - half,
                y: center_y - half / 2.0,
            },
            D2D_POINT_2F {
                x: center_x,
                y: center_y + half / 2.0,
            },
            brush,
            tokens::geometry::DROPDOWN_CHEVRON_STROKE,
            None::<&ID2D1StrokeStyle>,
        );
        render_target.DrawLine(
            D2D_POINT_2F {
                x: center_x,
                y: center_y + half / 2.0,
            },
            D2D_POINT_2F {
                x: center_x + half,
                y: center_y - half / 2.0,
            },
            brush,
            tokens::geometry::DROPDOWN_CHEVRON_STROKE,
            None::<&ID2D1StrokeStyle>,
        );
    }
}

fn draw_image(
    render_target: &ID2D1RenderTarget,
    box_: LayoutBox,
    image: &crate::tree::ImageData,
) -> Result<(), UiBackendError> {
    let width = u32::try_from(image.width).unwrap_or(u32::MAX);
    let height = u32::try_from(image.height).unwrap_or(u32::MAX);
    let properties = windows::Win32::Graphics::Direct2D::D2D1_BITMAP_PROPERTIES {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
    };
    // SAFETY: the pixel buffer is a live BGRA8 allocation whose length was
    // validated against the declared dimensions when it was attached.
    unsafe {
        let bitmap = render_target
            .CreateBitmap(
                D2D_SIZE_U { width, height },
                Some(image.pixels.as_ptr().cast::<core::ffi::c_void>()),
                width * 4,
                &raw const properties,
            )
            .map_err(|error| render_error("CreateBitmap", &error))?;
        render_target.DrawBitmap(
            &bitmap,
            Some(&box_.to_rect()),
            1.0,
            D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
            None,
        );
    }
    Ok(())
}
