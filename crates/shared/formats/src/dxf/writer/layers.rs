//! The drawing's layers as DXF layers. KentOS layers live in a tree and
//! their names may repeat in different groups; DXF layers are flat, their
//! names ignore case and exclude a few characters. Each layer keeps its
//! own name when it can; a name that repeats takes its groups in front
//! ("Kadastro - Sınır"), a character DXF refuses becomes "_", and every
//! change is reported. Line types are written with their dashes sized for
//! paper at the plot scale; line weights as AutoCAD's nearest weight.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{DxfWriteLayer, LineType};

use super::super::aci::{self, DxfColor};
use super::super::xdata::{self, Meta};
use super::Out;
use super::template::{self, LAYER_TABLE, LAYER_ZERO, LTYPE_TABLE, MATERIAL, PLOT_STYLE};
use crate::report::Report;

/// AutoCAD's line weights (hundredths of a millimetre); no other value is valid.
const LINE_WEIGHTS: [i64; 24] = [
    0, 5, 9, 13, 15, 18, 20, 25, 30, 35, 40, 50, 53, 60, 70, 80, 90, 100, 106, 120, 140, 158, 200,
    211,
];

/// Characters AutoCAD refuses in a symbol name.
const REFUSED: &[char] = &[
    '<', '>', '/', '\\', '"', ':', ';', '?', '*', '|', '=', '`', '\'',
];

/// A dashed line type: DXF name, description and pattern in paper millimetres
/// (positive dash, negative gap, zero a dot).
struct Dashes {
    name: &'static str,
    description: &'static str,
    mm: &'static [f64],
}

fn dashes(t: LineType) -> Option<Dashes> {
    match t {
        LineType::Continuous => None,
        LineType::Dashed => Some(Dashes {
            name: "DASHED",
            description: "Kesikli __ __ __",
            mm: &[2.5, -1.25],
        }),
        LineType::Dashdot => Some(Dashes {
            name: "DASHDOT",
            description: "Noktalı kesik __ . __ .",
            mm: &[4.0, -1.0, 0.0, -1.0],
        }),
        LineType::Dotted => Some(Dashes {
            name: "DOT",
            description: "Noktalı . . . .",
            mm: &[0.0, -1.0],
        }),
    }
}

pub(super) fn ltype_name(t: LineType) -> &'static str {
    dashes(t).map_or("Continuous", |d| d.name)
}

/// The AutoCAD weight nearest to `mm`, and whether it differs.
pub(super) fn line_weight(mm: f64) -> (i64, bool) {
    if !(mm >= 0.0) || !mm.is_finite() {
        return (-3, true);
    }
    let x = mm * 100.0;
    let best = LINE_WEIGHTS
        .iter()
        .copied()
        .min_by(|a, b| (*a as f64 - x).abs().total_cmp(&(*b as f64 - x).abs()))
        .unwrap_or(25);
    (best, (best as f64 - x).abs() > 1e-6)
}

/// A name DXF accepts: refused and control characters as "_", no outer spaces, at most 255 characters.
fn valid_name(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || REFUSED.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .take(255)
        .collect();
    if s.is_empty() {
        "Katman".to_string()
    } else {
        s
    }
}

fn key(name: &str) -> String {
    name.to_uppercase()
}

pub(super) struct Layer {
    pub name: String,
    pub color: DxfColor,
    /// The app's colour when DXF does not read back as it (kept in KENTOS data).
    pub app_color: Option<String>,
    pub visible: bool,
    pub locked: bool,
    pub line_type: LineType,
    pub weight: i64,
}

pub(super) struct Layers {
    pub list: Vec<Layer>,
    by_id: HashMap<String, usize>,
    /// The layer written as DXF layer "0", if the drawing has one of that name.
    zero: Option<usize>,
}

