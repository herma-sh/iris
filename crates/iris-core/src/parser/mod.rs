//! ANSI/VT escape sequence parsing for `iris-core`.

mod actions;
mod control;
mod csi;
mod state;

pub use actions::{Action, GraphicsRendition};
pub use state::{Parser, ParserConfig, ParserState};
