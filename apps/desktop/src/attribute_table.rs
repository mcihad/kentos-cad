//! The Blok öznitelikleri window's table as texts (the web's
//! `ui/blocks/attributeTable.ts`, docs/adr/0144 §7): each attribute
//! definition is a row of its tag, prompt, default, text height, turn
//! (degrees) and place as east (Y) and north (X) of the base point, in the
//! block's own units; a CAD project names them X and Y (docs/adr/0165 §4). A number is written as a prompt writes it
//! (`+n.toFixed(6)`); a cell left as it was shown gives back the exact value
//! it showed, so a list saved unchanged is the list it was. A definition's
//! alignment and width factor (docs/adr/0145) have no cells: the row keeps
//! them. Both pass fixtures/blocks/v1/attribute-table.json.

use kentos_contracts::{AttributeDefinition, TextAlign, Vec2};
use kentos_interaction::{fixed, js_trim};

use crate::calc::grid::Col;
use crate::calc::read::read_number;

/// The cells of a row, by column.
pub const TAG: usize = 0;
pub const PROMPT: usize = 1;
pub const VALUE: usize = 2;
pub const HEIGHT: usize = 3;
pub const ROTATION: usize = 4;
pub const EAST: usize = 5;
pub const NORTH: usize = 6;

/// The table's columns (the web's `ATTRIBUTE_COLUMNS`).
pub const COLUMNS: [Col; 7] = [
    Col {
        label: "Etiket",
        unit: None,
        numeric: false,
    },
    Col {
        label: "Soru",
        unit: None,
        numeric: false,
    },
    Col {
        label: "Varsayılan",
        unit: None,
        numeric: false,
    },
    Col {
        label: "Yükseklik",
        unit: Some("m"),
        numeric: true,
    },
    Col {
        label: "Açı",
        unit: Some("°"),
        numeric: true,
    },
    Col {
        label: "Y",
        unit: Some("m"),
        numeric: true,
    },
    Col {
        label: "X",
        unit: Some("m"),
        numeric: true,
    },
];

/// The columns in a CAD project, its east X and its north Y (docs/adr/0165 §4).
pub const CAD_COLUMNS: [Col; 7] = {
    let mut cols = COLUMNS;
    cols[EAST].label = "X";
    cols[NORTH].label = "Y";
    cols
};

/// The table's columns in the project's axes.
pub fn columns(format: &kentos_interaction::Format) -> &'static [Col; 7] {
    match format.axes {
        kentos_interaction::Axes::Cad => &CAD_COLUMNS,
        kentos_interaction::Axes::Gis => &COLUMNS,
    }
}

/// The exact values a row's numbers were shown from, and what the row has no cells for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exact {
    pub height: f64,
    pub rotation: f64,
    pub p: Vec2,
    pub align: Option<TextAlign>,
    pub width_factor: Option<f64>,
}

/// A row: its cells' texts, and the exact values the numbers were shown from.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub cells: [String; 7],
    pub exact: Exact,
}

/// A number as a cell shows it: at most six decimals, no trailing zeros,
/// never “-0” (JavaScript's `String(+n.toFixed(6))`).
pub fn number_text(n: f64) -> String {
    let v = fixed(n, 6).parse::<f64>().unwrap_or(n);
    if v == 0.0 {
        "0".to_owned()
    } else if v.is_nan() {
        "NaN".to_owned()
    } else if v.is_infinite() {
        if v > 0.0 { "Infinity" } else { "-Infinity" }.to_owned()
    } else {
        format!("{v}")
    }
}

/// A row showing `a` of a block whose base point is `base`.
pub fn row_of(a: &AttributeDefinition, base: Vec2) -> Row {
    Row {
        cells: [
            a.tag.clone(),
            a.prompt.clone().unwrap_or_default(),
            a.value.clone().unwrap_or_default(),
            number_text(a.height),
            number_text(a.rotation),
            number_text(a.p.x - base.x),
            number_text(a.p.y - base.y),
        ],
        exact: Exact {
            height: a.height,
            rotation: a.rotation,
            p: a.p,
            align: a.align,
            width_factor: a.width_factor,
        },
    }
}

/// A cell's number: the exact one while it reads as that was shown; else
/// what it reads (`empty` for an empty cell, NaN for no number).
fn number_of(text: &str, exact: f64, empty: f64) -> f64 {
    if text == number_text(exact) {
        exact
    } else {
        read_number(text).unwrap_or(empty)
    }
}

