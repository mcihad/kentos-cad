//! The model viewer's look (DESIGN.md §7.21, docs/adr/8061 §5, docs/adr/8073 §2, docs/adr/8118): one tool strip over
//! three panes and a status bar, the 3B görünüm's vocabulary. The strip holds the camera (Görünüş ▾, Sığdır, Seçime
//! yaklaş), then what shows (Gizle, Yalnız bırak, Hepsini göster; Mahaller, Açıklıklar, Saydam), then Kesit ▾, then
//! whatever the host puts at its end; it keeps its length whatever is cut. The panes: the tree (by structure, by class,
//! or the objects that could not be shown) with its search over it, the scene with its north arrow, its legend and,
//! while something cuts it, the section's card, the chosen object's details; the side panes widen by their edges. The
//! bar: the point clicked and what it is on, how much shows, what could not be shown, the host's facts (the schema), the
//! mouse's help. While the file is read the panes give way to its progress; a file refused says why.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
//! │ ◈ Kuzeydoğu ▾ ⤢ Sığdır ◎ Seçime yaklaş │ Gizle Yalnız bırak Hepsini göster │ ▱ Mahaller ▭ Açıklıklar ◐ Saydam │ Kesit ▾ │
//! ├ Yapı | Sınıf | Gösterilemeyen ┬┬─────────────────────────────────────────────┬┬ Özellikler ───────────────┤
//! │ ⌕ Ad, sınıf ya da GlobalId    ││ ┌ Yatay kesit ──────── × ┐             (K) ││ ▾ Nesne                    │
//! │ ▾ Proje                    1  ││ │ Kot ━━━━●━━━  1.604 m  │                 ││   Sınıf      IfcWall       │
//! │   ▾ Bina                   2  ││ │ Öbür yanı               │                 ││ ▾ Qto_WallBaseQuantities   │
//! │     ▸ Zemin kat           31  ││ └────────────────────────┘ ┌ Lejant ┐      ││   Length     12.000  m     │
//! ├───────────────────────────────┴┴─────────────────────────────────────────────┴┴───────────────────────────┤
//! │ ⌖ Duvar   Y …  X …  Z …                         70/70 gövde │ ⚠ 2 gösterilemeyen │ IFC4X3_ADD2 │ ? Fare   │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::{Row, button, column, container, row, scrollable, slider, space, text};
use kentos_ui::widget::input_field::InputField;
use kentos_ui::widget::sash::Sash;
use iced::{Alignment, Background, Center, Element, Fill, Theme};
use kentos_model_view::Cut;
use kentos_model_view::section::Section;
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::widget::tree_view::{self, Node, Toggle, TreeView};
use kentos_ui::widget::{EmptyState, Menu, PropertySheet, StatusBar, Tip, progress, property_grid, segmented::Segmented, strip};

use crate::look;
use crate::scene::{self, SceneView};
use crate::viewer::{Message, Panel, Stage, Viewer};

/// What the host decides of the viewer's look: how its messages reach it, what it puts at the strip's end, the sky
/// the scene stands on, how a coordinate is written, how a measured value is written by its unit (`m`, `m²`, `m³`,
/// none), and what the east and north axes are called.
pub struct Host<'a, M> {
    pub to: fn(Message) -> M,
    pub end: Vec<Element<'a, M>>,
    pub sky: Background,
    /// The quality the scene is drawn at (the application's setting, docs/adr/8120).
    pub quality: scene::Quality,
    pub coordinate: Box<dyn Fn(f64) -> String + 'a>,
    pub measure: Box<dyn Fn(f64, Option<&str>) -> String + 'a>,
    pub east: &'a str,
    pub north: &'a str,
}

/// The side panels' widths at 12 px text, and how far they may be dragged.
const TREE_WIDTH: f32 = 300.0;
const DETAILS_WIDTH: f32 = 300.0;
const TREE_RANGE: std::ops::RangeInclusive<f32> = 200.0..=560.0;
const DETAILS_RANGE: std::ops::RangeInclusive<f32> = 240.0..=600.0;
/// The section card's width at 12 px text.
const CARD_WIDTH: f32 = 330.0;
/// The sky's and the fill's shares of a building's light (docs/adr/8120).
const SKY: f32 = 0.16;
const FILL: f32 = 0.22;
/// Where a measured value of the file is said to come from, in its tip.
const IN_FILE: &str = "Dosyadaki değer";

