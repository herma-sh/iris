mod buffer;
mod line;
mod search;

pub use buffer::{Scrollback, ScrollbackConfig};
pub use line::Line;
pub use search::{SearchConfig, SearchEngine, SearchResult};

#[cfg(test)]
#[path = "../test/scrollback/tests.rs"]
mod tests;
