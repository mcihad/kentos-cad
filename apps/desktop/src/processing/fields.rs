//! One control per parameter type, made from the definition (the web's
//! `paramFields.ts`): scope buttons with what they resolve to and the kind
//! chips; numbers with their unit; texts; switches; choices as buttons or a
//! list; the target layer, new or existing; a point shown on the drawing;
//! an attribute name, typed or picked; an expression with its fields,
//! variables, functions and a live line.

use std::fmt;

use iced::widget::{Column, Row, button, column, container, row, text_input};
use iced::{Alignment, Center, Element, Fill, Length};
use kentos_interaction::Format;
use kentos_processing::expression::{FUNCTIONS, VARIABLES};
use kentos_processing::features::kind_label;
use kentos_processing::parameters::scopes_of;
use kentos_processing::text::{js_number, tr_lower};
use kentos_processing::values::{FeaturesValue, LayerValue, Scope, point};
use kentos_processing::{EnumOption, ParamDef, ParamKind, ScopeKind};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Menu, MenuButton, Segmented, Switch, Tip, focus_ring, tip};
use serde_json::{Value, json};

use super::Event;
use super::dialog::ToolDialog;
use crate::app::Message;

/// What a control reads besides its own parameter.
pub(super) struct Env<'a, 'b> {
    pub window: &'a ToolDialog,
    /// Where the control's events go: the tool's window, or the model
    /// designer's fixed value (processing/designer).
    pub send: fn(Event) -> Message,
    /// Sahneden seç beside a features field (the web's designer offers none).
    pub pick_objects: bool,
    /// İfade oluşturucu beside an expression field.
    pub builder: bool,
    pub doc: &'a kentos_domain::Document,
    pub format: &'b Format,
    /// A layer style's colour on the screen (hex or a theme token).
    pub color: &'b dyn Fn(&str) -> iced::Color,
}

impl Env<'_, '_> {
    fn value(&self, name: &str) -> &Value {
        self.window.values.get(name).unwrap_or(&Value::Null)
    }
}

/// The scope buttons' short names (the web's `SCOPE_SHORT`).
#[derive(Clone, Copy, PartialEq)]
struct ScopeButton(ScopeKind);

impl fmt::Display for ScopeButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            ScopeKind::Selection => "Seçili",
            ScopeKind::Visible => "Görünen",
            ScopeKind::All => "Tümü",
            ScopeKind::Layer => "Katman",
        })
    }
}

/// A choice's button (an index and its label).
#[derive(Clone, Copy, PartialEq)]
struct Option_<'a>(usize, &'a str);

impl fmt::Display for Option_<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.1)
    }
}

/// The control of a parameter.
pub(super) fn control<'a>(def: &'a ParamDef, env: &Env<'a, '_>) -> Element<'a, Message> {
    let ev = env.send;
    match &def.kind {
        ParamKind::Features { kinds, scopes, .. } => {
            features(def, kinds.as_deref(), scopes.as_deref(), env)
        }
        ParamKind::Number { unit, .. } => number(def, unit, env),
        ParamKind::Text {
            placeholder,
            max_length,
            ..
        } => text(def, placeholder.as_deref(), *max_length, env),
        ParamKind::Boolean => {
            let name = def.name.clone();
            let on = env.value(&def.name).as_bool().unwrap_or(false);
            Switch::new(on, move |v| ev(Event::Value(name.clone(), json!(v)))).into()
        }
        ParamKind::Choice { options } => {
            let control = choice(def, options, env);
            // One of its options is a point picked on the drawing (the numbering's start vertex).
            match &def.picks {
                Some((option, _)) => {
                    let chosen = env.value(&def.name).as_str() == Some(option.as_str());
                    row![
                        container(control).width(Fill),
                        tip(
                            pick_button(None, ev(Event::PickChoice(def.name.clone())), chosen),
                            Tip::new("Sahneden seç")
                                .body("Başlangıcı çizimde gösterin: her nesnede o noktaya en yakın köşeden başlanır."),
                            iced::widget::tooltip::Position::Top,
                        ),
                    ]
                    .spacing(6)
                    .align_y(Center)
                    .into()
                }
                None => control,
            }
        }
        ParamKind::Layer { .. } => layer(def, env),
        ParamKind::Point => point_field(def, env),
        ParamKind::Field {
            of,
            allow_new,
            multiple,
        } => field(def, of, *allow_new, *multiple, env),
        ParamKind::File { .. } => file(def, env),
        ParamKind::Expression {
            of, placeholder, ..
        } => expression(def, of.as_deref(), placeholder.as_deref(), env),
    }
}

