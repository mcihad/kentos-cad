//! How the layer style window looks (the web's `LayerStyleDialog` and its
//! `lsty__*` rules): the renderer kinds and the layer's objects at the top;
//! the kind's panel, which scrolls; what the window last said, Vazgeç,
//! Uygula and Tamam at the bottom, always in view. The categories' and
//! classes' tables line up their columns; the Nesne column says what each
//! row will draw.

use iced::widget::tooltip::Position;
use iced::widget::{Column, Row, button, column, container, rich_text, row, space, span, stack};
use iced::{Center, Element, Fill, Length};
use kentos_native_style::classify::{Method, RAMPS, ramp_colors, texts};
use kentos_native_style::renderer::SymbolSet;
use kentos_native_style::simple::symbols_of_layer_style;
use kentos_native_style::tally::{category_tally, class_tally, shadowed};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Segmented, Tip, horizontal_divider, overlay, tip};

use super::widgets::{Env, check, ev, expression, help, input, number, problem, slots, tool};
use super::{Event, Field, KINDS, Kind, LayerStyleWindow, SetAt, Source, bound_key};
use crate::app::{App, Message};
use crate::style::thumbs::Look;

/// The window's size (the web's: 980 wide, at most 760 tall).
const WIDTH: f32 = 980.0;
const HEIGHT: f32 = 760.0;
/// A slot's width in the tables, with its border and gap.
const SLOT: f32 = 76.0;
const COUNT: f32 = 64.0;
const CHECK: f32 = 26.0;
const DELETE: f32 = 30.0;

impl App {
    /// The open layer style window, over the drawing.
    pub(crate) fn layer_style_view(&self) -> Element<'_, Message> {
        let (Some(window), Some(doc)) = (&self.styles.layer_style, &self.document) else {
            return space().into();
        };
        let model = &doc.model;
        let node = model.layers().get(&window.layer);
        let simple = node
            .map(|n| {
                SymbolSet::from_value(&symbols_of_layer_style(&n.style, &n.style.color, false))
            })
            .unwrap_or_default();
        let palette = self.style_palette();
        let library = &self.styles.library;
        let name_of = |id: &str| library.get(id).map(|(item, _)| item.name().to_owned());
        let env = Env {
            look: Look {
                palette: &palette,
                library,
                images: &self.styles.images,
            },
            thumbs: &self.styles.thumbs,
            simple: &simple,
            classes: &window.classes,
            name_of: &name_of,
        };
        let src = Source {
            doc: model,
            store: self.spatial.store(),
        };
        let fields = window.fields(model);
        let top = row![
            Segmented::new(KINDS, window.kind, |k| ev(Event::Kind(k))),
            label::caption(objects_text(model.count(&window.layer), window))
                .style(style::text::muted),
        ]
        .spacing(14)
        .align_y(Center);
        let panel: Element<'_, Message> = match window.kind {
            Kind::Simple => simple_panel(&env),
            Kind::Single => column![
                help("Her nesne geometrisine göre bu sembolle çizilir. Sembolü değiştirmek için resmine tıklayın."),
                slots(&env, SetAt::Single, &window.single, "Tek sembol"),
            ]
            .spacing(10)
            .into(),
            Kind::Categorized => categorized_panel(window, &env, &src, &fields),
            Kind::Graduated => graduated_panel(window, &env, &src, &fields),
            Kind::Rules => super::rules::rules_panel(window, &env, &src, &fields),
            Kind::Unknown => unknown_panel(window),
        };
        let title = format!("Katman stili: {}", node.map_or("", |n| n.name.as_str()));
        let name = node.map_or(String::new(), |n| n.name.clone());
        let base = overlay::modal(
            Dialog::new(title)
                .push(top)
                .scroll_fill(container(panel).padding([0, 4]).width(Fill))
                .push(horizontal_divider())
                .push(footer(window))
                .width(WIDTH)
                .max_height(HEIGHT),
            ev(Event::Close),
        );
        if window.asking {
            stack![base, overlay::modal(question(&name), ev(Event::Stay))].into()
        } else {
            base
        }
    }

    /// The drawing's theme colours as symbols read them (`fg`, `ink`, `paper`).
    pub(crate) fn style_palette(&self) -> kentos_native_style::StylePalette {
        let p = crate::viewport::palette(self.canvas());
        let hex =
            |c: kentos_render_wgpu::Rgba8| format!("#{:02X}{:02X}{:02X}", c.0[0], c.0[1], c.0[2]);
        kentos_native_style::StylePalette {
            fg: hex(p.fg),
            fg_dim: hex(p.fg_dim),
            ink: hex(p.ink),
            paper: hex(p.background),
        }
    }
}

