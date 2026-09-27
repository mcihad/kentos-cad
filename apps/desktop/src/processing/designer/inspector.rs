//! Model tasarımcısı's settings, the right column (the web's
//! modelInspector): the model's name, category, outputs and problems when
//! nothing is selected; an input's definition; or a step's parameters, each
//! fed by the tool's default, a fixed value (the tool window's own
//! control), a model input or an earlier step's output.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space, text_input};
use iced::{Alignment, Center, Element, Fill, Length};
use kentos_interaction::Format;
use kentos_processing::categories::Category;
use kentos_processing::designer::{self as plan, texts::inspector as words};
use kentos_processing::features::kind_label;
use kentos_processing::model::{ModelInput, ModelIssue, step_name};
use kentos_processing::model_edit::{INPUT_TYPES, NodeRef, input_type_for, sources_for};
use kentos_processing::parameters::is_visible;
use kentos_processing::text::js_number;
use kentos_processing::{ParamDef, Registry, Tool, ValueSource};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Menu, MenuButton, Segmented, Switch, Tip, horizontal_divider, tip};
use serde_json::Value;

use super::update::{InputField, SourceChoice};
use super::{Designer, Event};
use crate::app::Message;
use crate::processing::fields::{Env, control};

fn ev(e: Event) -> Message {
    Message::ModelDesigner(e)
}

/// The settings' width (the web's 340 px, with the type size).
pub(super) const WIDTH: f32 = 340.0;

/// The object kinds a features input may take, in the web's order.
const KINDS: [&str; 7] = [
    "polygon", "polyline", "line", "point", "circle", "arc", "text",
];

/// What the settings read besides the designer: the tools, and for a
/// step's fixed values the drawing, its number format and layer colours
/// (read while the controls are built, as the tool window's `Env`).
pub(super) struct Context<'a, 'b> {
    pub registry: &'a Registry,
    pub doc: Option<&'a kentos_domain::Document>,
    pub format: &'b Format,
    pub color: &'b dyn Fn(&str) -> iced::Color,
}

pub(super) fn inspector<'a>(d: &'a Designer, cx: &Context<'a, '_>) -> Element<'a, Message> {
    let body: Element<'a, Message> = match &d.selected {
        Some(NodeRef::Input(name)) => match d.draft.inputs.iter().find(|i| i.name() == name) {
            Some(input) => input_settings(d, input, cx.registry),
            None => model_settings(d, cx.registry),
        },
        Some(NodeRef::Step(id)) if d.draft.steps.iter().any(|s| &s.id == id) => {
            step_settings(d, id, cx)
        }
        _ => model_settings(d, cx.registry),
    };
    scrollable(container(body).padding(iced::Padding::new(16.0).bottom(20.0)))
        .direction(style::field::body_scrollbar())
        .height(Fill)
        .into()
}

/// The head: an icon in its box (an input's in the info blue), the kind
/// and a line under it.
fn head<'a>(glyph: Icon, input: bool, kind: String, lead: String) -> Element<'a, Message> {
    let mark = container(icon(glyph).size(18.0))
        .center_x(Length::Fixed(typography::scaled(34.0)))
        .center_y(Length::Fixed(typography::scaled(34.0)))
        .style(move |theme: &iced::Theme| {
            let t = Tokens::of(theme);
            container::Style {
                text_color: Some(if input { t.info } else { t.muted }),
                ..style::container::field_box(theme)
            }
        });
    row![
        mark,
        column![
            label::body(kind).font(typography::ui_strong()),
            label::caption(lead).style(style::text::muted),
        ]
        .spacing(2)
        .width(Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Start)
    .into()
}

/// A section: an optional title (small, bold, second tone, a line under
/// it) and its rows.
fn section<'a>(title: Option<String>, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut out = Column::new().spacing(10);
    if let Some(title) = title {
        out = out.push(
            column![
                label::caption(title)
                    .font(typography::ui_strong())
                    .style(style::text::muted),
                horizontal_divider(),
            ]
            .spacing(6),
        );
    }
    out.extend(rows).into()
}