/// A place's coordinate from its cell (east or north of the base's `from`):
/// the exact one while it reads as shown.
fn coordinate_of(text: &str, exact: f64, from: f64) -> f64 {
    if text == number_text(exact - from) {
        exact
    } else {
        from + read_number(text).unwrap_or(0.0)
    }
}

/// A text trimmed as JavaScript trims it; none when it is empty.
fn trimmed(text: &str) -> Option<String> {
    let t = js_trim(text);
    (!t.is_empty()).then(|| t.to_owned())
}

/// The definition a row gives: the tag, prompt and default trimmed (an
/// empty prompt or default left out), the height (an empty cell is no
/// height), the turn and the place (an empty cell is 0); its alignment and
/// width factor as they were.
pub fn definition_of(row: &Row, base: Vec2) -> AttributeDefinition {
    let (c, e) = (&row.cells, &row.exact);
    AttributeDefinition {
        tag: js_trim(&c[TAG]).to_owned(),
        prompt: trimmed(&c[PROMPT]),
        value: trimmed(&c[VALUE]),
        p: Vec2 {
            x: coordinate_of(&c[EAST], e.p.x, base.x),
            y: coordinate_of(&c[NORTH], e.p.y, base.y),
        },
        height: number_of(&c[HEIGHT], e.height, f64::NAN),
        rotation: number_of(&c[ROTATION], e.rotation, 0.0),
        align: e.align,
        width_factor: e.width_factor,
    }
}

/// The row with its place now `p` (picked on an insert, in the definition's coordinates).
pub fn placed(row: &Row, p: Vec2, base: Vec2) -> Row {
    let mut out = row.clone();
    out.cells[EAST] = number_text(p.x - base.x);
    out.cells[NORTH] = number_text(p.y - base.y);
    out.exact.p = p;
    out
}

/// A box east and north of the base point: the block's drawing as placed at its base, unturned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// A new row after `above` (the rows before it): a line and a half below
/// the last one's text, its height; the first one right of the block's
/// drawing, its text's top level with the drawing's (at the base point when
/// the block draws nothing), `height` high.
pub fn new_row(above: &[Row], base: Vec2, extent: Option<Extent>, height: f64) -> Row {
    let prev = above.last().map(|r| definition_of(r, base));
    let h = match &prev {
        Some(d) if d.height.is_finite() && d.height > 0.0 => d.height,
        _ => height,
    };
    let p = match (&prev, extent) {
        (Some(d), _) if d.p.x.is_finite() && d.p.y.is_finite() => Vec2 {
            x: d.p.x,
            y: d.p.y - 1.5 * h,
        },
        (_, Some(e)) => Vec2 {
            x: base.x + e.max_x + 0.2 * h,
            y: base.y + e.max_y - h,
        },
        _ => base,
    };
    let row = Row {
        cells: [
            String::new(),
            String::new(),
            String::new(),
            number_text(h),
            "0".to_owned(),
            String::new(),
            String::new(),
        ],
        exact: Exact {
            height: h,
            rotation: 0.0,
            p,
            align: None,
            width_factor: None,
        },
    };
    placed(&row, p, base)
}

