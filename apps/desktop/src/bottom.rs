//! The bottom panel (the web's `BottomPanel`, ui/bottom/BottomPanel.ts): the
//! command line is always there; F2 (`view.bottomPanel`) opens a panel over
//! it with three tabs:
//!
//! - Komut geçmişi: the whole history (the command line's own, open);
//! - Koordinat listesi (`view.coords` opens it on this tab): the selection's
//!   points with Y, X, Z and layer, or the first object's vertices with each
//!   edge's length and bearing, and the outline's area or length; rows are
//!   built only as they scroll into view (a contour has thousands);
//! - Uyarılar: the warnings and errors; the tab counts the ones not seen yet.
//!
//! As on the web, the tab row ends with Geçmişi temizle and Paneli kapat,
//! and the panel's top edge is dragged to size it (a double click puts the
//! first height back).
//!
//! Numbers are the project's formats; the geometry is the shared core's.

use iced::widget::{Column, button, column, container, row};
use iced::{Element, Fill, Length};
use kentos_contracts::Entity;
use kentos_interaction::{Format, bearing_grad, dist, measures, vertices};
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::widget::command_line::{self, Entry};
use kentos_ui::widget::sash::Sash;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::tabs::{self, Tab, Tabs};
use kentos_ui::widget::{Tip, horizontal_divider, tip};

use crate::app::{App, Message};

/// The panel's tabs, in the web's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BottomTab {
    #[default]
    History,
    Coords,
    Messages,
}

impl BottomTab {
    const ALL: [BottomTab; 3] = [BottomTab::History, BottomTab::Coords, BottomTab::Messages];
}

/// The open history's least height: five lines.
const LEAST_LOG: f32 = 98.0;
/// The tab row's two buttons (Geçmişi temizle, Paneli kapat) and their margins.
const ACTIONS_WIDTH: f32 = 60.0;
/// What the panel leaves above it at least, whatever it is dragged to: the
/// ribbon, some of the drawing, the tab row, the command input and the status bar.
const KEEP: f32 = 360.0;

/// What the coordinate list shows (built from the selection, kept small:
/// the rows are formatted as they scroll into view).
enum Listing {
    Empty,
    /// Every selected object is a point.
    Points(Vec<Point>),
    /// The first object that is not a point or a text: its vertices, in
    /// rings (a polygon's outer ring, then each hole), each edge within its ring.
    Vertices {
        pts: Vec<kentos_interaction::Vec2>,
        rings: Vec<Ring>,
        footer: String,
    },
}

/// A run of the vertex list that forms one ring; `closed`: its last vertex
/// has an edge back to its first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ring {
    start: usize,
    end: usize,
    closed: bool,
}

/// The vertex after `i` along its ring, if the ring goes on.
fn next_in(rings: &[Ring], i: usize) -> Option<usize> {
    let ring = rings.iter().find(|r| (r.start..r.end).contains(&i))?;
    if i + 1 < ring.end {
        Some(i + 1)
    } else {
        ring.closed.then_some(ring.start)
    }
}

struct Point {
    name: String,
    x: f64,
    y: f64,
    z: Option<f64>,
    layer: String,
}

