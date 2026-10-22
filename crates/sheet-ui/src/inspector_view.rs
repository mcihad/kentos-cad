//! The inspector on the right (design §11): Öğe (position and size, the
//! constraints, the kind's own properties, the look, the data bindings),
//! Sayfa (paper, orientation, margins, layout variant, snap grid) and Ön
//! denetim (the preflight's findings with their fixes). Several items chosen:
//! what they share is edited together; a value they do not share is “—” and
//! a value typed is given to all.

use iced::widget::{button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill};
use kentos_sheet::model::{Item, Orientation, Paper};
use kentos_sheet::preflight::Severity;
use kentos_sheet::profile;
use kentos_sheet::units::mm_text;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::number::{Unit, units};
use kentos_ui::widget::property_grid::{choice, value};
use kentos_ui::widget::{
    EditCell, EmptyState, Inspector, Menu, NumberInput, PropertySheet, Segmented, Switch, Tip, tip,
};

use crate::designer::Designer;
use crate::message::{FrameField, InspectorTab, Message, Side};
use crate::panels::kind_icon;

pub const WIDTH: f32 = 316.0;

/// Millimetres, the field's unit; centimetres and metres may be typed.
const MM: &[Unit] = &[
    Unit::new("mm", 1.0),
    Unit::new("cm", 10.0),
    Unit::new("m", 1000.0),
];

const PAPERS: [Paper; 11] = Paper::STANDARD;

fn paper_name(p: Paper) -> &'static str {
    match p {
        Paper::A0 => "A0 (841 × 1189)",
        Paper::A1 => "A1 (594 × 841)",
        Paper::A2 => "A2 (420 × 594)",
        Paper::A3 => "A3 (297 × 420)",
        Paper::A4 => "A4 (210 × 297)",
        Paper::A5 => "A5 (148 × 210)",
        Paper::B0 => "B0 (1000 × 1414)",
        Paper::B1 => "B1 (707 × 1000)",
        Paper::B2 => "B2 (500 × 707)",
        Paper::B3 => "B3 (353 × 500)",
        Paper::B4 => "B4 (250 × 353)",
        Paper::Custom => "Özel",
    }
}

/// The value every item has, or none.
fn common<T: PartialEq>(items: &[&Item], of: impl Fn(&Item) -> T) -> Option<T> {
    let first = of(items.first()?);
    items.iter().all(|i| of(i) == first).then_some(first)
}

/// A number of the chosen items: an input while they share it, else “—” to type over.
fn number<'a>(
    v: Option<f64>,
    field: FrameField,
    units_: &'a [Unit],
    decimals: usize,
    enabled: bool,
) -> Element<'a, Message> {
    match (v, enabled) {
        (Some(v), true) => dragged(
            NumberInput::new(v, move |x| Message::Frame(field, x))
                .units(units_)
                .decimals(decimals)
                .step(if decimals == 0 { 1.0 } else { 0.5 })
                .inline(),
        )
        .into(),
        (None, true) => EditCell::new("—", move |t| Message::FrameText(field, t))
            .numeric(true)
            .unit(units_.first().map_or("", |u| u.symbol))
            .into(),
        (v, false) => value(
            v.map_or("—".to_owned(), |x| {
                kentos_geometry_core::display::fixed(x, decimals)
            }),
            true,
            units_.first().map(|u| u.symbol.to_owned()),
        ),
    }
}

/// A number of the inspector: written by the display rule (ADR 0149: a point,
/// as the web's), its label's drag one undo step from start to release.
fn dragged<'a>(n: NumberInput<'a, Message>) -> NumberInput<'a, Message> {
    n.point()
        .on_drag(Message::GestureStart)
        .on_release(Message::GestureEnd)
}

