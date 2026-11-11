//! Asansör bloğu (docs/adr/8045 §5): an insert of a block whose library card's category is under Asansör (KentOS's own
//! Asansör/Yolcu) read as an elevator's cabin and its landing door, by what PAİY 34 measures: the cabin's inner width
//! and depth (Kabin eni, Kabin derinliği) and the door's net clear width (Kapı genişliği). Each is the insert's value,
//! else its parameter's default, times the insert's factor along it; the block's geometry is in metres (its unit).
//!
//! A fix sets a parameter (`cad.blocks.parameters`) to the least value its rule allows that meets the limit
//! (`block_values`).

use kentos_commands::{BlockDefinition, BlockParameter, Entity, InsertEntity, ParamValue};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use serde_json::{Map, Value, json};

use crate::block_values::{PARAMETERS, button_metres};
use crate::regulation_audit::Fix;

/// The cabin's inner width, along the definition's x.
pub const WIDTH: &str = "Kabin eni";
/// The cabin's inner depth, along the definition's y.
pub const DEPTH: &str = "Kabin derinliği";
/// The landing door's net clear width, along the definition's x.
pub const DOOR: &str = "Kapı genişliği";

/// Whether `def` is an elevator's: its card's category under Asansör.
pub fn is_elevator(def: &BlockDefinition) -> bool {
    def.card
        .as_ref()
        .and_then(|c| c.category.as_deref())
        .and_then(|c| c.split('/').next())
        .is_some_and(|c| c.trim() == "Asansör")
}

/// An elevator's insert as PAİY 34 reads it, in metres.
#[derive(Clone, Debug, PartialEq)]
pub struct Cabin {
    pub slot: Slot,
    pub level: Option<String>,
    /// The cabin's middle in the drawing.
    pub at: V,
    pub width: f64,
    pub depth: f64,
    /// Its door's net clear width; none when the block has no Kapı genişliği.
    pub door: Option<f64>,
    /// The insert's factors along the definition's x and y.
    pub scale: (f64, f64),
}

impl Cabin {
    /// Its narrow side.
    pub fn narrow(&self) -> f64 {
        self.width.min(self.depth)
    }

    /// Its inner area, m².
    pub fn area(&self) -> f64 {
        self.width * self.depth
    }

    /// The parameter its narrow side is: the width when it is no wider than deep.
    pub fn narrow_name(&self) -> &'static str {
        if self.width <= self.depth {
            WIDTH
        } else {
            DEPTH
        }
    }

    /// The other side's parameter.
    pub fn long_name(&self) -> &'static str {
        if self.width <= self.depth {
            DEPTH
        } else {
            WIDTH
        }
    }

    /// The real size of `name`'s side and the insert's factor along it.
    pub fn side(&self, name: &str) -> (f64, f64) {
        match name {
            WIDTH => (self.width, self.scale.0),
            DEPTH => (self.depth, self.scale.1),
            _ => (self.door.unwrap_or(0.0), self.scale.0),
        }
    }
}

/// A number parameter's value on `insert` and the parameter.
fn number<'a>(
    def: &'a BlockDefinition,
    insert: &InsertEntity,
    name: &str,
) -> Option<(f64, &'a BlockParameter)> {
    let d = def.dynamic.as_ref()?;
    let p = d.parameters.iter().find(|p| p.name() == name)?;
    let v = insert
        .parameters
        .get(name)
        .cloned()
        .or_else(|| d.type_values.get(name).cloned())
        .or_else(|| p.default_value())?;
    match v {
        ParamValue::Number(x) if x.is_finite() => Some((x, p)),
        _ => None,
    }
}

/// A linear parameter's line: its start and its unit direction.
fn line(p: &BlockParameter) -> Option<(V, V)> {
    let BlockParameter::Linear { from, to, .. } = p else {
        return None;
    };
    let l = (to.x - from.x).hypot(to.y - from.y);
    (l > 0.0).then(|| {
        (
            V::new(from.x, from.y),
            V::new((to.x - from.x) / l, (to.y - from.y) / l),
        )
    })
}

