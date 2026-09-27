//! The designer's form parts (the web's `layerForms.ts` helpers `n`,
//! `ddNum`, `ddColor`, `assetPick`, `placementRows` and `designerFields.ts`'s
//! `dataDefined`): each control reads its value from the chosen layer and
//! sends the patch it makes, with the text as typed.

use std::collections::HashMap;

use iced::widget::tooltip::Position;
use iced::widget::{button, column, row};
use iced::{Center, Element, Fill};
use kentos_native_style::designer::{Choice, LayerPath, Patch, js_value, set};
use kentos_expression::js::number;
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::widget::{Tip, tip};
use serde_json::{Value, json};

use super::numbers::{self, Unit};
use super::{Edit, Event, ev, field_id};
use crate::app::Message;
use crate::style::fields::{self, ColorEnv};

/// What the form reads.
pub(super) struct Env<'a> {
    pub at: LayerPath,
    pub layer: &'a Value,
    /// Where the layer sits: an area symbol's line layers may choose rings;
    /// a marker's layers are markers wherever the marker is.
    pub context: &'a str,
    pub typed: &'a HashMap<String, String>,
    pub colors: ColorEnv<'a>,
    /// The library's drawings and pictures: id, name as listed, whether it is an SVG drawing.
    pub assets: &'a [(String, String, bool)],
}

/// A change of the chosen layer.
pub(super) fn edit(at: LayerPath, key: &str, text: Option<String>, patch: Option<Patch>) -> Message {
    ev(Event::Edit(Edit {
        at,
        key: key.to_owned(),
        text,
        patch,
        focus: None,
    }))
}

/// The layer's unit as its sizes are written (`unitOf`).
pub(super) fn unit_of(layer: &Value) -> &'static str {
    match layer.get("unit").and_then(Value::as_str) {
        Some("px") => "px",
        Some("m") => "m",
        _ => "mm",
    }
}

/// An expression value's text and fallback, when the value is one.
fn expr_of(v: Option<&Value>) -> Option<(&str, Option<&Value>)> {
    let o = v?.as_object()?;
    Some((o.get("expr")?.as_str().unwrap_or(""), o.get("fallback")))
}

// ── Numbers ────────────────────────────────────────────────────────────

/// A number field of the layer (`numberInput`), without its label.
pub(super) fn number_control<'a>(env: &Env<'_>, key: &'static str) -> Element<'a, Message> {
    let Some(spec) = numbers::spec_of(env.layer, key) else {
        return iced::widget::space().into();
    };
    let typed = env.typed.get(key);
    let text = typed.cloned().unwrap_or_else(|| {
        numbers::text_of(numbers::shown(env.layer, key))
    });
    let invalid = typed.is_some_and(|t| {
        numbers::parse(t).is_none_or(|v| v < spec.min || v > spec.max)
    });
    let unit = match spec.unit {
        Unit::Layer => Some(unit_of(env.layer)),
        Unit::Fixed(u) => Some(u),
        Unit::None => None,
    };
    let (at, layer) = (env.at, env.layer.clone());
    fields::number(
        field_id(key),
        &text,
        unit,
        invalid,
        move |t| {
            let patch = numbers::parse(&t)
                .map(|v| numbers::write(&layer, key, numbers::clamp(&spec, v)));
            edit(at, key, Some(t), patch)
        },
        ev(Event::Settle(key.to_owned())),
    )
}

/// A labelled number field (`n`).
pub(super) fn number_row<'a>(env: &Env<'_>, key: &'static str) -> Element<'a, Message> {
    let label = numbers::spec_of(env.layer, key).map_or("", |s| s.label);
    fields::labelled(label, number_control(env, key), None)
}

// ── Values that may come from each object's data ──────────────────────

