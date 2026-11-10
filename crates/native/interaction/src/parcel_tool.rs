//! Parsel (docs/adr/8032; ADV-22, M32): a closed line or a polygon of the drawing made the site plan's parcel, each of
//! its edges given its kind (Yan bahçe, Ön bahçe, Arka bahçe, Kamusal alan), and the setback line (yapı yaklaşma
//! sınırı) the document keeps drawn as a dashed closed line.
//!
//! The parcel is clicked on its edge; then a click on an edge turns its kind over (Yan → Ön → Arka → Kamusal → Yan),
//! each edge saying its garden distance (PAİY 23: 5 m on a road or a public area, 3 m at the side and the back with
//! half a metre for each storey past four) and the footprint's distance to it, the setback line shown as it will be;
//! Enter writes them. Started on a project with a parcel, it takes it up with its kinds.
//!
//! Ölçüler (Ö, on by default; docs/adr/8032 §2) draws the site plan's marks with them, in the same undo step: an aligned
//! dimension from the footprint's nearest point to each edge, a level mark ±0.00 in the footprint's middle and, with
//! ±0.00's elevation, “±0.00 = +1285.40” under it. Each carries the attribute `Vaziyet`; run again, the tool takes the
//! old marks away before it draws the new ones.

use kentos_commands::{EntityGeometry, LineType, SiteEdge, SiteEdgeKind, SitePlan};
use kentos_native_application::{ExecutionContext, create};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Label, Pointer, Preview, Stroke, Tag, Tone, Tool};

pub const ID: &str = "siteParcel";
pub const LABEL: &str = "Parsel";
/// The setback line's colour: the plan's own red for an approach line.
pub const ENVELOPE_COLOR: &str = "#d32f2f";
/// The attribute the site plan's marks carry (docs/adr/8032 §2): what the tool takes away before drawing them again.
pub const MARK_ATTR: &str = "Vaziyet";

/// The next kind a click gives an edge.
fn next(kind: SiteEdgeKind) -> SiteEdgeKind {
    match kind {
        SiteEdgeKind::Side => SiteEdgeKind::Front,
        SiteEdgeKind::Front => SiteEdgeKind::Back,
        SiteEdgeKind::Back => SiteEdgeKind::Public,
        SiteEdgeKind::Public => SiteEdgeKind::Side,
    }
}

/// The nearest a point comes to segment `ab`.
fn near(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.x - (a.x + t * dx)).hypot(p.y - (a.y + t * dy))
}

