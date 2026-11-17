//! The profile's mandatory rows as an Information Delivery Specification (buildingSMART IDS 1.0; docs/adr/8074 §5;
//! plan/ifc-uyum.md IFC-08): each EK-6 and EK-7 table with mandatory rows is a specification for its class in IFC 4.3
//! (`IFC4X3_ADD2`), applicable when the file has the class (EK-5 §5.1: a class is asked when the design has it; EK-7's
//! project, people, system and context, Tablo 7.1–7.8, are in every model and must be there: K51), each
//! mandatory row a requirement: an attribute, a property of its set, a quantity, the material, the group it is part
//! of. The file is data/temel-2026.ids, written from the catalog (a test keeps it current): a second checker opens it
//! beside KentOS's own (`crate::required`, the form's 9th question), so that what both read is the same list.
//!
//! - **Values:** a predefined type is required and any of its enumeration's but `NOTDEFINED` (`crate::required`'s
//!   rule: an undefined type is no type); a property's data type is the IFC 4.3 template's, a quantity's its kind's
//!   measure; a national set's (`TREpys_…`, EK-6 §6.5) is not the schema's, so none is said. Nothing else is asked of a
//!   value: the regulation asks that it be given.
//! - **Groups:** “IfcRelAssignsToGroup>IfcSystem” is a part-of facet whose group is IfcSystem or one of its subtypes.
//! - **Rows asked under a condition:** a user-defined role (EK-7 Tablo 7.5) is given exactly when the role is
//!   USERDEFINED (the schema's own rule, `IfcActorRole` WR1; K48): it is a specification of its own, applicable to the
//!   roles whose `Role` is USERDEFINED.
//!
//! Geometry, overlaps, names' patterns and a file's agreement with its source are not IDS's (its manual's scope): they
//! are the form's own questions.

use std::collections::BTreeMap;

use kentos_digital_project::Level;

use crate::catalog::{self, Kind, Requirement};
use crate::catalog_schema::dictionary;

/// IDS 1.0's namespace and schema.
const NAMESPACE: &str = "http://standards.buildingsmart.org/IDS";
const SCHEMA: &str = "http://standards.buildingsmart.org/IDS/1.0/ids.xsd";

/// The IFC version every specification is for (EK-5 §5.4).
const VERSION: &str = "IFC4X3_ADD2";

/// The profile's IDS text.
pub fn write() -> String {
    let c = catalog::base();
    let p = &c.profile;
    let mut tables: BTreeMap<(u32, u32, String), Vec<&Requirement>> = BTreeMap::new();
    for r in c.requirements.iter().filter(|r| r.level == Level::Mandatory) {
        tables.entry((order(&r.table), table_row(&r.table), r.class.clone())).or_default().push(r);
    }
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<ids xmlns=\"{NAMESPACE}\" xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"{NAMESPACE} {SCHEMA}\">\n"
    ));
    out.push_str("  <info>\n");
    out.push_str(&format!("    <title>{}: {} (EK-6, EK-7)</title>\n", esc(&p.act), esc(&p.name)));
    out.push_str(&format!("    <version>{}</version>\n", esc(&p.version)));
    out.push_str(&format!(
        "    <description>RG {}: zorunlu IFC sınıflarının zorunlu öznitelik ve özellikleri (Md. 12, 13; EK-6, EK-7). \
         KentOS'un kural kataloğundan ({}) yazılmıştır.</description>\n",
        esc(&p.gazette),
        esc(&p.id)
    ));
    out.push_str("    <purpose>Ruhsat modelinin teslimi</purpose>\n");
    out.push_str("  </info>\n");
    out.push_str("  <specifications>\n");
    for ((_, _, class), rows) in &tables {
        let Some(t) = c.tables.iter().find(|t| t.table == rows[0].table && t.class == *class) else { continue };
        let plain: Vec<&Requirement> = rows.iter().copied().filter(|r| condition(r).is_none()).collect();
        if !plain.is_empty() {
            out.push_str(&specification(t, &plain, None));
        }
        let mut conditions: Vec<(&str, &str)> = rows.iter().filter_map(|r| condition(r)).collect();
        conditions.dedup();
        for when in conditions {
            let under: Vec<&Requirement> = rows.iter().copied().filter(|r| condition(r) == Some(when)).collect();
            out.push_str(&specification(t, &under, Some(when)));
        }
    }
    out.push_str("  </specifications>\n");
    out.push_str("</ids>\n");
    out
}

