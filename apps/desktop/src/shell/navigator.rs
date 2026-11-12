//! Gezgin (docs/adr/0778; Revit's Project Browser, Archicad's Navigator): the project's levels, named views, sheets
//! and schedules, as `cad.navigator.read` reads them, and where the one selected object shows. A level's row makes it
//! the active level, a view's row shows it, a sheet's or a map's row opens the sheet, a schedule's or a tag's row
//! selects it and shows it; the selected object's section comes first. The answer is read again only when the drawing's
//! revision or the selection changes.

use std::cell::RefCell;
use std::collections::HashSet;

use iced::widget::{Column, button, column, container, row, text};
use iced::{Center, Element, Fill, Task, Theme};
use kentos_commands::{CommandResult, NavigatorRead, NavigatorReadResult};
use kentos_domain::{Slot, Uuid};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// The tree's sections, each folded or open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Levels,
    Views,
    Sheets,
    Schedules,
    Object,
}

impl Section {
    fn title(self) -> &'static str {
        match self {
            Section::Levels => "Katlar",
            Section::Views => "Görünümler",
            Section::Sheets => "Paftalar",
            Section::Schedules => "Çizelgeler",
            Section::Object => "Seçili nesne",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Fold(Section),
    /// A level's row: it becomes the active level; none: Katsız.
    Level(Option<String>),
    /// A view's row, by the view's name.
    View(String),
    /// A sheet's row, by its place in the book.
    Sheet(usize),
    /// A schedule's or a tag's row: those objects selected and shown.
    Objects(Vec<String>),
}

fn msg(e: Event) -> Message {
    Message::Navigator(e)
}

/// The panel's state: its folded sections, and the last answer with the revision and the object it was read for.
#[derive(Default)]
pub struct PanelState {
    pub folded: HashSet<Section>,
    read: RefCell<Option<(String, Option<String>, NavigatorReadResult)>>,
}

impl std::fmt::Debug for PanelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanelState").field("folded", &self.folded).finish()
    }
}

impl App {
    /// `arch.navigator`: the dock shows the Gezgin tab.
    pub(crate) fn show_navigator(&mut self) -> Task<Message> {
        if !self.right_panel_shown() {
            self.toggle_right_panel();
        }
        self.docks
            .update(kentos_ui::widget::docking::Event::Selected(crate::app::Panel::Navigator));
        Task::none()
    }

    /// The one selected object's persistent id; none for no or several.
    fn navigator_object(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        match self.selection.ids() {
            [slot] => doc.model.uid(*slot).map(|u| u.to_string()),
            _ => None,
        }
    }

    /// `cad.navigator.read`'s answer for the drawing as it is, read again when its revision or the object changed.
    pub(crate) fn navigator_read(&self) -> Option<NavigatorReadResult> {
        let doc = self.document.as_ref()?;
        let revision = doc.model.revision().to_string();
        let uid = self.navigator_object();
        let mut kept = self.navigator.read.borrow_mut();
        if let Some((r, u, answer)) = kept.as_ref()
            && *r == revision
            && *u == uid
        {
            return Some(answer.clone());
        }
        let input = NavigatorRead { uid: uid.clone() };
        let CommandResult::Completed { output, .. } =
            kentos_native_application::navigator::plan_in(&doc.model, &input)
        else {
            return None;
        };
        *kept = Some((revision, uid, output.clone()));
        Some(output)
    }

