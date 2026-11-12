//! Kullanım alanı (plan/map-cad.md MAP-02, docs/adr/5300): a small map under Proje ayarları › Koordinat sistemi's
//! picker. It draws the chosen system's EPSG area of use (its polygon where carried, else its box) over Türkiye's
//! extent and its provinces' capitals; with a second system on another datum, the datum transformation's area too
//! (its extent's polygon, dashed). Under it, in words, which extent each is and whether its polygon or its box is read.
//! A point beyond these is noted by the transformations (`outside_*`); the map only shows where they are.

use iced::widget::canvas::{self, LineDash, Path, Stroke};
use iced::widget::{column, container};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Theme, mouse};
use kentos_project::crs::{self, AreaOfUse, Region};
use kentos_ui::label;
use kentos_ui::theme::Tokens;

use crate::app::Message;

/// What the map draws: the system's area and the transformation's region, with its way's name.
pub struct AreaMap {
    pub area: Option<&'static AreaOfUse>,
    pub operation: Option<(String, Region)>,
}

impl AreaMap {
    /// The project's system `srid` and, when on another datum, the way to the second system `second`.
    pub fn of(srid: u32, second: Option<u32>) -> AreaMap {
        let operation = second.and_then(|t| {
            let a = crs::system(srid)?.transform_system()?;
            let b = crs::system(t)?.transform_system()?;
            let op = kentos_geometry_core::crs::operation::operation(&a, &b, &[], false).ok()?;
            let region = crs::operation_region(&op)?;
            Some((op.id.clone(), region))
        });
        AreaMap { area: crs::area(srid), operation }
    }
}

/// A region's rings, or its box as one ring.
fn rings(r: &Region) -> Vec<Vec<[f64; 2]>> {
    let boxed = |[w, s, e, n]: [f64; 4]| vec![vec![[w, s], [e, s], [e, n], [w, n], [w, s]]];
    match r {
        Region::Area(a) if !a.polygon.is_empty() => a.polygon.clone(),
        Region::Area(a) => boxed(a.bbox()),
        Region::Box(b) => boxed(*b),
    }
}

impl canvas::Program<Message> for AreaMap {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        // Türkiye's extent (onshore and offshore) as the ground every area is drawn on.
        let ground = crs::extent_area(kentos_geometry_core::crs::operation::TURKIYE_EXTENT).map(|a| rings(&Region::Area(a)));
        let area = self.area.map(|a| rings(&Region::Area(a)));
        let op = self.operation.as_ref().map(|(_, r)| rings(r));
        let all: Vec<&[f64; 2]> =
            [&ground, &area, &op].into_iter().flatten().flat_map(|r| r.iter().flatten()).collect();
        if all.is_empty() {
            return vec![frame.into_geometry()];
        }
        let (mut w, mut s, mut e, mut n) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &all {
            w = w.min(p[0]);
            e = e.max(p[0]);
            s = s.min(p[1]);
            n = n.max(p[1]);
        }
        // Equirectangular at the middle latitude, fitted with a margin.
        let k = ((s + n) / 2.0).to_radians().cos().max(0.2);
        let pad = 10.0_f32;
        let (bw, bh) = (bounds.width - 2.0 * pad, bounds.height - 2.0 * pad);
        let scale = (f64::from(bw) / ((e - w) * k).max(1e-9)).min(f64::from(bh) / (n - s).max(1e-9));
        let (ox, oy) = (
            pad + (bw - ((e - w) * k * scale) as f32) / 2.0,
            pad + (bh - ((n - s) * scale) as f32) / 2.0,
        );
        let at = |p: &[f64; 2]| Point::new(ox + ((p[0] - w) * k * scale) as f32, oy + ((n - p[1]) * scale) as f32);
        let path = |rs: &[Vec<[f64; 2]>]| {
            Path::new(|b| {
                for r in rs {
                    if let Some(first) = r.first() {
                        b.move_to(at(first));
                        for p in &r[1..] {
                            b.line_to(at(p));
                        }
                        b.close();
                    }
                }
            })
        };
        let solid = |w: f32, c: Color| Stroke::default().with_width(w).with_color(c);
        if let Some(g) = &ground {
            let p = path(g);
            frame.fill(&p, Color { a: 0.06, ..tk.text });
            frame.stroke(&p, solid(1.0, tk.border));
        }
        if let Some(a) = &area {
            let p = path(a);
            // Even-odd: a hole of the extent stays open.
            let fill = canvas::Fill { style: canvas::Style::Solid(Color { a: 0.22, ..tk.accent }), rule: canvas::fill::Rule::EvenOdd };
            frame.fill(&p, fill);
            frame.stroke(&p, solid(1.5, tk.accent));
        }
        if let Some(o) = &op {
            frame.stroke(
                &path(o),
                Stroke { line_dash: LineDash { segments: &[5.0, 3.0], offset: 0 }, ..solid(1.5, tk.warning) },
            );
        }
        for c in kentos_project::provinces::provinces() {
            frame.fill(&Path::circle(at(&[c.lon, c.lat]), 1.5), tk.muted);
        }
        vec![frame.into_geometry()]
    }
}

/// An area in words: its EPSG extent and whether its polygon or its box is read.
fn area_words(a: &AreaOfUse) -> String {
    match a.extent {
        Some(code) if !a.polygon.is_empty() => format!("EPSG alanı {code}: {} (çokgen)", a.name),
        _ => format!("{} (kutu, yaklaşık)", a.name),
    }
}

/// The map and its two lines.
pub fn view<'a>(map: AreaMap) -> Element<'a, Message> {
    let mut words = Vec::new();
    if let Some(a) = map.area {
        words.push(format!("Sistemin kullanım alanı — {}", area_words(a)));
    }
    if let Some((id, r)) = &map.operation {
        let w = match r {
            Region::Area(a) => area_words(a),
            Region::Box(_) => "kutu, yaklaşık".to_owned(),
        };
        words.push(format!("Datum dönüşümünün ({id}) kullanım alanı, kesikli — {w}"));
    }
    let mut c = column![container(canvas::Canvas::new(map).width(Fill).height(180)).width(Fill)].spacing(4);
    for w in words {
        c = c.push(label::caption(w).width(Fill));
    }
    c.into()
}