/// The attribute and its value a row is asked under, when it is asked only so (K48: a user-defined role when the role is
/// USERDEFINED).
fn condition(r: &Requirement) -> Option<(&'static str, &'static str)> {
    (r.class == "IfcActorRole" && r.kind == Kind::Attribute && r.name.trim_start_matches("Attribute:").trim() == "UserDefinedRole")
        .then_some(("Role", "USERDEFINED"))
}

/// A table's specification for `rows`, applicable to its class (and, `when` given, to its objects whose attribute has
/// that value).
fn specification(t: &catalog::Table, rows: &[&Requirement], when: Option<(&str, &str)>) -> String {
    let pages = t.pages.iter().map(u32::to_string).collect::<Vec<_>>().join("–");
    let annex = t.table.split('.').next().unwrap_or("");
    let mut name = format!("EK-{annex} Tablo {} {}{}", t.table, t.class, t.tr.as_ref().map_or(String::new(), |tr| format!(" ({tr})")));
    let mut id = t.id.clone();
    if let Some((attribute, value)) = when {
        name.push_str(&format!(", {attribute} {value} iken"));
        id.push_str(&format!("/{attribute}={value}"));
    }
    let mut out = format!(
        "    <specification name=\"{}\" ifcVersion=\"{VERSION}\" identifier=\"{}\" description=\"{}\" \
         instructions=\"Dijital Proje Yön. EK-{annex} Tablo {} · PDF s. {pages}\">\n",
        esc(&name),
        esc(&id),
        esc(&t.title),
        esc(&t.table)
    );
    // K51: EK-7's classes but the survey annotation (Tablo 7.9) are in every model.
    let least = u8::from(t.table.starts_with("7.") && t.class != "IfcAnnotation" && when.is_none());
    out.push_str(&format!("      <applicability minOccurs=\"{least}\" maxOccurs=\"unbounded\">\n"));
    match entities(&t.class).as_slice() {
        [one] => out.push_str(&format!(
            "        <entity>\n          <name>\n            <simpleValue>{one}</simpleValue>\n          </name>\n        </entity>\n"
        )),
        many => {
            out.push_str("        <entity>\n          <name>\n            <xs:restriction base=\"xs:string\">\n");
            for e in many {
                out.push_str(&format!("              <xs:enumeration value=\"{e}\"/>\n"));
            }
            out.push_str("            </xs:restriction>\n          </name>\n        </entity>\n");
        }
    }
    if let Some((attribute, value)) = when {
        out.push_str(&format!(
            "        <attribute>\n          <name>\n            <simpleValue>{attribute}</simpleValue>\n          </name>\n          <value>\n            <simpleValue>{value}</simpleValue>\n          </value>\n        </attribute>\n"
        ));
    }
    out.push_str("      </applicability>\n");
    out.push_str("      <requirements>\n");
    // IDS's facet order: part of, attribute, property, material.
    for kind in [Kind::Relation, Kind::Attribute, Kind::Property, Kind::Quantity, Kind::Material] {
        for r in rows.iter().filter(|r| r.kind == kind) {
            out.push_str(&facet(r));
        }
    }
    out.push_str("      </requirements>\n");
    out.push_str("    </specification>\n");
    out
}

/// The entities a table's class is written as: itself, and a map conversion its scaled kind too (a local system's,
/// docs/adr/8072 §3; IDS names an entity exactly, not its subtypes, and other subtypes are not the class: a context's
/// sub-contexts derive their attributes).
fn entities(class: &str) -> Vec<String> {
    let mut out = vec![class.to_ascii_uppercase()];
    if class == "IfcMapConversion" {
        out.push("IFCMAPCONVERSIONSCALED".to_owned());
    }
    out
}