fn m2(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

pub struct ParcelTool {
    /// Ölçüler: the distance dimensions and ±0.00 drawn too.
    marks: bool,
    /// The parcel: its persistent id and its corners as the boundary runs.
    parcel: Option<(String, Vec<Vec2>)>,
    /// Each edge's kind and the plan's own setback.
    edges: Vec<SiteEdge>,
    /// The closed line or polygon under the cursor before one is taken.
    hovered: Option<(String, Vec<Vec2>)>,
    /// The edge under the cursor once the parcel is taken.
    hot: Option<usize>,
    at: Option<Vec2>,
    /// The site plan as it would be written, read against the drawing.
    report: Option<kentos_domain::site::SiteReport>,
}

impl Default for ParcelTool {
    fn default() -> Self {
        Self {
            marks: true,
            parcel: None,
            edges: Vec::new(),
            hovered: None,
            hot: None,
            at: None,
            report: None,
        }
    }
}

impl ParcelTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The site plan's marks (docs/adr/8032 §2): the old ones taken away, a distance dimension for each edge the
    /// footprint stands apart from, ±0.00 in the footprint's middle and its elevation under it; how many drawn.
    fn draw_marks(report: &kentos_domain::site::SiteReport, cx: &mut Context<'_>) -> Option<usize> {
        let old: Vec<kentos_domain::Slot> = cx
            .doc
            .entities()
            .filter(|e| e.base().attrs.contains_key(MARK_ATTR))
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        if !old.is_empty() {
            cx.doc.remove(&old);
        }
        let layer = cx.doc.layers().active().to_owned();
        let look = |what: &str| kentos_commands::NewLook {
            color: None,
            line_weight: None,
            attrs: Some(std::iter::once((MARK_ATTR.to_owned(), what.to_owned())).collect()),
            label: None,
            symbol: None,
        };
        let mut drawn = 0;
        for e in &report.edges {
            let Some((p, q)) = e.near.filter(|_| e.drawn.is_some_and(|d| d > 1e-6)) else {
                continue;
            };
            let input = kentos_commands::DimensionsCreate {
                measure: kentos_commands::DimensionMeasure::Aligned {
                    a: kentos_commands::MeasuredPoint::At {
                        at: kentos_commands::Vec2 { x: p.x, y: p.y },
                    },
                    b: kentos_commands::MeasuredPoint::At {
                        at: kentos_commands::Vec2 { x: q.x, y: q.y },
                    },
                },
                at: kentos_commands::Vec2 {
                    x: (p.x + q.x) / 2.0,
                    y: (p.y + q.y) / 2.0,
                },
                offset: None,
                layer_id: layer.clone(),
                dimension_style: None,
                height: None,
                mask: Some(true),
                look: look("Bahçe mesafesi"),
                expected_revision: None,
            };
            let result = kentos_native_application::dimension_create::dimensions_create::execute(
                &mut ExecutionContext::new(cx.doc),
                input,
            );
            crate::points::written(result, cx)?;
            drawn += 1;
        }
        // ±0.00 in the footprint's middle, its elevation under it.
        let (_, shape) = cx.doc.footprint();
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in shape.iter().flat_map(|a| a.outer.pts.iter()) {
            b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
        }
        if b[0].is_finite() {
            // In the footprint's upper part: its middle is where a room's own tag stands.
            let mid = kentos_commands::Vec2 {
                x: (b[0] + b[2]) / 2.0,
                y: b[1] + 0.8 * (b[3] - b[1]),
            };
            let input = kentos_commands::DimensionsCreate {
                measure: kentos_commands::DimensionMeasure::Elevation {
                    a: kentos_commands::MeasuredPoint::At { at: mid },
                    z: Some(0.0),
                    level: None,
                },
                at: mid,
                offset: None,
                layer_id: layer.clone(),
                dimension_style: None,
                height: None,
                mask: Some(true),
                look: look("±0,00"),
                expected_revision: None,
            };
            let result = kentos_native_application::dimension_create::dimensions_create::execute(
                &mut ExecutionContext::new(cx.doc),
                input,
            );
            crate::points::written(result, cx)?;
            drawn += 1;
            if let Some(z) = report.datum {
                let height = cx.annotation_height(kentos_commands::AnnotationKind::Text);
                let input = kentos_commands::EntitiesCreate {
                    layer_id: layer,
                    objects: vec![kentos_commands::NewObject {
                        geometry: EntityGeometry::Text {
                            layout: None,
                            annotative: None,
                            p: kentos_commands::Vec2 {
                                x: mid.x,
                                y: mid.y - 3.0 * height,
                            },
                            text: format!("±0.00 = {}{:.2}", if z >= 0.0 { "+" } else { "" }, z),
                            height,
                            rotation: 0.0,
                            align: Some(kentos_commands::TextAlign::MiddleCenter),
                            width_factor: None,
                            mask: true,
                            box_width: None,
                            line_spacing: None,
                            runs: Vec::new(),
                            face: crate::styles::text_face(cx),
                            path: None,
                        },
                        color: None,
                        line_weight: None,
                        line_type: None,
                        attrs: Some(
                            std::iter::once((MARK_ATTR.to_owned(), "±0,00".to_owned())).collect(),
                        ),
                        label: None,
                        label_of: None,
                        label_scale: None,
                        symbol: None,
                        order: None,
                        transparency: None,
                    }],
                    operation: None,
                    expected_revision: None,
                };
                let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
                crate::points::written(result, cx)?;
                drawn += 1;
            }
        }
        Some(drawn)
    }

    /// The closed line or polygon under `at`: its persistent id and corners.
    fn boundary_at(at: Vec2, cx: &Context<'_>) -> Option<(String, Vec<Vec2>)> {
        cx.spatial
            .hits(at, cx.pick_tolerance())
            .into_iter()
            .find_map(|s| {
                let pts = match cx.doc.get(s)? {
                    kentos_commands::Entity::Polyline(p) if p.closed && p.pts.len() >= 3 => &p.pts,
                    kentos_commands::Entity::Polygon(p) if p.pts.len() >= 3 => &p.pts,
                    _ => return None,
                };
                let uid = cx.doc.uid(s)?.to_string();
                Some((uid, pts.iter().map(|p| Vec2::new(p.x, p.y)).collect()))
            })
    }

    /// The site plan as it would be written: the project's own with this parcel and these edges.
    fn plan(&self, cx: &Context<'_>) -> Option<SitePlan> {
        let (uid, _) = self.parcel.as_ref()?;
        let mut site = cx
            .doc
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.site.clone())
            .unwrap_or_default();
        if site.parcel.as_deref() != Some(uid.as_str()) {
            site.edges.clear();
        }
        site.parcel = Some(uid.clone());
        site.edges.clone_from(&self.edges);
        // Kinds past the boundary's edges are not kept; trailing side gardens are the default.
        while site.edges.last().is_some_and(|e| *e == SiteEdge::default()) {
            site.edges.pop();
        }
        Some(site)
    }

    fn read(&mut self, cx: &Context<'_>) {
        self.report = self.plan(cx).and_then(|s| cx.doc.site_report_for(&s));
    }

    fn take(&mut self, parcel: (String, Vec<Vec2>), cx: &Context<'_>) {
        let site = cx.doc.settings().arch.as_ref().and_then(|a| a.site.clone());
        let n = parcel.1.len();
        self.edges = match site.filter(|s| s.parcel.as_deref() == Some(parcel.0.as_str())) {
            Some(s) => (0..n).map(|i| s.edge(i)).collect(),
            None => vec![SiteEdge::default(); n],
        };
        self.parcel = Some(parcel);
        self.read(cx);
    }

    fn lines(&self) -> Vec<String> {
        let Some(r) = &self.report else {
            return vec![
                "Parselin kenarına tıklayın: kapalı bir çizgi ya da alan.".to_owned(),
                "Parseli önce çizin (Çoklu çizgi, kapalı) ya da koordinatlarından yazın."
                    .to_owned(),
            ];
        };
        let mut out = vec![
            format!("Parsel {} m² · {} kenar", m2(r.area), r.edges.len()),
            format!("Kat adedi {} (PAİY 23/1-d)", r.floors),
        ];
        for kind in SiteEdgeKind::ALL {
            let mine: Vec<&kentos_domain::site::SiteEdgeReport> =
                r.edges.iter().filter(|e| e.kind == kind).collect();
            if mine.is_empty() {
                continue;
            }
            let need = mine.iter().map(|e| e.required).fold(0.0, f64::max);
            let drawn = mine
                .iter()
                .filter_map(|e| e.drawn)
                .fold(f64::INFINITY, f64::min);
            out.push(if drawn.is_finite() {
                format!(
                    "{} {} m · binada {} m{}",
                    kind.label(),
                    m2(need),
                    m2(drawn),
                    if drawn + 1e-6 < need { " · dar" } else { "" }
                )
            } else {
                format!("{} {} m", kind.label(), m2(need))
            });
        }
        out.push(format!("Ağaç {} (her 30 m² bahçeye bir, 23/6)", r.trees));
        out.push("Kenara tıklayın: türü değişir · Enter: kaydet".to_owned());
        out
    }

    /// The site plan written to the settings and the setback line drawn (or kept and moved); the words said.
    fn write(&mut self, cx: &mut Context<'_>) -> bool {
        let Some(mut site) = self.plan(cx) else {
            cx.say(Level::Info, "Önce parselin kenarına tıklayın.");
            return false;
        };
        let Some(report) = cx.doc.site_report_for(&site) else {
            cx.say(Level::Warn, "Parsel kapalı bir çizgi ya da alan değil.");
            return false;
        };
        let group = cx.doc.begin_group(LABEL);
        if !self.write_in(&mut site, &report, cx) {
            cx.doc.cancel_group(group);
            return false;
        }
        cx.doc.end_group(group);
        true
    }

    /// `write`'s body inside its group: the setback line, the marks, the settings and the words.
    fn write_in(
        &mut self,
        site: &mut SitePlan,
        report: &kentos_domain::site::SiteReport,
        cx: &mut Context<'_>,
    ) -> bool {
        // The setback line: the one the site plan keeps where it still stands, else a new dashed closed line.
        let standing = site
            .envelope
            .clone()
            .as_deref()
            .and_then(|u| kentos_domain::Uuid::parse_str(u).ok())
            .and_then(|u| cx.doc.slot_of(u))
            .is_some();
        if !standing {
            let Some(pts) = cx.doc.site_envelope_ring(&site) else {
                cx.say(Level::Warn, "Bahçe mesafeleri parselde yapılacak yer bırakmıyor: yaklaşma sınırı çizilmedi.");
                return false;
            };
            let input = kentos_commands::EntitiesCreate {
                layer_id: cx.doc.layers().active().to_owned(),
                objects: vec![kentos_commands::NewObject {
                    geometry: EntityGeometry::Polyline {
                        pts,
                        bulges: None,
                        zs: None,
                        ms: None,
                        parts: None,
                        closed: true,
                        widths: None,
                    },
                    color: Some(ENVELOPE_COLOR.to_owned()),
                    line_weight: None,
                    line_type: Some(LineType::Dashed),
                    attrs: None,
                    label: Some("Yapı yaklaşma sınırı".to_owned()),
                    label_of: None,
                    label_scale: None,
                    symbol: None,
                    order: None,
                    transparency: None,
                }],
                operation: None,
                expected_revision: None,
            };
            let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
            match crate::points::written(result, cx) {
                Some(out) => site.envelope = out.created.first().cloned(),
                None => return false,
            }
        }
        let marks = if self.marks {
            Self::draw_marks(report, cx)
        } else {
            Some(0)
        };
        if marks.is_none() {
            return false;
        }
        let mut settings = cx.doc.settings().clone();
        let arch = settings.arch.get_or_insert_with(Default::default);
        // A project without a zoning takes the parcel's drawn area as its own (docs/adr/8030).
        if arch.zoning.is_none() {
            arch.zoning = Some(kentos_commands::Zoning {
                parcel_area: report.area,
                far: None,
                coverage: None,
            });
        }
        arch.site = Some(site.clone());
        cx.doc.set_settings(settings);
        let short: Vec<String> = report
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.short())
            .map(|(i, e)| {
                format!(
                    "{}. kenar ({}) {} m < {} m",
                    i + 1,
                    e.kind.label(),
                    m2(e.drawn.unwrap_or(0.0)),
                    m2(e.required)
                )
            })
            .collect();
        cx.say(
            if short.is_empty() {
                Level::Success
            } else {
                Level::Warn
            },
            format!(
                "Parsel {} m², kat adedi {}, ağaç {}; yapı yaklaşma sınırı çizildi{}.{}",
                m2(report.area),
                report.floors,
                report.trees,
                if self.marks {
                    ", bahçe mesafeleri ve ±0,00 ölçülendi"
                } else {
                    ""
                },
                if short.is_empty() {
                    String::new()
                } else {
                    format!(" Bahçe mesafesi dar: {}.", short.join("; "))
                }
            ),
        );
        true
    }
}

