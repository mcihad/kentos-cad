//! The chosen object's details for the viewer's properties panel (feature `view`; docs/adr/8061 §5 "Özellikler",
//! docs/adr/8073 §2): its attributes, its type, its material, its storey, then its property and quantity sets (its
//! own, then its type's) as the file writes them, each value with its unit: its own, else the project's for the
//! measure it is written with. Nothing is converted and nothing is measured from the triangles (CLAUDE.md §23.3).

use ifc_lite_core::SchemaVersion;
use kentos_model_view::{Details, Group, Line};

use super::{Model, ReadError, Units, Value, ValueKind, header};

impl Model {
    /// The details of object `object` (an index into [`Model::objects`]); `bytes` are the bytes it was read from.
    pub fn details(&self, bytes: &[u8], object: usize) -> Result<Details, ReadError> {
        let Some(o) = self.objects.get(object) else { return Ok(Details::default()) };
        let schema = header::version(self.summary.family);
        let mut own = Group { title: "Nesne".to_owned(), lines: Vec::new() };
        let mut add = |name: &str, value: Option<String>| {
            if let Some(value) = value.filter(|v| !v.trim().is_empty()) {
                own.lines.push(Line { name: name.to_owned(), value, unit: None, numeric: false });
            }
        };
        add("Sınıf", Some(o.class.clone()));
        add("GlobalId", Some(o.global_id.clone()));
        add("Ad", o.name.clone());
        add("Açıklama", o.description.clone());
        add("Nesne türü", o.object_type.clone());
        add("Uzun ad", o.long_name.clone());
        add("Etiket", o.tag.clone());
        add("Ön tanımlı tür", o.predefined.clone());
        add("Tip", o.type_object.map(|t| &self.objects[t]).map(|t| format!("{} ({})", t.name.as_deref().unwrap_or("adsız"), t.class)));
        add("Kat", self.storey_of(schema, object).map(|s| s.name.clone().unwrap_or_else(|| s.class.clone())));
        add("Malzeme", self.materials(bytes, object).ok().filter(|m| !m.is_empty()).map(|m| m.join("; ")));
        add("Kayıt", Some(format!("#{}", o.step)));
        let mut groups = vec![own];
        for set in self.property_sets(bytes, object)? {
            let title = if set.from_type { format!("{} (tipten)", set.name) } else { set.name.clone() };
            groups.push(Group { title, lines: set.values.iter().map(|v| line(v, &self.summary.units)).collect() });
        }
        Ok(Details { groups })
    }

    /// The storey the object stands on: the nearest up its parts and containers.
    fn storey_of(&self, schema: SchemaVersion, object: usize) -> Option<&super::Object> {
        let mut at = self.objects.get(object).and_then(|o| o.parent.or(o.container));
        for _ in 0..self.objects.len() {
            let o = self.objects.get(at?)?;
            if schema.is_subtype_of(&o.class, "IfcBuildingStorey") {
                return Some(o);
            }
            at = o.parent.or(o.container);
        }
        None
    }
}

/// A property or a quantity as a line: its value as written, its unit.
fn line(v: &Value, units: &Units) -> Line {
    let (value, numeric, measure) = words(&v.value);
    let unit = match &v.unit {
        Some(own) => Some(written(own)),
        None => measure.and_then(|m| of_measure(m, units)),
    };
    Line { name: v.name.clone(), value, unit, numeric }
}

/// A value in words, whether it is a number, and the measure it is written with.
fn words(v: &ValueKind) -> (String, bool, Option<&str>) {
    match v {
        ValueKind::Text(s) => (s.clone(), false, None),
        ValueKind::Number { value, measure } => (value.to_string(), true, measure.as_deref()),
        ValueKind::Integer { value, measure } => (value.to_string(), true, measure.as_deref()),
        ValueKind::Logical(Some(true)) => ("Doğru".to_owned(), false, None),
        ValueKind::Logical(Some(false)) => ("Yanlış".to_owned(), false, None),
        ValueKind::Logical(None) => ("Bilinmiyor".to_owned(), false, None),
        ValueKind::Enumeration(items) => (items.join(", "), false, None),
        ValueKind::List(items) => (items.iter().map(|i| words(i).0).collect::<Vec<_>>().join(", "), false, None),
        ValueKind::Bounded { lower, upper } => {
            let (lo, hi) = (words(lower), words(upper));
            (format!("{} – {}", lo.0, hi.0), lo.1 || hi.1, lo.2.or(hi.2))
        }
        ValueKind::Reference(r) => (format!("#{r}"), false, None),
        ValueKind::Empty => ("—".to_owned(), false, None),
    }
}