impl Viewer {
    /// The whole viewer.
    pub fn layout<'a, M: Clone + 'a>(&'a self, mut host: Host<'a, M>) -> Element<'a, M> {
        let to = host.to;
        let end = std::mem::take(&mut host.end);
        let status = self.status(&host).map(to);
        let tools: Vec<Element<'a, M>> = self.strip().into_iter().map(|e| e.map(to)).collect();
        let strip = strip::surface(
            row![
                Row::from_vec(tools).spacing(1).align_y(Center).width(Fill).wrap().vertical_spacing(4),
                Row::from_vec(end).spacing(1).align_y(Center)
            ]
            .spacing(6)
            .align_y(Center),
        );
        let body: Element<'a, Message> = match &self.stage {
            Stage::Reading { phase, done, total } => reading(phase, *done, *total),
            Stage::Failed(why) => container(
                EmptyState::error("Dosya açılamadı").description(why.as_str()).primary("Kapat", Message::Close),
            )
            .center(Fill)
            .into(),
            Stage::Shown => {
                // The side panes widen by their edges; a double click gives back their width (DESIGN.md §7.21).
                let tree = self.tree_width.unwrap_or_else(|| typography::scaled(TREE_WIDTH));
                let details = self.details_width.unwrap_or_else(|| typography::scaled(DETAILS_WIDTH));
                row![
                    container(self.tree_panel()).width(tree).height(Fill),
                    Sash::vertical(tree, |w| Message::TreeWidth(Some(w)))
                        .range(TREE_RANGE)
                        .on_double_click(Message::TreeWidth(None)),
                    self.scene_pane(&host),
                    Sash::vertical(details, |w| Message::DetailsWidth(Some(w)))
                        .reverse()
                        .range(DETAILS_RANGE)
                        .on_double_click(Message::DetailsWidth(None)),
                    container(self.details_panel(&host)).width(details).height(Fill),
                ]
                .height(Fill)
                .into()
            }
        };
        column![strip, strip::rule(), body.map(to), status].height(Fill).into()
    }

    fn strip<'a>(&'a self) -> Vec<Element<'a, Message>> {
        let orbit = self.orbit;
        let shown = self.stage == Stage::Shown;
        let chosen = self.view().and_then(|v| v.selected()).is_some();
        let partial = self.view().is_some_and(|v| v.is_isolated() || (0..v.input().objects.len()).any(|o| v.is_hidden(o)));
        let when = |ok: bool, m: Message| ok.then_some(m);
        let mut out: Vec<Element<'a, Message>> = vec![
            strip::menu(
                Icon::Cube,
                look::view_name(&orbit),
                "Görünüş: modele hangi yönden bakıldığı. Döndürünce bakış serbesttir; bir görünüş seçmek modeli o yönden sığdırır.",
                move || look::views_menu(&orbit, Message::View, Message::North),
            ),
            strip::labelled(Icon::ZoomExtents, "Sığdır", "Sığdır: bütün modeli bu bakışla görünüme sığdırır.", when(shown, Message::Fit)),
            strip::labelled(
                Icon::Target,
                "Seçime yaklaş",
                "Seçime yaklaş: seçili nesneyi ve altındakileri görünüme sığdırır.",
                when(chosen, Message::Focus),
            ),
        ];
        if !shown {
            return out;
        }
        out.extend([
            strip::divider(),
            strip::labelled(Icon::EyeOff, "Gizle", "Gizle: seçili nesneyi ve altındakileri gösterimden çıkarır.", when(chosen, Message::Hide)),
            strip::labelled(
                Icon::Filter,
                "Yalnız bırak",
                "Yalnız bırak: yalnız seçili nesneyi ve altındakileri gösterir.",
                when(chosen, Message::Isolate),
            ),
            strip::labelled(Icon::Eye, "Hepsini göster", "Hepsini göster: gizlenenleri geri getirir, yalnız bırakmayı kaldırır.", when(partial, Message::ShowAll)),
            strip::divider(),
        ]);
        let v = self.view();
        out.push(strip::toggle(
            Some(Icon::Polygon),
            "Mahaller",
            "Mahaller: mahallerin saydam hacimlerini gösterir.",
            v.is_some_and(|v| v.spaces),
            Message::Spaces,
        ));
        out.push(strip::toggle(
            Some(Icon::Rectangle),
            "Açıklıklar",
            "Açıklıklar: duvarlardan ve döşemelerden kesilen boşlukları saydam hacimler olarak gösterir.",
            v.is_some_and(|v| v.openings),
            Message::Openings,
        ));
        out.push(strip::toggle(
            Some(Icon::Contrast),
            "Saydam",
            "Saydam: bütün modeli saydam gösterir; seçili ve bulgulu nesneler dolu kalır.",
            self.see_through,
            Message::SeeThrough,
        ));
        out.push(strip::divider());
        let section = self.section;
        out.push(strip::menu(
            Icon::Layers,
            "Kesit",
            "Kesit: bir düzlemle ya da seçimin çevresindeki kutuyla modelin içine bakar. Kesilen yüzler kapatılmaz.",
            move || section_menu(section, chosen),
        ));
        out
    }

    /// While something cuts the model: the section's card over the scene's top left, its name and ×, and for a plane
    /// its axis, its place (a slider and the value with its unit) and Öbür yanı.
    fn section_card<'a, M>(&'a self, host: &Host<'a, M>) -> Option<Element<'a, Message>> {
        let (title, place): (&str, Option<Element<'a, Message>>) = match self.section {
            Section::None => return None,
            Section::Box(_) => ("Seçimin çevresinde kutu", None),
            Section::Plane { cut, at, flip } => {
                let (lo, hi) = self.span(cut)?;
                let (title, axis) = match cut {
                    Cut::Level => ("Yatay kesit", "Kot"),
                    Cut::NorthSouth => ("Kuzey–güney düşey kesiti", host.east),
                    Cut::EastWest => ("Doğu–batı düşey kesiti", host.north),
                };
                let place = column![
                    row![
                        label::body(axis).style(muted),
                        slider(lo..=hi, at.clamp(lo, hi), Message::At)
                            .step((hi - lo) / 1000.0)
                            .style(kentos_ui::style::field::slider)
                            .width(Fill),
                        label::mono(format!("{} m", (host.coordinate)(at))),
                    ]
                    .spacing(10)
                    .align_y(Center),
                    strip::toggle(None, "Öbür yanı", "Öbür yanı: düzlemin öbür yanını gösterir.", flip, Message::Flip),
                ]
                .spacing(6);
                (title, Some(place.into()))
            }
        };
        let head = row![
            label::strong(title),
            space::horizontal(),
            strip::icon_button(Icon::Close, "Kesiti kaldır: modeli bütün gösterir.", Some(Message::Uncut)),
        ]
        .align_y(Center);
        let mut body = column![head].spacing(6);
        if let Some(place) = place {
            body = body.push(place);
        }
        // The legend's frame: the scene's cards are one family.
        Some(container(kentos_ui::widget::legend::frame(container(body).padding([2, 4]))).width(typography::scaled(CARD_WIDTH)).into())
    }

    fn scene_pane<'a, M>(&'a self, host: &Host<'a, M>) -> Element<'a, Message> {
        let view = SceneView {
            scene: self.scene().clone(),
            camera: self.camera(),
            exaggeration: kentos_scene3d::Exaggeration::NONE,
            styles: self.styles(),
            light: self.orbit.light(),
            // A building's faces under a sky and with a fill (docs/adr/8120): a roof lighter than the wall under it,
            // two walls seen at a corner never the same.
            sky: SKY,
            fill: FILL,
            quality: host.quality,
            planes: self.planes(),
            marked: self.picked.as_ref().map(|p| p.point),
            seen: self.seen.clone(),
            pending: self.pending.clone(),
        };
        let swatches = self.view().map_or_else(Vec::new, |v| v.legend());
        look::framed(
            scene::view(view, Message::Gesture),
            look::compass(&self.orbit, Message::North),
            look::legend(&swatches, self.legend, Message::Legend),
            self.section_card(host),
            host.sky,
        )
    }

    fn tree_panel(&self) -> Element<'_, Message> {
        let not_shown = self.view().map_or(0, |v| v.input().not_shown.len());
        let tabs = Segmented::new_with(
            [Panel::Structure, Panel::Class, Panel::NotShown],
            self.panel,
            Message::Panel,
            move |p| p != Panel::NotShown || not_shown > 0,
        )
        .width(Fill);
        let search = InputField::new("Ad, sınıf ya da GlobalId ara", &self.query)
            .on_input(Message::Search)
            .icon(Icon::Search)
            .clear(Message::Search(String::new()));
        let list: Element<'_, Message> = match self.panel {
            Panel::NotShown => self.not_shown_list(),
            _ if !self.query.trim().is_empty() => self.found_list(),
            _ => {
                let rows = self.rows();
                TreeView::new([tree_view::Column::new("Nesne").width(Fill), tree_view::Column::new("Öğe").width(36).align_right()])
                    .header(false)
                    .virtualized(rows.len(), move |i| {
                        let r = &rows[i];
                        // Every row has its count's cell (empty when it holds nothing): the eyes stay in one column.
                        let count = if r.count > 0 { r.count.to_string() } else { String::new() };
                        let mut node = Node::new(r.label.as_str()).selected(r.chosen).muted(!r.shows).cells([label::caption(count).into()]);
                        if r.count > 0 {
                            node = node.expanded(r.open, Message::Open(r.id.clone()));
                        }
                        node = match r.object {
                            Some(o) => node.on_press(Message::Choose(Some(o))).toggle(Toggle::visible(r.shows, Message::Eye(o))),
                            None => node.on_press(Message::Open(r.id.clone())),
                        };
                        (r.depth, node)
                    })
                    .height(Fill)
                    .empty("Modelde nesne yok.")
                    .into()
            }
        };
        let head = column![tabs, search].spacing(6);
        column![container(head).padding(6), list].height(Fill).into()
    }

    /// The objects the search finds, each with the storey it is on (its long name, as the tree writes it after “—”),
    /// else its class; pressed, it is chosen.
    fn found_list(&self) -> Element<'_, Message> {
        let Some(v) = self.view() else { return space().into() };
        let found = self.found();
        let chosen = v.selected();
        TreeView::new([tree_view::Column::new("Nesne").width(Fill), tree_view::Column::new("Kat").width(Fill)])
            .header(false)
            .virtualized(found.len(), move |i| {
                let o = found[i];
                let objects = &v.input().objects;
                let object = &objects[o];
                let where_ = match object.storey.and_then(|s| objects.get(s)) {
                    Some(s) => s.label.rsplit_once(" — ").map_or(s.label.as_str(), |(_, long)| long),
                    None => object.class.as_str(),
                };
                let node = Node::new(object.label.as_str())
                    .selected(chosen == Some(o))
                    .muted(!v.shows(o))
                    .cells([label::caption(where_).into()])
                    .on_press(Message::Choose(Some(o)));
                (0, node)
            })
            .height(Fill)
            .empty("Eşleşen nesne yok: başka bir ad, sınıf ya da GlobalId yazın.")
            .into()
    }

    /// The objects with a shape that could not be shown, each with why.
    fn not_shown_list(&self) -> Element<'_, Message> {
        let Some(v) = self.view() else { return space().into() };
        let items = &v.input().not_shown;
        TreeView::new([tree_view::Column::new("Nesne").width(Fill), tree_view::Column::new("Neden").width(Fill)])
            .virtualized(items.len(), move |i| {
                let (o, why) = &items[i];
                let label = v.input().objects.get(*o).map_or("", |o| o.label.as_str());
                (0, Node::new(label).cells([label::caption(why.as_str()).into()]).on_press(Message::Choose(Some(*o))))
            })
            .height(Fill)
            .into()
    }

    /// The chosen object's details, a group a folding section; else what a choice gives.
    fn details_panel<'a, M>(&'a self, host: &Host<'a, M>) -> Element<'a, Message> {
        let head = container(label::strong("Özellikler")).padding([8, 10]);
        let Some(d) = self.details() else {
            let what = if self.view().and_then(|v| v.selected()).is_some() { "Okunuyor…" } else { "Bir nesne seçin" };
            return column![
                head,
                container(
                    EmptyState::new(Icon::Properties, what)
                        .description("Modelde ya da ağaçta seçilen nesnenin öznitelikleri, özellik ve miktar setleri dosyada yazdığı gibi burada okunur.")
                )
                .center(Fill)
            ]
            .into();
        };
        let mut sheet = PropertySheet::new();
        for g in &d.groups {
            sheet = sheet.section(g.title.as_str(), !self.is_folded(&g.title), Message::Fold(g.title.clone()));
            for l in &g.lines {
                let cell = match l.value.trim().parse::<f64>().ok().filter(|_| l.numeric) {
                    Some(n) => property_grid::measured((host.measure)(n, l.unit.as_deref()), &l.value, l.unit.clone(), IN_FILE),
                    None => property_grid::value(l.value.as_str(), l.numeric, l.unit.clone()),
                };
                sheet = sheet.row(l.name.as_str(), cell);
            }
        }
        column![head, scrollable(sheet).height(Fill)].into()
    }

    fn status<'a, M>(&'a self, host: &Host<'a, M>) -> Element<'a, Message> {
        let mut bar = StatusBar::new();
        bar = match &self.picked {
            Some(p) => {
                let at = format!(
                    "{} {}   {} {}   Z {}",
                    host.east,
                    (host.coordinate)(p.point[0]),
                    host.north,
                    (host.coordinate)(p.point[1]),
                    (host.coordinate)(p.point[2])
                );
                bar.push(
                    Readout::new(row![label::body(p.what.clone()), label::mono(at)].spacing(12).align_y(Center))
                        .icon(Icon::Crosshair)
                        .tip(Tip::new("Seçilen nokta").body("Tıklanan noktanın dosyadaki koordinatları; görüntüleyici koordinat dönüştürmez.")),
                )
            }
            None if self.stage == Stage::Shown => {
                bar.push(Readout::new(label::body("Bir nesneye tıklayın: seçilir, özellikleri sağda okunur.").style(muted)).icon(Icon::Crosshair))
            }
            None => bar.push(Readout::new(label::body(self.title.as_str())).icon(Icon::Document)),
        };
        bar = bar.spacer();
        if let Some(v) = self.view() {
            let (shown, all) = v.counts();
            bar = bar.push(Readout::new(label::body(format!("{shown}/{all} gövde"))).tip(Tip::new("Gövdeler").body(format!(
                "Gösterilen ve bütün gövdeler; sahne {} üçgen, {} karodur. Üçgen ağdan ölçü alınmaz: miktarlar dosyanın setlerindedir.",
                self.scene().triangles(),
                self.scene().contents()
            ))));
            let missing = v.input().not_shown.len();
            if missing > 0 {
                bar = bar.separator().push(
                    Readout::new(label::body(format!("{missing} gösterilemeyen")).style(|t: &Theme| text::Style { color: Some(Tokens::of(t).warning) }))
                        .icon(Icon::Warning)
                        .tip(Tip::new("Gösterilemeyen nesneler").body("Şekli olup gövdesi çıkarılamayan nesneler; nedenleri soldaki Gösterilemeyen sekmesinde.")),
                );
            }
        }
        for (words, tip) in &self.facts {
            bar = bar.separator().push(Readout::new(label::body(words.as_str())).tip(Tip::new(words.as_str()).body(tip.as_str())));
        }
        bar.separator()
            .push(Readout::new(label::body("Fare")).icon(Icon::Help).tip(Tip::new("Fareyle gezinme").body(
                "Sürükleyin: döndürür. Sağ tuşla ya da Shift ile sürükleyin: kaydırır. Tekerlek: yaklaştırır, uzaklaştırır. Tıklayın: nesneyi seçer.",
            )))
            .into()
    }
}

