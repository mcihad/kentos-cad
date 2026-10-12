//! The project's address data (document schema 42, docs/specs/kcad-v2.md
//! §6.4.10, docs/adr/0229 §2): checked whole by the contract's rules
//! (`address_locators_problem`), as the readers check them, each map's keys
//! sorted as RFC 8949 sorts them. An optional field is written only when it
//! has a value, the areas only when there are some.

use kentos_contracts::{AddressLocator, AreaLevel, address_locators_problem};

use super::Encoder;
use crate::cbor::{Seg, key_order};
use crate::error::{Code, KcadError};

/// One field before the fields are sorted by key.
enum V<'d> {
    Text(&'d str),
    Uint(u32),
    Float(f64),
    Map(Vec<(&'static str, V<'d>)>),
    List(Vec<V<'d>>),
}

fn level_name(l: AreaLevel) -> &'static str {
    match l {
        AreaLevel::Neighbourhood => "neighbourhood",
        AreaLevel::District => "district",
        AreaLevel::Province => "province",
    }
}

/// `key` with `value` when there is one.
fn optional<'d>(
    fields: &mut Vec<(&'static str, V<'d>)>,
    key: &'static str,
    value: &'d Option<String>,
) {
    if let Some(v) = value {
        fields.push((key, V::Text(v)));
    }
}

impl<'d> Encoder<'d> {
    fn address_value(&mut self, v: V<'d>) -> Result<(), KcadError> {
        match v {
            V::Text(t) => self.text(t),
            V::Uint(n) => {
                self.w.uint(u64::from(n));
                Ok(())
            }
            V::Float(x) => self.float(x),
            V::Map(mut fields) => {
                fields.sort_by(|a, b| key_order(a.0, b.0));
                self.open(fields.len(), true)?;
                for (key, v) in fields {
                    self.key(key);
                    self.at(Seg::Name(key), |e| e.address_value(v))?;
                }
                self.close();
                Ok(())
            }
            V::List(items) => {
                self.open(items.len(), false)?;
                for (i, v) in items.into_iter().enumerate() {
                    self.at(Seg::Index(i), |e| e.address_value(v))?;
                }
                self.close();
                Ok(())
            }
        }
    }

    /// The project's address locators (docs/adr/0229 §2), checked whole.
    pub(super) fn address_locators(&mut self, list: &'d [AddressLocator]) -> Result<(), KcadError> {
        if let Some(problem) = address_locators_problem(list) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let items = list
            .iter()
            .map(|l| {
                let mut f = vec![
                    ("id", V::Text(&l.id)),
                    ("name", V::Text(&l.name)),
                    ("minScore", V::Uint(l.min_score)),
                    ("reach", V::Float(l.reach)),
                ];
                if let Some(d) = &l.doors {
                    let mut g = vec![
                        ("layer", V::Text(&d.layer)),
                        ("street", V::Text(&d.street)),
                        ("door", V::Text(&d.door)),
                    ];
                    optional(&mut g, "neighbourhood", &d.neighbourhood);
                    optional(&mut g, "district", &d.district);
                    optional(&mut g, "province", &d.province);
                    optional(&mut g, "postcode", &d.postcode);
                    optional(&mut g, "building", &d.building);
                    optional(&mut g, "code", &d.code);
                    f.push(("doors", V::Map(g)));
                }
                if let Some(s) = &l.streets {
                    let mut g = vec![("layer", V::Text(&s.layer)), ("name", V::Text(&s.name))];
                    optional(&mut g, "neighbourhood", &s.neighbourhood);
                    optional(&mut g, "district", &s.district);
                    f.push(("streets", V::Map(g)));
                }
                if !l.areas.is_empty() {
                    let areas = l
                        .areas
                        .iter()
                        .map(|a| {
                            let mut g = vec![
                                ("layer", V::Text(&a.layer)),
                                ("level", V::Text(level_name(a.level))),
                                ("name", V::Text(&a.name)),
                            ];
                            optional(&mut g, "district", &a.district);
                            V::Map(g)
                        })
                        .collect();
                    f.push(("areas", V::List(areas)));
                }
                V::Map(f)
            })
            .collect();
        self.address_value(V::List(items))
    }
}
