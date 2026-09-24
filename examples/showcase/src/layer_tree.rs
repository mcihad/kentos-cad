//! İçindekiler: katmanların gruplar hâlindeki ağacı.
//!
//! Gruplar katmanları ve başka grupları toplar; derinlik sınırsızdır. Her
//! grubun ve katmanın kendi görünürlüğü vardır. Bir katman, kendisi ve
//! bütün üst grupları görünürse çizilir (ArcGIS'teki gibi): grubu kapatmak
//! içindekilerin işaretlerini değiştirmez, açınca eski hâlleriyle görünürler.
//! Alt katmanlar kütüphanenin `Sublayer`larıdır; görünürlükleri katmanda
//! tutulur.

/// Ağaçtaki bir düğüm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeId {
    Group(usize),
    Layer(usize),
    /// Katman sırası ve alt katman sırası.
    Sublayer(usize, usize),
}

impl NodeId {
    /// Düğümün katmanı; grupta yok.
    pub fn layer(self) -> Option<usize> {
        match self {
            NodeId::Group(_) => None,
            NodeId::Layer(layer) | NodeId::Sublayer(layer, _) => Some(layer),
        }
    }
}

/// Bir grubun ya da katmanın ağaçtaki yeri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    Group(usize),
    Layer(usize),
}

/// Katmanları ve başka grupları toplayan grup.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub visible: bool,
    pub expanded: bool,
    pub children: Vec<Entry>,
}

/// Katman ağacı.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerTree {
    /// Gruplar; `Entry::Group` bu listedeki sırayı gösterir.
    pub groups: Vec<Group>,
    /// En üst düzeydeki girdiler, çizim sırasıyla (üstteki üstte çizilir).
    pub roots: Vec<Entry>,
    /// Katmanların kendi görünürlüğü; çizilmesi üst gruplara da bağlıdır.
    pub visible: Vec<bool>,
    /// Alt katmanları açık katmanlar.
    pub expanded: Vec<bool>,
    /// Ağaçta seçili düğüm.
    pub selected: Option<NodeId>,
}

impl LayerTree {
    /// Katmanları grupsuz, düz bir liste olarak dizer.
    pub fn flat(layers: usize) -> Self {
        Self {
            groups: Vec::new(),
            roots: (0..layers).map(Entry::Layer).collect(),
            visible: vec![true; layers],
            expanded: vec![false; layers],
            selected: None,
        }
    }

    /// Grup ekler ve numarasını döndürür; grup henüz ağaçta bir yere
    /// bağlanmamıştır.
    pub fn add_group(&mut self, name: &str, expanded: bool, children: Vec<Entry>) -> usize {
        self.groups.push(Group {
            name: name.to_owned(),
            visible: true,
            expanded,
            children,
        });

        self.groups.len() - 1
    }

    /// Girdinin bağlı olduğu grup; en üst düzeyde `None`.
    pub fn parent(&self, entry: Entry) -> Option<usize> {
        self.groups
            .iter()
            .position(|group| group.children.contains(&entry))
    }

    /// Girdinin kendi görünürlüğü.
    fn own(&self, entry: Entry) -> bool {
        match entry {
            Entry::Group(group) => self.groups.get(group).is_some_and(|group| group.visible),
            Entry::Layer(layer) => self.visible.get(layer).copied().unwrap_or(false),
        }
    }

    /// Üst grupların hepsi görünür mü (girdinin kendisi hariç).
    pub fn ancestors_shown(&self, entry: Entry) -> bool {
        let mut current = entry;

        while let Some(parent) = self.parent(current) {
            if !self.groups[parent].visible {
                return false;
            }

            current = Entry::Group(parent);
        }

        true
    }

    /// Girdi ve bütün üst grupları görünür mü: katman çizilir mi.
    pub fn is_shown(&self, entry: Entry) -> bool {
        self.own(entry) && self.ancestors_shown(entry)
    }

    /// Her katmanın çizilip çizilmeyeceği, katman sırasıyla.
    pub fn effective(&self) -> Vec<bool> {
        (0..self.visible.len())
            .map(|layer| self.is_shown(Entry::Layer(layer)))
            .collect()
    }

    /// Grubun içindeki bütün katmanlar (alt gruplar dahil), ağaç sırasıyla.
    pub fn layers_in(&self, group: usize) -> Vec<usize> {
        let mut layers = Vec::new();
        self.collect_layers(&self.groups[group].children, &mut layers);
        layers
    }

