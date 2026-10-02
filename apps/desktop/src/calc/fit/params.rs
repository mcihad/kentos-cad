//! Parametrelerle (docs/adr/0156 §7; the web's `FitDialog.ts`): the
//! transform given by its numbers instead of control points: a base point,
//! the Y (east) and X (north) scales, a counter-clockwise turn and a shift,
//! an affine about the base whose linear part is the core's `scale_turn`.
//! An empty scale counts as 1, an empty turn or shift as 0; a scale cannot
//! be 0 (below 0 it mirrors). With equal scales the transform is a
//! similarity and every kind keeps its shape.

use std::fmt;

use kentos_contracts::Transform;
use kentos_domain::Document as Model;
use kentos_geometry_core::ops::fit::scale_turn;
use kentos_interaction::{Format, Vec2};
use kentos_processing::text::js_number;

use super::TITLE;
use crate::calc::read::{Known, read_number, resolve_point};
use crate::exchange::words::Kind as Line;

/// How the transform is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Points,
    Parameters,
}

impl Method {
    pub const ALL: [Method; 2] = [Method::Points, Method::Parameters];

    pub fn hint(self) -> &'static str {
        match self {
            Method::Points => "Ortak noktalardan en küçük kareler ile; artıklar ve m0 görünür.",
            Method::Parameters => {
                "Taban noktasına göre Y ve X ölçeği, dönüklük ve öteleme; Netcad'in XY Yönünde Ölçekle'si gibi."
            }
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Method::Points => "Kontrol noktaları",
            Method::Parameters => "Parametrelerle",
        })
    }
}

/// Parametrelerle's numbers, in their fields' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Param {
    ScaleY,
    ScaleX,
    Rotation,
    ShiftY,
    ShiftX,
}

impl Param {
    pub const ALL: [Param; 5] = [
        Param::ScaleY,
        Param::ScaleX,
        Param::Rotation,
        Param::ShiftY,
        Param::ShiftX,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Param::ScaleY => "Y ölçeği",
            Param::ScaleX => "X ölçeği",
            Param::Rotation => "Dönüklük",
            Param::ShiftY => "Öteleme ΔY",
            Param::ShiftX => "Öteleme ΔX",
        }
    }

    fn scale(self) -> bool {
        matches!(self, Param::ScaleY | Param::ScaleX)
    }

    /// What an empty field counts as.
    pub fn empty(self) -> &'static str {
        if self.scale() { "1" } else { "0" }
    }
}

/// What is typed, kept while the app runs: the base point (a name or Y,X)
/// and the numbers as typed.
#[derive(Clone, Debug)]
pub struct Typed {
    pub base: String,
    values: [String; 5],
}

impl Default for Typed {
    fn default() -> Self {
        Self {
            base: String::new(),
            values: Param::ALL.map(|p| p.empty().to_owned()),
        }
    }
}

/// Parametrelerle read: the base, the scales, the turn (radians) and the shift.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub base: Vec2,
    pub scale_y: f64,
    pub scale_x: f64,
    pub rotation: f64,
    pub shift: Vec2,
}

impl Typed {
    pub fn get(&self, p: Param) -> &str {
        &self.values[p as usize]
    }

    pub fn set(&mut self, p: Param, text: String) {
        self.values[p as usize] = text;
    }

