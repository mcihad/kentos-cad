//! The questions every driver answers the same way (docs/adr/9100 §8): a dataset of parcels made through the driver
//! itself ([`Transaction::create_table`], [`Transaction::insert`]), then asked for boxes, conditions, orders, pages,
//! columns, distinct values, simplified geometry and edits; each answer compared with the one worked out by brute
//! force over the dataset in memory ([`Condition::holds`], box tests), never with another driver's. A new driver
//! (Oracle, MySQL, SQL Server) passes [`run`] before it is registered.
//!
//! The dataset is fixed by a linear congruential generator (`x' = (1103515245 x + 12345) mod 2³¹`, seed 2026): 24 × 25
//! parcels on a grid of 40 m cells from (587 000, 4 400 000) in TM36, each a rectangle shrunk by a pseudo-random
//! margin; a few rows have NULL values and one has no geometry. Texts carry Turkish letters and are ordered in the
//! Turkish alphabet's order (`turkish::compare`) by every driver.

use crate::cancel::Cancel;
use crate::condition::{Condition, Op, compare};
use crate::driver::{Source, collect};
use crate::edit::{Change, NewFeature, NewTable};
use crate::feature::Feature;
use crate::geom::{Coord, Curve, Geometry};
use crate::request::{Order, Request, meets};
use crate::schema::{Field, FieldKind, GeometryField};
use crate::value::Value;

/// Columns of the dataset, in order.
pub const FIELDS: [&str; 8] = [
    "ada", "parsel", "kod", "mahalle", "alan", "kullanim", "onayli", "tarih",
];
const COLS: usize = 24;
const ROWS: usize = 25;
/// Rows in the dataset.
pub const N: usize = COLS * ROWS;
const X0: f64 = 587_000.0;
const Y0: f64 = 4_400_000.0;
const CELL: f64 = 40.0;
const MAHALLE: [&str; 5] = ["Kılavuz", "Esentepe", "Mevlana", "Yenişehir", "İnönü"];
const KULLANIM: [&str; 4] = ["Eğitim", "Konut", "Park", "Ticaret"];

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = (1_103_515_245 * self.0 + 12_345) % (1 << 31);
        self.0
    }

    /// A number in [0, 1).
    fn unit(&mut self) -> f64 {
        self.next() as f64 / (1u64 << 31) as f64
    }
}

/// A row of the dataset in memory: its geometry and values in [`FIELDS`]' order.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub geometry: Option<Geometry>,
    pub values: Vec<Value>,
}

/// The dataset's table, named `name`.
pub fn table(name: &str, srid: i32) -> NewTable {
    NewTable {
        name: name.to_owned(),
        title: Some("Uygunluk parselleri".into()),
        description: Some("KentOS sürücü uygunluk takımı".into()),
        key: "fid".into(),
        fields: vec![
            Field::new("ada", FieldKind::Int),
            Field::new("parsel", FieldKind::Int),
            Field::new("kod", FieldKind::Text { max: Some(16) }),
            Field::new("mahalle", FieldKind::Text { max: Some(40) }),
            Field::new("alan", FieldKind::Real),
            Field::new("kullanim", FieldKind::Text { max: None }),
            Field::new("onayli", FieldKind::Bool),
            Field::new("tarih", FieldKind::Date),
        ],
        geometry: Some(GeometryField {
            name: "geom".into(),
            type_name: "POLYGON".into(),
            z: false,
            m: false,
            srid: Some(srid),
        }),
        spatial_index: true,
    }
}

/// The dataset's rows, in insertion order (the n-th row gets key n + 1 from a source that numbers from one).
pub fn rows() -> Vec<Row> {
    let mut r = Lcg(2026);
    let mut out = Vec::with_capacity(N);
    for j in 0..ROWS {
        for i in 0..COLS {
            let k = j * COLS + i;
            let m = 1.0 + 6.0 * r.unit();
            let x = X0 + i as f64 * CELL;
            let y = Y0 + j as f64 * CELL;
            let (w, h) = (CELL - 2.0 * m, CELL - m - 2.0 * r.unit());
            let ring = vec![
                Coord::xy(x + m, y + m),
                Coord::xy(x + m + w, y + m),
                Coord::xy(x + m + w, y + m + h),
                // A vertex nearly on the edge: simplification may drop it.
                Coord::xy(x + m + w / 2.0, y + m + h + 0.001),
                Coord::xy(x + m, y + m + h),
                Coord::xy(x + m, y + m),
            ];
            let geometry = if k == 77 {
                None
            } else {
                Some(Geometry::Polygon(vec![Curve::Line(ring)]))
            };
            let pick = r.next();
            let alan = if k % 41 == 5 {
                Value::Null
            } else {
                Value::Real((w * h * 100.0).round() / 100.0)
            };
            let kullanim = if k % 53 == 7 {
                Value::Null
            } else {
                Value::Text(KULLANIM[(pick % 4) as usize].into())
            };
            let mahalle = if k % 67 == 3 {
                Value::Null
            } else {
                Value::Text(MAHALLE[(j / 5) % 5].into())
            };
            let tarih = if k % 29 == 11 {
                Value::Null
            } else {
                Value::Date(format!("2026-{:02}-{:02}", 1 + (k % 12), 1 + (k % 28)))
            };
            out.push(Row {
                geometry,
                values: vec![
                    Value::Int(100 + j as i64),
                    Value::Int(1 + i as i64),
                    Value::Text(format!("P-{:04}", (pick % 997) as i64 * 7 % 997)),
                    mahalle,
                    alan,
                    kullanim,
                    Value::Bool(pick.is_multiple_of(3)),
                    tarih,
                ],
            });
        }
    }
    out
}

