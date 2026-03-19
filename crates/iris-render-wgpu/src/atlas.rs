use crate::error::{Error, Result};

/// Non-zero dimensions for a glyph atlas texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasSize {
    /// Atlas width in pixels.
    pub width: u32,
    /// Atlas height in pixels.
    pub height: u32,
}

impl AtlasSize {
    /// Creates a validated atlas size.
    pub fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidAtlasSize { width, height });
        }

        Ok(Self { width, height })
    }
}

/// Configuration for the glyph atlas texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasConfig {
    /// Atlas texture size.
    pub size: AtlasSize,
    /// Atlas texture format.
    pub format: wgpu::TextureFormat,
    /// Atlas texture usage flags.
    pub usage: wgpu::TextureUsages,
}

impl AtlasConfig {
    /// Creates the default atlas configuration for the provided size.
    #[must_use]
    pub fn new(size: AtlasSize) -> Self {
        Self {
            size,
            ..Self::default()
        }
    }
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            size: AtlasSize {
                width: 1,
                height: 1,
            },
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        }
    }
}

/// A rectangular allocation within the atlas texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasRegion {
    /// Region x origin in pixels.
    pub x: u32,
    /// Region y origin in pixels.
    pub y: u32,
    /// Region width in pixels.
    pub width: u32,
    /// Region height in pixels.
    pub height: u32,
}

/// Row-packed texture atlas for glyph masks.
#[derive(Debug)]
pub struct GlyphAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    allocator: AtlasAllocator,
    config: AtlasConfig,
}

impl GlyphAtlas {
    /// Creates a glyph atlas texture and allocator.
    pub fn new(device: &wgpu::Device, config: AtlasConfig) -> Result<Self> {
        let size = AtlasSize::new(config.size.width, config.size.height)?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iris-render-wgpu-glyph-atlas"),
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("iris-render-wgpu-glyph-atlas-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("iris-render-wgpu-glyph-atlas-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("iris-render-wgpu-glyph-atlas-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Self {
            texture,
            view,
            sampler,
            bind_group_layout,
            bind_group,
            allocator: AtlasAllocator::new(size),
            config: AtlasConfig { size, ..config },
        })
    }

    /// Allocates a region in the atlas using row packing.
    pub fn allocate(&mut self, width: u32, height: u32) -> Result<AtlasRegion> {
        self.allocator.allocate(width, height)
    }

    /// Uploads a single-channel glyph mask into an allocated region.
    pub fn upload(&self, queue: &wgpu::Queue, region: AtlasRegion, data: &[u8]) -> Result<()> {
        let expected = region
            .width
            .checked_mul(region.height)
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or(Error::InvalidAtlasUploadSize {
                expected: usize::MAX,
                actual: data.len(),
            })?;
        if data.len() != expected {
            return Err(Error::InvalidAtlasUploadSize {
                expected,
                actual: data.len(),
            });
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(region.width),
                rows_per_image: Some(region.height),
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );

        Ok(())
    }

    /// Returns the atlas texture size.
    #[must_use]
    pub const fn size(&self) -> AtlasSize {
        self.config.size
    }

    /// Returns the atlas texture format.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Returns the atlas texture view.
    #[must_use]
    pub const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns the atlas sampler.
    #[must_use]
    pub const fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Returns the atlas bind-group layout.
    #[must_use]
    pub const fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns the atlas bind-group.
    #[must_use]
    pub const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
struct AtlasAllocator {
    size: AtlasSize,
    next_x: u32,
    next_y: u32,
    row_height: u32,
}

impl AtlasAllocator {
    fn new(size: AtlasSize) -> Self {
        Self {
            size,
            next_x: 0,
            next_y: 0,
            row_height: 0,
        }
    }

    fn allocate(&mut self, width: u32, height: u32) -> Result<AtlasRegion> {
        if width == 0 || height == 0 || width > self.size.width || height > self.size.height {
            return Err(Error::InvalidAtlasAllocation { width, height });
        }

        if self
            .next_x
            .checked_add(width)
            .is_none_or(|next_x| next_x > self.size.width)
        {
            self.next_y = self
                .next_y
                .checked_add(self.row_height)
                .ok_or(Error::AtlasFull { width, height })?;
            self.next_x = 0;
            self.row_height = 0;
        }

        if self
            .next_y
            .checked_add(height)
            .is_none_or(|next_y| next_y > self.size.height)
        {
            return Err(Error::AtlasFull { width, height });
        }

        let region = AtlasRegion {
            x: self.next_x,
            y: self.next_y,
            width,
            height,
        };
        self.next_x += width;
        self.row_height = self.row_height.max(height);
        Ok(region)
    }
}

#[cfg(test)]
mod tests {
    use super::{AtlasAllocator, AtlasConfig, AtlasSize, GlyphAtlas};
    use crate::error::Error;

