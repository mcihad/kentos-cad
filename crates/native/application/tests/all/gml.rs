//! The GML codec (docs/adr/0545): what `gml_write` writes `gml_read` reads back, geometry by geometry; the axis order
//! of a system; the files that are not GML. The files of GDAL are the shared cases'
//! (`fixtures/commands/v1/file.import.gml.json`).

use kentos_geopackage::{Column, ColumnKind, Coord, Curve, Geometry, Value};
use kentos_native_application::gml_read::{self, Typed};
use kentos_native_application::gml_write::{self, Notes, OutColumn, OutType};

fn c(x: f64, y: f64) -> Coord {
    Coord::xy(x, y)
}

fn cz(x: f64, y: f64, z: f64) -> Coord {
    Coord {
        x,
        y,
        z: Some(z),
        m: None,
    }
}

fn line(p: &[(f64, f64)]) -> Curve {
    Curve::Line(p.iter().map(|&(x, y)| c(x, y)).collect())
}

fn through(g: Geometry, srid: Option<u32>) -> Geometry {
    let t = OutType {
        name: "t".into(),
        columns: vec![],
        features: vec![(Some(g), vec![])],
    };
    let text = gml_write::write(&[t], srid, None, &mut Notes::default());
    let mut parsed = gml_read::parse(&text, None, None).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert_eq!(parsed.tables.len(), 1);
    let row = parsed.tables.remove(0).rows.remove(0);
    row.geometry.expect("a geometry").unwrap_or_else(|e| panic!("{e}\n{text}"))
}

#[test]
fn every_geometry_reads_back_as_it_was_written() {
    let ring = line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 0.0)]);
    let arc = Curve::Arc(vec![c(0.0, 0.0), c(5.0, 5.0), c(10.0, 0.0)]);
    let compound = Curve::Compound(vec![
        line(&[(0.0, 0.0), (0.0, 10.0)]),
        Curve::Arc(vec![c(0.0, 10.0), c(5.0, 15.0), c(10.0, 10.0)]),
    ]);
    for g in [
        Geometry::Point(Some(c(1.5, 2.5))),
        Geometry::Point(Some(cz(1.5, 2.5, 7.0))),
        Geometry::MultiPoint(vec![c(1.0, 2.0), c(3.0, 4.0)]),
        Geometry::Curve(line(&[(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)])),
        Geometry::Curve(Curve::Line(vec![cz(0.0, 0.0, 1.0), cz(1.0, 1.0, 2.0)])),
        Geometry::Curve(arc.clone()),
        Geometry::Curve(compound.clone()),
        Geometry::MultiCurve(vec![line(&[(0.0, 0.0), (1.0, 1.0)]), arc.clone()]),
        Geometry::Polygon(vec![ring.clone(), line(&[(1.0, 1.0), (2.0, 1.0), (2.0, 2.0), (1.0, 1.0)])]),
        Geometry::Polygon(vec![Curve::Compound(vec![
            Curve::Arc(vec![c(0.0, 0.0), c(5.0, 5.0), c(10.0, 0.0)]),
            line(&[(10.0, 0.0), (0.0, 0.0)]),
        ])]),
        Geometry::MultiPolygon(vec![vec![ring.clone()], vec![ring]]),
        Geometry::Collection(vec![
            Geometry::Point(Some(c(1.0, 2.0))),
            Geometry::Curve(line(&[(0.0, 0.0), (1.0, 1.0)])),
        ]),
    ] {
        // Without a system, and with the ones whose axes go east first, north first and latitude first.
        for srid in [None, Some(32636), Some(5256), Some(4326)] {
            assert_eq!(through(g.clone(), srid), g, "{g:?} in {srid:?}");
        }
    }
}

#[test]
fn the_axes_go_in_the_order_of_the_system() {
    let g = Geometry::Point(Some(c(487001.25, 4420001.5)));
    let write = |srid| {
        let t = OutType {
            name: "t".into(),
            columns: vec![],
            features: vec![(Some(g.clone()), vec![])],
        };
        gml_write::write(&[t], srid, None, &mut Notes::default())
    };
    assert!(write(Some(5256)).contains("<gml:pos>4420001.5 487001.25</gml:pos>"));
    assert!(write(Some(32636)).contains("<gml:pos>487001.25 4420001.5</gml:pos>"));
    assert!(write(None).contains("<gml:pos>487001.25 4420001.5</gml:pos>"));
    // The same numbers under a name with no order of its own (EPSG:n) are east first.
    let text = "<c xmlns:gml=\"http://www.opengis.net/gml\"><gml:featureMember><t><g><gml:Point srsName=\"EPSG:5256\"><gml:coordinates>487001.25,4420001.5</gml:coordinates></gml:Point></g></t></gml:featureMember></c>";
    let parsed = gml_read::parse(text, None, None).expect("reads");
    let row = &parsed.tables[0].rows[0];
    assert_eq!(
        row.geometry.as_ref().expect("a geometry").as_ref().expect("reads"),
        &g
    );
    assert_eq!(parsed.srid, Some(5256));
}

