//! The layer style window's small parts: symbol slots (the web's
//! `symbolSlot.ts`), the expression field with the layer's fields, the
//! language's variables and functions, single-line fields and switches.

use iced::widget::tooltip::Position;
use iced::widget::{Row, button, column, container, row, text_input};
use iced::{Center, Element, Fill, Length};
use kentos_native_style::preview::Geometry;
use kentos_native_style::renderer::{GeometryClass, SymbolSet, ref_id};
use kentos_native_style::tally::field_token;
use kentos_processing::expression::{FUNCTIONS, VARIABLES};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};
use serde_json::Value;

use super::{Event, Field, SetAt};
use crate::app::Message;
use crate::style::thumbs::Look;

pub(super) fn ev(e: Event) -> Message {
    Message::LayerStyle(e)
}

/// A slot's picture, logical pixels (the web's canvas).
const PICTURE: (f32, f32) = (64.0, 36.0);

/// What the slots of a window read.
pub(super) struct Env<'a> {
    pub look: Look<'a>,
    pub thumbs: &'a crate::style::thumbs::Thumbs,
    /// The layer's simple look: what a class without a symbol draws.
    pub simple: &'a SymbolSet,
    pub classes: &'a [GeometryClass],
    /// The symbols' names by library id.
    pub name_of: &'a dyn Fn(&str) -> Option<String>,
}

/// A symbol in a style: its picture (a class without one shows the layer's
/// simple look it falls back to) and its name; the menu picks another, edits
/// it, keeps it in the user's library or goes back to the simple look.
fn slot<'a>(
    env: &Env<'_>,
    at: SetAt,
    class: GeometryClass,
    symbol: Option<&Value>,
    title: &str,
) -> Element<'a, Message> {
    let missing = symbol
        .and_then(ref_id)
        .is_some_and(|id| env.look.library.symbol(id).is_none());
    let name = match symbol {
        Some(s) => match ref_id(s) {
            Some(id) => (env.name_of)(id).unwrap_or_else(|| "Kitaplıkta yok".to_owned()),
            None => "Bu stilde".to_owned(),
        },
        None => "Basit görünüş".to_owned(),
    };
    let shown = symbol.or_else(|| env.simple.get(class));
    let picture: Element<'a, Message> = match shown {
        Some(s) if !missing => env
            .thumbs
            .picture(s, Some(sample(class)), PICTURE, None, &env.look),
        _ => container(
            icon(if missing { Icon::Warning } else { Icon::Minus })
                .size(14.0)
                .tone(if missing { Tone::Danger } else { Tone::Muted }),
        )
        .center_x(PICTURE.0)
        .center_y(PICTURE.1)
        .into(),
    };
    // The name in the web's small size, a little past the picture at most (the web's 72 px).
    let size = typography::caption() - 1.0;
    let face = container(
        column![
            picture,
            label::caption(fit(&name, size, typography::scaled(PICTURE.0 + 8.0)))
                .size(size)
                .style(style::text::muted)
        ]
        .spacing(2)
        .align_x(Center),
    )
    .padding(3)
    .style(move |t: &iced::Theme| {
        let mut s = style::container::bordered(t);
        if missing {
            s.border.color = Tokens::of(t).danger;
        }
        s
    });
    let linked_id = symbol.and_then(ref_id).map(str::to_owned);
    let linked = linked_id.is_some();
    // A symbol written into the style (not a library reference) can be kept in Kitaplığım.
    let own = symbol.filter(|s| ref_id(s).is_none()).cloned();
    let has = symbol.is_some();
    let title_owned = title.to_owned();
    // What Düzenle starts from: the slot's own symbol, a copy of the library
    // symbol, or what the slot shows (the web's started from a default when
    // the slot had none); none when the library symbol is gone.
    let design = crate::style::designer::slot_symbol(
        env.look.library,
        symbol,
        env.simple.get(class),
        class,
    );
    let menu = move || {
        Menu::new()
            .item(
                "Kitaplıktan seç…",
                ev(Event::Pick(
                    at.clone(),
                    class,
                    title_owned.clone(),
                    linked_id.clone(),
                )),
            )
            .icon(crate::icons::from_web(Some("styles")))
            .item(
                if linked {
                    "Kopyasını burada düzenle…"
                } else {
                    "Düzenle…"
                },
                design
                    .clone()
                    .map(|s| ev(Event::Design(at.clone(), class, title_owned.clone(), s))),
            )
            .icon(crate::icons::from_web(Some("edit")))
            .item(
                "Kitaplığıma kaydet",
                own.clone()
                    .map(|s| ev(Event::Keep(at.clone(), class, title_owned.clone(), s))),
            )
            .icon(crate::icons::from_web(Some("save")))
            .separator()
            .item(
                "Basit görünüşe dön",
                has.then(|| ev(Event::Symbol(at.clone(), class, None))),
            )
    };
    tip(
        MenuButton::new(face, menu),
        Tip::new(format!("{}: {name}", class.label())).body(title.to_owned()),
        Position::Bottom,
    )
}

/// The sample a class's picture shows.
fn sample(class: GeometryClass) -> Geometry {
    match class {
        GeometryClass::Fill => Geometry::Area,
        GeometryClass::Line => Geometry::Line,
        GeometryClass::Marker => Geometry::Point,
    }
}