/// An expression field in place of a value, with its ƒ pressed (`dataDefined`
/// switched on): typing keeps the fallback; ƒ goes back to the fallback, or to
/// `plain` when there is none.
fn expression<'a>(env: &Env<'_>, key: &'static str, plain: Value) -> Element<'a, Message> {
    let (text, fallback) = expr_of(env.layer.get(key)).unwrap_or(("", None));
    let fallback = fallback.cloned();
    let at = env.at;
    let back = fallback.clone().unwrap_or(plain);
    let kept = fallback.clone();
    let line = fields::text(
        field_id(&format!("{key}:expr")),
        text,
        "\"Alan\" ya da ifade",
        true,
        move |t| {
            let mut v = json!({ "expr": t });
            if let (Some(o), Some(f)) = (v.as_object_mut(), &kept) {
                o.insert("fallback".into(), f.clone());
            }
            edit(at, key, None, Some(set(key, v)))
        },
    );
    column![
        row![line, fields::fx(true, edit(at, key, None, Some(set(key, back))))]
            .spacing(6)
            .align_y(Center),
        fields::hint("İfade boş sonuç verirse sabit değer kullanılır.")
    ]
    .spacing(4)
    .into()
}

/// The ƒ switch beside a plain value: pressed, the value becomes an
/// expression whose fallback is the value; the expression field takes the keyboard.
fn fx_on<'a>(env: &Env<'_>, key: &'static str, current: Value) -> Element<'a, Message> {
    fields::fx(
        false,
        ev(Event::Edit(Edit {
            at: env.at,
            key: key.to_owned(),
            text: None,
            patch: Some(set(key, json!({ "expr": "", "fallback": current }))),
            focus: Some(format!("{key}:expr")),
        })),
    )
}

/// A number that may come from each object's data (`ddNum`).
pub(super) fn dd_number<'a>(env: &Env<'_>, key: &'static str, fallback: f64) -> Element<'a, Message> {
    let label = numbers::spec_of(env.layer, key).map_or("", |s| s.label);
    let v = env.layer.get(key);
    let control = if expr_of(v).is_some() {
        expression(env, key, js_value(fallback))
    } else {
        let current = v.cloned().unwrap_or_else(|| js_value(fallback));
        row![number_control(env, key), fx_on(env, key, current)]
            .spacing(6)
            .align_y(Center)
            .into()
    };
    fields::labelled(label, control, None)
}

/// A colour that is set as the text says: a valid one is taken (hex upper-cased),
/// an empty one is none where none is allowed; anything else waits.
fn color_patch(key: &str, text: &str, none: bool) -> Option<Patch> {
    let v = kentos_processing::text::js_trim(text);
    if v.is_empty() {
        return none.then(|| set(key, Value::Null));
    }
    fields::is_color(v).then(|| {
        set(
            key,
            Value::from(if v.starts_with('#') {
                v.to_uppercase()
            } else {
                v.to_owned()
            }),
        )
    })
}

/// A colour field of the layer.
fn color_control<'a>(
    env: &Env<'_>,
    key: &'static str,
    label: &str,
    none: bool,
    patch: impl Fn(&str) -> Option<Patch> + 'a,
    value: Option<&str>,
) -> Element<'a, Message> {
    let at = env.at;
    fields::color(
        field_id(key),
        label,
        value,
        env.typed.get(key).map(String::as_str),
        none,
        &env.colors,
        move |t| {
            let p = patch(&t);
            edit(at, key, Some(t), p)
        },
    )
}

/// A colour that may come from each object's data (`ddColor`). Switched back
/// from an expression without a fallback, a colour that must be one is ink
/// (the web's became none, which the layer then refused).
pub(super) fn dd_color<'a>(
    env: &Env<'_>,
    key: &'static str,
    label: &str,
    none: bool,
) -> Element<'a, Message> {
    let v = env.layer.get(key);
    let plain = if none { Value::Null } else { Value::from("ink") };
    let control = if expr_of(v).is_some() {
        expression(env, key, plain)
    } else {
        let current = v.cloned().unwrap_or(Value::Null);
        let value = v.and_then(Value::as_str);
        row![
            color_control(env, key, label, none, move |t| color_patch(key, t, none), value),
            fx_on(env, key, current)
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    };
    fields::labelled(label.to_owned(), control, None)
}

