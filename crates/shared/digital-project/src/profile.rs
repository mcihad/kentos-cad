//! The profile's layers (plan/ifc-uyum.md §3; docs/adr/8062 §3): the base profile (the regulation), an
//! administration's layer (Md. 11/3: its own four-character sub-disciplines, not EK-2's; Md. 12/2 and 13: the
//! optional classes' and the project's optional fields it asks; its written readings of the open points) and a
//! project's. A layer narrows or widens the one below it and says where it comes from; the resolved profile keeps, for
//! every parameter, code and level, the layer that set it. A layer that names a row, a list or a parameter the profile
//! does not have, or takes a code EK-2 or EK-3 already gives, is refused whole.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::catalog::{self, Catalog};
use crate::decision::Decision;
use crate::source::Level;

/// Whose layer it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayerKind {
    /// The regulation itself and KentOS's decisions.
    Base,
    /// An administration's profile (belediye, il özel idaresi, Bakanlık birimi).
    Administration,
    /// One project's.
    Project,
}

/// What a layer does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Op {
    /// A code added to a list: an administration's sub-discipline (Md. 11/3), a room the project needs (EK-3 3.3).
    AddCode { list: String, code: String, name: String },
    /// A row asked more (or less): an optional class's field the administration asks (Md. 12/2).
    SetLevel { row: String, level: Level },
    /// A parameter settled: the view definition, the PDF/A part, the file size limit, the height datum …
    SetParam { key: String, value: String },
}

/// A layer: its id, name, kind, where it comes from (the administration's letter and its date) and what it does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub kind: LayerKind,
    pub source: String,
    pub ops: Vec<Op>,
}

/// A parameter as the layers leave it: its value (none while open), KentOS's default, and who set it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setting {
    pub value: Option<String>,
    pub default: Option<String>,
    /// The layer's id; the base profile's is its decision's (“K13”).
    pub by: String,
}

/// A code a layer added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Added {
    pub list: String,
    pub code: String,
    pub name: String,
    pub by: String,
}

/// The profile the layers make.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Resolved {
    /// The layers in order, the base first.
    pub layers: Vec<(String, LayerKind)>,
    pub params: BTreeMap<String, Setting>,
    pub added: Vec<Added>,
    /// Rows whose level a layer changed, and by whom.
    pub levels: BTreeMap<String, (Level, String)>,
}

impl Resolved {
    /// A list's code: the catalog's or a layer's.
    pub fn has_code(&self, catalog: &Catalog, list: &str, code: &str) -> bool {
        catalog.code(list, code).is_some() || self.added.iter().any(|a| a.list == list && a.code == code)
    }
}

/// Why a layer was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Layers go base, administrations, project; one out of order.
    Order { layer: String },
    UnknownList { layer: String, list: String },
    /// EK-2 or EK-3 (or a layer below) already gives the code.
    CodeTaken { layer: String, list: String, code: String },
    /// Not the list's form: four letters for an item or sub-discipline (EK-2 2.5, Md. 11/3), three for a room (EK-3 3.3).
    BadCode { layer: String, list: String, code: String, form: &'static str },
    UnknownRow { layer: String, row: String },
    UnknownParam { layer: String, key: String },
}

impl Refusal {
    /// What to tell the user: the reason and what resolves it.
    pub fn message(&self) -> String {
        match self {
            Refusal::Order { layer } => format!(
                "“{layer}” katmanı yerinde değil: sıra temel profil, idare profilleri, proje. Katmanı sırasına koyun."
            ),
            Refusal::UnknownList { layer, list } => {
                format!("“{layer}” katmanı olmayan bir listeye kod ekliyor ({list}). Listenin adını düzeltin.")
            }
            Refusal::CodeTaken { layer, list, code } => format!(
                "“{layer}” katmanının eklediği {code} kodu {list} listesinde zaten var; yönetmelik çakışan kod istemez \
                 (Md. 11/3, EK-3 3.3). Başka bir kod seçin."
            ),
            Refusal::BadCode { layer, list, code, form } => {
                format!("“{layer}” katmanının {code} kodu {list} listesine uymuyor: {form}. Kodu düzeltin.")
            }
            Refusal::UnknownRow { layer, row } => {
                format!("“{layer}” katmanı katalogda olmayan bir satırı değiştiriyor ({row}). Satırın kimliğini düzeltin.")
            }
            Refusal::UnknownParam { layer, key } => format!(
                "“{layer}” katmanı profilde olmayan bir değeri ayarlıyor ({key}). Değerin adını düzeltin."
            ),
        }
    }
}

