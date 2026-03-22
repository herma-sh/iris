use super::{
    build_surface_configuration, preferred_surface_format, SurfaceConfig, SurfaceSize, SurfaceState,
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
    let config = SurfaceConfig::new(SurfaceSize::new(1280, 720).expect("surface size is valid"));
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

    let surface_configuration =
        build_surface_configuration(&capabilities, config).expect("surface config should build");

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

#[test]
fn surface_state_resize_updates_size_and_configuration() {
    let initial_size = SurfaceSize::new(800, 600).expect("surface size is valid");
    let resized_size = SurfaceSize::new(1024, 768).expect("surface size is valid");
    let capabilities = wgpu::SurfaceCapabilities {
        formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
        present_modes: vec![wgpu::PresentMode::Fifo],
        alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
        usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
    };
    let expected_formats = capabilities.formats.clone();
    let expected_present_modes = capabilities.present_modes.clone();
    let expected_alpha_modes = capabilities.alpha_modes.clone();
    let initial_config =
        build_surface_configuration(&capabilities, SurfaceConfig::new(initial_size))
            .expect("surface config should build");
    let mut state = SurfaceState::new(capabilities, initial_config, initial_size);

    state.resize(resized_size);

    assert_eq!(state.size, resized_size);
    assert_eq!(state.config.width, resized_size.width);
    assert_eq!(state.config.height, resized_size.height);
    assert_eq!(state.config.format, wgpu::TextureFormat::Bgra8UnormSrgb);
    assert_eq!(state.capabilities.formats, expected_formats);
    assert_eq!(state.capabilities.present_modes, expected_present_modes);
    assert_eq!(state.capabilities.alpha_modes, expected_alpha_modes);
}
