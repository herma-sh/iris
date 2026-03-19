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
    capabilities: wgpu::SurfaceCapabilities,
    config: wgpu::SurfaceConfiguration,
    size: SurfaceSize,
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
            capabilities,
            config: surface_config,
            size,
        })
    }

    pub(crate) fn resize(&mut self, device: &wgpu::Device, size: SurfaceSize) -> Result<()> {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(device, &self.config);
        self.size = size;
        Ok(())
    }

    /// Returns the configured presentation surface size.
    #[must_use]
    pub const fn size(&self) -> SurfaceSize {
        self.size
    }

    /// Returns the texture format used for presentation.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Returns the applied `wgpu` surface configuration.
    #[must_use]
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    /// Returns the cached surface capabilities for the selected adapter.
    #[must_use]
    pub fn capabilities(&self) -> &wgpu::SurfaceCapabilities {
        &self.capabilities
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
mod tests {
    use super::{
        build_surface_configuration, preferred_surface_format, SurfaceConfig, SurfaceSize,
    };
    use crate::error::Error;

    #[test]
    fn surface_size_rejects_zero_width() {
        let result = SurfaceSize::new(0, 24);
        assert!(matches!(
            result,
            Err(Error::InvalidSurfaceSize {
                width: 0,
                height: 24
            })
        ));
    }

    #[test]
    fn surface_size_rejects_zero_height() {
        let result = SurfaceSize::new(80, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidSurfaceSize {
                width: 80,
                height: 0
            })
        ));
    }

    #[test]
    fn surface_config_defaults_to_fifo_and_auto_alpha() {
        let config =
            SurfaceConfig::new(SurfaceSize::new(1280, 720).expect("surface size is valid"));
        assert_eq!(config.present_mode, wgpu::PresentMode::Fifo);
        assert_eq!(config.alpha_mode, wgpu::CompositeAlphaMode::Auto);
        assert_eq!(config.desired_maximum_frame_latency, 2);
    }

    #[test]
    fn surface_configuration_prefers_an_srgb_format() {
        let config = SurfaceConfig::new(SurfaceSize::new(800, 600).expect("surface size is valid"));
        let capabilities = wgpu::SurfaceCapabilities {
            formats: vec![
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureFormat::Bgra8UnormSrgb,
            ],
            present_modes: vec![wgpu::PresentMode::Fifo],
            alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };

        let surface_configuration = build_surface_configuration(&capabilities, config)
            .expect("surface config should build");

        assert_eq!(
            surface_configuration.format,
            wgpu::TextureFormat::Bgra8UnormSrgb
        );
    }

    #[test]
    fn surface_configuration_falls_back_to_the_first_supported_format() {
        let formats = vec![
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Bgra8Unorm,
        ];
        assert_eq!(
            preferred_surface_format(&formats),
            Some(wgpu::TextureFormat::Rgba16Float)
        );
    }

    #[test]
    fn surface_configuration_rejects_unsupported_present_modes() {
        let config = SurfaceConfig {
            present_mode: wgpu::PresentMode::Mailbox,
            ..SurfaceConfig::new(SurfaceSize::new(800, 600).expect("surface size is valid"))
        };
        let capabilities = wgpu::SurfaceCapabilities {
            formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
            present_modes: vec![wgpu::PresentMode::Fifo],
            alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };

        let result = build_surface_configuration(&capabilities, config);
        assert!(matches!(
            result,
            Err(Error::UnsupportedSurfacePresentMode {
                present_mode: wgpu::PresentMode::Mailbox
            })
        ));
    }

    #[test]
    fn surface_configuration_rejects_unsupported_alpha_modes() {
        let config = SurfaceConfig {
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            ..SurfaceConfig::new(SurfaceSize::new(800, 600).expect("surface size is valid"))
        };
        let capabilities = wgpu::SurfaceCapabilities {
            formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
            present_modes: vec![wgpu::PresentMode::Fifo],
            alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };

        let result = build_surface_configuration(&capabilities, config);
        assert!(matches!(
            result,
            Err(Error::UnsupportedSurfaceAlphaMode {
                alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied
            })
        ));
    }

    #[test]
    fn surface_configuration_rejects_unsupported_surfaces() {
        let config = SurfaceConfig::new(SurfaceSize::new(800, 600).expect("surface size is valid"));
        let capabilities = wgpu::SurfaceCapabilities::default();

        let result = build_surface_configuration(&capabilities, config);
        assert!(matches!(result, Err(Error::SurfaceUnsupportedByAdapter)));
    }
}
