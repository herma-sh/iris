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
    assert!(config.usage.contains(wgpu::TextureUsages::TEXTURE_BINDING));
}
