use crate::error::{Error, Result};

/// Non-zero dimensions for a presentation surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceSize {
    /// Surface width in pixels.
    pub width: u32,
    /// Surface height in pixels.
    pub height: u32,
}

impl SurfaceSize {
    /// Creates a validated presentation surface size.
    pub fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidSurfaceSize { width, height });
        }

        Ok(Self { width, height })
    }
}

/// Configuration for a presentation surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceConfig {
    /// The surface dimensions.
    pub size: SurfaceSize,
    /// Preferred presentation mode.
    pub present_mode: wgpu::PresentMode,
    /// Preferred alpha-composition mode.
    pub alpha_mode: wgpu::CompositeAlphaMode,
    /// Desired frame-latency hint for the presentation engine.
    pub desired_maximum_frame_latency: u32,
}

impl SurfaceConfig {
    /// Creates a default surface configuration for the provided size.
    #[must_use]
    pub fn new(size: SurfaceSize) -> Self {
        Self {
            size,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: 2,
        }
    }
}

/// A configured presentation surface ready for frame acquisition.
#[derive(Debug)]
pub struct RendererSurface<'window> {
    surface: wgpu::Surface<'window>,
    state: SurfaceState,
}

#[derive(Debug)]
struct SurfaceState {
    capabilities: wgpu::SurfaceCapabilities,
    config: wgpu::SurfaceConfiguration,
    size: SurfaceSize,
}

impl SurfaceState {
    fn new(
        capabilities: wgpu::SurfaceCapabilities,
        config: wgpu::SurfaceConfiguration,
        size: SurfaceSize,
    ) -> Self {
        Self {
            capabilities,
            config,
            size,
        }
    }

    fn resize(&mut self, size: SurfaceSize) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.size = size;
    }
}

impl<'window> RendererSurface<'window> {
    pub(crate) fn new(
        surface: wgpu::Surface<'window>,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        config: SurfaceConfig,
    ) -> Result<Self> {
        let size = config.size;
        let capabilities = surface.get_capabilities(adapter);
        let surface_config = build_surface_configuration(&capabilities, config)?;
        surface.configure(device, &surface_config);

        Ok(Self {
            surface,
            state: SurfaceState::new(capabilities, surface_config, size),
        })
    }

    pub(crate) fn resize(&mut self, device: &wgpu::Device, size: SurfaceSize) -> Result<()> {
        self.state.resize(size);
        self.surface.configure(device, &self.state.config);
        Ok(())
    }

    /// Returns the configured presentation surface size.
    #[must_use]
    pub const fn size(&self) -> SurfaceSize {
        self.state.size
    }

    /// Returns the texture format used for presentation.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.state.config.format
    }

    /// Returns the applied `wgpu` surface configuration.
    #[must_use]
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.state.config
    }

    /// Returns the cached surface capabilities for the selected adapter.
    #[must_use]
    pub fn capabilities(&self) -> &wgpu::SurfaceCapabilities {
        &self.state.capabilities
    }

    /// Acquires the next presentation texture from the swapchain.
    pub fn current_texture(&self) -> std::result::Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }
}

fn build_surface_configuration(
    capabilities: &wgpu::SurfaceCapabilities,
    config: SurfaceConfig,
) -> Result<wgpu::SurfaceConfiguration> {
    let format = preferred_surface_format(&capabilities.formats)
        .ok_or(Error::SurfaceUnsupportedByAdapter)?;

    if !supports_present_mode(capabilities, config.present_mode) {
        return Err(Error::UnsupportedSurfacePresentMode {
            present_mode: config.present_mode,
        });
    }

    if !supports_alpha_mode(capabilities, config.alpha_mode) {
        return Err(Error::UnsupportedSurfaceAlphaMode {
            alpha_mode: config.alpha_mode,
        });
    }

    Ok(wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: config.size.width,
        height: config.size.height,
        present_mode: config.present_mode,
        desired_maximum_frame_latency: config.desired_maximum_frame_latency,
        alpha_mode: config.alpha_mode,
        view_formats: vec![],
    })
}

fn preferred_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| formats.first().copied())
}

fn supports_present_mode(
    capabilities: &wgpu::SurfaceCapabilities,
    present_mode: wgpu::PresentMode,
) -> bool {
    matches!(
        present_mode,
        wgpu::PresentMode::AutoVsync | wgpu::PresentMode::AutoNoVsync
    ) || capabilities.present_modes.contains(&present_mode)
}

fn supports_alpha_mode(
    capabilities: &wgpu::SurfaceCapabilities,
    alpha_mode: wgpu::CompositeAlphaMode,
) -> bool {
    alpha_mode == wgpu::CompositeAlphaMode::Auto || capabilities.alpha_modes.contains(&alpha_mode)
}

#[cfg(test)]
mod tests;
