//! One texture page for everything that is an image on the GPU: SVG and
//! raster markers, text markers and pattern tiles (the web's
//! `render/atlas.ts`, docs/STYLE.md §6).
//!
//! Images are drawn at the size they are shown, in power-of-two steps
//! (8 … 512 px), so they are sharp at every zoom and never shrink by more
//! than half: no mipmaps. A size still to be made is stood in for by the
//! nearest one already there. The page fills with shelves; when it is full it
//! starts over (the generation changes, every rectangle is looked up again).
//! If one frame alone needs more than the page, the largest step is halved
//! for a while.
//!
//! Unlike the browser, which decodes images in the background, the desktop
//! draws them on the CPU when they are first asked for. A frame spends at
//! most [`FRAME_BUDGET`] on new images; the rest wait for the next frame
//! (`pending`), so opening a drawing full of pictograms never stalls one frame.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use kentos_native_style::batches::AtlasImage;

use super::picture::ImageSource;
use super::raster::{self, Painted};

pub const PAGE: u32 = 2048;
const MIN_STEP: u32 = 8;
const MAX_STEP: u32 = 512;
/// Frames without an overflow before the largest step may grow again.
const RELAX_FRAMES: u64 = 240;
/// CPU time a frame may spend drawing new images.
pub const FRAME_BUDGET: Duration = Duration::from_millis(12);

/// Where an image sits in the page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// x, y, width, height in 0..1 of the page, y down.
    pub uv: [f32; 4],
    /// Height over width.
    pub aspect: f32,
}

/// Pixels waiting to be written into the texture at `x`, `y`.
pub struct Upload {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// The step for a wanted size: the next power of two, within the steps allowed now.
fn step_for(px: f64, cap: u32) -> u32 {
    let p = px.max(f64::from(MIN_STEP)).min(f64::from(cap));
    let step = 2f64.powf(p.log2().ceil());
    (step as u32).min(cap).max(MIN_STEP)
}

#[derive(Default)]
pub struct Atlas {
    entries: HashMap<String, (Hit, u32)>,
    steps: HashMap<String, Vec<u32>>,
    /// Images that could not be drawn (unreadable): not tried again.
    failed: HashSet<String>,
    shelf: (u32, u32, u32),
    /// Bumped when the page starts over.
    pub generation: u64,
    frame: u64,
    reset_frame: Option<u64>,
    cap: u32,
    last_overflow: Option<u64>,
    uploads: Vec<Upload>,
    started: Option<Instant>,
    /// An image was left for a later frame: the host draws once more.
    pub pending: bool,
}

impl Atlas {
    pub fn new() -> Atlas {
        Atlas {
            cap: MAX_STEP,
            ..Atlas::default()
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame += 1;
        self.started = Some(Instant::now());
        self.pending = false;
        if self.cap < MAX_STEP
            && self
                .last_overflow
                .is_none_or(|f| self.frame.saturating_sub(f) > RELAX_FRAMES)
        {
            self.cap *= 2;
            self.last_overflow = Some(self.frame);
        }
    }

    /// Where `image` drawn about `px` device pixels along its fitted side is;
    /// a nearby size while that one waits, or None while nothing is ready.
    pub fn lookup(&mut self, image: &AtlasImage, px: f64, source: &dyn ImageSource) -> Option<Hit> {
        let step = step_for(px, self.cap);
        let key = format!("{}@{step}", image.key());
        if let Some((hit, _)) = self.entries.get(&key) {
            return Some(*hit);
        }
        if self.failed.contains(image.key()) {
            return None;
        }
        let in_budget = self.started.is_none_or(|t| t.elapsed() < FRAME_BUDGET);
        if in_budget && let Some(hit) = self.make(image, step, &key, source) {
            return Some(hit);
        }
        if !in_budget {
            self.pending = true;
        }
        self.stand_in(image.key(), step)
    }

    /// The placed size nearest to `step` (larger first: shrinking looks better than growing).
    fn stand_in(&self, image_key: &str, step: u32) -> Option<Hit> {
        let have = self.steps.get(image_key)?;
        let mut best = *have.first()?;
        for &s in have {
            if s >= step {
                if best < step || s < best {
                    best = s;
                }
            } else if best < step && s > best {
                best = s;
            }
        }
        self.entries
            .get(&format!("{image_key}@{best}"))
            .map(|(h, _)| *h)
    }

    fn reset(&mut self) {
        self.entries.clear();
        self.steps.clear();
        self.shelf = (0, 0, 0);
        self.generation += 1;
    }

    /// A free spot for w × h pixels with their border (shelves), starting the page over once per frame when full.
    fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        let big_w = w + raster::PAD * 2;
        let big_h = h + raster::PAD * 2;
        if big_w > PAGE || big_h > PAGE {
            return None;
        }
        for _ in 0..2 {
            let (mut x, mut y, mut sh) = self.shelf;
            if x + big_w > PAGE {
                y += sh;
                x = 0;
                sh = 0;
            }
            if y + big_h <= PAGE {
                self.shelf = (x + big_w, y, sh.max(big_h));
                return Some((x, y));
            }
            if self.reset_frame == Some(self.frame) {
                break;
            }
            self.reset_frame = Some(self.frame);
            self.reset();
        }
        // This frame needs more than a page: smaller steps for a while.
        self.cap = (self.cap / 2).max(32);
        self.last_overflow = Some(self.frame);
        None
    }

    fn paint(
        &mut self,
        image: &AtlasImage,
        step: u32,
        source: &dyn ImageSource,
    ) -> Option<Painted> {
        raster::image(image, step, source)
    }

    /// Draws an image at a step and places it; None when it cannot be drawn or placed.
    fn make(
        &mut self,
        image: &AtlasImage,
        step: u32,
        key: &str,
        source: &dyn ImageSource,
    ) -> Option<Hit> {
        let Some(painted) = self.paint(image, step, source) else {
            self.failed.insert(image.key().to_owned());
            return None;
        };
        let (x, y) = self.alloc(painted.width, painted.height)?;
        let page = PAGE as f32;
        let hit = Hit {
            uv: [
                (x + raster::PAD) as f32 / page,
                (y + raster::PAD) as f32 / page,
                painted.width as f32 / page,
                painted.height as f32 / page,
            ],
            aspect: painted.height as f32 / painted.width.max(1) as f32,
        };
        self.uploads.push(Upload {
            x,
            y,
            width: painted.width + 2 * raster::PAD,
            height: painted.height + 2 * raster::PAD,
            data: painted.data,
        });
        self.entries.insert(key.to_owned(), (hit, step));
        self.steps
            .entry(image.key().to_owned())
            .or_default()
            .push(step);
        Some(hit)
    }

    /// The pixels drawn since the last call, to be written into the texture.
    pub fn take_uploads(&mut self) -> Vec<Upload> {
        std::mem::take(&mut self.uploads)
    }

    /// Forgets unreadable images (the library changed: an asset may be back).
    pub fn forget_failures(&mut self) {
        self.failed.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_powers_of_two_within_the_cap() {
        assert_eq!(step_for(3.0, 512), 8);
        assert_eq!(step_for(9.0, 512), 16);
        assert_eq!(step_for(64.0, 512), 64);
        assert_eq!(step_for(65.0, 512), 128);
        assert_eq!(step_for(5000.0, 512), 512);
        assert_eq!(step_for(300.0, 128), 128);
    }
}
