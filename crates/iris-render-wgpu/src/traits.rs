use iris_core::Grid;

/// Opaque render target marker used by later phases.
pub trait RenderSurface {}

/// Rendering abstraction defined in phase 0 without a concrete GPU backend.
pub trait Renderer {
    /// Reconfigures the renderer for the visible grid dimensions.
    fn resize(&mut self, rows: usize, cols: usize);

    /// Renders the current grid state into the target surface.
    fn render(&mut self, grid: &Grid, surface: &mut dyn RenderSurface);
}
