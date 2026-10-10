//! The attribute table's columns and rows (docs/adr/0199 §4; the web's
//! `model/featureTable.ts`), read from a layer's objects and fields for the
//! core's `ops::feature_table`, which shows and orders them. Columns: Tür,
//! the layer's fields (their aliases, in order), then the keys no field
//! names (the natural order). A field's cell shows its value as the field
//! displays it (a code its label, Evet/Hayır, GG.AA.YYYY) and sorts by its
//! canonical text (a value list by its label); a value that does not keep
//! the field's rules shows as written, with the reason, and sorts last. The
//! panel is the desktop's `features`.

use std::collections::{BTreeSet, HashMap};

use kentos_contracts::{Entity, LayerField, LayerFieldKind, check_value, display_value, js_trim};
use kentos_geometry_core::ops::feature_table::{Cell, Column, Row};
use kentos_geometry_core::ops::point_editor::natural_order;

use crate::data_search::kind_name;

/// A column of the table: the attribute's key (none: Tür), its header, its
/// field, how it sorts (the core's column).
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureColumn {
    pub key: Option<String>,
    pub label: String,
    pub field: Option<LayerField>,
    pub order: &'static str,
}

impl FeatureColumn {
    /// The core's column: its order, and whether the search looks in it (not Tür).
    pub fn core(&self) -> Column {
        Column {
            order: self.order.to_owned(),
            searched: self.key.is_some(),
        }
    }
}

/// How a field's values are ordered: a value list by its labels (what is
/// seen), else by its kind.
pub fn order_of(f: &LayerField) -> &'static str {
    if f.values.as_ref().is_some_and(|v| !v.is_empty()) {
        return "text";
    }
    match f.kind {
        LayerFieldKind::Integer | LayerFieldKind::Decimal => "number",
        LayerFieldKind::Date => "date",
        LayerFieldKind::Boolean => "boolean",
        LayerFieldKind::Text => "text",
    }
}

/// A field's cell: what it shows, its sort key, and why its value does not
/// keep the field's rules (when it does not).
pub fn field_cell(f: &LayerField, raw: Option<&str>) -> (Cell, Option<String>) {
    match check_value(f, raw.unwrap_or_default()) {
        Err(r) => (
            Cell {
                shown: raw.unwrap_or_default().to_owned(),
                key: None,
            },
            Some(r.message),
        ),
        Ok(v) if v.is_empty() => (Cell::default(), None),
        Ok(v) => {
            let shown = display_value(f, &v);
            let listed = f.values.as_ref().is_some_and(|l| !l.is_empty());
            let key = if listed { shown.clone() } else { v };
            (
                Cell {
                    shown,
                    key: Some(key),
                },
                None,
            )
        }
    }
}

/// The columns a layer's table has: Tür, its fields, then the other keys its
/// objects carry (the natural order).
pub fn feature_columns(fields: &[LayerField], entities: &[&Entity]) -> Vec<FeatureColumn> {
    let named: BTreeSet<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let mut others: Vec<String> = Vec::new();
    let mut seen = BTreeSet::new();
    for e in entities {
        for k in e.base().attrs.keys() {
            if !named.contains(k.as_str()) && seen.insert(k.clone()) {
                others.push(k.clone());
            }
        }
    }
    let mut columns = vec![FeatureColumn {
        key: None,
        label: "Tür".to_owned(),
        field: None,
        order: "text",
    }];
    columns.extend(fields.iter().map(|f| FeatureColumn {
        key: Some(f.name.clone()),
        label: f.label().to_owned(),
        field: Some(f.clone()),
        order: order_of(f),
    }));
    columns.extend(natural_order(&others).into_iter().map(|i| FeatureColumn {
        key: Some(others[i as usize].clone()),
        label: others[i as usize].clone(),
        field: None,
        order: "text",
    }));
    columns
}

/// The table read from a layer: its columns, the core's rows (the objects'
/// order), and why a cell's value does not keep its field's rules, by row
/// and column.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeatureTable {
    pub columns: Vec<FeatureColumn>,
    pub rows: Vec<Row>,
    pub problems: HashMap<(usize, usize), String>,
}

