//! Immutable, deduplicated long-pattern pages. Uploaded with a changed layer,
//! never rebuilt for camera movement. Short recipes keep the inline uniform path.
use kentos_native_style::batches::{BatchKind, StyledLayer, Unit};
use std::borrow::Cow;
use std::collections::HashMap;

/// Only the immutable GPU upload copy changes: CPU/export retain the complete
/// source distance. Fold hi+lo in f64 before the vertex shader multiplies it by
/// px/m; its bounded phase then fits f32 independently of the path's length.
pub(super) fn upload_values(layer: &StyledLayer) -> (Cow<'_, [f32]>, Cow<'_, [f32]>) {
    let mut high = Cow::Borrowed(layer.data.as_slice());
    let mut low = if layer.low.len() == layer.data.len() {
        Cow::Borrowed(layer.low.as_slice())
    } else {
        Cow::Owned(
            (0..layer.data.len())
                .map(|i| layer.low.get(i).copied().unwrap_or(0.))
                .collect::<Vec<_>>(),
        )
    };
    for batch in &layer.batches {
        if !matches!(
            &batch.kind,
            BatchKind::Stroke {
                unit: Unit::World,
                dash: Some(_),
                ..
            }
        ) {
            continue;
        }
        let period: f64 = kentos_native_style::uniform::batch_dashes(batch)
            .iter()
            .sum();
        if !period.is_finite() || period <= 0. {
            continue;
        }
        let Some(values) = layer.data.get(batch.range.clone()) else {
            continue;
        };
        for (i, segment) in values.chunks_exact(6).enumerate() {
            let at = batch.range.start + i * 6 + 4;
            let distance =
                f64::from(segment[4]) + f64::from(layer.low.get(at).copied().unwrap_or(0.));
            if !distance.is_finite() || distance.abs() < period {
                continue;
            }
            let phase = distance.rem_euclid(period);
            let h = phase as f32;
            high.to_mut()[at] = h;
            low.to_mut()[at] = (phase - f64::from(h)) as f32;
        }
    }
    (high, low)
}

const PAGE_FLOATS: usize = 65_536; // 256 KiB, below native baseline binding limits.

pub(super) struct DashPages {
    pub pages: Vec<Vec<f32>>,
    recipes: HashMap<Vec<u32>, (usize, u32, u32)>,
    capacity: usize,
}

impl Default for DashPages {
    fn default() -> Self {
        Self {
            pages: vec![vec![0.]],
            recipes: HashMap::new(),
            capacity: PAGE_FLOATS,
        }
    }
}

impl DashPages {
    /// Page, offset, count in floats. Values have already been made even.
    pub fn insert(&mut self, values: &[f64]) -> (usize, u32, u32) {
        if values.len() <= 8 {
            return (0, 0, 0);
        }
        let key: Vec<u32> = values.iter().map(|x| (*x as f32).to_bits()).collect();
        if let Some(&slot) = self.recipes.get(&key) {
            return slot;
        }
        // Default supplies a nonempty dummy page; insert only appends pages.
        if self.pages[self.pages.len() - 1].len() + key.len() > self.capacity {
            self.pages.push(Vec::new());
        }
        let page = self.pages.len() - 1;
        let values = &mut self.pages[page];
        let slot = (page, values.len() as u32, key.len() as u32);
        values.extend(key.iter().map(|x| f32::from_bits(*x)));
        self.recipes.insert(key, slot);
        slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_only_world_stroke_upload_distance_without_mutating_source() {
        use kentos_native_style::batches::{Cap, FillPaintBatch, StyledBatch};
        let stroke = |unit| BatchKind::Stroke {
            color: [1.; 4],
            width: 0.1,
            unit,
            dash: Some(vec![0.5, 0.25]),
            dash_offset: 0.,
            cap: Cap::Butt,
            blur: 0.,
        };
        let batch = |kind, range| StyledBatch {
            kind,
            range,
            level: 0.,
            key: 0,
            bounds: [0.; 4],
            origin: [0.; 2],
            reach: 0.,
            reach_unit: Unit::World,
            min_scale: None,
            max_scale: None,
            group: 0,
            effect: None,
            weight_cap: 0.,
            line_type_loss: None,
        };
        let mut layer = StyledLayer {
            data: vec![1.; 18],
            low: vec![0.; 18],
            batches: vec![
                batch(stroke(Unit::World), 0..6),
                batch(stroke(Unit::Px), 6..12),
                batch(
                    BatchKind::Fill {
                        paint: FillPaintBatch::Solid { color: [1.; 4] },
                    },
                    12..18,
                ),
            ],
            ..Default::default()
        };
        layer.data[4] = 1_000_000_000.;
        layer.low[4] = 0.125;
        layer.data[10] = 1_000_000_000.;
        layer.low[10] = 0.125;
        let (before, below) = (layer.data.clone(), layer.low.clone());
        let (high, low) = upload_values(&layer);
        assert_eq!(high[4], 0.375);
        assert_eq!(low[4], 0.);
        for i in (0..18).filter(|&i| i != 4) {
            assert_eq!(high[i], before[i]);
            assert_eq!(low[i], below[i]);
        }
        assert_eq!(layer.data, before);
        assert_eq!(layer.low, below);
        drop((high, low));
        layer.data[4] = 0.;
        layer.low[4] = 0.;
        let (high, low) = upload_values(&layer);
        assert!(matches!(high, Cow::Borrowed(_)));
        assert!(matches!(low, Cow::Borrowed(_)));
    }

    #[test]
    fn recipes_share_slots_and_overflow_to_another_bounded_page() {
        let mut pages = DashPages {
            capacity: 24,
            ..Default::default()
        };
        assert_eq!(pages.insert(&[1., 2.]), (0, 0, 0));
        let a = [1.; 12];
        assert_eq!(pages.insert(&a), (0, 1, 12));
        assert_eq!(pages.insert(&a), (0, 1, 12));
        assert_eq!(pages.insert(&[2.; 12]), (1, 0, 12));
        assert_eq!(pages.insert(&[3.; 12]), (1, 12, 12));
        assert_eq!(pages.insert(&[4.; 12]), (2, 0, 12));
        assert!(pages.pages.iter().all(|p| p.len() <= 24));
    }
}
