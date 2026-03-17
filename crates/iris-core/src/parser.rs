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

    /// Consumes a byte slice and applies the resulting actions to the terminal.
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
        let mut terminal = Terminal::new(3, 8).unwrap();

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
