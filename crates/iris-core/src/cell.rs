use bitflags::bitflags;

/// A single terminal cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    /// The character stored in the cell.
    pub character: char,
    /// The visible width of the cell.
    pub width: CellWidth,
    /// Text attributes for the cell.
    pub attrs: CellAttrs,
}

impl Default for Cell {
    /// Creates a blank cell with a space character, single-column width, and default attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// let c = Cell::default();
    /// assert_eq!(c.character, ' ');
    /// assert_eq!(c.width, CellWidth::Single);
    /// assert_eq!(c.attrs, CellAttrs::default());
    /// ```
    fn default() -> Self {
        Self {
            character: ' ',
            width: CellWidth::Single,
            attrs: CellAttrs::default(),
        }
    }
}

impl Cell {
    /// Create a `Cell` for `character` with default styling attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// let c = Cell::new('a');
    /// assert_eq!(c.character, 'a');
    /// assert_eq!(c.width.columns(), 1);
    /// ```
    #[must_use]
    pub fn new(character: char) -> Self {
        Self {
            character,
            width: CellWidth::from_char(character),
            attrs: CellAttrs::default(),
        }
    }

    /// Create a Cell from a character and explicit attributes.
    ///
    /// The cell's width is determined from the provided character.
    ///
    /// # Examples
    ///
    /// ```
    /// use iris_core::cell::{Cell, CellAttrs, CellFlags, CellWidth, Color};
    ///
    /// let attrs = CellAttrs {
    ///     fg: Color::Default,
    ///     bg: Color::Default,
    ///     flags: CellFlags::empty(),
    /// };
    /// let c = Cell::with_attrs('a', attrs);
    /// assert_eq!(c.character, 'a');
    /// assert_eq!(c.width, CellWidth::Single);
    /// ```
    #[must_use]
    pub fn with_attrs(character: char, attrs: CellAttrs) -> Self {
        Self {
            character,
            width: CellWidth::from_char(character),
            attrs,
        }
    }

    /// Create a continuation cell representing the trailing half of a wide character.
    ///
    /// The returned cell contains a space character, `CellWidth::Continuation` as its width,
    /// and the provided attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cell::{Cell, CellAttrs, CellWidth, Color, CellFlags};
    ///
    /// let attrs = CellAttrs {
    ///     fg: Color::Default,
    ///     bg: Color::Default,
    ///     flags: CellFlags::empty(),
    /// };
    /// let cont = Cell::continuation(attrs);
    /// assert_eq!(cont.character, ' ');
    /// assert_eq!(cont.width, CellWidth::Continuation);
    /// assert_eq!(cont.attrs.flags, CellFlags::empty());
    /// ```
    #[must_use]
    pub fn continuation(attrs: CellAttrs) -> Self {
        Self {
            character: ' ',
            width: CellWidth::Continuation,
            attrs,
        }
    }

    /// Checks whether the cell occupies two columns.
    ///
    /// # Returns
    ///
    /// `true` if the cell's width is `CellWidth::Double`, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let c = Cell::new('中'); // a CJK character typically two columns
    /// assert!(c.is_wide());
    /// let a = Cell::new('a');
    /// assert!(!a.is_wide());
    /// ```
    pub fn is_wide(self) -> bool {
        self.width == CellWidth::Double
    }

    /// Reports whether the cell is a blank single-column space.
    ///
    /// This is true when the cell's character is a space and its width is `CellWidth::Single`.
    ///
    /// # Examples
    ///
    /// ```
    /// let c = Cell::default();
    /// assert!(c.is_empty());
    /// ```
    pub fn is_empty(self) -> bool {
        self.character == ' ' && self.width == CellWidth::Single
    }

    /// Reset the cell to its default blank state (space character, `CellWidth::Single`, and default attributes).
    ///
    /// # Examples
    ///
    /// ```
    /// let mut c = Cell::with_attrs('x', CellAttrs { fg: Color::Ansi(1), bg: Color::Default, flags: CellFlags::BOLD });
    /// c.reset();
    /// let d = Cell::default();
    /// assert_eq!(c.character, d.character);
    /// assert_eq!(c.width, d.width);
    /// assert_eq!(c.attrs, d.attrs);
    /// ```
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The foreground or background color applied to a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Color {
    /// Uses the terminal default color.
    #[default]
    Default,
    /// Uses one of the ANSI base colors.
    Ansi(u8),
    /// Uses the extended 256-color palette.
    Indexed(u8),
    /// Uses an RGB color.
    Rgb { r: u8, g: u8, b: u8 },
}

bitflags! {
    /// Bitflag-backed styling attributes for a cell.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct CellFlags: u16 {
        const BOLD = 0b0000_0001;
        const ITALIC = 0b0000_0010;
        const UNDERLINE = 0b0000_0100;
        const STRIKETHROUGH = 0b0000_1000;
        const INVERSE = 0b0001_0000;
        const DIM = 0b0010_0000;
        const BLINK = 0b0100_0000;
        const HIDDEN = 0b1000_0000;
    }
}

/// Rendering attributes stored alongside a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellAttrs {
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Style flags.
    pub flags: CellFlags,
}

/// The width of a cell in columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellWidth {
    /// A regular one-column cell.
    #[default]
    Single,
    /// The leading half of a wide cell.
    Double,
    /// The trailing half of a wide cell.
    Continuation,
}

impl CellWidth {
    /// Determines the cell width category for a Unicode character.
    ///
    /// Characters whose displayed width is two columns are classified as `Double`; all other characters are `Single`.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(CellWidth::from_char('a'), CellWidth::Single);
    /// assert_eq!(CellWidth::from_char('中'), CellWidth::Double);
    /// ```
    #[must_use]
    pub fn from_char(character: char) -> Self {
        match unicode_width::UnicodeWidthChar::width(character) {
            Some(2) => Self::Double,
            _ => Self::Single,
        }
    }

    /// Get the number of terminal columns represented by this width class.
    ///
    /// Returns the number of columns: `Single` -> 1, `Double` -> 2, `Continuation` -> 0.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(CellWidth::Single.columns(), 1);
    /// assert_eq!(CellWidth::Double.columns(), 2);
    /// assert_eq!(CellWidth::Continuation.columns(), 0);
    /// ```
    pub fn columns(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Double => 2,
            Self::Continuation => 0,
        }
    }
}

#[cfg(test)]
mod tests {
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
}
