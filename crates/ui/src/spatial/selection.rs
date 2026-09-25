//! Çoklu seçim.
//!
//! Seçim, öğe adreslerinin ([`FeatureRef`]) sıralı bir kümesidir. Bir öğe
//! "birincil"dir: nesne inceleyicisi onu gösterir ve seçimde ileri-geri
//! gezinirken o değişir.

use std::collections::BTreeSet;
use std::fmt;

use super::FeatureRef;

/// Yeni bulunan öğelerin mevcut seçimle nasıl birleşeceği (ArcGIS'teki
/// seçim yöntemleri).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// Seçimi bulunan öğelerle değiştirir.
    #[default]
    New,
    /// Bulunan öğeleri seçime ekler.
    Add,
    /// Bulunan öğeleri seçimden çıkarır.
    Remove,
    /// Seçimde yalnızca bulunan öğeleri bırakır.
    Intersect,
}

impl SelectionMode {
    pub const ALL: [SelectionMode; 4] = [
        SelectionMode::New,
        SelectionMode::Add,
        SelectionMode::Remove,
        SelectionMode::Intersect,
    ];
}

impl fmt::Display for SelectionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SelectionMode::New => "Yeni seçim",
            SelectionMode::Add => "Seçime ekle",
            SelectionMode::Remove => "Seçimden çıkar",
            SelectionMode::Intersect => "Seçim içinde",
        })
    }
}

/// Seçili öğeler ve birincil öğe.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    items: BTreeSet<FeatureRef>,
    primary: Option<FeatureRef>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn contains(&self, reference: &FeatureRef) -> bool {
        self.items.contains(reference)
    }

    /// Katman ve numara sırasıyla seçili öğeler.
    pub fn iter(&self) -> impl Iterator<Item = FeatureRef> + '_ {
        self.items.iter().copied()
    }

    /// Katmandaki seçili öğe sayısı.
    pub fn count_in(&self, layer: usize) -> usize {
        self.items.iter().filter(|item| item.layer == layer).count()
    }

    /// Nesne inceleyicisinin gösterdiği öğe.
    pub fn primary(&self) -> Option<FeatureRef> {
        self.primary
    }

    /// Seçili bir öğeyi birincil yapar.
    pub fn focus(&mut self, reference: FeatureRef) {
        if self.items.contains(&reference) {
            self.primary = Some(reference);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.primary = None;
    }

    /// Seçimi tek öğe yapar.
    pub fn select(&mut self, reference: FeatureRef) {
        self.items.clear();
        self.items.insert(reference);
        self.primary = Some(reference);
    }

    /// Öğe seçiliyse çıkarır, değilse ekler ve birincil yapar.
    pub fn toggle(&mut self, reference: FeatureRef) {
        if self.items.remove(&reference) {
            self.repair_primary();
        } else {
            self.items.insert(reference);
            self.primary = Some(reference);
        }
    }

    /// Bulunan öğeleri seçim yöntemine göre uygular. Birincil öğe mümkünse
    /// korunur; değilse bulunan ilk öğe ya da seçimin ilk öğesi olur.
    pub fn apply(&mut self, mode: SelectionMode, found: impl IntoIterator<Item = FeatureRef>) {
        let found: BTreeSet<FeatureRef> = found.into_iter().collect();

        match mode {
            SelectionMode::New => {
                self.primary = found.iter().next().copied();
                self.items = found;
            }
            SelectionMode::Add => {
                if self.primary.is_none() {
                    self.primary = found.iter().next().copied();
                }

                self.items.extend(found);
            }
            SelectionMode::Remove => {
                self.items.retain(|item| !found.contains(item));
            }
            SelectionMode::Intersect => {
                self.items.retain(|item| found.contains(item));
            }
        }

        self.repair_primary();
    }

    /// Koşulu sağlamayan öğeleri seçimden çıkarır (ör. silinen öğeler).
    pub fn retain(&mut self, keep: impl FnMut(&FeatureRef) -> bool) {
        self.items.retain(keep);
        self.repair_primary();
    }

    /// Birincil öğenin seçimdeki sırası ve seçim büyüklüğü: (1'den başlayan
    /// sıra, toplam).
    pub fn position(&self) -> Option<(usize, usize)> {
        let primary = self.primary?;

        self.items
            .iter()
            .position(|item| *item == primary)
            .map(|index| (index + 1, self.items.len()))
    }

    /// Birincil öğeyi seçimde bir sonraki (ya da önceki) öğeye taşır; uçlarda
    /// başa döner.
    pub fn step(&mut self, forward: bool) {
        let Some((position, total)) = self.position() else {
            return;
        };

        let index = if forward {
            position % total
        } else {
            (position + total - 2) % total
        };

        self.primary = self.items.iter().nth(index).copied();
    }

    fn repair_primary(&mut self) {
        if self
            .primary
            .is_none_or(|primary| !self.items.contains(&primary))
        {
            self.primary = self.items.iter().next().copied();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::ObjectId;

    fn feature(layer: usize, id: u64) -> FeatureRef {
        FeatureRef::new(layer, ObjectId(id))
    }

    #[test]
    fn modes_combine_with_the_current_selection() {
        let mut selection = Selection::new();

        selection.apply(SelectionMode::New, [feature(0, 1), feature(0, 2)]);
        assert_eq!(selection.len(), 2);
        assert_eq!(selection.primary(), Some(feature(0, 1)));

        selection.apply(SelectionMode::Add, [feature(0, 3)]);
        assert_eq!(selection.len(), 3);
        assert_eq!(selection.primary(), Some(feature(0, 1)));

        selection.apply(SelectionMode::Remove, [feature(0, 1)]);
        assert!(!selection.contains(&feature(0, 1)));
        assert_eq!(selection.primary(), Some(feature(0, 2)));

        selection.apply(SelectionMode::Intersect, [feature(0, 3), feature(0, 9)]);
        assert_eq!(selection.iter().collect::<Vec<_>>(), [feature(0, 3)]);
    }

    #[test]
    fn toggling_and_stepping() {
        let mut selection = Selection::new();

        selection.toggle(feature(1, 5));
        selection.toggle(feature(1, 7));
        assert_eq!(selection.primary(), Some(feature(1, 7)));
        assert_eq!(selection.position(), Some((2, 2)));

        selection.step(true);
        assert_eq!(selection.primary(), Some(feature(1, 5)));
        selection.step(false);
        assert_eq!(selection.primary(), Some(feature(1, 7)));

        selection.toggle(feature(1, 7));
        assert_eq!(selection.primary(), Some(feature(1, 5)));
        assert_eq!(selection.count_in(1), 1);
    }
}