    pub(crate) fn navigator_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Fold(s) => {
                if !self.navigator.folded.remove(&s) {
                    self.navigator.folded.insert(s);
                }
            }
            Event::Level(level) => self.choose_level(level),
            Event::View(name) => self.show_named_view(&name),
            Event::Sheet(k) => return self.update(Message::Sheet(kentos_sheet_ui::Message::Tab(k + 1))),
            Event::Objects(uids) => {
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                let slots: Vec<Slot> = uids
                    .iter()
                    .filter_map(|u| Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
                self.selection.set(slots);
                if !ids.is_empty() {
                    self.navigating(|app| app.zoom_to_objects(&ids));
                }
            }
        }
        Task::none()
    }

    pub(crate) fn navigator_view(&self) -> Element<'_, Message> {
        let Some(nav) = self.navigator_read() else {
            return container(label::caption("Açık çizim yok.")).padding([8, 12]).into();
        };
        let f = self.format();
        let level_name = |id: &str| {
            nav.levels
                .iter()
                .find(|l| l.id == id)
                .map_or_else(|| id.to_owned(), |l| l.name.clone())
        };
        let view_name = |id: &str| {
            self.named_views()
                .iter()
                .find(|v| kentos_commands::view_key(v) == id)
                .map(|v| v.name.clone())
        };
        let sheet_at = |id: &str| nav.sheets.iter().position(|s| s.id == id);
        let active = self.active_level();
        let mut out = Column::new().spacing(2).padding([4, 0]);

        // Seçili nesne: first, where the one selected object shows.
        out = out.push(self.navigator_head(Section::Object, usize::from(nav.object.is_some())));
        if !self.navigator.folded.contains(&Section::Object) {
            match &nav.object {
                None => out = out.push(note("Bir nesne seçin: hangi katta, görünümde, paftada ve çizelgede olduğu burada.")),
                Some(o) => {
                    let level = o.level.as_deref().map_or("Katsız".to_owned(), level_name);
                    out = out.push(entry(
                        crate::selecting::kind_title(&o.kind),
                        level,
                        false,
                        0,
                        Some(msg(Event::Objects(vec![o.uid.clone()]))),
                    ));
                    if o.views.is_empty() && o.maps.is_empty() {
                        out = out.push(note("Hiçbir görünümde ve paftada çizilmiyor."));
                    }
                    for v in &o.views {
                        let name = view_name(v).unwrap_or_else(|| v.clone());
                        out = out.push(entry(&name, "görünüm".to_owned(), false, 1, Some(msg(Event::View(name.clone())))));
                    }
                    for m in &o.maps {
                        let Some(k) = sheet_at(&m.sheet) else { continue };
                        let sheet = &nav.sheets[k];
                        let map = sheet.maps.iter().find(|x| x.item == m.item).map_or(m.item.as_str(), |x| x.name.as_str());
                        out = out.push(entry(&sheet.name, format!("harita: {map}"), false, 1, Some(msg(Event::Sheet(k)))));
                    }
                    for s in &o.schedules {
                        let of = nav.schedules.iter().find(|x| x.uid == *s).map_or("Çizelge", |x| x.of.label());
                        out = out.push(entry(of, "listeliyor".to_owned(), false, 1, Some(msg(Event::Objects(vec![s.clone()])))));
                    }
                    if !o.tags.is_empty() {
                        out = out.push(entry(
                            "Etiket",
                            format!("{} etiket adlandırıyor", o.tags.len()),
                            false,
                            1,
                            Some(msg(Event::Objects(o.tags.clone()))),
                        ));
                    }
                }
            }
        }
        // Katlar.
        out = out.push(self.navigator_head(Section::Levels, nav.levels.len()));
        if !self.navigator.folded.contains(&Section::Levels) {
            if nav.levels.is_empty() {
                out = out.push(note("Projede kat yok: Proje ayarları › Mimari'de kat ekleyin."));
            }
            for l in &nav.levels {
                let about = format!(
                    "{} · {} nesne · {} görünüm · {} harita",
                    f.length(l.elevation),
                    l.objects,
                    l.views.len(),
                    l.maps.len()
                );
                let on = active.as_deref() == Some(l.id.as_str());
                out = out.push(entry(&l.name, about, on, 0, Some(msg(Event::Level(Some(l.id.clone()))))));
            }
            out = out.push(entry(
                "Katsız",
                format!("{} nesne", nav.unleveled),
                active.is_none(),
                0,
                Some(msg(Event::Level(None))),
            ));
        }

        // Görünümler.
        out = out.push(self.navigator_head(Section::Views, nav.views.len()));
        if !self.navigator.folded.contains(&Section::Views) {
            if nav.views.is_empty() {
                out = out.push(note("Adlı görünüm yok: Görünüm › Görünümler'den kaydedin."));
            }
            for v in &nav.views {
                let mut parts = vec![v.level.as_deref().map_or("Bütün katlar".to_owned(), level_name)];
                if let Some(s) = v.scale {
                    parts.push(format!("1:{s}"));
                }
                parts.push(match v.recipe {
                    Some(n) => format!("tarif {n}"),
                    None => "tarifsiz".to_owned(),
                });
                parts.push(format!("{} harita", v.maps.len()));
                out = out.push(entry(&v.name, parts.join(" · "), false, 0, Some(msg(Event::View(v.name.clone())))));
            }
        }

        // Paftalar.
        out = out.push(self.navigator_head(Section::Sheets, nav.sheets.len()));
        if !self.navigator.folded.contains(&Section::Sheets) {
            if nav.sheets.is_empty() {
                out = out.push(note("Pafta yok: Mimari › Proje › Kat planları ile her kata pafta açın."));
            }
            for (k, s) in nav.sheets.iter().enumerate() {
                out = out.push(entry(
                    &s.name,
                    format!("{} · {} harita", s.series, s.maps.len()),
                    false,
                    0,
                    Some(msg(Event::Sheet(k))),
                ));
                for m in &s.maps {
                    let mut parts = Vec::new();
                    if let Some(scale) = m.scale {
                        parts.push(format!("1:{}", scale.round()));
                    }
                    parts.push(m.level.as_deref().map_or("bütün katlar".to_owned(), level_name));
                    match (&m.view, m.missing) {
                        (Some(_), true) => parts.push("görünümü silinmiş".to_owned()),
                        (Some(v), false) => parts.push(format!("görünüm: {}", view_name(v).unwrap_or_else(|| v.clone()))),
                        (None, _) => {}
                    }
                    out = out.push(entry(&m.name, parts.join(" · "), false, 1, Some(msg(Event::Sheet(k)))));
                }
            }
        }

        // Çizelgeler.
        out = out.push(self.navigator_head(Section::Schedules, nav.schedules.len()));
        if !self.navigator.folded.contains(&Section::Schedules) {
            if nav.schedules.is_empty() {
                out = out.push(note("Çizelge yok: Mimari › Çizelge'den ekleyin."));
            }
            for s in &nav.schedules {
                out = out.push(entry(
                    s.of.label(),
                    format!("{} nesne", s.objects),
                    false,
                    0,
                    Some(msg(Event::Objects(vec![s.uid.clone()]))),
                ));
            }
        }

        out.into()
    }

    fn navigator_head(&self, section: Section, count: usize) -> Element<'_, Message> {
        let folded = self.navigator.folded.contains(&section);
        let mark = if folded { "▸" } else { "▾" };
        let face = row![
            label::caption(mark),
            text(section.title()).font(typography::ui_strong()).size(typography::body()),
            container(label::caption(count.to_string())).align_right(Fill),
        ]
        .spacing(6)
        .align_y(Center)
        .padding([4, 8]);
        button(face)
            .on_press(msg(Event::Fold(section)))
            .padding(0)
            .width(Fill)
            .style(style::button::row(false))
            .into()
    }
}

/// A row: its name over its line of facts, `depth` steps in; strong when it is the current one.
fn entry<'a>(name: &str, about: String, current: bool, depth: u16, on: Option<Message>) -> Element<'a, Message> {
    let font = if current { typography::ui_strong() } else { typography::ui() };
    let words = column![
        text(name.to_owned()).font(font).size(typography::body()).wrapping(text::Wrapping::None),
        label::caption(about)
            .wrapping(text::Wrapping::None)
            .style(|t: &Theme| text::Style { color: Some(Tokens::of(t).faint) }),
    ]
    .spacing(1);
    let face = container(words).padding(iced::Padding {
        top: 3.0,
        right: 8.0,
        bottom: 3.0,
        left: 20.0 + 14.0 * f32::from(depth),
    });
    let mut b = button(face).padding(0).width(Fill).style(style::button::row(current));
    if let Some(m) = on {
        b = b.on_press(m);
    }
    b.into()
}

fn note<'a>(words: &'static str) -> Element<'a, Message> {
    container(label::caption(words)).padding([4, 20]).width(Fill).into()
}
