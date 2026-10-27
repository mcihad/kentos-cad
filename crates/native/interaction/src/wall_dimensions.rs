//! A wall's live opening and overall dimensions: choose a source, review, place once. On a wall of more than one
//! edge (a closed room, a polyline wall) the straight edge nearest the pointer is measured, or every straight edge
//! at once (docs/adr/0700).
use kentos_commands::{
    CommandResult, DimensionSetFace, DimensionSetOpenings, DimensionSetRow,
    DimensionSetsCreateInput, Entity, STANDARD_STYLE, TextFace,
};
use kentos_domain::dimension_sets::WallEdges;
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{ExecutionContext, dimension_sets_create, geometry::shape};

use crate::edge::Outline;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Cursor, Flow, OptionChoice, Pointer, Preview, Stroke, Tag, TextGhost, Tone, Tool,
};
use crate::tool_settings::{Setting, choices};
use crate::{Format, Level, Vec2, dimension_layout, styles};

pub const ID: &str = "wallDimensions";
pub const LABEL: &str = "Duvar ölçüleri";

pub struct WallDimensions {
    wall: Option<Uuid>,
    /// The chosen wall's edges; none before a wall is chosen.
    edges: Option<WallEdges>,
    /// The straight edge measured: the one nearest the pointer on a wall of more than one edge; none on a two-point
    /// wall, whose only edge a recipe keeps without a number.
    edge: Option<u32>,
    /// Kenarlar → Bütün kenarlar: a chain on every straight edge, in one step.
    around: bool,
    source: String,
    at: Option<Vec2>,
    /// An entered distance stays authoritative while the pointer visits controls or menus.
    fixed_distance: Option<f64>,
    left: bool,
    face_override: Option<bool>,
    layer: String,
    style: Option<String>,
    openings: bool,
    overall: bool,
    centers: bool,
    gap_mm: f64,
    revision: Option<String>,
    seen: Option<u64>,
    drawn: Preview,
    error: Option<String>,
    count: usize,
    asking: Option<String>,
    layer_choices: Vec<OptionChoice>,
    style_choices: Vec<OptionChoice>,
    distance: String,
    done: bool,
}

impl Default for WallDimensions {
    fn default() -> Self {
        Self {
            wall: None,
            edges: None,
            edge: None,
            around: false,
            source: "Duvar seçin".into(),
            at: None,
            fixed_distance: None,
            left: true,
            face_override: None,
            layer: String::new(),
            style: None,
            openings: true,
            overall: true,
            centers: false,
            gap_mm: 7.5,
            revision: None,
            seen: None,
            drawn: Preview::default(),
            error: None,
            count: 0,
            asking: None,
            layer_choices: Vec::new(),
            style_choices: Vec::new(),
            distance: String::new(),
            done: false,
        }
    }
}

/// A wall path's edge `edge`: its start, its left normal and its length; none for a wall without that edge.
fn edge_basis(doc: &Document, uid: Uuid, edge: u32) -> Option<(Vec2, Vec2, f64)> {
    let Entity::Wall(w) = doc.get(doc.slot_of(uid)?)? else {
        return None;
    };
    let (n, k) = (w.wall.pts.len(), edge as usize);
    if k >= n || (!w.wall.closed && k + 1 >= n) {
        return None;
    }
    let a = Vec2::new(w.wall.pts[k].x, w.wall.pts[k].y);
    let b = Vec2::new(w.wall.pts[(k + 1) % n].x, w.wall.pts[(k + 1) % n].y);
    let length = (b.x - a.x).hypot(b.y - a.y);
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some((
        a,
        Vec2::new(-(b.y - a.y) / length, (b.x - a.x) / length),
        length,
    ))
}

/// The wall's straight edge nearest `p` along its path, the lower number on a tie.
fn nearest_edge(doc: &Document, uid: Uuid, edges: &WallEdges, p: Vec2) -> Option<u32> {
    edges
        .straight
        .iter()
        .filter_map(|&k| {
            let (a, n, length) = edge_basis(doc, uid, k)?;
            let b = Vec2::new(a.x + n.y * length, a.y - n.x * length);
            Some((k, kentos_geometry_core::geometry::dist_to_segment(p, a, b)))
        })
        .min_by(|x, y| x.1.total_cmp(&y.1))
        .map(|(k, _)| k)
}

