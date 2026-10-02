//! 3B nesne tarayıcısı: yapı, kent mobilyası, bitki ve altyapı modellerini
//! aranabilir, kategorili bir ızgarada gösterir; seçilen nesne yanda
//! döndürülebilir önizlemesi, ölçüleri ve ayrıntı düzeyiyle.
//!
//! ```text
//! [⌕ Nesne ara       ×] [Tümü│LOD1│LOD2│LOD3]   ┌───────────────────┐
//! (Tümü) (Yapı) (Kent mobilyası) (Bitki) …      │   ⬚ döner önizleme │
//! ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐               │  ├─ 8,0 m ─┤       │
//! │ ⌂   │ │ ▥   │ │ ♣   │ │ ⊥   │               │ Müstakil ev   LOD2│
//! │ Ev  │ │Blok │ │Çınar│ │Lamba│               │ Boyut 8 × 6 × 5 m │
//! └─────┘ └─────┘ └─────┘ └─────┘               │ [   Yerleştir    ]│
//! ```
//!
//! Bileşen yalnız görüntüleyicidir: nesneleri uygulama verir (kitaplık,
//! proje ya da sonradan bulut kitaplığı). Önizleme nesnenin ağından
//! ([`Mesh`]) çizilir; ağ henüz yüklenmemişse yeri boş kalır. Ayrıntı
//! düzeyi CityGML'in LOD kademeleridir.
//!
//! ```ignore
//! ObjectBrowser::new(&self.objects, self.selected, Message::ObjectSelected)
//!     .search(&self.query, Message::ObjectSearch)
//!     .category(self.category.as_deref(), Message::ObjectCategory)
//!     .lod(self.lod, Message::ObjectLod)
//!     .action("Yerleştir", Message::ObjectPlaced)
//! ```

use std::fmt;
use std::rc::Rc;

use iced::widget::{Column, Row, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length};

use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::library;
pub use crate::widget::material::Source;
use crate::widget::mesh::{self, Camera, Mesh};
use crate::widget::{PropertyGrid, Segmented};

/// Karonun genişliği ve önizlemesinin kenarı, 12 piksellik gövde metninde.
const TILE: f32 = 100.0;
const PREVIEW: f32 = 72.0;
/// Ayrıntı panelinin genişliği ve önizlemesinin yüksekliği.
const DETAIL: f32 = 268.0;
const DETAIL_PREVIEW: f32 = 210.0;

/// Ayrıntı düzeyi (CityGML LOD).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lod {
    /// İz: taban ve çatı izdüşümü.
    Lod0,
    /// Blok model: düz çatılı kütle.
    Lod1,
    /// Çatı biçimleri ve ana çıkıntılar.
    Lod2,
    /// Mimari ayrıntı: pencere, kapı, balkon.
    Lod3,
    /// İç mekân.
    Lod4,
}

impl Lod {
    pub const ALL: [Lod; 5] = [Lod::Lod0, Lod::Lod1, Lod::Lod2, Lod::Lod3, Lod::Lod4];

    pub fn label(self) -> &'static str {
        match self {
            Lod::Lod0 => "LOD0",
            Lod::Lod1 => "LOD1",
            Lod::Lod2 => "LOD2",
            Lod::Lod3 => "LOD3",
            Lod::Lod4 => "LOD4",
        }
    }

    /// Kademenin kısa açıklaması.
    pub fn description(self) -> &'static str {
        match self {
            Lod::Lod0 => "İz",
            Lod::Lod1 => "Blok model",
            Lod::Lod2 => "Çatılı model",
            Lod::Lod3 => "Mimari ayrıntı",
            Lod::Lod4 => "İç mekân",
        }
    }
}

impl fmt::Display for Lod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Ayrıntı düzeyi süzgeci: bütün kademeler ya da biri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LodFilter {
    #[default]
    All,
    Only(Lod),
}

