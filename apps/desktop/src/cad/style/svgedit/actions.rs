//! What the SVG editor's menus, panels and keys do to the drawing (the
//! web's `svgActions.ts` and `SvgEditor.action`): path operations, node
//! operations, the order and the groups, flips and turns, align and
//! distribute, the numeric transforms, arrays and selection helpers. Each is
//! one undo step and says what happened in the status line.

use kentos_geometry_core::api::json::Json;
use kentos_svg_core::arrange::{
    About, TransformSpec, align_moves, copies_of, distribute_moves, invertible, transform_moves,
    translate, units_of,
};
use kentos_svg_core::model::{rotate_about, shapes_box, transform_shape};
use kentos_svg_core::nodes::{
    Joined, NodeRef, align_nodes, break_at_nodes, corner_nodes, delete_nodes, delete_segments,
    distribute_nodes, insert_mid_nodes, join_ends, segments_to, set_node_type,
};
use kentos_svg_core::ops::{
    Ids, OpResult, boolean_shapes, break_apart, close_shapes, combine_shapes, cut_shapes,
    offset_shapes, open_shapes, reverse_shapes, shapes_to_path, simplify_shapes, stroke_to_path,
};
use kentos_svg_core::path::Matrix;
use kentos_svg_core::shape::{Obj, SubPath};

use super::doc::{id_of, shape_id};
use super::node_tool::{CornerMode, node_ref};
use super::stage::array_matrices;
use super::state::{ArrayKind, SvgEditor, TransformKind};

/// A path operation (the Yol menu).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathOp {
    Union,
    Difference,
    Intersection,
    Exclusion,
    Division,
    Cut,
    Combine,
    BreakApart,
    Split,
    ToPath,
    StrokeToPath,
    Inset,
    Outset,
    Simplify,
    Reverse,
    Close,
    Open,
}

impl PathOp {
    pub fn label(self) -> &'static str {
        match self {
            PathOp::Union => "Birleşim",
            PathOp::Difference => "Fark",
            PathOp::Intersection => "Kesişim",
            PathOp::Exclusion => "Dışlama",
            PathOp::Division => "Bölme",
            PathOp::Cut => "Yolu kes",
            PathOp::Combine => "Tek yolda topla",
            PathOp::BreakApart => "Parçalara ayır",
            PathOp::Split => "Parçalara ayır (delikler kalır)",
            PathOp::ToPath => "Nesneyi yola çevir",
            PathOp::StrokeToPath => "Çizgiyi yola çevir",
            PathOp::Inset => "İçe küçült",
            PathOp::Outset => "Dışa büyüt",
            PathOp::Simplify => "Sadeleştir",
            PathOp::Reverse => "Yönü çevir",
            PathOp::Close => "Yolu kapat",
            PathOp::Open => "Yolu aç",
        }
    }

    fn boolean(self) -> Option<&'static str> {
        Some(match self {
            PathOp::Union => "union",
            PathOp::Difference => "difference",
            PathOp::Intersection => "intersection",
            PathOp::Exclusion => "exclusion",
            PathOp::Division => "division",
            _ => return None,
        })
    }
}

/// An arranging action (`ActionName`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Delete,
    Duplicate,
    Group,
    Ungroup,
    FlipH,
    FlipV,
    Rot90,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restack {
    Raise,
    Lower,
    Top,
    Bottom,
}

/// What “Aynı … seç” compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Same {
    Fill,
    Stroke,
    Both,
    Kind,
}

/// A node operation (the node tool's box and keys).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeCmd {
    Type(&'static str),
    Insert,
    Delete {
        keep_shape: bool,
    },
    Join {
        merge: bool,
    },
    Break,
    DeleteSegment,
    Segments {
        line: bool,
    },
    Corner(CornerMode),
    Align {
        axis: &'static str,
        to: &'static str,
    },
    Distribute {
        axis: &'static str,
    },
}

fn node_type_name(t: &str) -> &'static str {
    match t {
        "cusp" => "köşe",
        "smooth" => "yumuşak",
        "symmetric" => "simetrik",
        _ => "otomatik",
    }
}