/// A single-line field in the dialog's look.
fn input<'a>(
    placeholder: &str,
    value: &str,
    invalid: bool,
    ev: fn(Event) -> Message,
) -> iced::widget::TextInput<'a, Message> {
    text_input(placeholder, value)
        .padding([5, 8])
        .font(typography::ui())
        .size(typography::body())
        .style(style::field::validated(invalid))
        .on_submit(ev(Event::Run))
}

fn features<'a>(
    def: &'a ParamDef,
    kinds: Option<&'a [String]>,
    scopes: Option<&'a [ScopeKind]>,
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    let ev = env.send;
    let value = FeaturesValue::read(env.value(&def.name))
        .unwrap_or_else(|| FeaturesValue::scope(Scope::Selection));
    let offered = scopes_of(scopes);
    let chosen = match &value.scope {
        Scope::Selection => ScopeKind::Selection,
        Scope::Visible => ScopeKind::Visible,
        Scope::All => ScopeKind::All,
        Scope::Layer(_) => ScopeKind::Layer,
        Scope::Ids(_) => offered.first().copied().unwrap_or(ScopeKind::Selection),
    };
    let name = def.name.clone();
    let mut parts = Column::new().spacing(8).width(Fill);
    let mut scope_row = row![
        Segmented::new(
            offered.iter().copied().map(ScopeButton),
            ScopeButton(chosen),
            move |s| ev(Event::Scope(name.clone(), s.0.id().into())),
        )
        .width(Fill),
    ]
    .spacing(6)
    .align_y(Center);
    if env.pick_objects {
        scope_row = scope_row.push(tip(
            pick_button(
                Some("Sahneden seç"),
                ev(Event::PickObjects(def.name.clone())),
                false,
            ),
            Tip::new("Sahneden seç").body(
                "Nesneleri çizimde tıklayarak ya da pencereyle seçin; Enter bitirir, Esc vazgeçer.",
            ),
            iced::widget::tooltip::Position::Top,
        ));
    }
    parts = parts.push(scope_row);
    if let Scope::Layer(id) = &value.scope {
        let leaves = env.doc.layers().leaves();
        let ids: Vec<String> = leaves.iter().map(|l| l.id.clone()).collect();
        let selected = ids.iter().position(|l| l == id);
        let choices = leaves
            .iter()
            .map(|l| Choice::new(l.name.clone()).color((env.color)(&l.style.color)));
        let name = def.name.clone();
        let chosen_kinds = value.kinds.clone();
        parts = parts.push(Select::new(choices, selected, move |i| {
            let mut v =
                json!({ "scope": "layer", "layerId": ids.get(i).cloned().unwrap_or_default() });
            if let Some(k) = &chosen_kinds {
                v["kinds"] = json!(k);
            }
            ev(Event::Value(name.clone(), v))
        }));
    }
    let found = env.window.inputs.get(&def.name);
    let empty = found.is_none_or(|f| f.count == 0);
    parts = parts.push(
        row![
            icon(if empty { Icon::Warning } else { Icon::Check })
                .size(14.0)
                .tone(if empty { Tone::Warning } else { Tone::Success }),
            label::body(found.map(|f| f.description.clone()).unwrap_or_default())
                .style(style::text::muted),
        ]
        .spacing(6)
        .align_y(Center),
    );
    // Kind chips: only when there is a choice (two or more kinds in scope), or to undo one.
    let present = found.map(|f| f.by_kind.as_slice()).unwrap_or_default();
    if present.len() >= 2 || (value.kinds.is_some() && !present.is_empty()) {
        let on = |k: &str| {
            value
                .kinds
                .as_ref()
                .is_none_or(|ks| ks.iter().any(|x| x == k))
        };
        let mut chips = Row::new()
            .spacing(4)
            .align_y(Center)
            .push(label::caption("Türler").style(style::text::muted));
        for (kind, count) in present {
            let pressed = on(kind);
            let mut face = Row::new().spacing(5).align_y(Center);
            if pressed {
                face = face.push(icon(Icon::Check).size(12.0).tone(Tone::Accent));
            }
            face = face
                .push(label::caption(kind_label(kind)))
                .push(label::mono_caption(count.to_string()).style(style::text::muted));
            chips = chips.push(tip(
                button(face)
                    .padding([3, 9])
                    .style(style::button::toggle(pressed))
                    .on_press(ev(Event::Kind(def.name.clone(), kind.clone()))),
                Tip::new(if pressed {
                    "Bu türü dışarıda bırak"
                } else {
                    "Bu türü de al"
                }),
                iced::widget::tooltip::Position::Bottom,
            ));
        }
        parts = parts.push(chips.wrap());
    } else if let Some(kinds) = kinds {
        // A tool that takes nearly every kind says what it leaves out (the web's `kindsView`).
        let left: Vec<&str> = KINDS
            .iter()
            .copied()
            .filter(|k| !kinds.iter().any(|x| x == k))
            .collect();
        let note = if left.len() < kinds.len() {
            let list: Vec<String> = left.iter().map(|k| tr_lower(kind_label(k))).collect();
            let list = list.join(", ");
            let mut chars = list.chars();
            let first = chars
                .next()
                .map(|c| kentos_processing::text::tr_upper(&c.to_string()))
                .unwrap_or_default();
            format!("{first}{} alınmaz.", chars.as_str())
        } else {
            let names: Vec<String> = kinds.iter().map(|k| tr_lower(kind_label(k))).collect();
            format!("Uygun nesneler: {}", names.join(", "))
        };
        parts = parts.push(label::caption(note).style(style::text::muted));
    }
    parts.into()
}

