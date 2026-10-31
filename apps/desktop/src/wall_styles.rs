//! Duvar stilleri (docs/adr/0339 §2, §6; `arch.wallStyles`): the project's wall styles, each with how many walls
//! follow it, then KentOS's own the project has not taken. A style chosen shows its form and its section drawn as the
//! drawing draws a wall of it at the detailed level: its components from the wall's left face (name, material,
//! thickness, the one without a thickness taking the rest, hatch and its scale, the least detail it is drawn at), its
//! ends, its schematic fill and the width a new wall of it takes. KentOS's own are not edited: Projeye al takes one
//! in. Yeni copies the chosen one, Sil takes the chosen one away (asking when walls follow it: they keep their look,
//! without the link).
//!
//! The form is a draft. Kaydet writes it through `cad.styles.edit` (kind `wall`: create, update, remove), one undo
//! step “Duvar stilleri” in which the walls that follow a changed style take it; a style a following wall cannot take
//! (an arc too tight for its new faces) is said and nothing is written. Vazgeç, Esc and × leave it.

use std::collections::HashMap;

use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::widget::{Column, button, container, row};
use iced::{Center, Color, Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    AnnotationStyleKind, ComponentHatch, Entity, StyleEditOperation, StylesEdit, WallCaps,
    WallComponent, WallDetail, WallFields, WallStyleDef, builtin_wall_styles, wall_styles_problem,
};
use kentos_interaction::{Format, Level};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Choice, Confirm, Dialog as Frame_, Elided, Select, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind as Line};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Duvar stilleri";
const NEW: &str = "Yeni";
const REMOVE: &str = "Sil";
const TAKE: &str = "Projeye al";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const ADD: &str = "Bileşen ekle";

/// The details a component may be drawn from, in order.
const DETAILS: [(WallDetail, &str); 3] = [
    (WallDetail::Schematic, "Şematik"),
    (WallDetail::Standard, "Standart"),
    (WallDetail::Detailed, "Ayrıntılı"),
];

/// A component's field typed as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Name,
    Material,
    Thickness,
    Scale,
    Angle,
}

#[derive(Debug, Clone)]
pub struct Window {
    /// The project's table as edited.
    styles: Vec<WallStyleDef>,
    /// KentOS's own the project has not taken, as when the window opened.
    builtin: Vec<WallStyleDef>,
    chosen: Option<String>,
    /// The number fields as typed: the width, then each component's by its place.
    typed: HashMap<(usize, Field), String>,
    width_typed: String,
    usage: HashMap<String, usize>,
    asking: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Option<String>),
    New,
    Take,
    Remove,
    RemoveConfirmed,
    RemoveDropped,
    Name(String),
    Width(String),
    OpenCaps(bool),
    Fill(Option<String>),
    Component(usize, Field, String),
    Pattern(usize, Option<String>),
    Detail(usize, WallDetail),
    /// A component's priority at a join (docs/adr/0340 §1), 1 to 9; none: the rule's 3.
    Priority(usize, Option<u8>),
    /// A component turning at the wall's ends and its openings' jambs (docs/adr/0771 §3).
    Wrap(usize, bool),
    Add,
    Drop(usize),
    Up(usize),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::WallStyles(event)
}

fn shown(n: Option<f64>) -> String {
    n.map(|v| format!("{v}")).unwrap_or_default()
}

/// A number field's value: none when empty, `Err` when it is not a number.
fn number(s: &str) -> Result<Option<f64>, ()> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map(Some)
        .ok_or(())
}

/// A name not taken yet: `base`, else `base 2`, `base 3` …
fn free_name(base: &str, taken: &[WallStyleDef]) -> String {
    let free = |n: &str| {
        !taken
            .iter()
            .any(|t| t.name.to_lowercase() == n.to_lowercase())
    };
    if free(base) {
        return base.to_owned();
    }
    (2..)
        .map(|i| format!("{base} {i}"))
        .find(|n| free(n))
        .unwrap_or_default()
}

