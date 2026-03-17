use crate::error::Result;

/// Screen position used for IME candidate windows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImePosition {
    /// Horizontal position in logical pixels.
    pub x: f32,
    /// Vertical position in logical pixels.
    pub y: f32,
}

/// IME abstraction owned by the platform layer.
pub trait ImeHandler {
    /// Updates the candidate window position.
    fn set_position(&mut self, position: ImePosition) -> Result<()>;

    /// Returns whether IME composition is currently active.
    fn active(&self) -> bool;
}

/// Placeholder IME implementation for phase 0.
#[derive(Debug, Default)]
pub struct NoopImeHandler {
    position: ImePosition,
    active: bool,
}

impl ImeHandler for NoopImeHandler {
    /// Updates the handler's stored IME candidate window position.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut handler = NoopImeHandler::default();
    /// let pos = ImePosition { x: 10.0, y: 20.0 };
    /// handler.set_position(pos).unwrap();
    /// assert_eq!(handler.position, pos);
    /// ```
    fn set_position(&mut self, position: ImePosition) -> Result<()> {
        self.position = position;
        Ok(())
    }

    /// Reports whether IME composition is currently active.
    ///
    /// # Examples
    ///
    /// ```
    /// let handler = NoopImeHandler::default();
    /// assert!(!handler.active());
    /// ```
    fn active(&self) -> bool {
        self.active
    }
}