    fn collect_layers(&self, entries: &[Entry], layers: &mut Vec<usize>) {
        for entry in entries {
            match *entry {
                Entry::Layer(layer) => layers.push(layer),
                Entry::Group(group) => self.collect_layers(&self.groups[group].children, layers),
            }
        }
    }

    /// Katmanı ve üst gruplarını görünür yapar.
    pub fn reveal(&mut self, layer: usize) {
        if let Some(visible) = self.visible.get_mut(layer) {
            *visible = true;
        }

        let mut current = Entry::Layer(layer);

        while let Some(parent) = self.parent(current) {
            self.groups[parent].visible = true;
            current = Entry::Group(parent);
        }
    }

    /// Yalnızca verilen katmanlar görünür; üst grupları da açılır.
    pub fn show_only(&mut self, layers: &[usize]) {
        for (layer, visible) in self.visible.iter_mut().enumerate() {
            *visible = layers.contains(&layer);
        }

        for &layer in layers {
            self.reveal(layer);
        }
    }

    /// Düğümün açık/kapalı durumunu değiştirir.
    pub fn toggle(&mut self, node: NodeId) {
        match node {
            NodeId::Group(group) => {
                if let Some(group) = self.groups.get_mut(group) {
                    group.expanded = !group.expanded;
                }
            }
            NodeId::Layer(layer) => {
                if let Some(expanded) = self.expanded.get_mut(layer) {
                    *expanded = !*expanded;
                }
            }
            NodeId::Sublayer(..) => {}
        }
    }

    /// Grubu ve içindeki bütün grupları ve katmanları açar ya da kapatır;
    /// `None` bütün ağaç içindir.
    pub fn expand_all(&mut self, group: Option<usize>, expanded: bool) {
        let entries = match group {
            Some(group) => {
                self.groups[group].expanded = expanded;
                self.groups[group].children.clone()
            }
            None => self.roots.clone(),
        };

        for entry in entries {
            match entry {
                Entry::Group(group) => self.expand_all(Some(group), expanded),
                Entry::Layer(layer) => {
                    if let Some(open) = self.expanded.get_mut(layer) {
                        *open = expanded;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Çizimler, Türkiye { Yerleşim { Şehirler, Önemli Yerler }, Ulaşım {
    /// Karayolları } }
    fn tree() -> LayerTree {
        let mut tree = LayerTree::flat(4);
        let settlement = tree.add_group("Yerleşim", true, vec![Entry::Layer(1), Entry::Layer(2)]);
        let transport = tree.add_group("Ulaşım", false, vec![Entry::Layer(3)]);
        let country = tree.add_group(
            "Türkiye",
            true,
            vec![Entry::Group(settlement), Entry::Group(transport)],
        );

        tree.roots = vec![Entry::Layer(0), Entry::Group(country)];
        tree
    }

    #[test]
    fn hidden_groups_hide_their_layers_without_changing_them() {
        let mut tree = tree();
        assert_eq!(tree.effective(), [true; 4]);

        tree.groups[2].visible = false;
        assert_eq!(tree.effective(), [true, false, false, false]);
        assert!(tree.visible[1], "katmanın kendi işareti değişmez");

        tree.groups[2].visible = true;
        tree.groups[0].visible = false;
        assert_eq!(tree.effective(), [true, false, false, true]);
    }

    #[test]
    fn revealing_a_layer_opens_its_groups() {
        let mut tree = tree();
        tree.groups[2].visible = false;
        tree.visible[3] = false;

        tree.reveal(3);

        // Türkiye açılınca Yerleşim'deki katmanlar da yeniden görünür.
        assert_eq!(tree.effective(), [true; 4]);
    }

    #[test]
    fn groups_collect_nested_layers_in_order() {
        let tree = tree();

        assert_eq!(tree.layers_in(2), [1, 2, 3]);
        assert_eq!(tree.parent(Entry::Layer(3)), Some(1));
        assert_eq!(tree.parent(Entry::Group(0)), Some(2));
        assert_eq!(tree.parent(Entry::Layer(0)), None);
    }

    #[test]
    fn show_only_and_expand_all() {
        let mut tree = tree();

        tree.show_only(&[2]);
        assert_eq!(tree.effective(), [false, false, true, false]);

        tree.expand_all(None, false);
        assert!(tree.groups.iter().all(|group| !group.expanded));

        tree.expand_all(Some(2), true);
        assert!(tree.groups.iter().all(|group| group.expanded));
        assert!(tree.expanded[1..].iter().all(|open| *open));
        assert!(!tree.expanded[0], "grubun dışındaki katman açılmaz");
    }
}
