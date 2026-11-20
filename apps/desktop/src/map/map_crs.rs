//! The project's map system on the desktop (docs/adr/0583 §3): the system the drawing area draws in, a view setting
//! of the project written with `cad.settings.set`'s `mapSrid`; nothing in the drawing moves.
//!
//! - the coordinate system cell's right-click menu: Harita sistemi ▸ (the project's own, Web Mercator, WGS 84, then
//!   the registry's systems by datum);
//! - the status bar's cell beside the coordinates: the map system's short name and the cursor in it.

use crate::map;
use iced::Element;
use iced::widget::row;
use kentos_commands::SettingsSet;
use kentos_interaction::{Format, Level, Vec2};
use kentos_native_application::ExecutionContext;
use kentos_ui::label;
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};

use crate::app::{App, Message};
use crate::crs;

/// The window's title (GIS-054).
pub const TITLE: &str = "Harita sistemi";

/// The map system chosen; none: the project's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choose(pub Option<u32>);

fn message(srid: Option<u32>) -> Message {
    Message::Map(map::Event::MapCrs(Choose(srid)))
}

/// The systems a web map and a GPS speak first.
const FIRST: [u32; 2] = [3857, 4326];

impl App {
    /// Harita sistemi chosen (docs/adr/0583 §3): a setting of the project, never a transformation of its objects.
    pub(crate) fn choose_map(&mut self, Choose(srid): Choose) {
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().map_srid == srid {
            return;
        }
        let result = kentos_native_application::settings_set::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SettingsSet {
                // 0 takes the map system away: the project's own again.
                map_srid: Some(srid.unwrap_or(0)),
                ..Default::default()
            },
        );
        if self.answered(result).is_some() {
            self.say(
                Level::Success,
                match srid {
                    None => "Harita sistemi: projenin sistemi.".to_owned(),
                    Some(s) => format!(
                        "Harita sistemi: {}. Çizim dönüştürülmedi; yalnız görünüm.",
                        crs::title_of(s)
                    ),
                },
            );
        }
    }

    /// Harita sistemi from the ribbon or the command line (GIS-054): its window, the systems listed at once, whatever
    /// the status bar shows.
    pub(crate) fn map_crs_run(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.dialog = Some(crate::app::Dialog::Map(map::MapDialogKind::MapCrs));
    }

    /// Harita sistemi's window: the objects' system and the map's, each named; the project's own, the web map's and
    /// GPS's as buttons, every other system of the registry by datum in a searchable list; Kapat.
    pub(crate) fn map_crs_view(&self) -> Element<'_, Message> {
        use crate::exchange::words::{self, Kind};
        let Some(doc) = &self.document else {
            return iced::widget::text("").into();
        };
        let settings = doc.settings();
        let close = Message::Map(map::Event::MapCrsClose);
        let mut body = iced::widget::Column::new().spacing(10);
        if !settings.has_system() {
            body = body.push(words::text_line(Kind::Warn, "Yerel proje başka sistemde gösterilmez."));
            return kentos_ui::widget::Dialog::new(TITLE)
                .push(body)
                .action(words::primary("Kapat", Some(close.clone())))
                .width(520.0)
                .modal(close);
        }
        let current = settings.map_srid.filter(|m| *m != settings.srid);
        body = body
            .push(words::text_line(
                Kind::Info,
                format!("Nesnelerin saklandığı sistem: {}.", crs::title_of(settings.srid)),
            ))
            .push(words::text_line(
                Kind::Info,
                format!(
                    "Harita şimdi şu sistemde çiziliyor: {}.",
                    current.map_or_else(|| "projenin sistemi".to_owned(), crs::title_of)
                ),
            ));
        let pick = |caption: String, srid: Option<u32>| {
            let chosen = current == srid;
            iced::widget::button(label::body(caption))
                .on_press(message(srid))
                .padding(kentos_ui::theme::metrics::action())
                .style(if chosen { kentos_ui::style::button::primary } else { kentos_ui::style::button::secondary })
        };
        let mut quick = iced::widget::Row::new().spacing(8);
        quick = quick.push(pick(format!("Projenin sistemi (EPSG:{})", settings.srid), None));
        for srid in FIRST.into_iter().filter(|s| *s != settings.srid) {
            if let Some(s) = kentos_project::crs::system(srid) {
                quick = quick.push(pick(s.name.clone(), Some(srid)));
            }
        }
        body = body.push(quick);
        let others: Vec<(u32, String)> = crate::second_crs::choices(settings.srid)
            .into_iter()
            .flat_map(|(datum, systems)| {
                systems.into_iter().filter(|s| !FIRST.contains(&s.srid)).map(move |s| (s.srid, format!("{datum} · {} (EPSG:{})", s.name, s.srid)))
            })
            .collect();
        let at = current.and_then(|c| others.iter().position(|(s, _)| *s == c));
        let ids: Vec<u32> = others.iter().map(|(s, _)| *s).collect();
        let select = kentos_ui::widget::select::Select::new(
            others.iter().map(|(_, n)| kentos_ui::widget::select::Choice::new(n.clone())),
            at,
            move |i| message(ids.get(i).copied()),
        )
        .placeholder("Başka bir sistem ara…");
        body = body.push(words::field("Başka sistem", select, Some("Çizim dönüştürülmez; yalnız haritanın çizildiği sistem değişir.".to_owned())));
        kentos_ui::widget::Dialog::new(TITLE)
            .push(body)
            .action(words::primary("Kapat", Some(close.clone())))
            .width(620.0)
            .modal(close)
    }

    /// Harita sistemi: the project's own, the web map's and GPS's, a submenu of systems for each datum.
    pub(crate) fn map_menu(&self) -> Menu<Message> {
        let Some(doc) = &self.document else {
            return Menu::new();
        };
        let settings = doc.settings();
        if !settings.has_system() {
            return Menu::new().item("Yerel proje başka sistemde gösterilmez", None);
        }
        let current = settings.map_srid.filter(|m| *m != settings.srid);
        let mut menu = Menu::new()
            .header("Harita sistemi")
            .radio("Projenin sistemi", current.is_none(), message(None))
            .hint(format!("EPSG:{}", settings.srid));
        for srid in FIRST.into_iter().filter(|s| *s != settings.srid) {
            if let Some(s) = kentos_project::crs::system(srid) {
                menu = menu
                    .radio(s.name.clone(), current == Some(srid), message(Some(srid)))
                    .hint(format!("EPSG:{srid}"));
            }
        }
        menu = menu.separator();
        for (datum, systems) in crate::second_crs::choices(settings.srid) {
            let chosen = systems
                .iter()
                .find(|s| current == Some(s.srid) && !FIRST.contains(&s.srid))
                .map(|s| s.name.clone());
            let list = systems.iter().fold(Menu::new(), |list, s| {
                list.radio(
                    s.name.clone(),
                    current == Some(s.srid),
                    message(Some(s.srid)),
                )
                .hint(format!("EPSG:{}", s.srid))
            });
            menu = menu.submenu(datum, list);
            if let Some(name) = chosen {
                menu = menu.hint(name);
            }
        }
        menu
    }

    /// The cursor's values in the map system: “E 3226837.120   N 4865123.004”, “40°59′38.0581″K 28°58′45.7302″D”.
    fn map_values(&self, cursor: Option<Vec2>) -> Option<(String, String)> {
        // The cell is a map's programs' (module_surfaces.rs).
        if !self.shows_cell(crate::module_surfaces::Cell::MapCrs) {
            return None;
        }
        let doc = self.document.as_ref()?;
        let settings = doc.settings();
        let map = kentos_project::systems::map_projection(settings)?;
        let system = kentos_project::crs::system(settings.map_srid?)?;
        let f = Format::of(settings);
        let geographic = map.map.is_geographic();
        let at = cursor.and_then(|p| map.forward(kentos_geometry_core::Vec2::new(p.x, p.y)));
        let values = match (at, geographic) {
            (Some(q), true) => format!(
                "{}  {}",
                self.draft.geographic.write(q.y, true),
                self.draft.geographic.write(q.x, false)
            ),
            (Some(q), false) => format!(
                "{} {}   {} {}",
                f.east_label(),
                f.coord(q.x),
                f.north_label(),
                f.coord(q.y)
            ),
            (None, true) => "—  —".to_owned(),
            (None, false) => format!("{} —   {} —", f.east_label(), f.north_label()),
        };
        // The code, short in the bar; the name in the tip.
        Some((format!("EPSG:{}", system.srid), values))
    }

    /// The status bar's cell of the map system, when the project has one: its code; the tip says the pointer on
    /// the map, a click lists Harita sistemi.
    pub(crate) fn map_cell(&self, cursor: Option<Vec2>) -> Option<Element<'_, Message>> {
        let (name, values) = self.map_values(cursor)?;
        let face = row![label::muted("Harita"), label::mono(name)]
            .spacing(6)
            .align_y(iced::Center);
        let title = self
            .document
            .as_ref()
            .and_then(|d| d.settings().map_srid)
            .map_or_else(String::new, crs::title_of);
        Some(tip(
            MenuButton::new(iced::widget::container(face).padding([0, 8]), move || {
                self.map_menu()
            }),
            Tip::new(format!("Harita sistemi: {title}")).body(format!(
                "İmleç haritada: {values}. Çizim bu sistemde gösteriliyor; nesneler, ölçüler ve yazılanlar projenin \
                 sistemindedir. Sistemi değiştirmek için tıklayın."
            )),
            iced::widget::tooltip::Position::Top,
        ))
    }

    /// About how wide the map cell is with its separator; none without a map system.
    pub(crate) fn map_width(&self) -> f32 {
        let Some((name, _)) = self.map_values(None) else {
            return 0.0;
        };
        let size = kentos_ui::theme::typography::body();
        ("Harita ".len() + name.chars().count()) as f32 * 0.62 * size + 6.0 + 16.0 + 9.0
    }
}
