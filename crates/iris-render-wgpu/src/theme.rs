use std::path::{Path, PathBuf};

use iris_core::cell::{CellAttrs, CellFlags, Color};
use thiserror::Error;
use toml::Value;

use crate::cell::CellColors;

const INV_255: f32 = 1.0 / 255.0;

const DEFAULT_ANSI_COLORS: [ThemeColor; 16] = [
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

/// Errors returned when loading a renderer theme from TOML.
#[derive(Debug, Error)]
pub enum ThemeLoadError {
    /// The supplied TOML input could not be parsed.
    #[error("failed to parse theme TOML: {reason}")]
    ParseToml { reason: String },

    /// A theme field had an unexpected TOML type.
    #[error("theme field {field} must be {expected}, got {actual}")]
    InvalidFieldType {
        /// Field name.
        field: String,
        /// Expected TOML type.
        expected: &'static str,
        /// Actual TOML type.
        actual: &'static str,
    },

    /// A color field used an unsupported color format.
    #[error("theme color {field} must be #RRGGBB or #RRGGBBAA, got {value}")]
    InvalidColor { field: String, value: String },

    /// ANSI palettes must have exactly 16 entries.
    #[error("theme ansi palette must contain exactly 16 colors, got {actual}")]
    InvalidAnsiPaletteLength {
        /// Number of entries found in the parsed palette.
        actual: usize,
    },

    /// Reading a theme file failed.
    #[error("failed to read theme file {path}: {reason}")]
    ReadFile {
        /// Path that failed to load.
        path: PathBuf,
        /// I/O failure reason.
        reason: String,
    },

    /// A theme table contained an unsupported field.
    #[error("unknown theme field {field} in {section} section")]
    UnknownField {
        /// Logical section name where the unknown field was found.
        section: String,
        /// Unknown field key.
        field: String,
    },
}

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
            f32::from(self.r) * INV_255,
            f32::from(self.g) * INV_255,
            f32::from(self.b) * INV_255,
            f32::from(self.a) * INV_255,
        ]
    }

    /// Converts the color into a `wgpu` clear or draw color.
    #[must_use]
    pub fn to_wgpu_color(self) -> wgpu::Color {
        wgpu::Color {
            r: f64::from(self.r) / 255.0,
            g: f64::from(self.g) / 255.0,
            b: f64::from(self.b) / 255.0,
            a: f64::from(self.a) / 255.0,
        }
    }

    #[must_use]
    fn dimmed(self) -> Self {
        Self {
            r: dim_channel(self.r),
            g: dim_channel(self.g),
            b: dim_channel(self.b),
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
    /// Loads a theme from a TOML document.
    ///
    /// The parser accepts either top-level color fields or a nested `[colors]`
    /// table with the following keys:
    ///
    /// - `foreground = "#RRGGBB"` (or `#RRGGBBAA`)
    /// - `background = "#RRGGBB"` (or `#RRGGBBAA`)
    /// - `cursor = "#RRGGBB"` (or `#RRGGBBAA`)
    /// - `ansi = ["#RRGGBB", ...]` (exactly 16 entries)
    pub fn from_toml_str(input: &str) -> std::result::Result<Self, ThemeLoadError> {
        let root = input
            .parse::<Value>()
            .map_err(|error| ThemeLoadError::ParseToml {
                reason: error.to_string(),
            })?;

        let root_table = root
            .as_table()
            .ok_or_else(|| ThemeLoadError::InvalidFieldType {
                field: "root".to_string(),
                expected: "table",
                actual: toml_value_kind(&root),
            })?;

        let colors_table = if let Some(colors) = root_table.get("colors") {
            validate_known_fields(root_table, "root", &["colors"])?;
            let colors_table =
                colors
                    .as_table()
                    .ok_or_else(|| ThemeLoadError::InvalidFieldType {
                        field: "colors".to_string(),
                        expected: "table",
                        actual: toml_value_kind(colors),
                    })?;
            validate_known_fields(
                colors_table,
                "colors",
                &["foreground", "background", "cursor", "ansi"],
            )?;
            colors_table
        } else {
            validate_known_fields(
                root_table,
                "root",
                &["foreground", "background", "cursor", "ansi"],
            )?;
            root_table
        };

        let mut theme = Self::default();
        if let Some(color) = parse_optional_color(colors_table, "foreground")? {
            theme.foreground = color;
        }
        if let Some(color) = parse_optional_color(colors_table, "background")? {
            theme.background = color;
        }
        if let Some(color) = parse_optional_color(colors_table, "cursor")? {
            theme.cursor = color;
        }
        if let Some(ansi) = parse_optional_ansi(colors_table)? {
            theme.ansi = ansi;
        }

        Ok(theme)
    }

    /// Loads a theme from a TOML file on disk.
    pub fn from_toml_file(path: impl AsRef<Path>) -> std::result::Result<Self, ThemeLoadError> {
        let path = path.as_ref();
        let input = std::fs::read_to_string(path).map_err(|error| ThemeLoadError::ReadFile {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
        Self::from_toml_str(&input)
    }

    /// Resolves the provided cell attributes into render-ready colors.
    #[must_use]
    pub fn resolve_cell_colors(&self, attrs: CellAttrs) -> CellColors {
        let mut fg = self.resolve_foreground(attrs.fg);
        let mut bg = self.resolve_background(attrs.bg);

        if attrs.flags.contains(CellFlags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        if attrs.flags.contains(CellFlags::HIDDEN) {
            fg = bg;
        } else if attrs.flags.contains(CellFlags::DIM) {
            fg = fg.dimmed();
        }

        CellColors::new(fg.to_f32_array(), bg.to_f32_array())
    }

    /// Resolves selected cell colors by swapping resolved foreground/background.
    #[must_use]
    pub fn resolve_selected_cell_colors(&self, attrs: CellAttrs) -> CellColors {
        let mut colors = self.resolve_cell_colors(attrs);
        std::mem::swap(&mut colors.fg, &mut colors.bg);
        colors
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
            Color::Indexed(index) => indexed_color(index, &self.ansi),
            Color::Rgb { r, g, b } => ThemeColor::rgb(r, g, b),
            // Xterm-compatible extended ANSI indices wrap into the base 16-color table.
            Color::Ansi(index) => self.ansi[usize::from(index % 16)],
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            foreground: DEFAULT_ANSI_COLORS[7],
            background: ThemeColor::rgb(0x1e, 0x1e, 0x1e),
            cursor: DEFAULT_ANSI_COLORS[7],
            ansi: DEFAULT_ANSI_COLORS,
        }
    }
}

#[must_use]
fn indexed_color(index: u8, ansi: &[ThemeColor; 16]) -> ThemeColor {
    if index < 16 {
        return ansi[index as usize];
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

#[must_use]
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

#[must_use]
fn dim_channel(channel: u8) -> u8 {
    u16::from(channel).div_ceil(2) as u8
}

fn parse_optional_color(
    table: &toml::value::Table,
    key: &str,
) -> std::result::Result<Option<ThemeColor>, ThemeLoadError> {
    let Some(value) = table.get(key) else {
        return Ok(None);
    };

    let color = value
        .as_str()
        .ok_or_else(|| ThemeLoadError::InvalidFieldType {
            field: key.to_string(),
            expected: "string",
            actual: toml_value_kind(value),
        })
        .and_then(|value| parse_hex_color(key, value))?;

    Ok(Some(color))
}

fn validate_known_fields(
    table: &toml::value::Table,
    section: &str,
    allowed_keys: &[&str],
) -> std::result::Result<(), ThemeLoadError> {
    if let Some(field) = table
        .keys()
        .find(|key| !allowed_keys.contains(&key.as_str()))
    {
        return Err(ThemeLoadError::UnknownField {
            section: section.to_string(),
            field: field.clone(),
        });
    }

    Ok(())
}

fn parse_optional_ansi(
    table: &toml::value::Table,
) -> std::result::Result<Option<[ThemeColor; 16]>, ThemeLoadError> {
    let Some(value) = table.get("ansi") else {
        return Ok(None);
    };
    let array = value
        .as_array()
        .ok_or_else(|| ThemeLoadError::InvalidFieldType {
            field: "ansi".to_string(),
            expected: "array",
            actual: toml_value_kind(value),
        })?;
    if array.len() != 16 {
        return Err(ThemeLoadError::InvalidAnsiPaletteLength {
            actual: array.len(),
        });
    }

    let mut palette = [ThemeColor::rgb(0, 0, 0); 16];
    for (index, value) in array.iter().enumerate() {
        let field = format!("ansi[{index}]");
        let color = value
            .as_str()
            .ok_or_else(|| ThemeLoadError::InvalidFieldType {
                field: field.clone(),
                expected: "string",
                actual: toml_value_kind(value),
            })
            .and_then(|value| parse_hex_color(&field, value))?;
        palette[index] = color;
    }

    Ok(Some(palette))
}

fn parse_hex_color(field: &str, value: &str) -> std::result::Result<ThemeColor, ThemeLoadError> {
    let color = value.trim();
    let normalized = color
        .strip_prefix('#')
        .ok_or_else(|| ThemeLoadError::InvalidColor {
            field: field.to_string(),
            value: value.to_string(),
        })?;

    match normalized.len() {
        6 => u32::from_str_radix(normalized, 16)
            .map(|rgb| {
                ThemeColor::rgb(
                    ((rgb >> 16) & 0xff) as u8,
                    ((rgb >> 8) & 0xff) as u8,
                    (rgb & 0xff) as u8,
                )
            })
            .map_err(|_| ThemeLoadError::InvalidColor {
                field: field.to_string(),
                value: value.to_string(),
            }),
        8 => u32::from_str_radix(normalized, 16)
            .map(|rgba| {
                ThemeColor::rgba(
                    ((rgba >> 24) & 0xff) as u8,
                    ((rgba >> 16) & 0xff) as u8,
                    ((rgba >> 8) & 0xff) as u8,
                    (rgba & 0xff) as u8,
                )
            })
            .map_err(|_| ThemeLoadError::InvalidColor {
                field: field.to_string(),
                value: value.to_string(),
            }),
        _ => Err(ThemeLoadError::InvalidColor {
            field: field.to_string(),
            value: value.to_string(),
        }),
    }
}

const fn toml_value_kind(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Boolean(_) => "boolean",
        Value::Datetime(_) => "datetime",
        Value::Array(_) => "array",
        Value::Table(_) => "table",
    }
}

#[cfg(test)]
#[path = "test/theme/tests.rs"]
mod tests;
