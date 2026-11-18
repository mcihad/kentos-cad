//! What the model says of an element without being told, and why (docs/adr/8124; EK-6 `IsExternal`,
//! `Pset_ColumnCommon.LoadBearing`): Öznitelikler shows it beside the element's own declaration, which wins over it.
//!
//! - **Outside:** a wall with a corner on its storey's outline (its walls' and columns' bodies together, the reading the
//!   model's `IsExternal` and Ek-3/B's exterior wall share: [`Document::fire_elements`]); a balcony's or a terrace's slab;
//!   a roof always; a door or a window as its wall stands, declared or read.
//! - **Carries:** a column of the frame; a post (a curtain wall's, a pergola's) does not.

use kentos_contracts::{Entity, OpeningForm, SlabRole};

use crate::Document;

/// A reading of the drawing and why it reads so (“katın dış çizgisinde olduğu için dışarıda”).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    pub value: bool,
    pub why: &'static str,
}

impl Reading {
    fn of(value: bool, yes: &'static str, no: &'static str) -> Reading {
        Reading { value, why: if value { yes } else { no } }
    }
}

impl Document {
    /// Whether `e` (its persistent id `uid`) is outside as the drawing reads it, its own declaration aside, and why; none
    /// for an element the model asks no `IsExternal` of.
    pub fn outside_reading(&self, uid: &str, e: &Entity) -> Option<Reading> {
        match e {
            Entity::Wall(_) => {
                let on = self.fire_elements().iter().any(|f| f.uid == uid && f.outline);
                Some(Reading::of(
                    on,
                    "katın dış çizgisinde olduğu için dışarıda",
                    "katın dış çizgisine değmediği için içeride",
                ))
            }
            Entity::Slab(s) if s.slab.covering.is_none() => Some(match s.slab.role {
                SlabRole::Floor => Reading { value: false, why: "kat döşemesi olduğu için içeride" },
                SlabRole::Balcony => Reading { value: true, why: "balkon döşemesi olduğu için dışarıda" },
                SlabRole::Terrace => Reading { value: true, why: "teras döşemesi olduğu için dışarıda" },
            }),
            Entity::Roof(_) => Some(Reading { value: true, why: "çatı olduğu için dışarıda" }),
            Entity::Opening(o) if matches!(o.opening.form, OpeningForm::Door | OpeningForm::Window) => {
                let wall = crate::identity::Uuid::parse_str(&o.opening.host)
                    .ok()
                    .and_then(|u| self.slot_of(u))
                    .and_then(|slot| self.get(slot))?;
                let read = self.outside_reading(&o.opening.host, wall)?;
                let outside = wall.arch_info().and_then(|a| a.external.as_ref()).map_or(read.value, |x| x.external);
                Some(Reading::of(outside, "duvarı dışarıda olduğu için dışarıda", "duvarı içeride olduğu için içeride"))
            }
            _ => None,
        }
    }

    /// Whether a column carries as the drawing reads it, its own declaration aside, and why; none for another element.
    pub fn bearing_reading(e: &Entity) -> Option<Reading> {
        match e {
            Entity::Column(c) => Some(Reading::of(
                c.column.member.is_none(),
                "çerçevenin kolonu olduğu için taşıyıcı",
                "dikme olduğu için taşıyıcı değil",
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::wall_height::tests::{document, slab};

    fn wall(a: [f64; 2], b: [f64; 2]) -> serde_json::Value {
        serde_json::json!({
            "kind": "wall", "pts": [{"x": a[0], "y": a[1]}, {"x": b[0], "y": b[1]}],
            "width": 0.2, "justify": "center", "arch": {"level": "zemin"}
        })
    }

    /// A 6 × 4 box of walls with a wall across it: the box's walls are on the storey's outline, the one across is not;
    /// a slab reads by its role; a declared wall keeps its reading and the door follows the declaration.
    #[test]
    fn walls_and_slabs_read_by_the_drawing_and_say_why() {
        let mut items = vec![
            wall([0.0, 0.0], [6.0, 0.0]),
            wall([6.0, 0.0], [6.0, 4.0]),
            wall([6.0, 4.0], [0.0, 4.0]),
            wall([0.0, 4.0], [0.0, 0.0]),
            wall([3.0, 0.0], [3.0, 4.0]),
        ];
        let mut balcony = slab("zemin", 0.0, 2.0, 0.15, 0.0);
        balcony["role"] = serde_json::json!("balcony");
        items.push(balcony);
        items.push(slab("zemin", 2.0, 4.0, 0.15, 0.0));
        let doc = document(serde_json::Value::Array(items));
        let read: Vec<(bool, &str)> = doc
            .entities_with_uids()
            .filter_map(|(u, e)| doc.outside_reading(&u.to_string(), e).map(|r| (r.value, r.why)))
            .collect();
        assert_eq!(read.len(), 7, "five walls and two slabs");
        assert!(read[..4].iter().all(|(v, why)| *v && why.contains("dış çizgisinde")), "{read:?}");
        assert_eq!(read[4], (false, "katın dış çizgisine değmediği için içeride"));
        assert_eq!(read[5], (true, "balkon döşemesi olduğu için dışarıda"));
        assert_eq!(read[6], (false, "kat döşemesi olduğu için içeride"));
    }
}