/// “6 nesne: 6 alan”, or that nothing of the layer is drawn by a style.
fn objects_text(n: usize, window: &LayerStyleWindow) -> String {
    let p = window.present;
    let parts: Vec<String> = [(p.fill, "alan"), (p.line, "çizgi"), (p.marker, "nokta")]
        .into_iter()
        .filter(|(k, _)| *k > 0)
        .map(|(k, w)| format!("{k} {w}"))
        .collect();
    let what = if parts.is_empty() {
        "çizilecek nesne yok".to_owned()
    } else {
        parts.join(", ")
    };
    format!("{n} nesne: {what}")
}

fn footer<'a>(window: &LayerStyleWindow) -> Element<'a, Message> {
    let status: Element<'a, Message> = match window.status() {
        Some((text, warn)) => row![
            icon(if warn { Icon::Warning } else { Icon::Check })
                .size(14.0)
                .tone(if warn { Tone::Warning } else { Tone::Success }),
            label::body(text).style(move |t: &iced::Theme| iced::widget::text::Style {
                color: Some(if warn {
                    Tokens::of(t).warning
                } else {
                    Tokens::of(t).muted
                }),
            }),
        ]
        .spacing(6)
        .align_y(Center)
        .into(),
        None => space().into(),
    };
    row![
        container(status).width(Fill),
        button(label::body("Vazgeç"))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(Event::Close)),
        tip(
            button(label::body("Uygula"))
                .padding([5, 14])
                .style(style::button::secondary)
                .on_press(ev(Event::Apply)),
            Tip::new("Haritada göster, pencere açık kalsın"),
            Position::Top,
        ),
        button(
            row![
                icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
                label::body("Tamam").style(style::text::on_accent)
            ]
            .spacing(6)
            .align_y(Center)
        )
        .padding([5, 14])
        .style(style::button::primary)
        .on_press(ev(Event::Done)),
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

/// Closing with changes not applied (the web's `askUnsaved` with `apply`,
/// DESIGN.md §7.9.1): “Uygulamadan kapat” aside on the left, Vazgeç, and
/// “Uygula ve kapat” first in weight.
fn question<'a>(name: &str) -> Element<'a, Message> {
    let symbol = container(icon(Icon::Warning).size(18.0).tone(Tone::Warning))
        .center_x(36)
        .center_y(36)
        .style(|t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(
                Tokens::of(t).warning.scale_alpha(0.14),
            )),
            border: iced::border::rounded(18.0),
            ..container::Style::default()
        });
    let text = column![
        label::title("Uygulanmamış değişiklikler"),
        label::body(format!(
            "“{name}” katman stilinde uygulanmamış değişiklikler var. Pencere kapanırsa bu değişiklikler kaybolur."
        )),
    ]
    .spacing(6)
    .width(Fill);
    let answer = |text: &'static str, e: Event| {
        button(label::body(text))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(e))
    };
    let actions = row![
        answer("Uygulamadan kapat", Event::Discard),
        space::horizontal(),
        answer("Vazgeç", Event::Stay),
        button(label::body("Uygula ve kapat").style(style::text::on_accent))
            .padding([5, 14])
            .style(style::button::primary)
            .on_press(ev(Event::Done)),
    ]
    .spacing(6)
    .align_y(Center);
    container(column![row![symbol, text].spacing(14), actions].spacing(18))
        .width(typography::scaled(480.0))
        .padding(18)
        .style(style::container::popover)
        .into()
}