/// The box of outline paths (`flags, n, x0, y0, …`, the geometry store's
/// `insert_outlines`); none for none.
pub fn outline_extent(paths: &[f64]) -> Option<Extent> {
    let mut out: Option<Extent> = None;
    let mut i = 0;
    while i + 1 < paths.len() {
        let n = paths[i + 1] as usize;
        for k in 0..n {
            let (Some(&x), Some(&y)) = (paths.get(i + 2 + 2 * k), paths.get(i + 3 + 2 * k)) else {
                return out;
            };
            out = Some(match out {
                Some(b) => Extent {
                    min_x: b.min_x.min(x),
                    min_y: b.min_y.min(y),
                    max_x: b.max_x.max(x),
                    max_y: b.max_y.max(y),
                },
                None => Extent {
                    min_x: x,
                    min_y: y,
                    max_x: x,
                    max_y: y,
                },
            });
        }
        i += 2 + 2 * n;
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    const FIXTURE: &str = include_str!("../../../fixtures/blocks/v1/attribute-table.json");

    /// A number of the file; "NaN" where JSON has none.
    fn num(v: &Value) -> f64 {
        match v {
            Value::String(s) if s == "NaN" => f64::NAN,
            v => v.as_f64().expect("a number"),
        }
    }

    fn vec2(v: &Value) -> Vec2 {
        Vec2 {
            x: num(&v["x"]),
            y: num(&v["y"]),
        }
    }

    fn definition(v: &Value) -> AttributeDefinition {
        AttributeDefinition {
            tag: v["tag"].as_str().expect("a tag").to_owned(),
            prompt: v["prompt"].as_str().map(str::to_owned),
            value: v["value"].as_str().map(str::to_owned),
            p: vec2(&v["p"]),
            height: num(&v["height"]),
            rotation: num(&v["rotation"]),
            align: v["align"].as_str().and_then(TextAlign::from_name),
            width_factor: v["widthFactor"].as_f64(),
        }
    }

    fn cells(v: &Value) -> [String; 7] {
        ["tag", "prompt", "value", "height", "rotation", "y", "x"]
            .map(|k| v[k].as_str().expect("a cell").to_owned())
    }

    /// Equal numbers, NaN equal to NaN.
    fn same(a: f64, b: f64) -> bool {
        a == b || (a.is_nan() && b.is_nan())
    }

    fn same_definition(a: &AttributeDefinition, b: &AttributeDefinition) -> bool {
        a.tag == b.tag
            && a.prompt == b.prompt
            && a.value == b.value
            && same(a.p.x, b.p.x)
            && same(a.p.y, b.p.y)
            && same(a.height, b.height)
            && same(a.rotation, b.rotation)
            && a.align == b.align
            && a.width_factor == b.width_factor
    }

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("the fixture reads")
    }

    fn list<'a>(v: &'a Value, key: &str) -> &'a Vec<Value> {
        v[key].as_array().expect("a list")
    }

    #[test]
    fn numbers_are_written_with_six_decimals_at_most_never_minus_zero() {
        for pair in list(&fixture(), "numbers") {
            let n = num(&pair[0]);
            assert_eq!(number_text(n), pair[1].as_str().expect("text"), "{n}");
        }
    }

    #[test]
    fn a_definition_is_shown_as_its_row() {
        for c in list(&fixture(), "rows") {
            let row = row_of(&definition(&c["definition"]), vec2(&c["base"]));
            assert_eq!(row.cells, cells(&c["cells"]), "{}", c["name"]);
        }
    }

    #[test]
    fn a_row_gives_back_its_definition_untouched_cells_exactly() {
        for c in list(&fixture(), "definitions") {
            let base = vec2(&c["base"]);
            let mut row = row_of(&definition(&c["from"]), base);
            row.cells = cells(&c["cells"]);
            let got = definition_of(&row, base);
            let want = definition(&c["definition"]);
            assert!(
                same_definition(&got, &want),
                "{}: {got:?} ≠ {want:?}",
                c["name"]
            );
        }
    }

    #[test]
    fn a_new_row_goes_below_the_last_the_first_right_of_the_drawing() {
        for c in list(&fixture(), "newRows") {
            let base = vec2(&c["base"]);
            let above: Vec<Row> = list(c, "above")
                .iter()
                .map(|a| row_of(&definition(a), base))
                .collect();
            let extent = c["extent"].as_object().map(|e| Extent {
                min_x: num(&e["minX"]),
                min_y: num(&e["minY"]),
                max_x: num(&e["maxX"]),
                max_y: num(&e["maxY"]),
            });
            let row = new_row(&above, base, extent, num(&c["height"]));
            assert_eq!(row.cells, cells(&c["cells"]), "{}", c["name"]);
            assert_eq!(
                row.exact,
                Exact {
                    height: num(&c["exactHeight"]),
                    rotation: 0.0,
                    p: vec2(&c["p"]),
                    align: None,
                    width_factor: None,
                },
                "{}",
                c["name"]
            );
        }
    }

    #[test]
    fn outline_paths_give_their_box() {
        // A closed square and a point: flags, count, coordinates.
        let paths = [
            1.0, 4.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 2.0, 1.0, 3.0, 0.5,
        ];
        assert_eq!(
            outline_extent(&paths),
            Some(Extent {
                min_x: -1.0,
                min_y: -1.0,
                max_x: 3.0,
                max_y: 1.0,
            })
        );
        assert_eq!(outline_extent(&[]), None);
    }
}
