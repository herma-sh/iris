
use super::{Cell, CellAttrs, CellFlags, CellWidth, Color};

#[test]
fn cell_default_is_blank() {
    let cell = Cell::default();
    assert_eq!(cell.character, ' ');
    assert_eq!(cell.width, CellWidth::Single);
    assert!(cell.attrs.flags.is_empty());
}

#[test]
fn cell_width_detects_ascii() {
    assert_eq!(Cell::new('a').width, CellWidth::Single);
}

#[test]
fn cell_width_detects_cjk() {
    assert_eq!(Cell::new('中').width, CellWidth::Double);
}

#[test]
fn cell_width_allows_emoji_width_variance() {
    let width = Cell::new('😀').width;
    assert!(matches!(width, CellWidth::Single | CellWidth::Double));
}

#[test]
fn cell_with_attrs_keeps_style() {
    let attrs = CellAttrs {
        fg: Color::Ansi(2),
        bg: Color::Indexed(8),
        flags: CellFlags::BOLD | CellFlags::UNDERLINE,
    };
    let cell = Cell::with_attrs('x', attrs);
    assert_eq!(cell.attrs, attrs);
}
