//! Geometri üretici's form and the draw effects (docs/adr/0226 §1, §2; the
//! web's `generatorForm`, `stepFields` and `effectsForm` in `layerForms.ts`):
//! the class of the generator's symbol (whose layers are its child rows), its
//! steps in order with ↑, ↓ and ✕, Adım ekle and the chosen step's values (ƒ:
//! from each object's data); each effect type's switch and, when it is on,
//! its values.

use iced::widget::tooltip::Position;
use iced::widget::{Column, button, container, row};
use iced::{Border, Center, Color, Element, Fill, Theme};
use kentos_native_style::designer::effects::{
    EFFECT_TYPES, effect_label, effect_of, put_effect, set_effect,
};
use kentos_native_style::designer::steps::{
    BUFFER_SIDES, STEP_KINDS, VERTEX_CHOICES, add_step, class_of, move_step, put_step, remove_step,
    set_generator_kind, step_label, step_output, step_summary,
};
use kentos_native_style::designer::{Choice, js_value, set, texts};
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};
use serde_json::{Value, json};

use super::numbers;
use super::parts::{
    Env, edit, expression_block, glyph, number_control, number_row, plain_color, unit_of,
};
use super::{Edit, Event, ev};
use crate::app::Message;
use crate::style::fields;

/// The symbol classes a generator may draw with.
const DRAW_KINDS: [Choice; 3] = [("fill", "Alan"), ("line", "Çizgi"), ("marker", "İşaret")];

fn steps_of(layer: &Value) -> &[Value] {
    layer
        .get("steps")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A change of the steps' list, the step chosen after it.
fn steps_event(changed: &Value, choose: usize) -> Message {
    ev(Event::Steps {
        steps: changed.get("steps").cloned().unwrap_or_else(|| json!([])),
        choose,
    })
}

/// The generator's own rows.
pub(super) fn rows<'a>(env: &Env<'_>) -> Vec<Element<'a, Message>> {
    let l = env.layer;
    let steps = steps_of(l);
    let u = unit_of(l);
    let at = env.at;
    let kind = l
        .get("symbol")
        .and_then(|s| s.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("fill")
        .to_owned();
    let layer = l.clone();
    let draws = fields::labelled(
        texts::DRAWS_WITH,
        fields::select(&DRAW_KINDS, &kind, move |v| {
            let next = set_generator_kind(&layer, v);
            edit(
                at,
                "symbol",
                None,
                Some(set(
                    "symbol",
                    next.get("symbol").cloned().unwrap_or(Value::Null),
                )),
            )
        }),
        Some("Üretilen geometriyi çizen sembol; katmanları listede bu katmanın altındadır."),
    );
    let chosen = env.step.min(steps.len().saturating_sub(1));
    let list: Element<'a, Message> = if steps.is_empty() {
        fields::hint(texts::NO_STEPS)
    } else {
        Column::with_children(
            steps
                .iter()
                .enumerate()
                .map(|(i, st)| step_row(l, i, st, i == chosen, steps.len(), u)),
        )
        .spacing(2)
        .into()
    };
    let n = steps.len();
    let layer = l.clone();
    let add = MenuButton::new(
        container(
            row![
                icon(glyph("plus")).size(13.0),
                label::body(texts::ADD_STEP),
                icon(glyph("chevronDown")).size(11.0)
            ]
            .spacing(5)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::container::field_box),
        move || {
            let mut m = Menu::new();
            for k in STEP_KINDS {
                m = m
                    .item(
                        step_label(k).unwrap_or(k),
                        steps_event(&add_step(&layer, k), n),
                    )
                    .icon(crate::icons::from_web(Some(step_icon(k))));
            }
            m
        },
    );
    let mut out: Vec<Element<'a, Message>> = vec![
        draws,
        fields::group_title(texts::STEPS),
        list,
        container(add).into(),
    ];
    let input = class_of(env.context);
    if step_output(steps, input).is_none() {
        out.push(
            label::caption(texts::NOTHING_LEFT)
                .style(|t: &Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(t).warning),
                })
                .into(),
        );
    }
    if let Some(st) = steps.get(chosen) {
        let kind = st.get("kind").and_then(Value::as_str).unwrap_or("");
        let mut form: Vec<Element<'a, Message>> = vec![fields::group_title(&format!(
            "{}. {}",
            chosen + 1,
            step_label(kind).unwrap_or(kind)
        ))];
        form.extend(step_fields(env, chosen, st));
        out.push(
            container(Column::with_children(form).spacing(8))
                .padding(iced::Padding {
                    left: 10.0,
                    top: 6.0,
                    ..iced::Padding::ZERO
                })
                .into(),
        );
    }
    out
}