/// A note with a bold lead (the web's `note('info', …)`).
fn note<'a>(lead: &'static str, text: &'static str) -> Element<'a, Message> {
    container(
        row![
            icon(Icon::Info).size(15.0).tone(Tone::Accent),
            rich_text([
                span(lead).font(typography::ui_strong()),
                span(text).font(typography::ui()),
            ])
            .on_link_click(iced::never)
            .size(typography::body()),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([10, 12])
    .width(Fill)
    .style(style::container::bordered)
    .into()
}

/// Basit: the layer's own look, and the pictures of what it draws now.
fn simple_panel<'a>(env: &Env<'_>) -> Element<'a, Message> {
    let mut pictures = Row::new().spacing(12).align_y(Center);
    for class in env.classes {
        if let Some(s) = env.simple.get(*class) {
            pictures = pictures.push(
                column![
                    env.thumbs.picture(s, None, (96.0, 54.0), None, &env.look),
                    label::caption(class.label()).style(style::text::muted),
                ]
                .spacing(4)
                .align_x(Center),
            );
        }
    }
    column![
        note(
            "Katmanın kendi görünüşü. ",
            "Renk, çizgi tipi, kalınlık ve dolgu Katmanlar panelinden gelir; nesnelere verilen semboller yine önce gelir."
        ),
        row![
            label::caption("Şimdiki görünüşü").style(style::text::muted),
            pictures
        ]
        .spacing(14)
        .align_y(Center),
    ]
    .spacing(14)
    .into()
}

/// A renderer this version cannot read: said, and kept.
fn unknown_panel<'a>(window: &LayerStyleWindow) -> Element<'a, Message> {
    let why = window
        .unknown
        .as_ref()
        .map(|(_, why)| why.clone())
        .unwrap_or_default();
    container(
        row![
            icon(Icon::Warning).size(15.0).tone(Tone::Warning),
            label::body(format!(
                "Bu katmanın stili ({why}) KentOS'un bu sürümünde düzenlenemiyor. Uygula ya da Tamam onu olduğu gibi bırakır; yukarıdan başka bir işleyici seçip uygularsanız onun yerine geçer."
            )),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([10, 12])
    .width(Fill)
    .style(style::container::bordered)
    .into()
}

/// A table's header cell.
fn head<'a>(text: &'static str, width: Length, right: bool) -> Element<'a, Message> {
    let t = label::caption(text).style(style::text::muted);
    let c = container(t).width(width);
    if right {
        c.align_right(width).into()
    } else {
        c.into()
    }
}

/// A count cell, right-aligned figures.
fn count<'a>(text: String, width: f32) -> Element<'a, Message> {
    container(label::mono(text))
        .align_right(Length::Fixed(typography::scaled(width)))
        .into()
}

fn row_line<'a>() -> Element<'a, Message> {
    horizontal_divider().into()
}

fn small_button<'a>(
    glyph: Option<Icon>,
    text: &'static str,
    press: Option<Message>,
) -> Element<'a, Message> {
    let face: Element<'a, Message> = match glyph {
        Some(g) => row![icon(g).size(13.0), label::body(text)]
            .spacing(5)
            .align_y(Center)
            .into(),
        None => label::body(text).into(),
    };
    button(face)
        .padding([4, 10])
        .style(style::button::secondary)
        .on_press_maybe(press)
        .into()
}