/// The form a list's new code must have, and its words.
fn code_form(list: &str) -> Option<(fn(&str) -> bool, &'static str)> {
    fn letters(n: usize) -> impl Fn(&str) -> bool {
        move |c: &str| c.len() == n && c.bytes().all(|b| b.is_ascii_uppercase())
    }
    if list.starts_with("item:") || list.starts_with("subdiscipline:") {
        Some((|c| letters(4)(c), "dört büyük harf, Türkçe harf olmadan"))
    } else if list == catalog::ROOMS {
        Some((|c| letters(3)(c), "üç büyük harf"))
    } else if list == catalog::DISCIPLINES {
        Some((|c| letters(2)(c), "iki büyük harf"))
    } else if list == catalog::FILE_TYPES {
        Some((
            |c| c.len() == 3 && c.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()),
            "üç büyük harf ya da rakam",
        ))
    } else {
        None
    }
}

/// The profile the base decisions and the layers make. `rows` says whether a row id is the catalog's (the 2D rows
/// here, kentos-ifc's IFC rows) and its level. All of a layer's refusals are said; a refused layer changes nothing.
pub fn resolve(
    catalog: &Catalog,
    decisions: &[&Decision],
    layers: &[Layer],
    rows: &dyn Fn(&str) -> Option<Level>,
) -> Result<Resolved, Vec<Refusal>> {
    let mut out = Resolved::default();
    for d in decisions {
        if let Some(p) = &d.param {
            out.params.insert(
                p.key.clone(),
                Setting {
                    value: p.value.clone(),
                    default: p.default.clone(),
                    by: d.id.clone(),
                },
            );
        }
    }
    let lists: std::collections::BTreeSet<&str> = catalog.codes.iter().map(|c| c.list.as_str()).collect();
    let mut refused = Vec::new();
    let mut last = LayerKind::Base;
    for layer in layers {
        if layer.kind < last || layer.kind == LayerKind::Base {
            refused.push(Refusal::Order {
                layer: layer.id.clone(),
            });
            continue;
        }
        last = layer.kind;
        let before = refused.len();
        let mut changes = out.clone();
        for op in &layer.ops {
            match op {
                Op::AddCode { list, code, name } => {
                    if !lists.contains(list.as_str()) {
                        refused.push(Refusal::UnknownList {
                            layer: layer.id.clone(),
                            list: list.clone(),
                        });
                    } else if changes.has_code(catalog, list, code) {
                        refused.push(Refusal::CodeTaken {
                            layer: layer.id.clone(),
                            list: list.clone(),
                            code: code.clone(),
                        });
                    } else if let Some(form) = code_form(list).and_then(|(ok, form)| (!ok(code)).then_some(form)) {
                        refused.push(Refusal::BadCode {
                            layer: layer.id.clone(),
                            list: list.clone(),
                            code: code.clone(),
                            form,
                        });
                    } else {
                        changes.added.push(Added {
                            list: list.clone(),
                            code: code.clone(),
                            name: name.clone(),
                            by: layer.id.clone(),
                        });
                    }
                }
                Op::SetLevel { row, level } => {
                    if rows(row).is_none() {
                        refused.push(Refusal::UnknownRow {
                            layer: layer.id.clone(),
                            row: row.clone(),
                        });
                    } else {
                        changes.levels.insert(row.clone(), (*level, layer.id.clone()));
                    }
                }
                Op::SetParam { key, value } => match changes.params.get_mut(key) {
                    Some(s) => {
                        s.value = Some(value.clone());
                        s.by = layer.id.clone();
                    }
                    None => refused.push(Refusal::UnknownParam {
                        layer: layer.id.clone(),
                        key: key.clone(),
                    }),
                },
            }
        }
        if refused.len() == before {
            out = changes;
            out.layers.push((layer.id.clone(), layer.kind));
        }
    }
    if refused.is_empty() { Ok(out) } else { Err(refused) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision;

    fn base_decisions() -> Vec<&'static Decision> {
        decision::base().iter().collect()
    }

    fn rows(id: &str) -> Option<Level> {
        catalog::base().row(id).map(|_| Level::Mandatory)
    }

    fn administration(ops: Vec<Op>) -> Layer {
        Layer {
            id: "ornek-idare".into(),
            name: "Örnek Belediyesi".into(),
            kind: LayerKind::Administration,
            source: "Örnek Belediyesi'nin yazısı, 1.3.2027".into(),
            ops,
        }
    }

    #[test]
    fn the_base_profile_leaves_open_points_open() {
        let r = resolve(catalog::base(), &base_decisions(), &[], &rows).expect("the base profile");
        let pdfa = &r.params["pdfa.part"];
        assert_eq!(pdfa.value, None);
        assert_eq!(pdfa.default.as_deref(), Some("PDF/A-2b"));
        assert_eq!(pdfa.by, "K13");
        assert_eq!(r.params["model.lod"].value.as_deref(), Some("LOD 300"));
    }

    #[test]
    fn an_administration_settles_a_point_and_adds_its_sub_discipline() {
        let layer = administration(vec![
            Op::SetParam {
                key: "pdfa.part".into(),
                value: "PDF/A-3b".into(),
            },
            Op::AddCode {
                list: "item:MM".into(),
                code: "ENGL".into(),
                name: "Engelli erişimi".into(),
            },
        ]);
        let r = resolve(catalog::base(), &base_decisions(), &[layer], &rows).expect("the layer applies");
        assert_eq!(r.params["pdfa.part"].value.as_deref(), Some("PDF/A-3b"));
        assert_eq!(r.params["pdfa.part"].by, "ornek-idare");
        assert!(r.has_code(catalog::base(), "item:MM", "ENGL"));
        assert_eq!(r.layers, vec![("ornek-idare".to_owned(), LayerKind::Administration)]);
    }

    #[test]
    fn a_layer_that_takes_a_code_or_names_nothing_is_refused_whole() {
        let layer = administration(vec![
            Op::SetParam {
                key: "pdfa.part".into(),
                value: "PDF/A-3b".into(),
            },
            Op::AddCode {
                list: "item:MM".into(),
                code: "DUVA".into(),
                name: "Duvar".into(),
            },
            Op::AddCode {
                list: "item:MM".into(),
                code: "Duv1".into(),
                name: "Duvar 1".into(),
            },
            Op::AddCode {
                list: catalog::ROOMS.into(),
                code: "SLN".into(),
                name: "Salon".into(),
            },
            Op::SetLevel {
                row: "T9.9/Yok".into(),
                level: Level::Mandatory,
            },
            Op::SetParam {
                key: "renk".into(),
                value: "mavi".into(),
            },
        ]);
        let refused = resolve(catalog::base(), &base_decisions(), &[layer], &rows).expect_err("refused");
        assert_eq!(refused.len(), 5, "{refused:?}");
        assert!(matches!(&refused[0], Refusal::CodeTaken { code, .. } if code == "DUVA"));
        assert!(matches!(&refused[1], Refusal::BadCode { code, .. } if code == "Duv1"));
        assert!(matches!(&refused[2], Refusal::CodeTaken { code, .. } if code == "SLN"));
        assert!(matches!(&refused[3], Refusal::UnknownRow { .. }));
        assert!(matches!(&refused[4], Refusal::UnknownParam { .. }));
        assert!(refused[0].message().contains("Md. 11/3"));
    }

    #[test]
    fn a_project_comes_after_its_administration() {
        let project = Layer {
            id: "proje".into(),
            name: "Proje".into(),
            kind: LayerKind::Project,
            source: "Müellif".into(),
            ops: vec![Op::AddCode {
                list: catalog::ROOMS.into(),
                code: "MZK".into(),
                name: "Müzik odası".into(),
            }],
        };
        let refused = resolve(
            catalog::base(),
            &base_decisions(),
            &[project.clone(), administration(vec![])],
            &rows,
        )
        .expect_err("out of order");
        assert!(matches!(&refused[0], Refusal::Order { layer } if layer == "ornek-idare"));
        let r = resolve(catalog::base(), &base_decisions(), &[administration(vec![]), project], &rows)
            .expect("in order");
        assert_eq!(r.added[0].by, "proje");
    }
}
