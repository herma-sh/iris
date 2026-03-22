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
    fn default() -> Self {
        Self {
            character: ' ',
            width: CellWidth::Single,
            attrs: CellAttrs::default(),
        }
    }
}

impl Cell {
    /// Creates a cell using the default attributes for the provided character.
    #[must_use]
    pub fn new(character: char) -> Self {
        Self {
            character,
            width: CellWidth::from_char(character),
            attrs: CellAttrs::default(),
        }
    }

    /// Creates a cell with explicit attributes.
    #[must_use]
    pub fn with_attrs(character: char, attrs: CellAttrs) -> Self {
        Self {
            character,
            width: CellWidth::from_char(character),
            attrs,
        }
    }

    /// Creates the hidden continuation cell for a wide character.
    #[must_use]
    pub fn continuation(attrs: CellAttrs) -> Self {
        Self {
            character: ' ',
            width: CellWidth::Continuation,
            attrs,
        }
    }

    /// Returns `true` when the cell occupies two columns.
    #[must_use]
    pub fn is_wide(self) -> bool {
        self.width == CellWidth::Double
    }

    /// Returns `true` when the cell contains a blank single-width space.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.character == ' ' && self.width == CellWidth::Single
    }

    /// Resets the cell to the default blank state.
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
    /// Computes a width classification for a Unicode scalar value.
    #[must_use]
    pub fn from_char(character: char) -> Self {
        match unicode_width::UnicodeWidthChar::width(character) {
            Some(2) => Self::Double,
            _ => Self::Single,
        }
    }

    /// Returns the number of columns occupied by this width class.
    #[must_use]
    pub fn columns(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Double => 2,
            Self::Continuation => 0,
        }
    }
}

#[cfg(test)]
mod tests;