/// A labelled row: the label over its control, a note under it.
fn field_row<'a>(
    title: &'a str,
    control: Element<'a, Message>,
    note: Option<String>,
) -> Element<'a, Message> {
    let mut out = column![label::body(title).font(typography::ui_strong()), control].spacing(6);
    if let Some(note) = note {
        out = out.push(label::caption(note).style(style::text::muted));
    }
    out.into()
}

fn text_field<'a>(
    placeholder: &str,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .padding([5, 8])
        .font(typography::ui())
        .size(typography::body())
        .style(style::field::validated(false))
        .into()
}

/// A list's face, as the tool window's lists look: its text and a
/// chevron; blue edged while the value is `linked` to an input or a step.
fn list_face<'a>(text: String, linked: bool) -> Element<'a, Message> {
    container(
        row![
            label::body(text)
                .wrapping(iced::widget::text::Wrapping::None)
                .width(Fill),
            icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([3, 8])
    .width(Fill)
    .clip(true)
    .style(move |theme: &iced::Theme| {
        let mut s = style::container::field_box(theme);
        if linked {
            s.border.color = Tokens::of(theme).info;
        }
        s
    })
    .into()
}

/// A section's removing button (Girdiyi sil, Adımı sil, Modeli sil).
fn danger<'a>(text: &'a str, on_press: Message) -> Element<'a, Message> {
    button(
        row![
            icon(crate::icons::from_web(Some("erase")))
                .size(14.0)
                .tone(Tone::Danger),
            label::body(text).style(style::text::danger)
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([5, 6])
    .style(style::button::ghost)
    .on_press(on_press)
    .into()
}

/// The problems: of the model (each opens its step), or of one step; with
/// none, the model says it is ready (or how to begin).
fn problems<'a>(
    d: &'a Designer,
    step: Option<&str>,
    lookup: &dyn Fn(&str) -> Option<Tool>,
) -> Option<Element<'a, Message>> {
    let list: Vec<&ModelIssue> = d
        .problems
        .iter()
        .filter(|p| step.is_none() || p.step.as_deref() == step)
        .collect();
    if list.is_empty() {
        if step.is_some() {
            return None;
        }
        let text = if d.draft.steps.is_empty() {
            words::problems::START
        } else {
            words::problems::READY
        };
        return Some(
            row![
                icon(Icon::Check).size(14.0).tone(Tone::Success),
                label::body(text).style(style::text::muted).width(Fill)
            ]
            .spacing(6)
            .align_y(Alignment::Start)
            .into(),
        );
    }
    let title = if step.is_some() {
        words::problems::TITLE.to_owned()
    } else {
        words::problems::title_count(list.len())
    };
    let rows = list
        .into_iter()
        .map(|p| {
            let owner = p
                .step
                .as_ref()
                .and_then(|id| d.draft.steps.iter().find(|s| &s.id == id));
            let text = match (owner, step) {
                (Some(s), None) => words::problems::of_step(&step_name(s, lookup), &p.message),
                _ => p.message.clone(),
            };
            let face = row![
                icon(Icon::Warning).size(14.0).tone(Tone::Warning),
                label::caption(text).width(Fill),
            ]
            .spacing(6)
            .align_y(Alignment::Start);
            let b = button(face)
                .padding([5, 0])
                .width(Fill)
                .style(style::button::ghost);
            match (owner, step) {
                (Some(s), None) => b.on_press(ev(Event::Select(Some(NodeRef::Step(s.id.clone()))))),
                _ => b,
            }
            .into()
        })
        .collect();
    Some(section(Some(title), rows))
}