fn new_features(rows: &[Row]) -> Vec<NewFeature> {
    rows.iter()
        .map(|r| NewFeature {
            fid: None,
            geometry: r.geometry.clone(),
            values: FIELDS
                .iter()
                .zip(&r.values)
                .map(|(f, v)| ((*f).to_owned(), v.clone()))
                .collect(),
        })
        .collect()
}

/// What a run found wrong, one line each.
#[derive(Debug, Default)]
pub struct Report {
    pub failures: Vec<String>,
    pub checks: usize,
}

impl Report {
    fn check(&mut self, ok: bool, what: impl FnOnce() -> String) {
        self.checks += 1;
        if !ok {
            self.failures.push(what());
        }
    }
}

/// The dataset in memory by key: the keys the driver gave the inserted rows, in insertion order.
struct Truth {
    rows: Vec<(i64, Row)>,
}

impl Truth {
    fn value(&self, fid: i64, field: &str) -> Option<Value> {
        let at = FIELDS.iter().position(|f| *f == field)?;
        self.rows
            .iter()
            .find(|(k, _)| *k == fid)
            .map(|(_, r)| r.values[at].clone())
    }

    fn keep(&self, bbox: Option<[f64; 4]>, condition: Option<&Condition>) -> Vec<i64> {
        self.rows
            .iter()
            .filter(|(fid, r)| {
                let in_box = match bbox {
                    None => true,
                    Some(b) => r
                        .geometry
                        .as_ref()
                        .and_then(Geometry::bounds)
                        .is_some_and(|g| meets(g, b)),
                };
                in_box
                    && condition
                        .is_none_or(|c| c.holds(*fid, &|f: &str| self.value(*fid, f)) == Some(true))
            })
            .map(|(fid, _)| *fid)
            .collect()
    }

    /// Keys in an order (NULLs last either way; equal values by key).
    fn ordered(&self, mut fids: Vec<i64>, order: &[Order]) -> Vec<i64> {
        fids.sort_by(|a, b| {
            for o in order {
                let (va, vb) = (
                    self.value(*a, &o.field).unwrap_or(Value::Null),
                    self.value(*b, &o.field).unwrap_or(Value::Null),
                );
                let c = match (va.is_null(), vb.is_null()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    (false, false) => {
                        let c = match (&va, &vb) {
                            (Value::Text(a), Value::Text(b)) => crate::turkish::compare(a, b),
                            _ => compare(&va, &vb).unwrap_or(std::cmp::Ordering::Equal),
                        };
                        if o.descending { c.reverse() } else { c }
                    }
                };
                if c != std::cmp::Ordering::Equal {
                    return c;
                }
            }
            a.cmp(b)
        });
        fids
    }
}

fn fids(features: &[Feature]) -> Vec<i64> {
    features.iter().map(|f| f.fid).collect()
}

fn sorted(mut v: Vec<i64>) -> Vec<i64> {
    v.sort_unstable();
    v
}