fn categorized_panel<'a>(
    window: &LayerStyleWindow,
    env: &Env<'_>,
    src: &Source<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let c = &window.categorized;
    let values = window.values(src);
    let error = values.error_text();
    let (counts, rest) = category_tally(&values.values, &window.drawn(src), &c.categories);
    let has_expr = !kentos_processing::text::js_trim(&c.expr).is_empty();
    let classify = small_button(
        None,
        "Değerlerden sınıfla",
        (has_expr && error.is_none()).then(|| ev(Event::Classify)),
    );
    let mut out = Column::new().spacing(10).push(
        row![
            label::caption("Değer").style(style::text::muted),
            expression(
                Field::Categories,
                &c.expr,
                "Alan adı ya da ifade: Nitelik",
                fields,
                error.is_some(),
                false
            ),
            classify,
        ]
        .spacing(8)
        .align_y(Center),
    );
    if let Some(e) = error {
        out = out.push(problem(e));
    }
    let slot_w = Length::Fixed(typography::scaled(SLOT * env.classes.len() as f32));
    let fixed = |w: f32| Length::Fixed(typography::scaled(w));
    let mut table = Column::new().spacing(4).push(
        row![
            head("", fixed(CHECK), false),
            head("Sembol", slot_w, false),
            head("Değer", Length::FillPortion(1), false),
            head("Etiket", Length::FillPortion(1), false),
            head("Nesne", fixed(COUNT), true),
            head("", fixed(DELETE), false),
        ]
        .spacing(8)
        .align_y(Center),
    );
    table = table.push(row_line());
    for (i, k) in c.categories.iter().enumerate() {
        let enabled = k.enabled != Some(false);
        let mut value_cell = Row::new().spacing(4).align_y(Center).push(
            input("Değer", &k.value)
                .on_input(move |t| ev(Event::CategoryValue(i, t)))
                .width(Fill),
        );
        if shadowed(&c.categories, i) {
            value_cell = value_cell.push(tip(
                icon(Icon::Warning).size(14.0).tone(Tone::Warning),
                Tip::new("Bu değer yukarıda da var; yalnız ilki çizer."),
                Position::Top,
            ));
        }
        let title = if k.label.is_empty() {
            k.value.clone()
        } else {
            k.label.clone()
        };
        table = table.push(
            row![
                container(check(enabled, ev(Event::CategoryEnabled(i, !enabled))))
                    .width(fixed(CHECK)),
                container(slots(env, SetAt::Category(i), &k.symbols, &title)).width(slot_w),
                container(value_cell).width(Length::FillPortion(1)),
                container(
                    input("Etiket", &k.label)
                        .on_input(move |t| ev(Event::CategoryLabel(i, t)))
                        .width(Fill)
                )
                .width(Length::FillPortion(1)),
                count(counts.get(i).copied().unwrap_or(0).to_string(), COUNT),
                container(tool(
                    "trash",
                    "Kategoriyi sil",
                    Some(ev(Event::RemoveCategory(i)))
                ))
                .width(fixed(DELETE)),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }
    // Diğer değerler: what no category takes, drawn or not.
    let other_on = c.other.is_some();
    let other_slots: Element<'a, Message> = match &c.other {
        Some(set) => slots(env, SetAt::Other, set, texts::OTHER),
        None => label::caption("çizilmez").style(style::text::muted).into(),
    };
    table = table.push(row_line()).push(
        row![
            container(check(other_on, ev(Event::Other(!other_on)))).width(fixed(CHECK)),
            container(other_slots).width(slot_w),
            container(label::body(texts::OTHER).style(style::text::muted))
                .width(Length::FillPortion(2)),
            count(rest.to_string(), COUNT),
            space().width(fixed(DELETE)),
        ]
        .spacing(8)
        .align_y(Center),
    );
    out = out.push(table);
    if c.categories.is_empty() {
        out = out.push(help(texts::CATEGORIES_HELP));
    }
    out.push(
        row![
            small_button(
                Some(Icon::Plus),
                "Kategori ekle",
                Some(ev(Event::AddCategory))
            ),
            small_button(
                None,
                "Hepsini sil",
                (!c.categories.is_empty()).then(|| ev(Event::ClearCategories))
            ),
        ]
        .spacing(6),
    )
    .into()
}

/// A method of Sınıfla, as the list names it.
fn method_label(m: Method) -> &'static str {
    match m {
        Method::Interval => "Eşit aralık",
        Method::Count => "Eşit sayı (dilimler)",
    }
}

/// A ramp's colours, as small squares.
fn ramp_strip<'a>(stops: &[&str]) -> Element<'a, Message> {
    let mut out = Row::new().spacing(0);
    for c in ramp_colors(stops, 7) {
        let color = crate::view::hex_color(&c);
        out = out.push(
            container(space())
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(14.0))
                .style(style::container::solid(color)),
        );
    }
    out.into()
}

