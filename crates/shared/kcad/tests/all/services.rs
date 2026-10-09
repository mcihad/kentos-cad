//! Map services in the project file (docs/adr/0208 §2, docs/specs/kcad-v2.md
//! §6.1, §6.4.7, §6.5): document schema 32 is written only for a drawing with
//! a layer drawn from a service, a layer whose objects came from a source, or
//! the project's connections; every other drawing keeps its schema and its
//! bytes. They come back bit for bit, flags and lists only when set; the
//! writer refuses what the reader would; the reader's errors name their
//! places.

use kentos_kcad::Code;
use kentos_kcad::contracts::{
    AuthKind, DocumentSnapshotV2, FeatureFeed, FeedKind, LayerNode, LayerNodeType,
    ServiceConnection, ServiceKind, ServiceLayer,
};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says (one byte below 24, else the byte after 0x18).
fn schema(file: &[u8]) -> u8 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    if payload[at] == 0x18 {
        payload[at + 1]
    } else {
        payload[at]
    }
}

fn osm() -> ServiceLayer {
    ServiceLayer {
        kind: ServiceKind::Xyz,
        url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".into(),
        layers: Vec::new(),
        style: None,
        format: None,
        srid: None,
        grid: None,
        matrix_set: None,
        template: None,
        tile_size: None,
        min_zoom: None,
        max_zoom: Some(19),
        subdomains: Vec::new(),
        y_flip: false,
        transparent: false,
        version: None,
        params: Vec::new(),
        dynamic: false,
        attribution: Some("© OpenStreetMap katkıcıları".into()),
        opacity: None,
        connection: None,
        preset: Some("osm-standard".into()),
        bbox: None,
    }
}

fn wfs() -> FeatureFeed {
    FeatureFeed {
        kind: FeedKind::Wfs,
        url: "https://cbs.example.gov.tr/geoserver/wfs".into(),
        name: Some("tkgm:parsel".into()),
        srid: Some(5256),
        filter: None,
        bbox: None,
        limit: None,
        version: None,
        key: None,
        connection: None,
        fetched: None,
    }
}

fn connection(id: &str) -> ServiceConnection {
    ServiceConnection {
        id: id.into(),
        name: "HGM ATLAS".into(),
        origin: "https://atlas.harita.gov.tr".into(),
        auth: AuthKind::Query,
        names: vec!["apikey".into()],
        token_url: None,
        scope: None,
    }
}

/// minimal.json (one point on layer "0", schema 2) with a second layer, "altlik", given `service` and `feed`.
fn drawing(service: Option<ServiceLayer>, feed: Option<FeatureFeed>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    let mut node: LayerNode = doc.layers[0].clone();
    node.id = "altlik".into();
    node.name = "Altlık".into();
    node.service = service;
    node.feed = feed;
    doc.layers.push(node);
    doc
}

#[test]
fn only_a_drawing_with_a_service_a_feed_or_a_connection_is_schema_32() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("services.kcad");
    assert_eq!(schema(&data), 32);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    assert_eq!(of(&drawing(None, None)), 2);
    assert_eq!(of(&drawing(Some(osm()), None)), 32);
    assert_eq!(of(&drawing(None, Some(wfs()))), 32);
    let mut connected = content("minimal.json");
    connected.settings.connections = vec![connection("hgm")];
    assert_eq!(of(&connected), 32);
    // The newest drawings before it keep their schemas.
    assert_eq!(of(&content("pointclouds.json")), 31);
    assert_eq!(of(&content("layer-fields.json")), 26);
}

