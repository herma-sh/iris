//! Terminal state and buffer primitives for Iris.
//!
//! `iris-core` owns the terminal grid, cursor state, damage tracking, mode
//! flags, and the current parser implementation.

pub mod cell;
pub mod cursor;
pub mod damage;
pub mod error;
pub mod grid;
pub mod modes;
pub mod parser;
pub mod terminal;
pub mod utils;

pub use cell::{Cell, CellAttrs, CellFlags, CellWidth, Color};
pub use cursor::{Cursor, CursorPosition, CursorStyle, SavedCursor};
pub use damage::{DamageRegion, DamageTracker};
pub use error::{Error, Result};
pub use grid::{Grid, GridSize};
pub use modes::{Mode, TerminalModes};
pub use parser::{Action, GraphicsRendition, Parser, ParserConfig, ParserState};
pub use terminal::Terminal;