impl fmt::Display for LodFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LodFilter::All => f.write_str("Tümü"),
            LodFilter::Only(lod) => f.write_str(lod.label()),
        }
    }
}

/// Kitaplıktaki bir 3B nesne.
#[derive(Debug, Clone, PartialEq)]
pub struct Object3d {
    /// Kalıcı kimlik (ör. bulut kitaplığındaki anahtar).
    pub id: String,
    pub name: String,
    pub category: String,
    /// Önizlemenin ağı; henüz yüklenmemişse yok.
    pub mesh: Option<Rc<Mesh>>,
    pub lod: Lod,
    /// Dosya biçimi (ör. glTF, IFC, CityGML).
    pub format: String,
    /// Kaynaktaki üçgen sayısı; verilmezse önizleme ağınınki.
    pub triangles: Option<usize>,
    /// Gerçek boyut (metre); verilmezse ağın kapsayan kutusundan.
    pub size: Option<[f32; 3]>,
    pub source: Source,
    pub tags: Vec<String>,
}

impl Object3d {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
        mesh: Option<Rc<Mesh>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: category.into(),
            mesh,
            lod: Lod::Lod2,
            format: "glTF".to_owned(),
            triangles: None,
            size: None,
            source: Source::default(),
            tags: Vec::new(),
        }
    }

    pub fn lod(mut self, lod: Lod) -> Self {
        self.lod = lod;
        self
    }

    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = format.into();
        self
    }

    pub fn triangles(mut self, triangles: usize) -> Self {
        self.triangles = Some(triangles);
        self
    }

    pub fn size(mut self, size: [f32; 3]) -> Self {
        self.size = Some(size);
        self
    }

    pub fn source(mut self, source: Source) -> Self {
        self.source = source;
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Gerçek boyut: verilmişse o, yoksa ağdan.
    pub fn dimensions(&self) -> Option<[f32; 3]> {
        self.size
            .or_else(|| self.mesh.as_ref().map(|mesh| mesh.size()))
    }

    /// Üçgen sayısı: verilmişse o, yoksa ağdan.
    pub fn triangle_count(&self) -> Option<usize> {
        self.triangles
            .or_else(|| self.mesh.as_ref().map(|mesh| mesh.triangles()))
    }

    /// Arama: ad, kategori, kimlik, biçim ya da etiketlerden biri yazılanı
    /// içeriyor mu (Türkçe harf ve büyük/küçük harf ayırmadan).
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim();
        let contains = crate::attribute::text::contains;

        query.is_empty()
            || contains(&self.name, query)
            || contains(&self.category, query)
            || contains(&self.id, query)
            || contains(&self.format, query)
            || contains(self.lod.label(), query)
            || self.tags.iter().any(|tag| contains(tag, query))
    }
}

type OnIndex<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;
type OnText<'a, Message> = Box<dyn Fn(String) -> Message + 'a>;
type OnCategory<'a, Message> = Box<dyn Fn(Option<String>) -> Message + 'a>;
type OnLod<'a, Message> = Box<dyn Fn(LodFilter) -> Message + 'a>;