/// The drawing's elevators: every insert of an elevator's block with a cabin width and depth.
pub fn cabins(doc: &Document) -> Vec<Cabin> {
    doc.entities()
        .filter_map(|e| {
            let Entity::Insert(i) = e else { return None };
            let def = doc.block(i.block)?;
            if !is_elevator(def) {
                return None;
            }
            let (sx, sy) = (i.scale, i.y_scale.unwrap_or(i.scale));
            let (w, wp) = number(def, i, WIDTH)?;
            let (d, dp) = number(def, i, DEPTH)?;
            let door = number(def, i, DOOR).map(|(x, _)| x * sx.abs());
            // The cabin's middle: half its width along the width's line from its start, half its depth along the depth's.
            let mid = match (line(wp), line(dp)) {
                (Some((a, u)), Some((_, v))) => V::new(
                    a.x + u.x * w / 2.0 + v.x * d / 2.0,
                    a.y + u.y * w / 2.0 + v.y * d / 2.0,
                ),
                _ => V::new(def.base.x, def.base.y),
            };
            let m = kentos_geometry_core::block::insert_matrix_xy(
                V::new(def.base.x, def.base.y),
                V::new(i.p.x, i.p.y),
                (sx, sy),
                i.rotation,
                i.mirror,
            );
            Some(Cabin {
                slot: Slot(i.base.id),
                level: i.arch.level.clone(),
                at: kentos_geometry_core::geom::affine::apply(&m, mid),
                width: w * sx.abs(),
                depth: d * sy.abs(),
                door,
                scale: (sx.abs(), sy.abs()),
            })
        })
        .collect()
}

/// The value of `name` on `cabin`'s block that makes its side at least `real` metres (`block_values::least_value`).
pub fn value_for(doc: &Document, cabin: &Cabin, name: &str, real: f64) -> Option<f64> {
    let (_, factor) = cabin.side(name);
    if factor <= 0.0 {
        return None;
    }
    crate::block_values::least_value(doc, cabin.slot, name, real / factor)
}

/// The fix that makes `cabin` at least `narrow` on its narrow side, `area` m² and (given) its door `door` wide: its
/// narrow side raised to `narrow`, its other side to what `area` asks of it, its door to `door`, each the least its
/// rule allows. None when it needs none or a rule cannot give it. Its operation is `cad.blocks.parameters`' values.
pub fn fix(
    doc: &Document,
    cabin: &Cabin,
    narrow: f64,
    area: f64,
    door: Option<f64>,
) -> Option<Fix> {
    let (short, long) = (cabin.narrow_name(), cabin.long_name());
    let mut values = Map::new();
    let set = |name: &str, real: f64, values: &mut Map<String, Value>| -> Option<f64> {
        let v = value_for(doc, cabin, name, real)?;
        values.insert(name.to_owned(), json!(v));
        Some(v * cabin.side(name).1)
    };
    let mut short_real = cabin.narrow();
    if short_real + 1e-6 < narrow {
        short_real = set(short, narrow, &mut values)?;
    }
    let mut long_real = cabin.side(long).0;
    if short_real * long_real + 1e-6 < area {
        long_real = set(long, area / short_real, &mut values)?;
    }
    let sides = if short == WIDTH {
        (short_real, long_real)
    } else {
        (long_real, short_real)
    };
    let mut door_real = None;
    if let (Some(least), Some(now)) = (door, cabin.door)
        && now + 1e-6 < least
    {
        door_real = Some(set(DOOR, least, &mut values)?);
    }
    if values.is_empty() {
        return None;
    }
    let cabin_changed = values.contains_key(WIDTH) || values.contains_key(DEPTH);
    let label = match (cabin_changed, door_real) {
        (true, Some(d)) => format!(
            "Kabini {} × {} m, kapıyı {} m yap",
            button_metres(sides.0),
            button_metres(sides.1),
            button_metres(d)
        ),
        (true, None) => format!(
            "Kabini {} × {} m yap",
            button_metres(sides.0),
            button_metres(sides.1)
        ),
        (false, Some(d)) => format!("Kapıyı {} m yap", button_metres(d)),
        (false, None) => return None,
    };
    Some(Fix {
        label,
        operation: Value::Object(values),
        command: Some(PARAMETERS),
    })
}