impl Window {
    pub(crate) fn asking(&self) -> bool {
        self.asking.is_some()
    }

    fn problem(&self) -> Option<String> {
        wall_styles_problem(&self.styles)
    }

    fn own(&mut self) -> Option<&mut WallStyleDef> {
        let id = self.chosen.clone()?;
        self.styles.iter_mut().find(|s| s.id == id)
    }

    fn shown_style(&self) -> Option<(&WallStyleDef, bool)> {
        let id = self.chosen.as_ref()?;
        self.styles
            .iter()
            .find(|s| &s.id == id)
            .map(|s| (s, true))
            .or_else(|| {
                self.builtin
                    .iter()
                    .find(|s| &s.id == id)
                    .map(|s| (s, false))
            })
    }

    /// The number fields refilled from the chosen style.
    fn fill(&mut self) {
        self.typed.clear();
        self.width_typed.clear();
        let Some((s, _)) = self.shown_style() else {
            return;
        };
        let (width, comps) = (shown(s.width), s.components.clone());
        self.width_typed = width;
        for (i, c) in comps.iter().enumerate() {
            self.typed.insert((i, Field::Name), c.name.clone());
            self.typed
                .insert((i, Field::Material), c.material.clone().unwrap_or_default());
            self.typed.insert((i, Field::Thickness), shown(c.thickness));
            self.typed
                .insert((i, Field::Scale), shown(c.hatch.as_ref().map(|h| h.scale)));
            self.typed
                .insert((i, Field::Angle), shown(c.hatch.as_ref().map(|h| h.angle)));
        }
    }

    fn component(&mut self, i: usize, field: Field, typed: String) {
        let value = number(&typed);
        self.typed.insert((i, field), typed.clone());
        let Some(c) = self.own().and_then(|s| s.components.get_mut(i)) else {
            return;
        };
        match field {
            Field::Name => c.name = typed,
            Field::Material => c.material = (!typed.trim().is_empty()).then_some(typed),
            Field::Thickness => {
                if let Ok(v) = value {
                    c.thickness = v;
                }
            }
            Field::Scale => {
                if let (Ok(Some(v)), Some(h)) = (value, c.hatch.as_mut()) {
                    h.scale = v;
                }
            }
            Field::Angle => {
                if let (Ok(v), Some(h)) = (value, c.hatch.as_mut()) {
                    h.angle = v.unwrap_or(0.0);
                }
            }
        }
    }
}