fn graduated_panel<'a>(
    window: &LayerStyleWindow,
    env: &Env<'_>,
    src: &Source<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let g = &window.graduated;
    let numbers = window.numbers(src);
    let any = numbers.values.iter().any(Option::is_some);
    let has_expr = !kentos_processing::text::js_trim(&g.expr).is_empty();
    let error = numbers
        .error_text()
        .or_else(|| (has_expr && !any).then(|| texts::NO_NUMBERS.to_owned()));
    let (counts, rest) = class_tally(&numbers.values, &window.drawn(src), &g.classes);
    let mut out = Column::new().spacing(10).push(
        row![
            label::caption("Değer").style(style::text::muted),
            expression(
                Field::Classes,
                &g.expr,
                "Sayı veren ifade: $alan, Kat",
                fields,
                numbers.error.is_some(),
                false
            ),
        ]
        .spacing(8)
        .align_y(Center),
    );
    if let Some(e) = error {
        out = out.push(problem(e));
    }
    let methods = [Method::Interval, Method::Count];
    let method = Select::new(
        methods.map(|m| Choice::new(method_label(m))),
        methods.iter().position(|m| *m == window.method),
        move |i| ev(Event::Method(methods[i.min(1)])),
    )
    .searchable(false);
    let ramp = Select::new(
        RAMPS.iter().map(|r| {
            Choice::new(r.label).color(crate::view::hex_color(r.stops[r.stops.len() - 1]))
        }),
        RAMPS.iter().position(|r| r.key == window.ramp),
        |i| ev(Event::Ramp(RAMPS[i.min(RAMPS.len() - 1)].key)),
    )
    .searchable(false);
    let current = kentos_native_style::classify::ramp(window.ramp);
    out = out.push(
        row![
            label::caption("Yöntem").style(style::text::muted),
            container(method).width(Length::Fixed(typography::scaled(190.0))),
            label::caption("Sınıf").style(style::text::muted),
            number("5", &window.count, 56.0, |t| ev(Event::Count(t))),
            label::caption("Renkler").style(style::text::muted),
            container(ramp).width(Length::Fixed(typography::scaled(180.0))),
            ramp_strip(current.stops),
            small_button(None, "Sınıfla", any.then(|| ev(Event::Graduate))),
        ]
        .spacing(8)
        .align_y(Center),
    );
    let fixed = |w: f32| Length::Fixed(typography::scaled(w));
    let slot_w = fixed(SLOT * env.classes.len() as f32);
    let mut table = Column::new().spacing(4).push(
        row![
            head("Sembol", slot_w, false),
            head("Alt (dahil)", fixed(110.0), false),
            head("Üst", fixed(110.0), false),
            head("Etiket", Length::Fill, false),
            head("Nesne", fixed(COUNT), true),
            head("", fixed(DELETE), false),
        ]
        .spacing(8)
        .align_y(Center),
    );
    table = table.push(row_line());
    for (i, c) in g.classes.iter().enumerate() {
        let typed = |min: bool, v: f64| {
            window
                .typed
                .get(&bound_key(i, min))
                .cloned()
                // The bound as the labels round it; the class keeps it whole until one is typed.
                .unwrap_or_else(|| kentos_native_style::classify::rounded(v, 2))
        };
        let n = counts.get(i).copied().unwrap_or(0);
        table = table.push(
            row![
                container(slots(env, SetAt::Class(i), &c.symbols, &c.label)).width(slot_w),
                number("Alt sınır", &typed(true, c.min), 110.0, move |t| ev(
                    Event::ClassMin(i, t)
                )),
                number("Üst sınır", &typed(false, c.max), 110.0, move |t| ev(
                    Event::ClassMax(i, t)
                )),
                input("Etiket", &c.label)
                    .on_input(move |t| ev(Event::ClassLabel(i, t)))
                    .width(Fill),
                count(n.to_string(), COUNT),
                container(tool("trash", "Sınıfı sil", Some(ev(Event::RemoveClass(i)))))
                    .width(fixed(DELETE)),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }
    // What no class takes (no number, or outside every class): not drawn.
    if !g.classes.is_empty() && rest > 0 {
        table = table.push(row_line()).push(
            row![
                space().width(slot_w),
                container(
                    label::body("Sınıfların dışında kalanlar: çizilmez").style(style::text::muted)
                )
                .width(Fill),
                count(rest.to_string(), COUNT),
                space().width(fixed(DELETE)),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }
    out = out.push(table);
    out.push(help(if g.classes.is_empty() {
        texts::CLASSES_EMPTY_HELP
    } else {
        texts::CLASSES_HELP
    }))
    .into()
}
