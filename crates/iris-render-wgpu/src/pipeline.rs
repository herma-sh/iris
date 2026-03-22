mod cursor;
mod present;
mod text;

pub use cursor::CursorPipeline;
pub use present::{FullscreenPipeline, PresentPipeline, PresentUniforms};
pub use text::TextPipeline;

#[cfg(test)]
#[path = "test/pipeline/tests.rs"]
mod tests;