/// An operation's new shapes' ids made here (the core names them `\u{1}k`).
fn with_ids(r: OpResult, ids: &Ids) -> OpResult {
    let OpResult::Done {
        mut add,
        remove,
        note,
    } = r
    else {
        return r;
    };
    if ids.count > 0 {
        let fresh: Vec<String> = (0..ids.count).map(|_| shape_id()).collect();
        let name = |v: &Json| -> Option<Json> {
            let Json::Str(s) = v else { return None };
            let k: usize = s.strip_prefix('\u{1}')?.parse().ok()?;
            fresh.get(k).map(|id| Json::Str(id.clone()))
        };
        for s in &mut add {
            if let Some(id) = name(s.get("id")) {
                s.set("id", id);
            }
            if s.has("group")
                && let Some(g) = name(s.get("group"))
            {
                s.set("group", g);
            }
        }
    }
    OpResult::Done { add, remove, note }
}

fn text_of(v: &Json) -> String {
    match v {
        Json::Str(s) => s.clone(),
        other => kentos_geometry_core::api::json::to_string(other),
    }
}

/// Puts an operation's result into the list: new shapes where the bottom-most removed one was (`applyResult`).
pub fn apply_result(list: &[Obj], add: &[Obj], remove: &[Json]) -> Vec<Obj> {
    let gone: Vec<String> = remove.iter().map(text_of).collect();
    let is_gone = |id: &str| gone.iter().any(|g| g == id);
    let replaced: Vec<&Obj> = add.iter().filter(|s| is_gone(id_of(s))).collect();
    let replaced_of = |id: &str| replaced.iter().find(|s| id_of(s) == id).copied();
    let fresh: Vec<&Obj> = add
        .iter()
        .filter(|s| !is_gone(id_of(s)) || replaced_of(id_of(s)).is_none())
        .collect();
    let first_gone = list.iter().position(|s| is_gone(id_of(s)));
    let Some(first) = first_gone else {
        // Nothing removed (object to path): same-id shapes swap in place, the rest go on top.
        let mut out: Vec<Obj> = list
            .iter()
            .map(|s| {
                add.iter()
                    .find(|a| id_of(a) == id_of(s))
                    .cloned()
                    .unwrap_or_else(|| s.clone())
            })
            .collect();
        out.extend(
            add.iter()
                .filter(|a| !list.iter().any(|x| id_of(x) == id_of(a)))
                .cloned(),
        );
        return out;
    };
    let mut out = Vec::with_capacity(list.len() + add.len());
    for (i, s) in list.iter().enumerate() {
        if i == first {
            out.extend(
                fresh
                    .iter()
                    .filter(|f| replaced_of(id_of(f)).is_none())
                    .map(|f| (*f).clone()),
            );
        }
        if is_gone(id_of(s)) {
            if let Some(k) = replaced_of(id_of(s)) {
                out.push(k.clone());
            }
        } else {
            out.push(s.clone());
        }
    }
    out
}

/// The chosen shapes one step up or down past a neighbour, or to the top or bottom (`restack`).
pub fn restack(list: &[Obj], chosen: &[String], op: Restack) -> Vec<Obj> {
    let on = |s: &Obj| chosen.iter().any(|c| c == id_of(s));
    let sel: Vec<Obj> = list.iter().filter(|s| on(s)).cloned().collect();
    let rest: Vec<Obj> = list.iter().filter(|s| !on(s)).cloned().collect();
    match op {
        Restack::Top => return [rest, sel].concat(),
        Restack::Bottom => return [sel, rest].concat(),
        _ => {}
    }
    let mut out = list.to_vec();
    let n = out.len();
    let order: Vec<usize> = if op == Restack::Raise {
        (0..n).rev().collect()
    } else {
        (0..n).collect()
    };
    for i in order {
        let j = if op == Restack::Raise {
            i as isize + 1
        } else {
            i as isize - 1
        };
        if j >= 0 && (j as usize) < n && on(&out[i]) && !on(&out[j as usize]) {
            out.swap(i, j as usize);
        }
    }
    out
}

