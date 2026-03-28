use std::cell::Cell as FlagCell;

use iris_core::cursor::Cursor;
use iris_core::damage::{DamageRegion, ScrollDelta};
use iris_core::grid::Grid;
use iris_core::terminal::Terminal;
use iris_core::{Line, Scrollback, ScrollbackConfig, SearchConfig};

use crate::error::Result;
use crate::font::{FontRasterizer, FontRasterizerConfig};
use crate::pipeline::{PresentPipeline, PresentUniforms};
use crate::renderer::Renderer;
use crate::surface::RendererSurface;
use crate::text_renderer::{TextRenderer, TextRendererConfig};
use crate::texture::{TextureSurface, TextureSurfaceConfig, TextureSurfaceSize};
use crate::theme::Theme;
use crate::TextUniforms;

mod internals;
use internals::{
    create_frame_surface, create_scroll_surface, damage_overlaps_region,
    frame_surface_size_for_uniforms, frame_uniforms_for_requested, is_full_grid_scroll_delta,
    normalized_scroll_delta, present_uniforms_for_requested,
};
#[cfg(test)]
use internals::{partial_scroll_copy_region, scroll_copy_region};

/// Configuration for the higher-level terminal renderer integration.
#[derive(Clone, Debug, Default)]
pub struct TerminalRendererConfig {
    /// Stateful text-renderer configuration.
    pub text: TextRendererConfig,
    /// System font rasterizer configuration.
    pub font_rasterizer: FontRasterizerConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionRowRange {
    row: usize,
    start_col: usize,
    end_col: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SelectionSnapshot {
    rows: Vec<SelectionRowRange>,
}

impl SelectionSnapshot {
    fn from_terminal(terminal: &Terminal) -> Option<Self> {
        let (start_row, end_row) = terminal.selection_row_span()?;
        let mut rows = Vec::with_capacity(end_row.saturating_sub(start_row) + 1);
        for row in start_row..=end_row {
            let Some((start_col, end_col)) = terminal.selection_row_bounds(row) else {
                continue;
            };
            rows.push(SelectionRowRange {
                row,
                start_col,
                end_col,
            });
        }

        if rows.is_empty() {
            None
        } else {
            Some(Self { rows })
        }
    }

    fn contains(&self, row: usize, col: usize) -> bool {
        let Ok(index) = self.rows.binary_search_by_key(&row, |range| range.row) else {
            return false;
        };
        let range = self.rows[index];
        col >= range.start_col && col <= range.end_col
    }

    fn push_damage_regions(&self, damage: &mut Vec<DamageRegion>) {
        for range in &self.rows {
            damage.push(DamageRegion::new(
                range.row,
                range.row,
                range.start_col,
                range.end_col,
            ));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchRowRange {
    row: usize,
    start_col: usize,
    end_col: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SearchSnapshot {
    rows: Vec<SearchRowRange>,
}

impl SearchSnapshot {
    fn from_rows(
        visible_rows: &Scrollback,
        grid_rows: usize,
        grid_cols: usize,
        config: &SearchConfig,
    ) -> Option<Self> {
        let mut rows = Vec::new();
        for candidate in visible_rows.search_with_config(config) {
            let Some(row) = visible_rows.oldest_index_by_number(candidate.line_number) else {
                continue;
            };
            if row >= grid_rows || candidate.length == 0 || candidate.column >= grid_cols {
                continue;
            }

            let end_col = candidate
                .column
                .saturating_add(candidate.length)
                .saturating_sub(1)
                .min(grid_cols.saturating_sub(1));
            if candidate.column > end_col {
                continue;
            }

            rows.push(SearchRowRange {
                row,
                start_col: candidate.column,
                end_col,
            });
        }

        if rows.is_empty() {
            return None;
        }

        rows.sort_unstable_by_key(|range| (range.row, range.start_col, range.end_col));
        let mut merged = Vec::with_capacity(rows.len());
        for range in rows {
            let Some(previous) = merged.last_mut() else {
                merged.push(range);
                continue;
            };
            if previous.row == range.row && range.start_col <= previous.end_col.saturating_add(1) {
                previous.end_col = previous.end_col.max(range.end_col);
            } else {
                merged.push(range);
            }
        }

        Some(Self { rows: merged })
    }

    fn contains(&self, row: usize, col: usize) -> bool {
        let Ok(mut index) = self.rows.binary_search_by_key(&row, |range| range.row) else {
            return false;
        };
        while index > 0 && self.rows[index - 1].row == row {
            index -= 1;
        }

        for range in self.rows[index..]
            .iter()
            .take_while(|range| range.row == row)
        {
            if col >= range.start_col && col <= range.end_col {
                return true;
            }
            if col < range.start_col {
                break;
            }
        }

        false
    }

    fn push_damage_regions(&self, damage: &mut Vec<DamageRegion>) {
        for range in &self.rows {
            damage.push(DamageRegion::new(
                range.row,
                range.row,
                range.start_col,
                range.end_col,
            ));
        }
    }
}

#[derive(Clone, Debug)]
struct SearchRowsCache {
    grid_rows: usize,
    grid_cols: usize,
    viewport_offset: usize,
    scrollback_identity: u64,
    scrollback_total_lines_seen: u64,
    scrollback_retained_len: usize,
    visible_rows: Scrollback,
}

impl SearchRowsCache {
    fn from_terminal(terminal: &Terminal) -> Option<Self> {
        let grid_rows = terminal.grid.rows();
        let grid_cols = terminal.grid.cols();
        if grid_rows == 0 || grid_cols == 0 {
            return None;
        }

        let mut visible_rows = Scrollback::new(ScrollbackConfig {
            max_lines: grid_rows.max(1),
            max_memory_bytes: None,
        });
        for row in 0..grid_rows {
            let cells = terminal.viewport_row_cells(row)?.to_vec();
            visible_rows.push(Line::new(cells, false));
        }

        Some(Self {
            grid_rows,
            grid_cols,
            viewport_offset: terminal.scrollback_view_offset(),
            scrollback_identity: terminal.scrollback().instance_id(),
            scrollback_total_lines_seen: terminal.scrollback().total_lines_seen(),
            scrollback_retained_len: terminal.scrollback().len(),
            visible_rows,
        })
    }

    fn matches_terminal_state(&self, terminal: &Terminal) -> bool {
        let scrollback = terminal.scrollback();
        self.grid_rows == terminal.grid.rows()
            && self.grid_cols == terminal.grid.cols()
            && self.viewport_offset == terminal.scrollback_view_offset()
            && self.scrollback_identity == scrollback.instance_id()
            && self.scrollback_total_lines_seen == scrollback.total_lines_seen()
            && self.scrollback_retained_len == scrollback.len()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct HighlightSnapshots<'a> {
    selection: Option<&'a SelectionSnapshot>,
    search: Option<&'a SearchSnapshot>,
}

impl HighlightSnapshots<'_> {
    fn contains(self, row: usize, col: usize) -> bool {
        self.selection
            .is_some_and(|selection| selection.contains(row, col))
            || self.search.is_some_and(|search| search.contains(row, col))
    }
}

/// Renderer-owned terminal draw state that prepares text and cursor output from
/// `iris-core` terminal state into a cached frame surface.
pub struct TerminalRenderer {
    text_renderer: TextRenderer,
    text_config: TextRendererConfig,
    font_rasterizer: FontRasterizer,
    font_rasterizer_config: FontRasterizerConfig,
    frame_surface: TextureSurface,
    scroll_surface: TextureSurface,
    present_pipeline: PresentPipeline,
    present_uniform_buffer: wgpu::Buffer,
    requested_uniforms: TextUniforms,
    present_uniforms_dirty: FlagCell<bool>,
    present_bind_group: wgpu::BindGroup,
    present_uniform_bind_group: wgpu::BindGroup,
    full_redraw_damage: Vec<DamageRegion>,
    previous_cursor: Option<Cursor>,
    previous_selection: Option<SelectionSnapshot>,
    previous_search: Option<SearchSnapshot>,
    search_rows_cache: Option<SearchRowsCache>,
    frame_initialized: bool,
}

impl TerminalRenderer {
    /// Creates a terminal renderer for the provided render-target format.
    pub fn new(
        renderer: &Renderer,
        format: wgpu::TextureFormat,
        config: TerminalRendererConfig,
    ) -> Result<Self> {
        let TerminalRendererConfig {
            text,
            font_rasterizer,
        } = config;
        let requested_uniforms = text.uniforms;
        let frame_surface = create_frame_surface(renderer, format, requested_uniforms);
        let scroll_surface = create_scroll_surface(renderer, format, requested_uniforms);
        let mut text_config = text;
        text_config.uniforms = frame_uniforms_for_requested(requested_uniforms);
        let text_renderer =
            TextRenderer::new(renderer, frame_surface.format(), text_config.clone())?;
        let font_rasterizer_config = font_rasterizer;
        let font_rasterizer = FontRasterizer::new(font_rasterizer_config.clone())?;
        let present_pipeline = renderer.create_present_pipeline(format);
        let present_uniform_buffer = present_pipeline.create_uniform_buffer(renderer.device());
        let present_uniform_bind_group =
            present_pipeline.create_uniform_bind_group(renderer.device(), &present_uniform_buffer);
        let present_bind_group =
            present_pipeline.create_texture_bind_group(renderer.device(), &frame_surface);

        let terminal_renderer = Self {
            text_renderer,
            text_config,
            font_rasterizer,
            font_rasterizer_config,
            frame_surface,
            scroll_surface,
            present_pipeline,
            present_uniform_buffer,
            requested_uniforms,
            present_uniforms_dirty: FlagCell::new(true),
            present_bind_group,
            present_uniform_bind_group,
            full_redraw_damage: Vec::with_capacity(4),
            previous_cursor: None,
            previous_selection: None,
            previous_search: None,
            search_rows_cache: None,
            frame_initialized: false,
        };
        terminal_renderer.write_present_uniforms(renderer);

        Ok(terminal_renderer)
    }

    /// Returns the active theme used for text, cursor, and clear color.
    #[must_use]
    pub const fn theme(&self) -> &Theme {
        self.text_renderer.theme()
    }

    /// Replaces the active renderer theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.text_renderer.set_theme(theme.clone());
        self.text_config.theme = theme;
        self.present_uniforms_dirty.set(true);
        self.invalidate_cached_frame();
    }

    /// Returns the current text uniforms.
    #[must_use]
    pub const fn uniforms(&self) -> TextUniforms {
        self.requested_uniforms
    }

    /// Returns the cached frame-surface size.
    #[must_use]
    pub const fn frame_surface_size(&self) -> TextureSurfaceSize {
        self.frame_surface.size()
    }

    /// Updates the viewport and cell metrics written to the uniform buffer.
    pub fn set_uniforms(&mut self, renderer: &Renderer, uniforms: TextUniforms) {
        let frame_uniforms = frame_uniforms_for_requested(uniforms);
        let frame_uniforms_changed = self.text_renderer.uniforms() != frame_uniforms;
        self.requested_uniforms = uniforms;
        self.present_uniforms_dirty.set(true);
        self.text_renderer.set_uniforms(renderer, frame_uniforms);
        self.text_config.uniforms = frame_uniforms;
        self.resize_frame_surface(renderer, uniforms);
        if frame_uniforms_changed {
            self.invalidate_cached_frame();
        }
    }

    /// Returns the configured system font size in pixels.
    #[must_use]
    pub const fn font_size_px(&self) -> f32 {
        self.font_rasterizer.font_size_px()
    }

    /// Updates the system font size used for glyph rasterization.
    ///
    /// Changing the rasterizer size rebuilds renderer-owned glyph resources so
    /// cached atlas entries cannot be reused across different font scales.
    pub fn set_font_size_px(&mut self, renderer: &Renderer, font_size_px: f32) -> Result<()> {
        if self.font_rasterizer_config.font_size_px.to_bits() == font_size_px.to_bits() {
            return Ok(());
        }

        let mut font_rasterizer_config = self.font_rasterizer_config.clone();
        font_rasterizer_config.font_size_px = font_size_px;
        let font_rasterizer = FontRasterizer::new(font_rasterizer_config.clone())?;

        let mut text_config = self.text_config.clone();
        text_config.theme = self.theme().clone();
        text_config.uniforms = frame_uniforms_for_requested(self.requested_uniforms);
        let text_renderer =
            TextRenderer::new(renderer, self.frame_surface.format(), text_config.clone())?;

        self.text_renderer = text_renderer;
        self.text_config = text_config;
        self.font_rasterizer = font_rasterizer;
        self.font_rasterizer_config = font_rasterizer_config;
        self.present_uniforms_dirty.set(true);
        self.invalidate_cached_frame();

        Ok(())
    }

    /// Returns the number of prepared text instances.
    #[must_use]
    pub const fn instance_count(&self) -> usize {
        self.text_renderer.instance_count()
    }

    /// Returns the number of prepared cursor instances.
    #[must_use]
    pub const fn cursor_instance_count(&self) -> usize {
        self.text_renderer.cursor_instance_count()
    }

    /// Prepares a full visible terminal frame from `iris-core` terminal state.
    pub fn prepare_terminal(&mut self, renderer: &Renderer, terminal: &Terminal) -> Result<()> {
        self.prepare_terminal_with_search(renderer, terminal, None)
    }

    /// Prepares a full visible terminal frame and applies optional search highlighting.
    pub fn prepare_terminal_with_search(
        &mut self,
        renderer: &Renderer,
        terminal: &Terminal,
        search: Option<&SearchConfig>,
    ) -> Result<()> {
        let selection = SelectionSnapshot::from_terminal(terminal);
        let search = self.search_snapshot_from_terminal(terminal, search, true);
        let highlights = HighlightSnapshots {
            selection: selection.as_ref(),
            search: search.as_ref(),
        };
        self.prepare_grid_and_cursor_internal(
            renderer,
            &terminal.grid,
            terminal.cursor,
            highlights,
        )?;
        self.previous_selection = selection;
        self.previous_search = search;
        Ok(())
    }

    /// Applies an incremental terminal update into the cached frame using the
    /// terminal's accumulated damage plus cursor old/new regions.
    pub fn update_terminal(&mut self, renderer: &Renderer, terminal: &mut Terminal) -> Result<()> {
        self.update_terminal_with_search(renderer, terminal, None)
    }

    /// Applies an incremental terminal update with optional search highlighting.
    pub fn update_terminal_with_search(
        &mut self,
        renderer: &Renderer,
        terminal: &mut Terminal,
        search: Option<&SearchConfig>,
    ) -> Result<()> {
        let selection = SelectionSnapshot::from_terminal(terminal);
        if !self.frame_initialized {
            let search_snapshot = self.search_snapshot_from_terminal(terminal, search, true);
            let highlights = HighlightSnapshots {
                selection: selection.as_ref(),
                search: search_snapshot.as_ref(),
            };
            let result = self.prepare_grid_and_cursor_internal(
                renderer,
                &terminal.grid,
                terminal.cursor,
                highlights,
            );
            if result.is_ok() {
                let _ = terminal.take_scroll_delta();
                let _ = terminal.take_damage();
                self.previous_selection = selection;
                self.previous_search = search_snapshot;
            }
            return result;
        }

        let scroll_delta = terminal.take_scroll_delta();
        let mut damage = terminal.take_damage();
        let search_snapshot = self.search_snapshot_from_terminal(
            terminal,
            search,
            scroll_delta.is_some() || !damage.is_empty(),
        );
        let highlights = HighlightSnapshots {
            selection: selection.as_ref(),
            search: search_snapshot.as_ref(),
        };
        self.push_selection_damage_pair(
            &mut damage,
            self.previous_selection.as_ref(),
            selection.as_ref(),
        );
        self.push_search_damage_pair(
            &mut damage,
            self.previous_search.as_ref(),
            search_snapshot.as_ref(),
        );
        let original_damage_len = damage.len();
        let result = self.update_grid_and_cursor_internal(
            renderer,
            &terminal.grid,
            &mut damage,
            scroll_delta,
            terminal.cursor,
            highlights,
        );
        self.full_redraw_damage = damage;

        match result {
            Ok(()) => {
                self.previous_selection = selection;
                self.previous_search = search_snapshot;
                Ok(())
            }
            Err(error) => {
                self.invalidate_cached_frame();
                terminal.restore_scroll_delta(scroll_delta);
                terminal.restore_damage(&self.full_redraw_damage[..original_damage_len]);
                Err(error)
            }
        }
    }

    /// Prepares a full visible frame from explicit grid and cursor state.
    pub fn prepare_grid_and_cursor(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        cursor: Cursor,
    ) -> Result<()> {
        self.prepare_grid_and_cursor_internal(
            renderer,
            grid,
            cursor,
            HighlightSnapshots::default(),
        )?;
        self.previous_selection = None;
        self.previous_search = None;
        self.search_rows_cache = None;
        Ok(())
    }

    /// Applies an incremental grid and cursor update into the cached frame.
    pub fn update_grid_and_cursor(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        scroll_delta: Option<ScrollDelta>,
        cursor: Cursor,
    ) -> Result<()> {
        if !self.frame_initialized {
            return self.prepare_grid_and_cursor(renderer, grid, cursor);
        }

        let mut full_redraw_damage = std::mem::take(&mut self.full_redraw_damage);
        full_redraw_damage.clear();
        full_redraw_damage.extend_from_slice(damage);
        let result = self.update_grid_and_cursor_internal(
            renderer,
            grid,
            &mut full_redraw_damage,
            scroll_delta,
            cursor,
            HighlightSnapshots::default(),
        );
        self.full_redraw_damage = full_redraw_damage;
        if result.is_ok() {
            self.previous_selection = None;
            self.previous_search = None;
            self.search_rows_cache = None;
        }
        result
    }

    fn prepare_grid_and_cursor_internal(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        cursor: Cursor,
        highlights: HighlightSnapshots<'_>,
    ) -> Result<()> {
        self.rebuild_full_redraw_damage(grid);
        let is_highlighted = |row: usize, col: usize| highlights.contains(row, col);
        self.text_renderer
            .prepare_grid_with_font_rasterizer_with_selection(
                renderer,
                grid,
                &self.full_redraw_damage,
                &mut self.font_rasterizer,
                false,
                is_highlighted,
            )?;
        self.text_renderer.prepare_cursor(renderer, grid, cursor)?;
        self.text_renderer
            .render_to_texture_surface(renderer, &self.frame_surface);
        self.previous_cursor = Some(cursor);
        self.frame_initialized = true;
        Ok(())
    }

    fn update_grid_and_cursor_internal(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &mut Vec<DamageRegion>,
        scroll_delta: Option<ScrollDelta>,
        cursor: Cursor,
        highlights: HighlightSnapshots<'_>,
    ) -> Result<()> {
        let original_damage_len = damage.len();
        let cursor_changed = self.previous_cursor != Some(cursor);
        let normalized_scroll = normalized_scroll_delta(scroll_delta, grid);
        if normalized_scroll.is_none() && damage.is_empty() && !cursor_changed {
            return Ok(());
        }

        let mut shifted_retained_frame = false;
        if let Some(scroll_delta) = normalized_scroll {
            shifted_retained_frame = true;
            if is_full_grid_scroll_delta(scroll_delta, grid) {
                self.shift_retained_frame_for_scroll(renderer, scroll_delta);
            } else {
                self.shift_retained_frame_for_partial_scroll(renderer, scroll_delta);
            }
        }

        if cursor_changed || shifted_retained_frame {
            self.push_cursor_damage_pair(damage, grid, self.previous_cursor, Some(cursor));
        }
        let should_prepare_cursor = if cursor_changed || shifted_retained_frame {
            true
        } else {
            self.cursor_damage_region(grid, Some(cursor))
                .is_some_and(|region| {
                    damage_overlaps_region(&damage[..original_damage_len], region)
                })
        };

        if damage.is_empty() {
            // Keep cursor state current even when no redraw work is required.
            self.previous_cursor = Some(cursor);
            return Ok(());
        }

        let is_highlighted = |row: usize, col: usize| highlights.contains(row, col);
        self.text_renderer
            .prepare_grid_with_font_rasterizer_with_selection(
                renderer,
                grid,
                damage,
                &mut self.font_rasterizer,
                true,
                is_highlighted,
            )?;
        if should_prepare_cursor {
            self.text_renderer.prepare_cursor(renderer, grid, cursor)?;
        }
        self.text_renderer
            .render_to_texture_surface_with_load(renderer, &self.frame_surface);
        self.previous_cursor = Some(cursor);
        Ok(())
    }

    /// Renders the cached frame into an off-screen texture surface.
    pub fn render_to_texture_surface(&self, renderer: &Renderer, surface: &TextureSurface) {
        self.write_present_uniforms(renderer);
        renderer.draw_present_pipeline_to_texture_surface(
            &self.present_pipeline,
            &self.present_uniform_bind_group,
            &self.present_bind_group,
            surface,
        );
    }

    /// Renders the cached frame into the next presentation frame.
    pub fn render_to_surface(
        &self,
        renderer: &Renderer,
        surface: &RendererSurface<'_>,
    ) -> Result<()> {
        let frame = surface.current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("iris-render-wgpu-terminal-renderer-surface-encoder"),
                });
        self.write_present_uniforms(renderer);
        self.present_pipeline.render(
            &mut encoder,
            &view,
            &self.present_uniform_bind_group,
            &self.present_bind_group,
        );
        renderer.queue().submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn rebuild_full_redraw_damage(&mut self, grid: &Grid) {
        self.full_redraw_damage.clear();
        if grid.rows() == 0 || grid.cols() == 0 {
            return;
        }

        self.full_redraw_damage.push(DamageRegion::new(
            0,
            grid.rows().saturating_sub(1),
            0,
            grid.cols().saturating_sub(1),
        ));
    }

    fn push_cursor_damage_pair(
        &self,
        damage: &mut Vec<DamageRegion>,
        grid: &Grid,
        previous_cursor: Option<Cursor>,
        current_cursor: Option<Cursor>,
    ) {
        if previous_cursor == current_cursor {
            if let Some(region) = self.cursor_damage_region(grid, current_cursor) {
                damage.push(region);
            }
            return;
        }

        let previous_region = self.cursor_damage_region(grid, previous_cursor);
        let current_region = self.cursor_damage_region(grid, current_cursor);

        if let Some(region) = previous_region {
            damage.push(region);
        }
        if let Some(region) = current_region {
            if Some(region) != previous_region {
                damage.push(region);
            }
        }
    }

    fn push_selection_damage_pair(
        &self,
        damage: &mut Vec<DamageRegion>,
        previous_selection: Option<&SelectionSnapshot>,
        current_selection: Option<&SelectionSnapshot>,
    ) {
        if previous_selection == current_selection {
            return;
        }

        if let Some(selection) = previous_selection {
            selection.push_damage_regions(damage);
        }
        if let Some(selection) = current_selection {
            selection.push_damage_regions(damage);
        }
    }

    fn push_search_damage_pair(
        &self,
        damage: &mut Vec<DamageRegion>,
        previous_search: Option<&SearchSnapshot>,
        current_search: Option<&SearchSnapshot>,
    ) {
        if previous_search == current_search {
            return;
        }

        if let Some(search) = previous_search {
            search.push_damage_regions(damage);
        }
        if let Some(search) = current_search {
            search.push_damage_regions(damage);
        }
    }

    fn cursor_damage_region(&self, grid: &Grid, cursor: Option<Cursor>) -> Option<DamageRegion> {
        crate::cursor::cursor_damage_region(cursor?, grid)
    }

    fn search_snapshot_from_terminal(
        &mut self,
        terminal: &Terminal,
        config: Option<&SearchConfig>,
        force_rebuild_cache: bool,
    ) -> Option<SearchSnapshot> {
        let config = config?;
        let cache_is_compatible = self
            .search_rows_cache
            .as_ref()
            .is_some_and(|cache| cache.matches_terminal_state(terminal));
        if force_rebuild_cache || !cache_is_compatible {
            self.search_rows_cache = SearchRowsCache::from_terminal(terminal);
        }

        let cache = self.search_rows_cache.as_ref()?;
        SearchSnapshot::from_rows(
            &cache.visible_rows,
            cache.grid_rows,
            cache.grid_cols,
            config,
        )
    }

    fn resize_frame_surface(&mut self, renderer: &Renderer, uniforms: TextUniforms) {
        let next_size = frame_surface_size_for_uniforms(uniforms);
        if next_size == self.frame_surface.size() {
            return;
        }

        self.frame_surface =
            create_frame_surface(renderer, self.present_pipeline.format(), uniforms);
        self.scroll_surface =
            create_scroll_surface(renderer, self.present_pipeline.format(), uniforms);
        self.present_bind_group = self
            .present_pipeline
            .create_texture_bind_group(renderer.device(), &self.frame_surface);
        self.present_uniforms_dirty.set(true);
        self.invalidate_cached_frame();
    }

    fn write_present_uniforms(&self, renderer: &Renderer) {
        if !self.present_uniforms_dirty.get() {
            return;
        }

        renderer.queue().write_buffer(
            &self.present_uniform_buffer,
            0,
            present_uniforms_for_requested(
                self.requested_uniforms,
                self.theme(),
                self.frame_surface.size(),
            )
            .as_bytes(),
        );
        self.present_uniforms_dirty.set(false);
    }

    fn invalidate_cached_frame(&mut self) {
        self.frame_initialized = false;
        self.previous_cursor = None;
        self.previous_selection = None;
        self.previous_search = None;
        self.search_rows_cache = None;
    }
}

#[cfg(test)]
#[path = "test/terminal_renderer/tests.rs"]
mod tests;
