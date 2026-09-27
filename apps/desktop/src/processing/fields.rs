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
use kentos_ui::widget::{Menu, MenuButton, Segmented, Switch, Tip, tip};
use serde_json::{Value, json};

use super::Event;
use super::dialog::ToolDialog;
use crate::app::Message;

fn ev(e: Event) -> Message {
    Message::Processing(e)
}

/// What a control reads besides its own parameter.
pub(super) struct Env<'a, 'b> {
    pub window: &'a ToolDialog,
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
        ParamKind::Choice { options } => choice(def, options, env),
        ParamKind::Layer { .. } => layer(def, env),
        ParamKind::Point => point_field(def, env),
        ParamKind::Field { of, allow_new } => field(def, of, *allow_new, env),
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
    parts = parts.push(
        Segmented::new(
            offered.iter().copied().map(ScopeButton),
            ScopeButton(chosen),
            move |s| ev(Event::Scope(name.clone(), s.0.id().into())),
        )
        .width(Fill),
    );
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
        let names: Vec<String> = kinds.iter().map(|k| tr_lower(kind_label(k))).collect();
        parts = parts.push(
            label::caption(format!("Uygun nesneler: {}", names.join(", ")))
                .style(style::text::muted),
        );
    }
    parts.into()
}

fn number<'a>(def: &'a ParamDef, unit: &'a str, env: &Env<'a, '_>) -> Element<'a, Message> {
    let v = env.value(&def.name);
    let typed = env.window.numbers.get(&def.name).cloned();
    let shown = typed.unwrap_or_else(|| v.as_f64().map(js_number).unwrap_or_default());
    let invalid = v.as_f64().is_none_or(|n| !n.is_finite());
    let name = def.name.clone();
    let field = input("", &shown, invalid)
        .on_input(move |t| ev(Event::Number(name.clone(), t)))
        .font(typography::mono())
        .align_x(Alignment::End)
        .width(Length::Fixed(typography::scaled(120.0)));
    let mut line = row![field].spacing(8).align_y(Center);
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
    let value = env.value(&def.name).as_str().unwrap_or("");
    let invalid = env.window.issue_of(&def.name).is_some();
    let name = def.name.clone();
    let field = input(placeholder.unwrap_or(""), value, invalid)
        .on_input(move |t| ev(Event::Text(name.clone(), t)));
    // A one or two letter field is short and centred (the web's `pfield__text--short`).
    if max_length.is_some_and(|m| m <= 2) {
        field
            .align_x(Alignment::Center)
            .width(Length::Fixed(typography::scaled(64.0)))
            .into()
    } else {
        field.width(Fill).into()
    }
}

/// A choice: up to three short options as buttons, else a list with each one's hint.
fn choice<'a>(
    def: &'a ParamDef,
    options: &'a [EnumOption],
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
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
    let exists = leaves
        .iter()
        .any(|l| tr_lower(&l.name) == tr_lower(kentos_processing::text::js_trim(&new_name)));
    let mut choices = vec![
        Choice::new(format!(
            "{new_name} ({})",
            if exists { "mevcut" } else { "yeni" }
        ))
        .icon(Icon::Plus)
        .detail("yeni katman"),
    ];
    let mut ids = vec![String::new()];
    for l in &leaves {
        let locked = layers.is_locked(&l.id);
        let mut c = Choice::new(l.name.clone()).color((env.color)(&l.style.color));
        if locked {
            c = c.detail("kilitli").disabled();
        }
        choices.push(c);
        ids.push(l.id.clone());
    }
    let selected = match &value {
        Some(LayerValue::New(_)) | None => Some(0),
        Some(LayerValue::Existing(id)) => ids.iter().position(|x| x == id),
    };
    let name = def.name.clone();
    let fresh = new_name.clone();
    let list = Select::new(choices, selected, move |i| match i {
        0 => ev(Event::Value(name.clone(), json!({ "newName": fresh }))),
        i => ev(Event::Value(
            name.clone(),
            json!({ "layerId": ids.get(i).cloned().unwrap_or_default() }),
        )),
    });
    if !matches!(value, Some(LayerValue::New(_))) {
        return list.into();
    }
    let name = def.name.clone();
    let field = input("Yeni katmanın adı", &new_name, false)
        .on_input(move |t| ev(Event::LayerName(name.clone(), t)))
        .width(Fill);
    column![list, field].spacing(8).width(Fill).into()
}

fn point_field<'a>(def: &'a ParamDef, env: &Env<'a, '_>) -> Element<'a, Message> {
    let p = point(env.value(&def.name));
    let coord = match p {
        Some(p) => label::mono(env.format.point(kentos_interaction::Vec2::new(p.x, p.y))),
        None => label::body("Henüz gösterilmedi").style(style::text::muted),
    };
    let pick = button(
        row![
            icon(Icon::Magnet).size(14.0),
            label::body(if p.is_some() {
                "Yeniden göster"
            } else {
                "Haritadan göster"
            })
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([5, 10])
    .style(style::button::secondary)
    .on_press(ev(Event::Pick(def.name.clone())));
    row![container(coord).width(Fill), pick]
        .spacing(10)
        .align_y(Center)
        .into()
}

/// An attribute name: typed (a new one too, with `allow_new`) or picked
/// from those the objects have, with a note on what writing it does.
fn field<'a>(
    def: &'a ParamDef,
    of: &'a str,
    allow_new: bool,
    env: &Env<'a, '_>,
) -> Element<'a, Message> {
    let value = env.value(&def.name).as_str().unwrap_or("").to_owned();
    let fields: Vec<(String, usize)> = env
        .window
        .inputs
        .get(of)
        .map(|s| s.fields.clone())
        .unwrap_or_default();
    let typed = kentos_processing::text::js_trim(&value).to_owned();
    let known = fields.iter().find(|(n, _)| *n == typed);
    let note = if typed.is_empty() {
        String::new()
    } else if let Some((_, count)) = known {
        format!("{count} nesnede var; değeri değişir.")
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
        menu_fields.iter().fold(Menu::new(), |m, (f, count)| {
            m.radio(
                f.clone(),
                *f == chosen,
                ev(Event::Value(name.clone(), json!(f))),
            )
            .shortcut(format!("{count} nesne"))
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
            input("Alan adı", &value, invalid)
                .on_input(move |t| ev(Event::Text(name.clone(), t)))
                .width(Fill),
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
    let value = env.value(&def.name).as_str().unwrap_or("");
    let invalid = env.window.issue_of(&def.name).is_some();
    let name = def.name.clone();
    let line = input(placeholder.unwrap_or(""), value, invalid)
        .on_input(move |t| ev(Event::Text(name.clone(), t)))
        .font(typography::mono())
        .width(Fill);
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
