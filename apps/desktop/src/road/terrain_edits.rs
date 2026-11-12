//! Arazi yüzeyleri › Düzenlemeler (docs/adr/1560; plan/map-cad.md MAP-14): the chosen surface's edits, picked on the
//! drawing as Civil 3D's Edit Surface tools are. Nokta ekle puts a point where the click is (its height the window's
//! Kot, or the surface's there); Kot değiştir gives the corner nearest the click the window's Kot; Kenar çevir puts in
//! the other diagonal of the two triangles beside the edge nearest the click (their quadrilateral convex, the edge no
//! breakline or ring); Üçgen sil takes out the triangle clicked (its centroid kept, so a later rebuild finds it again).
//! Yüzey yapıştır (docs/adr/3920) pastes another saved surface, chosen by its name, as Civil 3D's Paste Surface: its
//! triangles come in as they are, this surface's records under it go.
//! The edits are the definition's: Kaydet writes them with it (`road.terrains.edit` set), the surface made again from
//! its sources then says the ones that no longer fit.

use crate::road;
use std::fmt;

use iced::widget::{button, column, row};
use iced::{Center, Element, Task};
use kentos_commands::{SurfaceEdit, TerrainReport, TerrainSurfaceDef};
use kentos_interaction::Level;
use kentos_interaction::pick::PickPoint;
use kentos_native_application::road_terrains;
use kentos_road::surface::Surface;
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::road_dialog::{col, num, sheet};
use crate::terrain_window::{Event, TITLE};

/// A tool of Düzenlemeler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditTool {
    AddPoint,
    SetHeight,
    FlipEdge,
    RemoveTriangle,
}

impl EditTool {
    pub const ALL: [EditTool; 4] = [Self::AddPoint, Self::SetHeight, Self::FlipEdge, Self::RemoveTriangle];

    fn icon(self) -> &'static str {
        match self {
            Self::AddPoint => "surfaceAddPoint",
            Self::SetHeight => "surfaceSetHeight",
            Self::FlipEdge => "surfaceFlipEdge",
            Self::RemoveTriangle => "surfaceRemoveTriangle",
        }
    }

    /// What the pick asks for (the command line's prompt).
    fn asks(self) -> &'static str {
        match self {
            Self::AddPoint => "Eklenecek noktanın yeri",
            Self::SetHeight => "Kotu değişecek köşe (en yakını alınır)",
            Self::FlipEdge => "Çevrilecek kenar (en yakını alınır)",
            Self::RemoveTriangle => "Silinecek üçgenin içi",
        }
    }
}

impl fmt::Display for EditTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AddPoint => "Nokta ekle",
            Self::SetHeight => "Kot değiştir",
            Self::FlipEdge => "Kenar çevir",
            Self::RemoveTriangle => "Üçgen sil",
        })
    }
}

/// An edit's words for the table: what, where (to the centimetre, to tell it; the definition keeps the place
/// exactly) or the surface pasted (its name), the height.
fn edit_row(e: &SurfaceEdit, name_of: impl Fn(&str) -> String) -> (String, String, String) {
    let m = |v: f64| format!("{v:.2}");
    match *e {
        SurfaceEdit::AddPoint { x, y, z } => ("Nokta ekle".into(), format!("{}, {}", m(x), m(y)), format!("{z:.3}")),
        SurfaceEdit::SetHeight { x, y, z } => ("Kot değiştir".into(), format!("{}, {}", m(x), m(y)), format!("{z:.3}")),
        SurfaceEdit::AddEdge { ax, ay, bx, by } => {
            ("Kenar".into(), format!("{}, {} – {}, {}", m(ax), m(ay), m(bx), m(by)), String::new())
        }
        SurfaceEdit::RemoveTriangle { x, y } => ("Üçgen sil".into(), format!("{}, {}", m(x), m(y)), String::new()),
        SurfaceEdit::Paste { ref terrain } => ("Yapıştır".into(), format!("“{}” yüzeyi", name_of(terrain)), String::new()),
    }
}

