use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use iris_core::cell::{CellAttrs, CellFlags, Color};

use super::{Theme, ThemeColor, ThemeLoadError};

struct TempFileGuard {
    path: PathBuf,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

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
fn theme_resolves_ansi_boundary_colors() {
    let theme = Theme::default();

    assert_eq!(
        theme.resolve_foreground(Color::Ansi(0)),
        ThemeColor::rgb(0x00, 0x00, 0x00)
    );
    assert_eq!(
        theme.resolve_foreground(Color::Ansi(15)),
        ThemeColor::rgb(0xff, 0xff, 0xff)
    );
    assert_eq!(theme.resolve_foreground(Color::Ansi(16)), theme.ansi[0]);
    assert_eq!(theme.resolve_foreground(Color::Ansi(17)), theme.ansi[1]);
    assert_eq!(theme.resolve_foreground(Color::Ansi(255)), theme.ansi[15]);
}

#[test]
fn theme_resolves_indexed_palette_boundaries() {
    let theme = Theme::default();

    assert_eq!(
        theme.resolve_foreground(Color::Indexed(16)),
        ThemeColor::rgb(0x00, 0x00, 0x00)
    );
    assert_eq!(
        theme.resolve_foreground(Color::Indexed(231)),
        ThemeColor::rgb(0xff, 0xff, 0xff)
    );
    assert_eq!(
        theme.resolve_foreground(Color::Indexed(232)),
        ThemeColor::rgb(0x08, 0x08, 0x08)
    );
    assert_eq!(
        theme.resolve_foreground(Color::Indexed(255)),
        ThemeColor::rgb(0xee, 0xee, 0xee)
    );
}

#[test]
fn theme_resolves_low_palette_indices_from_the_theme_palette() {
    let mut theme = Theme::default();
    theme.ansi[1] = ThemeColor::rgb(0xaa, 0xbb, 0xcc);

    assert_eq!(
        theme.resolve_foreground(Color::Indexed(1)),
        ThemeColor::rgb(0xaa, 0xbb, 0xcc)
    );
    assert_eq!(
        theme.resolve_foreground(Color::Ansi(1)),
        ThemeColor::rgb(0xaa, 0xbb, 0xcc)
    );
}

#[test]
fn theme_resolve_cell_colors_applies_inverse_flag() {
    let theme = Theme::default();
    let colors = theme.resolve_cell_colors(CellAttrs {
        fg: Color::Rgb {
            r: 0xff,
            g: 0xff,
            b: 0xff,
        },
        bg: Color::Rgb {
            r: 0x00,
            g: 0x00,
            b: 0x00,
        },
        flags: CellFlags::INVERSE,
    });

    assert_eq!(colors.fg, ThemeColor::rgb(0x00, 0x00, 0x00).to_f32_array());
    assert_eq!(colors.bg, ThemeColor::rgb(0xff, 0xff, 0xff).to_f32_array());
}

#[test]
fn theme_resolve_cell_colors_applies_dim_flag_without_hiding_dark_colors() {
    let theme = Theme::default();
    let colors = theme.resolve_cell_colors(CellAttrs {
        fg: Color::Rgb {
            r: 0x01,
            g: 0x03,
            b: 0x05,
        },
        bg: Color::Default,
        flags: CellFlags::DIM,
    });

    assert_eq!(colors.fg, ThemeColor::rgb(0x01, 0x02, 0x03).to_f32_array());
    assert_eq!(colors.bg, theme.background.to_f32_array());
}

#[test]
fn theme_resolve_cell_colors_applies_hidden_flag() {
    let theme = Theme::default();
    let colors = theme.resolve_cell_colors(CellAttrs {
        fg: Color::Ansi(2),
        bg: Color::Ansi(3),
        flags: CellFlags::HIDDEN,
    });

    assert_eq!(colors.fg, colors.bg);
}

#[test]
fn theme_resolve_cell_colors_prioritizes_hidden_over_dim() {
    let theme = Theme::default();
    let colors = theme.resolve_cell_colors(CellAttrs {
        fg: Color::Rgb {
            r: 0x80,
            g: 0x40,
            b: 0x20,
        },
        bg: Color::Ansi(4),
        flags: CellFlags::DIM | CellFlags::HIDDEN,
    });

    assert_eq!(colors.fg, colors.bg);
}

#[test]
fn theme_color_converts_to_normalized_channels() {
    let color = ThemeColor::rgba(0x80, 0x40, 0x20, 0xff);

    assert_eq!(
        color.to_f32_array(),
        [128.0 / 255.0, 64.0 / 255.0, 32.0 / 255.0, 1.0]
    );
}

#[test]
fn theme_color_converts_to_wgpu_channels() {
    let color = ThemeColor::rgba(0x80, 0x40, 0x20, 0xff);

    assert_eq!(
        color.to_wgpu_color(),
        wgpu::Color {
            r: 128.0 / 255.0,
            g: 64.0 / 255.0,
            b: 32.0 / 255.0,
            a: 1.0,
        }
    );
}

#[test]
fn theme_loads_from_toml_colors_table() {
    let theme = Theme::from_toml_str(
        r##"
[colors]
foreground = "#d0d0d0"
background = "#101010"
cursor = "#ffcc00ff"
ansi = [
    "#000000", "#aa0000", "#00aa00", "#aa5500",
    "#0000aa", "#aa00aa", "#00aaaa", "#aaaaaa",
    "#555555", "#ff5555", "#55ff55", "#ffff55",
    "#5555ff", "#ff55ff", "#55ffff", "#ffffff"
]
"##,
    )
    .expect("valid colors table should load");

    assert_eq!(theme.foreground, ThemeColor::rgb(0xd0, 0xd0, 0xd0));
    assert_eq!(theme.background, ThemeColor::rgb(0x10, 0x10, 0x10));
    assert_eq!(theme.cursor, ThemeColor::rgba(0xff, 0xcc, 0x00, 0xff));
    assert_eq!(theme.ansi[1], ThemeColor::rgb(0xaa, 0x00, 0x00));
    assert_eq!(theme.ansi[15], ThemeColor::rgb(0xff, 0xff, 0xff));
}

#[test]
fn theme_loads_from_top_level_toml_fields() {
    let theme = Theme::from_toml_str(
        r##"
foreground = "#112233"
background = "#445566"
"##,
    )
    .expect("top-level fields should load");

    assert_eq!(theme.foreground, ThemeColor::rgb(0x11, 0x22, 0x33));
    assert_eq!(theme.background, ThemeColor::rgb(0x44, 0x55, 0x66));
}

#[test]
fn theme_toml_defaults_missing_fields() {
    let theme = Theme::from_toml_str(
        r##"
[colors]
foreground = "#123456"
"##,
    )
    .expect("partial themes should load");

    assert_eq!(theme.foreground, ThemeColor::rgb(0x12, 0x34, 0x56));
    assert_eq!(theme.background, Theme::default().background);
    assert_eq!(theme.ansi, Theme::default().ansi);
}

#[test]
fn theme_toml_rejects_invalid_color_values() {
    let result = Theme::from_toml_str(
        r##"
[colors]
foreground = "#12zz90"
"##,
    );

    assert!(matches!(
        result,
        Err(ThemeLoadError::InvalidColor { field, .. }) if field == "foreground"
    ));
}

#[test]
fn theme_toml_rejects_non_ascii_hex_without_panicking() {
    let invalid = format!("#{}{}{}", '\u{00E9}', '\u{00E9}', '\u{00E9}');
    let toml = format!("[colors]\nforeground = \"{invalid}\"\n");
    let result = Theme::from_toml_str(&toml);

    assert!(matches!(
        result,
        Err(ThemeLoadError::InvalidColor { field, value })
        if field == "foreground" && value == invalid
    ));
}

#[test]
fn theme_toml_rejects_missing_hash_prefix() {
    let result = Theme::from_toml_str(
        r##"
[colors]
foreground = "123456"
"##,
    );

    assert!(matches!(
        result,
        Err(ThemeLoadError::InvalidColor { field, value })
        if field == "foreground" && value == "123456"
    ));
}

#[test]
fn theme_toml_accepts_mixed_case_hex() {
    let theme = Theme::from_toml_str(
        r##"
[colors]
foreground = "#AaBbCc"
"##,
    )
    .expect("mixed-case hex should parse");

    assert_eq!(theme.foreground, ThemeColor::rgb(0xaa, 0xbb, 0xcc));
}

#[test]
fn theme_toml_trims_surrounding_color_whitespace() {
    let theme = Theme::from_toml_str(
        r##"
[colors]
foreground = "  #aAbBcC  "
"##,
    )
    .expect("surrounding color whitespace should be ignored");

    assert_eq!(theme.foreground, ThemeColor::rgb(0xaa, 0xbb, 0xcc));
}

#[test]
fn theme_toml_rejects_invalid_ansi_palette_length() {
    let result = Theme::from_toml_str(
        r##"
[colors]
ansi = ["#000000", "#ffffff"]
"##,
    );

    assert!(matches!(
        result,
        Err(ThemeLoadError::InvalidAnsiPaletteLength { actual: 2 })
    ));
}

#[test]
fn theme_loads_from_toml_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("iris-theme-{unique}.toml"));
    let guard = TempFileGuard::new(path);
    std::fs::write(
        guard.path(),
        r##"
[colors]
cursor = "#abcdef"
"##,
    )
    .expect("temp theme file should be writable");

    let loaded = Theme::from_toml_file(guard.path()).expect("theme file should parse");

    assert_eq!(loaded.cursor, ThemeColor::rgb(0xab, 0xcd, 0xef));
}

