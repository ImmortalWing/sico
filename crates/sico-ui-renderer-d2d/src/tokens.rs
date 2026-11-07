//! The frozen Fluent-subset token table (ADR-0018 §2).
//!
//! Every value in this module is part of the v0 renderer contract. The table
//! is constant: no token may be derived from the environment (system theme,
//! DPI scale, locale, accent colour), because the frame corpus hashes the
//! rendered output and an environment-derived token would silently change
//! those hashes.
//!
//! A token change is a new renderer version — bump [`TOKEN_SET_VERSION`],
//! re-freeze the frame corpus and publish the new digests. Pixel-perfect
//! `WinUI 3` parity is explicitly **not** claimed; "Fluent-like subset" is
//! the contract (ADR-0018 §2).

/// Identity of the frozen token set.
///
/// Bump this on any token change; the frame corpus records it.
pub const TOKEN_SET_VERSION: &str = "fluent-subset-v0";

/// The Fluent light-theme colour ramp plus the Windows 11 default accent.
///
/// Values are `0xRRGGBB`.
pub mod color {
    /// Accent fill (Windows 11 default accent blue, `#0078D4`).
    pub const ACCENT: u32 = 0x0000_78D4;
    /// Accent border / hover stroke.
    pub const ACCENT_BORDER: u32 = 0x0000_5FB8;
    /// Window background.
    pub const BACKGROUND: u32 = 0xF3_F3_F3;
    /// Raised surface (dropdown, image placeholder surround).
    pub const SURFACE: u32 = 0x00FF_FFFF;
    /// Control fill for secondary controls.
    pub const CONTROL_FILL: u32 = 0x00FF_FFFF;
    /// Control border.
    pub const CONTROL_BORDER: u32 = 0x00D1_D1D1;
    /// Primary text.
    pub const TEXT_PRIMARY: u32 = 0x001A_1A1A;
    /// Secondary text.
    pub const TEXT_SECONDARY: u32 = 0x005D_5D5D;
    /// Text drawn on the accent fill.
    pub const TEXT_ON_ACCENT: u32 = 0x00FF_FFFF;
    /// Progress track (unfilled part).
    pub const PROGRESS_TRACK: u32 = 0x00E6_E6E6;
    /// Image-preview placeholder fill.
    pub const IMAGE_PLACEHOLDER: u32 = 0x00E9_E9E9;
}

/// Control geometry, in 96-DPI logical pixels.
pub mod geometry {
    /// Corner radius of buttons, dropdowns and the progress bar.
    pub const CORNER_RADIUS: f32 = 4.0;
    /// Border stroke width.
    pub const BORDER_WIDTH: f32 = 1.0;
    /// Height of a button or dropdown.
    pub const CONTROL_HEIGHT: f32 = 32.0;
    /// Horizontal padding inside a control.
    pub const CONTROL_PADDING_H: f32 = 12.0;
    /// Vertical padding inside a control.
    pub const CONTROL_PADDING_V: f32 = 6.0;
    /// Gap between sibling controls.
    pub const SPACING: f32 = 8.0;
    /// Padding between the window edge and its content.
    pub const WINDOW_PADDING: f32 = 16.0;
    /// Height of a progress bar.
    pub const PROGRESS_HEIGHT: f32 = 4.0;
    /// Corner radius of a progress bar.
    pub const PROGRESS_RADIUS: f32 = 2.0;
    /// Minimum width of a button.
    pub const MIN_BUTTON_WIDTH: f32 = 64.0;
    /// Reserved width of the dropdown chevron.
    pub const DROPDOWN_CHEVRON_WIDTH: f32 = 20.0;
    /// Width of the dropdown chevron stroke.
    pub const DROPDOWN_CHEVRON_STROKE: f32 = 1.5;
    /// Maximum rendered width of a single text run before it is clipped.
    pub const TEXT_MAX_WIDTH: f32 = 4096.0;
}

/// The Segoe UI Variable type ramp.
pub mod typography {
    /// Font family requested from DirectWrite.
    pub const FONT_FAMILY: &str = "Segoe UI Variable";
    /// Locale requested from DirectWrite.
    pub const LOCALE: &str = "en-us";
    /// Caption size.
    pub const CAPTION_SIZE: f32 = 12.0;
    /// Body size.
    pub const BODY_SIZE: f32 = 14.0;
    /// Subtitle size.
    pub const SUBTITLE_SIZE: f32 = 16.0;
    /// Title size.
    pub const TITLE_SIZE: f32 = 20.0;
    /// Caption line height.
    pub const CAPTION_LINE_HEIGHT: f32 = 16.0;
    /// Body line height.
    pub const BODY_LINE_HEIGHT: f32 = 20.0;
    /// Subtitle line height.
    pub const SUBTITLE_LINE_HEIGHT: f32 = 22.0;
    /// Title line height.
    pub const TITLE_LINE_HEIGHT: f32 = 28.0;
}

/// The four text roles the v0 renderer draws.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextRole {
    /// Small secondary text.
    Caption,
    /// Default body text.
    Body,
    /// Emphasised text.
    Subtitle,
    /// Heading text.
    Title,
}

impl TextRole {
    /// Font size in logical pixels.
    #[must_use]
    pub fn size(self) -> f32 {
        match self {
            Self::Caption => typography::CAPTION_SIZE,
            Self::Body => typography::BODY_SIZE,
            Self::Subtitle => typography::SUBTITLE_SIZE,
            Self::Title => typography::TITLE_SIZE,
        }
    }

    /// Line height in logical pixels.
    #[must_use]
    pub fn line_height(self) -> f32 {
        match self {
            Self::Caption => typography::CAPTION_LINE_HEIGHT,
            Self::Body => typography::BODY_LINE_HEIGHT,
            Self::Subtitle => typography::SUBTITLE_LINE_HEIGHT,
            Self::Title => typography::TITLE_LINE_HEIGHT,
        }
    }
}

/// Splits a packed `0xRRGGBB` token into normalised D2D colour channels.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn to_color_f(rgb: u32) -> (f32, f32, f32, f32) {
    let red = ((rgb >> 16) & 0xFF) as f32 / 255.0;
    let green = ((rgb >> 8) & 0xFF) as f32 / 255.0;
    let blue = (rgb & 0xFF) as f32 / 255.0;
    (red, green, blue, 1.0)
}