#[test]
fn services_come_back_bit_for_bit() {
    let doc = content("services.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let find = |id: &str| -> LayerNode {
        fn walk<'a>(nodes: &'a [LayerNode], id: &str) -> Option<&'a LayerNode> {
            nodes
                .iter()
                .find_map(|n| (n.id == id).then_some(n).or_else(|| walk(&n.children, id)))
        }
        walk(&again.layers, id).cloned().expect(id)
    };
    let orto = find("orto").service.expect("a service");
    assert_eq!(orto.kind, ServiceKind::Wmts);
    let grid = orto.grid.expect("a grid");
    assert_eq!((grid.srid, grid.matrices.len()), (5256, 3));
    assert_eq!(grid.matrices[2].matrix_width, 2400);
    assert_eq!(orto.matrix_set.as_deref(), Some("TUREF_TM33"));
    let imar = find("imar").service.expect("a service");
    assert!(imar.transparent && !imar.y_flip && !imar.dynamic);
    assert_eq!(imar.params.len(), 2);
    assert_eq!(imar.opacity, Some(0.75));
    let tms = find("tms").service.expect("a service");
    assert!(tms.y_flip);
    assert_eq!(
        (tms.tile_size, tms.min_zoom, tms.max_zoom),
        (Some(512), Some(5), Some(20))
    );
    let google = find("uydu").service.expect("a service");
    assert_eq!(
        (google.kind, google.url.as_str()),
        (ServiceKind::Google, "")
    );
    assert_eq!(google.style.as_deref(), Some("satellite"));
    let parsel = find("parsel").feed.expect("a feed");
    assert_eq!(
        parsel.bbox,
        Some([499_900.0, 4_399_900.0, 500_100.5, 4_400_100.25])
    );
    assert_eq!(parsel.limit, Some(5000));
    assert_eq!(find("durak").feed.expect("a feed").kind, FeedKind::Geojson);
    let c = &again.settings.connections;
    assert_eq!(c.len(), 6);
    assert_eq!(c[3].auth, AuthKind::Arcgis);
    assert_eq!(c[4].scope.as_deref(), Some("tiles.read"));
    assert_eq!(
        c[5].names,
        vec!["X-Api-Key".to_string(), "X-Client".to_string()]
    );
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |doc: DocumentSnapshotV2| kentos_kcad::encode(&doc).expect_err("refused");
    let with = |f: fn(&mut ServiceLayer)| {
        let mut s = osm();
        f(&mut s);
        drawing(Some(s), None)
    };
    let mut group = drawing(Some(osm()), None);
    group.layers[1].kind = LayerNodeType::Group;
    let mut both = drawing(Some(osm()), Some(wfs()));
    both.layers[1].name = "İkisi".into();
    let mut holds = drawing(Some(osm()), None);
    holds.entities[0].base_mut().layer_id = "altlik".into();
    let mut duplicated = content("minimal.json");
    duplicated.settings.connections = vec![connection("hgm"), connection("hgm")];
    let mut with_path = content("minimal.json");
    with_path.settings.connections = vec![connection("hgm")];
    with_path.settings.connections[0].origin = "https://atlas.harita.gov.tr/wms".into();
    for (doc, place, words) in [
        (group, "layers/1/service", "grubun servisi yazılmaz"),
        (both, "document/layers", "ikisi birden olmaz"),
        (holds, "entities/0", "servis katmanı nesne tutmaz"),
        (
            with(|s| s.connection = Some("hgm".into())),
            "layers",
            "projenin bağlantıları arasında yok",
        ),
        (
            with(|s| s.url = "https://tile.example.com/{z}/{x}.png".into()),
            "layers/1/service",
            "{quadkey}",
        ),
        (
            with(|s| s.opacity = Some(0.0)),
            "layers/1/service",
            "donukluğu",
        ),
        (
            with(|s| s.max_zoom = Some(31)),
            "layers/1/service",
            "Katlar 0 ile 30",
        ),
        (duplicated, "settings/connections", "iki kez var"),
        (with_path, "settings/connections", "yolu olmamalı"),
    ] {
        let err = refused(doc);
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(place), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/service-in-schema-31.kcad",
            Code::UnknownField,
            "layers/0/service",
        ),
        (
            "broken/connections-in-schema-31.kcad",
            Code::UnknownField,
            "settings/connections",
        ),
        (
            "broken/connection-secret.kcad",
            Code::UnknownField,
            "connections/0/password",
        ),
        (
            "broken/service-without-url.kcad",
            Code::MissingField,
            "service/url",
        ),
        (
            "broken/service-yflip-false.kcad",
            Code::BadValue,
            "service/yFlip",
        ),
        (
            "broken/service-layers-empty.kcad",
            Code::BadValue,
            "service/layers",
        ),
        (
            "broken/service-holds-object.kcad",
            Code::BadValue,
            "entities/0",
        ),
        (
            "broken/service-on-group.kcad",
            Code::BadValue,
            "grubun servisi",
        ),
        (
            "broken/service-and-feed.kcad",
            Code::BadValue,
            "ikisi birden",
        ),
        (
            "broken/service-unknown-connection.kcad",
            Code::BadValue,
            "bağlantıları arasında yok",
        ),
        (
            "broken/service-wmts-two-layers.kcad",
            Code::BadValue,
            "tek bir katman",
        ),
        (
            "broken/service-google-url.kcad",
            Code::BadValue,
            "adresi boş olmalı",
        ),
        ("broken/feed-bbox-three.kcad", Code::BadValue, "feed/bbox"),
        (
            "broken/feed-version-geojson.kcad",
            Code::BadValue,
            "yalnız WFS",
        ),
        (
            "broken/connection-oauth2-no-token-url.kcad",
            Code::BadValue,
            "belirteç adresi",
        ),
        (
            "broken/connections-empty.kcad",
            Code::BadValue,
            "settings/connections",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}