fn number<'a>(def: &'a ParamDef, unit: &'a str, env: &Env<'a, '_>) -> Element<'a, Message> {
    let ev = env.send;
    let v = env.value(&def.name);
    let typed = env.window.numbers.get(&def.name).cloned();
    let shown = typed.unwrap_or_else(|| v.as_f64().map(js_number).unwrap_or_default());
    let invalid = v.as_f64().is_none_or(|n| !n.is_finite());
    let name = def.name.clone();
    let field = input("", &shown, invalid, ev)
        .on_input(move |t| ev(Event::Number(name.clone(), t)))
        .font(typography::mono())
        .align_x(Alignment::End)
        .width(Length::Fixed(typography::scaled(120.0)));
    let mut line = row![focus_ring(field)].spacing(8).align_y(Center);
    if !unit.is_empty() {
        line = line.push(label::caption(unit).style(style::text::muted));
    }
    line.into()
}

fn text<'a>(
    def: &'a ParamDef,
    placeholder: Option<&'a str>,
    max_length: Option<usize>,
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    let ev = env.send;
    let value = env.value(&def.name).as_str().unwrap_or("");
    let invalid = env.window.issue_of(&def.name).is_some();
    let name = def.name.clone();
    let field = input(placeholder.unwrap_or(""), value, invalid, ev)
        .on_input(move |t| ev(Event::Text(name.clone(), t)));
    // A one or two letter field is short and centred (the web's `pfield__text--short`).
    if max_length.is_some_and(|m| m <= 2) {
        focus_ring(
            field
                .align_x(Alignment::Center)
                .width(Length::Fixed(typography::scaled(64.0))),
        )
        .into()
    } else {
        focus_ring(field.width(Fill)).into()
    }
}

