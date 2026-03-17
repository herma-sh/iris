use crate::error::Result;
use crate::terminal::Terminal;

/// Phase-0 parser that handles printable ASCII plus basic control characters.
#[derive(Clone, Debug, Default)]
pub struct Parser;

impl Parser {
    /// Creates a parser in the ground state.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Processes a sequence of bytes and applies their effects to the given Terminal.
    ///
    /// Control bytes in the range 0x08..=0x0d are forwarded to `terminal.execute_control`.
    /// Printable ASCII bytes in the range 0x20..=0x7e are written as characters via `terminal.write_char`.
    /// Bytes outside these ranges are ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if any terminal operation returns an error.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = Parser::new();
    /// let mut term = Terminal::new(3, 8);
    /// parser.advance(&mut term, b"ab\x08c\t\rZ\nQ").unwrap();
    /// assert_eq!(term.grid[(0, 0)].ch, 'Z');
    /// assert_eq!(term.grid[(0, 1)].ch, 'c');
    /// assert_eq!(term.grid[(1, 1)].ch, 'Q');
    /// ```
    pub fn advance(&mut self, terminal: &mut Terminal, input: &[u8]) -> Result<()> {
        for &byte in input {
            match byte {
                0x08..=0x0d => terminal.execute_control(byte)?,
                0x20..=0x7e => terminal.write_char(char::from(byte))?,
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::terminal::Terminal;

    #[test]
    fn parser_handles_basic_control_characters() {
        let mut parser = Parser::new();
        let mut terminal = Terminal::new(3, 8);

        parser.advance(&mut terminal, b"ab\x08c\t\rZ\nQ").unwrap();

        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('Z')
        );
        assert_eq!(
            terminal.grid.cell(0, 1).map(|cell| cell.character),
            Some('c')
        );
        assert_eq!(
            terminal.grid.cell(1, 1).map(|cell| cell.character),
            Some('Q')
        );
    }
}