impl App {
    /// The command line, and the panel over it while it is open.
    pub(crate) fn bottom(&self) -> Element<'_, Message> {
        if !self.command_expanded {
            return self.command_line_as(false, None);
        }
        let unseen = self.warning_count().saturating_sub(self.seen_warnings);
        let tabs = Tabs::new(
            [
                Tab::new("Komut geçmişi").icon(Icon::Clock).closable(false),
                Tab::new("Koordinat listesi")
                    .icon(Icon::Table)
                    .closable(false),
                Tab::new(if unseen > 0 && self.bottom_tab != BottomTab::Messages {
                    format!("Uyarılar ({unseen})")
                } else {
                    "Uyarılar".to_owned()
                })
                .icon(Icon::Warning)
                .closable(false),
            ],
            BottomTab::ALL
                .iter()
                .position(|t| *t == self.bottom_tab)
                .unwrap_or(0),
            |i| Message::BottomTab(BottomTab::ALL[i]),
        );
        let log = self.bottom_log();
        // The top edge sizes the panel; the drawing gives or takes the room.
        let sash = Sash::horizontal(log, Message::BottomResized)
            .reverse()
            .range(LEAST_LOG..=f32::INFINITY)
            .keep(KEEP)
            .on_double_click(Message::BottomReset);
        let bar = row![container(tabs).width(Fill), self.bottom_actions()].width(Fill);
        let panel = match self.bottom_tab {
            BottomTab::History => {
                return column![sash, bar, self.command_line_as(true, None)].into();
            }
            BottomTab::Coords => self.coordinate_list(),
            BottomTab::Messages => self.messages(),
        };
        // As tall as the open history, so switching tabs does not move the drawing.
        let height = command_line::height_with_log(log) - command_line::height(0, false);
        column![
            sash,
            bar,
            // On the command line's own ground, so a tab reads as part of it.
            container(panel)
                .width(Fill)
                .height(Length::Fixed(height))
                .style(|theme: &iced::Theme| container::Style {
                    background: Some(kentos_ui::theme::Tokens::of(theme).field.into()),
                    ..container::Style::default()
                }),
            self.command_line_as(false, Some(0)),
        ]
        .into()
    }

    /// The open history's height: dragged, or the command line's own.
    pub(crate) fn bottom_log(&self) -> f32 {
        self.bottom_log
            .unwrap_or_else(command_line::default_log_height)
            .max(LEAST_LOG)
    }

    /// The tab row's end, as on the web: Geçmişi temizle and Paneli kapat, on
    /// the tab strip's own ground and above its line.
    fn bottom_actions(&self) -> Element<'_, Message> {
        let action = |glyph: &str, message: Message, about: Tip| {
            tip(
                button(kentos_ui::icon::icon(crate::icons::from_web(Some(glyph))).size(15.0))
                    .on_press(message)
                    .padding([4, 5])
                    .style(kentos_ui::style::button::flat),
                about,
                iced::widget::tooltip::Position::Top,
            )
        };
        let buttons = row![
            action("clear", Message::HistoryCleared, Tip::new("Geçmişi temizle")),
            action(
                "chevronDown",
                Message::CommandHistoryToggled,
                Tip::new("Paneli kapat").detail("F2"),
            ),
        ]
        .spacing(2)
        .padding([0, 4])
        .height(tabs::height() - 1.0)
        .align_y(iced::Center);
        // A fixed width: the strip's line under the buttons would take the row's
        // whole width otherwise, and the tabs half of it.
        container(column![buttons, horizontal_divider()])
            .width(ACTIONS_WIDTH)
            .style(|theme: &iced::Theme| container::Style {
                background: Some(kentos_ui::theme::Tokens::of(theme).window.into()),
                ..container::Style::default()
            })
            .into()
    }

    /// Geçmişi temizle: the history goes, and with it the warnings' count.
    pub(crate) fn clear_history(&mut self) {
        self.history.clear();
        self.seen_warnings = 0;
        self.last_level = None;
    }

    /// Warnings and errors in the history.
    fn warning_count(&self) -> usize {
        self.history
            .iter()
            .filter(|e| matches!(e, Entry::Warning(_) | Entry::Error(_)))
            .count()
    }

    /// Opens the panel on a tab (`view.coords`, a tab clicked).
    pub(crate) fn show_bottom(&mut self, tab: BottomTab) {
        self.bottom_tab = tab;
        self.command_expanded = true;
        if tab == BottomTab::Messages {
            self.seen_warnings = self.warning_count();
        }
    }

    /// `view.bottomPanel` (F2): opens or closes the panel.
    pub(crate) fn toggle_bottom(&mut self) {
        if self.command_expanded {
            self.command_expanded = false;
        } else {
            self.show_bottom(self.bottom_tab);
        }
    }

    fn messages(&self) -> Element<'_, Message> {
        let list: Vec<Element<'_, Message>> = self
            .history
            .iter()
            .filter_map(|e| match e {
                Entry::Warning(t) => Some(
                    label::body(t.as_str())
                        .style(|theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(kentos_ui::theme::Tokens::of(theme).warning),
                        })
                        .into(),
                ),
                Entry::Error(t) => Some(
                    label::body(t.as_str())
                        .style(kentos_ui::style::text::danger)
                        .into(),
                ),
                _ => None,
            })
            .collect();
        if list.is_empty() {
            return container(label::muted("Uyarı yok.")).padding(12).into();
        }
        iced::widget::scrollable(Column::with_children(list).spacing(4).padding([6, 12]))
            .direction(kentos_ui::style::field::body_scrollbar())
            .anchor_bottom()
            .height(Fill)
            .into()
    }

    fn listing(&self) -> Listing {
        let Some(doc) = &self.document else {
            return Listing::Empty;
        };
        let model = &doc.model;
        let entities: Vec<&Entity> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| model.get(*s))
            .collect();
        if entities.is_empty() {
            return Listing::Empty;
        }
        let layer_name = |id: &str| {
            model
                .layers()
                .get(id)
                .map_or_else(String::new, |l| l.name.clone())
        };
        if entities.iter().all(|e| matches!(e, Entity::Point(_))) {
            return Listing::Points(
                entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Point(p) => Some(Point {
                            name: p
                                .base
                                .label
                                .clone()
                                .unwrap_or_else(|| format!("#{}", p.base.id)),
                            x: p.p.x,
                            y: p.p.y,
                            z: p.z,
                            layer: layer_name(&p.base.layer_id),
                        }),
                        _ => None,
                    })
                    .collect(),
            );
        }
        let e = entities
            .iter()
            .find(|e| !matches!(e, Entity::Point(_) | Entity::Text(_)))
            .unwrap_or(&entities[0]);
        let pts = vertices(e);
        // The core lists a polygon's outer ring, then its holes; each is its own ring.
        let rings = match e {
            Entity::Polygon(p) => {
                let mut rings = vec![Ring {
                    start: 0,
                    end: p.pts.len(),
                    closed: true,
                }];
                for hole in p.holes.iter().flatten() {
                    let start = rings.last().map_or(0, |r| r.end);
                    rings.push(Ring {
                        start,
                        end: start + hole.pts.len(),
                        closed: true,
                    });
                }
                rings
            }
            _ => vec![Ring {
                start: 0,
                end: pts.len(),
                closed: false,
            }],
        };
        let base = e.base();
        let title = match &base.label {
            Some(label) => match base.attrs.get("Ada") {
                Some(ada) if !ada.is_empty() => format!("{ada} ada {label}"),
                _ => label.clone(),
            },
            None => format!("#{}", base.id),
        };
        let format = Format::of(doc.settings());
        // The object's own area and length (arcs and holes counted), as the
        // properties panel shows them; never the vertex list's.
        let footer = match measures(e) {
            (Some(area), length) => format!(
                "{title}   Alan {}{}",
                format.area(area),
                length.map_or_else(String::new, |l| format!("   Çevre {}", format.length(l)))
            ),
            (None, Some(length)) => format!("{title}   Uzunluk {}", format.length(length)),
            (None, None) => title,
        };
        let footer = if entities.len() > 1 {
            format!("{footer}   (ilk nesne gösteriliyor)")
        } else {
            footer
        };
        Listing::Vertices { pts, rings, footer }
    }

    fn coordinate_list(&self) -> Element<'_, Message> {
        let format = self
            .document
            .as_ref()
            .map_or_else(Format::default, |d| Format::of(d.settings()));
        let number = |heading: &'static str| {
            TableColumn::new(heading)
                .width(Length::FillPortion(2))
                .align_right()
        };
        let (table, footer) = match self.listing() {
            Listing::Empty => {
                return container(label::muted(
                    "Koordinat listesi için çizimde bir nesne seçin.",
                ))
                .padding(12)
                .into();
            }
            Listing::Points(points) => {
                let count = points.len();
                let table = Table::new([
                    TableColumn::new("Nokta").width(Length::FillPortion(2)),
                    number("Y (sağa)"),
                    number("X (yukarı)"),
                    number("Z (kot)"),
                    TableColumn::new("Katman").width(Length::FillPortion(2)),
                ])
                .virtualized(count, move |i| {
                    let p = &points[i];
                    TableRow::new([
                        label::body(p.name.clone()).into(),
                        label::mono(format.coord(p.x)).into(),
                        label::mono(format.coord(p.y)).into(),
                        label::mono(p.z.map_or_else(|| "—".to_owned(), |z| format.length_bare(z)))
                            .into(),
                        label::body(p.layer.clone()).into(),
                    ])
                });
                (table, format!("{count} nokta"))
            }
            Listing::Vertices { pts, rings, footer } => {
                let count = pts.len();
                let table =
                    Table::new([
                        TableColumn::new("Köşe")
                            .width(Length::FillPortion(1))
                            .align_right(),
                        number("Y (sağa)"),
                        number("X (yukarı)"),
                        number("Kenar (m)"),
                        TableColumn::new(format!("Semt ({})", format.angle_unit_label()))
                            .width(Length::FillPortion(2))
                            .align_right(),
                    ])
                    .virtualized(count, move |i| {
                        let p = pts[i];
                        let next = next_in(&rings, i).map(|n| pts[n]);
                        TableRow::new([
                            label::mono((i + 1).to_string()).into(),
                            label::mono(format.coord(p.x)).into(),
                            label::mono(format.coord(p.y)).into(),
                            label::mono(
                                next.map_or_else(String::new, |n| format.length_bare(dist(p, n))),
                            )
                            .into(),
                            label::mono(next.map_or_else(String::new, |n| {
                                format.bearing_bare(bearing_grad(p, n))
                            }))
                            .into(),
                        ])
                    });
                (table, footer)
            }
        };
        column![
            container(table).height(Fill),
            container(label::mono_caption(footer)).padding([4, 12]),
        ]
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::BottomTab;
    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;

    fn run(app: &mut App, id: &'static str) {
        let _ = app.update(Message::Run(id));
    }

    #[test]
    fn f2_opens_and_closes_the_panel_on_its_last_tab_and_view_coords_opens_the_list() {
        let mut app = app_with_drawing();
        assert!(!app.command_expanded);
        run(&mut app, "view.bottomPanel");
        assert!(app.command_expanded);
        assert_eq!(app.bottom_tab, BottomTab::History);
        run(&mut app, "view.bottomPanel");
        assert!(!app.command_expanded);
        run(&mut app, "view.coords");
        assert!(app.command_expanded);
        assert_eq!(app.bottom_tab, BottomTab::Coords);
        // Closed and opened again: the tab it was on.
        run(&mut app, "view.bottomPanel");
        run(&mut app, "view.bottomPanel");
        assert_eq!(app.bottom_tab, BottomTab::Coords);
    }

    #[test]
    fn the_list_shows_the_selections_points_or_the_first_objects_vertices() {
        let mut app = app_with_drawing();
        let doc = &app.document.as_ref().expect("open").model;
        // A store id, and a slot, is the object's id (spatial.rs).
        let slot_of = |kind: &str| {
            doc.entities()
                .find(|e| e.kind() == kind)
                .map(|e| kentos_domain::Slot(e.base().id))
                .expect(kind)
        };
        let (point, polygon) = (slot_of("point"), slot_of("polygon"));
        app.selection.set(vec![point]);
        match app.listing() {
            super::Listing::Points(points) => {
                assert_eq!(points.len(), 1);
                assert_eq!(points[0].name, "P1");
            }
            _ => panic!("the point's row"),
        }
        app.selection.set(vec![point, polygon]);
        match app.listing() {
            super::Listing::Vertices { pts, rings, footer } => {
                assert!(pts.len() >= 3);
                // The sample parcel has one hole: two closed rings, no edge between them.
                assert_eq!(rings.len(), 2);
                assert!(rings.iter().all(|r| r.closed));
                assert_eq!(super::next_in(&rings, rings[0].end - 1), Some(0));
                assert_eq!(
                    super::next_in(&rings, rings[1].end - 1),
                    Some(rings[1].start)
                );
                // The area is the parcel's own (the properties panel's), not the vertex list's.
                assert!(footer.contains("Alan 742.26 m²"), "{footer}");
                assert!(footer.contains("Çevre"), "{footer}");
                assert!(footer.ends_with("(ilk nesne gösteriliyor)"), "{footer}");
            }
            _ => panic!("the parcel's vertices"),
        }
        app.selection.clear();
        assert!(matches!(app.listing(), super::Listing::Empty));
    }

    #[test]
    fn warnings_are_counted_until_their_tab_is_seen() {
        let mut app = app_with_drawing();
        app.warn("Bir uyarı.");
        app.error("Bir hata.");
        assert_eq!(app.warning_count() - app.seen_warnings, 2);
        let _ = app.update(Message::BottomTab(BottomTab::Messages));
        assert_eq!(app.warning_count() - app.seen_warnings, 0);
    }

    #[test]
    fn geçmişi_temizle_empties_the_history_and_new_warnings_count_from_zero() {
        let mut app = app_with_drawing();
        app.warn("Bir uyarı.");
        let _ = app.update(Message::BottomTab(BottomTab::Messages));
        let _ = app.update(Message::BottomTab(BottomTab::History));
        let _ = app.update(Message::HistoryCleared);
        assert!(app.history.is_empty());
        // Cleared from another tab: the next warning is counted (the web's badge missed it).
        app.warn("Yeni bir uyarı.");
        assert_eq!(app.warning_count() - app.seen_warnings, 1);
    }

    #[test]
    fn the_panel_is_sized_by_its_edge_and_a_double_click_puts_it_back() {
        let mut app = app_with_drawing();
        let first = app.bottom_log();
        let _ = app.update(Message::BottomResized(320.0));
        assert_eq!(app.bottom_log(), 320.0);
        // Never shorter than five lines.
        let _ = app.update(Message::BottomResized(20.0));
        assert_eq!(app.bottom_log(), super::LEAST_LOG);
        let _ = app.update(Message::BottomReset);
        assert_eq!(app.bottom_log(), first);
    }
}

