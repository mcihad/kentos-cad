//! The right column of Stil yöneticisi (the web's `managerDetails.ts`): a
//! large picture of the chosen item on a sample the user picks, what it is
//! and where it comes from, its editable fields (user and project items),
//! and what can be done with it; or the import panel while a .kstil file is
//! being taken in; or a word on what to do when nothing is chosen.

use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::text_editor::{self, Binding, KeyPress};
use iced::widget::{
    Column, Row, button, column, container, row, space, text_editor as editor, text_input,
};
use iced::{Center, Element, Fill, Length};
use kentos_native_style::file::{ConflictMode, kind_of};
use kentos_native_style::library::{Item, ItemKind, Source, StyleLibrary};
use kentos_native_style::preview::Geometry;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Menu, MenuButton, RadioGroup, Segmented, Tip, tip};
use serde_json::{Value, json};

use super::{Event, Field, ImportDraft, Manager, ev};
use crate::app::Message;
use crate::style::thumbs::{Look, Thumbs};

/// Why an edit that needs the SVG editor waits.
pub const SVG_NOT_YET: &str = "SVG çizim düzenleyicisi web'de var; masaüstüne henüz taşınmadı.";

/// The picture of an item: a symbol, or a drawing as a marker of itself (`symbolOfItem`).
pub fn symbol_of_item(item: &Item) -> Value {
    match item.kind() {
        ItemKind::Symbol => item.symbol().cloned().unwrap_or(Value::Null),
        ItemKind::Asset if item.format() == Some("svg") => json!({ "type": "marker", "layers": [
            { "id": "a", "type": "svg", "asset": item.id(), "size": 14, "fill": "ink" },
        ] }),
        ItemKind::Asset => json!({ "type": "marker", "layers": [
            { "id": "a", "type": "raster", "asset": item.id(), "size": 14 },
        ] }),
    }
}

/// A sample choice of the preview, with the name the segmented control gives it.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Sample(Geometry, &'static str);

impl std::fmt::Display for Sample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

/// The samples a symbol type can be shown on (`GEOMETRY_OPTIONS`); a line symbol on an area draws its edge.
fn samples(kind: &str) -> &'static [Sample] {
    match kind {
        "line" => &[
            Sample(Geometry::Line, "Düz"),
            Sample(Geometry::Bent, "Kırık"),
            Sample(Geometry::Area, "Alan kenarı"),
        ],
        "fill" => &[
            Sample(Geometry::Area, "Alan"),
            Sample(Geometry::Hole, "Adalı"),
        ],
        _ => &[Sample(Geometry::Point, "Nokta")],
    }
}

/// The source's badge: “Sistem · salt okunur”, “Kitaplığım”, “Proje”.
pub fn source_badge<'a>(source: Source, long: bool) -> Element<'a, Message> {
    let text = match source {
        Source::System if long => "Sistem · salt okunur",
        s => s.label(),
    };
    container(label::caption(text).size(typography::caption() - 1.0))
        .padding([1, 6])
        .style(move |t: &iced::Theme| {
            let tk = Tokens::of(t);
            let tone = match source {
                Source::System => tk.muted,
                Source::User => tk.accent,
                Source::Project => tk.success,
            };
            container::Style {
                background: Some(iced::Background::Color(tone.scale_alpha(0.14))),
                text_color: Some(tone),
                border: iced::border::rounded(8.0),
                ..container::Style::default()
            }
        })
        .into()
}

fn field_row<'a>(name: &'static str, value: Element<'a, Message>) -> Element<'a, Message> {
    column![label::caption(name).style(style::text::muted), value]
        .spacing(3)
        .width(Fill)
        .into()
}

fn small_button<'a>(
    glyph: &str,
    text: String,
    press: Option<Message>,
    danger: bool,
) -> iced::widget::Button<'a, Message> {
    button(
        row![
            icon(crate::icons::from_web(Some(glyph))).size(13.0),
            label::body(text)
        ]
        .spacing(5)
        .align_y(Center),
    )
    .padding([4, 9])
    .style(if danger {
        style::button::danger
    } else {
        style::button::secondary
    })
    .on_press_maybe(press)
}

/// The Açıklama box's keys: Ctrl+Enter keeps the text, Esc keeps it and lets
/// the keyboard go (the web keeps it when the box loses focus); the rest as
/// in any text box.
fn notes_keys(press: KeyPress) -> Option<Binding<Message>> {
    let focused = matches!(press.status, text_editor::Status::Focused { .. });
    match press.key.as_ref() {
        Key::Named(Named::Enter) if focused && press.modifiers.command() => {
            Some(Binding::Custom(ev(Event::Commit(Field::Description))))
        }
        Key::Named(Named::Escape) if focused => Some(Binding::Sequence(vec![
            Binding::Unfocus,
            Binding::Custom(ev(Event::Commit(Field::Description))),
        ])),
        _ => Binding::from_key_press(press),
    }
}

