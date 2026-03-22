use super::*;

impl TerminalRenderer {
    fn begin_scroll_shift_prelude(
        &self,
        renderer: &Renderer,
        encoder_label: &'static str,
        clear_pass_label: &'static str,
    ) -> wgpu::CommandEncoder {
        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(encoder_label),
                });
        let surface_size = self.frame_surface.size();
        let full_extent = wgpu::Extent3d {
            width: surface_size.width,
            height: surface_size.height,
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: self.frame_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: self.scroll_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            full_extent,
        );
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(clear_pass_label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.frame_surface.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.theme().background.to_wgpu_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        encoder
    }

    pub(super) fn shift_retained_frame_for_scroll(
        &mut self,
        renderer: &Renderer,
        scroll_delta: ScrollDelta,
    ) {
        let Some((source_y, destination_y, copy_height)) = scroll_copy_region(
            self.requested_uniforms,
            self.frame_surface.size(),
            scroll_delta,
        ) else {
            return;
        };

        let mut encoder = self.begin_scroll_shift_prelude(
            renderer,
            "iris-render-wgpu-terminal-renderer-scroll-shift",
            "iris-render-wgpu-terminal-renderer-scroll-clear-pass",
        );
        let surface_size = self.frame_surface.size();
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: self.scroll_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: source_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: self.frame_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: destination_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: surface_size.width,
                height: copy_height,
                depth_or_array_layers: 1,
            },
        );
        renderer.queue().submit(std::iter::once(encoder.finish()));
    }

    pub(super) fn shift_retained_frame_for_partial_scroll(
        &mut self,
        renderer: &Renderer,
        scroll_delta: ScrollDelta,
    ) {
        let Some((region_top_y, region_bottom_y, source_y, destination_y, copy_height)) =
            partial_scroll_copy_region(
                self.requested_uniforms,
                self.frame_surface.size(),
                scroll_delta,
            )
        else {
            return;
        };

        let mut encoder = self.begin_scroll_shift_prelude(
            renderer,
            "iris-render-wgpu-terminal-renderer-partial-scroll-shift",
            "iris-render-wgpu-terminal-renderer-partial-scroll-clear-pass",
        );
        let surface_size = self.frame_surface.size();
        if region_top_y > 0 {
            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: self.scroll_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: self.frame_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: surface_size.width,
                    height: region_top_y,
                    depth_or_array_layers: 1,
                },
            );
        }
        let below_region_height = surface_size.height.saturating_sub(region_bottom_y);
        if below_region_height > 0 {
            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: self.scroll_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: region_bottom_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: self.frame_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: region_bottom_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: surface_size.width,
                    height: below_region_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        if copy_height > 0 {
            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: self.scroll_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: source_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: self.frame_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: destination_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: surface_size.width,
                    height: copy_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        renderer.queue().submit(std::iter::once(encoder.finish()));
    }
}

pub(super) fn create_frame_surface(
    renderer: &Renderer,
    format: wgpu::TextureFormat,
    uniforms: TextUniforms,
) -> TextureSurface {
    renderer
        .create_texture_surface(TextureSurfaceConfig {
            size: frame_surface_size_for_uniforms(uniforms),
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING,
        })
        .expect("internal frame-surface config should remain valid")
}

pub(super) fn create_scroll_surface(
    renderer: &Renderer,
    format: wgpu::TextureFormat,
    uniforms: TextUniforms,
) -> TextureSurface {
    renderer
        .create_texture_surface(TextureSurfaceConfig {
            size: frame_surface_size_for_uniforms(uniforms),
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
        })
        .expect("internal scroll-surface config should remain valid")
}

pub(super) fn frame_surface_size_for_uniforms(uniforms: TextUniforms) -> TextureSurfaceSize {
    let viewport_size = viewport_surface_size_for_uniforms(uniforms);
    let vertical_padding = frame_vertical_padding_pixels(uniforms);
    TextureSurfaceSize {
        width: viewport_size.width,
        height: viewport_size
            .height
            .saturating_add(vertical_padding.saturating_mul(2)),
    }
}

pub(super) fn frame_uniforms_for_requested(uniforms: TextUniforms) -> TextUniforms {
    let frame_size = frame_surface_size_for_uniforms(uniforms);
    let vertical_padding = frame_vertical_padding_pixels(uniforms) as f32;
    TextUniforms::new(
        [frame_size.width as f32, frame_size.height as f32],
        uniforms.cell_size,
        vertical_padding,
    )
}

pub(super) fn present_uniforms_for_requested(
    uniforms: TextUniforms,
    theme: &Theme,
    frame_surface_size: TextureSurfaceSize,
) -> PresentUniforms {
    PresentUniforms::new(
        [
            frame_surface_size.width as f32,
            frame_surface_size.height as f32,
        ],
        [0.0, frame_vertical_padding_pixels(uniforms) as f32],
        uniforms.scroll_offset,
        theme.background.to_f32_array(),
    )
}

fn viewport_surface_size_for_uniforms(uniforms: TextUniforms) -> TextureSurfaceSize {
    TextureSurfaceSize {
        width: normalized_surface_dimension(uniforms.resolution[0]),
        height: normalized_surface_dimension(uniforms.resolution[1]),
    }
}

fn frame_vertical_padding_pixels(uniforms: TextUniforms) -> u32 {
    viewport_surface_size_for_uniforms(uniforms).height
}

