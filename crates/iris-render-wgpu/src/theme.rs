use iris_core::cell::{CellAttrs, CellFlags, Color};

use crate::cell::CellColors;

const ANSI_COLORS: [ThemeColor; 16] = [
    ThemeColor::rgb(0x00, 0x00, 0x00),
    ThemeColor::rgb(0xcd, 0x31, 0x31),
    ThemeColor::rgb(0x0f, 0xa8, 0x00),
    ThemeColor::rgb(0xe5, 0xe5, 0x10),
    ThemeColor::rgb(0x24, 0x64, 0xd6),
    ThemeColor::rgb(0xbc, 0x3f, 0xbc),
    ThemeColor::rgb(0x11, 0xa8, 0xcd),
    ThemeColor::rgb(0xe5, 0xe5, 0xe5),
    ThemeColor::rgb(0x66, 0x66, 0x66),
    ThemeColor::rgb(0xf1, 0x4c, 0x4c),
    ThemeColor::rgb(0x23, 0xd1, 0x8b),
    ThemeColor::rgb(0xf5, 0xf5, 0x43),
    ThemeColor::rgb(0x3b, 0x8e, 0xff),
    ThemeColor::rgb(0xd6, 0x70, 0xd6),
    ThemeColor::rgb(0x29, 0xb8, 0xdb),
    ThemeColor::rgb(0xff, 0xff, 0xff),
];

/// RGBA color used by the renderer theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl ThemeColor {
    /// Creates an opaque RGB color.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 0xff }
    }

    /// Creates an RGBA color.
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Converts the color into normalized floating-point channels.
    #[must_use]
    pub fn to_f32_array(self) -> [f32; 4] {
        [
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
            f32::from(self.a) / 255.0,
        ]
    }

    fn dimmed(self) -> Self {
        Self {
            r: self.r / 2,
            g: self.g / 2,
            b: self.b / 2,
            a: self.a,
        }
    }
}

/// Terminal theme colors used to resolve renderable cell colors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Default foreground color.
    pub foreground: ThemeColor,
    /// Default background color.
    pub background: ThemeColor,
    /// Cursor color.
    pub cursor: ThemeColor,
    /// ANSI base and bright colors.
    pub ansi: [ThemeColor; 16],
}

impl Theme {
    /// Resolves the provided cell attributes into render-ready colors.
    #[must_use]
    pub fn resolve_cell_colors(&self, attrs: CellAttrs) -> CellColors {
        let mut fg = self.resolve_foreground(attrs.fg);
        let mut bg = self.resolve_background(attrs.bg);

        if attrs.flags.contains(CellFlags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        if attrs.flags.contains(CellFlags::DIM) {
            fg = fg.dimmed();
        }

        if attrs.flags.contains(CellFlags::HIDDEN) {
            fg = bg;
        }

        CellColors::new(fg.to_f32_array(), bg.to_f32_array())
    }

    /// Resolves a terminal color value against the theme foreground defaults.
    #[must_use]
    pub fn resolve_foreground(&self, color: Color) -> ThemeColor {
        self.resolve_color(color, self.foreground)
    }

    /// Resolves a terminal color value against the theme background defaults.
    #[must_use]
    pub fn resolve_background(&self, color: Color) -> ThemeColor {
        self.resolve_color(color, self.background)
    }

    fn resolve_color(&self, color: Color, default: ThemeColor) -> ThemeColor {
        match color {
            Color::Default => default,
            Color::Ansi(index) | Color::Indexed(index) if index < 16 => self.ansi[index as usize],
            Color::Indexed(index) => indexed_color(index),
            Color::Rgb { r, g, b } => ThemeColor::rgb(r, g, b),
            Color::Ansi(index) => self.ansi[usize::from(index % 16)],
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            foreground: ANSI_COLORS[7],
            background: ThemeColor::rgb(0x1e, 0x1e, 0x1e),
            cursor: ANSI_COLORS[7],
            ansi: ANSI_COLORS,
        }
    }
}

fn indexed_color(index: u8) -> ThemeColor {
    if index < 16 {
        return ANSI_COLORS[index as usize];
    }

    if index >= 232 {
        let shade = 8 + (index - 232) * 10;
        return ThemeColor::rgb(shade, shade, shade);
    }

    let cube = index - 16;
    let red = cube / 36;
    let green = (cube / 6) % 6;
    let blue = cube % 6;

    ThemeColor::rgb(cube_level(red), cube_level(green), cube_level(blue))
}

fn cube_level(component: u8) -> u8 {
    match component {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

#[cfg(test)]
mod tests {
    use iris_core::cell::{CellAttrs, CellFlags, Color};

    use super::{Theme, ThemeColor};

    #[test]
    fn theme_defaults_match_terminal_expectations() {
        let theme = Theme::default();

        assert_eq!(theme.foreground, ThemeColor::rgb(0xe5, 0xe5, 0xe5));
        assert_eq!(theme.background, ThemeColor::rgb(0x1e, 0x1e, 0x1e));
        assert_eq!(theme.cursor, ThemeColor::rgb(0xe5, 0xe5, 0xe5));
    }

    #[test]
    fn theme_resolves_default_and_rgb_colors() {
        let theme = Theme::default();

        assert_eq!(
            theme.resolve_foreground(Color::Default),
            ThemeColor::rgb(0xe5, 0xe5, 0xe5)
        );
        assert_eq!(
            theme.resolve_background(Color::Rgb {
                r: 0x12,
                g: 0x34,
                b: 0x56,
            }),
            ThemeColor::rgb(0x12, 0x34, 0x56)
        );
    }

    #[test]
    fn theme_resolves_ansi_and_indexed_palette_colors() {
        let theme = Theme::default();

        assert_eq!(
            theme.resolve_foreground(Color::Ansi(1)),
            ThemeColor::rgb(0xcd, 0x31, 0x31)
        );
        assert_eq!(
            theme.resolve_foreground(Color::Indexed(46)),
            ThemeColor::rgb(0x00, 0xff, 0x00)
        );
        assert_eq!(
            theme.resolve_background(Color::Indexed(244)),
            ThemeColor::rgb(0x80, 0x80, 0x80)
        );
    }

    #[test]
    fn theme_resolve_cell_colors_applies_inverse_dim_and_hidden() {
        let theme = Theme::default();
        let colors = theme.resolve_cell_colors(CellAttrs {
            fg: Color::Rgb {
                r: 0x80,
                g: 0x40,
                b: 0x20,
            },
            bg: Color::Ansi(4),
            flags: CellFlags::INVERSE | CellFlags::DIM,
        });

        assert_eq!(colors.fg, ThemeColor::rgb(0x12, 0x32, 0x6b).to_f32_array());
        assert_eq!(colors.bg, ThemeColor::rgb(0x80, 0x40, 0x20).to_f32_array());

        let hidden = theme.resolve_cell_colors(CellAttrs {
            fg: Color::Ansi(2),
            bg: Color::Ansi(3),
            flags: CellFlags::HIDDEN,
        });

        assert_eq!(hidden.fg, hidden.bg);
    }

    #[test]
    fn theme_color_converts_to_normalized_channels() {
        let color = ThemeColor::rgba(0x80, 0x40, 0x20, 0xff);

        assert_eq!(
            color.to_f32_array(),
            [128.0 / 255.0, 64.0 / 255.0, 32.0 / 255.0, 1.0]
        );
    }
}