/// A choice: up to three short options as buttons, else a list with each one's hint.
fn choice<'a>(
    def: &'a ParamDef,
    options: &'a [EnumOption],
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    let ev = env.send;
    let value = env.value(&def.name).as_str().unwrap_or("");
    let chosen = options.iter().position(|o| o.value == value);
    let name = def.name.clone();
    let values: Vec<String> = options.iter().map(|o| o.value.clone()).collect();
    let short = options.len() <= 3 && options.iter().all(|o| o.label.chars().count() <= 22);
    if short {
        let buttons = options
            .iter()
            .enumerate()
            .map(|(i, o)| Option_(i, o.label.as_str()));
        let hints: Vec<String> = options
            .iter()
            .map(|o| o.hint.clone().unwrap_or_default())
            .collect();
        return Segmented::new(
            buttons,
            Option_(
                chosen.unwrap_or(0),
                options
                    .get(chosen.unwrap_or(0))
                    .map_or("", |o| o.label.as_str()),
            ),
            move |o| ev(Event::Value(name.clone(), json!(values[o.0]))),
        )
        .hints(hints)
        .width(Fill)
        .into();
    }
    let choices = options.iter().map(|o| {
        let c = Choice::new(o.label.clone());
        match &o.hint {
            Some(hint) => c.detail(hint.clone()),
            None => c,
        }
    });
    Select::new(choices, chosen, move |i| {
        ev(Event::Value(
            name.clone(),
            json!(values.get(i).cloned().unwrap_or_default()),
        ))
    })
    .searchable(false)
    .into()
}

/// The target layer: a new one (named below) or an existing, unlocked one.
fn layer<'a>(def: &'a ParamDef, env: &Env<'a, '_>) -> Element<'a, Message> {
    let ev = env.send;
    let layers = env.doc.layers();
    let value = LayerValue::read(env.value(&def.name));
    let suggested = match &def.default {
        Some(kentos_processing::DefaultValue::Value(v)) => v
            .get("newName")
            .and_then(Value::as_str)
            .unwrap_or("Yeni katman")
            .to_owned(),
        _ => "Yeni katman".to_owned(),
    };
    let new_name = match &value {
        Some(LayerValue::New(n)) => n.clone(),
        _ => suggested,
    };
    let leaves = layers.leaves();
    let trimmed = kentos_processing::text::js_trim(&new_name);
    // A new name an existing layer has writes to that layer (the runner's rule): “(mevcut)”.
    let same = leaves
        .iter()
        .find(|l| tr_lower(kentos_processing::text::js_trim(&l.name)) == tr_lower(trimmed));
    let shown = match same {
        Some(l) => format!("{} (mevcut)", l.name),
        None if trimmed.is_empty() => "(yeni)".to_owned(),
        None => format!("{trimmed} (yeni)"),
    };
    // The web's plan (fieldPlan.ts `layerFieldView`): the new layer, then the
    // existing ones by their place in the tree; locked ones cannot be chosen.
    let mut choices = vec![
        Choice::header("Yeni katman"),
        Choice::new(format!("Yeni: {trimmed}"))
            .icon(crate::icons::from_web(Some("layerAdd")))
            .shown(shown),
        Choice::header("Mevcut katmanlar"),
    ];
    let mut ids = vec![String::new(); 3];
    for l in &leaves {
        let locked = layers.is_locked(&l.id);
        let mut c = Choice::new(layers.path(&l.id)).color((env.color)(&l.style.color));
        if locked {
            c = c.detail("kilitli").disabled();
        }
        choices.push(c);
        ids.push(l.id.clone());
    }
    let selected = match &value {
        Some(LayerValue::New(_)) | None => Some(1),
        Some(LayerValue::Existing(id)) => ids.iter().position(|x| !x.is_empty() && x == id),
    };
    let name = def.name.clone();
    let fresh = new_name.clone();
    let list = Select::new(choices, selected, move |i| match i {
        1 => ev(Event::Value(name.clone(), json!({ "newName": fresh }))),
        i => ev(Event::Value(
            name.clone(),
            json!({ "layerId": ids.get(i).cloned().unwrap_or_default() }),
        )),
    });
    if !matches!(value, Some(LayerValue::New(_))) {
        return list.into();
    }
    let name = def.name.clone();
    let field = input("Yeni katmanın adı", &new_name, false, ev)
        .on_input(move |t| ev(Event::LayerName(name.clone(), t)))
        .width(Fill);
    column![list, focus_ring(field)]
        .spacing(8)
        .width(Fill)
        .into()
}