impl App {
    fn saved_wall_styles(&self) -> Vec<WallStyleDef> {
        self.document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.settings().wall_styles.clone())
    }

    /// Duvar stilleri (`arch.wallStyles`): the window.
    pub(crate) fn open_wall_styles(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return;
        };
        let mut usage = HashMap::new();
        for e in doc.model.entities() {
            if let Entity::Wall(w) = e
                && let Some(id) = &w.wall.style
            {
                *usage.entry(id.clone()).or_insert(0) += 1;
            }
        }
        let styles = self.saved_wall_styles();
        let builtin = builtin_wall_styles()
            .into_iter()
            .filter(|b| !styles.iter().any(|s| s.id == b.id))
            .collect::<Vec<_>>();
        let chosen = styles.first().or(builtin.first()).map(|s| s.id.clone());
        let mut w = Window {
            styles,
            builtin,
            chosen,
            typed: HashMap::new(),
            width_typed: String::new(),
            usage,
            asking: None,
        };
        w.fill();
        self.wall_styles = Some(w);
        self.dialog = Some(Dialog::WallStyles);
    }

    fn wall_styles_changed(&self) -> bool {
        self.wall_styles
            .as_ref()
            .is_some_and(|w| w.styles != self.saved_wall_styles())
    }

    /// Kaydet: each style removed, made or changed through `cad.styles.edit`, in one undo step.
    fn save_wall_styles(&mut self) {
        let Some(w) = self.wall_styles.clone() else {
            return;
        };
        if w.problem().is_some() {
            return;
        }
        let saved = self.saved_wall_styles();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        // A style a following wall cannot take is said first: nothing is written.
        if let Err((_, why)) =
            kentos_native_application::style_tables::wall_followers(&doc.model, &w.styles)
        {
            self.warn(why);
            return;
        }
        let edit = |operation, style: Option<String>, wall: Option<WallStyleDef>| StylesEdit {
            kind: AnnotationStyleKind::Wall,
            operation,
            style,
            name: None,
            text: None,
            dimension: None,
            leader: None,
            wall,
            styles: None,
            library: None,
            replace: None,
            expected_revision: None,
        };
        let mut calls = Vec::new();
        for s in saved
            .iter()
            .filter(|s| !w.styles.iter().any(|n| n.id == s.id))
        {
            calls.push(edit(StyleEditOperation::Remove, Some(s.id.clone()), None));
        }
        for s in &w.styles {
            match saved.iter().find(|o| o.id == s.id) {
                None => calls.push(edit(StyleEditOperation::Create, None, Some(s.clone()))),
                Some(o) if o != s => calls.push(edit(
                    StyleEditOperation::Update,
                    Some(s.id.clone()),
                    Some(s.clone()),
                )),
                Some(_) => {}
            }
        }
        let group = doc.model.begin_group(TITLE);
        let (mut followed, mut locked, mut refused) = (0u32, 0u32, None);
        for input in calls {
            let result = kentos_native_application::styles_edit::execute(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                input,
            );
            match result {
                kentos_commands::CommandResult::Completed { output, .. } => {
                    followed += output.followed;
                    locked += output.locked;
                }
                kentos_commands::CommandResult::Failed { error }
                | kentos_commands::CommandResult::Conflict { error }
                | kentos_commands::CommandResult::NeedsInput { error } => {
                    refused = Some(error.message);
                    break;
                }
                _ => {}
            }
        }
        doc.model.end_group(group);
        if let Some(why) = refused {
            self.warn(why);
            return;
        }
        let followed = if followed > 0 {
            format!("; {followed} duvar stiline uydu")
        } else {
            String::new()
        };
        self.announce(
            Level::Success,
            format!("Duvar stilleri kaydedildi{followed}."),
        );
        if locked > 0 {
            self.warn(format!(
                "{locked} duvar kilitli katmanda; eski görünüşüyle kaldı. Kilidi açıp stili yeniden kaydedin."
            ));
        }
        self.wall_styles = None;
        self.dialog = None;
    }

    pub(crate) fn wall_styles_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Save => {
                self.save_wall_styles();
                return Task::none();
            }
            Event::Cancel => {
                self.wall_styles = None;
                self.dialog = None;
                return Task::none();
            }
            _ => {}
        }
        let Some(w) = self.wall_styles.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Choose(id) => {
                w.chosen = id;
                w.fill();
            }
            Event::New | Event::Take => {
                let from = w.shown_style().map(|(s, _)| s.clone());
                let take = matches!(event, Event::Take);
                let mut s = from.unwrap_or_else(|| WallStyleDef {
                    id: String::new(),
                    name: String::new(),
                    components: Vec::new(),
                    caps: WallCaps::Closed,
                    fill: None,
                    width: Some(kentos_commands::DEFAULT_WALL_WIDTH),
                });
                // Projeye al keeps KentOS's id, as a wall made with it would (docs/adr/0339 §2).
                if !take {
                    s.id = kentos_domain::Uuid::now_v7().to_string();
                    s.name = format!(
                        "{} kopyası",
                        if s.name.is_empty() {
                            "Duvar stili"
                        } else {
                            &s.name
                        }
                    );
                }
                s.name = free_name(&s.name, &w.styles);
                if take {
                    w.builtin.retain(|b| b.id != s.id);
                }
                w.chosen = Some(s.id.clone());
                w.styles.push(s);
                w.fill();
            }
            Event::Remove => {
                if let Some(id) = w
                    .chosen
                    .clone()
                    .filter(|id| w.styles.iter().any(|s| &s.id == id))
                {
                    if w.usage.get(&id).copied().unwrap_or(0) > 0 {
                        w.asking = Some(id);
                    } else {
                        w.styles.retain(|s| s.id != id);
                        w.chosen = w.styles.first().map(|s| s.id.clone());
                        w.fill();
                    }
                }
            }
            Event::RemoveConfirmed => {
                if let Some(id) = w.asking.take() {
                    w.styles.retain(|s| s.id != id);
                    w.chosen = w.styles.first().map(|s| s.id.clone());
                    w.fill();
                }
            }
            Event::RemoveDropped => w.asking = None,
            Event::Name(name) => {
                if let Some(s) = w.own() {
                    s.name = name;
                }
            }
            Event::Width(typed) => {
                let value = number(&typed);
                w.width_typed = typed;
                if let (Ok(v), Some(s)) = (value, w.own()) {
                    s.width = v;
                }
            }
            Event::OpenCaps(open) => {
                if let Some(s) = w.own() {
                    s.caps = if open {
                        WallCaps::Open
                    } else {
                        WallCaps::Closed
                    };
                }
            }
            Event::Fill(color) => {
                if let Some(s) = w.own() {
                    s.fill = color;
                }
            }
            Event::Component(i, field, typed) => w.component(i, field, typed),
            Event::Pattern(i, pattern) => {
                if let Some(c) = w.own().and_then(|s| s.components.get_mut(i)) {
                    c.hatch = pattern.map(|p| {
                        let (scale, angle) =
                            c.hatch.as_ref().map_or((1.0, 0.0), |h| (h.scale, h.angle));
                        ComponentHatch {
                            pattern: p,
                            scale,
                            angle,
                            color: None,
                        }
                    });
                }
                w.fill();
            }
            Event::Detail(i, detail) => {
                if let Some(c) = w.own().and_then(|s| s.components.get_mut(i)) {
                    c.detail = detail;
                }
            }
            Event::Priority(i, p) => {
                if let Some(c) = w.own().and_then(|s| s.components.get_mut(i)) {
                    c.priority = p;
                }
            }
            Event::Wrap(i, on) => {
                if let Some(c) = w.own().and_then(|s| s.components.get_mut(i)) {
                    c.wrap = on;
                }
            }
            Event::Add => {
                if let Some(s) = w.own() {
                    let n = s.components.len() + 1;
                    s.components.push(WallComponent {
                        name: format!("Katman {n}"),
                        material: None,
                        thickness: Some(0.02),
                        edge: None,
                        hatch: None,
                        detail: WallDetail::Standard,
                        priority: None,
                        wrap: false,
                    });
                }
                w.fill();
            }
            Event::Drop(i) => {
                if let Some(s) = w.own()
                    && i < s.components.len()
                {
                    s.components.remove(i);
                }
                w.fill();
            }
            Event::Up(i) => {
                if let Some(s) = w.own()
                    && i > 0
                    && i < s.components.len()
                {
                    s.components.swap(i - 1, i);
                }
                w.fill();
            }
            Event::Save | Event::Cancel => {}
        }
        Task::none()
    }

    pub(crate) fn wall_styles_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.wall_styles else {
            return iced::widget::text("").into();
        };
        let Some(doc) = self.document.as_ref() else {
            return iced::widget::text("").into();
        };
        let format = Format::of(doc.model.settings());
        // The list: the project's, each with how many walls follow it, then KentOS's own.
        let item = |s: &WallStyleDef, count: String| {
            let face = row![
                Elided::new(s.name.clone())
                    .size(typography::body())
                    .font(typography::ui())
                    .width(Fill),
                label::caption(count).style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(theme).muted),
                }),
            ]
            .spacing(6)
            .align_y(Center);
            button(face)
                .on_press(msg(Event::Choose(Some(s.id.clone()))))
                .padding([4, 8])
                .width(Fill)
                .style(style::button::row(w.chosen.as_ref() == Some(&s.id)))
        };
        let mut list = Column::new().spacing(1).padding(2);
        if w.styles.is_empty() {
            list = list.push(container(label::caption("Projede duvar stili yok.")).padding([4, 8]));
        }
        for s in &w.styles {
            list = list.push(item(
                s,
                w.usage.get(&s.id).copied().unwrap_or(0).to_string(),
            ));
        }
        if !w.builtin.is_empty() {
            list = list.push(container(label::caption("KentOS'un stilleri")).padding(
                iced::Padding {
                    top: 8.0,
                    right: 8.0,
                    bottom: 2.0,
                    left: 8.0,
                },
            ));
            for s in &w.builtin {
                list = list.push(item(s, String::new()));
            }
        }
        let list = container(iced::widget::scrollable(list).height(typography::scaled(300.0)))
            .style(style::container::field_box)
            .width(Fill);
        let own = w.shown_style().is_some_and(|(_, own)| own);
        let side = Column::new()
            .spacing(10)
            .width(typography::scaled(220.0))
            .push(words::field(TITLE, list, None))
            .push(
                row![
                    words::secondary(NEW, Some(msg(Event::New))),
                    words::secondary(REMOVE, own.then(|| msg(Event::Remove))),
                ]
                .spacing(8),
            );
        let form: Element<'_, Message> = match w.shown_style() {
            None => label::body("Duvar stili, duvarın bileşenlerini (sıva, tuğla, yalıtım), uçlarını ve şematik dolgusunu tutar. Yeni bir stil için Yeni'ye basın.").into(),
            Some((s, true)) => self.wall_style_form(w, s, &format),
            Some((s, false)) => Column::new()
                .spacing(10)
                .push(label::body(format!(
                    "“{}” KentOS'un stili: duvar çizerken seçilince projeye alınır. Düzenlemek için projeye alın.",
                    s.name
                )))
                .push(words::secondary(TAKE, Some(msg(Event::Take))))
                .push(components_text(s, &format))
                .into(),
        };
        let canvas = self.canvas();
        let sample = Sample {
            style: w.shown_style().map(|(s, _)| s.clone()),
            colors: crate::labels::colors(canvas, &crate::viewport::palette(canvas)),
        };
        let preview = container(
            canvas::Canvas::new(sample)
                .width(Fill)
                .height(typography::scaled(120.0)),
        )
        .style(style::container::field_box)
        .width(Fill);
        let main = Column::new()
            .spacing(12)
            .width(Fill)
            .push(form)
            .push(words::field("Kesit (ayrıntılı)", preview, None));
        let mut body = Column::new().spacing(12);
        let problem = w.problem();
        if let Some(p) = &problem {
            body = body.push(words::summary(vec![words::text_line(
                Line::Warn,
                format!("Duvar stilleri: {p}."),
            )]));
        }
        let body = body.push(row![side, main].spacing(16));
        let can_save = problem.is_none() && self.wall_styles_changed();
        let window = Frame_::new(TITLE)
            .scroll(body)
            .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .action(words::primary(SAVE, can_save.then(|| msg(Event::Save))))
            .width(860.0)
            .modal(msg(Event::Cancel));
        let Some(id) = &w.asking else {
            return window;
        };
        let n = w.usage.get(id).copied().unwrap_or(0);
        let name = w
            .styles
            .iter()
            .find(|s| &s.id == id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let question = overlay::modal(
            Confirm::new("Stili sil", msg(Event::RemoveConfirmed), msg(Event::RemoveDropped))
                .message(format!(
                    "{n} duvar “{name}” stilini kullanıyor. Kaydedince görünüşleri kalır, stilsiz olurlar."
                ))
                .confirm(REMOVE)
                .destructive(),
            msg(Event::RemoveDropped),
        );
        iced::widget::stack![window, question].into()
    }

    fn wall_style_form<'a>(
        &'a self,
        w: &'a Window,
        s: &'a WallStyleDef,
        format: &Format,
    ) -> Element<'a, Message> {
        let name =
            kentos_ui::widget::text_field("Stilin adı", &s.name).on_input(|t| msg(Event::Name(t)));
        let fixed = kentos_commands::fixed_width(&s.components);
        let width = kentos_ui::widget::text_field("", &w.width_typed)
            .on_input(|t| msg(Event::Width(t)))
            .style(style::field::validated(number(&w.width_typed).is_err()));
        let width_hint = match fixed {
            Some(f) => format!("Bileşenlerin toplamı: {}", format.length(f)),
            None => format!("Yeni duvarın; şimdi {}", format.length(s.new_width())),
        };
        let fills = crate::ribbon_panels::line_colors();
        let mut values: Vec<Option<String>> = vec![None];
        let mut names: Vec<String> = vec!["Dolgu yok".to_owned()];
        for (n, v) in fills {
            values.push(Some(v.to_owned()));
            names.push(n.to_owned());
        }
        if let Some(f) = &s.fill
            && !values.iter().flatten().any(|v| v.eq_ignore_ascii_case(f))
        {
            values.push(Some(f.clone()));
            names.push(f.to_uppercase());
        }
        let chosen_fill = values.iter().position(|v| match (v, &s.fill) {
            (None, None) => true,
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        });
        let fill = Select::new(names.into_iter().map(Choice::new), chosen_fill, move |i| {
            msg(Event::Fill(values.get(i).cloned().flatten()))
        })
        .searchable(false);
        let top = row![
            container(words::field("Ad", name, None)).width(Fill),
            container(words::field("Genişlik", width, Some(width_hint))).width(Fill),
            container(words::field("Şematik dolgu", fill, None)).width(Fill),
        ]
        .spacing(12)
        .align_y(iced::Top);
        // The components, from the wall's left face.
        let head = |t: &'static str, part: u16| {
            container(kentos_ui::widget::table::title(t))
                .padding([0, 8])
                .width(iced::Length::FillPortion(part))
        };
        let mut table = Column::new()
            .spacing(4)
            .push(kentos_ui::widget::table::band(
                row![
                    head("Bileşen", 4),
                    head("Malzeme", 4),
                    head("Kalınlık (m)", 4),
                    head("Tarama", 4),
                    head("Ölçek", 2),
                    head("Gösterim", 4),
                    head("Öncelik", 3),
                    head("Döner", 2),
                    iced::widget::space().width(iced::Length::Fixed(2.0 * 26.0 + 6.0)),
                ]
                .spacing(6),
            ));
        let patterns: Vec<&'static str> = std::iter::once("SOLID")
            .chain(
                kentos_geometry_core::geom::hatch_pattern::library()
                    .iter()
                    .map(|p| p.name),
            )
            .collect();
        for (i, c) in s.components.iter().enumerate() {
            let typed = |f: Field| w.typed.get(&(i, f)).cloned().unwrap_or_default();
            let input = |f: Field, part: u16, hint: &'static str, wrong: bool| {
                container(
                    kentos_ui::widget::text_field(hint, &typed(f))
                        .on_input(move |t| msg(Event::Component(i, f, t)))
                        .style(style::field::validated(wrong)),
                )
                .width(iced::Length::FillPortion(part))
            };
            let thickness = typed(Field::Thickness);
            let chosen = c
                .hatch
                .as_ref()
                .and_then(|h| {
                    patterns
                        .iter()
                        .position(|p| p.eq_ignore_ascii_case(&h.pattern))
                })
                .map_or(0, |k| k + 1);
            let options = std::iter::once("Yok")
                .chain(patterns.iter().copied())
                .map(Choice::new);
            let names = patterns.clone();
            let pattern = Select::new(options, Some(chosen), move |k| {
                msg(Event::Pattern(
                    i,
                    k.checked_sub(1)
                        .and_then(|k| names.get(k))
                        .map(|p| (*p).to_owned()),
                ))
            });
            let detail = Select::new(
                DETAILS.iter().map(|(_, l)| Choice::new(*l)),
                DETAILS.iter().position(|(d, _)| *d == c.detail),
                move |k| {
                    msg(Event::Detail(
                        i,
                        DETAILS.get(k).map_or(WallDetail::Standard, |(d, _)| *d),
                    ))
                },
            )
            .searchable(false);
            // 1 the strongest: a joining wall's component passes the host's weaker ones (docs/adr/0340 §1).
            let ranks: Vec<String> = std::iter::once("Kural".to_owned())
                .chain((1..=9).map(|r: u8| r.to_string()))
                .collect();
            let priority = Select::new(
                ranks.into_iter().map(Choice::new),
                Some(c.priority.map_or(0, usize::from)),
                move |k| {
                    msg(Event::Priority(
                        i,
                        (k > 0).then(|| u8::try_from(k).unwrap_or(3)),
                    ))
                },
            )
            .searchable(false);
            let scale: Element<'_, Message> = if c.hatch.is_some() {
                input(Field::Scale, 2, "1", number(&typed(Field::Scale)).is_err()).into()
            } else {
                container(label::caption(""))
                    .width(iced::Length::FillPortion(2))
                    .into()
            };
            table = table.push(
                row![
                    input(Field::Name, 4, "Ad", typed(Field::Name).trim().is_empty()),
                    input(Field::Material, 4, "", false),
                    input(Field::Thickness, 4, "kalan", number(&thickness).is_err()),
                    container(pattern).width(iced::Length::FillPortion(4)),
                    scale,
                    container(detail).width(iced::Length::FillPortion(4)),
                    container(priority).width(iced::Length::FillPortion(3)),
                    container(words::check(c.wrap, "", Some(msg(Event::Wrap(i, !c.wrap)))))
                        .width(iced::Length::FillPortion(2)),
                    small("chevronUp", i > 0, msg(Event::Up(i))),
                    small("trash", true, msg(Event::Drop(i))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        let caps = words::check(
            s.caps == WallCaps::Open,
            "Uçlar açık (duvarın ucuna çizgi çizilmez)",
            Some(msg(Event::OpenCaps(s.caps != WallCaps::Open))),
        );
        Column::new()
            .spacing(10)
            .push(top)
            .push(words::field(
                "Bileşenler (duvarın sol yüzünden)",
                table,
                Some(
                    "Kalınlığı boş bırakılan tek bileşen duvarın genişliğinden kalanı alır. Öncelik birleşimde: 1 en güçlü (çekirdek), 5 sıva; Kural 3'tür. Döner: yüzün dış katmanları duvar ucunda ve kapı ile pencere sövesinde döner."
                        .to_owned(),
                ),
            ))
            .push(
                row![words::secondary(ADD, Some(msg(Event::Add))), caps]
                    .spacing(16)
                    .align_y(Center),
            )
            .into()
    }
}

#[cfg(test)]
#[path = "wall_styles_layout_tests.rs"]
mod layout_tests;

/// A row's small button: its icon, enabled or not.
fn small(glyph: &'static str, enabled: bool, on: Message) -> Element<'static, Message> {
    button(
        container(kentos_ui::icon::icon(crate::icons::from_web(Some(glyph))).size(14.0))
            .center_x(Fill),
    )
    .on_press_maybe(enabled.then_some(on))
    .padding([4, 0])
    .width(iced::Length::Fixed(26.0))
    .style(style::button::ghost)
    .into()
}

/// KentOS's style's components as lines (it is not edited).
fn components_text<'a>(s: &WallStyleDef, format: &Format) -> Element<'a, Message> {
    let mut col = Column::new().spacing(2);
    for c in &s.components {
        let t = c
            .thickness
            .map_or_else(|| "kalan".to_owned(), |t| format.length(t));
        let h = c
            .hatch
            .as_ref()
            .map_or_else(String::new, |h| format!(", {}", h.pattern));
        let p = c
            .priority
            .map_or_else(String::new, |p| format!(", öncelik {p}"));
        col = col.push(label::caption(format!("{}: {t}{h}{p}", c.name)));
    }
    col.into()
}

/// The section of a straight wall of the style, at the detailed level, as the drawing draws it.
struct Sample {
    style: Option<WallStyleDef>,
    colors: crate::labels::Colors,
}

impl canvas::Program<Message> for Sample {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use kentos_geometry_core::entity::Shape;
        use kentos_geometry_core::geom::wall::{self, Detail, View};
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.colors.halo);
        let Some(style) = &self.style else {
            return vec![frame.into_geometry()];
        };
        let width =
            kentos_commands::fixed_width(&style.components).unwrap_or_else(|| style.new_width());
        // NaN is no width either.
        if width.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
            || kentos_commands::component_thicknesses(width, &style.components).is_none()
        {
            return vec![frame.into_geometry()];
        }
        let length = (width * 5.0).max(1.0);
        let fields = WallFields {
            pts: vec![
                kentos_commands::Vec2 { x: 0.0, y: 0.0 },
                kentos_commands::Vec2 { x: length, y: 0.0 },
            ],
            bulges: None,
            closed: false,
            width,
            justify: kentos_commands::WallJustify::Left,
            offset: 0.0,
            style: Some(style.id.clone()),
            components: style.components.clone(),
            caps: style.caps,
            fill: style.fill.clone(),
            height: None,
            arch: Default::default(),
            joins: Default::default(),
            cleanup: true,
        };
        let Shape::Wall {
            pts,
            bulges,
            wall: spec,
        } = kentos_formats::walls::core_wall(&fields)
        else {
            return vec![frame.into_geometry()];
        };
        let Ok(pieces) = wall::pieces(
            &pts,
            bulges.as_deref(),
            &spec,
            View {
                detail: Detail::Detailed,
                scale: 20.0,
            },
        ) else {
            return vec![frame.into_geometry()];
        };
        // The wall lies right of its path (left-justified): y from 0 down to −width.
        let pad = 16.0_f64;
        let k = ((f64::from(bounds.width) - 2.0 * pad) / length)
            .min((f64::from(bounds.height) - 2.0 * pad) / width);
        let (x0, y0) = (
            (f64::from(bounds.width) - length * k) / 2.0,
            (f64::from(bounds.height) - width * k) / 2.0,
        );
        let at = |p: kentos_geometry_core::Vec2| {
            Point::new((x0 + p.x * k) as f32, (y0 - p.y * k) as f32)
        };
        let ink = self.colors.label;
        let line = |frame: &mut Frame, pts: &[kentos_geometry_core::Vec2], closed: bool| {
            let path = Path::new(|b| {
                if let Some(first) = pts.first() {
                    b.move_to(at(*first));
                    for p in &pts[1..] {
                        b.line_to(at(*p));
                    }
                    if closed {
                        b.close();
                    }
                }
            });
            frame.stroke(&path, Stroke::default().with_color(ink).with_width(1.0));
        };
        for piece in &pieces {
            match &piece.shape {
                Shape::Hatch {
                    ring,
                    holes,
                    pattern,
                    ..
                } => {
                    if pattern.kind == "solid" {
                        let color = piece
                            .color
                            .as_deref()
                            .map_or(Color { a: 0.55, ..ink }, crate::view::hex_color);
                        let path = Path::new(|b| {
                            if let Some(first) = ring.first() {
                                b.move_to(at(*first));
                                for p in &ring[1..] {
                                    b.line_to(at(*p));
                                }
                                b.close();
                            }
                        });
                        frame.fill(&path, color);
                    } else {
                        let holes = holes.clone().unwrap_or_default();
                        let drawn = kentos_geometry_core::geom::hatch_pattern::pattern_pieces(
                            ring, &holes, pattern, 4000,
                        );
                        for [a, b] in drawn.segments {
                            frame.stroke(
                                &Path::line(at(a), at(b)),
                                Stroke::default()
                                    .with_color(Color { a: 0.7, ..ink })
                                    .with_width(0.8),
                            );
                        }
                    }
                }
                Shape::Polyline { pts, closed, .. } => line(&mut frame, pts, *closed),
                Shape::Line { a, b, .. } => line(&mut frame, &[*a, *b], false),
                _ => {}
            }
        }
        vec![frame.into_geometry()]
    }
}