/// A step's row: its number and line, ↑, ↓ and ✕; pressed, the step is chosen.
fn step_row<'a>(
    l: &Value,
    i: usize,
    st: &Value,
    chosen: bool,
    n: usize,
    u: &str,
) -> Element<'a, Message> {
    let tool = |name: &str, words: &str, press: Option<Message>| -> Element<'a, Message> {
        tip(
            button(icon(glyph(name)).size(13.0))
                .padding([2, 4])
                .style(style::button::ghost)
                .on_press_maybe(press),
            Tip::new(words.to_owned()),
            Position::Top,
        )
    };
    let up = (i > 0)
        .then(|| move_step(l, i, -1).map(|s| steps_event(&s, i - 1)))
        .flatten();
    let down = (i + 1 < n)
        .then(|| move_step(l, i, 1).map(|s| steps_event(&s, i + 1)))
        .flatten();
    let gone = steps_event(&remove_step(l, i), i.min(n.saturating_sub(2)));
    let face = row![
        label::caption(format!("{}.", i + 1)).style(style::text::muted),
        label::body(step_summary(st, u)).width(Fill),
        tool("chevronUp", "Yukarı", up),
        tool("chevronDown", "Aşağı", down),
        tool("trash", "Adımı sil", Some(gone)),
    ]
    .spacing(6)
    .align_y(Center);
    button(face)
        .on_press(ev(Event::ChooseStep(i)))
        .padding([2, 4])
        .width(Fill)
        .style(move |t: &Theme, status| {
            let tk = Tokens::of(t);
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let (background, edge) = if chosen {
                (tk.accent.scale_alpha(0.14), tk.accent.scale_alpha(0.55))
            } else if hovered {
                (tk.surface_hover, Color::TRANSPARENT)
            } else {
                (Color::TRANSPARENT, Color::TRANSPARENT)
            };
            button::Style {
                background: Some(iced::Background::Color(background)),
                text_color: tk.text,
                border: Border {
                    color: edge,
                    width: 1.0,
                    radius: kentos_ui::theme::shape::radius(4.0).into(),
                },
                ..button::Style::default()
            }
        })
        .into()
}

/// An expression value's text and fallback, when the value is one.
fn expr_of(v: Option<&Value>) -> Option<(&str, Option<&Value>)> {
    let o = v?.as_object()?;
    Some((o.get("expr")?.as_str().unwrap_or(""), o.get("fallback")))
}

/// A step's number that may come from each object's data (`dataDefined`): its field, or its expression with ƒ pressed.
/// A step's icon in Adım ekle's menu (the web's `STEP_ICON`): the app's icon of the same operation.
fn step_icon(kind: &str) -> &'static str {
    match kind {
        "centroid" => "snapCentroid",
        "pointOnSurface" => "featureToPoint",
        "vertices" => "verticesToPoints",
        "boundary" => "polygonToLine",
        "offset" => "offset",
        "extend" => "extend",
        "buffer" => "geoBuffer",
        "envelope" => "stepEnvelope",
        "hull" => "boundingGeometry",
        "circle" => "circle",
        "wedge" => "sector",
        "polygon" => "regularPolygon",
        "translate" => "move",
        "rotate" => "rotate",
        "scale" => "scale",
        _ => "geometryGenerator",
    }
}