/// A wall with more than one edge needs one named (docs/adr/0700).
fn many(edges: &WallEdges) -> bool {
    edges.closed || edges.count > 1
}

pub(crate) fn wrapped(text: &str) -> Vec<String> {
    let mut rows = vec![String::new()];
    for word in text.split_whitespace() {
        if rows
            .last()
            .is_some_and(|s| s.chars().count() + word.chars().count() > 48)
        {
            rows.push(String::new());
        }
        if let Some(row) = rows.last_mut() {
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(word);
        }
    }
    rows
}

impl WallDimensions {
    fn place_distance(&mut self, distance: f64, doc: &Document) -> bool {
        let (Some(uid), Some(at)) = (self.wall, self.at) else {
            return false;
        };
        let Some((a, n, _)) = self.face_basis(doc, uid) else {
            return false;
        };
        let old = (at.x - a.x) * n.x + (at.y - a.y) * n.y;
        let sign = if self.left { 1. } else { -1. };
        self.at = Some(Vec2::new(
            at.x + n.x * (sign * distance - old),
            at.y + n.y * (sign * distance - old),
        ));
        true
    }

    fn face(&self) -> DimensionSetFace {
        if self.left {
            DimensionSetFace::Left
        } else {
            DimensionSetFace::Right
        }
    }

    /// The measured face edge: its start, its left normal and its length.
    fn face_basis(&self, doc: &Document, uid: Uuid) -> Option<(Vec2, Vec2, f64)> {
        Self::face_edge(doc, uid, self.face(), self.edge)
    }

    fn face_edge(
        doc: &Document,
        uid: Uuid,
        face: DimensionSetFace,
        edge: Option<u32>,
    ) -> Option<(Vec2, Vec2, f64)> {
        let points = doc.dimension_set_wall_face(uid, face, edge).ok()?;
        let a = Vec2::new(points[0].x, points[0].y);
        let b = Vec2::new(points[1].x, points[1].y);
        let length = (b.x - a.x).hypot(b.y - a.y);
        (length.is_finite() && length > 0.0).then(|| {
            (
                a,
                Vec2::new(-(b.y - a.y) / length, (b.x - a.x) / length),
                length,
            )
        })
    }

    /// The closed wall's outer face is the left one: on a clockwise path; none for an open wall.
    fn outer_left(&self) -> Option<bool> {
        self.edges
            .as_ref()
            .filter(|e| e.closed)
            .map(|e| !e.counter_clockwise)
    }
    fn request(&self, expected: bool) -> Option<DimensionSetsCreateInput> {
        let wall = self.wall?;
        let at = self.at?;
        let mut rows = Vec::new();
        if self.openings {
            rows.push(DimensionSetRow::Openings);
        }
        if self.overall {
            rows.push(DimensionSetRow::Overall);
        }
        Some(DimensionSetsCreateInput {
            wall: wall.to_string(),
            layer_id: self.layer.clone(),
            at: kentos_commands::Vec2 { x: at.x, y: at.y },
            edge: self.edge,
            around: self.around,
            face: self.face(),
            rows,
            openings: if self.centers {
                DimensionSetOpenings::Centers
            } else {
                DimensionSetOpenings::Ends
            },
            gap_mm: self.gap_mm,
            dimension_style: self.style.clone(),
            expected_revision: expected.then(|| self.revision.clone()).flatten(),
        })
    }

