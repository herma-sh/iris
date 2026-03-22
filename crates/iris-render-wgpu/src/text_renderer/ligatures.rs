use super::*;

const GLYPH_STYLE_FLAGS: CellFlags = CellFlags::BOLD.union(CellFlags::ITALIC);
const LIGATURE_CONTEXT_COLUMNS: usize = 2;

#[derive(Clone, Copy, Debug)]
pub(super) struct LigatureOverride {
    pub(super) glyph: crate::glyph::CachedGlyph,
    pub(super) span: usize,
}

impl TextRenderer {
    pub(super) fn apply_operator_ligatures(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        font_rasterizer: &mut FontRasterizer,
    ) -> Result<()> {
        if self.instances.is_empty() || self.normalized_damage.is_empty() {
            return Ok(());
        }

        self.ligature_overrides.clear();
        self.ligature_followers.clear();

        for region in &self.normalized_damage {
            let Some(row_cells) = grid.row(region.start_row) else {
                continue;
            };
            if region.end_col <= region.start_col {
                continue;
            }

            let mut col = region.start_col;
            while col < region.end_col {
                let Some((replacement_character, span)) =
                    operator_ligature_replacement_for_row(row_cells, col, region.end_col)
                else {
                    col += 1;
                    continue;
                };

                let left = row_cells[col];
                let ligature_cells = &row_cells[col..col + span];
                if left.width != CellWidth::Single
                    || ligature_cells
                        .iter()
                        .any(|cell| cell.width != CellWidth::Single || cell.attrs != left.attrs)
                {
                    col += 1;
                    continue;
                }

                let replacement_cell = Cell {
                    character: replacement_character,
                    width: CellWidth::Single,
                    attrs: left.attrs,
                };
                let replacement_key = glyph_key_for_cell(replacement_cell);
                if !self.glyph_cache.contains(replacement_key) {
                    let rasterized = match font_rasterizer.rasterize_cell(replacement_cell) {
                        Ok(Some(rasterized)) => rasterized,
                        Ok(None) => {
                            col += 1;
                            continue;
                        }
                        Err(error) => {
                            tracing::debug!(
                                ?error,
                                replacement_character = %replacement_character,
                                "skipping operator ligature replacement after rasterization failure"
                            );
                            col += 1;
                            continue;
                        }
                    };

                    if let Err(error) = renderer.cache_glyph_with_placement(
                        &mut self.glyph_cache,
                        &mut self.atlas,
                        replacement_key,
                        rasterized.as_bitmap(),
                        rasterized.placement(),
                    ) {
                        tracing::debug!(
                            ?error,
                            replacement_character = %replacement_character,
                            "skipping operator ligature replacement after cache insertion failure"
                        );
                        col += 1;
                        continue;
                    }
                }

                let Some(glyph) = self.glyph_cache.get(replacement_key).copied() else {
                    col += 1;
                    continue;
                };

                self.ligature_overrides
                    .insert((region.start_row, col), LigatureOverride { glyph, span });
                for follower_col in (col + 1)..(col + span) {
                    self.ligature_followers
                        .insert((region.start_row, follower_col));
                }
                col += span;
            }
        }

        if self.ligature_overrides.is_empty() && self.ligature_followers.is_empty() {
            return Ok(());
        }

        let atlas_size = self.atlas.size();
        self.rewritten_instances.clear();
        self.rewritten_instances.reserve(self.instances.len());

        for instance in &self.instances {
            let row = instance.grid_position[1] as usize;
            let col = instance.grid_position[0] as usize;

            if self.ligature_followers.contains(&(row, col)) {
                continue;
            }

            let Some(override_glyph) = self.ligature_overrides.get(&(row, col)).copied() else {
                self.rewritten_instances.push(*instance);
                continue;
            };

            let Some(&cell) = grid.cell(row, col) else {
                self.rewritten_instances.push(*instance);
                continue;
            };

            let row_u32 = u32::try_from(row)
                .map_err(|_| crate::error::Error::GridCoordinateOutOfRange { row, col })?;
            let col_u32 = u32::try_from(col)
                .map_err(|_| crate::error::Error::GridCoordinateOutOfRange { row, col })?;
            let mut rewritten = CellInstance::from_cell(
                cell,
                col_u32,
                row_u32,
                override_glyph.glyph,
                atlas_size,
                self.theme.resolve_cell_colors(cell.attrs),
            )?;
            rewritten.cell_span = override_glyph.span as f32;
            self.rewritten_instances.push(rewritten);
        }

        std::mem::swap(&mut self.instances, &mut self.rewritten_instances);
        renderer.write_text_instances(&mut self.buffers, &self.instances)
    }
}