/// Twice the signed area of (a, b, c): left of a→b is positive.
fn orient(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// The distance from (x, y) to the segment a–b.
fn to_segment(p: [f64; 2], a: [f64; 3], b: [f64; 3]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = dx * dx + dy * dy;
    let t = if len > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len).clamp(0.0, 1.0) } else { 0.0 };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

/// The edit a pick at `p` makes on `s` with the window's height `z`, or why none is made.
pub fn edit_at(tool: EditTool, s: &Surface, p: [f64; 2], z: Option<f64>) -> Result<SurfaceEdit, String> {
    let on = s.height(p[0], p[1]);
    match tool {
        EditTool::AddPoint => {
            if s.vertices.iter().any(|v| v[0] == p[0] && v[1] == p[1]) {
                return Err("O yerde yüzeyin köşesi var; kotunu Kot değiştir ile verin.".into());
            }
            let z = z.or(on.z).ok_or("Yüzeyin dışındaki noktanın kotunu Kot (m) alanına yazın.")?;
            Ok(SurfaceEdit::AddPoint { x: p[0], y: p[1], z })
        }
        EditTool::SetHeight => {
            let z = z.ok_or("Yeni kotu Kot (m) alanına yazın.")?;
            let v = s
                .vertices
                .iter()
                .min_by(|a, b| (a[0] - p[0]).hypot(a[1] - p[1]).total_cmp(&(b[0] - p[0]).hypot(b[1] - p[1])))
                .ok_or("Yüzeyin köşesi yok.")?;
            Ok(SurfaceEdit::SetHeight { x: v[0], y: v[1], z })
        }
        EditTool::FlipEdge => {
            let t = on.triangle.ok_or("Tıklanan yer yüzeyin bir üçgeni değil.")? as usize;
            let tri = s.triangles[t];
            let v = |i: u32| s.vertices[i as usize];
            let (a, b) = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])]
                .into_iter()
                .min_by(|e, f| to_segment(p, v(e.0), v(e.1)).total_cmp(&to_segment(p, v(f.0), v(f.1))))
                .ok_or("Üçgenin kenarı yok.")?;
            let pair = [a.min(b), a.max(b)];
            if s.constrained.binary_search(&pair).is_ok() {
                return Err("Bu kenar bir kırık hat, sınır ya da eklenmiş bir kenar; çevrilmez.".into());
            }
            let other = s
                .triangles
                .iter()
                .enumerate()
                .find(|(k, o)| *k != t && o.contains(&a) && o.contains(&b))
                .map(|(_, o)| *o)
                .ok_or("Kenarın öbür yanında üçgen yok (yüzeyin dış kenarı); çevrilmez.")?;
            let c = tri.into_iter().find(|i| *i != a && *i != b).ok_or("Üçgen bozuk.")?;
            let d = other.into_iter().find(|i| *i != a && *i != b).ok_or("Üçgen bozuk.")?;
            // The quadrilateral is convex when a and b lie on either side of c–d.
            let (sa, sb) = (orient(v(c), v(d), v(a)), orient(v(c), v(d), v(b)));
            if sa * sb >= 0.0 {
                return Err("İki üçgenin dörtgeni dışbükey değil; kenar çevrilmez.".into());
            }
            let (c, d) = (v(c), v(d));
            Ok(SurfaceEdit::AddEdge { ax: c[0], ay: c[1], bx: d[0], by: d[1] })
        }
        EditTool::RemoveTriangle => {
            let t = on.triangle.ok_or("Tıklanan yer yüzeyin bir üçgeni değil.")? as usize;
            let [a, b, c] = s.triangles[t].map(|i| s.vertices[i as usize]);
            Ok(SurfaceEdit::RemoveTriangle { x: (a[0] + b[0] + c[0]) / 3.0, y: (a[1] + b[1] + c[1]) / 3.0 })
        }
    }
}

/// A number the user typed, a comma or a point; empty: none.
fn typed(text: &str) -> Result<Option<f64>, ()> {
    let text = text.trim().replace(',', ".");
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or(())
}