/// The reading's progress: its phase, how far, and İptal.
fn reading<'a>(phase: &'a str, done: usize, total: usize) -> Element<'a, Message> {
    let fraction = (total > 0).then(|| done as f32 / total as f32);
    let words = if total > 0 { format!("{phase}: {done} / {total}") } else { phase.to_owned() };
    container(
        column![
            label::body(words),
            container(progress::bar(fraction)).width(typography::scaled(320.0)),
            button(label::body("İptal")).style(kentos_ui::style::button::secondary).on_press(Message::Cancel),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .center(Fill)
    .into()
}

/// The section's menu: the three planes, the box about the chosen object, and none.
fn section_menu(section: Section, chosen: bool) -> Menu<Message> {
    let plane = |c: Cut| matches!(section, Section::Plane { cut, .. } if cut == c);
    let mut m = Menu::new()
        .radio("Yatay kesit", plane(Cut::Level), Message::Cut(Cut::Level))
        .radio("Kuzey–güney düşey kesiti", plane(Cut::NorthSouth), Message::Cut(Cut::NorthSouth))
        .radio("Doğu–batı düşey kesiti", plane(Cut::EastWest), Message::Cut(Cut::EastWest));
    if chosen {
        m = m.radio("Seçimin çevresinde kutu", matches!(section, Section::Box(_)), Message::Boxed);
    }
    m.separator().item("Kesiti kaldır", Message::Uncut)
}

fn muted(t: &Theme) -> text::Style {
    text::Style { color: Some(Tokens::of(t).muted) }
}