impl SvgEditor {
    /// One undo step under a label of its own, then everything redrawn.
    fn run(&mut self, label: &str, f: impl FnOnce(&mut SvgEditor)) {
        self.settle();
        self.edit(label, f);
        self.settle();
        self.touch();
    }

    // ── Path operations ──────────────────────────────────────────────────

    pub fn path_op(&mut self, op: PathOp) {
        let sel = self.chosen();
        if sel.is_empty() {
            self.warn("Önce şekil seçin.");
            return;
        }
        let extent = shapes_box(&sel)
            .ok()
            .flatten()
            .map_or(1.0, |b| (b.max_x - b.min_x).max(b.max_y - b.min_y));
        let mut ids = Ids::default();
        let r: Result<OpResult, String> = match op {
            op if op.boolean().is_some() => {
                boolean_shapes(op.boolean().unwrap_or("union"), &sel, &mut ids)
            }
            PathOp::Cut => cut_shapes(&sel, &mut ids),
            PathOp::Combine => combine_shapes(&sel, &mut ids),
            PathOp::BreakApart | PathOp::Split => {
                let mut add = Vec::new();
                let mut remove = Vec::new();
                let mut first_error = None;
                for s in &sel {
                    match break_apart(s, op == PathOp::Split, &mut ids) {
                        Ok(OpResult::Done {
                            add: a, remove: r, ..
                        }) => {
                            add.extend(a);
                            remove.extend(r);
                        }
                        Ok(OpResult::Error(e)) | Err(e) => {
                            first_error.get_or_insert(e);
                        }
                    }
                }
                if add.is_empty() && remove.is_empty() {
                    Ok(OpResult::Error(first_error.unwrap_or_default()))
                } else {
                    Ok(OpResult::Done {
                        add,
                        remove,
                        note: None,
                    })
                }
            }
            PathOp::ToPath => Ok(shapes_to_path(&sel)),
            PathOp::StrokeToPath => stroke_to_path(&sel, &mut ids),
            PathOp::Inset | PathOp::Outset => offset_shapes(
                &sel,
                if op == PathOp::Inset {
                    -self.ui.offset
                } else {
                    self.ui.offset
                },
                self.ui.offset_join,
            ),
            PathOp::Simplify => simplify_shapes(&sel, extent * self.ui.simplify / 100.0),
            PathOp::Reverse => reverse_shapes(&sel),
            PathOp::Close => close_shapes(&sel),
            PathOp::Open => open_shapes(&sel),
            _ => Ok(OpResult::Error(String::new())),
        };
        let r = match r {
            Ok(r) => with_ids(r, &ids),
            Err(e) => OpResult::Error(e),
        };
        let (add, remove, note) = match r {
            OpResult::Error(e) => {
                self.warn(e);
                return;
            }
            OpResult::Done { add, remove, note } => (add, remove, note),
        };
        let label = op.label();
        self.run(label, |ed| {
            ed.doc.shapes = apply_result(&ed.doc.shapes, &add, &remove);
            if let Some(n) = &ed.node_edit
                && remove.iter().any(|r| text_of(r) == *n)
            {
                ed.edit_nodes(None);
            }
        });
        self.select(add.iter().map(|s| id_of(s).to_owned()).collect());
        self.say(match note {
            Some(n) => format!("{label}: {n}"),
            None => format!("{label}."),
        });
    }

    // ── Nodes ────────────────────────────────────────────────────────────