/// The details of the chosen item, or the import panel, or what to do.
/// `open`: how many objects the drawing has selected, and whether a drawing
/// is open (the project's library needs one); `preview`: the picture's size.
pub fn details<'a>(
    m: &'a Manager,
    lib: &'a StyleLibrary,
    thumbs: &Thumbs,
    look: &Look<'_>,
    (selected_objects, project_open): (usize, bool),
    preview: (f32, f32),
) -> Element<'a, Message> {
    if let Some(draft) = &m.import {
        return import_panel(draft, lib, project_open, preview.0);
    }
    let Some((item, source)) = m.selected.as_deref().and_then(|id| lib.get(id)) else {
        return container(
            column![
                icon(crate::icons::from_web(Some("styles")))
                    .size(28.0)
                    .tone(Tone::Muted),
                label::strong("Bir sembol seçin"),
                label::body("Soldan bir kaynak ya da kategori, ortadan bir sembol seçin. Sistem sembolleri salt okunurdur; kopyalayıp kendi kitaplığınızda ya da projede düzenleyebilirsiniz.")
                    .style(style::text::muted)
                    .align_x(iced::alignment::Horizontal::Center),
            ]
            .spacing(10)
            .align_x(Center),
        )
        .padding([60, 18])
        .width(Fill)
        .into();
    };
    let editable = source.editable();
    let symbol = symbol_of_item(item);
    let kind = kind_of(item.kind(), item.symbol());
    let options = if item.kind() == ItemKind::Symbol {
        samples(kind)
    } else {
        &[][..]
    };
    let geometry = m
        .geometry
        .get(kind)
        .copied()
        .or_else(|| options.first().map(|s| s.0));
    // The web's 300 × 172 at 3.2 pixels a millimetre; narrower when the column is.
    let picture = thumbs.picture(&symbol, geometry, preview, Some(3.2), look);
    let mut preview = Column::new().spacing(8).align_x(Center).push(picture);
    if options.len() > 1 {
        let kind_key: &'static str = match kind {
            "line" => "line",
            "fill" => "fill",
            _ => "marker",
        };
        let chosen = options
            .iter()
            .copied()
            .find(|s| Some(s.0) == geometry)
            .unwrap_or(options[0]);
        preview = preview.push(Segmented::new(options.iter().copied(), chosen, move |s| {
            ev(Event::Geometry(kind_key, s.0))
        }));
    }
    let typed = |f: Field, value: String| m.typed.get(&f).cloned().unwrap_or(value);
    let input = |f: Field, value: String, placeholder: &str| -> Element<'a, Message> {
        text_input(placeholder, &typed(f, value))
            .padding([4, 7])
            .font(typography::ui())
            .size(typography::body())
            .style(style::field::input)
            .on_input(move |t| ev(Event::Field(f, t)))
            .on_submit(ev(Event::Commit(f)))
            .into()
    };
    let text_or_dash = |v: String| -> Element<'a, Message> {
        if v.is_empty() {
            label::body("—").style(style::text::muted).into()
        } else {
            label::body(v).into()
        }
    };
    let title: Element<'a, Message> = if editable {
        input(Field::Name, item.name().to_owned(), "Ad")
    } else {
        label::heading(item.name().to_owned()).into()
    };
    let kind_text = match (item.kind(), kind) {
        (ItemKind::Asset, _) => format!("Çizim ({})", item.format().unwrap_or("").to_uppercase()),
        (_, "fill") => "Alan sembolü".into(),
        (_, "line") => "Çizgi sembolü".into(),
        _ => "İşaret sembolü".into(),
    };
    let kind_line = row![
        label::caption(kind_text).style(style::text::muted),
        source_badge(source, true)
    ]
    .spacing(8)
    .align_y(Center);
    let path = item.path().join(" / ");
    let mut fields = Column::new().spacing(10);
    fields = fields.push(field_row(
        "Kategori",
        if editable {
            input(Field::Path, path, "Ana / Alt")
        } else {
            text_or_dash(path)
        },
    ));
    fields = fields.push(field_row(
        "Kimlik",
        label::mono_caption(item.id().to_owned())
            .style(style::text::muted)
            .into(),
    ));
    if item.kind() == ItemKind::Symbol {
        let reference = item.text("reference").unwrap_or("").to_owned();
        if !reference.is_empty() || editable {
            fields = fields.push(field_row(
                "Kaynak",
                if editable {
                    input(Field::Reference, reference, "Yönetmelik, sayfa")
                } else {
                    text_or_dash(reference)
                },
            ));
        }
        let description = item.text("description").unwrap_or("").to_owned();
        let notes = m
            .notes
            .as_ref()
            .filter(|(id, _)| id == item.id())
            .map(|(_, content)| content);
        fields = fields.push(field_row(
            "Açıklama",
            match notes {
                Some(content) if editable => editor(content)
                    .on_action(|a| ev(Event::Describe(a)))
                    .key_binding(notes_keys)
                    .height(Length::Fixed(typography::from_default(60.0)))
                    .size(typography::body())
                    .padding([5, 7])
                    .style(style::field::text_area)
                    .into(),
                _ if editable => input(Field::Description, description, ""),
                _ => text_or_dash(description),
            },
        ));
    }
    let tags = item.tags().join(", ");
    fields = fields.push(field_row(
        "Etiketler",
        if editable {
            input(Field::Tags, tags, "virgülle ayırın")
        } else {
            text_or_dash(tags)
        },
    ));
    if item.kind() == ItemKind::Asset {
        let users = lib.users_of(item.id()).len();
        fields = fields.push(field_row(
            "Kullanan",
            if users > 0 {
                label::body(format!("{users} sembol")).into()
            } else {
                label::body("Hiçbir sembol kullanmıyor")
                    .style(style::text::muted)
                    .into()
            },
        ));
    }
    if editable {
        fields = fields.push(
            label::caption(
                "Değişiklik Enter'la (Açıklama'da Ctrl+Enter) ya da başka bir öğeye geçince kaydedilir.",
            )
            .style(style::text::muted),
        );
    }
    let id = item.id().to_owned();
    let mut actions = Row::new().spacing(6);
    // Symbols open in Sembol tasarımcısı (a system one's copy in Kitaplığım);
    // SVG drawings in the SVG editor, the web's for now.
    let edit_label = if editable {
        "Düzenle"
    } else {
        "Kopyasını düzenle"
    };
    let editor = match item.kind() {
        ItemKind::Symbol => Some((
            if editable {
                "Sembol tasarımcısında açar"
            } else {
                "Kitaplığım'a bir kopya alır ve onu açar"
            },
            Some(ev(Event::Edit(id.clone()))),
        )),
        ItemKind::Asset if item.format() == Some("svg") => Some((SVG_NOT_YET, None)),
        ItemKind::Asset => None,
    };
    if let Some((note, press)) = editor {
        actions = actions.push(tip(
            small_button("edit", edit_label.into(), press, false),
            Tip::new(edit_label).body(note),
            iced::widget::tooltip::Position::Top,
        ));
    }
    if item.kind() == ItemKind::Symbol {
        let label = if selected_objects > 0 {
            format!("Seçili nesnelere uygula ({selected_objects})")
        } else {
            "Seçili nesnelere uygula".into()
        };
        actions = actions.push(tip(
            small_button(
                "check",
                label,
                (selected_objects > 0).then(|| ev(Event::Apply(id.clone()))),
                false,
            ),
            Tip::new(if selected_objects > 0 {
                "Nesnelerin sembolü olur; katman stilinin önüne geçer"
            } else {
                "Önce çizimde nesne seçin"
            }),
            iced::widget::tooltip::Position::Top,
        ));
    }
    let mut second = Row::new().spacing(6);
    let copy_id = id.clone();
    second = second.push(MenuButton::new(
        container(
            row![
                icon(crate::icons::from_web(Some("copy"))).size(13.0),
                label::body("Kopyala"),
                icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted)
            ]
            .spacing(5)
            .align_y(Center),
        )
        .padding([4, 9])
        .style(style::container::field_box),
        move || {
            Menu::new()
                .item(
                    "Kitaplığıma",
                    ev(Event::Copy(copy_id.clone(), Source::User)),
                )
                .detail("Bu bilgisayarda, bütün çizimlerde")
                .item(
                    "Projeye",
                    project_open.then(|| ev(Event::Copy(copy_id.clone(), Source::Project))),
                )
                .detail(if project_open {
                    "Proje dosyasında; projeyi açan herkes görür"
                } else {
                    "Açık çizim yok."
                })
        },
    ));
    second = second.push(small_button(
        "export",
        "Dışa aktar".into(),
        Some(ev(Event::Export(vec![id.clone()], item.name().to_owned()))),
        false,
    ));
    if editable {
        second = second.push(small_button(
            "trash",
            "Sil".into(),
            Some(ev(Event::Delete(id))),
            true,
        ));
    }
    let mut buttons = Column::new().spacing(6);
    if item.kind() == ItemKind::Symbol || item.format() == Some("svg") {
        buttons = buttons.push(actions.wrap());
    }
    column![
        container(preview).center_x(Fill),
        column![title, kind_line].spacing(4),
        fields,
        buttons.push(second.wrap()),
    ]
    .spacing(14)
    .padding([4, 4])
    .width(Fill)
    .into()
}