impl Layers {
    pub fn new(input: &[DxfWriteLayer], report: &mut Report) -> Layers {
        let bases: Vec<String> = input.iter().map(|l| valid_name(&l.name)).collect();
        let mut repeats: HashMap<String, usize> = HashMap::new();
        for b in &bases {
            *repeats.entry(key(b)).or_insert(0) += 1;
        }
        let mut taken: HashSet<String> = HashSet::new();
        let mut list = Vec::with_capacity(input.len());
        let mut by_id = HashMap::new();
        let mut zero = None;
        for (i, l) in input.iter().enumerate() {
            let base = &bases[i];
            let unique = repeats.get(&key(base)).copied().unwrap_or(0) == 1;
            let name = if (unique || base == "0") && !taken.contains(&key(base)) {
                base.clone()
            } else {
                // The groups in front, innermost first, until the name is free; then a number.
                let mut name = base.clone();
                let mut found = false;
                for g in l.path.iter().rev() {
                    name = format!("{} - {name}", valid_name(g));
                    if !taken.contains(&key(&name)) && !repeats.contains_key(&key(&name)) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    let stem = name.clone();
                    let mut k = 2;
                    while taken.contains(&key(&name)) || repeats.contains_key(&key(&name)) {
                        name = format!("{stem} ({k})");
                        k += 1;
                    }
                }
                name
            };
            if name != l.name {
                let path: Vec<&str> = l
                    .path
                    .iter()
                    .map(String::as_str)
                    .chain([l.name.as_str()])
                    .collect();
                let mut why: Vec<&str> = Vec::new();
                if l.name
                    .chars()
                    .any(|c| c.is_control() || REFUSED.contains(&c))
                {
                    why.push("DXF'in kabul etmediği karakterler “_” oldu");
                }
                if l.name.trim() != l.name
                    || l.name.trim().is_empty()
                    || l.name.trim().chars().count() > 255
                {
                    why.push("boşlukları ya da uzunluğu düzeltildi");
                }
                if name != *base {
                    why.push("aynı adlı başka bir katman var; DXF katmanları ağaçsızdır ve adları büyük/küçük harf ayırmaz");
                }
                report.note(
                    "Katman adı",
                    &format!(
                        "“{}” katmanı “{name}” adıyla yazıldı ({})",
                        path.join(" / "),
                        why.join("; ")
                    ),
                    0,
                );
            }
            taken.insert(key(&name));
            if name == "0" && zero.is_none() {
                zero = Some(list.len());
            }
            let (color, note) = aci::from_app(&l.color);
            if let Some(n) = note {
                report.note("Katman rengi", &format!("“{name}”: {n}"), 0);
            }
            let (weight, moved) = line_weight(l.line_weight);
            if moved {
                report.note(
                    "Çizgi kalınlığı",
                    &format!(
                        "“{name}”: {} mm, AutoCAD'in en yakın kalınlığı {} mm olarak yazıldı",
                        l.line_weight,
                        weight as f64 / 100.0
                    ),
                    0,
                );
            }
            by_id.entry(l.id.clone()).or_insert(list.len());
            list.push(Layer {
                app_color: (color.read_back() != l.color).then(|| l.color.clone()),
                name,
                color,
                visible: l.visible,
                locked: l.locked,
                line_type: l.line_type,
                weight,
            });
        }
        Layers { list, by_id, zero }
    }

    /// The DXF layer name of an object's `layerId`; None when the drawing has no such layer.
    pub fn name_of(&self, id: &str) -> Option<&str> {
        self.by_id.get(id).map(|&i| self.list[i].name.as_str())
    }

    /// LTYPE: the built-in three and the dashed ones the layers use, sized for paper at `scale`.
    pub fn ltype_table(&self, out: &mut Out, handles: &mut super::Handles, scale: f64) {
        let mut used: Vec<LineType> = Vec::new();
        for l in &self.list {
            if dashes(l.line_type).is_some() && !used.contains(&l.line_type) {
                used.push(l.line_type);
            }
        }
        template::table(out, "LTYPE", LTYPE_TABLE, 3 + used.len());
        template::builtin_ltypes(out);
        for t in used {
            let Some(d) = dashes(t) else { continue };
            let metres: Vec<f64> = d.mm.iter().map(|m| m * scale / 1000.0).collect();
            template::record(
                out,
                "LTYPE",
                handles.take(),
                LTYPE_TABLE,
                "AcDbLinetypeTableRecord",
            );
            out.str(2, d.name);
            out.int(70, 0);
            out.str(3, d.description);
            out.int(72, 65);
            out.int(73, metres.len() as i64);
            out.real(40, metres.iter().map(|m| m.abs()).sum());
            for m in metres {
                out.real(49, m);
                out.int(74, 0);
            }
        }
        out.str(0, "ENDTAB");
    }