    fn nodes_op(
        &mut self,
        label: &str,
        need: usize,
        f: impl FnOnce(&[SubPath], &[NodeRef]) -> Result<(Vec<SubPath>, Option<Vec<NodeRef>>), String>,
    ) {
        let Some(subs) = self.node_shape().and_then(|s| s.subs().ok()) else {
            self.warn("Önce bir yolun düğümlerini düzenlemeye başlayın (Düğüm aracı, A).");
            return;
        };
        let refs = self.node_refs();
        if refs.len() < need {
            self.warn(if need > 1 {
                format!(
                    "Bu işlem için en az {need} düğüm seçin (Shift ile ekleyin ya da kutu çizin)."
                )
            } else {
                "Önce düğüm seçin: düğüme tıklayın ya da boşlukta sürükleyip kutu çizin.".to_owned()
            });
            return;
        }
        match f(&subs, &refs) {
            Err(e) => self.warn(e),
            Ok((subs, chosen)) => {
                let chosen = chosen
                    .unwrap_or(refs)
                    .iter()
                    .filter_map(|r| {
                        Some((
                            kentos_svg_core::nodes::at(r.sub)?,
                            kentos_svg_core::nodes::at(r.index)?,
                        ))
                    })
                    .collect();
                self.node_apply(label, subs, chosen);
                self.say(format!("{label}."));
            }
        }
    }

    pub fn node_cmd(&mut self, cmd: NodeCmd) {
        let joined = |j: Joined| match j {
            Joined::Done(subs, refs) => Ok((subs, Some(refs))),
            Joined::Error(e) => Err(e),
        };
        match cmd {
            NodeCmd::Type(t) => self.nodes_op(
                &format!("Düğüm türü: {}", node_type_name(t)),
                1,
                |subs, refs| Ok((set_node_type(subs, refs, t), None)),
            ),
            NodeCmd::Insert => self.nodes_op("Düğüm ekle", 2, |subs, refs| {
                let (subs, out) = insert_mid_nodes(subs, refs);
                if out.len() > refs.len() {
                    Ok((subs, Some(out)))
                } else {
                    Err("Araya düğüm eklemek için bir parçanın iki ucunu seçin.".to_owned())
                }
            }),
            NodeCmd::Delete { keep_shape } => self.nodes_op("Düğümü sil", 1, |subs, refs| {
                Ok((delete_nodes(subs, refs, keep_shape), Some(Vec::new())))
            }),
            NodeCmd::Join { merge } => self.nodes_op(
                if merge {
                    "Uçları birleştir"
                } else {
                    "Uçları parçayla birleştir"
                },
                2,
                |subs, refs| joined(join_ends(subs, refs, merge)),
            ),
            NodeCmd::Break => self.nodes_op("Düğümde kır", 1, |subs, refs| {
                Ok((break_at_nodes(subs, refs), Some(Vec::new())))
            }),
            NodeCmd::DeleteSegment => self.nodes_op("Parçayı sil", 2, |subs, refs| {
                delete_segments(subs, refs).map(|s| (s, Some(Vec::new())))
            }),
            NodeCmd::Segments { line } => self.nodes_op(
                if line {
                    "Parçalar düz"
                } else {
                    "Parçalar eğri"
                },
                2,
                |subs, refs| Ok((segments_to(subs, refs, line), None)),
            ),
            NodeCmd::Corner(mode) => {
                let size = self.ui.corner;
                let fillet = mode == CornerMode::Fillet;
                let label = if fillet {
                    format!(
                        "Köşe yuvarla (R {})",
                        kentos_expression::js::number::to_string(size)
                    )
                } else {
                    format!(
                        "Pah kır ({})",
                        kentos_expression::js::number::to_string(size)
                    )
                };
                self.nodes_op(&label, 1, |subs, refs| {
                    joined(corner_nodes(subs, refs, fillet, size))
                })
            }
            NodeCmd::Align { axis, to } => self.nodes_op("Düğümleri hizala", 2, |subs, refs| {
                Ok((align_nodes(subs, refs, axis, to), None))
            }),
            NodeCmd::Distribute { axis } => {
                self.nodes_op("Düğümleri dağıt", 3, |subs, refs| {
                    Ok((distribute_nodes(subs, refs, axis), None))
                })
            }
        }
    }