impl App {
    /// A tool of Düzenlemeler: the window steps aside for the pick on the drawing.
    pub(crate) fn start_terrain_edit(&mut self, tool: EditTool) -> Task<Message> {
        let Some(w) = self.road.terrain_window.as_mut() else {
            return Task::none();
        };
        if w.chosen_def().is_none() {
            return Task::none();
        }
        w.picking = Some(tool);
        self.dialog = None;
        self.say(Level::Command, format!("{TITLE} › {tool}"));
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(PickPoint::new(TITLE, tool.asks())));
        self.with_tool(|s, cx| s.activate(cx));
        Task::none()
    }

    /// The point Düzenlemeler asked for (input.rs); false when it asked none. The edit goes to the chosen surface's
    /// definition and the window opens again; one that cannot be made is said.
    pub(crate) fn terrain_picked(&mut self, p: Option<kentos_interaction::Vec2>) -> bool {
        let Some(tool) = self
            .road
            .terrain_window
            .as_mut()
            .and_then(|w| w.picking.take())
        else {
            return false;
        };
        self.dialog = Some(Dialog::Road(road::RoadDialogKind::Terrains));
        let Some(p) = p else {
            return true;
        };
        let (Some(doc), Some(w)) = (self.document.as_ref(), self.road.terrain_window.as_ref())
        else {
            return true;
        };
        let Some(def) = w.chosen_def().cloned() else {
            return true;
        };
        let z = match typed(&w.edit_z) {
            Ok(z) => z,
            Err(()) => {
                self.warn("Kot (m) bir sayı olmalı.".to_owned());
                return true;
            }
        };
        let (_, surface) = road_terrains::made_of(&doc.model, &def);
        let Some(surface) = surface else {
            self.warn(format!("“{}” yüzeyi şimdi kurulamıyor; önce kaynaklarını düzeltin.", def.name));
            return true;
        };
        match edit_at(tool, &surface, [p.x, p.y], z) {
            Ok(edit) => {
                if let Some(w) = self.road.terrain_window.as_mut() {
                    w.push_edit(edit);
                }
                self.refresh_edit_report();
                self.say(Level::Info, format!("{tool}: düzenleme eklendi; Kaydet ile yazılır."));
            }
            Err(why) => self.warn(why),
        }
        true
    }

    /// The chosen definition made now with its edits, saved or not: what each edit came to.
    pub(crate) fn refresh_edit_report(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let Some(w) = self.road.terrain_window.as_ref() else {
            return;
        };
        let report = w
            .chosen_def()
            .map(|def| road_terrains::made_of(&doc.model, def).0);
        if let Some(w) = self.road.terrain_window.as_mut() {
            w.edit_report = report;
        }
    }

    /// Düzenlemeler: the tools, Kot, the edits in a table with what each came to, and Sonuncuyu kaldır.
    pub(crate) fn terrain_edits_view<'a>(
        &'a self,
        def: &'a TerrainSurfaceDef,
        report: Option<&'a TerrainReport>,
        z: &'a str,
    ) -> Element<'a, Message> {
        let tool = |t: EditTool| -> Element<'a, Message> {
            button(
                row![
                    icon(from_web(Some(t.icon()))).size(16.0),
                    label::body(t.to_string())
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press(Message::Road(road::Event::Terrains(Event::EditTool(t))))
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::secondary)
            .into()
        };
        let tools = row(EditTool::ALL.map(tool)).spacing(6).wrap();
        let height = kentos_ui::widget::text_field("yüzeyin kotu", z)
            .on_input(|t| Message::Road(road::Event::Terrains(Event::EditZ(t))));
        // Yüzey yapıştır (docs/adr/3920): a saved surface other than this one, by its name.
        let kept = self
            .road
            .terrain_window
            .as_ref()
            .map_or(&[][..], |w| w.kept());
        let others: Vec<&TerrainSurfaceDef> = kept.iter().filter(|k| k.id != def.id).collect();
        let ids: Vec<String> = others.iter().map(|k| k.id.clone()).collect();
        let from = self
            .road
            .terrain_window
            .as_ref()
            .and_then(|w| w.paste_from.as_ref());
        let chosen = from.and_then(|id| ids.iter().position(|i| i == id));
        let select = kentos_ui::widget::select::Select::new(
            others.iter().map(|k| kentos_ui::widget::select::Choice::new(k.name.clone())),
            chosen,
            move |i| Message::Road(road::Event::Terrains(Event::PasteFrom(ids[i].clone()))),
        )
        .placeholder(if others.is_empty() {
            "kayıtlı başka yüzey yok"
        } else {
            "yüzey seçin"
        });
        let paste = button(
            row![
                icon(from_web(Some("surfacePaste"))).size(16.0),
                label::body(PASTE)
            ]
            .spacing(6)
            .align_y(Center),
        )
        .on_press_maybe(chosen.map(|_| Message::Road(road::Event::Terrains(Event::Paste))))
        .padding(kentos_ui::theme::metrics::action())
        .style(style::button::secondary);
        let name_of = |id: &str| {
            kept.iter()
                .find(|k| k.id == id)
                .map_or_else(|| id.to_owned(), |k| k.name.clone())
        };
        let why_of = |k: usize| {
            report.and_then(|r| {
                r.left
                    .iter()
                    .find(|l| l.role == "edits" && l.id == format!("edits[{k}]"))
                    .map(|l| road_terrains::why_words(&l.why).to_owned())
            })
        };
        let rows: Vec<Vec<String>> = def
            .edits
            .iter()
            .enumerate()
            .map(|(k, e)| {
                let (what, at, h) = edit_row(e, name_of);
                let state = match (report, why_of(k)) {
                    (None, _) => "kaydedilince",
                    (Some(_), Some(_)) => "yapılamadı",
                    (Some(_), None) => "yapıldı",
                }
                .to_owned();
                vec![(k + 1).to_string(), what, at, h, state]
            })
            .collect();
        let table = sheet(
            // As wide as the pane with each cell's pads (no sideways bar): the widths are the words'.
            vec![num("Sıra", 36.0), col("İşlem", 86.0), col("Yer (Y, X)", 150.0), num("Kot (m)", 62.0), col("Durum", 80.0)],
            rows,
            "Düzenleme yok. Bir araç seçip çizimde tıklayın.",
            160.0,
        );
        let skipped: Vec<String> =
            (0..def.edits.len()).filter_map(|k| why_of(k).map(|why| format!("{}. düzenleme yapılamadı: {why}.", k + 1))).collect();
        let mut col = column![
            row![iced::widget::container(words::field("Kot (m)", height, None)).width(110), tools]
                .spacing(12)
                .align_y(iced::alignment::Vertical::Bottom),
            row![iced::widget::container(words::field(PASTE_FROM, select, None)).width(240), paste]
                .spacing(12)
                .align_y(iced::alignment::Vertical::Bottom),
            table,
            row![
                words::ghost(
                    UNDO_EDIT,
                    (!def.edits.is_empty())
                        .then(|| Message::Road(road::Event::Terrains(Event::EditUndo)))
                ),
                words::ghost(
                    CLEAR_EDITS,
                    (!def.edits.is_empty())
                        .then(|| Message::Road(road::Event::Terrains(Event::EditClear)))
                ),
            ]
            .spacing(8),
        ]
        .spacing(8);
        for line in skipped {
            col = col.push(words::text_line(Kind::Warn, line));
        }
        col.into()
    }
}