    /// The numbers read and the base resolved, the turn in radians; or what
    /// is wrong, in the fields' order (the web's `readParams`).
    pub fn read(&self, model: &Model, format: &Format) -> Result<Params, Vec<(Line, String)>> {
        let mut problems = Vec::new();
        let base = match resolve_point(model, &self.base) {
            Known::Empty => {
                problems.push((
                    Line::Info,
                    "Taban noktasını yazın ya da çizimden seçin.".to_owned(),
                ));
                None
            }
            Known::Error(e) => {
                problems.push((Line::Warn, format!("Taban noktası: {e}")));
                None
            }
            Known::Point { p, .. } => Some(p),
        };
        let mut values = [1.0, 1.0, 0.0, 0.0, 0.0];
        for p in Param::ALL {
            match read_number(self.get(p)) {
                Some(v) if !v.is_finite() => {
                    problems.push((Line::Warn, format!("{} sayı olmalı.", p.label())));
                }
                Some(v) if p.scale() && v == 0.0 => problems.push((
                    Line::Warn,
                    format!("{} sıfır olamaz; eksi ölçek aynalar.", p.label()),
                )),
                Some(v) => values[p as usize] = v,
                None => values[p as usize] = if p.scale() { 1.0 } else { 0.0 },
            }
        }
        match base {
            Some(base) if problems.is_empty() => Ok(Params {
                base,
                scale_y: values[Param::ScaleY as usize],
                scale_x: values[Param::ScaleX as usize],
                rotation: format.angle_from_typed(values[Param::Rotation as usize]),
                shift: Vec2::new(
                    values[Param::ShiftY as usize],
                    values[Param::ShiftX as usize],
                ),
            }),
            _ => Err(problems),
        }
    }
}

impl Params {
    /// Where the base lands.
    fn to(&self) -> Vec2 {
        Vec2::new(self.base.x + self.shift.x, self.base.y + self.shift.y)
    }

    /// `cad.entities.transform`'s affine about the base (its centred form).
    pub fn transform(&self) -> Transform {
        let to = self.to();
        Transform::Affine {
            from: kentos_contracts::Vec2 {
                x: self.base.x,
                y: self.base.y,
            },
            to: kentos_contracts::Vec2 { x: to.x, y: to.y },
            m: scale_turn(self.scale_y, self.scale_x, self.rotation),
        }
    }

    /// The summary (the web's `solveParams`): the numbers, where the base
    /// lands, and what the objects become.
    pub fn summary(&self, format: &Format) -> Vec<(Line, String)> {
        let mirrored = if self.scale_y * self.scale_x < 0.0 {
            " Ölçeklerden biri eksi: nesneler aynalanır."
        } else {
            ""
        };
        vec![
            (
                Line::Ok,
                format!(
                    "Y ölçeği {}, X ölçeği {}, dönüklük {}, öteleme ΔY {}, ΔX {}.",
                    js_number(self.scale_y),
                    js_number(self.scale_x),
                    format.angle(self.rotation),
                    format.length(self.shift.x),
                    format.length(self.shift.y)
                ),
            ),
            (
                Line::Info,
                format!(
                    "Taban noktası {} → {}.",
                    format.point(self.base),
                    format.point(self.to())
                ),
            ),
            (
                Line::Info,
                if self.scale_y == self.scale_x {
                    "Ölçekler eşit: benzerlik; her nesne biçimini korur.".to_owned()
                } else {
                    format!(
                        "Ölçekler farklı: afin dönüşüm; daireler ve yaylar elips olur, yazılar, bloklar ve ölçüler yerinde biçimini korur.{mirrored}"
                    )
                },
            ),
        ]
    }

    /// The report (the web's `reportParams`): the base, the numbers as read
    /// and the linear part, tab-separated.
    pub fn report(&self, typed: &Typed, format: &Format) -> Vec<Vec<String>> {
        let line = |cells: &[&str]| cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>();
        let mut numbers = vec!["Merkezli sayılar".to_owned()];
        numbers.extend(
            scale_turn(self.scale_y, self.scale_x, self.rotation)
                .iter()
                .map(|v| js_number(*v)),
        );
        vec![
            line(&[TITLE, "Parametrelerle"]),
            vec![
                "Taban Y".to_owned(),
                js_number(self.base.x),
                "Taban X".to_owned(),
                js_number(self.base.y),
            ],
            vec!["Y ölçeği".to_owned(), js_number(self.scale_y)],
            vec!["X ölçeği".to_owned(), js_number(self.scale_x)],
            vec![
                format!("Dönüklük ({})", format.angle_unit_label()),
                js_number(read_number(typed.get(Param::Rotation)).unwrap_or(0.0)),
            ],
            vec!["Öteleme ΔY (m)".to_owned(), js_number(self.shift.x)],
            vec!["Öteleme ΔX (m)".to_owned(), js_number(self.shift.y)],
            numbers,
        ]
    }
}