fn dd<'a>(env: &Env<'_>, i: usize, field: &str, fallback: f64) -> Element<'a, Message> {
    let key = format!("step.{i}.{field}");
    let label = numbers::spec_of(env.layer, &key).map_or("", |s| s.label);
    let st = steps_of(env.layer)
        .get(i)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let v = st.get(field).cloned();
    let at = env.at;
    let layer = env.layer.clone();
    let name = field.to_owned();
    let with = move |value: Value| {
        let mut step = st.clone();
        if let Some(o) = step.as_object_mut() {
            o.insert(name.clone(), value);
        }
        let next = put_step(&layer, i, step);
        set("steps", next.get("steps").cloned().unwrap_or(Value::Null))
    };
    let v = v.as_ref();
    let control: Element<'a, Message> = if let Some((text, fb)) = expr_of(v) {
        let fallback_value = fb.cloned();
        let back = fallback_value.clone().unwrap_or_else(|| js_value(fallback));
        let with_line = with.clone();
        let line_key = key.clone();
        expression_block(
            env,
            &key,
            text,
            move |t| {
                let mut e = json!({ "expr": t });
                if let (Some(o), Some(f)) = (e.as_object_mut(), &fallback_value) {
                    o.insert("fallback".into(), f.clone());
                }
                edit(at, &line_key, None, Some(with_line(e)))
            },
            fields::fx(true, edit(at, &key, None, Some(with(back)))),
        )
    } else {
        let current = v
            .cloned()
            .unwrap_or_else(|| js_value(numbers::shown(env.layer, &key)));
        let fx = ev(Event::Edit(Edit {
            at,
            key: key.clone(),
            text: None,
            patch: Some(with(json!({ "expr": "", "fallback": current }))),
            focus: Some(format!("{key}:expr")),
        }));
        row![number_control(env, &key), fields::fx(false, fx)]
            .spacing(6)
            .align_y(Center)
            .into()
    };
    fields::labelled(label.to_owned(), control, None)
}

/// A step's choice (Köşeler, Yan), written into the step.
fn step_select<'a>(
    env: &Env<'_>,
    i: usize,
    field: &'static str,
    label: &str,
    choices: &[Choice],
    default: &str,
    hint: Option<&str>,
) -> Element<'a, Message> {
    let st = steps_of(env.layer)
        .get(i)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let value = st
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_owned();
    let (at, layer) = (env.at, env.layer.clone());
    fields::labelled(
        label.to_owned(),
        fields::select(choices, &value, move |v| {
            let mut step = st.clone();
            if let Some(o) = step.as_object_mut() {
                o.insert(field.into(), Value::from(v));
            }
            let next = put_step(&layer, i, step);
            edit(
                at,
                &format!("step.{i}.{field}"),
                None,
                Some(set(
                    "steps",
                    next.get("steps").cloned().unwrap_or(Value::Null),
                )),
            )
        }),
        hint,
    )
}