    /// Arrow keys move the chosen nodes.
    pub fn node_nudge(&mut self, dx: f64, dy: f64) -> bool {
        if self.node_shape().is_none() || self.node_selected().is_empty() {
            return false;
        }
        self.nodes_op("Düğümü taşı", 1, |subs, refs| {
            let on = |si: usize, ni: usize| refs.iter().any(|r| *r == node_ref((si, ni)));
            let moved = subs
                .iter()
                .enumerate()
                .map(|(si, sp)| SubPath {
                    closed: sp.closed,
                    nodes: sp
                        .nodes
                        .iter()
                        .enumerate()
                        .map(|(ni, n)| {
                            if !on(si, ni) {
                                return n.clone();
                            }
                            let mut m = n.clone();
                            m.x += dx;
                            m.y += dy;
                            m.in_ = n.in_.map(|h| [h[0] + dx, h[1] + dy]);
                            m.out = n.out.map(|h| [h[0] + dx, h[1] + dy]);
                            m
                        })
                        .collect(),
                })
                .collect();
            Ok((moved, None))
        });
        true
    }

    pub fn select_all_nodes(&mut self) {
        let Some(subs) = self.node_shape().and_then(|s| s.subs().ok()) else {
            return;
        };
        let all = subs
            .iter()
            .enumerate()
            .flat_map(|(si, sp)| (0..sp.nodes.len()).map(move |ni| (si, ni)))
            .collect();
        self.set_node_selected(all);
    }

    // ── Arranging ────────────────────────────────────────────────────────

    /// Matrices per unit applied to the unit's shapes.
    fn move_units(&mut self, label: &str, ms: &[Matrix], units: &[kentos_svg_core::arrange::Unit]) {
        let mut by_id: Vec<(String, Matrix)> = Vec::new();
        for (u, m) in units.iter().zip(ms) {
            for id in &u.ids {
                by_id.push((id.clone(), *m));
            }
        }
        self.run(label, |ed| {
            for s in &mut ed.doc.shapes {
                if let Some((_, m)) = by_id.iter().find(|(id, _)| id == id_of(s))
                    && let Ok(Some(t)) = transform_shape(s, m)
                {
                    *s = t;
                }
            }
        });
    }

    fn units(&mut self) -> Option<Vec<kentos_svg_core::arrange::Unit>> {
        let units = units_of(&self.doc.shapes, &self.selection).ok()?;
        let locked = units.iter().any(|u| {
            u.ids
                .iter()
                .any(|id| self.doc.shape(id).is_some_and(|s| s.is("locked")))
        });
        if locked {
            self.warn(
                "Seçimde kilitli şekil var: önce kilidini açın (şekil listesinde kilit düğmesi).",
            );
            return None;
        }
        Some(units)
    }

    pub fn align(&mut self, side: &'static str) {
        if self.selection.is_empty() {
            self.warn("Hizalamak için şekil seçin.");
            return;
        }
        let Some(units) = self.units() else {
            return;
        };
        let one = units.len() < 2;
        let ms = align_moves(
            &units,
            side,
            self.ui.align_to,
            self.doc.width,
            self.doc.height,
            self.ui.align_as_one,
        );
        self.move_units("Hizala", &ms, &units);
        self.say(if one && self.ui.align_to != "canvas" {
            "Tek şekil tuvale hizalandı."
        } else {
            "Hizalandı."
        });
    }

    pub fn distribute(&mut self, how: &'static str) {
        let n = units_of(&self.doc.shapes, &self.selection).map_or(0, |u| u.len());
        if n < 3 {
            self.warn("Dağıtmak için en az üç şekil (ya da grup) seçin.");
            return;
        }
        let Some(units) = self.units() else {
            return;
        };
        let ms = distribute_moves(&units, how);
        self.move_units("Dağıt", &ms, &units);
    }

