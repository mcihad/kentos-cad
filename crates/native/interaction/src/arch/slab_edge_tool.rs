//! Edits one straight slab perimeter edge through `cad.slabs.edit`.

use kentos_commands::{CommandResult, Entity, SlabEdgeLink, SlabsEdit, SlabsOperation};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::wall;
use kentos_geometry_core::tools::point_text::parse_length;
use kentos_native_application::{ExecutionContext, geometry::shape, slabs_edit};

use crate::tool::{Context, Flow, OptionChoice, Pointer, Preview, Stroke, Tone, Tool};
use crate::tool_settings::Setting;
use crate::{Vec2, format::Format, log::Level, points, prompt::Prompt};

pub const ID: &str = "slabEdgeBind";
const LABEL: &str = "Döşeme kenarı bağla";
const REACH: f64 = 18.0;

#[derive(Clone, Debug, Default)]
pub struct SlabEdgeTool {
    edge: Option<(String, u32)>,
    edge_line: Option<[Vec2; 2]>,
    link: Option<SlabEdgeLink>,
    faces: usize,
    offset: f64,
    locked: bool,
    detach: bool,
    was_bound: bool,
    preview: Preview,
}

fn distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (x, y) = (b.x - a.x, b.y - a.y);
    let den = x * x + y * y;
    if den <= 1e-16 {
        return f64::INFINITY;
    }
    let t = (((p.x - a.x) * x + (p.y - a.y) * y) / den).clamp(0.0, 1.0);
    (p.x - a.x - x * t).hypot(p.y - a.y - y * t)
}

fn points_of(line: &wall::Line, i: usize, closed: bool) -> Option<[Vec2; 2]> {
    if line.bulges.get(i).is_some_and(|v| *v != 0.0) {
        return None;
    }
    Some([
        *line.pts.get(i)?,
        *line
            .pts
            .get(i + 1)
            .or_else(|| closed.then(|| &line.pts[0]))?,
    ])
}

fn wall_lines(e: &Entity) -> Option<(Vec<wall::Line>, bool)> {
    let Shape::Wall { pts, bulges, wall } = shape(e) else {
        return None;
    };
    Some((
        wall::lines(&pts, bulges.as_deref(), &wall).ok()?,
        wall.closed,
    ))
}

impl SlabEdgeTool {
    fn find_edge(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let reach = cx.view.world_length(REACH);
        let mut best = None;
        let mut nearest = reach;
        for item in cx.spatial.store().near(p, reach) {
            let slot = Slot(item.id as u32);
            let Some(Entity::Slab(s)) = cx.doc.get(slot) else {
                continue;
            };
            if s.slab.arch.level.as_deref() != cx.level || s.slab.parts.len() != 1 {
                continue;
            }
            let ring = &s.slab.parts[0].outer;
            if ring
                .bulges
                .as_ref()
                .is_some_and(|v| v.iter().any(|b| *b != 0.0))
            {
                continue;
            }
            for (i, (a, b)) in ring
                .pts
                .iter()
                .zip(ring.pts.iter().cycle().skip(1))
                .enumerate()
            {
                let line = [Vec2::new(a.x, a.y), Vec2::new(b.x, b.y)];
                let d = distance(p, line[0], line[1]);
                if d < nearest {
                    nearest = d;
                    best = Some((slot, i as u32, line));
                }
            }
        }
        let Some((slot, edge, line)) = best else {
            cx.say(
                Level::Warn,
                "Etkin kattaki tek parçalı döşemenin doğrusal dış kenarına tıklayın.",
            );
            return;
        };
        let Some(uid) = cx.doc.uid(slot) else { return };
        self.edge = Some((uid.to_string(), edge));
        self.edge_line = Some(line);
        let Some(Entity::Slab(s)) = cx.doc.get(slot) else {
            return;
        };
        self.link = s
            .slab
            .bound
            .as_ref()
            .and_then(|b| b.edges.as_ref())
            .and_then(|e| e.links.iter().find(|l| l.edge == edge).cloned());
        self.was_bound = self.link.is_some();
        if let Some(l) = &self.link {
            self.offset = l.offset;
            self.locked = l.locked;
            self.faces = kentos_domain::Uuid::parse_str(&l.wall)
                .ok()
                .and_then(|u| cx.doc.slot_of(u))
                .and_then(|s| cx.doc.get(s))
                .and_then(wall_lines)
                .map_or(0, |(lines, _)| lines.len());
        }
        self.redraw(cx);
    }

    fn find_wall(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let reach = cx.view.world_length(REACH);
        let mut nearest = reach;
        let mut best = None;
        for item in cx.spatial.store().near(p, reach) {
            let slot = Slot(item.id as u32);
            let Some(e @ Entity::Wall(_)) = cx.doc.get(slot).filter(|e| e.own_level() == cx.level)
            else {
                continue;
            };
            let Some((lines, closed)) = wall_lines(e) else {
                continue;
            };
            for (face, line) in lines.iter().enumerate() {
                for i in 0..line.pts.len() {
                    let Some([a, b]) = points_of(line, i, closed) else {
                        continue;
                    };
                    let d = distance(p, a, b);
                    if d < nearest {
                        nearest = d;
                        best = Some((slot, face, i, lines.len()));
                    }
                }
            }
        }
        let Some((slot, face, segment, faces)) = best else {
            cx.say(
                Level::Warn,
                "Etkin kattaki duvarın doğrusal bileşen yüzüne tıklayın.",
            );
            return;
        };
        let (Some((_, edge)), Some(uid)) = (&self.edge, cx.doc.uid(slot)) else {
            return;
        };
        self.link = Some(SlabEdgeLink {
            edge: *edge,
            wall: uid.to_string(),
            segment: segment as u32,
            face: (face + 1 != faces).then_some(face as u32),
            offset: self.offset,
            locked: self.locked,
        });
        self.faces = faces;
        self.detach = false;
        self.redraw(cx);
    }