/// The project's unit of a measure (`IFCLENGTHMEASURE`: its length unit), as a symbol.
fn of_measure(measure: &str, units: &Units) -> Option<String> {
    let u = match measure.to_ascii_uppercase().as_str() {
        "IFCLENGTHMEASURE" | "IFCPOSITIVELENGTHMEASURE" | "IFCNONNEGATIVELENGTHMEASURE" => units.length.as_ref(),
        "IFCAREAMEASURE" => units.area.as_ref(),
        "IFCVOLUMEMEASURE" => units.volume.as_ref(),
        "IFCPLANEANGLEMEASURE" | "IFCPOSITIVEPLANEANGLEMEASURE" => units.plane_angle.as_ref(),
        _ => None,
    }?;
    Some(symbol(u.prefix.as_deref(), &u.name))
}

/// A value's own unit as the reading face writes it (its prefix before its name, `MILLIMETRE`), as a symbol.
fn written(unit: &str) -> String {
    PREFIXES
        .iter()
        .find_map(|(p, _)| unit.strip_prefix(p).filter(|rest| base(rest).is_some()).map(|rest| symbol(Some(p), rest)))
        .unwrap_or_else(|| symbol(None, unit))
}

/// The SI prefixes and their symbols.
const PREFIXES: [(&str, &str); 10] = [
    ("MILLI", "m"),
    ("CENTI", "c"),
    ("DECI", "d"),
    ("KILO", "k"),
    ("MICRO", "µ"),
    ("MEGA", "M"),
    ("GIGA", "G"),
    ("NANO", "n"),
    ("HECTO", "h"),
    ("DECA", "da"),
];

/// An SI unit's symbol; a unit the file names itself (a conversion's `foot`) keeps its name.
fn symbol(prefix: Option<&str>, name: &str) -> String {
    let Some(b) = base(name) else { return name.to_owned() };
    let p = prefix.and_then(|p| PREFIXES.iter().find(|(n, _)| p.trim_matches('.').eq_ignore_ascii_case(n))).map_or("", |(_, s)| s);
    format!("{p}{b}")
}

fn base(name: &str) -> Option<&'static str> {
    Some(match name.trim_matches('.').to_ascii_uppercase().as_str() {
        "METRE" => "m",
        "SQUARE_METRE" => "m²",
        "CUBIC_METRE" => "m³",
        "RADIAN" => "rad",
        "GRAM" => "g",
        "SECOND" => "s",
        "NEWTON" => "N",
        "PASCAL" => "Pa",
        "WATT" => "W",
        "JOULE" => "J",
        "KELVIN" => "K",
        "DEGREE_CELSIUS" => "°C",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::{WRITTEN, read_ok};
    use super::*;

    #[test]
    fn units_are_written_as_symbols_and_named_ones_kept() {
        assert_eq!(written("MILLIMETRE"), "mm");
        assert_eq!(written("SQUARE_METRE"), "m²");
        assert_eq!(symbol(Some(".MILLI."), "CUBIC_METRE"), "mm³");
        assert_eq!(written("foot"), "foot");
        assert_eq!(symbol(None, "DEGREE"), "DEGREE", "a conversion's name stays as written");
    }

    #[test]
    fn a_wall_s_details_are_its_attributes_and_sets_as_the_file_writes_them() {
        let (_, bytes) = WRITTEN[0];
        let m = read_ok(bytes);
        let wall = m.objects.iter().position(|o| o.class == "IfcWall").expect("a wall");
        let d = m.details(bytes, wall).expect("the details");
        let own = &d.groups[0];
        let value = |name: &str| own.lines.iter().find(|l| l.name == name).map(|l| l.value.as_str());
        assert_eq!(value("Sınıf"), Some("IfcWall"));
        assert_eq!(value("GlobalId"), Some(m.objects[wall].global_id.as_str()));
        assert!(value("Kat").is_some(), "{own:?}");
        let sets = m.property_sets(bytes, wall).expect("the sets");
        assert_eq!(d.groups.len(), 1 + sets.len());
        for (g, s) in d.groups[1..].iter().zip(&sets) {
            assert!(g.title.starts_with(&s.name));
            assert_eq!(g.lines.len(), s.values.len());
        }
        assert_eq!(value("Malzeme"), Some("Tuğla"), "its layer set's material");
        // A quantity is a number as written with the project's unit for its measure.
        let quantities = d.groups.iter().find(|g| g.title == "Qto_WallBaseQuantities").expect("the wall's quantities");
        let at = |name: &str| quantities.lines.iter().find(|l| l.name == name).expect(name);
        assert_eq!((at("Length").value.as_str(), at("Length").unit.as_deref(), at("Length").numeric), ("12", Some("m"), true));
        assert_eq!((at("NetSideArea").value.as_str(), at("NetSideArea").unit.as_deref()), ("28.74", Some("m²")));
        assert_eq!(at("NetVolume").unit.as_deref(), Some("m³"));
        let common = d.groups.iter().find(|g| g.title == "Pset_WallCommon").expect("the common set");
        assert_eq!(common.lines[0].value, "Doğru");
    }
}