/// Sonuncuyu kaldır and Hepsini kaldır: the edits taken back from the definition (Kaydet writes it).
pub const UNDO_EDIT: &str = "Sonuncuyu kaldır";
pub const CLEAR_EDITS: &str = "Düzenlemeleri temizle";
/// Yüzey yapıştır (docs/adr/3920): the saved surface's box and the button that pastes it.
pub const PASTE_FROM: &str = "Yapıştırılacak yüzey";
pub const PASTE: &str = "Yüzey yapıştır";

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_road::surface::{SpotPoint, SurfaceInput, build};

    fn square() -> Surface {
        let p = |x: f64, y: f64, z: f64| SpotPoint { x, y, z, source: None };
        build(&SurfaceInput {
            points: vec![p(0.0, 0.0, 10.0), p(10.0, 0.0, 11.0), p(10.0, 10.0, 12.0), p(0.0, 10.0, 11.0), p(4.0, 6.0, 13.0)],
            ..SurfaceInput::default()
        })
        .expect("builds")
    }

    #[test]
    fn each_tool_makes_its_edit_or_says_why_not() {
        let s = square();
        assert_eq!(
            edit_at(EditTool::AddPoint, &s, [7.0, 3.0], Some(10.5)),
            Ok(SurfaceEdit::AddPoint { x: 7.0, y: 3.0, z: 10.5 })
        );
        // Without a height, inside: the surface's there.
        assert!(matches!(edit_at(EditTool::AddPoint, &s, [7.0, 3.0], None), Ok(SurfaceEdit::AddPoint { .. })));
        assert!(edit_at(EditTool::AddPoint, &s, [20.0, 20.0], None).is_err(), "outside, no height");
        assert!(edit_at(EditTool::AddPoint, &s, [4.0, 6.0], Some(1.0)).is_err(), "a corner is there");
        assert_eq!(
            edit_at(EditTool::SetHeight, &s, [4.2, 5.9], Some(14.0)),
            Ok(SurfaceEdit::SetHeight { x: 4.0, y: 6.0, z: 14.0 })
        );
        assert!(edit_at(EditTool::SetHeight, &s, [4.2, 5.9], None).is_err(), "no height typed");
        let removed = edit_at(EditTool::RemoveTriangle, &s, [5.0, 1.0], None).expect("a triangle");
        let SurfaceEdit::RemoveTriangle { x, y } = removed else { panic!("{removed:?}") };
        assert_eq!(s.height(x, y).triangle, s.height(5.0, 1.0).triangle, "its centroid is in it");
        assert!(edit_at(EditTool::RemoveTriangle, &s, [-1.0, -1.0], None).is_err());
    }

    #[test]
    fn kenar_cevir_puts_in_the_other_diagonal() {
        let s = square();
        // Near an inner edge: the flip's diagonal is between the two triangles' other corners.
        let mut flipped = None;
        // Beside the inner edges from (4, 6) to the corners, a little off their middles.
        for p in [[7.1, 3.0], [2.1, 3.0], [7.1, 8.0], [2.1, 8.0]] {
            if let Ok(e) = edit_at(EditTool::FlipEdge, &s, p, None) {
                flipped = Some(e);
                break;
            }
        }
        let Some(SurfaceEdit::AddEdge { ax, ay, bx, by }) = flipped else { panic!("{flipped:?}") };
        // Made again with it, the diagonal is an edge of the surface.
        let mut input = SurfaceInput { ..SurfaceInput::default() };
        let p = |x: f64, y: f64, z: f64| SpotPoint { x, y, z, source: None };
        input.points = vec![p(0.0, 0.0, 10.0), p(10.0, 0.0, 11.0), p(10.0, 10.0, 12.0), p(0.0, 10.0, 11.0), p(4.0, 6.0, 13.0)];
        input.edges = vec![kentos_road::surface::ForcedEdge { from: [ax, ay], to: [bx, by], edit: 0 }];
        let t = build(&input).expect("builds");
        assert!(t.report.skipped.is_empty(), "{:?}", t.report.skipped);
        let at = |x: f64, y: f64| t.vertices.iter().position(|v| v[0] == x && v[1] == y).expect("a corner") as u32;
        let (a, b) = (at(ax, ay), at(bx, by));
        assert!(t.triangles.iter().any(|tri| tri.contains(&a) && tri.contains(&b)), "the diagonal is an edge");
        // The outer edge cannot be flipped.
        assert!(edit_at(EditTool::FlipEdge, &s, [5.0, 0.01], None).is_err());
    }
}
