//! Attachments, actions and information cards in the project file
//! (docs/adr/0225, docs/specs/kcad-v2.md §6.5, §6.6): document schema 40 is
//! written only for a drawing with an object's attachment or a layer's action
//! or card; every other drawing keeps its schema and its bytes. They come
//! back as they were, a card's switches written only off; the writer refuses
//! what the reader would; the reader's errors name their places; the typed
//! columns carry attachments.

use kentos_kcad::contracts::{
    Attachment, DocumentSnapshotV2, Entity, InfoCard, LayerAction, LayerNodeType,
};
use kentos_kcad::{Code, Quiet};

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

fn linked(name: &str, file: &str) -> Attachment {
    Attachment {
        name: name.into(),
        file: Some(file.into()),
        ..Attachment::default()
    }
}

/// The minimal fixture's drawing with `attachments` on its point.
fn attached(attachments: Vec<Attachment>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    doc.entities[0].base_mut().attachments = attachments;
    doc
}

fn action(name: &str, target: &str) -> LayerAction {
    LayerAction {
        name: name.into(),
        target: target.into(),
    }
}

#[test]
fn only_a_drawing_with_attachments_actions_or_a_card_is_schema_40() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("attachments.kcad");
    assert_eq!(schema(&data), 40);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    assert_eq!(
        of(&attached(vec![linked("Tapu", "belgeler/101-5.pdf")])),
        40
    );
    let mut doc = content("minimal.json");
    doc.layers[0].actions = vec![action("Parsel sorgu", "https://a.org/{Ada}")];
    assert_eq!(of(&doc), 40);
    let mut doc = content("minimal.json");
    doc.layers[0].card = Some(InfoCard {
        actions: false,
        ..InfoCard::default()
    });
    assert_eq!(of(&doc), 40);
    // A default card is a card too: a field the file holds.
    let mut doc = content("minimal.json");
    doc.layers[0].card = Some(InfoCard::default());
    assert_eq!(of(&doc), 40);
    // The newest drawings before it keep their schemas.
    assert_eq!(of(&content("measures.json")), 39);
    assert_eq!(of(&content("filters.json")), 35);
}

#[test]
fn they_come_back_as_they_were() {
    let doc = content("attachments.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let parcel = &again.entities[0].base().attachments;
    assert_eq!(parcel.len(), 3);
    assert_eq!(
        parcel[0].note.as_deref(),
        Some("2019 tapusu\nAsıl nüsha kurumda")
    );
    assert!(parcel[1].asset.is_some() && parcel[2].url.is_some());
    let parsel = &again.layers[0];
    assert_eq!(parsel.actions.len(), 3);
    let card = parsel.card.as_ref().expect("a card");
    assert!(card.attachments && card.actions);
    assert_eq!(card.fields, ["Malik", "Nitelik"]);
    let direk = again.layers[1].card.as_ref().expect("a card");
    assert!(!direk.attachments && direk.actions && direk.title.is_none());
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |doc: DocumentSnapshotV2| kentos_kcad::encode(&doc).expect_err("refused");
    // An attachment has one source.
    let e = refused(attached(vec![Attachment {
        url: Some("https://a.org/a.pdf".into()),
        ..linked("Tapu", "a.pdf")
    }]));
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("attachments"), "{e}");
    // An address is http or https.
    let e = refused(attached(vec![Attachment {
        name: "Tapu".into(),
        url: Some("ftp://a.org/a.pdf".into()),
        ..Attachment::default()
    }]));
    assert_eq!(e.code, Code::BadValue);
    // A block definition's object has none.
    let mut doc = content("blocks.json");
    doc.blocks[0].entities[0].base_mut().attachments = vec![linked("Foto", "a.jpg")];
    let e = refused(doc);
    assert_eq!(e.code, Code::BadValue);
    assert!(
        e.message.contains("blok tanımının nesnesinde ek olmaz"),
        "{e}"
    );
    // A group has neither actions nor a card; a layer drawn from a service neither.
    let mut doc = content("minimal.json");
    doc.layers[0].kind = LayerNodeType::Group;
    doc.layers[0].card = Some(InfoCard::default());
    assert_eq!(refused(doc).code, Code::BadValue);
    let mut doc = content("services.json");
    let tiles = doc
        .layers
        .iter_mut()
        .flat_map(|n| n.children.iter_mut())
        .find(|n| n.service.is_some())
        .expect("a service layer");
    tiles.actions = vec![action("A", "https://a.org")];
    let e = refused(doc);
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("nesneleri olan katmanda"), "{e}");
    // Two actions of one name, whatever its case.
    let mut doc = content("minimal.json");
    doc.layers[0].actions = vec![action("Sorgu", "a"), action("SORGU", "b")];
    assert_eq!(refused(doc).code, Code::BadValue);
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/attachments-in-schema-39.kcad",
            Code::UnknownField,
            "point/attachments",
        ),
        (
            "broken/attachments-empty.kcad",
            Code::BadValue,
            "point/attachments",
        ),
        (
            "broken/attachment-two-sources.kcad",
            Code::BadValue,
            "attachments",
        ),
        (
            "broken/attachment-without-name.kcad",
            Code::MissingField,
            "attachments/0",
        ),
        (
            "broken/attachments-in-block.kcad",
            Code::UnknownField,
            "circle/attachments",
        ),
        (
            "broken/actions-in-schema-39.kcad",
            Code::UnknownField,
            "layers/0/actions",
        ),
        ("broken/actions-on-group.kcad", Code::BadValue, "layers/0"),
        ("broken/card-switch-on.kcad", Code::BadValue, "card"),
        (
            "broken/card-unknown-field.kcad",
            Code::UnknownField,
            "card/color",
        ),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}

#[test]
fn the_typed_columns_carry_attachments() {
    let doc = content("attachments.json");
    let (head, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (bytes, _) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect("writes");
    assert!(bytes == read("attachments.kcad"));
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(
        serde_json::to_string(&entities).expect("serializes"),
        serde_json::to_string(&doc.entities).expect("serializes")
    );
    assert!(matches!(entities[1], Entity::Point(_)));
    assert_eq!(
        entities[1].base().attachments,
        [linked("Direk", "foto/direk-12.jpg")]
    );
}