/// A text that may come from each object's data (Metin).
pub(super) fn dd_text<'a>(env: &Env<'_>, key: &'static str, label: &str, hint: &str) -> Element<'a, Message> {
    let v = env.layer.get(key);
    let control = if expr_of(v).is_some() {
        expression(env, key, Value::from(""))
    } else {
        let at = env.at;
        let text = v.and_then(Value::as_str).unwrap_or("");
        let current = v.cloned().unwrap_or_else(|| Value::from(""));
        row![
            fields::text(field_id(key), text, "", false, move |t| {
                edit(at, key, None, Some(set(key, Value::from(t))))
            }),
            fx_on(env, key, current)
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    };
    fields::labelled(label.to_owned(), control, Some(hint))
}

/// Görünür: drawn or not, or by a condition (`dataDefined` of `enabled`).
pub(super) fn dd_enabled<'a>(env: &Env<'_>) -> Element<'a, Message> {
    const KEY: &str = "enabled";
    let v = env.layer.get(KEY);
    let control = if expr_of(v).is_some() {
        expression(env, KEY, Value::Bool(true))
    } else {
        let on = v.and_then(Value::as_bool).unwrap_or(true);
        row![
            iced::widget::container(fields::check(
                Check::from(on),
                "Çizilsin",
                Some(edit(env.at, KEY, None, Some(set(KEY, Value::Bool(!on))))),
            ))
            .width(Fill),
            fx_on(env, KEY, Value::Bool(on))
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    };
    fields::labelled(
        "Görünür",
        control,
        Some("ƒ ile koşula bağlanabilir: ör. [Nitelik] = 'Arsa'"),
    )
}

// ── Choices, switches, dashes ──────────────────────────────────────────

/// A labelled drop-down writing its choice's value as text.
pub(super) fn select_row<'a>(
    env: &Env<'_>,
    key: &'static str,
    label: &str,
    choices: &[Choice],
    default: &str,
    hint: Option<&str>,
) -> Element<'a, Message> {
    let value = env
        .layer
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default);
    let at = env.at;
    fields::labelled(
        label.to_owned(),
        fields::select(choices, value, move |v| {
            edit(at, key, None, Some(set(key, Value::from(v))))
        }),
        hint,
    )
}

/// A labelled check box writing true or false.
pub(super) fn check_row<'a>(
    env: &Env<'_>,
    key: &'static str,
    label: &str,
    words: &str,
    on: bool,
) -> Element<'a, Message> {
    fields::labelled(
        label.to_owned(),
        fields::check(
            Check::from(on),
            words.to_owned(),
            Some(edit(env.at, key, None, Some(set(key, Value::Bool(!on))))),
        ),
        None,
    )
}

