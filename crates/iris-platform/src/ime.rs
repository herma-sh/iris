use crate::error::Result;

/// Screen position used for IME candidate windows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImePosition {
    /// Horizontal position in logical pixels.
    pub x: f32,
    /// Vertical position in logical pixels.
    pub y: f32,
}

/// Active IME composition text and cursor state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImeComposition {
    /// Current composition text.
    pub text: String,
    /// Cursor offset within `text`.
    pub cursor: usize,
}

/// IME abstraction owned by the platform layer.
pub trait ImeHandler {
    /// Updates the candidate window position.
    fn set_position(&mut self, position: ImePosition) -> Result<()>;

    /// Starts a new IME composition session.
    fn start_composition(&mut self) -> Result<()>;

    /// Updates IME composition text and cursor.
    fn update_composition(&mut self, text: &str, cursor: usize) -> Result<()>;

    /// Commits the current composition and returns committed text.
    fn end_composition(&mut self) -> Result<Option<String>>;

    /// Cancels the current composition without committing text.
    fn cancel_composition(&mut self) -> Result<()>;

    /// Returns whether IME composition is currently active.
    fn active(&self) -> bool;

    /// Returns composition details when a composition is active.
    fn composition(&self) -> Option<&ImeComposition>;
}

/// Placeholder IME implementation for phase 0.
#[derive(Debug, Default)]
pub struct NoopImeHandler {
    position: ImePosition,
    active: bool,
    composition: Option<ImeComposition>,
}

impl ImeHandler for NoopImeHandler {
    fn set_position(&mut self, position: ImePosition) -> Result<()> {
        self.position = position;
        Ok(())
    }

    fn start_composition(&mut self) -> Result<()> {
        self.active = true;
        self.composition = Some(ImeComposition::default());
        Ok(())
    }

    fn update_composition(&mut self, text: &str, cursor: usize) -> Result<()> {
        let clamped_cursor = cursor.min(text.chars().count());
        self.active = true;
        self.composition = Some(ImeComposition {
            text: text.to_string(),
            cursor: clamped_cursor,
        });
        Ok(())
    }

    fn end_composition(&mut self) -> Result<Option<String>> {
        let committed = self.composition.take().map(|composition| composition.text);
        self.active = false;
        Ok(committed)
    }

    fn cancel_composition(&mut self) -> Result<()> {
        self.active = false;
        self.composition = None;
        Ok(())
    }

    fn active(&self) -> bool {
        self.active
    }

    fn composition(&self) -> Option<&ImeComposition> {
        self.composition.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::{ImeHandler, ImePosition, NoopImeHandler};

    #[test]
    fn noop_ime_tracks_position_and_composition_lifecycle() {
        let mut ime = NoopImeHandler::default();
        ime.set_position(ImePosition { x: 8.0, y: 12.0 }).unwrap();
        ime.start_composition().unwrap();
        ime.update_composition("kanji", 3).unwrap();

        assert!(ime.active());
        let composition = ime.composition().unwrap();
        assert_eq!(composition.text, "kanji");
        assert_eq!(composition.cursor, 3);

        let committed = ime.end_composition().unwrap();
        assert_eq!(committed.as_deref(), Some("kanji"));
        assert!(!ime.active());
        assert!(ime.composition().is_none());
    }

    #[test]
    fn noop_ime_cancel_clears_composition_without_commit() {
        let mut ime = NoopImeHandler::default();
        ime.start_composition().unwrap();
        ime.update_composition("input", 99).unwrap();
        assert_eq!(ime.composition().unwrap().cursor, 5);

        ime.cancel_composition().unwrap();
        assert!(!ime.active());
        assert!(ime.composition().is_none());
        let committed = ime.end_composition().unwrap();
        assert_eq!(committed, None);
    }
}
