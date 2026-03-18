//! ANSI/VT escape sequence parsing for `iris-core`.

mod actions;
mod control;
mod csi;
mod dcs;
mod osc;
mod state;

pub use actions::{Action, GraphicsRendition, GraphicsRenditions, ModeParams};
pub use state::{Parser, ParserConfig, ParserState};