    /// The Dönüştür tab's transform as the core reads it (`specOf`).
    pub fn transform_spec(&self) -> TransformSpec {
        let t = &self.ui.transform;
        match t.kind {
            TransformKind::Move => TransformSpec::Move {
                x: t.x,
                y: t.y,
                relative: t.relative,
            },
            TransformKind::Scale => TransformSpec::Scale {
                sx: t.sx,
                sy: t.sy,
                anchor: t.anchor.to_owned(),
            },
            TransformKind::Rotate => TransformSpec::Rotate {
                deg: t.deg,
                ccw: t.ccw,
                about: match t.about {
                    Some(a) => About::Anchor(a.to_owned()),
                    None => About::Point(t.point.unwrap_or([0.0, 0.0])),
                },
            },
            TransformKind::Skew => TransformSpec::Skew {
                ax: t.ax,
                ay: t.ay,
                anchor: t.anchor.to_owned(),
            },
            TransformKind::Matrix => TransformSpec::Matrix { m: t.m },
        }
    }

    pub fn apply_transform(&mut self) {
        if self.selection.is_empty() {
            self.warn("Dönüştürmek için şekil seçin.");
            return;
        }
        let Ok(units) = units_of(&self.doc.shapes, &self.selection) else {
            return;
        };
        let spec = self.transform_spec();
        let ms = transform_moves(&units, &spec, self.ui.transform.separately);
        if !ms.iter().all(invertible) {
            self.warn("Bu dönüşüm şekilleri bir çizgiye ezer: ölçek ve matris sıfır olmamalı.");
            return;
        }
        let Some(units) = self.units() else {
            return;
        };
        let label = match self.ui.transform.kind {
            TransformKind::Move => "Taşı",
            TransformKind::Scale => "Ölçekle",
            TransformKind::Rotate => "Döndür",
            TransformKind::Skew => "Eğ",
            TransformKind::Matrix => "Matris",
        };
        self.move_units(label, &ms, &units);
    }

    pub fn restack(&mut self, op: Restack) {
        if self.selection.is_empty() {
            self.warn("Sırasını değiştirmek için şekil seçin.");
            return;
        }
        let chosen = self.selection.clone();
        self.run("Sıra", |ed| {
            ed.doc.shapes = restack(&ed.doc.shapes, &chosen, op)
        });
    }

    // ── Arrays ───────────────────────────────────────────────────────────

    pub fn apply_array(&mut self) {
        let sel = self.chosen();
        if sel.is_empty() {
            self.warn("Çoğaltmak için şekil seçin.");
            return;
        }
        let Some(ms) = array_matrices(self) else {
            self.warn("Merkez noktası seçilmedi: “Tuvalde göster” ile tıklayın.");
            return;
        };
        if ms.is_empty() {
            self.warn("Kopya çıkmıyor: sayıları artırın.");
            return;
        }
        let mut ids = Ids::default();
        let Ok(copies) = copies_of(&sel, &ms, &mut ids) else {
            return;
        };
        let OpResult::Done { add: copies, .. } = with_ids(
            OpResult::Done {
                add: copies,
                remove: Vec::new(),
                note: None,
            },
            &ids,
        ) else {
            return;
        };
        let label = if self.ui.array.kind == ArrayKind::Mirror {
            "Aynalı kopya"
        } else {
            "Dizi"
        };
        let n = copies.len();
        self.run(label, |ed| ed.doc.shapes.extend(copies));
        self.say(format!("{label}: {n} şekil eklendi."));
    }

    // ── Selection helpers ────────────────────────────────────────────────

    pub fn select_all(&mut self) {
        let ids = self
            .doc
            .shapes
            .iter()
            .filter(|s| !s.is("hidden") && !s.is("locked"))
            .map(|s| id_of(s).to_owned())
            .collect();
        self.select(ids);
    }

    pub fn invert_selection(&mut self) {
        let ids = self
            .doc
            .shapes
            .iter()
            .filter(|s| !s.is("hidden") && !s.is("locked") && !self.is_selected(id_of(s)))
            .map(|s| id_of(s).to_owned())
            .collect();
        self.select(ids);
    }