/// Makes the dataset in `table` through the source (replacing a table of that name) and asks it every question.
pub fn run(source: &dyn Source, table_name: &str, srid: i32) -> Result<Report, String> {
    let mut report = Report::default();
    let data = rows();
    // Made through the driver, in one transaction.
    let mut tx = source.begin().map_err(|e| format!("begin: {e}"))?;
    tx.create_table(&table(table_name, srid), true)
        .map_err(|e| format!("create_table: {e}"))?;
    let keys = tx
        .insert(table_name, &new_features(&data))
        .map_err(|e| format!("insert: {e}"))?;
    tx.commit().map_err(|e| format!("commit: {e}"))?;
    report.check(keys.len() == N, || {
        format!("{} anahtar döndü, {N} beklendi", keys.len())
    });
    let truth = Truth {
        rows: keys.iter().copied().zip(data.iter().cloned()).collect(),
    };

    // The shape.
    let schema = source
        .schema(table_name)
        .map_err(|e| format!("schema: {e}"))?;
    let names: Vec<&str> = schema.fields.iter().map(|f| f.name.as_str()).collect();
    report.check(names == FIELDS, || format!("alanlar {names:?}"));
    // A table the driver made gives a new row its key itself (docs/adr/9103 §4).
    report.check(schema.key_made, || "anahtarı kaynak vermiyor".into());
    report.check(
        schema
            .field("alan")
            .is_some_and(|f| f.kind == FieldKind::Real)
            && schema
                .field("ada")
                .is_some_and(|f| f.kind == FieldKind::Int)
            && schema
                .field("onayli")
                .is_some_and(|f| f.kind == FieldKind::Bool)
            && schema
                .field("tarih")
                .is_some_and(|f| f.kind == FieldKind::Date),
        || format!("alan türleri {:?}", schema.fields),
    );
    report.check(
        schema
            .geometry
            .as_ref()
            .is_some_and(|g| g.srid == Some(srid)),
        || format!("geometri {:?}", schema.geometry),
    );

    // Every row, every value, every geometry.
    let all = collect(source, table_name, &Request::all()).map_err(|e| format!("all: {e}"))?;
    report.check(fids(&all) == sorted(keys.clone()), || {
        "bütün satırlar anahtar sırasıyla gelmedi".into()
    });
    for f in &all {
        let Some((_, r)) = truth.rows.iter().find(|(k, _)| *k == f.fid) else {
            continue;
        };
        report.check(f.values == r.values, || {
            format!("{}: değerler {:?} ≠ {:?}", f.fid, f.values, r.values)
        });
        report.check(f.shape() == r.geometry.as_ref(), || {
            format!("{}: geometri farklı", f.fid)
        });
    }
    // The same rows read inside a transaction by key, a missing key left out (docs/adr/9104 §3).
    {
        let mut tx = source.begin().map_err(|e| format!("begin: {e}"))?;
        let asked = [keys[2], keys[0], i64::MAX - 7];
        let got = tx.rows(table_name, &asked).map_err(|e| format!("rows: {e}"))?;
        report.check(fids(&got) == sorted(vec![keys[0], keys[2]]), || {
            format!("işlemde okunan anahtarlar {:?}", fids(&got))
        });
        for f in &got {
            if let Some((_, r)) = truth.rows.iter().find(|(k, _)| *k == f.fid) {
                report.check(f.values == r.values && f.shape() == r.geometry.as_ref(), || {
                    format!("{}: işlemde okunan satır farklı", f.fid)
                });
            }
        }
        tx.rollback().map_err(|e| format!("rollback: {e}"))?;
    }
    let count = source
        .count(table_name, &Request::all(), false)
        .map_err(|e| format!("count: {e}"))?;
    report.check(count == N as u64, || format!("sayı {count}"));

    // Boxes.
    for b in [
        [X0 + 100.0, Y0 + 100.0, X0 + 260.0, Y0 + 190.0],
        [X0 - 50.0, Y0 - 50.0, X0 + 3.0, Y0 + 3.0],
        [X0 + 5000.0, Y0, X0 + 6000.0, Y0 + 10.0],
        [X0 + 79.0, Y0 + 79.0, X0 + 81.0, Y0 + 81.0],
    ] {
        let want = truth.keep(Some(b), None);
        let got = collect(
            source,
            table_name,
            &Request {
                bbox: Some(b),
                ..Request::all()
            },
        )
        .map_err(|e| format!("bbox: {e}"))?;
        report.check(sorted(fids(&got)) == want, || {
            format!("kutu {b:?}: {:?} ≠ {want:?}", sorted(fids(&got)))
        });
        let n = source
            .count(
                table_name,
                &Request {
                    bbox: Some(b),
                    ..Request::all()
                },
                false,
            )
            .map_err(|e| format!("bbox count: {e}"))?;
        report.check(n == want.len() as u64, || {
            format!("kutu sayısı {n} ≠ {}", want.len())
        });
    }

    // Conditions.
    let conditions = vec![
        Condition::cmp("alan", Op::Gt, 900.0),
        Condition::cmp("ada", Op::Le, 102_i64),
        Condition::eq("kullanim", "Konut"),
        Condition::cmp("kullanim", Op::Ne, "Park"),
        Condition::In {
            field: "parsel".into(),
            values: vec![Value::Int(1), Value::Int(5), Value::Int(24)],
        },
        Condition::Between {
            field: "alan".into(),
            low: Value::Real(700.0),
            high: Value::Real(800.0),
        },
        Condition::IsNull("alan".into()),
        Condition::NotNull("mahalle".into()),
        Condition::Like {
            field: "mahalle".into(),
            pattern: "%ŞEHİR".into(),
            fold: true,
        },
        Condition::Contains {
            fields: vec!["mahalle".into(), "kullanim".into()],
            text: "İNÖ".into(),
        },
        Condition::Fids(vec![keys[3], keys[400], keys[599]]),
        Condition::eq("onayli", true),
        Condition::cmp("tarih", Op::Ge, Value::Date("2026-10-01".into())),
        Condition::Not(Box::new(Condition::eq("kullanim", "Ticaret"))),
        Condition::Or(vec![
            Condition::cmp("alan", Op::Lt, 650.0),
            Condition::And(vec![
                Condition::eq("kullanim", "Eğitim"),
                Condition::cmp("ada", Op::Gt, 120_i64),
            ]),
        ]),
    ];
    for c in &conditions {
        let want = truth.keep(None, Some(c));
        let got = collect(
            source,
            table_name,
            &Request {
                condition: Some(c.clone()),
                ..Request::all()
            },
        )
        .map_err(|e| format!("condition {c:?}: {e}"))?;
        report.check(sorted(fids(&got)) == want, || {
            format!("koşul {c:?}: {} satır, {} beklendi", got.len(), want.len())
        });
        let n = source
            .count(
                table_name,
                &Request {
                    condition: Some(c.clone()),
                    ..Request::all()
                },
                false,
            )
            .map_err(|e| format!("condition count {c:?}: {e}"))?;
        report.check(n == want.len() as u64, || {
            format!("koşul sayısı {c:?}: {n} ≠ {}", want.len())
        });
    }

    // Orders and pages.
    for order in [
        vec![Order {
            field: "alan".into(),
            descending: true,
        }],
        vec![
            Order {
                field: "kod".into(),
                descending: false,
            },
            Order {
                field: "ada".into(),
                descending: true,
            },
        ],
        vec![Order {
            field: "tarih".into(),
            descending: false,
        }],
        // Turkish letters in the Turkish alphabet's order whatever the source's collation (docs/adr/9102 §3):
        // Esentepe, İnönü, Kılavuz, Mevlana, Yenişehir.
        vec![
            Order {
                field: "mahalle".into(),
                descending: false,
            },
            Order {
                field: "kullanim".into(),
                descending: true,
            },
        ],
    ] {
        let want = truth.ordered(truth.keep(None, None), &order);
        for (offset, limit) in [(0u64, 25u64), (25, 25), (590, 25)] {
            let got = collect(
                source,
                table_name,
                &Request {
                    order: order.clone(),
                    offset,
                    limit: Some(limit),
                    chunk: 7,
                    ..Request::all()
                },
            )
            .map_err(|e| format!("order: {e}"))?;
            let w: Vec<i64> = want
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .copied()
                .collect();
            report.check(fids(&got) == w, || {
                format!("sıra {order:?} {offset}+{limit}: {:?} ≠ {w:?}", fids(&got))
            });
        }
    }

    // Columns.
    let got = collect(
        source,
        table_name,
        &Request {
            columns: Some(vec!["kullanim".into(), "ada".into()]),
            geometry: false,
            limit: Some(5),
            ..Request::all()
        },
    )
    .map_err(|e| format!("columns: {e}"))?;
    for f in &got {
        let want = vec![
            truth.value(f.fid, "kullanim").unwrap_or(Value::Null),
            truth.value(f.fid, "ada").unwrap_or(Value::Null),
        ];
        report.check(f.values == want && f.geometry.is_none(), || {
            format!("sütunlar {}: {:?}", f.fid, f.values)
        });
    }

    // Distinct values.
    let got = source
        .unique(table_name, "kullanim", None, 10)
        .map_err(|e| format!("unique: {e}"))?;
    let want: Vec<Value> = KULLANIM.iter().map(|k| Value::Text((*k).into())).collect();
    report.check(got == want, || format!("farklı değerler {got:?}"));

    // Simplified geometry: rings keep closing, the nearly straight vertex goes.
    let got = collect(
        source,
        table_name,
        &Request {
            simplify: Some(0.5),
            limit: Some(20),
            ..Request::all()
        },
    )
    .map_err(|e| format!("simplify: {e}"))?;
    for f in got.iter().filter(|f| f.shape().is_some()) {
        let ok = match f.shape() {
            Some(Geometry::Polygon(rings)) => rings.iter().all(|r| match r {
                Curve::Line(p) => p.len() >= 4 && p.len() <= 5 && p.first() == p.last(),
                _ => false,
            }),
            _ => false,
        };
        report.check(ok, || format!("sadeleştirme {}: {:?}", f.fid, f.shape()));
    }

    // Stopping.
    let cancel = Cancel::new();
    let mut parts = 0;
    let read = source
        .features(
            table_name,
            &Request {
                chunk: 50,
                ..Request::all()
            },
            &cancel,
            &mut |_| {
                parts += 1;
                cancel.cancel();
                true
            },
        )
        .map_err(|e| format!("cancel: {e}"))?;
    report.check(parts == 1 && read.stopped, || {
        format!("durdurma: {parts} parça, {read:?}")
    });

    // Edits: a rolled back transaction writes nothing; a committed one writes it all.
    let (a, b, c) = (keys[10], keys[11], keys[12]);
    let moved = Geometry::Polygon(vec![Curve::Line(vec![
        Coord::xy(X0, Y0),
        Coord::xy(X0 + 5.0, Y0),
        Coord::xy(X0 + 5.0, Y0 + 5.0),
        Coord::xy(X0, Y0),
    ])]);
    for commit in [false, true] {
        let mut tx = source.begin().map_err(|e| format!("begin: {e}"))?;
        tx.update(
            table_name,
            &[
                Change {
                    fid: a,
                    geometry: None,
                    values: vec![("kullanim".into(), Value::Text("Sağlık".into()))],
                },
                Change {
                    fid: b,
                    geometry: Some(Some(moved.clone())),
                    values: vec![("alan".into(), Value::Null)],
                },
            ],
        )
        .map_err(|e| format!("update: {e}"))?;
        tx.delete(table_name, &[c])
            .map_err(|e| format!("delete: {e}"))?;
        let new = tx
            .insert(
                table_name,
                &[NewFeature {
                    fid: None,
                    geometry: Some(moved.clone()),
                    values: vec![
                        ("ada".into(), Value::Int(999)),
                        ("mahalle".into(), Value::Text("Çiftlik".into())),
                    ],
                }],
            )
            .map_err(|e| format!("insert one: {e}"))?;
        if commit {
            tx.commit().map_err(|e| format!("commit: {e}"))?;
        } else {
            tx.rollback().map_err(|e| format!("rollback: {e}"))?;
        }
        let after = collect(
            source,
            table_name,
            &Request::fids(vec![a, b, c, new.first().copied().unwrap_or(-1)]),
        )
        .map_err(|e| format!("after edit: {e}"))?;
        let at = |fid: i64| after.iter().find(|f| f.fid == fid);
        let kullanim = FIELDS.iter().position(|f| *f == "kullanim").unwrap_or(0);
        let alan = FIELDS.iter().position(|f| *f == "alan").unwrap_or(0);
        if commit {
            report.check(
                at(a).is_some_and(|f| f.values[kullanim] == Value::Text("Sağlık".into())),
                || "yazılan değer okunmadı".into(),
            );
            report.check(
                at(b).is_some_and(|f| f.shape() == Some(&moved) && f.values[alan].is_null()),
                || "yazılan geometri okunmadı".into(),
            );
            report.check(at(c).is_none(), || "silinen satır duruyor".into());
            report.check(
                new.len() == 1 && new.first().and_then(|k| at(*k)).is_some(),
                || "eklenen satır okunmadı".into(),
            );
        } else {
            report.check(
                at(a).is_some_and(|f| {
                    f.values[kullanim] == truth.value(a, "kullanim").unwrap_or(Value::Null)
                }),
                || "geri alınan değer yazılmış".into(),
            );
            report.check(at(c).is_some(), || "geri alınan silme yazılmış".into());
            report.check(after.len() == 3, || {
                format!("geri alınan ekleme yazılmış: {}", after.len())
            });
        }
    }

    // A change of a key the table does not have is refused, and nothing of its transaction is written.
    let mut tx = source.begin().map_err(|e| format!("begin: {e}"))?;
    let refused = tx
        .update(
            table_name,
            &[Change {
                fid: i64::MAX - 1,
                geometry: None,
                values: vec![("ada".into(), Value::Int(1))],
            }],
        )
        .is_err();
    let _ = tx.rollback();
    report.check(refused, || {
        "olmayan satırın değişikliği reddedilmedi".into()
    });

    Ok(report)
}