/// 3B nesne tarayıcısı.
pub struct ObjectBrowser<'a, Message> {
    objects: &'a [Object3d],
    selected: Option<usize>,
    on_select: OnIndex<'a, Message>,
    on_activate: Option<OnIndex<'a, Message>>,
    search: Option<(&'a str, OnText<'a, Message>)>,
    category: Option<(Option<&'a str>, OnCategory<'a, Message>)>,
    lod: Option<(LodFilter, OnLod<'a, Message>)>,
    action: Option<(String, OnIndex<'a, Message>)>,
    detail: bool,
    loading: bool,
    status: Option<String>,
    height: Length,
}

impl<'a, Message: Clone + 'a> ObjectBrowser<'a, Message> {
    /// `selected` nesnelerin sırasıdır (süzmeden önceki).
    pub fn new(
        objects: &'a [Object3d],
        selected: Option<usize>,
        on_select: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        Self {
            objects,
            selected,
            on_select: Box::new(on_select),
            on_activate: None,
            search: None,
            category: None,
            lod: None,
            action: None,
            detail: true,
            loading: false,
            status: None,
            height: Length::Fill,
        }
    }

    /// Çift tıklanınca (ör. çizime yerleştir).
    pub fn on_activate(mut self, on_activate: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_activate = Some(Box::new(on_activate));
        self
    }

    /// Arama kutusu: ad, kategori, kimlik, biçim, LOD ve etiketlerde arar.
    pub fn search(mut self, query: &'a str, on_search: impl Fn(String) -> Message + 'a) -> Self {
        self.search = Some((query, Box::new(on_search)));
        self
    }

    /// Kategori hapları; `None` bütün kategoriler.
    pub fn category(
        mut self,
        category: Option<&'a str>,
        on_category: impl Fn(Option<String>) -> Message + 'a,
    ) -> Self {
        self.category = Some((category, Box::new(on_category)));
        self
    }

    /// Ayrıntı düzeyi süzgeci: nesnelerde bulunan kademelerin parçalı seçimi.
    pub fn lod(mut self, filter: LodFilter, on_lod: impl Fn(LodFilter) -> Message + 'a) -> Self {
        self.lod = Some((filter, Box::new(on_lod)));
        self
    }

    /// Ayrıntı panelinin altındaki birincil eylem (ör. “Yerleştir”).
    pub fn action(
        mut self,
        label: impl Into<String>,
        on_action: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        self.action = Some((label.into(), Box::new(on_action)));
        self
    }

    /// Seçilen nesnenin ayrıntı paneli (varsayılan: gösterilir).
    pub fn detail(mut self, detail: bool) -> Self {
        self.detail = detail;
        self
    }

    /// Nesneler yükleniyor (ör. bulut kitaplığından): iskelet karolar.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Izgaranın altındaki durum satırı.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Bileşenin yüksekliği (varsayılan: `Fill`); ızgara içinde kayar.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<'a, Message: Clone + 'a> From<ObjectBrowser<'a, Message>> for Element<'a, Message> {
    fn from(browser: ObjectBrowser<'a, Message>) -> Self {
        let ObjectBrowser {
            objects,
            selected,
            on_select,
            on_activate,
            search,
            category,
            lod,
            action,
            detail,
            loading,
            status,
            height,
        } = browser;

        let query = search.as_ref().map_or("", |(query, _)| *query);
        let chosen_category = category.as_ref().and_then(|(category, _)| *category);
        let chosen_lod = lod.as_ref().map_or(LodFilter::All, |(filter, _)| *filter);

        let visible: Vec<usize> = (0..objects.len())
            .filter(|&index| {
                let object = &objects[index];
                chosen_category.is_none_or(|category| object.category == category)
                    && match chosen_lod {
                        LodFilter::All => true,
                        LodFilter::Only(lod) => object.lod == lod,
                    }
                    && object.matches(query)
            })
            .collect();

        let mut top = Row::new().spacing(8).align_y(Center);

        top = match search {
            Some((query, on_search)) => top.push(
                container(library::search_field(
                    "Nesne ara",
                    query,
                    on_search,
                    visible.len(),
                    objects.len(),
                ))
                .width(Fill),
            ),
            None => top.push(space::horizontal()),
        };

        if let Some((filter, on_lod)) = lod {
            // Yalnız nesnelerde bulunan kademeler.
            let mut present: Vec<Lod> = objects.iter().map(|object| object.lod).collect();
            present.sort();
            present.dedup();

            let options = std::iter::once(LodFilter::All)
                .chain(present.into_iter().map(LodFilter::Only))
                .collect::<Vec<_>>();
            let hints = options.iter().map(|option| match option {
                LodFilter::All => "Bütün ayrıntı düzeyleri".to_owned(),
                LodFilter::Only(lod) => lod.description().to_owned(),
            });

            top = top.push(Segmented::new(options.clone(), filter, on_lod).hints(hints));
        }

        let mut left = Column::new().spacing(10).width(Fill).push(top);

        if let Some((selected, on_category)) = &category {
            let categories = library::categories(objects.iter().map(|o| o.category.as_str()));
            left = left.push(library::chips(&categories, *selected, on_category));
        }

        let tile = typography::scaled(TILE).round();
        let preview_side = typography::scaled(PREVIEW).round();

        let body: Element<'a, Message> = if loading {
            library::skeletons(12, preview_side, tile, false)
        } else if visible.is_empty() {
            library::empty(if objects.is_empty() {
                "Kitaplıkta 3B nesne yok.".to_owned()
            } else {
                "Aramaya uyan nesne yok. Aramayı, kategoriyi ya da ayrıntı düzeyini değiştirin."
                    .to_owned()
            })
        } else {
            let tiles = visible.iter().map(|&index| {
                let object = &objects[index];
                let preview: Element<'a, Message> = match &object.mesh {
                    Some(mesh) => mesh::view(mesh.clone()).into(),
                    None => placeholder(),
                };

                library::tile(
                    preview,
                    preview_side,
                    tile,
                    object.name.clone(),
                    object.dimensions().map(|[w, d, h]| size_text(w, d, h)),
                    Some(object.lod.label().to_owned()),
                    selected == Some(index),
                    on_select(index),
                    on_activate.as_ref().map(|on_activate| on_activate(index)),
                )
            });

            Row::with_children(tiles)
                .spacing(8)
                .wrap()
                .vertical_spacing(8)
                .into()
        };

        left = left.push(
            scrollable(container(body).width(Fill).padding(iced::Padding {
                right: 10.0,
                ..iced::Padding::ZERO
            }))
            .direction(style::field::thin_scrollbar())
            .width(Fill)
            .height(Fill),
        );

        let filtered =
            chosen_category.is_some() || chosen_lod != LodFilter::All || !query.trim().is_empty();
        let count = if loading {
            "Yükleniyor…".to_owned()
        } else if filtered {
            format!("{} / {} nesne", visible.len(), objects.len())
        } else {
            format!("{} nesne", objects.len())
        };

        let mut footer = row![label::caption(count)].spacing(8).align_y(Center);

        if let Some(status) = status {
            footer = footer
                .push(space::horizontal())
                .push(label::caption(status));
        }

        left = left.push(footer);

        let mut whole = Row::new().spacing(12).height(height).push(left);

        if detail {
            let panel = match selected.and_then(|index| objects.get(index).map(|o| (index, o))) {
                Some((index, object)) => detail_panel(
                    object,
                    action.map(|(label, on_action)| (label, on_action(index))),
                ),
                None => library::panel(
                    column![label::muted(
                        "Bir nesne seçin: döndürülebilir önizlemesi ve ölçüleri burada görünür."
                    )],
                    Length::Fixed(typography::scaled(DETAIL)),
                ),
            };

            whole = whole.push(panel);
        }

        whole.into()
    }
}

