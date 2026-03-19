use crate::error::{Error, Result};

/// Non-zero dimensions for an off-screen texture render target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureSurfaceSize {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
}

impl TextureSurfaceSize {
    /// Creates a validated texture size.
    pub fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidTextureSurfaceSize { width, height });
        }

        Ok(Self { width, height })
    }
}

/// Configuration for an off-screen texture render target.
#[derive(Clone, Copy, Debug)]
pub struct TextureSurfaceConfig {
    /// The texture dimensions.
    pub size: TextureSurfaceSize,
    /// The color format used for the render target.
    pub format: wgpu::TextureFormat,
    /// The texture usage flags enabled for the render target.
    pub usage: wgpu::TextureUsages,
}

impl TextureSurfaceConfig {
    /// Creates a default render-target configuration for the provided size.
    #[must_use]
    pub fn new(size: TextureSurfaceSize) -> Self {
        Self {
            size,
            ..Self::default()
        }
    }
}

impl Default for TextureSurfaceConfig {
    fn default() -> Self {
        Self {
            size: TextureSurfaceSize {
                width: 1,
                height: 1,
            },
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        }
    }
}

/// Off-screen texture surface used for renderer bootstrap and tests.
#[derive(Debug)]
pub struct TextureSurface {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    config: TextureSurfaceConfig,
}

impl TextureSurface {
    /// Allocates a new texture render target.
    pub(crate) fn new(device: &wgpu::Device, config: TextureSurfaceConfig) -> Result<Self> {
        let size = TextureSurfaceSize::new(config.size.width, config.size.height)?;
        if !config
            .usage
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(Error::InvalidTextureSurfaceUsage);
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iris-render-wgpu-texture-surface"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: config.usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self {
            texture,
            view,
            config: TextureSurfaceConfig { size, ..config },
        })
    }

    /// Returns the render-target size.
    #[must_use]
    pub const fn size(&self) -> TextureSurfaceSize {
        self.config.size
    }

    /// Returns the render-target format.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Returns the render-target usage flags.
    #[must_use]
    pub const fn usage(&self) -> wgpu::TextureUsages {
        self.config.usage
    }

    /// Returns the underlying texture resource.
    #[must_use]
    pub const fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub(crate) const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

#[cfg(test)]
mod tests {
    use super::{TextureSurfaceConfig, TextureSurfaceSize};
    use crate::error::Error;

    #[test]
    fn texture_surface_size_rejects_zero_width() {
        let result = TextureSurfaceSize::new(0, 24);
        assert!(matches!(
            result,
            Err(Error::InvalidTextureSurfaceSize {
                width: 0,
                height: 24
            })
        ));
    }

    #[test]
    fn texture_surface_size_rejects_zero_height() {
        let result = TextureSurfaceSize::new(80, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidTextureSurfaceSize {
                width: 80,
                height: 0
            })
        ));
    }

    #[test]
    fn texture_surface_config_defaults_to_srgb_render_target() {
        let config = TextureSurfaceConfig::default();
        assert_eq!(config.size.width, 1);
        assert_eq!(config.size.height, 1);
        assert_eq!(config.format, wgpu::TextureFormat::Bgra8UnormSrgb);
        assert!(config
            .usage
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT));
        assert!(config.usage.contains(wgpu::TextureUsages::COPY_SRC));
    }
}