    fn changes(&self, rev: u64) -> Option<SlabsEdit> {
        let (uid, edge) = self.edge.as_ref()?;
        let operation = if self.detach {
            SlabsOperation::UnbindEdge { edge: *edge }
        } else {
            let mut link = self.link.clone()?;
            link.offset = self.offset;
            link.locked = self.locked;
            SlabsOperation::BindEdge(link)
        };
        Some(SlabsEdit {
            uids: vec![uid.clone()],
            operation,
            expected_revision: Some(rev.to_string()),
        })
    }

    fn redraw(&mut self, cx: &Context<'_>) {
        self.preview = Preview::default();
        if let Some(line) = self.edge_line {
            self.preview.strokes.push(
                Stroke::dashed(line.to_vec(), false, [7.0, 4.0])
                    .width(3.0)
                    .tone(Tone::Snap),
            );
        }
        if let Some(edit) = self.changes(cx.doc.revision())
            && let CommandResult::Completed { output, .. } = slabs_edit::plan_in(cx.doc, &edit)
            && let Some(s) = output.slabs.first()
        {
            crate::slab_tools::drawn_slab(s, Tone::Accent, &mut self.preview);
        }
    }
}

impl Tool for SlabEdgeTool {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn snaps(&self) -> bool {
        false
    }
    fn point_count(&self) -> usize {
        usize::from(self.edge.is_some()) + usize::from(self.link.is_some())
    }
    fn prompt(&self) -> Prompt {
        Prompt::new(
            LABEL,
            if self.edge.is_none() {
                "döşemenin dış kenarını seçin"
            } else if self.link.is_none() {
                "kaynak duvarın izlenecek yüzünü seçin"
            } else {
                "ayarları gözden geçirin, başka yüz seçin veya Enter ile uygulayın"
            },
        )
    }

    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.edge.is_some() {
            self.find_wall(p.raw, cx);
        } else {
            self.find_edge(p.raw, cx);
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(at) = cx.typed_point(text, None, None) else {
            return false;
        };
        if self.edge.is_some() {
            self.find_wall(at, cx);
        } else {
            self.find_edge(at, cx);
        }
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        let pending = self.edge.is_some();
        *self = Self::default();
        pending
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let Some(edit) = self.changes(cx.doc.revision()) else {
            return Flow::Stay;
        };
        let result = slabs_edit::execute(&mut ExecutionContext::new(cx.doc), edit);
        if points::written(result, cx).is_some() {
            cx.say(
                Level::Success,
                if self.detach {
                    "Döşeme kenarı serbest bırakıldı."
                } else {
                    "Döşeme kenarı güncellendi."
                },
            );
            *self = Self::default();
        }
        Flow::Stay
    }

    fn settings(&self, fmt: &Format) -> Vec<Setting> {
        let mut rows = vec![
            Setting::text("offset", "Dışa öteleme", fmt.length_bare(self.offset)),
            Setting::toggle("locked", "Kenar kilidi", self.locked),
        ];
        if let Some(link) = &self.link
            && self.faces >= 2
        {
            let last = self.faces - 1;
            rows.push(Setting::choice(
                "face",
                "Bileşen yüzü",
                (0..self.faces)
                    .map(|i| OptionChoice {
                        label: if i == 0 {
                            "Sol dış yüz".into()
                        } else if i == last {
                            "Sağ dış yüz".into()
                        } else {
                            format!("Bileşen sınırı {i}")
                        },
                        typed: i.to_string(),
                        icon: None,
                        preview: None,
                        command: None,
                        checked: link.face.map_or(last, |n| n as usize) == i,
                    })
                    .collect(),
            ));
        }
        if self.was_bound {
            rows.push(Setting::toggle(
                "detach",
                "Kenarı serbest bırak",
                self.detach,
            ));
        }
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "offset" => {
                let fmt = cx.format();
                let Some(n) = parse_length(value)
                    .map(|n| n.metres(|v| fmt.to_metres(v)))
                    .filter(|n| n.abs() <= kentos_commands::MAX_BOUND_OFFSET)
                else {
                    return false;
                };
                self.offset = n;
            }
            "locked" => self.locked = value == "yes",
            "detach" => self.detach = value == "yes",
            "face" => {
                let (Some(l), Ok(i)) = (&mut self.link, value.parse::<usize>()) else {
                    return false;
                };
                if i >= self.faces {
                    return false;
                }
                l.face = (i + 1 != self.faces).then_some(i as u32);
            }
            _ => return false,
        }
        self.redraw(cx);
        true
    }

    fn preview(&self, _fmt: &Format) -> Preview {
        self.preview.clone()
    }
}