fn model_settings<'a>(d: &'a Designer, registry: &'a Registry) -> Element<'a, Message> {
    let lookup = |id: &str| registry.tool(id);
    let model = &d.draft;
    // The toolbox's categories, and the model's own when it is not among them.
    let mut categories: Vec<Category> = registry
        .tree(&|_| true)
        .iter()
        .map(|n| n.category)
        .collect();
    if let Some(c) = registry.category(&model.category)
        && !categories.iter().any(|x| x.id == c.id)
    {
        categories.push(*c);
    }
    let chosen = model.category.clone();
    let shown = registry
        .category(&model.category)
        .map_or_else(|| model.category.clone(), |c| c.label.to_owned());
    let category = MenuButton::new(list_face(shown, false), move || {
        categories.iter().fold(Menu::new(), |m, c| {
            m.radio(
                c.label,
                c.id == chosen,
                ev(Event::Category(c.id.to_owned())),
            )
            .icon(crate::icons::from_web(Some(c.icon)))
        })
    });

    let mut outputs: Vec<Element<'a, Message>> = Vec::new();
    for (i, o) in model.outputs.iter().enumerate() {
        let step = model.steps.iter().find(|s| s.id == o.step);
        let out_label = step
            .and_then(|s| lookup(&s.tool))
            .and_then(|t| t.outputs.into_iter().find(|x| x.name == o.output))
            .map_or_else(|| o.output.clone(), |x| x.label);
        let from = match step {
            Some(s) => words::model::output(&step_name(s, &lookup), &out_label),
            None => words::model::NO_STEP.to_owned(),
        };
        outputs.push(
            column![
                row![
                    column![
                        label::body(o.label.clone()).font(typography::ui_strong()),
                        label::caption(from).style(style::text::muted),
                    ]
                    .spacing(2)
                    .width(Fill),
                    tip(
                        button(icon(Icon::Close).size(14.0))
                            .padding(4)
                            .style(style::button::ghost)
                            .on_press(ev(Event::RemoveOutput(i))),
                        Tip::new(words::model::remove_output(&o.label)),
                        iced::widget::tooltip::Position::Left,
                    ),
                ]
                .spacing(8)
                .align_y(Center),
                horizontal_divider(),
            ]
            .spacing(6)
            .into(),
        );
    }
    let candidates: Vec<(String, String, String, String)> = model
        .steps
        .iter()
        .flat_map(|s| {
            let name = step_name(s, &lookup);
            lookup(&s.tool)
                .map(|t| t.outputs)
                .unwrap_or_default()
                .into_iter()
                .filter(|o| {
                    !model
                        .outputs
                        .iter()
                        .any(|x| x.step == s.id && x.output == o.name)
                })
                .map(|o| (s.id.clone(), o.name, o.label, name.clone()))
                .collect::<Vec<_>>()
        })
        .collect();
    outputs.push(
        MenuButton::new(
            list_face(words::model::ADD_OUTPUT.to_owned(), false),
            move || {
                if candidates.is_empty() {
                    return Menu::new().item(words::model::NO_OUTPUT, None::<Message>);
                }
                candidates
                    .iter()
                    .fold(Menu::new(), |m, (step, output, label, name)| {
                        m.item(
                            label.clone(),
                            ev(Event::AddOutput {
                                step: step.clone(),
                                output: output.clone(),
                            }),
                        )
                        .detail(name.clone())
                    })
            },
        )
        .into(),
    );

    let mut out = column![
        head(
            crate::icons::from_web(Some("processing")),
            false,
            words::model::KIND.to_owned(),
            words::model::LEAD.to_owned()
        ),
        section(
            None,
            vec![
                field_row(
                    words::model::NAME,
                    text_field("", &model.label, |t| ev(Event::Label(t))),
                    None
                ),
                field_row(words::model::CATEGORY, category.into(), None),
                field_row(
                    words::model::DESCRIPTION,
                    text_field(words::model::DESCRIPTION_HINT, &model.description, |t| {
                        ev(Event::Description(t))
                    }),
                    None
                ),
            ]
        ),
        section(Some(words::model::OUTPUTS.to_owned()), outputs),
    ]
    .spacing(18);
    if let Some(p) = problems(d, None, &lookup) {
        out = out.push(p);
    }
    if d.is_saved(registry) && !d.builtin_copy {
        out = out.push(danger(words::model::REMOVE, ev(Event::Delete)));
    }
    out.into()
}