/// A choice of the import panel.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Choice<T: Copy + PartialEq>(T, &'static str);

impl<T: Copy + PartialEq> std::fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

/// The import panel: what the file holds, where it goes, what an id the
/// library has does (`renderImport`); `width`: the column's room.
fn import_panel<'a>(
    draft: &ImportDraft,
    lib: &StyleLibrary,
    project_open: bool,
    width: f32,
) -> Element<'a, Message> {
    let symbols = draft
        .file
        .items
        .iter()
        .filter(|i| i.get("kind").and_then(Value::as_str) == Some("symbol"))
        .count();
    let assets = draft.file.items.len() - symbols;
    let clashes = draft
        .file
        .items
        .iter()
        .filter(|i| {
            i.get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| lib.get(id).is_some())
        })
        .count();
    let mut what = format!("“{}”: {symbols} sembol", draft.name);
    if assets > 0 {
        what.push_str(&format!(", {assets} çizim"));
    }
    what.push('.');
    let targets = [
        Choice(Source::User, "Kitaplığım"),
        Choice(Source::Project, "Proje"),
    ];
    let to = targets
        .iter()
        .copied()
        .find(|c| c.0 == draft.to)
        .unwrap_or(targets[0]);
    let mut out = Column::new()
        .spacing(10)
        .push(label::heading("İçe aktar"))
        .push(label::body(what).style(style::text::muted))
        .push(label::caption("Nereye").style(style::text::muted))
        .push(Segmented::new_with(
            targets,
            to,
            |c| ev(Event::ImportTo(c.0)),
            move |c| c.0 != Source::Project || project_open,
        ));
    if clashes > 0 {
        let modes = [
            Choice(ConflictMode::Copy, "Kopya olarak al"),
            Choice(ConflictMode::Replace, "Üzerine yaz"),
            Choice(ConflictMode::Skip, "Atla"),
        ];
        let mode = modes
            .iter()
            .copied()
            .find(|c| c.0 == draft.mode)
            .unwrap_or(modes[0]);
        // The web's segmented choice; one under the other when the column is too narrow for it.
        let body = typography::body();
        let needed: f32 = modes
            .iter()
            .map(|c| typography::text_width(c.1, body) + 25.0)
            .sum::<f32>()
            + 2.0;
        let choice: Element<'a, Message> = if needed <= width {
            Segmented::new(modes, mode, |c| ev(Event::ImportMode(c.0))).into()
        } else {
            modes
                .iter()
                .fold(
                    RadioGroup::new(draft.mode, |m| ev(Event::ImportMode(m))),
                    |group, c| group.option(c.0, c.1, ""),
                )
                .into()
        };
        out = out
            .push(
                label::caption(format!("{clashes} öğe kitaplıkta zaten var"))
                    .style(style::text::muted),
            )
            .push(choice);
        if draft.mode == ConflictMode::Replace {
            // As `importStyles` does (fixtures/style/v1/kstil.json); the web's note said
            // they came as copies.
            out = out.push(
                label::caption(
                    "Yalnız hedef kitaplıktakilerin üzerine yazılır; sistemdekiler ve öbür kitaplıktakiler atlanır.",
                )
                .style(style::text::muted),
            );
        }
    }
    out.push(
        row![
            button(
                row![
                    icon(crate::icons::from_web(Some("import")))
                        .size(13.0)
                        .tone(Tone::OnAccent),
                    label::body("İçe aktar").style(style::text::on_accent)
                ]
                .spacing(5)
                .align_y(Center)
            )
            .padding([4, 10])
            .style(style::button::primary)
            .on_press(ev(Event::ImportRun)),
            button(label::body("Vazgeç"))
                .padding([4, 10])
                .style(style::button::secondary)
                .on_press(ev(Event::ImportCancel)),
            space::horizontal(),
        ]
        .spacing(6),
    )
    .padding([4, 4])
    .width(Length::Fill)
    .into()
}
