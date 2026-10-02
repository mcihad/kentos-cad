//! Malzeme tarayıcısı: kitaplığın, projenin ve bulutun malzemelerini
//! aranabilir, kategorili bir ızgarada gösterir; seçilen malzemenin
//! ayrıntısı yanda.
//!
//! ```text
//! [⌕ Malzeme ara          3/36 ×] [◉│▦]       ┌──────────────────┐
//! (Tümü) (Tuğla) (Beton) (Taş) (Ahşap) …      │       ( ◉ )      │
//! ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐             │ Kırmızı tuğla    │
//! │  ◉  │ │  ◉  │ │  ◉  │ │  ◉  │             │ Tuğla      Bulut │
//! │Tuğla│ │Beton│ │ Taş │ │Meşe │             │ Renk   #a4553f   │
//! └─────┘ └─────┘ └─────┘ └─────┘             │ [     Uygula    ]│
//! 36 malzeme                                  └──────────────────┘
//! ```
//!
//! Bileşen yalnız görüntüleyicidir: malzemeleri uygulama verir (kitaplık,
//! proje ya da sonradan bulut kitaplığı) ve süzme, seçim ve önizleme
//! biçimini kendi durumunda tutar. Bulut kitaplığı yüklenirken
//! [`MaterialBrowser::loading`] iskelet karolar gösterir; altta bir durum
//! satırı (ör. “Bulut kitaplığına bağlı değil”) verilebilir.
//!
//! ```ignore
//! MaterialBrowser::new(&self.materials, self.selected, Message::MaterialSelected)
//!     .search(&self.query, Message::MaterialSearch)
//!     .category(self.category.as_deref(), Message::MaterialCategory)
//!     .shape(self.shape, Message::MaterialShape)
//!     .action("Uygula", Message::MaterialApplied)
//! ```

use iced::widget::{Column, Row, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length};

use crate::icon::Icon;
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::library;
use crate::widget::material::{self, Material, Shape};
use crate::widget::{PropertyGrid, Segmented};

/// Karonun genişliği ve önizlemesinin kenarı, 12 piksellik gövde metninde.
const TILE: f32 = 92.0;
const PREVIEW: f32 = 64.0;
/// Ayrıntı panelinin genişliği ve büyük önizlemesi.
const DETAIL: f32 = 248.0;
const DETAIL_PREVIEW: f32 = 168.0;

type OnIndex<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;
type OnText<'a, Message> = Box<dyn Fn(String) -> Message + 'a>;
type OnCategory<'a, Message> = Box<dyn Fn(Option<String>) -> Message + 'a>;
type OnShape<'a, Message> = Box<dyn Fn(Shape) -> Message + 'a>;

/// Küre ve karo ikonları.
const SPHERE: &str = r#"<circle cx="10" cy="10" r="6.5"/><path d="M3.5 10c0 2 2.9 3.4 6.5 3.4s6.5-1.4 6.5-3.4" stroke-dasharray="1.6 1.6"/><circle cx="7.6" cy="7.4" r="1.2" fill="currentColor" stroke="none"/>"#;
const SWATCH: &str = r#"<rect x="3.5" y="3.5" width="13" height="13" rx="1.5"/><path d="M3.5 8h13M3.5 12.5h13M8 3.5V8M12.5 8v4.5M8 12.5v4"/>"#;