/// Creates a glyph cache key for the rendered glyph shape of a cell.
///
/// Bit layout:
/// - bits `0..=31`: Unicode scalar value
/// - bits `32..=47`: shape-affecting style flags (`BOLD | ITALIC`)
/// - bits `48..=63`: width tag (`0` continuation, `1` single-width, `2` double-width)
///
/// Decorations such as underline and strikethrough are intentionally excluded
/// because they do not change glyph rasterization.
pub(super) fn glyph_key_for_cell(cell: Cell) -> GlyphKey {
    let style_bits = (cell.attrs.flags & GLYPH_STYLE_FLAGS).bits();
    let width_tag = match cell.width {
        iris_core::cell::CellWidth::Single => 1u64,
        iris_core::cell::CellWidth::Double => 2u64,
        iris_core::cell::CellWidth::Continuation => 0u64,
    };

    GlyphKey::new(
        u64::from(cell.character as u32) | (u64::from(style_bits) << 32) | (width_tag << 48),
    )
}

pub(super) fn operator_ligature_replacement(
    first: char,
    second: Option<char>,
    third: Option<char>,
) -> Option<(char, usize)> {
    match (first, second, third) {
        ('<', Some('-'), Some('>')) => Some(('\u{2194}', 3)),
        ('<', Some('='), Some('>')) => Some(('\u{21D4}', 3)),
        ('=', Some('='), Some('=')) => Some(('\u{2261}', 3)),
        ('!', Some('='), Some('=')) => Some(('\u{2262}', 3)),
        ('-', Some('>'), _) => Some(('\u{2192}', 2)),
        ('<', Some('-'), _) => Some(('\u{2190}', 2)),
        ('=', Some('>'), _) => Some(('\u{21D2}', 2)),
        ('<', Some('='), _) => Some(('\u{2264}', 2)),
        ('>', Some('='), _) => Some(('\u{2265}', 2)),
        ('!', Some('='), _) => Some(('\u{2260}', 2)),
        _ => None,
    }
}

fn operator_ligature_replacement_for_row(
    row_cells: &[Cell],
    col: usize,
    last_col: usize,
) -> Option<(char, usize)> {
    if col > last_col || col >= row_cells.len() {
        return None;
    }

    let first = row_cells[col].character;
    let second = if col < last_col {
        Some(row_cells[col + 1].character)
    } else {
        None
    };
    let third = if col + 2 <= last_col {
        Some(row_cells[col + 2].character)
    } else {
        None
    };
    operator_ligature_replacement(first, second, third)
}

pub(super) fn expand_damage_regions_for_ligature_context(
    grid: &Grid,
    damage: &[DamageRegion],
    output: &mut Vec<DamageRegion>,
) {
    output.clear();
    if grid.cols() == 0 {
        return;
    }

    let last_col = grid.cols().saturating_sub(1);
    for region in damage {
        let context_start_col = region.start_col.saturating_sub(LIGATURE_CONTEXT_COLUMNS);
        let context_end_col = region
            .end_col
            .saturating_add(LIGATURE_CONTEXT_COLUMNS)
            .min(last_col);

        let mut needs_context = false;
        for row_index in region.start_row..=region.end_row {
            let Some(row_cells) = grid.row(row_index) else {
                continue;
            };

            if operator_ligature_crosses_damage_boundary(
                row_cells,
                region.start_col,
                region.end_col,
                last_col,
            ) {
                needs_context = true;
                break;
            }
        }

        let (start_col, end_col) = if needs_context {
            (context_start_col, context_end_col)
        } else {
            (region.start_col, region.end_col)
        };
        output.push(DamageRegion::new(
            region.start_row,
            region.end_row,
            start_col,
            end_col,
        ));
    }
}

fn operator_ligature_crosses_damage_boundary(
    row_cells: &[Cell],
    start_col: usize,
    end_col: usize,
    last_col: usize,
) -> bool {
    if row_cells.is_empty() || start_col > end_col || start_col > last_col {
        return false;
    }

    let boundary_start = start_col.saturating_sub(LIGATURE_CONTEXT_COLUMNS);
    let boundary_end = end_col.saturating_add(1).min(last_col);
    for col in boundary_start..=boundary_end {
        let Some((_, span)) = operator_ligature_replacement_for_row(row_cells, col, last_col)
        else {
            continue;
        };
        let ligature_end = col.saturating_add(span.saturating_sub(1)).min(last_col);
        let crosses_left = col < start_col && ligature_end >= start_col;
        let crosses_right = col <= end_col && ligature_end > end_col;
        if crosses_left || crosses_right {
            return true;
        }
    }

    false
}