/// The table of `entities` (a layer's objects in the drawing's order) under
/// `fields`: each object's cells, whether it is selected, in the view, and
/// kept by the expression filter (`passes`, none: every one).
pub fn feature_table_of(
    fields: &[LayerField],
    entities: &[&Entity],
    selected: &dyn Fn(&Entity) -> bool,
    in_view: &dyn Fn(&Entity) -> bool,
    passes: Option<&[bool]>,
) -> FeatureTable {
    let columns = feature_columns(fields, entities);
    let mut problems = HashMap::new();
    let rows = entities
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let cells = columns
                .iter()
                .enumerate()
                .map(|(j, c)| {
                    let Some(key) = &c.key else {
                        let kind = kind_name(e).to_owned();
                        return Cell {
                            shown: kind.clone(),
                            key: Some(kind),
                        };
                    };
                    let raw = e.base().attrs.get(key).map(String::as_str);
                    match &c.field {
                        Some(f) => {
                            let (cell, problem) = field_cell(f, raw);
                            if let Some(p) = problem {
                                problems.insert((i, j), p);
                            }
                            cell
                        }
                        None => Cell {
                            shown: raw.unwrap_or_default().to_owned(),
                            key: raw
                                .map(js_trim)
                                .filter(|v| !v.is_empty())
                                .map(str::to_owned),
                        },
                    }
                })
                .collect();
            Row {
                cells,
                selected: selected(e),
                in_view: in_view(e),
                passes: passes.is_none_or(|p| p.get(i).copied().unwrap_or(false)),
            }
        })
        .collect();
    FeatureTable {
        columns,
        rows,
        problems,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{EntityBase, FieldChoice, PointEntity, Vec2};

    fn point(attrs: &[(&str, &str)]) -> Entity {
        Entity::Point(PointEntity {
            base: EntityBase {
                id: 1,
                layer_id: "a".into(),
                color: None,
                attrs: attrs
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
                label: None,
                symbol: None,
                line_weight: None,
                label_pins: Vec::new(),
            },
            p: Vec2 { x: 0.0, y: 0.0 },
            z: None,
            parts: None,
        })
    }

    /// The same table the web's model test reads (model/featureTable.test.ts).
    #[test]
    fn a_layers_table_has_its_fields_then_the_other_keys_and_the_cells_as_the_fields_show_them() {
        let fields = vec![
            LayerField {
                values: Some(vec![
                    FieldChoice {
                        code: "K".into(),
                        label: "Konut".into(),
                    },
                    FieldChoice {
                        code: "T".into(),
                        label: "Ticaret".into(),
                    },
                ]),
                ..LayerField::new("Kullanım", LayerFieldKind::Text)
            },
            LayerField {
                alias: Some("Kat sayısı".into()),
                ..LayerField::new("Kat", LayerFieldKind::Integer)
            },
            LayerField::new("Tarih", LayerFieldKind::Date),
        ];
        let a = point(&[
            ("Kat", "+03"),
            ("Kullanım", "T"),
            ("Tarih", "7.10.2026"),
            ("not 10", "x"),
            ("Not 2", " "),
        ]);
        let b = point(&[("Kat", "3a"), ("Ada", "101")]);
        let t = feature_table_of(&fields, &[&a, &b], &|_| true, &|_| false, None);
        let headers: Vec<&str> = t.columns.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            headers,
            [
                "Tür",
                "Kullanım",
                "Kat sayısı",
                "Tarih",
                "Ada",
                "Not 2",
                "not 10"
            ]
        );
        let orders: Vec<&str> = t.columns.iter().map(|c| c.order).collect();
        assert_eq!(
            orders,
            ["text", "text", "number", "date", "text", "text", "text"]
        );
        let shown = |r: usize| -> Vec<(String, Option<String>)> {
            t.rows[r]
                .cells
                .iter()
                .map(|c| (c.shown.clone(), c.key.clone()))
                .collect()
        };
        let s = |v: &str| v.to_owned();
        assert_eq!(
            shown(0),
            [
                (s("Nokta"), Some(s("Nokta"))),
                (s("Ticaret"), Some(s("Ticaret"))),
                (s("3"), Some(s("3"))),
                (s("07.10.2026"), Some(s("2026-10-07"))),
                (s(""), None),
                (s(" "), None),
                (s("x"), Some(s("x"))),
            ]
        );
        assert_eq!(shown(1)[2], (s("3a"), None));
        assert!(t.problems[&(1, 2)].contains("tam sayı ister"));
        assert_eq!(t.problems.len(), 1);
        assert!(t.rows[0].selected && !t.rows[0].in_view && t.rows[0].passes);
    }
}
