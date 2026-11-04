//! Immutable, deduplicated long-pattern pages. Uploaded with a changed layer,
//! never rebuilt for camera movement. Short recipes keep the inline uniform path.
use std::collections::HashMap;

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