/// A table's annex and its number in it (`6.12` → 6, 12), for the file's order.
fn order(table: &str) -> u32 {
    table.split('.').next().and_then(|a| a.parse().ok()).unwrap_or(0)
}

fn table_row(table: &str) -> u32 {
    table.split('.').nth(1).and_then(|a| a.parse().ok()).unwrap_or(0)
}

/// A row's requirement facet, its instructions its table's row and its Turkish name.
fn facet(r: &Requirement) -> String {
    // The building's heights are asked "sadece yüksekliğin sabit olduğu durumda" (Tablo 6.7; docs/adr/8076 §4): IDS 1.0
    // has no condition, so its words say it to the reader of a failure.
    let only_constant = r.class == "IfcBuilding"
        && r.set.as_deref() == Some("Qto_BuildingBaseQuantities")
        && matches!(r.name.as_str(), "Height" | "EavesHeight");
    let condition = if only_constant { "; yalnız katların yüksekliği aynıyken sorulur" } else { "" };
    let say = esc(&format!("{} (Tablo {} satır {}{condition})", r.tr, r.table, r.row));
    match r.kind {
        Kind::Attribute => {
            let name = r.name.trim_start_matches("Attribute:").trim();
            let value = if name == "PredefinedType" { defined_types(&r.class) } else { None };
            let mut s = format!(
                "        <attribute cardinality=\"required\" instructions=\"{say}\">\n          <name>\n            <simpleValue>{}</simpleValue>\n          </name>\n",
                esc(name)
            );
            if let Some(values) = value {
                s.push_str("          <value>\n            <xs:restriction base=\"xs:string\">\n");
                for v in values {
                    s.push_str(&format!("              <xs:enumeration value=\"{}\"/>\n", esc(&v)));
                }
                s.push_str("            </xs:restriction>\n          </value>\n");
            }
            s.push_str("        </attribute>\n");
            s
        }
        Kind::Property | Kind::Quantity => {
            let set = r.set.as_deref().unwrap_or("");
            let data = data_type(set, &r.name).map_or(String::new(), |d| format!(" dataType=\"{d}\""));
            format!(
                "        <property cardinality=\"required\"{data} instructions=\"{say}\">\n          <propertySet>\n            <simpleValue>{}</simpleValue>\n          </propertySet>\n          <baseName>\n            <simpleValue>{}</simpleValue>\n          </baseName>\n        </property>\n",
                esc(set),
                esc(&r.name)
            )
        }
        Kind::Material => format!("        <material cardinality=\"required\" instructions=\"{say}\"/>\n"),
        Kind::Relation => {
            let (relation, group) = r.path().split_once('>').map(|(a, b)| (a.to_owned(), b.to_owned())).unwrap_or_default();
            let mut s = format!(
                "        <partOf relation=\"{}\" cardinality=\"required\" instructions=\"{say}\">\n          <entity>\n            <name>\n",
                relation.to_ascii_uppercase()
            );
            let groups = subtypes(&group);
            s.push_str("              <xs:restriction base=\"xs:string\">\n");
            for g in groups {
                s.push_str(&format!("                <xs:enumeration value=\"{g}\"/>\n"));
            }
            s.push_str("              </xs:restriction>\n            </name>\n          </entity>\n        </partOf>\n");
            s
        }
    }
}

/// The class's predefined types but `NOTDEFINED`, as IFC 4.3 spells them; none for a class the dictionary has no
/// enumeration of.
fn defined_types(class: &str) -> Option<Vec<String>> {
    let d = dictionary();
    let def = d.classes.iter().find(|(k, _)| k.eq_ignore_ascii_case(class)).map(|(_, v)| v)?;
    let values = d.enums.get(def.predefined.as_deref()?)?;
    Some(values.iter().filter(|v| *v != "NOTDEFINED").cloned().collect())
}