    /// Shapes painted like the chosen ones: same fill, same stroke, both, or the same kind.
    pub fn select_same(&mut self, what: Same) {
        let sel = self.chosen();
        if sel.is_empty() {
            self.warn("Benzerini seçmek için önce bir şekil seçin.");
            return;
        }
        let key = |s: &Obj| -> String {
            let paint = |k: &str| s.text(k).unwrap_or("").to_owned();
            let width = kentos_expression::js::number::to_string(s.num("strokeWidth"));
            match what {
                Same::Fill => paint("fill"),
                Same::Stroke => format!(
                    "{}|{}",
                    paint("stroke"),
                    if paint("stroke") == "none" {
                        String::new()
                    } else {
                        width
                    }
                ),
                Same::Kind => s.kind().to_owned(),
                Same::Both => format!("{}|{}|{}", paint("fill"), paint("stroke"), width),
            }
        };
        let want: Vec<String> = sel.iter().map(key).collect();
        let ids: Vec<String> = self
            .doc
            .shapes
            .iter()
            .filter(|s| !s.is("hidden") && !s.is("locked") && want.contains(&key(s)))
            .map(|s| id_of(s).to_owned())
            .collect();
        let n = ids.len();
        self.select(ids);
        self.say(format!("{n} şekil seçildi."));
    }

    // ── The editor's own actions (`action`) ──────────────────────────────

    pub fn action(&mut self, name: Action) {
        let sel = self.chosen();
        if sel.is_empty() {
            return;
        }
        let ids = self.selection.clone();
        let on = |s: &Obj| ids.iter().any(|i| i == id_of(s));
        match name {
            Action::Delete => self.edit("act:delete", |ed| {
                // A locked shape stays (unlock it in the list first).
                ed.doc.shapes.retain(|s| !on(s) || s.is("locked"));
                ed.selection.clear();
                ed.node_edit = None;
            }),
            Action::Duplicate => {
                let step = if self.options.grid > 0.0 {
                    self.options.grid
                } else {
                    2.0
                };
                let mut groups: Vec<(String, String)> = Vec::new();
                let copies: Vec<Obj> = sel
                    .iter()
                    .filter_map(|s| {
                        let mut c = transform_shape(s, &translate(step, step)).ok().flatten()?;
                        c.set("id", Json::Str(shape_id()));
                        let g = s.text("group").filter(|g| !g.is_empty()).map(|g| {
                            match groups.iter().find(|(k, _)| k == g) {
                                Some((_, v)) => v.clone(),
                                None => {
                                    let v = shape_id();
                                    groups.push((g.to_owned(), v.clone()));
                                    v
                                }
                            }
                        });
                        c.put("group", g.map(Json::Str));
                        Some(c)
                    })
                    .collect();
                let new_ids: Vec<String> = copies.iter().map(|c| id_of(c).to_owned()).collect();
                self.edit("act:duplicate", |ed| {
                    ed.doc.shapes.extend(copies);
                    ed.selection = new_ids;
                });
            }
            Action::Group => {
                let g = shape_id();
                self.edit("act:group", |ed| {
                    for s in &mut ed.doc.shapes {
                        if on(s) {
                            s.set_text("group", &g);
                        }
                    }
                    // Group members sit together in the stack (the file writes them in one <g>).
                    let at = ed.doc.shapes.iter().position(on).unwrap_or(0);
                    let members: Vec<Obj> =
                        ed.doc.shapes.iter().filter(|s| on(s)).cloned().collect();
                    let mut rest: Vec<Obj> =
                        ed.doc.shapes.iter().filter(|s| !on(s)).cloned().collect();
                    let at = at.min(rest.len());
                    rest.splice(at..at, members);
                    ed.doc.shapes = rest;
                });
            }
            Action::Ungroup => self.edit("act:ungroup", |ed| {
                for s in &mut ed.doc.shapes {
                    if on(s) {
                        s.set_undefined("group");
                    }
                }
            }),
            Action::FlipH | Action::FlipV | Action::Rot90 => {
                let Some(all) = shapes_box(&sel).ok().flatten() else {
                    return;
                };
                let cx = (all.min_x + all.max_x) / 2.0;
                let cy = (all.min_y + all.max_y) / 2.0;
                let m: Matrix = match name {
                    Action::FlipH => [-1.0, 0.0, 0.0, 1.0, 2.0 * cx, 0.0],
                    Action::FlipV => [1.0, 0.0, 0.0, -1.0, 0.0, 2.0 * cy],
                    _ => rotate_about(90.0, cx, cy),
                };
                let key = match name {
                    Action::FlipH => "act:flipH",
                    Action::FlipV => "act:flipV",
                    _ => "act:rot90",
                };
                self.edit(key, |ed| {
                    for s in &mut ed.doc.shapes {
                        if on(s)
                            && !s.is("locked")
                            && let Ok(Some(t)) = transform_shape(s, &m)
                        {
                            *s = t;
                        }
                    }
                });
            }
        }
        self.touch();
    }