#[test]
fn values_are_typed_by_the_schema_then_the_values() {
    let xsd = "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"><xs:element name=\"t\" type=\"ogr:t_Type\"/><xs:complexType name=\"t_Type\"><xs:sequence><xs:element name=\"a\" type=\"xs:string\"/><xs:element name=\"n\"><xs:simpleType><xs:restriction base=\"xs:integer\"/></xs:simpleType></xs:element><xs:element name=\"s\"><xs:simpleType><xs:restriction base=\"xs:string\"><xs:maxLength value=\"7\"/></xs:restriction></xs:simpleType></xs:element></xs:sequence></xs:complexType></xs:schema>";
    let text = "<c xmlns:gml=\"http://www.opengis.net/gml/3.2\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><gml:featureMember><t><a>007</a><n>007</n><s>x</s><b>12</b><d>2024-02-29</d></t></gml:featureMember></c>";
    // By the schema: 007 is text where the schema says text, the integer 7 where it says integer.
    let by_schema = gml_read::parse(text, Some(xsd), None).expect("reads");
    assert_eq!(by_schema.typed_by, Typed::Schema);
    let cols: Vec<(&str, &ColumnKind)> = by_schema.tables[0]
        .columns
        .iter()
        .map(|c: &Column| (c.name.as_str(), &c.kind))
        .collect();
    assert_eq!(cols[0], ("a", &ColumnKind::Text(None)));
    assert_eq!(cols[1], ("n", &ColumnKind::Integer));
    assert_eq!(cols[2], ("s", &ColumnKind::Text(Some(7))));
    // The properties the schema does not name are typed by their values.
    assert_eq!(cols[3], ("b", &ColumnKind::Integer));
    assert_eq!(cols[4], ("d", &ColumnKind::Date));
    let row = &by_schema.tables[0].rows[0];
    assert_eq!(row.values[0], Value::Text("007".into()));
    assert_eq!(row.values[1], Value::Integer(7));
    // By the values alone an identifier with a leading zero stays text.
    let by_values = gml_read::parse(text, None, None).expect("reads");
    assert_eq!(by_values.typed_by, Typed::Values);
    assert_eq!(by_values.tables[0].columns[0].kind, ColumnKind::Text(None));
    assert_eq!(by_values.tables[0].columns[1].kind, ColumnKind::Text(None));
    // A value that is not what the schema says makes the column text, said.
    let bad = "<c xmlns:gml=\"http://www.opengis.net/gml/3.2\"><gml:featureMember><t><a>x</a><n>yedi</n><s>x</s></t></gml:featureMember></c>";
    let parsed = gml_read::parse(bad, Some(xsd), None).expect("reads");
    assert_eq!(parsed.tables[0].columns[1].kind, ColumnKind::Text(None));
    assert!(parsed.losses.iter().any(|l| l.contains("“n”")));
}

#[test]
fn what_is_no_gml_is_refused() {
    assert!(gml_read::parse("PK\u{3}\u{4} zip", None, None).is_err());
    assert!(gml_read::parse("<a><b></a>", None, None).is_err());
    assert!(gml_read::parse("<kitap><baslik>x</baslik></kitap>", None, None).is_err());
    // A collection with no member is a collection.
    assert!(gml_read::parse("<gml:FeatureCollection xmlns:gml=\"http://www.opengis.net/gml/3.2\"/>", None, None)
        .is_ok_and(|p| p.tables.is_empty()));
    // Elements nested deeper than the guard are refused before they are parsed.
    let deep = format!("{}{}", "<a>".repeat(300), "</a>".repeat(300));
    assert!(gml_read::parse(&deep, None, None).is_err());
    // A position that is no number, a list that is not whole positions, an arc that is not an odd count of points.
    for body in [
        "<gml:LineString><gml:posList>1 2 x 4</gml:posList></gml:LineString>",
        "<gml:LineString><gml:posList srsDimension=\"2\">1 2 3</gml:posList></gml:LineString>",
        "<gml:Curve><gml:segments><gml:ArcString><gml:posList>0 0 1 1</gml:posList></gml:ArcString></gml:segments></gml:Curve>",
        "<gml:Solid/>",
    ] {
        let text = format!(
            "<c xmlns:gml=\"http://www.opengis.net/gml/3.2\"><gml:featureMember><t><g>{body}</g></t></gml:featureMember></c>"
        );
        let parsed = gml_read::parse(&text, None, None).expect("a file; the feature is the trouble");
        assert!(
            parsed.tables[0].rows[0].geometry.as_ref().is_some_and(Result::is_err),
            "{body}"
        );
    }
}

#[test]
fn xml_names_are_made_of_any_text() {
    assert_eq!(gml_write::xml_name("Yol kenarı"), "Yol_kenarı");
    assert_eq!(gml_write::xml_name("1. kat"), "_1._kat");
    assert_eq!(gml_write::xml_name("a<b&c"), "a_b_c");
    assert_eq!(gml_write::xml_name(""), "_");
}

#[test]
fn the_envelope_holds_an_arcs_bulge() {
    // An arc of the circle about the origin of radius 5, from (4,3) through (3,4) to (-3,4), passes the top (0,5),
    // which none of its three points is.
    let t = OutType {
        name: "t".into(),
        columns: vec![OutColumn {
            name: "a".into(),
            kind: ColumnKind::Text(None),
        }],
        features: vec![(
            Some(Geometry::Curve(Curve::Arc(vec![c(4.0, 3.0), c(3.0, 4.0), c(-3.0, 4.0)]))),
            vec![Value::Text("x".into())],
        )],
    };
    let text = gml_write::write(&[t], None, None, &mut Notes::default());
    assert!(
        text.contains("<gml:lowerCorner>-3 3</gml:lowerCorner><gml:upperCorner>4 5</gml:upperCorner>"),
        "{text}"
    );
}

#[test]
fn the_schema_location_is_a_uri_reference() {
    // The location list is split at white space: a file name with a space is percent-encoded.
    let t = OutType {
        name: "t".into(),
        columns: vec![],
        features: vec![(Some(Geometry::Point(Some(c(1.0, 2.0)))), vec![])],
    };
    let text = gml_write::write(&[t], None, Some("Örnek pafta.xsd"), &mut Notes::default());
    assert!(
        text.contains("xsi:schemaLocation=\"http://ogr.maptools.org/ Örnek%20pafta.xsd\""),
        "{text}"
    );
}