fn point_field<'a>(def: &'a ParamDef, env: &Env<'a, '_>) -> Element<'a, Message> {
    let ev = env.send;
    let p = point(env.value(&def.name));
    let coord = match p {
        Some(p) => label::mono(env.format.point(kentos_interaction::Vec2::new(p.x, p.y))),
        None => label::body("Henüz seçilmedi").style(style::text::muted),
    };
    let pick = pick_button(
        Some(if p.is_some() {
            "Yeniden seç"
        } else {
            "Sahneden seç"
        }),
        ev(Event::Pick(def.name.clone())),
        false,
    );
    row![container(coord).width(Fill), pick]
        .spacing(10)
        .align_y(Center)
        .into()
}

/// Sahneden seç: the KentOS UI inspector's pick (its target icon), for a
/// point or objects picked on the drawing (docs/adr/0088); `on` when what it
/// picks is in use.
fn pick_button<'a>(caption: Option<&'a str>, on_press: Message, on: bool) -> Element<'a, Message> {
    let glyph = icon(Icon::Target)
        .size(14.0)
        .tone(if on { Tone::Accent } else { Tone::Inherit });
    let face: Element<'a, Message> = match caption {
        Some(text) => row![glyph, label::body(text)]
            .spacing(6)
            .align_y(Center)
            .into(),
        None => glyph.into(),
    };
    button(face)
        .padding(if caption.is_some() { [5, 10] } else { [5, 8] })
        .style(style::button::secondary)
        .on_press(on_press)
        .into()
}

/// Every object kind in the web's order (`ENTITY_KIND_LABEL`): what a
/// features field's note names as left out.
const KINDS: [&str; 17] = [
    "point",
    "line",
    "polyline",
    "polygon",
    "circle",
    "arc",
    "ellipse",
    "spline",
    "xline",
    "ray",
    "text",
    "dimension",
    "hatch",
    "insert",
    "leader",
    "table",
    "image",
];

/// An attribute name: typed (a new one too, with `allow_new`) or picked
/// from those its source has (a layer's objects, or a file's columns, the
/// first of `of` shown), with a note on what writing it does; `multiple`:
/// several names, the list checks each (the web's `attrFieldView`).
fn field<'a>(
    def: &'a ParamDef,
    of: &'a [String],
    allow_new: bool,
    multiple: bool,
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    let ev = env.send;
    let value = env.value(&def.name).as_str().unwrap_or("").to_owned();
    let source =
        kentos_processing::parameters::field_source(&env.window.tool, of, &env.window.values)
            .and_then(|p| env.window.inputs.get(&p.name));
    let fields: Vec<(String, usize)> = source.map(|s| s.fields.clone()).unwrap_or_default();
    let rows = source.is_some_and(|s| s.rows);
    let names = kentos_processing::parameters::field_names(multiple, &value);
    let typed = kentos_processing::text::js_trim(&value).to_owned();
    let known = fields.iter().find(|(n, _)| *n == typed);
    let note = if multiple {
        names
            .iter()
            .find(|n| !fields.iter().any(|(f, _)| f == *n))
            .map(|n| format!("“{n}” kaynakta yok."))
            .unwrap_or_default()
    } else if typed.is_empty() {
        String::new()
    } else if let Some((_, count)) = known {
        if rows {
            format!("{count} satırda var.")
        } else if allow_new {
            format!("{count} nesnede var; değeri değişir.")
        } else {
            format!("{count} nesnede var.")
        }
    } else if allow_new {
        "Yeni alan: nesnelere eklenir.".into()
    } else {
        "Bu nesnelerde böyle bir alan yok.".into()
    };
    let name = def.name.clone();
    let menu_fields = fields.clone();
    let chosen = value.clone();
    let menu = move || {
        if menu_fields.is_empty() {
            return Menu::new().item("Bu nesnelerde öznitelik alanı yok", None);
        }
        let names = kentos_processing::parameters::field_names(multiple, &chosen);
        menu_fields.iter().fold(Menu::new(), |m, (f, count)| {
            let on = names.iter().any(|n| n == f);
            let count = if rows {
                format!("{count} satır")
            } else {
                format!("{count} nesne")
            };
            if multiple {
                // A name checked joins the names written, or leaves them.
                let mut next = names.clone();
                if on {
                    next.retain(|n| n != f);
                } else {
                    next.push(f.clone());
                }
                m.check(
                    f.clone(),
                    on,
                    ev(Event::Value(name.clone(), json!(next.join(", ")))),
                )
                .shortcut(count)
            } else {
                m.radio(f.clone(), on, ev(Event::Value(name.clone(), json!(f))))
                    .shortcut(count)
            }
        })
    };
    let open = MenuButton::new(
        container(icon(Icon::ChevronDown).size(14.0).tone(Tone::Muted))
            .padding([6, 8])
            .style(style::container::field_box),
        menu,
    );
    let invalid = env.window.issue_of(&def.name).is_some();
    let combo: Element<'a, Message> = if allow_new {
        let name = def.name.clone();
        row![
            focus_ring(
                input("Alan adı", &value, invalid, ev)
                    .on_input(move |t| ev(Event::Text(name.clone(), t)))
                    .width(Fill)
            ),
            open
        ]
        .spacing(2)
        .align_y(Center)
        .into()
    } else {
        row![
            container(label::body(if value.is_empty() {
                "Alan seçin".to_owned()
            } else {
                value.clone()
            }))
            .padding([5, 8])
            .width(Fill)
            .style(style::container::field_box),
            open
        ]
        .spacing(2)
        .align_y(Center)
        .into()
    };
    let mut out = column![combo].spacing(6).width(Fill);
    if !note.is_empty() {
        out = out.push(label::caption(note).style(style::text::muted));
    }
    out.into()
}