    fn menus(&mut self, cx: &Context<'_>) {
        self.layer_choices = cx
            .doc
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| {
                !cx.doc.layers().is_locked(&l.id)
                    && l.service.is_none()
                    && l.geometry
                        .is_none_or(|geometry| geometry.admits("dimension"))
            })
            .map(|l| OptionChoice {
                typed: l.id.clone(),
                label: cx.doc.layers().path(&l.id),
                checked: l.id == self.layer,
                icon: None,
                preview: None,
                command: None,
            })
            .collect();
        self.style_choices = vec![OptionChoice {
            typed: STANDARD_STYLE.into(),
            label: STANDARD_STYLE.into(),
            checked: self.style.is_none(),
            icon: None,
            preview: None,
            command: None,
        }];
        self.style_choices
            .extend(
                cx.doc
                    .settings()
                    .dimension_styles
                    .iter()
                    .map(|s| OptionChoice {
                        typed: s.name.clone(),
                        label: s.name.clone(),
                        checked: self.style.as_deref() == Some(s.name.as_str()),
                        icon: None,
                        preview: None,
                        command: None,
                    }),
            );
    }

    fn see(&mut self, cx: &Context<'_>) {
        if let Some(distance) = self.fixed_distance {
            self.place_distance(distance, cx.doc);
        }
        self.seen = Some(cx.doc.revision());
        self.menus(cx);
        self.drawn = Preview::default();
        self.error = None;
        self.revision = None;
        self.count = 0;
        if let Some(uid) = self.wall {
            if let Some(entity) = cx.doc.slot_of(uid).and_then(|s| cx.doc.get(s)) {
                self.drawn
                    .strokes
                    .extend(Outline::of(&shape(entity), Some([6., 4.]), 2.0, Tone::Snap).strokes);
            }
            // The measured face edges, solid over the dashed wall: the one edge, or every straight edge.
            let measured: Vec<Option<u32>> = match &self.edges {
                Some(edges) if self.around && many(edges) => {
                    edges.straight.iter().map(|&k| Some(k)).collect()
                }
                _ => vec![self.edge],
            };
            for edge in measured {
                if let Some((a, n, length)) = Self::face_edge(cx.doc, uid, self.face(), edge) {
                    let mut stroke = Stroke::solid(
                        vec![a, Vec2::new(a.x + n.y * length, a.y - n.x * length)],
                        false,
                    );
                    stroke.width = 3.0;
                    stroke.tone = Tone::Snap;
                    self.drawn.strokes.push(stroke);
                }
            }
            if let (Some((a, n, _)), Some(at)) = (self.face_basis(cx.doc, uid), self.at) {
                self.distance = cx
                    .format()
                    .length(((at.x - a.x) * n.x + (at.y - a.y) * n.y).abs());
            }
        }
        let Some(input) = self.request(false) else {
            return;
        };
        // The command's read-only plan is the only source of geometry: no project clone per pointer event.
        match dimension_sets_create::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                self.revision = Some(output.revision);
                self.count = output.entities.len();
                for entity in &output.entities {
                    self.dimension_ghost(entity, cx);
                }
                let edges = self
                    .edges
                    .as_ref()
                    .filter(|e| self.around && many(e))
                    .map_or(1, |e| e.straight.len());
                self.drawn.tag = self.at.map(|at| Tag {
                    at,
                    lines: vec![
                        if edges > 1 {
                            format!("{} ölçü · {edges} kenar · tek adım", self.count)
                        } else {
                            format!("{} ölçü · tek adım", self.count)
                        },
                        if self.fixed_distance.is_some() {
                            "Uzaklık sabit · tıklayın veya Enter ile yerleştirin"
                        } else {
                            "Yerini seçin veya Enter ile yerleştirin"
                        }
                        .into(),
                    ],
                });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.error = Some(error.message),
            _ => self.error = Some("Ölçü önizlemesi hazır değil; işlemi yeniden başlatın.".into()),
        }
        if let Some(why) = &self.error {
            self.drawn.tag = self.at.map(|at| Tag {
                at,
                lines: wrapped(why),
            });
            self.drawn.tag_tone = Tone::Danger;
        }
    }

    fn dimension_ghost(&mut self, entity: &Entity, cx: &Context<'_>) {
        let preview = dimension_preview(entity, &cx.format(), cx.doc.settings().drawing_font);
        self.drawn.strokes.extend(preview.strokes);
        self.drawn.texts.extend(preview.texts);
    }

    fn choose(&mut self, slot: Slot, near: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(Entity::Wall(w)) = cx.doc.get(slot) else {
            return;
        };
        let Some(uid) = cx.doc.uid(slot) else { return };
        let Some(edges) = cx
            .doc
            .dimension_set_wall_edges(uid)
            .filter(|e| !e.straight.is_empty())
        else {
            cx.say(
                Level::Info,
                "Bu duvarın düz kenarı yok; ölçü dizisi yalnız düz kenarı ölçer. Düz kenarlı bir duvar seçin.",
            );
            return;
        };
        let layer = cx
            .doc
            .layers()
            .get(&w.base.layer_id)
            .map_or(w.base.layer_id.as_str(), |l| l.name.as_str());
        self.source = if many(&edges) {
            format!("Duvar · {layer} · {} kenar", edges.count)
        } else {
            format!("Duvar · {layer}")
        };
        self.edge = many(&edges).then(|| {
            near.and_then(|p| nearest_edge(cx.doc, uid, &edges, p))
                .unwrap_or(edges.straight[0])
        });
        // A closed wall starts on its outer face.
        if edges.closed && self.face_override.is_none() {
            self.left = !edges.counter_clockwise;
        }
        self.wall = Some(uid);
        self.edges = Some(edges);
        if let Some((a, n, length)) = self.face_basis(cx.doc, uid) {
            let distance = 3.0 * styles::standard_dimension_height(cx.doc.settings());
            let side = if self.left { 1.0 } else { -1.0 };
            self.at = Some(Vec2::new(
                a.x + n.y * length * 0.5 + n.x * distance * side,
                a.y - n.x * length * 0.5 + n.y * distance * side,
            ));
        }
        self.see(cx);
    }

    fn source_at(p: &Pointer, cx: &Context<'_>) -> Option<Slot> {
        let allowed = |slot: Slot| {
            cx.doc.get(slot).is_some_and(|entity| {
                matches!(entity, Entity::Wall(_))
                    && crate::selectable::entity_allowed(cx.draft.select_kinds, entity)
                    && crate::selectable::on_selected_layer(cx.doc, entity)
            })
        };
        let edge = cx
            .spatial
            .hits(p.raw, cx.pick_tolerance())
            .into_iter()
            .find(|slot| allowed(*slot));
        if edge.is_some() {
            return edge;
        }
        // Walls without a hatch have no generic pickable interior. This tool
        // still accepts their real body, including unhatched finish bands.
        // The spatial bounds query preserves visibility and avoids a broad pick radius.
        cx.spatial
            .store()
            .near(p.raw, 0.)
            .into_iter()
            .filter_map(|item| {
                let slot = Slot(item.id as u32);
                if !allowed(slot) {
                    return None;
                }
                let entity = cx.doc.get(slot)?;
                let kentos_geometry_core::entity::Shape::Wall { pts, bulges, wall } = shape(entity)
                else {
                    return None;
                };
                let (ring, holes) =
                    kentos_geometry_core::geom::wall::body(&pts, bulges.as_deref(), &wall).ok()?;
                if !kentos_geometry_core::geometry::point_in_polygon(p.raw, &ring)
                    || holes
                        .iter()
                        .flatten()
                        .any(|hole| kentos_geometry_core::geometry::point_in_polygon(p.raw, hole))
                {
                    return None;
                }
                // The nearest physical centre line: the faces' middle along each edge of the path.
                let faces =
                    kentos_geometry_core::geom::wall::lines(&pts, bulges.as_deref(), &wall).ok()?;
                let (left, right) = (faces.first()?, faces.last()?);
                let n = left.pts.len().min(right.pts.len());
                let middle = |i: usize| {
                    Vec2::new(
                        (left.pts[i].x + right.pts[i].x) * 0.5,
                        (left.pts[i].y + right.pts[i].y) * 0.5,
                    )
                };
                let edges = if wall.closed { n } else { n.saturating_sub(1) };
                (0..edges)
                    .map(|i| {
                        kentos_geometry_core::geometry::dist_to_segment(
                            p.raw,
                            middle(i),
                            middle((i + 1) % n),
                        )
                    })
                    .min_by(f64::total_cmp)
                    .map(|d| (slot, d))
            })
            .min_by(|a, b| {
                a.1.total_cmp(&b.1)
                    .then_with(|| cx.doc.place(a.0).cmp(&cx.doc.place(b.0)))
            })
            .map(|(slot, _)| slot)
    }

    fn place_at(&mut self, at: Vec2, cx: &Context<'_>) {
        self.at = Some(at);
        if let Some(uid) = self.wall {
            // The edge under the pointer, on a wall of more than one.
            if let Some(edges) = self.edges.as_ref().filter(|e| many(e))
                && let Some(k) = nearest_edge(cx.doc, uid, edges, at)
            {
                self.edge = Some(k);
            }
            if let Some(left) = self.outer_left().filter(|_| self.around) {
                // Every edge of a closed wall is measured on its outer face.
                self.left = left;
            } else if let Some((mut a, n, _)) = edge_basis(cx.doc, uid, self.edge.unwrap_or(0)) {
                if let (Ok(left), Ok(right)) = (
                    cx.doc
                        .dimension_set_wall_face(uid, DimensionSetFace::Left, self.edge),
                    cx.doc
                        .dimension_set_wall_face(uid, DimensionSetFace::Right, self.edge),
                ) {
                    a = Vec2::new(
                        (left[0].x + right[0].x) * 0.5,
                        (left[0].y + right[0].y) * 0.5,
                    );
                }
                self.left = self
                    .face_override
                    .unwrap_or((at.x - a.x) * n.x + (at.y - a.y) * n.y >= 0.0);
            }
        }
        self.see(cx);
    }

    fn write(&mut self, cx: &mut Context<'_>) -> bool {
        if self.count == 0 || self.error.is_some() {
            cx.say(
                Level::Warn,
                self.error.clone().unwrap_or_else(|| {
                    "Önce bir duvar seçin ve ölçü sıralarını kontrol edin.".into()
                }),
            );
            return false;
        }
        let Some(input) = self.request(true) else {
            return false;
        };
        let result = dimension_sets_create::execute(&mut ExecutionContext::new(cx.doc), input);
        match crate::points::written(result, cx) {
            Some(output) => {
                let ids: Vec<Slot> = output
                    .uids
                    .iter()
                    .filter_map(|u| u.parse().ok())
                    .filter_map(|u| cx.doc.slot_of(u))
                    .collect();
                if !ids.is_empty() {
                    cx.selection.set(ids);
                }
                cx.say(
                    Level::Success,
                    if output.sets.len() > 1 {
                        format!(
                            "{} ölçü {} kenara yerleştirildi; duvar ve açıklıkları değişince güncellenir.",
                            self.count,
                            output.sets.len()
                        )
                    } else {
                        format!(
                            "{} ölçü yerleştirildi; duvarın açıklıkları değişince güncellenir.",
                            self.count
                        )
                    },
                );
                self.done = true;
                true
            }
            None => {
                self.see(cx);
                false
            }
        }
    }
}