/// The chosen step's own values (`stepFields`).
fn step_fields<'a>(env: &Env<'_>, i: usize, st: &Value) -> Vec<Element<'a, Message>> {
    match st.get("kind").and_then(Value::as_str).unwrap_or("") {
        "vertices" => vec![step_select(
            env,
            i,
            "which",
            "Köşeler",
            &VERTEX_CHOICES,
            "all",
            None,
        )],
        "offset" => vec![
            dd(env, i, "distance", 1.0),
            fields::hint("Artı çizginin soluna, alanın içine."),
        ],
        "extend" => vec![fields::pair(
            dd(env, i, "start", 0.0),
            dd(env, i, "end", 0.0),
        )],
        "buffer" => vec![
            dd(env, i, "distance", 2.0),
            step_select(
                env,
                i,
                "side",
                "Yan",
                &BUFFER_SIDES,
                "both",
                Some("Eksi uzaklık alanı içe küçültür; yan yalnız çizgilerde."),
            ),
        ],
        "circle" => vec![dd(env, i, "radius", 2.0)],
        "wedge" => vec![
            fields::pair(dd(env, i, "direction", 0.0), dd(env, i, "width", 60.0)),
            fields::pair(dd(env, i, "radius", 5.0), dd(env, i, "inner", 0.0)),
            fields::hint("Yön kuzeyden saat yönünde (ör. fotoğrafın Yön özniteliği: ƒ ile [Yön])."),
        ],
        "polygon" => vec![
            fields::pair(dd(env, i, "sides", 6.0), dd(env, i, "radius", 2.0)),
            dd(env, i, "rotation", 0.0),
            fields::hint("İlk köşe kuzeyde; döndürme saat yönünün tersine."),
        ],
        "translate" => vec![fields::pair(dd(env, i, "dx", 0.0), dd(env, i, "dy", 0.0))],
        "rotate" => vec![
            dd(env, i, "angle", 45.0),
            fields::hint("Saat yönünün tersine, ağırlık merkezinde."),
        ],
        "scale" => vec![
            fields::pair(dd(env, i, "x", 1.0), dd(env, i, "y", 1.0)),
            fields::hint("Ağırlık merkezinde; biri boşsa öbürü."),
        ],
        _ => vec![fields::hint("Bu adımın değeri yok.")],
    }
}

/// The layer's draw effects (`effectsForm`): each type's switch, its values when it is on.
pub(super) fn effects<'a>(env: &Env<'_>) -> Element<'a, Message> {
    let l = env.layer;
    let mut rows: Vec<Element<'a, Message>> = vec![fields::group_title(texts::EFFECTS)];
    for kind in EFFECT_TYPES {
        let e = effect_of(l, kind);
        let on = e.is_some_and(|e| e.get("enabled") != Some(&Value::Bool(false)));
        let toggle = edit(
            env.at,
            &format!("effects:{kind}"),
            None,
            Some(numbers::effects_patch(&set_effect(l, kind, !on))),
        );
        rows.push(fields::check(
            Check::from(on),
            effect_label(kind),
            Some(toggle),
        ));
        let Some(e) = e.filter(|_| on) else {
            continue;
        };
        let mut own: Vec<Element<'a, Message>> = Vec::new();
        let n = |field: &str| number_row(env, &format!("fx.{kind}.{field}"));
        let color = {
            let value = e.get("color").and_then(Value::as_str).map(str::to_owned);
            let (layer, effect) = (l.clone(), e.clone());
            plain_color(
                env,
                &format!("fx.{kind}.color"),
                "Renk",
                value.as_deref(),
                move |c| {
                    let mut next = effect.clone();
                    if let (Some(o), Some(c)) = (next.as_object_mut(), c) {
                        o.insert("color".into(), Value::from(c));
                    }
                    numbers::effects_patch(&put_effect(&layer, next))
                },
            )
        };
        match kind {
            "dropShadow" | "innerShadow" => {
                own.push(fields::pair(n("distance"), n("angle")));
                own.push(n("blur"));
                own.push(fields::pair(
                    fields::labelled("Renk", color, None),
                    n("transparency"),
                ));
            }
            "outerGlow" | "innerGlow" => {
                own.push(fields::pair(n("spread"), n("blur")));
                own.push(fields::pair(
                    fields::labelled("Renk", color, None),
                    n("transparency"),
                ));
            }
            _ => own.push(n("blur")),
        }
        rows.push(
            container(Column::with_children(own).spacing(8))
                .padding(iced::Padding {
                    left: 10.0,
                    ..iced::Padding::ZERO
                })
                .into(),
        );
    }
    Column::with_children(rows).spacing(8).width(Fill).into()
}