#[test]
fn theme_toml_rejects_invalid_toml_documents() {
    let result = Theme::from_toml_str(
        r##"
[colors
foreground = "#ffffff"
"##,
    );

    assert!(matches!(result, Err(ThemeLoadError::ParseToml { .. })));
}

#[test]
fn theme_load_from_toml_file_returns_error_for_missing_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("missing-iris-theme-{unique}.toml"));

    let result = Theme::from_toml_file(&path);

    assert!(matches!(
        result,
        Err(ThemeLoadError::ReadFile {
            path: error_path,
            ..
        }) if error_path == path
    ));
}

#[test]
fn theme_toml_rejects_non_table_colors_section() {
    let result = Theme::from_toml_str(
        r##"
colors = "#ffffff"
"##,
    );

    assert!(matches!(
        result,
        Err(ThemeLoadError::InvalidFieldType { field, expected: "table", .. }) if field == "colors"
    ));
}

#[test]
fn theme_toml_rejects_unknown_root_fields() {
    let result = Theme::from_toml_str(
        r##"
foreground = "#ffffff"
bogus = "#000000"
"##,
    );

    assert!(matches!(
        result,
        Err(ThemeLoadError::UnknownField { section, field })
        if section == "root" && field == "bogus"
    ));
}

#[test]
fn theme_toml_rejects_unknown_fields_inside_colors_section() {
    let result = Theme::from_toml_str(
        r##"
[colors]
foreground = "#ffffff"
foregroun = "#000000"
"##,
    );

    assert!(matches!(
        result,
        Err(ThemeLoadError::UnknownField { section, field })
        if section == "colors" && field == "foregroun"
    ));
}

#[test]
fn theme_toml_rejects_mixed_root_and_colors_layout() {
    let result = Theme::from_toml_str(
        r##"
foreground = "#ffffff"
[colors]
background = "#000000"
"##,
    );

    assert!(matches!(
        result,
        Err(ThemeLoadError::UnknownField { section, field })
        if section == "root" && field == "foreground"
    ));
}