/// The actual dimension layout and value, shared by source-edit previews.
pub(crate) fn dimension_preview(
    entity: &Entity,
    format: &Format,
    font: Option<kentos_commands::DrawingFont>,
) -> Preview {
    let mut preview = Preview::default();
    let Entity::Dimension(d) = entity else {
        return preview;
    };
    let Some(layout) = dimension_layout(entity) else {
        return preview;
    };
    preview.strokes.extend(
        layout
            .lines
            .iter()
            .map(|&[a, b]| Stroke::solid(vec![a, b], false)),
    );
    preview.strokes.extend(
        layout
            .fills
            .into_iter()
            .flatten()
            .map(|p| Stroke::solid(p, true)),
    );
    preview.texts.push(TextGhost {
        p: layout.text_at,
        text: format.dimension_value(
            layout.prefix,
            layout.unit,
            layout.value,
            &d.look,
            d.text.as_deref(),
        ),
        height: d.height,
        rotation: layout.rotation,
        align: Some(kentos_geometry_core::text::TextAlign::MiddleCenter),
        mask: d.mask,
        face: TextFace {
            font: d.look.font.or(font),
            ..Default::default()
        },
        width_factor: 1.0,
    });
    preview
}

/// A source command's complete preview, including added and disappearing chain members.
pub(crate) fn append_followers(
    preview: &mut Preview,
    followers: &kentos_commands::DerivedEntitiesPlan,
    cx: &Context<'_>,
) {
    for entity in followers
        .updated
        .iter()
        .chain(&followers.added)
        .take(crate::modify::MAX_GHOSTS)
    {
        if matches!(entity, Entity::Dimension(_)) {
            let mut own = dimension_preview(entity, &cx.format(), cx.doc.settings().drawing_font);
            for stroke in &mut own.strokes {
                stroke.tone = Tone::Snap;
            }
            preview.strokes.extend(own.strokes);
            preview.texts.extend(own.texts);
        } else {
            preview
                .strokes
                .extend(Outline::of(&shape(entity), Some([2., 3.]), 1.0, Tone::Snap).strokes);
        }
    }
    for entity in followers
        .removed
        .iter()
        .filter_map(|uid| {
            uid.parse()
                .ok()
                .and_then(|uid| cx.doc.slot_of(uid))
                .and_then(|slot| cx.doc.get(slot))
        })
        .take(crate::modify::MAX_GHOSTS)
    {
        preview
            .strokes
            .extend(Outline::of(&shape(entity), Some([2., 3.]), 1.0, Tone::Danger).strokes);
    }
}