/// A file (docs/adr/0200 §7): its button, its name and what it holds, or
/// that it must be chosen again (the last values keep only its name and path).
fn file<'a>(def: &'a ParamDef, env: &Env<'a, '_>) -> Element<'a, Message> {
    let ev = env.send;
    let v = env.value(&def.name);
    let name = v.get("name").and_then(Value::as_str);
    let table = kentos_processing::parameters::file_table(v);
    let (text, note, button_text, chosen) = match (name, &table) {
        (None, _) => (
            "Dosya seçilmedi".to_owned(),
            String::new(),
            "Dosya seç…",
            false,
        ),
        (Some(n), Some((header, rows))) => (
            n.to_owned(),
            format!(
                "{} satır, {} sütun",
                rows.len(),
                header.iter().filter(|h| !h.is_empty()).count()
            ),
            "Başka dosya…",
            true,
        ),
        (Some(n), None) => (
            n.to_owned(),
            if v.get("rows").is_some() {
                "Dosya boş.".to_owned()
            } else {
                "Yeniden seçin: dosyanın içeriği saklanmaz.".to_owned()
            },
            "Dosya seç…",
            false,
        ),
    };
    let shown = label::body(text);
    let shown = if chosen {
        shown
    } else {
        shown.style(style::text::muted)
    };
    let pick = button(
        row![
            icon(crate::icons::from_web(Some("tableFile"))).size(14.0),
            label::body(button_text)
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([5, 10])
    .style(style::button::secondary)
    .on_press(ev(Event::ChooseFile(def.name.clone())));
    let mut out = column![
        row![container(shown).width(Fill).clip(true), pick]
            .spacing(10)
            .align_y(Center)
    ]
    .spacing(6)
    .width(Fill);
    if !note.is_empty() {
        out = out.push(label::caption(note).style(style::text::muted));
    }
    out.into()
}

/// Fields shown as chips; the rest sit behind a “+n” menu.
const CHIP_FIELDS: usize = 6;

/// How a field is written in an expression: bare when it can be, in
/// brackets otherwise (the web's `fieldToken`).
fn field_token(name: &str) -> String {
    const RESERVED: [&str; 12] = [
        "VE", "VEYA", "DEGIL", "AND", "OR", "NOT", "DOGRU", "YANLIS", "TRUE", "FALSE", "BOS",
        "NULL",
    ];
    let mut chars = name.chars();
    let plain = chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_');
    let folded = kentos_processing::text::fold_turkish(name);
    if plain && !RESERVED.contains(&folded.as_str()) {
        name.to_owned()
    } else {
        format!("[{name}]")
    }
}

/// An expression: one line in mono (typed like the command line), the
/// input's fields as chips, the variables and functions in menus that say
/// what each one does, and a live line on what it gives.
fn expression<'a>(
    def: &'a ParamDef,
    of: Option<&'a str>,
    placeholder: Option<&'a str>,
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    let ev = env.send;
    let value = env.value(&def.name).as_str().unwrap_or("");
    let invalid = env.window.issue_of(&def.name).is_some();
    let name = def.name.clone();
    let line = input(placeholder.unwrap_or(""), value, invalid, ev)
        .on_input(move |t| ev(Event::Text(name.clone(), t)))
        .font(typography::mono())
        .width(Fill);
    // İfade oluşturucu on this field's text (expression/, DESIGN.md §7.16).
    let mut line = row![focus_ring(line)].spacing(6).align_y(Center);
    if env.builder {
        line = line.push(tip(
            button(icon(crate::icons::from_web(Some("expression"))).size(16.0))
                .padding([4, 6])
                .style(style::button::secondary)
                .on_press(Message::Builder(crate::expression::Event::OpenProcessing(
                    def.name.clone(),
                ))),
            Tip::new("İfade oluşturucu…"),
            iced::widget::tooltip::Position::Top,
        ));
    }
    let fields: Vec<(String, usize)> = of
        .and_then(|of| env.window.inputs.get(of))
        .map(|s| s.fields.clone())
        .unwrap_or_default();
    let insert = |text: String| ev(Event::Insert(def.name.clone(), text));
    let mut chips = Row::new().spacing(4).align_y(Center);
    if !fields.is_empty() {
        chips = chips.push(label::caption("Alanlar").style(style::text::muted));
    }
    for (f, count) in fields.iter().take(CHIP_FIELDS) {
        chips = chips.push(tip(
            button(label::caption(f.clone()))
                .padding([3, 8])
                .style(style::button::secondary)
                .on_press(insert(field_token(f))),
            Tip::new(format!("{count} nesnede var; ifadeye ekle")),
            iced::widget::tooltip::Position::Bottom,
        ));
    }
    let rest: Vec<(String, usize)> = fields.iter().skip(CHIP_FIELDS).cloned().collect();
    if !rest.is_empty() {
        let name = def.name.clone();
        let count = rest.len();
        chips = chips.push(MenuButton::new(menu_face(format!("+{count}")), move || {
            rest.iter().fold(Menu::new(), |m, (f, n)| {
                m.item(f.clone(), ev(Event::Insert(name.clone(), field_token(f))))
                    .shortcut(n.to_string())
            })
        }));
    }
    let variables = {
        let name = def.name.clone();
        MenuButton::new(menu_face("Değişkenler".into()), move || {
            VARIABLES.iter().fold(Menu::new(), |m, v| {
                m.item(
                    format!("${}", v.name),
                    ev(Event::Insert(name.clone(), format!("${}", v.name))),
                )
                .detail(v.description)
            })
        })
    };
    let functions = {
        let name = def.name.clone();
        MenuButton::new(menu_face("İşlevler".into()), move || {
            FUNCTIONS.iter().fold(Menu::new(), |m, f| {
                m.item(
                    f.signature,
                    ev(Event::Insert(name.clone(), format!("{}(", f.name))),
                )
                .detail(f.description)
            })
        })
    };
    let bar = row![
        container(chips.wrap()).width(Fill),
        row![variables, functions].spacing(2)
    ]
    .spacing(8)
    .align_y(Alignment::Start);
    let mut out = column![line, bar].spacing(8).width(Fill);
    // The live line, hidden while the field shows a problem (the web's).
    if let (Some(text), false) = (env.window.previews.get(&def.name), invalid) {
        let note = text.contains(" yok.") || text.contains(" boş.");
        out = out.push(
            row![
                icon(if note { Icon::Info } else { Icon::Check })
                    .size(14.0)
                    .tone(if note { Tone::Muted } else { Tone::Success }),
                label::body(text.clone()).style(style::text::muted)
            ]
            .spacing(6)
            .align_y(Alignment::Start),
        );
    }
    out.into()
}

/// A menu's face in the expression bar: its name and a chevron.
fn menu_face<'a>(text: String) -> Element<'a, Message> {
    container(
        row![
            label::caption(text),
            icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted)
        ]
        .spacing(3)
        .align_y(Center),
    )
    .padding([3, 6])
    .into()
}
