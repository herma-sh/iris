mod engine;
mod types;

pub use engine::SelectionEngine;
pub use types::{Anchor, Selection, SelectionKind, SelectionState};

#[cfg(test)]
#[path = "../test/selection/tests.rs"]
mod tests;