/// A set's item's data type, as IDS spells it (`IFCLABEL`): the template's measure, a quantity's kind's; none for a
/// national set or an item the template does not have.
fn data_type(set: &str, name: &str) -> Option<String> {
    let (kind, measure) = dictionary().sets.get(set)?.item(name)?;
    let measure = measure.or_else(|| {
        Some(
            match kind.as_str() {
                "Q_LENGTH" => "IfcLengthMeasure",
                "Q_AREA" => "IfcAreaMeasure",
                "Q_VOLUME" => "IfcVolumeMeasure",
                "Q_COUNT" => "IfcCountMeasure",
                "Q_WEIGHT" => "IfcMassMeasure",
                "Q_TIME" => "IfcTimeMeasure",
                _ => return None,
            }
            .to_owned(),
        )
    })?;
    Some(measure.to_ascii_uppercase())
}

/// A class and its subtypes in the dictionary, as STEP spells them, in order.
fn subtypes(class: &str) -> Vec<String> {
    let d = dictionary();
    let mut out: Vec<String> = d
        .classes
        .iter()
        .filter(|(k, v)| k.eq_ignore_ascii_case(class) || v.supertypes.iter().any(|s| s.eq_ignore_ascii_case(class)))
        .map(|(k, _)| k.to_ascii_uppercase())
        .collect();
    if out.is_empty() {
        out.push(class.to_ascii_uppercase());
    }
    out.sort();
    out
}

/// Text as XML writes it in an element or an attribute.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// data/temel-2026.ids is what the catalog writes: `KENTOS_WRITE_IDS=1` writes it, else it is compared.
    #[test]
    fn the_profile_s_ids_is_current() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/temel-2026.ids");
        let text = write();
        if std::env::var_os("KENTOS_WRITE_IDS").is_some() {
            std::fs::write(&path, &text).expect("writes the IDS");
        } else {
            let kept = std::fs::read_to_string(&path).unwrap_or_default();
            assert!(kept == text, "{}: not current; KENTOS_WRITE_IDS=1 writes it, read the difference", path.display());
        }
    }

    /// Every mandatory row is a requirement of its table's specification, once; a wall's type any but NOTDEFINED, its
    /// fire rating a label of Pset_WallCommon.
    #[test]
    fn every_mandatory_row_is_one_requirement() {
        let text = write();
        let rows = catalog::base().requirements.iter().filter(|r| r.level == Level::Mandatory).count();
        assert_eq!(text.matches(" instructions=\"").count() - text.matches("<specification ").count(), rows);
        let wall = text.split("<specification ").find(|s| s.contains("<simpleValue>IFCWALL</simpleValue>")).expect("the wall's");
        assert!(wall.contains("<xs:enumeration value=\"STANDARD\"/>") && !wall.contains("value=\"NOTDEFINED\""), "{wall}");
        assert!(wall.contains("dataType=\"IFCLABEL\"") && wall.contains("<simpleValue>FireRating</simpleValue>"), "{wall}");
        assert_eq!(esc("a<b & \"c\""), "a&lt;b &amp; &quot;c&quot;");
        // K48: the user-defined role in a specification of its own, for the roles that are USERDEFINED.
        let role = text.split("<specification ").find(|s| s.contains("/Role=USERDEFINED")).expect("the role's");
        assert!(role.contains("<simpleValue>Role</simpleValue>") && role.contains("<simpleValue>UserDefinedRole</simpleValue>"));
        assert!(role.contains("minOccurs=\"0\""), "asked of the USERDEFINED roles there are");
        // K51: a person is in every model; a survey annotation only where there is one.
        let person = text.split("<specification ").find(|s| s.contains("<simpleValue>IFCPERSON</simpleValue>")).expect("the person's");
        assert!(person.contains("minOccurs=\"1\""), "{person}");
        let note = text.split("<specification ").find(|s| s.contains("<simpleValue>IFCANNOTATION</simpleValue>")).expect("the annotation's");
        assert!(note.contains("minOccurs=\"0\""), "{note}");
        // A local system's scaled conversion is a map conversion.
        assert!(text.contains("<xs:enumeration value=\"IFCMAPCONVERSIONSCALED\"/>"));
    }
}