/// Malzeme tarayıcısı.
pub struct MaterialBrowser<'a, Message> {
    materials: &'a [Material],
    selected: Option<usize>,
    on_select: OnIndex<'a, Message>,
    on_activate: Option<OnIndex<'a, Message>>,
    search: Option<(&'a str, OnText<'a, Message>)>,
    category: Option<(Option<&'a str>, OnCategory<'a, Message>)>,
    shape: Option<(Shape, OnShape<'a, Message>)>,
    action: Option<(String, OnIndex<'a, Message>)>,
    detail: bool,
    loading: bool,
    status: Option<String>,
    height: Length,
}

impl<'a, Message: Clone + 'a> MaterialBrowser<'a, Message> {
    /// `selected` malzemelerin sırasıdır (süzmeden önceki).
    pub fn new(
        materials: &'a [Material],
        selected: Option<usize>,
        on_select: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        Self {
            materials,
            selected,
            on_select: Box::new(on_select),
            on_activate: None,
            search: None,
            category: None,
            shape: None,
            action: None,
            detail: true,
            loading: false,
            status: None,
            height: Length::Fill,
        }
    }

    /// Çift tıklanınca (ör. seçili nesneye uygula).
    pub fn on_activate(mut self, on_activate: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_activate = Some(Box::new(on_activate));
        self
    }

    /// Arama kutusu: ad, kategori, kimlik ve etiketlerde arar.
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

    /// Önizleme biçimi (küre ya da karo) ve değiştiren parçalı seçim.
    pub fn shape(mut self, shape: Shape, on_shape: impl Fn(Shape) -> Message + 'a) -> Self {
        self.shape = Some((shape, Box::new(on_shape)));
        self
    }

    /// Ayrıntı panelinin altındaki birincil eylem (ör. “Uygula”).
    pub fn action(
        mut self,
        label: impl Into<String>,
        on_action: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        self.action = Some((label.into(), Box::new(on_action)));
        self
    }

    /// Seçilen malzemenin ayrıntı paneli (varsayılan: gösterilir).
    pub fn detail(mut self, detail: bool) -> Self {
        self.detail = detail;
        self
    }

    /// Malzemeler yükleniyor (ör. bulut kitaplığından): iskelet karolar.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Izgaranın altındaki durum satırı (ör. “Bulut kitaplığına bağlı değil”).
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

impl<'a, Message: Clone + 'a> From<MaterialBrowser<'a, Message>> for Element<'a, Message> {
    fn from(browser: MaterialBrowser<'a, Message>) -> Self {
        let MaterialBrowser {
            materials,
            selected,
            on_select,
            on_activate,
            search,
            category,
            shape,
            action,
            detail,
            loading,
            status,
            height,
        } = browser;

        let query = search.as_ref().map_or("", |(query, _)| *query);
        let chosen_category = category.as_ref().and_then(|(category, _)| *category);
        let current_shape = shape.as_ref().map_or(Shape::Sphere, |(shape, _)| *shape);

        let visible: Vec<usize> = (0..materials.len())
            .filter(|&index| {
                let material = &materials[index];
                chosen_category.is_none_or(|category| material.category == category)
                    && material.matches(query)
            })
            .collect();

        // Üst satır: arama ve önizleme biçimi.
        let mut top = Row::new().spacing(8).align_y(Center);

        top = match search {
            Some((query, on_search)) => top.push(
                container(library::search_field(
                    "Malzeme ara",
                    query,
                    on_search,
                    visible.len(),
                    materials.len(),
                ))
                .width(Fill),
            ),
            None => top.push(space::horizontal()),
        };

        if let Some((current, on_shape)) = shape {
            top = top.push(
                Segmented::new(Shape::ALL, current, on_shape)
                    .icons([Icon::Svg(SPHERE), Icon::Svg(SWATCH)])
                    .icon_only(),
            );
        }

        let mut left = Column::new().spacing(10).width(Fill).push(top);

        if let Some((selected, on_category)) = &category {
            let categories = library::categories(materials.iter().map(|m| m.category.as_str()));
            left = left.push(library::chips(&categories, *selected, on_category));
        }

        let tile = typography::scaled(TILE).round();
        let preview_side = typography::scaled(PREVIEW).round();

        let body: Element<'a, Message> = if loading {
            library::skeletons(12, preview_side, tile, current_shape == Shape::Sphere)
        } else if visible.is_empty() {
            library::empty(if materials.is_empty() {
                "Kitaplıkta malzeme yok.".to_owned()
            } else {
                "Aramaya uyan malzeme yok. Aramayı ya da kategoriyi değiştirin.".to_owned()
            })
        } else {
            let tiles = visible.iter().map(|&index| {
                let material = &materials[index];
                let caption = material.size.map(|[w, h]| size_text(w, h));

                library::tile(
                    material::tile(material.look, current_shape, preview_side),
                    preview_side,
                    tile,
                    material.name.clone(),
                    caption.or_else(|| Some(material.category.clone())),
                    None,
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

        let count = if loading {
            "Yükleniyor…".to_owned()
        } else if chosen_category.is_some() || !query.trim().is_empty() {
            format!("{} / {} malzeme", visible.len(), materials.len())
        } else {
            format!("{} malzeme", materials.len())
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
            let panel = match selected.and_then(|index| materials.get(index).map(|m| (index, m))) {
                Some((index, material)) => detail_panel(
                    material,
                    current_shape,
                    action.map(|(label, on_action)| (label, on_action(index))),
                ),
                None => library::panel(
                    column![label::muted(
                        "Bir malzeme seçin: önizlemesi ve özellikleri burada görünür."
                    )],
                    Length::Fixed(typography::scaled(DETAIL)),
                ),
            };

            whole = whole.push(panel);
        }

        whole.into()
    }
}

/// Seçilen malzemenin ayrıntısı: büyük önizleme, ad, kategori ve kaynak,
/// özellikler, etiketler ve varsa eylem.
fn detail_panel<'a, Message: Clone + 'a>(
    material: &'a Material,
    shape: Shape,
    action: Option<(String, Message)>,
) -> Element<'a, Message> {
    let side = typography::scaled(DETAIL_PREVIEW).round();
    let small = typography::scaled(52.0).round();
    let other = match shape {
        Shape::Sphere => Shape::Swatch,
        Shape::Swatch => Shape::Sphere,
    };

    // Büyük önizleme; köşesinde öbür biçim küçük.
    let stage = iced::widget::stack![
        container(material::tile(material.look, shape, side)).center_x(Fill),
        container(material::tile(material.look, other, small))
            .width(Fill)
            .height(side)
            .align_x(iced::Right)
            .align_y(iced::Bottom),
    ];

    let look = &material.look;
    let mut grid = PropertyGrid::new()
        .property("Kimlik", material.id.clone())
        .property("Renk", material::hex(look.color))
        .figure("Pürüzlülük", decimal(look.roughness))
        .figure("Metaliklik", decimal(look.metallic))
        .property("Desen", look.pattern.label());

    if let Some([width, height]) = material.size {
        grid = grid.figure("Boyut", size_text(width, height));
    }

    let mut content = column![
        stage,
        column![
            label::title(material.name.clone()),
            row![
                label::muted(material.category.clone()),
                space::horizontal(),
                source_pill(material.source),
            ]
            .align_y(Center),
        ]
        .spacing(4),
        grid,
    ];

    if !material.tags.is_empty() {
        content = content.push(library::tags(&material.tags));
    }

    if let Some((label, message)) = action {
        content = content.push(library::action(label, message));
    }

    library::panel(content, Length::Fixed(typography::scaled(DETAIL)))
}

/// Kaynağın küçük hapı (Kitaplık, Proje, Bulut).
fn source_pill<'a, Message: 'a>(source: material::Source) -> Element<'a, Message> {
    container(label::caption(source.label()).style(style::text::default))
        .padding([1, 8])
        .style(style::container::badge)
        .into()
}

/// 0..1 değerinin yazımı: iki basamak, Türkçe ondalık.
fn decimal(value: f32) -> String {
    crate::attribute::number::real(f64::from(value), 2)
}

/// Desenin gerçek boyutu: bir metreden küçükse santim (“25 × 7,5 cm”),
/// değilse metre (“2,40 × 1,20 m”).
fn size_text(width: f32, height: f32) -> String {
    let real = crate::attribute::number::real;

    if width < 1.0 && height < 1.0 {
        let centimetres = |value: f32| {
            let value = f64::from(value) * 100.0;
            let whole = (value - value.round()).abs() < 0.05;

            real(value, if whole { 0 } else { 1 })
        };

        format!("{} × {} cm", centimetres(width), centimetres(height))
    } else {
        format!(
            "{} × {} m",
            real(f64::from(width), 2),
            real(f64::from(height), 2)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_values_are_written_in_turkish() {
        assert_eq!(size_text(0.25, 0.075), "25 × 7,5 cm");
        assert_eq!(size_text(2.4, 1.2), "2,40 × 1,20 m");
        assert_eq!(decimal(0.85), "0,85");
    }
}