    #[test]
    fn atlas_size_rejects_zero_width() {
        let result = AtlasSize::new(0, 32);
        assert!(matches!(
            result,
            Err(Error::InvalidAtlasSize {
                width: 0,
                height: 32
            })
        ));
    }

    #[test]
    fn atlas_size_rejects_zero_height() {
        let result = AtlasSize::new(32, 0);
        assert!(matches!(
            result,
            Err(Error::InvalidAtlasSize {
                width: 32,
                height: 0
            })
        ));
    }

    #[test]
    fn atlas_allocator_places_regions_in_rows() {
        let mut allocator = AtlasAllocator::new(AtlasSize::new(8, 8).expect("atlas size is valid"));

        let first = allocator.allocate(3, 2).expect("first region should fit");
        let second = allocator.allocate(2, 2).expect("second region should fit");

        assert_eq!((first.x, first.y), (0, 0));
        assert_eq!((second.x, second.y), (3, 0));
    }

    #[test]
    fn atlas_allocator_wraps_to_the_next_row() {
        let mut allocator = AtlasAllocator::new(AtlasSize::new(8, 8).expect("atlas size is valid"));

        allocator.allocate(6, 2).expect("first region should fit");
        let wrapped = allocator.allocate(4, 3).expect("wrapped region should fit");

        assert_eq!((wrapped.x, wrapped.y), (0, 2));
    }

    #[test]
    fn atlas_allocator_advances_by_the_tallest_item_in_a_row() {
        let mut allocator =
            AtlasAllocator::new(AtlasSize::new(10, 10).expect("atlas size is valid"));

        allocator.allocate(3, 5).expect("first region should fit");
        allocator.allocate(3, 2).expect("second region should fit");
        allocator.allocate(3, 3).expect("third region should fit");
        let wrapped = allocator.allocate(3, 1).expect("wrapped region should fit");

        assert_eq!((wrapped.x, wrapped.y), (0, 5));
    }

    #[test]
    fn atlas_allocator_rejects_oversized_regions() {
        let mut allocator = AtlasAllocator::new(AtlasSize::new(8, 8).expect("atlas size is valid"));
        let result = allocator.allocate(9, 1);
        assert!(matches!(
            result,
            Err(Error::InvalidAtlasAllocation {
                width: 9,
                height: 1
            })
        ));
    }

    #[test]
    fn atlas_allocator_rejects_zero_dimensions() {
        let mut allocator = AtlasAllocator::new(AtlasSize::new(8, 8).expect("atlas size is valid"));

        let zero_width = allocator.allocate(0, 1);
        assert!(matches!(
            zero_width,
            Err(Error::InvalidAtlasAllocation {
                width: 0,
                height: 1
            })
        ));

        let zero_height = allocator.allocate(1, 0);
        assert!(matches!(
            zero_height,
            Err(Error::InvalidAtlasAllocation {
                width: 1,
                height: 0
            })
        ));
    }

    #[test]
    fn atlas_allocator_fills_exactly_before_reporting_full() {
        let mut allocator = AtlasAllocator::new(AtlasSize::new(4, 4).expect("atlas size is valid"));

        for _ in 0..4 {
            allocator.allocate(4, 1).expect("row should fit exactly");
        }

        let result = allocator.allocate(1, 1);
        assert!(matches!(
            result,
            Err(Error::AtlasFull {
                width: 1,
                height: 1
            })
        ));
    }

    #[test]
    fn atlas_allocator_reports_full_atlas() {
        let mut allocator = AtlasAllocator::new(AtlasSize::new(4, 4).expect("atlas size is valid"));

        allocator.allocate(4, 2).expect("first row should fit");
        allocator.allocate(4, 2).expect("second row should fit");
        let result = allocator.allocate(1, 1);

        assert!(matches!(
            result,
            Err(Error::AtlasFull {
                width: 1,
                height: 1
            })
        ));
    }

    #[test]
    fn glyph_atlas_rejects_invalid_upload_size() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })) {
                Some(adapter) => adapter,
                None => return,
            };
        let (device, queue) = match pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("iris-render-wgpu-atlas-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        )) {
            Ok(result) => result,
            Err(_) => return,
        };
        let mut atlas = GlyphAtlas::new(
            &device,
            AtlasConfig::new(AtlasSize::new(16, 16).expect("atlas size is valid")),
        )
        .expect("atlas should be created");
        let region = atlas.allocate(4, 4).expect("region should fit");

        let result = atlas.upload(&queue, region, &[0; 15]);
        assert!(matches!(
            result,
            Err(Error::InvalidAtlasUploadSize {
                expected: 16,
                actual: 15
            })
        ));
    }
}