/// Ağı yüklenmemiş nesnenin önizleme yeri.
fn placeholder<'a, Message: 'a>() -> Element<'a, Message> {
    container(label::caption("Önizleme yok"))
        .center(Fill)
        .style(|theme: &iced::Theme| container::Style {
            background: Some(crate::theme::Tokens::of(theme).layer(0.04).into()),
            border: iced::Border {
                radius: crate::theme::shape::md().into(),
                ..iced::Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// Seçilen nesnenin ayrıntısı: döndürülebilir önizleme, ad, kategori ve
/// ayrıntı düzeyi, ölçüler ve kaynak bilgileri, etiketler ve varsa eylem.
fn detail_panel<'a, Message: Clone + 'a>(
    object: &'a Object3d,
    action: Option<(String, Message)>,
) -> Element<'a, Message> {
    let stage: Element<'a, Message> = match &object.mesh {
        Some(mesh) => container(mesh::detail(mesh.clone(), Camera::DEFAULT))
            .height(typography::scaled(DETAIL_PREVIEW))
            .style(|theme: &iced::Theme| container::Style {
                background: Some(crate::theme::Tokens::of(theme).field.into()),
                border: iced::Border {
                    radius: crate::theme::shape::md().into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            })
            .into(),
        None => container(placeholder())
            .height(typography::scaled(DETAIL_PREVIEW))
            .into(),
    };

    let mut grid = PropertyGrid::new().property("Kimlik", object.id.clone());

    if let Some([w, d, h]) = object.dimensions() {
        grid = grid
            .figure("Genişlik", metres(w))
            .figure("Derinlik", metres(d))
            .figure("Yükseklik", metres(h));
    }

    if let Some(triangles) = object.triangle_count() {
        grid = grid.figure("Üçgen", crate::attribute::number::integer(triangles as i64));
    }

    grid = grid
        .property(
            "Ayrıntı",
            format!("{} ({})", object.lod.label(), object.lod.description()),
        )
        .property("Biçim", object.format.clone())
        .property("Kaynak", object.source.label());

    let mut content = column![
        stage,
        label::caption("Sürükleyerek döndürün; çift tık ilk bakışa döndürür."),
        column![
            label::title(object.name.clone()),
            label::muted(object.category.clone()),
        ]
        .spacing(4),
        grid,
    ];

    if !object.tags.is_empty() {
        content = content.push(library::tags(&object.tags));
    }

    if let Some((label, message)) = action {
        content = content.push(library::action(label, message));
    }

    library::panel(content, Length::Fixed(typography::scaled(DETAIL)))
}

/// Metre değeri: bir basamak.
fn metres(value: f32) -> String {
    format!("{} m", crate::attribute::number::real(f64::from(value), 1))
}

/// Karodaki boyut: “8 × 6 × 5,2 m” (gereksiz sıfırlar yazılmaz).
fn size_text(width: f32, depth: f32, height: f32) -> String {
    let short = |value: f32| {
        let value = f64::from(value);
        let whole = (value - value.round()).abs() < 0.05;

        crate::attribute::number::real(value, if whole { 0 } else { 1 })
    };

    format!("{} × {} × {} m", short(width), short(depth), short(height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_match_by_name_category_lod_and_tag() {
        let lamp = Object3d::new("lamba", "Sokak lambası", "Kent mobilyası", None)
            .lod(Lod::Lod3)
            .format("glTF")
            .tags(["aydınlatma"]);

        assert!(lamp.matches("SOKAK"));
        assert!(lamp.matches("mobilya"));
        assert!(lamp.matches("lod3"));
        assert!(lamp.matches("gltf"));
        assert!(lamp.matches("Aydınlatma"));
        assert!(!lamp.matches("çınar"));
    }

    #[test]
    fn dimensions_and_triangles_come_from_the_mesh_unless_given() {
        let mesh = Rc::new(Mesh::cuboid([0.0; 3], [2.0, 3.0, 4.0], iced::Color::WHITE));
        let block = Object3d::new("blok", "Blok", "Yapı", Some(mesh));

        assert_eq!(block.dimensions(), Some([2.0, 3.0, 4.0]));
        assert_eq!(block.triangle_count(), Some(12));

        let given = block.clone().size([20.0, 30.0, 40.0]).triangles(1_284);
        assert_eq!(given.dimensions(), Some([20.0, 30.0, 40.0]));
        assert_eq!(given.triangle_count(), Some(1_284));
    }

    #[test]
    fn sizes_drop_needless_zeros() {
        assert_eq!(size_text(8.0, 6.0, 5.2), "8 × 6 × 5,2 m");
        assert_eq!(metres(12.04), "12,0 m");
    }
}
