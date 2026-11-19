//! The chosen object's details for the viewer's properties panel (feature `view`; docs/adr/8061 §5 "Özellikler",
//! docs/adr/8073 §2): its attributes, its type, its material, its storey, its material layers and how a usage lays them
//! (docs/adr/8166), its section (its own or its type's profile set, docs/adr/8167), the storeys it spans besides its
//! own (docs/adr/8168), then its property and quantity sets
//! (its own, then its type's) as the file writes them, each value
//! with its unit: its own, else the project's for the measure it is written with. Nothing is converted and nothing is
//! measured from the triangles (CLAUDE.md §23.3).

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
        // The storeys it rises into besides its own (docs/adr/8168; `IfcRelReferencedInSpatialStructure`).
        for storey in self.spanned_storeys(schema, object) {
            add("Uzandığı kat", Some(storey));
        }
        for group in self.groups_of(schema, object) {
            add("Grup", Some(group));
        }
        add("Malzeme", self.materials(bytes, object).ok().filter(|m| !m.is_empty()).map(|m| m.join("; ")));
        add("Kayıt", Some(format!("#{}", o.step)));
        let mut groups = vec![own];
        // Its material layers (docs/adr/8166): how a usage lays them, then each in order, its thickness in the
        // project's length unit.
        if let Some(list) = self.layers(bytes, object)? {
            let length = self.summary.units.length.as_ref().map(|u| symbol(u.prefix.as_deref(), &u.name));
            let mut lines = Vec::new();
            if let Some((direction, sense, offset)) = &list.usage {
                let value = laid(direction, sense);
                lines.push(Line { name: "Diziliş".to_owned(), value, unit: None, numeric: false });
                lines.push(Line {
                    name: "Taban çizgisi".to_owned(),
                    value: offset.to_string(),
                    unit: length.clone(),
                    numeric: true,
                });
            }
            let mut materials = Vec::new();
            for (i, (name, material, thickness)) in list.layers.iter().enumerate() {
                let name = format!("{}. {name}", i + 1);
                if let Some(m) = material {
                    materials.push(Line { name: name.clone(), value: m.clone(), unit: None, numeric: false });
                }
                lines.push(Line { name, value: thickness.to_string(), unit: length.clone(), numeric: true });
            }
            groups.push(Group { title: "Malzeme katmanları".to_owned(), lines });
            // What each layer is made of, apart: a name and a material side by side would not fit the panel.
            if !materials.is_empty() {
                groups.push(Group { title: "Katmanların malzemesi".to_owned(), lines: materials });
            }
        }
        // Its section (docs/adr/8167): its own material profile set's, else its type's (a column type's, a beam
        // type's).
        let (sections, from_type) = match self.profiles(bytes, object)? {
            own if !own.is_empty() => (own, false),
            _ => match o.type_object {
                Some(t) => (self.profiles(bytes, t)?, true),
                None => (Vec::new(), false),
            },
        };
        for p in sections {
            let length = self.summary.units.length.as_ref().map(|u| symbol(u.prefix.as_deref(), &u.name));
            let mut lines = Vec::new();
            if let Some(n) = p.name.filter(|n| !n.trim().is_empty()) {
                lines.push(Line { name: "Ad".to_owned(), value: n, unit: None, numeric: false });
            }
            let shape = match p.class.to_ascii_uppercase().as_str() {
                "IFCRECTANGLEPROFILEDEF" => "Dikdörtgen".to_owned(),
                "IFCCIRCLEPROFILEDEF" => "Daire".to_owned(),
                "IFCARBITRARYCLOSEDPROFILEDEF" => "Serbest biçim".to_owned(),
                _ => p.class.clone(),
            };
            lines.push(Line { name: "Biçim".to_owned(), value: shape, unit: None, numeric: false });
            for (key, v) in &p.measures {
                lines.push(Line { name: (*key).to_owned(), value: v.to_string(), unit: length.clone(), numeric: true });
            }
            if let Some(m) = p.material {
                lines.push(Line { name: "Malzeme".to_owned(), value: m, unit: None, numeric: false });
            }
            let title = if from_type { "Kesit (tipten)" } else { "Kesit" };
            groups.push(Group { title: title.to_owned(), lines });
        }
        for set in self.property_sets(bytes, object)? {
            let title = if set.from_type { format!("{} (tipten)", set.name) } else { set.name.clone() };
            groups.push(Group { title, lines: set.values.iter().map(|v| line(v, &self.summary.units)).collect() });
        }
        Ok(Details { groups })
    }

    /// The groups the object is assigned to (`IfcRelAssignsToGroup`: a zone, such as a unit or a fire compartment,
    /// docs/adr/8088), each by its long name, else its name, else its object type, else its class.
    fn groups_of(&self, schema: SchemaVersion, object: usize) -> Vec<String> {
        let Some(o) = self.objects.get(object) else { return Vec::new() };
        self.relations_of(o.step)
            .iter()
            .map(|&r| &self.relations[r])
            .filter(|r| schema.is_subtype_of(&r.class, "IfcRelAssignsToGroup") && r.related.contains(&o.step))
            .filter_map(|r| self.object(r.relating?))
            .map(|g| g.long_name.clone().or_else(|| g.name.clone()).or_else(|| g.object_type.clone()).unwrap_or_else(|| g.class.clone()))
            .collect()
    }

    /// The storeys that reference the object without containing it (`IfcRelReferencedInSpatialStructure`,
    /// docs/adr/8168: an element spanning storeys), each by its name, else its class.
    fn spanned_storeys(&self, schema: SchemaVersion, object: usize) -> Vec<String> {
        let Some(o) = self.objects.get(object) else { return Vec::new() };
        self.relations_of(o.step)
            .iter()
            .map(|&r| &self.relations[r])
            .filter(|r| r.related.contains(&o.step))
            .filter(|r| schema.is_subtype_of(&r.class, "IfcRelReferencedInSpatialStructure"))
            .filter_map(|r| self.object(r.relating?))
            .filter(|s| schema.is_subtype_of(&s.class, "IfcBuildingStorey"))
            .map(|s| s.name.clone().unwrap_or_else(|| s.class.clone()))
            .collect()
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

/// How a usage lays its set, in words (docs/adr/8166): along the object's x, y or z axis (AXIS1–3), in its plus or
/// minus direction from the base line (“z ekseni, eksi yön”); a word the file writes otherwise stays as written.
fn laid(direction: &str, sense: &str) -> String {
    let axis = match direction {
        "AXIS1" => "x",
        "AXIS2" => "y",
        "AXIS3" => "z",
        other => other,
    };
    let sign = match sense {
        "POSITIVE" => "artı",
        "NEGATIVE" => "eksi",
        other => other,
    };
    format!("{axis} ekseni, {sign} yön")
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
        // Its own lines, its material layers and their materials (docs/adr/8166), its sets.
        assert_eq!(d.groups.len(), 3 + sets.len());
        let titles = (d.groups[1].title.as_str(), d.groups[2].title.as_str());
        assert_eq!(titles, ("Malzeme katmanları", "Katmanların malzemesi"));
        for (g, s) in d.groups[3..].iter().zip(&sets) {
            assert!(g.title.starts_with(&s.name));
            assert_eq!(g.lines.len(), s.values.len());
        }
        // Its layer set by its name: four layers since the wall types (docs/adr/8105; the fixture written again with
        // them, docs/adr/8117), one (Tuğla) before.
        assert_eq!(value("Malzeme"), Some("Dış duvar 25"), "its layer set");
        // A quantity is a number as written with the project's unit for its measure.
        let quantities = d.groups.iter().find(|g| g.title == "Qto_WallBaseQuantities").expect("the wall's quantities");
        let at = |name: &str| quantities.lines.iter().find(|l| l.name == name).expect(name);
        assert_eq!((at("Length").value.as_str(), at("Length").unit.as_deref(), at("Length").numeric), ("12", Some("m"), true));
        assert_eq!((at("NetSideArea").value.as_str(), at("NetSideArea").unit.as_deref()), ("28.74", Some("m²")));
        assert_eq!(at("NetVolume").unit.as_deref(), Some("m³"));
        let common = d.groups.iter().find(|g| g.title == "Pset_WallCommon").expect("the common set");
        assert_eq!(common.lines[0].value, "Doğru");
        assert_eq!(value("Grup"), None, "a wall is in no group");
    }

    /// A layered slab's details (docs/adr/8166): how its usage lays its set — along its z axis, downwards, from its
    /// top — and its layers from the top with their thicknesses in metres, the structure with its material; a wall's
    /// set has no usage and lists its layers from its left face.
    #[test]
    fn a_layered_slab_s_details_list_its_layers_and_how_they_are_laid() {
        let (_, bytes) = WRITTEN[0];
        let m = read_ok(bytes);
        let floors: Vec<usize> = (0..m.objects.len()).filter(|&i| m.objects[i].class == "IfcSlab").collect();
        let lines = |i: usize| -> Vec<(String, String, Option<String>)> {
            let d = m.details(bytes, i).expect("the details");
            let g = d.groups.iter().find(|g| g.title == "Malzeme katmanları").expect("its layers");
            g.lines.iter().map(|l| (l.name.clone(), l.value.clone(), l.unit.clone())).collect()
        };
        let m_ = || Some("m".to_owned());
        let ground = floors
            .iter()
            .map(|&i| lines(i))
            .find(|l| l.len() == 7)
            .expect("the ground floor: two lines of its usage and five layers");
        assert_eq!(
            ground,
            [
                ("Diziliş".to_owned(), "z ekseni, eksi yön".to_owned(), None),
                ("Taban çizgisi".to_owned(), "0".to_owned(), m_()),
                ("1. Kaplama".to_owned(), "0.02".to_owned(), m_()),
                ("2. Şap".to_owned(), "0.05".to_owned(), m_()),
                ("3. Isı yalıtımı".to_owned(), "0.05".to_owned(), m_()),
                ("4. Su yalıtımı".to_owned(), "0.01".to_owned(), m_()),
                ("5. Taşıyıcı".to_owned(), "0.15".to_owned(), m_()),
            ]
        );
        // Their materials apart: the structure's alone is given.
        let ground_at = *floors.iter().find(|&&i| lines(i).len() == 7).expect("the ground floor");
        let d = m.details(bytes, ground_at).expect("the details");
        let made = d.groups.iter().find(|g| g.title == "Katmanların malzemesi").expect("their materials");
        let made: Vec<(&str, &str)> = made.lines.iter().map(|l| (l.name.as_str(), l.value.as_str())).collect();
        assert_eq!(made, [("5. Taşıyıcı", "Betonarme")]);
        let wall = m.objects.iter().position(|o| o.class == "IfcWall").expect("a wall");
        let w = lines(wall);
        assert!(w.iter().all(|(n, ..)| n != "Diziliş"), "{w:?}");
        assert_eq!(w.len(), 4, "the style's four layers: {w:?}");
    }

    /// A typed column's details (docs/adr/8167): its type's section, the profile set the type holds — its name, a
    /// rectangle, its XDim and YDim in metres and its material.
    #[test]
    fn a_typed_column_shows_its_type_s_section() {
        let bytes: &[u8] = include_bytes!("../../../../../fixtures/arch/v1/ifc/kesit-binasi-eleman-tipli.ifc");
        let m = read_ok(bytes);
        let column = m.objects.iter().position(|o| o.class == "IfcColumn").expect("a column");
        let d = m.details(bytes, column).expect("the details");
        let g = d.groups.iter().find(|g| g.title == "Kesit (tipten)").expect("its type's section");
        let lines: Vec<(&str, &str, Option<&str>)> =
            g.lines.iter().map(|l| (l.name.as_str(), l.value.as_str(), l.unit.as_deref())).collect();
        assert_eq!(
            lines,
            [
                ("Ad", "30×30", None),
                ("Biçim", "Dikdörtgen", None),
                ("XDim", "0.3", Some("m")),
                ("YDim", "0.3", Some("m")),
                ("Malzeme", "Betonarme", None),
            ]
        );
        // A wall has no section.
        let wall = m.objects.iter().position(|o| o.class == "IfcWall").expect("a wall");
        assert!(m.details(bytes, wall).expect("the details").groups.iter().all(|g| !g.title.starts_with("Kesit")));
    }

    /// A space of the file written with fire compartments (docs/adr/8088) says its compartment's zone.
    #[test]
    fn a_space_says_the_zones_it_is_grouped_in() {
        let bytes: &[u8] = include_bytes!("../../../../../fixtures/arch/v1/ifc/kesit-binasi-kompartiman.ifc");
        let m = read_ok(bytes);
        // Each space's groups, one line each: its compartment's long name among them.
        let zoned: Vec<Vec<String>> = m
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.class == "IfcSpace")
            .map(|(i, _)| {
                let d = m.details(bytes, i).expect("the details");
                d.groups[0].lines.iter().filter(|l| l.name == "Grup").map(|l| l.value.clone()).collect()
            })
            .collect();
        assert_eq!(zoned.len(), 10, "{zoned:?}");
        let count = |n: &str| zoned.iter().filter(|g| g.iter().any(|x| x == n)).count();
        assert_eq!((count("Yangın kompartımanı 1"), count("Yangın kompartımanı 2")), (5, 5), "{zoned:?}");
    }
}