fn input_settings<'a>(
    d: &'a Designer,
    input: &'a ModelInput,
    registry: &'a Registry,
) -> Element<'a, Message> {
    let lookup = |id: &str| registry.tool(id);
    let name = input.name().to_owned();
    let kind = INPUT_TYPES
        .iter()
        .find(|t| t.type_name == input.type_name());
    let def = &input.0;
    let text_of = |key: &str| {
        def.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let flag = |key: &str| def.get(key).and_then(Value::as_bool) == Some(true);
    let set = |field: fn(String) -> InputField| {
        let name = name.clone();
        move |t: String| ev(Event::Input(name.clone(), field(t)))
    };
    let toggle = |field: fn(bool) -> InputField| {
        let name = name.clone();
        move |on: bool| ev(Event::Input(name.clone(), field(on)))
    };

    let mut rows: Vec<Element<'a, Message>> = vec![
        field_row(
            words::input::LABEL,
            text_field("", input.label(), set(InputField::Label)),
            Some(words::input::variable(&name)),
        ),
        field_row(
            words::input::DESCRIPTION,
            text_field(
                words::input::DESCRIPTION_HINT,
                &text_of("description"),
                set(InputField::Description),
            ),
            None,
        ),
        field_row(
            words::input::OPTIONAL,
            Switch::new(flag("optional"), toggle(InputField::Optional)).into(),
            None,
        ),
    ];
    match input.type_name() {
        "features" => {
            let scope = def
                .get("default")
                .and_then(|v| v.get("scope"))
                .and_then(Value::as_str)
                .filter(|s| matches!(*s, "selection" | "visible" | "all"))
                .unwrap_or("selection");
            let scopes = words::input::SCOPES;
            let chosen = scopes
                .iter()
                .find(|(id, _)| *id == scope)
                .map_or(scopes[0].1, |(_, l)| *l);
            let n = name.clone();
            rows.push(field_row(
                words::input::DEFAULT,
                Segmented::new(scopes.iter().map(|(_, l)| *l), chosen, move |l| {
                    let id = scopes
                        .iter()
                        .find(|(_, x)| *x == l)
                        .map_or("selection", |(id, _)| *id);
                    ev(Event::Input(n.clone(), InputField::Scope(id.to_owned())))
                })
                .width(Fill)
                .into(),
                None,
            ));
            let kinds: Vec<&str> = def
                .get("kinds")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let mut chips = Row::new().spacing(4).align_y(Center);
            for id in KINDS {
                let on = kinds.contains(&id);
                let mut face = Row::new().spacing(5).align_y(Center);
                if on {
                    face = face.push(icon(Icon::Check).size(12.0).tone(Tone::Accent));
                }
                face = face.push(label::caption(kind_label(id)));
                chips = chips.push(
                    button(face)
                        .padding([3, 9])
                        .style(style::button::toggle(on))
                        .on_press(ev(Event::Input(
                            name.clone(),
                            InputField::Kind(id.to_owned()),
                        ))),
                );
            }
            rows.push(field_row(
                words::input::KINDS,
                chips.wrap().into(),
                kinds
                    .is_empty()
                    .then(|| words::input::KINDS_NOTE.to_owned()),
            ));
        }
        "number" => {
            for (key, title, optional) in [
                ("default", words::input::DEFAULT, false),
                ("min", words::input::MIN, true),
                ("max", words::input::MAX, true),
            ] {
                // What was typed stays as typed ("1." on the way to "1.5").
                let shown = d
                    .typed
                    .get(&format!("{name}.{key}"))
                    .cloned()
                    .unwrap_or_else(|| match def.get(key).and_then(Value::as_f64) {
                        Some(n) => js_number(n),
                        None if !optional => "0".to_owned(),
                        None => String::new(),
                    });
                let n = name.clone();
                rows.push(field_row(
                    title,
                    text_input(if optional { words::input::NONE } else { "" }, &shown)
                        .on_input(move |t| ev(Event::Input(n.clone(), InputField::Number(key, t))))
                        .padding([5, 8])
                        .font(typography::mono())
                        .size(typography::body())
                        .style(style::field::validated(false))
                        .into(),
                    None,
                ));
            }
            rows.push(field_row(
                words::input::INTEGER,
                Switch::new(flag("integer"), toggle(InputField::Integer)).into(),
                None,
            ));
        }
        "string" => {
            let text = match def.get("default") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Null) | None => String::new(),
                Some(v) => v.to_string(),
            };
            rows.push(field_row(
                words::input::DEFAULT,
                text_field("", &text, set(InputField::Text)),
                None,
            ));
            rows.push(field_row(
                words::input::ALLOW_EMPTY,
                Switch::new(flag("allowEmpty"), toggle(InputField::AllowEmpty)).into(),
                None,
            ));
        }
        "boolean" => {
            // The web's `!!d.default`.
            let on = def.get("default").is_some_and(|v| match v {
                Value::Bool(b) => *b,
                Value::Null => false,
                Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
                Value::String(s) => !s.is_empty(),
                _ => true,
            });
            rows.push(field_row(
                words::input::DEFAULT,
                Switch::new(on, toggle(InputField::Flag)).into(),
                None,
            ));
        }
        "layer" => {
            let fresh = def
                .get("default")
                .and_then(|v| v.get("newName"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            rows.push(field_row(
                words::input::NEW_LAYER,
                text_field("", &fresh, set(InputField::NewLayer)),
                Some(words::input::NEW_LAYER_NOTE.to_owned()),
            ));
        }
        "point" => rows.push(
            label::caption(words::input::POINT_NOTE)
                .style(style::text::muted)
                .into(),
        ),
        _ => {}
    }

    let users: Vec<Element<'a, Message>> = d
        .draft
        .steps
        .iter()
        .filter(|s| {
            s.values
                .iter()
                .any(|(_, v)| matches!(v, ValueSource::Input(n) if *n == name))
        })
        .map(|s| {
            let glyph = lookup(&s.tool).and_then(|t| t.icon);
            button(
                row![
                    icon(crate::icons::from_web(
                        glyph.as_deref().or(Some("processing"))
                    ))
                    .size(14.0)
                    .tone(Tone::Muted),
                    label::body(step_name(s, &lookup)),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([5, 0])
            .width(Fill)
            .style(style::button::ghost)
            .on_press(ev(Event::Select(Some(NodeRef::Step(s.id.clone())))))
            .into()
        })
        .collect();
    let users = if users.is_empty() {
        vec![
            label::caption(words::input::NO_USERS)
                .style(style::text::muted)
                .into(),
        ]
    } else {
        users
    };

    column![
        head(
            crate::icons::from_web(Some(kind.map_or("processing", |k| k.icon))),
            true,
            words::input::kind(kind.map_or(input.type_name(), |k| k.label)),
            words::input::LEAD.to_owned()
        ),
        section(None, rows),
        section(Some(words::input::USERS.to_owned()), users),
        danger(words::input::REMOVE, ev(Event::RemoveInput(name.clone()))),
    ]
    .spacing(18)
    .into()
}

fn step_settings<'a>(d: &'a Designer, id: &'a str, cx: &Context<'a, '_>) -> Element<'a, Message> {
    let registry = cx.registry;
    let lookup = |x: &str| registry.tool(x);
    let Some(step) = d.draft.steps.iter().find(|s| s.id == id) else {
        return space().into();
    };
    let Some(tool) = lookup(&step.tool) else {
        return section(
            Some(words::step::UNKNOWN.to_owned()),
            vec![
                label::caption(words::step::unknown_note(&step.tool))
                    .style(style::text::muted)
                    .into(),
            ],
        );
    };
    // The controls read the fixed values and the tool's defaults, as the tool window holds them.
    let fields = d.fields.as_ref().filter(|f| f.tool.id == tool.id);
    let env = fields.zip(cx.doc).map(|(window, doc)| Env {
        window,
        send: |e| Message::ModelDesigner(Event::Field(e)),
        pick_objects: false,
        builder: false,
        doc,
        format: cx.format,
        color: cx.color,
    });
    let param_row = |p: &'a ParamDef| -> Element<'a, Message> {
        let src = step.source(&p.name);
        let options = sources_for(&d.draft, id, p, &lookup);
        let as_input = input_type_for(p).is_some();
        let name = p.name.clone();
        let text = plan::source_text(&d.draft, src, &lookup);
        let linked = matches!(
            src,
            Some(ValueSource::Input(_) | ValueSource::Output { .. })
        );
        let current = src.cloned();
        let list = MenuButton::new(list_face(text, linked), move || {
            let choose = |choice: SourceChoice| {
                ev(Event::Source {
                    param: name.clone(),
                    choice,
                })
            };
            let mut m = Menu::new()
                .radio(
                    words::step::TOOL_DEFAULT,
                    current.is_none(),
                    choose(SourceChoice::ToolDefault),
                )
                .radio(
                    words::step::FIXED,
                    matches!(current, Some(ValueSource::Value(_))),
                    choose(SourceChoice::Fixed),
                );
            if !options.is_empty() {
                m = m.separator();
            }
            for o in &options {
                let detail = if o.group == "Girdi" {
                    words::step::MODEL_INPUT.to_owned()
                } else {
                    o.group.clone()
                };
                m = m
                    .radio(
                        o.label.clone(),
                        current.as_ref() == Some(&o.src),
                        choose(SourceChoice::From(o.src.clone())),
                    )
                    .detail(detail);
            }
            if as_input {
                m = m
                    .separator()
                    .item(words::step::AS_INPUT, choose(SourceChoice::AsInput))
                    .icon(crate::icons::from_web(Some("modelNew")))
                    .detail(words::step::AS_INPUT_NOTE);
            }
            m
        });
        let mut title = Row::new()
            .spacing(8)
            .align_y(Alignment::End)
            .push(label::body(p.label.clone()).font(typography::ui_strong()));
        if p.optional {
            title = title.push(label::caption(words::step::OPTIONAL).style(style::text::muted));
        }
        let mut out = column![title, list].spacing(6);
        if let (Some(ValueSource::Value(_)), Some(env)) = (src, &env) {
            out = out.push(control(p, env));
        }
        out.into()
    };
    // Shown: what has a source, or what is visible with the fixed values and defaults.
    let known = fields.map(|f| f.values.clone()).unwrap_or_default();
    let shown: Vec<&'a ParamDef> = fields
        .map(|f| f.tool.parameters.iter().collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| step.source(&p.name).is_some() || is_visible(p, &known))
        .collect();
    let main: Vec<Element<'a, Message>> = shown
        .iter()
        .filter(|p| !p.advanced)
        .map(|p| param_row(p))
        .collect();
    let advanced: Vec<&'a ParamDef> = shown.iter().copied().filter(|p| p.advanced).collect();
    let mut out = column![
        head(
            crate::icons::from_web(tool.icon.as_deref().or(Some("processing"))),
            false,
            tool.label.clone(),
            tool.description.clone()
        ),
        field_row(
            words::step::CAPTION,
            text_field(&tool.label, step.caption.as_deref().unwrap_or(""), |t| {
                ev(Event::Caption(t))
            }),
            Some(words::step::CAPTION_NOTE.to_owned()),
        ),
    ]
    .spacing(18);
    if let Some(p) = problems(d, Some(id), &lookup) {
        out = out.push(p);
    }
    out = out.push(section(Some(words::step::PARAMS.to_owned()), main));
    if !advanced.is_empty() {
        let toggle = button(
            row![
                icon(if d.advanced_open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                })
                .size(12.0)
                .tone(Tone::Muted),
                label::caption(words::step::advanced(advanced.len()))
                    .font(typography::ui_strong())
                    .style(style::text::muted),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([2, 0])
        .width(Fill)
        .style(style::button::ghost)
        .on_press(ev(Event::Advanced));
        let mut part = column![column![toggle, horizontal_divider()].spacing(4)].spacing(10);
        if d.advanced_open {
            for p in advanced {
                part = part.push(param_row(p));
            }
        }
        out = out.push(part);
    }
    out.push(danger(words::step::REMOVE, ev(Event::RemoveStep)))
        .into()
}
