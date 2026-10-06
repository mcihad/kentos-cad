//! The drawing's pictures on the GPU (docs/adr/0192 §3): each picture's own
//! texture, premultiplied, with its mip levels made on the CPU (a box filter,
//! level by level) so that a photograph seen small stays smooth, at most
//! [`MAX_SIDE`] pixels a side (a larger one is made smaller first). A picture
//! the host cannot give yet draws light grey. Each view binds a picture with
//! its own frame uniform in group 0, where the atlas otherwise is.

use std::collections::HashMap;
use std::sync::Arc;

use super::picture::{Bitmap, ImageSource};

/// The longest side a picture is drawn with, in pixels.
pub const MAX_SIDE: u32 = 4096;

/// Light grey, a picture's stand-in (premultiplied, opaque).
const GREY: [u8; 4] = [200, 200, 200, 255];

/// Straight-alpha RGBA made premultiplied.
fn premultiplied(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| {
            let a = u32::from(p[3]);
            let m = |c: u8| ((u32::from(c) * a + 127) / 255) as u8;
            [m(p[0]), m(p[1]), m(p[2]), p[3]]
        })
        .collect()
}

/// A level half as large (rounded down, as the GPU sizes a texture's levels:
/// at most ⌊log₂ side⌋ + 1 of them), each pixel the mean of the up to four
/// it covers.
fn halved(rgba: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in 0..nh {
        for x in 0..nw {
            let mut sum = [0u32; 4];
            let mut n = 0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = (2 * x + dx, 2 * y + dy);
                if sx < w && sy < h {
                    let i = ((sy * w + sx) * 4) as usize;
                    for c in 0..4 {
                        sum[c] += u32::from(rgba[i + c]);
                    }
                    n += 1;
                }
            }
            for s in sum {
                out.push(((s + n / 2) / n) as u8);
            }
        }
    }
    (out, nw, nh)
}

/// Its levels, the largest first: the picture made at most `limit` a side, then halved to 1 × 1.
fn levels(bitmap: &Bitmap, limit: u32) -> Vec<(Vec<u8>, u32, u32)> {
    let (mut data, mut w, mut h) = (premultiplied(&bitmap.rgba), bitmap.width, bitmap.height);
    while w > limit || h > limit {
        (data, w, h) = halved(&data, w, h);
    }
    let mut out = vec![(data, w, h)];
    while let Some((d, w, h)) = out.last() {
        if (*w, *h) == (1, 1) {
            break;
        }
        let next = halved(d, *w, *h);
        out.push(next);
    }
    out
}

/// One picture's texture.
fn texture(device: &wgpu::Device, queue: &wgpu::Queue, bitmap: &Bitmap) -> wgpu::TextureView {
    let limit = MAX_SIDE.min(device.limits().max_texture_dimension_2d);
    let levels = levels(bitmap, limit);
    let (_, w, h) = &levels[0];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kentos.styled.picture"),
        size: wgpu::Extent3d {
            width: *w,
            height: *h,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (data, w, h)) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(*h),
            },
            wgpu::Extent3d {
                width: *w,
                height: *h,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// The device's pictures: a texture per key, kept while a view draws it.
pub struct PictureTextures {
    views: HashMap<String, (Arc<wgpu::TextureView>, bool)>,
    grey: Option<Arc<wgpu::TextureView>>,
    pub(crate) sampler: wgpu::Sampler,
}

impl std::fmt::Debug for PictureTextures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PictureTextures")
            .field("pictures", &self.views.len())
            .finish_non_exhaustive()
    }
}

impl PictureTextures {
    pub fn new(device: &wgpu::Device) -> PictureTextures {
        PictureTextures {
            views: HashMap::new(),
            grey: None,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("kentos.styled.picture"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..wgpu::SamplerDescriptor::default()
            }),
        }
    }

    /// The texture of the picture `key`: made on first use from the host's
    /// pixels; light grey while the host has none (looked for again next frame).
    pub fn view(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: &str,
        images: &dyn ImageSource,
    ) -> Arc<wgpu::TextureView> {
        if let Some((view, real)) = self.views.get(key)
            && *real
        {
            return view.clone();
        }
        match images.bitmap(key) {
            Some(bitmap) if bitmap.width > 0 && bitmap.height > 0 => {
                let view = Arc::new(texture(device, queue, &bitmap));
                self.views.insert(key.to_owned(), (view.clone(), true));
                view
            }
            _ => {
                let grey = self
                    .grey
                    .get_or_insert_with(|| {
                        Arc::new(texture(
                            device,
                            queue,
                            &Bitmap {
                                width: 1,
                                height: 1,
                                rgba: GREY.to_vec(),
                            },
                        ))
                    })
                    .clone();
                self.views.insert(key.to_owned(), (grey.clone(), false));
                grey
            }
        }
    }

    /// Lets go of the pictures no view drew in the frame just prepared.
    pub fn keep_only(&mut self, keys: &std::collections::HashSet<String>) {
        self.views.retain(|k, _| keys.contains(k));
    }
}

/// A picture bound with a view's frame uniform, in group 0 (docs/adr/0192 §3).
pub fn binding(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    frame: &wgpu::Buffer,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("kentos.styled.picture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_has_every_level_down_to_one_pixel() {
        let bitmap = Bitmap {
            width: 5,
            height: 3,
            rgba: vec![255; 5 * 3 * 4],
        };
        let sizes: Vec<(u32, u32)> = levels(&bitmap, 4096)
            .iter()
            .map(|(_, w, h)| (*w, *h))
            .collect();
        assert_eq!(sizes, [(5, 3), (2, 1), (1, 1)]);
        // As the GPU sizes a texture's levels (each side ⌊side / 2ⁱ⌋, at least 1; ⌊log₂ side⌋ + 1 of
        // them): 480 × 360 has 9, not the 10 halving upwards would make.
        let site = Bitmap {
            width: 480,
            height: 360,
            rgba: vec![0; 480 * 360 * 4],
        };
        let made = levels(&site, 4096);
        assert_eq!(made.len(), 9);
        for (i, (_, w, h)) in made.iter().enumerate() {
            assert_eq!(
                (*w, *h),
                ((480 >> i).max(1), (360 >> i).max(1)),
                "level {i}"
            );
        }
        // Larger than the limit: halved first.
        let big = Bitmap {
            width: 10,
            height: 4,
            rgba: vec![0; 10 * 4 * 4],
        };
        assert_eq!(levels(&big, 4)[0].1, 2);
    }

    #[test]
    fn colours_are_premultiplied_and_means_kept() {
        assert_eq!(premultiplied(&[200, 100, 50, 128]), [100, 50, 25, 128]);
        let (data, w, h) = halved(&[10, 10, 10, 255, 30, 30, 30, 255], 2, 1);
        assert_eq!((w, h), (1, 1));
        assert_eq!(&data[..4], &[20, 20, 20, 255]);
    }
}