/// Pictures of the bottom panel for the owner: its history, the parcel's
/// coordinate list and the warnings, dark and light, at 1440×900 and
/// 1100×650; `.run/shots/alt-panel-*`:
///
/// ```text
/// cargo test -p kentos-desktop bottom::screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for (tab, name) in [
                (BottomTab::History, "gecmis"),
                (BottomTab::Coords, "koordinat"),
                (BottomTab::Messages, "uyarilar"),
                // Dragged taller, with a command running: the history still names the commands.
                (BottomTab::History, "gecmis-yuksek"),
            ] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let parcel = app
                    .document
                    .as_ref()
                    .and_then(|d| d.model.entities().find(|e| e.kind() == "polygon"))
                    .map(|e| kentos_domain::Slot(e.base().id))
                    .expect("the parcel");
                app.selection.set(vec![parcel]);
                app.warn("“yollar.shp” içinde alınmayanlar: Kısa halka: 1, üçten az köşesi var; alınmadı.");
                app.output("Çizgi: 12.500 m");
                let _ = app.update(Message::BottomTab(tab));
                if name == "gecmis-yuksek" {
                    let _ = app.update(Message::CommandRun("L".to_owned()));
                    let _ = app.update(Message::CommandRun("PAN".to_owned()));
                    let _ = app.update(Message::CommandRun("L".to_owned()));
                    let _ = app.update(Message::BottomResized(if height > 800.0 { 330.0 } else { 240.0 }));
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("alt-panel-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