    /// LAYER: "0" first (the drawing's own "0" when it has one), then the layers in the drawing's order.
    pub fn layer_table(&self, out: &mut Out, handles: &mut super::Handles) {
        let count = self.list.len() + usize::from(self.zero.is_none());
        template::table(out, "LAYER", LAYER_TABLE, count);
        match self.zero {
            Some(i) => self.record(out, LAYER_ZERO, &self.list[i]),
            None => self.record(
                out,
                LAYER_ZERO,
                &Layer {
                    name: "0".into(),
                    color: DxfColor { aci: 7, rgb: None },
                    app_color: None,
                    visible: true,
                    locked: false,
                    line_type: LineType::Continuous,
                    weight: -3,
                },
            ),
        }
        for (i, l) in self.list.iter().enumerate() {
            if Some(i) != self.zero {
                self.record(out, handles.take(), l);
            }
        }
        out.str(0, "ENDTAB");
    }

    fn record(&self, out: &mut Out, handle: u64, l: &Layer) {
        template::record(out, "LAYER", handle, LAYER_TABLE, "AcDbLayerTableRecord");
        out.str(2, &l.name);
        out.int(70, if l.locked { 4 } else { 0 });
        let aci = i64::from(l.color.aci);
        out.int(62, if l.visible { aci } else { -aci });
        if let Some(rgb) = l.color.rgb {
            out.int(420, rgb);
        }
        out.str(6, ltype_name(l.line_type));
        out.int(370, l.weight);
        out.handle(390, PLOT_STYLE);
        out.handle(347, MATERIAL);
        out.xdata(&xdata::groups(&Meta {
            color: l.app_color.clone(),
            ..Meta::default()
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(id: &str, name: &str, path: &[&str]) -> DxfWriteLayer {
        DxfWriteLayer {
            id: id.into(),
            name: name.into(),
            path: path.iter().map(|s| s.to_string()).collect(),
            color: "ink".into(),
            visible: true,
            locked: false,
            line_type: LineType::Continuous,
            line_weight: 0.25,
        }
    }

    #[test]
    fn names_stay_unless_they_repeat_or_hold_what_dxf_refuses() {
        let mut report = Report::default();
        let l = Layers::new(
            &[
                layer("a", "Sınır", &["Kadastro"]),
                layer("b", "sınır", &["İmar"]),
                layer("c", "Yol ekseni", &["Ulaşım"]),
                layer("d", "a/b: c?", &[]),
                layer("e", "Sınır", &[]),
                layer("f", "0", &[]),
            ],
            &mut report,
        );
        let names: Vec<&str> = l.list.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Kadastro - Sınır",
                "İmar - sınır",
                "Yol ekseni",
                "a_b_ c_",
                "Sınır (2)",
                "0"
            ]
        );
        assert_eq!(l.name_of("c"), Some("Yol ekseni"));
        assert_eq!(l.name_of("x"), None);
        assert_eq!(l.zero, Some(5));
        let notes = report.export().notes;
        assert_eq!(notes.iter().filter(|n| n.what == "Katman adı").count(), 4);
        assert!(notes.iter().any(|n| n.reason.contains(
            "“a/b: c?” katmanı “a_b_ c_” adıyla yazıldı (DXF'in kabul etmediği karakterler"
        )));
    }

    #[test]
    fn line_weights_snap_to_autocad_weights() {
        assert_eq!(line_weight(0.35), (35, false));
        assert_eq!(line_weight(0.13), (13, false));
        assert_eq!(line_weight(0.7), (70, false));
        assert_eq!(line_weight(0.33), (35, true));
        assert_eq!(line_weight(5.0), (211, true));
        assert_eq!(line_weight(f64::NAN), (-3, true));
    }
}
