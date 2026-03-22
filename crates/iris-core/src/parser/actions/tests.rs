
use super::{Action, GraphicsRendition, GraphicsRenditions};
use crate::cell::Color;

#[test]
fn sgr_defaults_to_reset_when_empty() {
    assert_eq!(
        Action::parse_sgr(&[]),
        GraphicsRenditions::from(vec![GraphicsRendition::Reset])
    );
}

#[test]
fn sgr_parses_truecolor_and_reset_codes() {
    assert_eq!(
        Action::parse_sgr(&[1, 38, 2, 1, 2, 3, 49, 22]),
        GraphicsRenditions::from(vec![
            GraphicsRendition::Bold(true),
            GraphicsRendition::Foreground(Color::Rgb { r: 1, g: 2, b: 3 }),
            GraphicsRendition::Background(Color::Default),
            GraphicsRendition::Bold(false),
            GraphicsRendition::Dim(false),
        ])
    );
}

#[test]
fn sgr_parses_bright_ansi_colors() {
    assert_eq!(
        Action::parse_sgr(&[94, 103]),
        GraphicsRenditions::from(vec![
            GraphicsRendition::Foreground(Color::Ansi(12)),
            GraphicsRendition::Background(Color::Ansi(11)),
        ])
    );
}

#[test]
fn sgr_parses_supported_attribute_toggle_codes() {
    assert_eq!(
        Action::parse_sgr(&[1, 2, 3, 4, 5, 7, 8, 9, 22, 23, 24, 25, 27, 28, 29]),
        GraphicsRenditions::from(vec![
            GraphicsRendition::Bold(true),
            GraphicsRendition::Dim(true),
            GraphicsRendition::Italic(true),
            GraphicsRendition::Underline(true),
            GraphicsRendition::Blink(true),
            GraphicsRendition::Inverse(true),
            GraphicsRendition::Hidden(true),
            GraphicsRendition::Strikethrough(true),
            GraphicsRendition::Bold(false),
            GraphicsRendition::Dim(false),
            GraphicsRendition::Italic(false),
            GraphicsRendition::Underline(false),
            GraphicsRendition::Blink(false),
            GraphicsRendition::Inverse(false),
            GraphicsRendition::Hidden(false),
            GraphicsRendition::Strikethrough(false),
        ])
    );
}

#[test]
fn sgr_parses_standard_and_default_colors() {
    let foreground = Action::parse_sgr(&[30, 31, 32, 33, 34, 35, 36, 37, 39]);
    assert_eq!(
        foreground,
        GraphicsRenditions::from(vec![
            GraphicsRendition::Foreground(Color::Ansi(0)),
            GraphicsRendition::Foreground(Color::Ansi(1)),
            GraphicsRendition::Foreground(Color::Ansi(2)),
            GraphicsRendition::Foreground(Color::Ansi(3)),
            GraphicsRendition::Foreground(Color::Ansi(4)),
            GraphicsRendition::Foreground(Color::Ansi(5)),
            GraphicsRendition::Foreground(Color::Ansi(6)),
            GraphicsRendition::Foreground(Color::Ansi(7)),
            GraphicsRendition::Foreground(Color::Default),
        ])
    );

    let background = Action::parse_sgr(&[40, 41, 42, 43, 44, 45, 46, 47, 49]);
    assert_eq!(
        background,
        GraphicsRenditions::from(vec![
            GraphicsRendition::Background(Color::Ansi(0)),
            GraphicsRendition::Background(Color::Ansi(1)),
            GraphicsRendition::Background(Color::Ansi(2)),
            GraphicsRendition::Background(Color::Ansi(3)),
            GraphicsRendition::Background(Color::Ansi(4)),
            GraphicsRendition::Background(Color::Ansi(5)),
            GraphicsRendition::Background(Color::Ansi(6)),
            GraphicsRendition::Background(Color::Ansi(7)),
            GraphicsRendition::Background(Color::Default),
        ])
    );
}

#[test]
fn sgr_clamps_extended_color_components() {
    assert_eq!(
        Action::parse_sgr(&[38, 5, 999, 48, 2, 256, 257, 258]),
        GraphicsRenditions::from(vec![
            GraphicsRendition::Foreground(Color::Indexed(u8::MAX)),
            GraphicsRendition::Background(Color::Rgb {
                r: u8::MAX,
                g: u8::MAX,
                b: u8::MAX,
            }),
        ])
    );
}