pub(super) fn normalized_scroll_delta(
    scroll_delta: Option<ScrollDelta>,
    grid: &Grid,
) -> Option<ScrollDelta> {
    let scroll_delta = scroll_delta?;
    if grid.rows() == 0 || scroll_delta.lines == 0 {
        return None;
    }
    if scroll_delta.top > scroll_delta.bottom || scroll_delta.bottom >= grid.rows() {
        return None;
    }

    Some(scroll_delta)
}

pub(super) fn damage_overlaps_region(damage: &[DamageRegion], region: DamageRegion) -> bool {
    damage.iter().any(|candidate| {
        candidate.start_row <= region.end_row
            && candidate.end_row >= region.start_row
            && candidate.start_col <= region.end_col
            && candidate.end_col >= region.start_col
    })
}

pub(super) fn is_full_grid_scroll_delta(scroll_delta: ScrollDelta, grid: &Grid) -> bool {
    let Some(full_grid_bottom) = grid.rows().checked_sub(1) else {
        return false;
    };
    scroll_delta.top == 0 && scroll_delta.bottom == full_grid_bottom
}

pub(super) fn partial_scroll_copy_region(
    uniforms: TextUniforms,
    frame_surface_size: TextureSurfaceSize,
    scroll_delta: ScrollDelta,
) -> Option<(u32, u32, u32, u32, u32)> {
    let viewport_size = viewport_surface_size_for_uniforms(uniforms);
    if viewport_size.height == 0 || frame_surface_size.width == 0 {
        return None;
    }

    let cell_height = validated_cell_height_pixels(uniforms.cell_size[1])?;

    let top_row = u32::try_from(scroll_delta.top).unwrap_or(u32::MAX);
    let bottom_row_exclusive =
        u32::try_from(scroll_delta.bottom.saturating_add(1)).unwrap_or(u32::MAX);
    let region_top_in_viewport = top_row
        .saturating_mul(cell_height)
        .min(viewport_size.height);
    let region_bottom_in_viewport = bottom_row_exclusive
        .saturating_mul(cell_height)
        .min(viewport_size.height);
    if region_top_in_viewport >= region_bottom_in_viewport {
        return None;
    }

    let vertical_padding = frame_vertical_padding_pixels(uniforms);
    let region_top_y = vertical_padding.saturating_add(region_top_in_viewport);
    let region_bottom_y = vertical_padding.saturating_add(region_bottom_in_viewport);
    let region_height = region_bottom_y.saturating_sub(region_top_y);
    let shift_pixels = scroll_delta
        .lines
        .unsigned_abs()
        .saturating_mul(cell_height);
    if shift_pixels == 0 || shift_pixels > region_height {
        return None;
    }

    let copy_height = region_height.saturating_sub(shift_pixels);
    let (source_y, destination_y) = match scroll_delta.lines {
        lines if lines > 0 => (region_top_y.saturating_add(shift_pixels), region_top_y),
        lines if lines < 0 => (region_top_y, region_top_y.saturating_add(shift_pixels)),
        _ => return None,
    };

    if region_bottom_y > frame_surface_size.height {
        return None;
    }

    Some((
        region_top_y,
        region_bottom_y,
        source_y,
        destination_y,
        copy_height,
    ))
}

pub(super) fn scroll_copy_region(
    uniforms: TextUniforms,
    frame_surface_size: TextureSurfaceSize,
    scroll_delta: ScrollDelta,
) -> Option<(u32, u32, u32)> {
    let viewport_size = viewport_surface_size_for_uniforms(uniforms);
    if viewport_size.height == 0 || frame_surface_size.width == 0 {
        return None;
    }

    let vertical_padding = frame_vertical_padding_pixels(uniforms);
    let cell_height = validated_cell_height_pixels(uniforms.cell_size[1])?;
    let shift_pixels = scroll_delta
        .lines
        .unsigned_abs()
        .saturating_mul(cell_height);
    if shift_pixels == 0 || shift_pixels > viewport_size.height {
        return None;
    }

    let (source_y, destination_y, copy_height) = match scroll_delta.lines {
        lines if lines > 0 => (
            vertical_padding,
            vertical_padding.saturating_sub(shift_pixels),
            viewport_size.height,
        ),
        lines if lines < 0 => (
            vertical_padding,
            vertical_padding.saturating_add(shift_pixels),
            viewport_size.height,
        ),
        _ => return None,
    };

    if !copy_region_fits_in_surface_height(frame_surface_size.height, source_y, copy_height)
        || !copy_region_fits_in_surface_height(
            frame_surface_size.height,
            destination_y,
            copy_height,
        )
    {
        return None;
    }

    Some((source_y, destination_y, copy_height))
}

fn copy_region_fits_in_surface_height(
    surface_height: u32,
    origin_y: u32,
    copy_height: u32,
) -> bool {
    match origin_y.checked_add(copy_height) {
        Some(copy_end) => copy_end <= surface_height,
        None => false,
    }
}

fn validated_cell_height_pixels(raw_cell_height: f32) -> Option<u32> {
    if !raw_cell_height.is_finite() || raw_cell_height <= 0.0 {
        tracing::warn!(
            ?raw_cell_height,
            "invalid cell height for retained scroll copy planning"
        );
        return None;
    }

    Some(normalized_surface_dimension(raw_cell_height))
}

fn normalized_surface_dimension(dimension: f32) -> u32 {
    if !dimension.is_finite() || dimension <= 0.0 {
        tracing::warn!(
            ?dimension,
            "invalid terminal frame dimension normalized to a 1px fallback"
        );
        1
    } else {
        dimension.round().max(1.0) as u32
    }
}