impl Tool for WallDimensions {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn point_count(&self) -> usize {
        usize::from(self.wall.is_some())
    }
    fn cursor(&self) -> Cursor {
        if self.wall.is_some() {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }
    fn snaps(&self) -> bool {
        false
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.layer = cx.doc.layers().active().into();
        self.style = styles::dimension_style(cx.memory, cx.doc.settings()).map(|s| s.name.clone());
        self.openings = cx.memory.wall_dimensions_rows & 1 != 0;
        self.overall = cx.memory.wall_dimensions_rows & 2 != 0;
        self.centers = cx.memory.wall_dimensions_centers;
        self.gap_mm = cx.memory.wall_dimensions_gap_mm;
        self.around = cx.memory.wall_dimensions_around;
        self.menus(cx);
        if !self
            .layer_choices
            .iter()
            .any(|choice| choice.typed == self.layer)
            && let Some(choice) = self.layer_choices.first()
        {
            self.layer = choice.typed.clone();
        }
        if let [slot] = cx.selection.ids() {
            self.choose(*slot, None, cx);
        } else {
            self.menus(cx);
        }
        Flow::Stay
    }
    fn prompt(&self) -> Prompt {
        if self.asking.is_some() {
            return Prompt::new(LABEL, "değeri yazın").option("Geri", "G");
        }
        Prompt::new(
            LABEL,
            if self.wall.is_some() && self.fixed_distance.is_some() {
                "girilen uzaklık sabit; tıklayın veya Enter ile yerleştirin"
            } else if self.edges.as_ref().is_some_and(|e| many(e) && !self.around) {
                "ölçülecek kenarı ve sıraların yerini imleçle seçin veya uzaklık yazın"
            } else if self.wall.is_some() {
                "ölçü sıralarının yerini seçin veya uzaklık yazın"
            } else {
                "ölçülendirilecek duvara tıklayın"
            },
        )
        .option("Başka duvar", "K")
        .option("Yerleştir", "Enter")
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.wall.is_none() {
            cx.selection.set_hover(Self::source_at(p, cx));
            return;
        }
        if self.fixed_distance.is_some() {
            if self.seen != Some(cx.doc.revision()) {
                self.see(cx);
            }
            return;
        }
        if self.at != Some(p.raw) || self.seen != Some(cx.doc.revision()) {
            self.place_at(p.raw, cx);
        }
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.wall.is_none() {
            if let Some(slot) = Self::source_at(p, cx) {
                self.choose(slot, Some(p.raw), cx);
            } else {
                cx.say(Level::Info, "Ölçülendirilecek duvara tıklayın.");
            }
        } else {
            if self.fixed_distance.is_none() {
                self.place_at(p.raw, cx);
            }
            self.write(cx);
        }
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(key) = self.asking.take() {
            return self.configure(&key, text, cx);
        }
        let key = upper_tr(text.trim());
        if key == "K" {
            self.wall = None;
            self.edges = None;
            self.edge = None;
            self.source = "Duvar seçin".into();
            self.at = None;
            self.fixed_distance = None;
            self.distance.clear();
            self.see(cx);
            return true;
        }
        if self
            .settings(&cx.format())
            .iter()
            .any(|s| s.key == key && s.editable)
        {
            self.asking = Some(key);
            return true;
        }
        if self.wall.is_some() && cx.typed_length(text).is_some() {
            if self.configure("D", text, cx) {
                self.write(cx);
            }
            return true;
        }
        false
    }
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.take().is_some() {
            return Flow::Stay;
        }
        if self.write(cx) {
            Flow::Exit
        } else {
            Flow::Stay
        }
    }
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.asking.take().is_some()
    }
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            return true;
        }
        if self.wall.take().is_none() {
            return false;
        }
        self.edges = None;
        self.edge = None;
        self.source = "Duvar seçin".into();
        self.at = None;
        self.fixed_distance = None;
        self.distance.clear();
        self.see(cx);
        true
    }
    fn finished(&self) -> bool {
        self.done
    }
    fn preview(&self, _: &Format) -> Preview {
        self.drawn.clone()
    }
    fn settings(&self, _: &Format) -> Vec<Setting> {
        let face = match self.face_override {
            None => "auto",
            Some(true) => "left",
            Some(false) => "right",
        };
        // A closed wall's faces by where they are: the outer one on the right of a counter-clockwise path.
        let faces = match &self.edges {
            Some(e) if e.closed && e.counter_clockwise => {
                [("auto", "İmleç tarafı"), ("right", "Dış yüz"), ("left", "İç yüz")]
            }
            Some(e) if e.closed => [("auto", "İmleç tarafı"), ("left", "Dış yüz"), ("right", "İç yüz")],
            _ => [
                ("auto", "İmleç tarafı"),
                ("left", "Duvarın sol yüzü"),
                ("right", "Duvarın sağ yüzü"),
            ],
        };
        let all_outer = self.around && self.outer_left().is_some();
        let mut rows = vec![
            Setting::text("C", "Kaynak", self.source.clone()).read_only(),
            Setting::choice("L", "Ölçü katmanı", self.layer_choices.clone()),
            Setting::choice("S", "Ölçü stili", self.style_choices.clone()),
            if all_outer {
                Setting::text("F", "Ölçülen yüz", "Dış yüz (bütün kenarlar)").read_only()
            } else {
                Setting::choice("F", "Ölçülen yüz", choices(&faces, face))
            },
            Setting::toggle("A", "Açıklık sırası", self.openings),
            Setting::choice(
                "B",
                "Açıklıkları ölç",
                choices(
                    &[
                        ("ends", "Açıklık uçları"),
                        ("centers", "Açıklık merkezleri"),
                    ],
                    if self.centers { "centers" } else { "ends" },
                ),
            ),
            Setting::toggle("T", "Toplam boy", self.overall),
            Setting::choice(
                "P",
                "Yerleşim",
                choices(
                    &[("cursor", "İmleçle"), ("fixed", "Sabit uzaklık")],
                    if self.fixed_distance.is_some() {
                        "fixed"
                    } else {
                        "cursor"
                    },
                ),
            ),
            Setting::text("D", "Duvara uzaklık", self.distance.clone()),
            Setting::text("R", "Sıra aralığı (kağıtta)", format!("{} mm", self.gap_mm)),
            Setting::hint(
                "H",
                "Duvar ölçüleri",
                if self.fixed_distance.is_some() {
                    "Girilen uzaklık korunur. Tıklama veya Enter ile yerleştirin. Yeniden fareyle yer seçmek için Yerleşim → İmleçle seçin. Duvarın açıklıkları değişince ölçü dizisi güncellenir."
                } else {
                    "Duvarın imlece en yakın düz kenarındaki açıklıkları ve toplam boyu birlikte ölçülendirir; Kenarlar → Bütün kenarlar kapalı duvarın her kenarına dış yüzden zincir koyar. İmleçle yer seçin veya Duvara uzaklık girin; girilen uzaklık sabitlenir. Ölçü stili ve mevcut çizim ölçeği kullanılır."
                },
            ),
        ];
        // Kenarlar: shown while it can matter, before a wall or on one with more than one straight edge.
        if self
            .edges
            .as_ref()
            .is_none_or(|e| e.straight.len() > 1)
        {
            rows.insert(
                4,
                Setting::choice(
                    "N",
                    "Kenarlar",
                    choices(
                        &[("one", "Bu kenar"), ("all", "Bütün kenarlar")],
                        if self.around { "all" } else { "one" },
                    ),
                ),
            );
        }
        if let Some(why) = &self.error {
            rows.push(Setting::notice("E", "Yerleştirilemiyor", why));
        }
        rows
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let value = value.trim();
        let valid = match key {
            "L" if self.layer_choices.iter().any(|c| c.typed == value) => {
                self.layer = value.into();
                true
            }
            "S" if self.style_choices.iter().any(|c| c.typed == value) => {
                self.style = (value != STANDARD_STYLE).then(|| value.into());
                styles::take_dimension(value, cx)
            }
            "F" => match value {
                "auto" => {
                    self.face_override = None;
                    true
                }
                "left" => {
                    self.face_override = Some(true);
                    self.left = true;
                    true
                }
                "right" => {
                    self.face_override = Some(false);
                    self.left = false;
                    true
                }
                _ => false,
            },
            "A" | "T" if matches!(value, "yes" | "no") => {
                if key == "A" {
                    self.openings = value == "yes";
                } else {
                    self.overall = value == "yes";
                }
                true
            }
            "B" if matches!(value, "ends" | "centers") => {
                self.centers = value == "centers";
                true
            }
            "N" if matches!(value, "one" | "all") => {
                self.around = value == "all";
                true
            }
            "R" => match paper_mm(value) {
                Some(mm) if mm > 0.0 && mm <= 1000.0 => {
                    self.gap_mm = mm;
                    true
                }
                _ => false,
            },
            "P" => match value {
                "cursor" => {
                    self.fixed_distance = None;
                    true
                }
                "fixed" => {
                    if let (Some(uid), Some(at)) = (self.wall, self.at)
                        && let Some((a, n, _)) = self.face_basis(cx.doc, uid)
                    {
                        self.fixed_distance = Some(((at.x - a.x) * n.x + (at.y - a.y) * n.y).abs());
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            "D" => match (self.wall, self.at, cx.typed_length(value)) {
                (Some(_), Some(_), Some(d))
                    if d.is_finite() && d > 0.0 && self.place_distance(d, cx.doc) =>
                {
                    self.fixed_distance = Some(d);
                    true
                }
                _ => false,
            },
            _ => false,
        };
        if valid {
            cx.memory.wall_dimensions_rows = u8::from(self.openings) | (u8::from(self.overall) * 2);
            cx.memory.wall_dimensions_centers = self.centers;
            cx.memory.wall_dimensions_gap_mm = self.gap_mm;
            cx.memory.wall_dimensions_around = self.around;
            if (key == "N" || key == "F" && self.face_override.is_none())
                && self.fixed_distance.is_none()
                && let Some(at) = self.at
            {
                self.place_at(at, cx);
            } else {
                self.see(cx);
            }
        } else {
            cx.say(
                Level::Warn,
                if key == "R" {
                    "Kağıt üzerinde 0'dan büyük, en çok 1000 mm bir aralık girin; örneğin 7.5 mm."
                } else if key == "P" && self.wall.is_none() {
                    "Önce ölçülendirilecek duvarı seçin."
                } else {
                    "Geçerli bir değer seçin; uzaklığı 40 cm veya 0.4 m gibi yazabilirsiniz."
                },
            );
        }
        valid
    }
}

fn paper_mm(text: &str) -> Option<f64> {
    let text = text.trim();
    let number = text.strip_suffix("mm").unwrap_or(text).trim();
    kentos_geometry_core::tools::point_text::parse_number(number).filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paper_spacing_is_paper_millimetres_and_rejects_other_units() {
        assert_eq!(paper_mm("7.5 mm"), Some(7.5));
        assert_eq!(paper_mm("12"), Some(12.0));
        assert_eq!(paper_mm("0.5 m"), None);
        assert_eq!(paper_mm("10 cm"), None);
    }
}