impl Tool for ParcelTool {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let p = if self.parcel.is_none() {
            Prompt::new(LABEL, "parselin kenarına tıklayın")
        } else {
            Prompt::new(LABEL, "kenara tıklayın: türü değişir; Enter ile kaydedin")
        };
        p.option_with("Ölçüler", "Ö", if self.marks { "Açık" } else { "Kapalı" })
    }

    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.parcel.take().is_some() {
            self.edges.clear();
            self.report = None;
            let _ = cx;
            return true;
        }
        false
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        // A project with a parcel takes it up with its kinds.
        if let Some(site) = cx.doc.settings().arch.as_ref().and_then(|a| a.site.clone())
            && let (Some(uid), Some(corners)) = (site.parcel.clone(), cx.doc.site_parcel(&site))
        {
            self.take(
                (uid, corners.iter().map(|p| Vec2::new(p.x, p.y)).collect()),
                cx,
            );
        }
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.at = Some(p.raw);
        match &self.parcel {
            None => self.hovered = Self::boundary_at(p.raw, cx),
            Some((_, corners)) => {
                let n = corners.len();
                let tol = cx.pick_tolerance() * 2.0;
                self.hot = (0..n)
                    .map(|i| (i, near(p.raw, corners[i], corners[(i + 1) % n])))
                    .filter(|(_, d)| *d <= tol)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(i, _)| i);
            }
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.parcel.is_none() {
            if let Some(found) = Self::boundary_at(p.raw, cx) {
                self.take(found, cx);
            } else {
                cx.say(
                    Level::Info,
                    "Burada kapalı bir çizgi ya da alan yok: parselin kenarına tıklayın.",
                );
            }
            return;
        }
        self.pointer_move(p, cx);
        if let Some(i) = self.hot
            && let Some(e) = self.edges.get_mut(i)
        {
            e.kind = next(e.kind);
            self.read(cx);
        }
    }

    fn input(&mut self, text: &str, _cx: &mut Context<'_>) -> bool {
        if matches!(crate::prompt::upper_tr(text.trim()).as_str(), "Ö" | "O") {
            self.marks = !self.marks;
            return true;
        }
        false
    }

    fn settings(&self, _format: &Format) -> Vec<crate::tool_settings::Setting> {
        vec![crate::tool_settings::Setting::toggle(
            "Ö",
            "Ölçüler (bahçe mesafeleri ve ±0,00)",
            self.marks,
        )]
    }

    fn configure(&mut self, key: &str, value: &str, _cx: &mut Context<'_>) -> bool {
        if key != "Ö" {
            return false;
        }
        self.marks = matches!(
            crate::prompt::fold_tr(value.trim()).as_str(),
            "YES" | "AÇIK" | "ACIK" | "EVET" | "1" | "TRUE"
        );
        true
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.write(cx) {
            Flow::Exit
        } else {
            Flow::Stay
        }
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = Preview::default();
        if let Some((_, corners)) = &self.hovered
            && self.parcel.is_none()
        {
            out.strokes
                .push(Stroke::solid(corners.clone(), true).width(2.0));
        }
        if let (Some((_, corners)), Some(r)) = (&self.parcel, &self.report) {
            let n = corners.len();
            for i in 0..n {
                let (a, b) = (corners[i], corners[(i + 1) % n]);
                let e = r.edges.get(i);
                let short = e.is_some_and(kentos_domain::site::SiteEdgeReport::short);
                let tone = if short { Tone::Danger } else { Tone::Accent };
                out.strokes.push(
                    Stroke::solid(vec![a, b], false)
                        .width(if self.hot == Some(i) { 4.0 } else { 2.0 })
                        .tone(tone),
                );
                if let Some(e) = e {
                    out.labels.push(Label {
                        at: Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0),
                        text: format!("{} {} m", e.kind.label(), m2(e.required)),
                        offset: [6.0, -6.0],
                        tone,
                    });
                }
            }
            for area in &r.envelope {
                out.strokes.push(
                    Stroke::dashed(
                        area.outer.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect(),
                        true,
                        [6.0, 4.0],
                    )
                    .tone(Tone::Danger),
                );
            }
        }
        out.tag = self.at.map(|at| Tag {
            at,
            lines: self.lines(),
        });
        out
    }
}