impl Designer {
    /// The inspector: its tabs and the open one.
    pub fn inspector_panel(&self) -> Element<'_, Message> {
        let errors = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        let tabs = Segmented::new(
            [
                InspectorTab::Item,
                InspectorTab::Page,
                InspectorTab::Preflight,
            ],
            self.inspector_tab,
            Message::InspectorTab,
        )
        .compact()
        .width(Fill);
        let body = match self.inspector_tab {
            InspectorTab::Item => self.item_tab(),
            InspectorTab::Page => self.page_tab(),
            InspectorTab::Preflight => self.preflight_tab(),
        };
        let mut head = column![container(tabs).padding([8, 8])];
        if errors > 0 && self.inspector_tab != InspectorTab::Preflight {
            head = head.push(
                container(
                    button(
                        row![
                            icon(Icon::Error).size(14.0),
                            label::caption(format!("Ön denetimde {errors} hata"))
                        ]
                        .spacing(6)
                        .align_y(Center),
                    )
                    .on_press(Message::InspectorTab(InspectorTab::Preflight))
                    .padding([3, 8])
                    .style(style::button::ghost),
                )
                .padding([0, 8]),
            );
        }
        column![
            head,
            scrollable(container(body).padding(iced::Padding::ZERO.bottom(12))).height(Fill)
        ]
        .width(WIDTH)
        .height(Fill)
        .into()
    }

    fn open_section(&self, key: &'static str) -> bool {
        !self.closed_sections.contains(key)
    }

    fn item_tab(&self) -> Element<'_, Message> {
        let chosen = self.chosen();
        if chosen.is_empty() {
            return container(
                EmptyState::new(icons_select(), "Seçili öğe yok")
                    .description("Paftada bir öğeye tıklayın ya da Öğeler listesinden seçin. Shift ile ekler, Ctrl ile çıkarırsınız; birden çok öğede ortak değerler birlikte düzenlenir."),
            )
            .padding(16)
            .into();
        }
        let names = &self.profile.names;
        let kind_name = |i: &Item| {
            names
                .get(profile::tool_of(i))
                .or_else(|| names.get(i.kind.type_name()))
                .cloned()
                .unwrap_or_else(|| i.kind.label().to_owned())
        };
        // The head: one item's kind and name, or how many of which kinds.
        let (glyph, title, sub) = if let [one] = chosen.as_slice() {
            let k = kind_name(one);
            let sub = if one.name == k {
                String::new()
            } else {
                one.name.clone()
            };
            (kind_icon(one), k, sub)
        } else {
            let mut kinds: Vec<(String, usize)> = Vec::new();
            for i in &chosen {
                let k = kind_name(i);
                match kinds.iter_mut().find(|(x, _)| *x == k) {
                    Some((_, n)) => *n += 1,
                    None => kinds.push((k, 1)),
                }
            }
            let sub = kinds
                .iter()
                .map(|(k, n)| format!("{n} {}", crate::text::fold(k)))
                .collect::<Vec<_>>()
                .join(", ");
            (
                crate::icons::GROUP,
                format!("{} öğe seçili", chosen.len()),
                sub,
            )
        };
        let mut head = column![
            row![
                icon(glyph).size(20.0),
                column![
                    label::strong(title),
                    label::caption(sub).style(style::text::muted)
                ]
            ]
            .spacing(10)
            .align_y(Center)
        ]
        .spacing(6)
        .padding([4, 12]);
        let mut notes: Vec<String> = chosen
            .iter()
            .filter_map(|i| profile::item_note(self.ctx.workspace, i))
            .collect();
        notes.dedup();
        for n in notes {
            head = head.push(
                container(label::caption(n))
                    .padding(6)
                    .style(style::container::bordered),
            );
        }
        if chosen.iter().any(|i| i.locked) {
            head = head.push(
                label::caption("Kilitli öğe taşınmaz ve boyutlanmaz; kilidi öğe ağacından açın.")
                    .style(style::text::muted),
            );
        }

        let editable = chosen.iter().all(|i| !i.locked);
        let f = |of: fn(&Item) -> i64| common(&chosen, of).map(|v| v as f64 / 1000.0);
        let mut sheet = PropertySheet::new().section(
            "Konum ve boyut",
            self.open_section("frame"),
            Message::Section("frame"),
        );
        sheet = sheet
            .row(
                "Sol",
                number(
                    f(|i| i64::from(i.frame.left)),
                    FrameField::Left,
                    MM,
                    1,
                    editable,
                ),
            )
            .row(
                "Üst",
                number(
                    f(|i| i64::from(i.frame.top)),
                    FrameField::Top,
                    MM,
                    1,
                    editable,
                ),
            )
            .row(
                "Genişlik",
                number(
                    f(|i| i64::from(i.frame.width)),
                    FrameField::Width,
                    MM,
                    1,
                    editable,
                ),
            )
            .row(
                "Yükseklik",
                number(
                    f(|i| i64::from(i.frame.height)),
                    FrameField::Height,
                    MM,
                    1,
                    editable,
                ),
            )
            .row(
                "Dönüş",
                number(
                    f(|i| i64::from(i.rotation)),
                    FrameField::Rotation,
                    units::ANGLE,
                    1,
                    editable,
                ),
            );

        let mut col = column![head, sheet];
        // The constraints.
        col = col.push(PropertySheet::new().section(
            "Kısıtlar",
            self.open_section("constraints"),
            Message::Section("constraints"),
        ));
        if self.open_section("constraints") {
            let in_group = chosen.iter().all(|i| i.group.is_some());
            col = col.push(
                container(crate::constraint::editor(
                    &self.chosen_constraints,
                    in_group,
                    editable,
                ))
                .padding([8, 12]),
            );
        }
        if chosen.len() == 1
            && let Some(origin) = &chosen[0].cad_origin
        {
            let source = format!(
                "Kaynak: {} · {} · {}. nesne",
                origin.file.as_deref().unwrap_or("CAD çizimi"),
                origin.layout,
                origin.record.saturating_add(1)
            );
            col = col.push(container(label::caption(source)).padding([8, 12]));
        }
        // The kind's own properties.
        if !self.kind_fields.is_empty() {
            let kind = kind_name(chosen[0]);
            col = col.push(PropertySheet::new().section(
                kind,
                self.open_section("kind"),
                Message::Section("kind"),
            ));
            if self.open_section("kind") {
                if let Some(row) = self.map_row(&chosen, editable) {
                    col = col.push(row);
                }
                // A north arrow's declination part stands after “Biçim”, the value typed by hand
                // and its year after it with their hint, as the web's does.
                let north = self.north_part(&chosen, editable);
                let len = self.kind_fields.len();
                let after_style = self
                    .kind_fields
                    .iter()
                    .position(|kf| kf.prop.path == ["kind", "style"])
                    .map_or(len, |i| i + 1);
                let typed_end = (after_style..len)
                    .take_while(|&i| {
                        matches!(
                            self.kind_fields[i].prop.path,
                            ["kind", "declination"] | ["kind", "declinationYear"]
                        )
                    })
                    .last()
                    .map_or(after_style, |i| i + 1);
                let fields = |range: std::ops::Range<usize>| {
                    let mut insp = Inspector::new(&self.inspector, Message::Inspector)
                        .toolbar(false)
                        .help(false);
                    for (id, kf) in self.kind_fields.iter().enumerate() {
                        if range.contains(&id) {
                            insp = insp.field(id, &kf.field, &kf.value);
                            if kf.varies {
                                insp = insp.varies(id);
                            }
                        }
                    }
                    container(insp).padding([2, 4])
                };
                if let Some(part) = self.coordinate_part(&chosen, editable) {
                    col = col.push(part);
                }
                match north {
                    Some(part) => {
                        col = col.push(fields(0..after_style)).push(part);
                        if typed_end > after_style {
                            col = col.push(fields(after_style..typed_end)).push(
                                container(
                                    label::caption(
                                        "Doğu artı, batı eksi; yıl kâğıtta “elle, 2024” diye yazılır.",
                                    )
                                    .style(style::text::muted),
                                )
                                .padding([0, 16])
                                .width(Fill),
                            );
                        }
                        if typed_end < len {
                            col = col.push(fields(typed_end..len));
                        }
                    }
                    None => col = col.push(fields(0..len)),
                }
                if let Some(row) = self.picture_row(&chosen, editable) {
                    col = col.push(row);
                }
                if let Some(part) = self.map_layers_part(&chosen, editable) {
                    col = col.push(part);
                }
            }
        }
        // The look.
        let fill = common(&chosen, |i| i.fill.clone());
        let fill_text = match &fill {
            Some(Some(c)) => c.clone(),
            Some(None) => "Yok".to_owned(),
            None => "—".to_owned(),
        };
        let swatch = fill.clone().flatten().and_then(|c| crate::paint::color(&c));
        let fills: [(&str, Option<&str>); 6] = [
            ("Yok", None),
            ("Beyaz", Some("#ffffff")),
            ("Açık gri", Some("#f1f3f5")),
            ("Kâğıt", Some("#fbf7ee")),
            ("Açık mavi", Some("#e7f0fb")),
            ("Siyah", Some("#000000")),
        ];
        let current = fill.clone();
        let look = PropertySheet::new()
            .section(
                "Görünüş",
                self.open_section("look"),
                Message::Section("look"),
            )
            .row(
                "Dolgu",
                choice(fill_text, swatch, move || {
                    fills.iter().fold(Menu::new(), |m, (name, c)| {
                        m.radio(
                            *name,
                            current.as_ref().is_some_and(|x| x.as_deref() == *c),
                            Message::Fill(c.map(str::to_owned)),
                        )
                    })
                }),
            )
            .row(
                "Saydamsızlık",
                match common(&chosen, |i| i.opacity) {
                    Some(o) => dragged(
                        NumberInput::new(f64::from(o), Message::Opacity)
                            .units(units::PERCENT)
                            .range(0.0..=100.0)
                            .decimals(0)
                            .inline(),
                    )
                    .into(),
                    None => value("—", true, Some("%".into())),
                },
            )
            .row(
                "İç boşluk",
                match common(&chosen, |i| i.padding) {
                    Some(p) => dragged(
                        NumberInput::new(f64::from(p) / 1000.0, Message::Padding)
                            .units(MM)
                            .range(0.0..=100.0)
                            .decimals(1)
                            .step(0.5)
                            .inline(),
                    )
                    .into(),
                    None => value("—", true, Some("mm".into())),
                },
            )
            .row(
                "Yazdırılır",
                Switch::new(
                    common(&chosen, |i| i.printable).unwrap_or(true),
                    Message::Printable,
                ),
            );
        col = col.push(look);
        // The data bindings (design §7): every property an expression may give, ƒ to bind or edit it.
        let mut data = PropertySheet::new().section(
            "Veri (ƒ)",
            self.open_section("data"),
            Message::Section("data"),
        );
        if let [one] = chosen.as_slice() {
            for p in crate::binding::bindables_of(&one.kind) {
                let bound = one.bindings.iter().find(|b| b.property == p.property);
                let open = Message::Binding(crate::binding::BindingMessage::Open {
                    item: one.id.clone(),
                    property: p.property.clone(),
                });
                let shown = match bound {
                    Some(b) => {
                        label::caption(format!("ƒ {}", b.expression)).style(style::text::accent)
                    }
                    None => label::caption("—").style(style::text::muted),
                };
                let cell = button(
                    row![
                        container(shown).width(Fill).clip(true),
                        label::caption("ƒ").style(if bound.is_some() {
                            style::text::accent
                        } else {
                            style::text::muted
                        })
                    ]
                    .spacing(4)
                    .align_y(Center),
                )
                .on_press_maybe(editable.then_some(open))
                .width(Fill)
                .padding([2, 6])
                .style(style::button::ghost);
                let title = if p.unit.is_empty() {
                    p.label.clone()
                } else {
                    format!("{} ({})", p.label, p.unit)
                };
                data = data.row(title, cell);
            }
        } else {
            data = data.row("Bağ", value("Bir öğe seçin", false, None));
        }
        col.push(data).into()
    }

    /// A north arrow's declination (design §8a; the web's `northSection` and its words): what the
    /// paper writes and from what (the core's `north_info`: the World Magnetic Model at the map's
    /// centre on the sheet's date), the date it is for and where that comes from with the way to
    /// change it; “Sapmayı elle gir”, and the value typed by hand with its year. Several arrows
    /// chosen: a line saying it is shown for one.
    fn north_part(&self, chosen: &[&Item], editable: bool) -> Option<Element<'_, Message>> {
        use crate::inspect::{by_hand, date_hint, declination_text, shows_magnetic, value_hint};
        use crate::message::NorthMessage;
        use kentos_sheet::kinds::ItemKind;
        let arrows: Vec<&kentos_sheet::kinds::NorthArrowItem> = chosen
            .iter()
            .filter_map(|i| match &i.kind {
                ItemKind::NorthArrow(k) => Some(k),
                _ => None,
            })
            .collect();
        if arrows.is_empty() || arrows.len() != chosen.len() {
            return None;
        }
        if let [k] = arrows.as_slice() {
            if !shows_magnetic(k) {
                return None;
            }
            let hand = by_hand(k);
            let info = self
                .north_info
                .as_ref()
                .filter(|(id, _)| *id == chosen[0].id)
                .and_then(|(_, i)| i.as_ref());
            let mut part = column![].spacing(4).padding([6, 4]);
            match info {
                None => part = part.push(hint("Manyetik sapma şimdi okunamadı.".into())),
                Some(info) => {
                    let known = info.declination.is_some() && info.missing.is_none();
                    let value = label::body(declination_text(info));
                    part = part.push(labelled(
                        "Manyetik sapma",
                        if known {
                            value.into()
                        } else {
                            value.style(crate::gallery::warning).into()
                        },
                        value_hint(info, hand),
                    ));
                    if !hand {
                        let variables = tip(
                            button(
                                row![
                                    icon(crate::icons::VARIABLES).size(14.0),
                                    label::caption("Değişkenler…")
                                ]
                                .spacing(4)
                                .align_y(Center),
                            )
                            .on_press_maybe(editable.then_some(Message::Variables(
                                crate::variables::VariablesMessage::Open,
                            )))
                            .padding([3, 8])
                            .style(style::button::secondary),
                            Tip::new("Tarihi değiştir").body(
                                "Paftaya ya da projeye ISO biçiminde bir “tarih” değişkeni girin (2026-10-03); sapma o günün değeriyle yazılır.",
                            ),
                            iced::widget::tooltip::Position::Bottom,
                        );
                        part = part.push(labelled(
                            "Hesap tarihi",
                            row![
                                label::body(info.date.clone().unwrap_or_else(|| "—".into()))
                                    .font(kentos_ui::theme::typography::mono())
                                    .width(Fill),
                                variables,
                            ]
                            .spacing(8)
                            .align_y(Center)
                            .into(),
                            date_hint(info),
                        ));
                    }
                }
            }
            let switch = if editable {
                Switch::new(hand, |on| Message::North(NorthMessage::Hand(on)))
            } else {
                Switch::disabled(hand)
            };
            part = part.push(PropertySheet::new().row("Sapmayı elle gir", switch));
            part = part.push(hint(
                "Kapalıyken sapma modelden hesaplanır; açıkken buraya girilen değer kullanılır ve kâğıtta “elle” yazılır.".into(),
            ));
            return Some(part.into());
        }
        // Several arrows: the declination is shown for one.
        arrows
            .iter()
            .all(|k| shows_magnetic(k))
            .then(|| hint("Manyetik sapma tek kuzey oku seçiliyken gösterilir.".into()))
    }

    /// Picture frames: the picture they show and “Resim seç…” (the web's picture section).
    /// A coordinate list's source and names (docs/adr/0206 §2, the web's `coordinateSection`):
    /// Noktalar (the objects taken, a layer, or the drawing's choice now), what it gives in the
    /// core's words, Seçimi al and Çizimde göster, Nokta adları. One list chosen.
    fn coordinate_part(&self, chosen: &[&Item], editable: bool) -> Option<Element<'_, Message>> {
        use kentos_sheet::kinds::{CoordSource, ItemKind, PointNaming};
        let [one] = chosen else { return None };
        let ItemKind::CoordinateList(c) = &one.kind else {
            return None;
        };
        let layers = self.ctx.layers.clone();
        // The choices: the objects taken, each layer, the drawing's choice now.
        let mut labels = vec!["Seçilen nesneler".to_owned()];
        labels.extend(layers.iter().map(|(_, name)| format!("Katman: {name}")));
        labels.push("Çizimde şu an seçili olanlar (canlı)".to_owned());
        let live = labels.len() - 1;
        let at = match &c.source {
            CoordSource::Objects(_) => Some(0),
            CoordSource::Layer(l) => layers
                .iter()
                .position(|(id, _)| *id == l.layer)
                .map(|i| i + 1),
            CoordSource::Selection(_) => Some(live),
        };
        let pick = {
            let layers = layers.clone();
            move |i: usize| {
                if i == 0 {
                    Message::CoordTakeSelection
                } else if i == live {
                    Message::CoordLive
                } else {
                    Message::CoordLayer(
                        layers
                            .get(i - 1)
                            .map(|(id, _)| id.clone())
                            .unwrap_or_default(),
                    )
                }
            }
        };
        let source = kentos_ui::widget::select::Select::new(
            labels
                .into_iter()
                .map(kentos_ui::widget::select::Choice::new),
            at,
            pick,
        )
        .searchable(false);
        let input = self.data.coordinates.iter().find(|x| x.item == one.id);
        let layer = match &c.source {
            CoordSource::Layer(l) => layers
                .iter()
                .find(|(id, _)| *id == l.layer)
                .map(|(_, n)| n.as_str()),
            _ => None,
        };
        let said = kentos_sheet::display::coordinate_summary(&c.source, input, layer);
        let objects = input.and_then(|i| i.objects).unwrap_or(0);
        let action = |text: &'static str, glyph: Icon, on: Option<Message>| {
            button(
                // The caption's size in the text's own colour: a faint caption reads as off.
                row![
                    icon(glyph).size(14.0),
                    iced::widget::text(text).size(kentos_ui::theme::typography::caption())
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press_maybe(on)
            .padding([3, 8])
            .style(style::button::secondary)
        };
        let numbered = matches!(c.naming, PointNaming::Sequence(_));
        let naming = kentos_ui::widget::select::Select::new(
            ["Kendi adları", "Sıra numarası"].map(kentos_ui::widget::select::Choice::new),
            Some(usize::from(numbered)),
            |i| Message::CoordNumbered(i == 1),
        )
        .searchable(false);
        Some(
            column![
                labelled("Noktalar", source.into(), said),
                container(
                    row![
                        action(
                            "Seçimi al",
                            crate::icons::SELECT,
                            editable.then_some(Message::CoordTakeSelection),
                        ),
                        action(
                            "Çizimde göster",
                            crate::icons::ZOOM_SELECTION,
                            (objects > 0).then_some(Message::CoordShow),
                        ),
                    ]
                    .spacing(6)
                    .wrap()
                )
                .padding([0, 12]),
                labelled("Nokta adları", naming.into(), String::new()),
            ]
            .spacing(6)
            .padding([4, 4])
            .into(),
        )
    }

    /// One map's own layers (docs/adr/0314 §1): Katman durumu (a layer state of the project in this map only, or the
    /// drawing's), and a card per overridden layer: Dondur, Yazdır, Renk, Kalınlık, Çizgi tipi, Kaldır; Katman ekle
    /// lists the layers not overridden yet. The drawing's layers stay as they are.
    fn map_layers_part(&self, chosen: &[&Item], editable: bool) -> Option<Element<'_, Message>> {
        use crate::message::OverrideEdit;
        use kentos_sheet::kinds::{ItemKind, MapLayers};
        use kentos_ui::widget::select::{Choice, Select};
        let [one] = chosen else { return None };
        let ItemKind::Map(m) = &one.kind else {
            return None;
        };
        let states = self.ctx.layer_states.clone();
        let mut labels = vec!["Çizimdeki gibi".to_owned()];
        labels.extend(states.iter().cloned());
        let at = match &m.layers {
            MapLayers::Theme(t) => match states.iter().position(|s| s.trim() == t.name.trim()) {
                Some(i) => Some(i + 1),
                None => {
                    labels.push(format!("{} (projede yok)", t.name));
                    Some(labels.len() - 1)
                }
            },
            _ => Some(0),
        };
        let kept = states.clone();
        let state = Select::new(labels.into_iter().map(Choice::new), at, move |i| {
            Message::MapLayerState(i.checked_sub(1).and_then(|i| kept.get(i).cloned()))
        })
        .searchable(false);
        let said = match &m.layers {
            MapLayers::Theme(_) => "Durumun görünürlüğü, dondurması, yazdırması ve sakladığı görünüşler yalnız bu haritada.".to_owned(),
            MapLayers::List(_) => "Harita seçilen katmanları gösterir.".to_owned(),
            MapLayers::All(_) => "Çizimin katmanları olduğu gibi.".to_owned(),
        };
        let mut col = column![labelled("Katman durumu", state.into(), said)].spacing(6);
        let name_of = |id: &str| {
            self.ctx
                .layers
                .iter()
                .find(|(l, _)| l == id)
                .map_or_else(|| format!("{id} (çizimde yok)"), |(_, n)| n.clone())
        };
        let small = |s: Select<'static, Message>| -> Element<'static, Message> {
            s.searchable(false).into()
        };
        const COLORS: [(&str, Option<&str>); 7] = [
            ("Renk: katmana göre", None),
            ("Siyah", Some("#000000")),
            ("Koyu gri", Some("#555555")),
            ("Kırmızı", Some("#E5484D")),
            ("Mavi", Some("#3E63DD")),
            ("Yeşil", Some("#30A46C")),
            ("Turuncu", Some("#F5A524")),
        ];
        const WEIGHTS: [(&str, Option<i32>); 8] = [
            ("Kalınlık: katmana göre", None),
            ("0,13 mm", Some(130)),
            ("0,18 mm", Some(180)),
            ("0,25 mm", Some(250)),
            ("0,35 mm", Some(350)),
            ("0,50 mm", Some(500)),
            ("0,70 mm", Some(700)),
            ("1,00 mm", Some(1000)),
        ];
        const BUILT_IN: [(&str, &str); 4] = [
            ("Sürekli", "continuous"),
            ("Kesikli", "dashed"),
            ("Noktalı kesik", "dashdot"),
            ("Noktalı", "dotted"),
        ];
        for (k, o) in m.layer_overrides.iter().enumerate() {
            let frozen = Switch::new(o.frozen, move |on| {
                Message::MapOverride(k, OverrideEdit::Frozen(on))
            })
            .label("Dondur");
            let plot_at = match o.plot {
                None => 0,
                Some(true) => 1,
                Some(false) => 2,
            };
            let plot = Select::new(
                ["Yazdır: katmana göre", "Yazdır", "Yazdırma"].map(Choice::new),
                Some(plot_at),
                move |i| {
                    Message::MapOverride(
                        k,
                        OverrideEdit::Plot([None, Some(true), Some(false)][i.min(2)]),
                    )
                },
            );
            let mut colors: Vec<(String, Option<String>)> = COLORS
                .iter()
                .map(|(n, c)| ((*n).to_owned(), c.map(str::to_owned)))
                .collect();
            if let Some(c) = &o.color
                && !colors.iter().any(|(_, x)| x.as_deref() == Some(c.as_str()))
            {
                colors.push((format!("Özel {c}"), Some(c.clone())));
            }
            let color_at = colors.iter().position(|(_, c)| *c == o.color);
            let picked = colors.clone();
            let color = Select::new(
                colors.into_iter().map(|(n, _)| Choice::new(n)),
                color_at,
                move |i| {
                    Message::MapOverride(
                        k,
                        OverrideEdit::Color(picked.get(i).and_then(|(_, c)| c.clone())),
                    )
                },
            );
            let weight_at = WEIGHTS.iter().position(|(_, w)| *w == o.line_weight);
            let weight = Select::new(
                WEIGHTS.iter().map(|(n, _)| Choice::new(*n)),
                weight_at,
                move |i| {
                    Message::MapOverride(
                        k,
                        OverrideEdit::Weight(WEIGHTS.get(i).and_then(|(_, w)| *w)),
                    )
                },
            );
            let mut lines: Vec<(String, Option<String>)> =
                vec![("Çizgi tipi: katmana göre".to_owned(), None)];
            lines.extend(
                BUILT_IN
                    .iter()
                    .map(|(n, v)| ((*n).to_owned(), Some((*v).to_owned()))),
            );
            lines.extend(
                self.ctx
                    .line_types
                    .iter()
                    .map(|p| (format!("Desen: {p}"), Some(format!("pattern:{p}")))),
            );
            let current = match (&o.line_pattern, &o.line_type) {
                (Some(p), _) => Some(format!("pattern:{p}")),
                (None, Some(t)) => Some(t.clone()),
                _ => None,
            };
            let line_at = lines.iter().position(|(_, v)| *v == current);
            let picked = lines.clone();
            let line = Select::new(
                lines.into_iter().map(|(n, _)| Choice::new(n)),
                line_at,
                move |i| {
                    Message::MapOverride(
                        k,
                        OverrideEdit::Line(picked.get(i).and_then(|(_, v)| v.clone())),
                    )
                },
            );
            let remove = button(icon(Icon::Close).size(12.0))
                .on_press_maybe(editable.then_some(Message::MapOverrideRemove(k)))
                .padding([2, 6])
                .style(style::button::ghost);
            let card = column![
                row![
                    container(
                        kentos_ui::widget::Elided::new(name_of(&o.layer))
                            .size(kentos_ui::theme::typography::body())
                    )
                    .width(Fill),
                    remove
                ]
                .align_y(Center),
                row![frozen, container(small(plot)).width(Fill)]
                    .spacing(8)
                    .align_y(Center),
                row![
                    container(small(color)).width(Fill),
                    container(small(weight)).width(Fill)
                ]
                .spacing(6),
                small(line),
            ]
            .spacing(4);
            col = col.push(
                container(card)
                    .padding([6, 8])
                    .width(Fill)
                    .style(style::container::bordered),
            );
        }
        let free: Vec<(String, String)> = self
            .ctx
            .layers
            .iter()
            .filter(|(id, _)| !m.layer_overrides.iter().any(|o| o.layer == *id))
            .cloned()
            .collect();
        if editable && !free.is_empty() {
            let names = free.clone();
            let add = Select::new(
                free.into_iter().map(|(_, n)| Choice::new(n)),
                None,
                move |i| {
                    Message::MapOverrideAdd(
                        names.get(i).map(|(id, _)| id.clone()).unwrap_or_default(),
                    )
                },
            )
            .placeholder("Katmanı bu haritada geçersiz kıl…");
            col = col.push(container(small(add)).padding([0, 12]));
        }
        Some(container(col).padding([4, 4]).into())
    }

    /// Fixed maps' scale and centre from the drawing (docs/adr/0206 §1, the web's `mapSection`):
    /// Çizim ölçeğini al (off when every map is at it), Görünüme sığdır and Görünümden al (off
    /// without a drawing area); the scale typed above stays.
    fn map_row(&self, chosen: &[&Item], editable: bool) -> Option<Element<'_, Message>> {
        use crate::inspect::fixed_map;
        if chosen.is_empty() || !chosen.iter().all(|i| fixed_map(i).is_some()) {
            return None;
        }
        let plot = self.ctx.capabilities.plot_scale.filter(|s| *s >= 1);
        let at_plot = plot.is_some_and(|p| {
            chosen
                .iter()
                .all(|i| fixed_map(i).is_some_and(|v| v.scale == p))
        });
        let has_view = self.ctx.center.is_some();
        let fit_view = self.ctx.center.is_some() && self.ctx.view_size.is_some();
        let action = |text: &'static str, glyph: Icon, on: Option<Message>| {
            button(
                // The caption's size in the text's own colour: a faint caption reads as off.
                row![
                    icon(glyph).size(14.0),
                    iced::widget::text(text).size(kentos_ui::theme::typography::caption())
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press_maybe(on.filter(|_| editable))
            .padding([3, 8])
            .style(style::button::secondary)
        };
        let scale = plot.map_or_else(String::new, |p| format!("Projenin çizim ölçeği 1/{p}. "));
        Some(
            column![
                row![
                    action(
                        "Çizim ölçeğini al",
                        crate::icons::PLOT_SCALE,
                        (plot.is_some() && !at_plot).then_some(Message::MapScaleFromDrawing),
                    ),
                    action(
                        "Çizime sığdır", crate::icons::ZOOM_EXTENTS,
                        self.ctx.drawing_bounds.is_some().then_some(Message::MapFitDrawing),
                    ),
                    action(
                        "Seçime sığdır", crate::icons::TARGET,
                        self.ctx.selection_bounds.is_some().then_some(Message::MapFitSelection),
                    ),
                    action(
                        "Görünüme sığdır",
                        crate::icons::ZOOM_EXTENTS,
                        fit_view.then_some(Message::MapFitView),
                    ),
                    action(
                        "Görünümden al",
                        crate::icons::TARGET,
                        has_view.then_some(Message::MapCentreFromView),
                    ),
                ]
                .spacing(6)
                .wrap(),
                label::caption(format!(
                    "{scale}Çizime/Seçime sığdır: nesneleri %5 kenar payıyla yerleştirir. Görünüme sığdır, çizim alanında görüneni standart ölçekte gösterir."
                ))
                .style(style::text::muted),
            ]
            .spacing(6)
            .padding([4, 16])
            .into(),
        )
    }

    fn picture_row(&self, chosen: &[&Item], editable: bool) -> Option<Element<'_, Message>> {
        let asset = |i: &Item| match &i.kind {
            kentos_sheet::kinds::ItemKind::Picture(p) => Some(p.asset.clone()),
            _ => None,
        };
        let first = asset(chosen.first()?)?;
        let same = chosen.iter().all(|i| asset(i).as_ref() == Some(&first));
        let meta = first
            .filter(|_| same)
            .and_then(|sha| self.book.assets.iter().find(|a| a.sha256 == sha));
        let shown = match meta {
            Some(m) => format!("{} · {} × {} px", m.name, m.width, m.height),
            None if same => "Seçilmedi".to_owned(),
            None => "—".to_owned(),
        };
        let choose = button(
            container(label::caption(if meta.is_some() {
                "Başka resim seç…"
            } else {
                "Resim seç…"
            }))
            .center_x(Fill),
        )
        .on_press_maybe(editable.then_some(Message::ChoosePicture))
        .width(Fill)
        .padding([3, 8])
        .style(style::button::secondary);
        // The file on a line of its own (a long name ends in “…”), the button under it.
        let rows = PropertySheet::new()
            .row("Resim", value(shown, false, None))
            .row(
                "Dosya",
                tip(
                    choose,
                    Tip::new("Resim seç").body(
                        "PNG, JPEG ya da SVG; en çok 4 MB. Şablon ve .kpafta resmi kendisiyle taşır.",
                    ),
                    iced::widget::tooltip::Position::Bottom,
                ),
            );
        Some(container(rows).padding([2, 4]).into())
    }

    fn page_tab(&self) -> Element<'_, Message> {
        let Some(s) = self.open_sheet() else {
            return space().into();
        };
        let page = &s.page;
        let paper = page.paper;
        let paper_menu = move || {
            PAPERS.iter().fold(Menu::new(), |m, p| {
                m.radio(paper_name(*p), *p == paper, Message::Paper(*p))
            })
        };
        let margin = |side: Side, v: i32| -> Element<'_, Message> {
            dragged(
                NumberInput::new(f64::from(v) / 1000.0, move |x| Message::Margin(side, x))
                    .units(MM)
                    .range(0.0..=200.0)
                    .decimals(1)
                    .inline(),
            )
            .into()
        };
        let mut sheet = PropertySheet::new()
            .section(
                "Kâğıt",
                self.open_section("paper"),
                Message::Section("paper"),
            )
            .row("Boy", choice(paper_name(paper), None, paper_menu))
            .row(
                "Yön",
                Segmented::new(
                    [
                        crate::gallery::OrientationName(Orientation::Portrait),
                        crate::gallery::OrientationName(Orientation::Landscape),
                    ],
                    crate::gallery::OrientationName(page.orientation),
                    |o: crate::gallery::OrientationName| Message::Orientation(o.0),
                )
                .compact()
                .width(Fill),
            )
            .row(
                "Ölçüler",
                value(
                    format!(
                        "{} × {}",
                        mm_text(i64::from(page.size.width)),
                        mm_text(i64::from(page.size.height))
                    ),
                    true,
                    Some("mm".into()),
                ),
            )
            .section(
                "Kenar boşlukları",
                self.open_section("margins"),
                Message::Section("margins"),
            )
            .row("Üst", margin(Side::Top, page.margins.top))
            .row("Sağ", margin(Side::Right, page.margins.right))
            .row("Alt", margin(Side::Bottom, page.margins.bottom))
            .row("Sol", margin(Side::Left, page.margins.left));
        // The layout in force (design §3.2a): chosen by the paper, never by hand.
        sheet = sheet.section(
            "Yerleşim düzeni",
            self.open_section("variant"),
            Message::Section("variant"),
        );
        let active = s
            .active_variant
            .as_deref()
            .and_then(|id| s.variants.iter().find(|v| v.id == id))
            .map_or_else(
                || "Temel düzen".to_owned(),
                |v| format!("{} (kendiliğinden)", v.name),
            );
        sheet = sheet.row("Geçerli", value(active, false, None));
        for v in &s.variants {
            let when = format!(
                "{}{}",
                if v.when.orientation == Orientation::Landscape {
                    "yatay"
                } else {
                    "dikey"
                },
                v.when
                    .max_width
                    .map_or(String::new(), |w| format!(", en çok {} mm", w / 1000))
            );
            sheet = sheet.row(v.name.as_str(), value(when, false, None));
        }
        let master = s
            .master
            .as_deref()
            .and_then(|m| self.book.master(m))
            .map_or("Yok".to_owned(), |m| m.name.clone());
        sheet = sheet
            .section(
                "Ana sayfa ve ızgara",
                self.open_section("grid"),
                Message::Section("grid"),
            )
            .row("Ana sayfa", value(master, false, None))
            .row(
                "Izgaraya yapış",
                Switch::new(s.snap_grid.enabled, Message::SnapToGrid),
            )
            .row(
                "Izgara aralığı",
                dragged(
                    NumberInput::new(
                        f64::from(s.snap_grid.spacing) / 1000.0,
                        Message::GridSpacing,
                    )
                    .units(MM)
                    .range(0.5..=100.0)
                    .decimals(1)
                    .inline(),
                ),
            )
            .row("Kılavuzlar", value(s.guides.len().to_string(), true, None));
        // The template it was made from (the web's page tab): its name, “Yeni sürüm var”.
        let open = self.open_section("template");
        let origin = s.origin.as_ref().map(|o| {
            self.template_of(o)
                .map_or_else(|| o.template_id.clone(), |(name, _)| name)
        });
        let newer = self.template_newer(s);
        sheet = sheet
            .section("Şablon", open, Message::Section("template"))
            .row(
                "Kaynak",
                value(
                    origin.unwrap_or_else(|| "Şablonsuz".to_owned()),
                    false,
                    None,
                ),
            );
        if newer {
            sheet = sheet.row(
                "Sürüm",
                label::body("Yeni sürüm var").style(style::text::info),
            );
        }
        if !open {
            return sheet.into();
        }
        let mut after = column![].spacing(6).padding([6, 8]);
        if newer {
            after = after.push(
                label::caption(
                    "Şablonun daha yeni bir sürümü kaydedildi. Pafta kendiliğinden değişmez; güncellemeyi uygulamak sonraki aşamada gelecek.",
                )
                .style(style::text::muted),
            );
        }
        after = after.push(
            button(label::body("Şablon olarak kaydet"))
                .on_press(Message::SaveTemplate(
                    crate::save_template::SaveMessage::Open,
                ))
                .padding([4, 10])
                .style(style::button::secondary),
        );
        column![sheet, after].into()
    }

    fn preflight_tab(&self) -> Element<'_, Message> {
        if self.findings.is_empty() {
            return container(
                EmptyState::new(Icon::Success, "Ön denetim temiz")
                    .description("Sayfa dışında kalan, örtülen ya da bağı kopuk öğe, sığmayan yazı, eksik resim yok."),
            )
            .padding(16)
            .into();
        }
        let count = |s: Severity| self.findings.iter().filter(|f| f.severity == s).count();
        let mut col = column![
            container(label::caption(format!(
                "{} hata, {} uyarı, {} not",
                count(Severity::Error),
                count(Severity::Warning),
                count(Severity::Info)
            )))
            .padding([2, 12])
        ]
        .spacing(2);
        for (i, f) in self.findings.iter().enumerate() {
            let glyph = match f.severity {
                Severity::Error => Icon::Error,
                Severity::Warning => Icon::Warning,
                Severity::Info => Icon::Info,
            };
            let mut body = column![
                label::body(f.message.as_str()),
                label::caption(f.fix.as_str()).style(style::text::muted)
            ]
            .spacing(2);
            let mut actions = row![].spacing(6);
            if let Some(item) = &f.item {
                let name = self
                    .book
                    .item(item)
                    .map_or_else(|| item.clone(), |it| it.name.clone());
                actions = actions.push(
                    button(label::caption(format!("Göster: {name}")))
                        .on_press(Message::Reveal(item.clone()))
                        .padding([2, 8])
                        .style(style::button::secondary),
                );
            }
            for (j, fix) in f.fixes.iter().enumerate() {
                actions = actions.push(
                    button(label::caption(fix.label.as_str()))
                        .on_press(Message::Fix(i, j))
                        .padding([2, 8])
                        .style(style::button::secondary),
                );
            }
            body = body.push(actions);
            col = col.push(
                container(row![icon(glyph).size(16.0), body].spacing(8))
                    .padding([8, 12])
                    .width(Fill)
                    .style(style::container::bordered),
            );
        }
        col.into()
    }
}

/// A muted line under a field: what it is from, or how to change it.
fn hint<'a>(text: String) -> Element<'a, Message> {
    container(label::caption(text).style(style::text::muted))
        .padding([0, 12])
        .width(Fill)
        .into()
}

/// A label over its value and the hint under it (the web's `labelled`).
fn labelled<'a>(
    name: &'static str,
    content: Element<'a, Message>,
    under: String,
) -> Element<'a, Message> {
    column![
        container(label::caption(name).style(style::text::muted)).padding([0, 12]),
        container(content).padding([0, 12]).width(Fill),
        hint(under),
    ]
    .spacing(2)
    .into()
}

fn icons_select() -> Icon {
    Icon::Select
}