    /// The box's X, Y, width or height typed (`boxFields`): the selection moves or scales to it.
    pub fn set_box(&mut self, key: &'static str, v: f64) {
        let sel = self.chosen();
        let Some(b) = shapes_box(&sel).ok().flatten() else {
            return;
        };
        let (w0, h0) = (b.max_x - b.min_x, b.max_y - b.min_y);
        let (mut x, mut y, mut w, mut h) = (b.min_x, b.min_y, w0, h0);
        match key {
            "x" => x = v,
            "y" => y = v,
            "w" => {
                if self.ui.box_lock && w0 > 0.0 {
                    h = h0 * v / w0;
                }
                w = v;
            }
            _ => {
                if self.ui.box_lock && h0 > 0.0 {
                    w = w0 * v / h0;
                }
                h = v;
            }
        }
        let m = if key == "x" || key == "y" {
            translate(x - b.min_x, y - b.min_y)
        } else {
            let from = kentos_geometry_core::geometry::Bounds {
                min_x: b.min_x,
                min_y: b.min_y,
                max_x: b.min_x + w0.max(1e-9),
                max_y: b.min_y + h0.max(1e-9),
            };
            let to = kentos_geometry_core::geometry::Bounds {
                min_x: x,
                min_y: y,
                max_x: x + w.max(1e-6),
                max_y: y + h.max(1e-6),
            };
            kentos_svg_core::model::box_to_box(&from, &to)
        };
        let ids = self.selection.clone();
        self.edit(&format!("box-{key}"), |ed| {
            for s in &mut ed.doc.shapes {
                if ids.iter().any(|i| i == id_of(s))
                    && !s.is("locked")
                    && let Ok(Some(t)) = transform_shape(s, &m)
                {
                    *s = t;
                }
            }
        });
    }

    /// The box's ↺ and ↻ by the typed angle.
    pub fn turn_box(&mut self, ccw: bool) {
        let sel = self.chosen();
        let Some(b) = shapes_box(&sel).ok().flatten() else {
            return;
        };
        let deg = self.ui.turn;
        let m = rotate_about(
            if ccw { -deg } else { deg },
            (b.min_x + b.max_x) / 2.0,
            (b.min_y + b.max_y) / 2.0,
        );
        let ids = self.selection.clone();
        self.run("Döndür", |ed| {
            for s in &mut ed.doc.shapes {
                if ids.iter().any(|i| i == id_of(s))
                    && !s.is("locked")
                    && let Ok(Some(t)) = transform_shape(s, &m)
                {
                    *s = t;
                }
            }
        });
    }

    /// The selection's shapes changed one by one (a paint, a field of every chosen shape).
    pub fn set_chosen(&mut self, key: &str, f: impl Fn(&mut Obj)) {
        let ids = self.selection.clone();
        self.edit(key, |ed| {
            for s in &mut ed.doc.shapes {
                if ids.iter().any(|i| i == id_of(s)) {
                    f(s);
                }
            }
        });
    }

    /// One shape replaced (the geometry fields of a single selection).
    pub fn set_shape(&mut self, key: &str, id: &str, f: impl FnOnce(&mut Obj)) {
        self.edit(key, |ed| {
            if let Some(s) = ed.doc.shapes.iter_mut().find(|s| id_of(s) == id) {
                f(s);
            }
        });
    }
}