/// Slots for each geometry class the layer has (`symbolSetSlots`).
pub(super) fn slots<'a>(
    env: &Env<'_>,
    at: SetAt,
    set: &SymbolSet,
    title: &str,
) -> Element<'a, Message> {
    let mut out = Row::new().spacing(4);
    for class in env.classes {
        out = out.push(slot(
            env,
            at.clone(),
            *class,
            set.get(*class),
            &format!(
                "{title} ({})",
                kentos_processing::text::tr_lower(class.label())
            ),
        ));
    }
    out.into()
}

/// A name cut with an ellipsis to `width` logical pixels at `size` (the web's `text-overflow`).
fn fit(s: &str, size: f32, width: f32) -> String {
    if typography::text_width(s, size) <= width {
        return s.to_owned();
    }
    let chars: Vec<char> = s.chars().collect();
    for n in (1..chars.len()).rev() {
        let mut out: String = chars[..n].iter().collect();
        out.push('…');
        if typography::text_width(&out, size) <= width {
            return out;
        }
    }
    "…".to_owned()
}

/// A single-line field in the window's look.
pub(super) fn input<'a>(placeholder: &str, value: &str) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value)
        .padding([4, 7])
        .font(typography::ui())
        .size(typography::body())
        .style(style::field::validated(false))
}

/// A number field, right-aligned figures, `width` logical pixels.
pub(super) fn number<'a>(
    placeholder: &str,
    value: &str,
    width: f32,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    input(placeholder, value)
        .on_input(on_input)
        .align_x(iced::alignment::Horizontal::Right)
        .width(Length::Fixed(typography::scaled(width)))
        .into()
}

/// A switch in a row (the web's checkbox).
pub(super) fn check<'a>(on: bool, press: Message) -> Element<'a, Message> {
    check_box(
        if on { Check::Checked } else { Check::Unchecked },
        Some(press),
    )
}

/// A small icon button with its tip.
pub(super) fn tool<'a>(glyph: &str, label: &str, press: Option<Message>) -> Element<'a, Message> {
    tip(
        button(icon(crate::icons::from_web(Some(glyph))).size(14.0))
            .padding([3, 4])
            .style(style::button::ghost)
            .on_press_maybe(press),
        Tip::new(label.to_owned()),
        Position::Bottom,
    )
}

/// A menu's face: its name and a chevron.
fn menu_face<'a>(text: &str) -> Element<'a, Message> {
    container(
        row![
            label::caption(text.to_owned()),
            icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted)
        ]
        .spacing(3)
        .align_y(Center),
    )
    .padding([4, 6])
    .style(style::container::field_box)
    .into()
}

fn fields_menu(field: &Field, fields: &[(String, usize)]) -> Menu<Message> {
    if fields.is_empty() {
        return Menu::new().item("Bu katmanın nesnelerinde öznitelik alanı yok", None);
    }
    fields.iter().fold(Menu::new(), |m, (f, n)| {
        m.item(f.clone(), ev(Event::Insert(field.clone(), field_token(f))))
            .shortcut(format!("{n} nesne"))
    })
}

fn variables_menu(field: &Field) -> Menu<Message> {
    VARIABLES.iter().fold(Menu::new(), |m, v| {
        m.item(
            format!("${}", v.name),
            ev(Event::Insert(field.clone(), format!("${}", v.name))),
        )
        .detail(v.description)
    })
}

fn functions_menu(field: &Field) -> Menu<Message> {
    FUNCTIONS.iter().fold(Menu::new(), |m, f| {
        m.item(
            f.signature,
            ev(Event::Insert(field.clone(), format!("{}(", f.name))),
        )
        .detail(f.description)
    })
}

/// An expression: one line in mono, and menus that put in the layer's
/// fields (with how many objects have each), the variables and functions.
/// `compact`: the three menus behind one ƒ (a rule's condition).
pub(super) fn expression<'a>(
    field: Field,
    text: &str,
    placeholder: &str,
    fields: &[(String, usize)],
    invalid: bool,
    compact: bool,
) -> Element<'a, Message> {
    let on = field.clone();
    let line = text_input(placeholder, text)
        .padding([4, 7])
        .font(typography::mono())
        .size(typography::body())
        .style(style::field::validated(invalid))
        .on_input(move |t| ev(Event::Expr(on.clone(), t)))
        .width(Fill);
    let fields = fields.to_vec();
    if compact {
        let menu = move || {
            Menu::new()
                .submenu("Alanlar", fields_menu(&field, &fields))
                .submenu("Değişkenler", variables_menu(&field))
                .submenu("İşlevler", functions_menu(&field))
        };
        return row![line, MenuButton::new(menu_face("ƒ"), menu)]
            .spacing(2)
            .align_y(Center)
            .into();
    }
    let (f1, f2, f3) = (field.clone(), field.clone(), field);
    row![
        line,
        MenuButton::new(menu_face("Alanlar"), move || fields_menu(&f1, &fields)),
        MenuButton::new(menu_face("Değişkenler"), move || variables_menu(&f2)),
        MenuButton::new(menu_face("İşlevler"), move || functions_menu(&f3)),
    ]
    .spacing(2)
    .align_y(Center)
    .into()
}

/// A problem under a field, in the danger colour.
pub(super) fn problem<'a>(text: String) -> Element<'a, Message> {
    row![
        icon(Icon::Warning).size(13.0).tone(Tone::Danger),
        label::caption(text).style(style::text::danger)
    ]
    .spacing(6)
    .align_y(Center)
    .into()
}

/// Help under a table, muted.
pub(super) fn help<'a>(text: impl Into<String>) -> Element<'a, Message> {
    label::caption(text.into()).style(style::text::muted).into()
}