/// On and off lengths as “4 1.5” (empty: continuous) (`dashInput`): what
/// reads as up to eight lengths, none negative and one not zero, is taken.
pub(super) fn dash_row<'a>(env: &Env<'_>, hint: &str) -> Element<'a, Message> {
    const KEY: &str = "dash";
    let shown = env
        .layer
        .get(KEY)
        .and_then(Value::as_array)
        .map(|d| {
            d.iter()
                .map(|x| x.as_f64().map_or_else(String::new, number::to_string))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let text = env.typed.get(KEY).cloned().unwrap_or(shown);
    let at = env.at;
    let field = fields::input(Some(field_id(KEY)), "sürekli (ör. 4 1.5)", &text, true, false)
        .on_input(move |t| {
            let parts: Vec<Option<f64>> = t
                .split(|c: char| c.is_whitespace() || c == ';')
                .filter(|s| !s.is_empty())
                .map(numbers::parse)
                .collect();
            let patch = if parts.is_empty() {
                Some(set(KEY, Value::Null))
            } else if parts.iter().all(|p| p.is_some_and(|v| v >= 0.0))
                && parts.iter().any(|p| p.is_some_and(|v| v > 0.0))
            {
                let lengths: Vec<Value> = parts.iter().flatten().take(8).map(|v| js_value(*v)).collect();
                Some(set(KEY, Value::Array(lengths)))
            } else {
                None
            };
            edit(at, KEY, Some(t), patch)
        })
        .on_submit(ev(Event::Settle(KEY.to_owned())));
    fields::labelled("Kesik", field, Some(hint))
}

// ── Pictures and drawings ──────────────────────────────────────────────

/// Why the SVG editor's buttons wait.
const SVG_NOT_YET: &str = "SVG çizim düzenleyicisi web'de var; masaüstüne henüz taşınmadı.";

/// A small button with its tip.
fn small<'a>(text: &str, tip_text: &str, press: Option<Message>) -> Element<'a, Message> {
    tip(
        button(label::body(text.to_owned()))
            .padding([3, 9])
            .style(style::button::secondary)
            .on_press_maybe(press),
        Tip::new(tip_text.to_owned()),
        Position::Top,
    )
}

/// Çizim: a drawing or picture of the library, a new drawing, and one from a file (`assetPick`).
pub(super) fn asset_row<'a>(env: &Env<'_>, key: &'static str) -> Element<'a, Message> {
    let value = env.layer.get(key).and_then(Value::as_str).unwrap_or("");
    let mut choices: Vec<(String, String)> = vec![(String::new(), "Çizim seçin…".to_owned())];
    choices.extend(env.assets.iter().map(|(id, name, _)| (id.clone(), name.clone())));
    let at = env.at;
    let select = fields::select_owned(choices, value, move |id| {
        edit(at, key, None, Some(set(key, Value::from(id))))
    });
    let current_svg = env.assets.iter().any(|(id, _, svg)| id == value && *svg);
    let mut buttons = row![small(
        "Yeni çizim…",
        &format!("SVG çizim düzenleyicisinde yeni bir çizim yapar. {SVG_NOT_YET}"),
        None
    )]
    .spacing(6);
    if current_svg {
        buttons = buttons.push(small(
            "Düzenle…",
            &format!("Seçili çizimi düzenleyicide açar (sistem çiziminin kopyası). {SVG_NOT_YET}"),
            None,
        ));
    }
    buttons = buttons.push(small(
        "Dosya al…",
        "Bilgisayardan SVG, PNG ya da JPEG alır (Kitaplığım'a eklenir)",
        Some(ev(Event::ImportAsset(key.to_owned()))),
    ));
    fields::labelled("Çizim", column![select, buttons].spacing(6), None)
}

/// Where a marker stands (`placementRows`): its turn, its shift and its anchor.
pub(super) fn placement_rows<'a>(env: &Env<'_>) -> Vec<Element<'a, Message>> {
    vec![
        {
            let rotation = dd_number(env, "rotation", 0.0);
            column![
                rotation,
                fields::hint("Saat yönünün tersine; çizgi boyunca işaretlerde çizginin yönüne eklenir.")
            ]
            .spacing(4)
            .into()
        },
        fields::pair(number_row(env, "offsetX"), number_row(env, "offsetY")),
        select_row(
            env,
            "anchor",
            "Çapa",
            &kentos_native_style::designer::ANCHORS,
            "center",
            Some("Noktanın işaretin neresine düştüğü."),
        ),
    ]
}

/// A colour of a layer's own (the halo's), written by `patch`.
pub(super) fn plain_color<'a>(
    env: &Env<'_>,
    key: &'static str,
    label: &str,
    value: Option<&str>,
    patch: impl Fn(Option<String>) -> Patch + 'a,
) -> Element<'a, Message> {
    color_control(
        env,
        key,
        label,
        true,
        move |t| {
            let v = kentos_processing::text::js_trim(t);
            if v.is_empty() {
                Some(patch(None))
            } else if fields::is_color(v) {
                Some(patch(Some(if v.starts_with('#') {
                    v.to_uppercase()
                } else {
                    v.to_owned()
                })))
            } else {
                None
            }
        },
        value,
    )
}

/// A small helper for the icon of a tool.
pub(super) fn glyph(name: &str) -> kentos_ui::icon::Icon {
    crate::icons::from_web(Some(name))
}

/// A tool's face: its icon.
pub(super) fn tool_face<'a>(name: &str) -> Element<'a, Message> {
    icon(glyph(name)).size(15.0).into()
}
