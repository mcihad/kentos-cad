//! Kurtarma (docs/adr/1013): a damaged `.kcad` gives back what passes the strict reader, and says what it left out.
//! The damages are made here from a valid file and the expected outcome follows from how each was made, not from the
//! recovery's own reading: a changed checksum keeps every object; object k given object k−1's id loses object k
//! alone (an id twice); a file cut inside object k keeps objects 0…k−1 and puts an empty drawing's settings and
//! active layer in place of the ones the cut took, said.

use kentos_kcad::salvage::salvage;

const FILE: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/drawing.kcad");

fn uid_bytes(doc: &kentos_contracts::DocumentSnapshotV2, k: usize) -> [u8; 16] {
    doc.uids[k].0
}

/// Where object `k`'s id is in the file: its 16 bytes after the CBOR head of a 16-byte string.
fn at(data: &[u8], uid: &[u8; 16]) -> usize {
    let mut needle = vec![0x50u8];
    needle.extend_from_slice(uid);
    let hits: Vec<usize> = data.windows(17).enumerate().filter(|(_, w)| *w == needle.as_slice()).map(|(i, _)| i).collect();
    assert_eq!(hits.len(), 1, "the id is once in the file");
    hits[0]
}

#[test]
fn a_whole_file_is_said_whole() {
    let got = salvage(FILE).expect("reads");
    assert!(got.whole);
    assert!(got.lost.is_empty());
}

#[test]
fn a_wrong_checksum_keeps_every_object_and_says_so() {
    let original = kentos_kcad::decode(FILE).expect("the fixture");
    let mut data = FILE.to_vec();
    let last = data.len() - 1;
    data[last] ^= 0xff;
    assert!(kentos_kcad::decode(&data).is_err(), "the reader refuses it");
    let got = salvage(&data).expect("recovers");
    assert!(!got.whole);
    assert!(got.container.iter().any(|c| c.contains("özeti tutmuyor")), "{:?}", got.container);
    assert!(got.lost.is_empty(), "{:?}", got.lost);
    assert_eq!(got.document.entities, original.entities);
    assert_eq!(got.document.uids, original.uids);
    assert_eq!(got.objects_kept, original.entities.len());
}

#[test]
fn an_object_whose_id_is_taken_twice_is_the_only_one_left_out() {
    let original = kentos_kcad::decode(FILE).expect("the fixture");
    let n = original.entities.len();
    assert!(n >= 4, "{n} objects");
    let k = n / 2;
    let mut data = FILE.to_vec();
    let place = at(&data, &uid_bytes(&original, k));
    data[place + 1..place + 17].copy_from_slice(&uid_bytes(&original, k - 1));
    let got = salvage(&data).expect("recovers");
    assert_eq!(got.objects_read, n);
    assert_eq!(got.objects_kept, n - 1);
    assert_eq!(got.lost.len(), 1, "{:?}", got.lost);
    assert_eq!(got.lost[0].part, format!("entities[{k}]"));
    assert!(got.lost[0].reason.starts_with("duplicate_uid"), "{}", got.lost[0].reason);
    let mut want = original.uids.clone();
    want.remove(k);
    assert_eq!(got.document.uids, want);
    // What came back is a drawing the reader reads as written.
    let bytes = kentos_kcad::encode(&got.document).expect("writes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("reads").uids, want);
}

#[test]
fn a_cut_inside_an_object_keeps_those_before_it_and_says_what_was_put_in_place() {
    let original = kentos_kcad::decode(FILE).expect("the fixture");
    let n = original.entities.len();
    let k = n / 2;
    let place = at(FILE, &uid_bytes(&original, k));
    let data = &FILE[..place + 5];
    let got = salvage(data).expect("recovers");
    assert!(got.container.iter().any(|c| c.contains("kesik")), "{:?}", got.container);
    assert_eq!(got.document.uids, original.uids[..k].to_vec());
    assert_eq!(got.document.entities, original.entities[..k].to_vec());
    let parts: Vec<&str> = got.lost.iter().map(|l| l.part.as_str()).collect();
    assert!(parts.contains(&"document.settings"), "{parts:?}");
    assert!(parts.contains(&"document.activeLayer"), "{parts:?}");
    assert_eq!(got.document.settings.srid, 0, "the empty drawing's");
    assert_eq!(Some(&got.document.active_layer), original.layers.first().map(|l| &l.id), "the first layer");
    assert_eq!(got.document.layers, original.layers, "the layers came before the cut");
}

#[test]
fn not_a_kcad_is_refused() {
    assert!(salvage(b"{\"format\": \"kentos.document\"}").is_err());
    assert!(salvage(&FILE[..20]).is_err());
}

/// docs/adr/1013 (blocks): definition 1 of three given definition 0's id loses that definition alone (an id twice)
/// and the one insert of it (its block gone); the other definitions and objects come back.
#[test]
fn a_definition_whose_id_is_taken_twice_goes_with_its_inserts_alone() {
    let file: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/block-extras.kcad");
    let original = kentos_kcad::decode(file).expect("the fixture");
    assert_eq!(original.blocks.len(), 3);
    let (first, second) = (original.blocks[0].id, original.blocks[1].id);
    let mut needle = vec![0x50u8];
    needle.extend_from_slice(&second.0);
    // The definitions come before the objects (key order): the first time the id is there is the definition's own.
    let at = file.windows(17).position(|w| w == needle.as_slice()).expect("the definition's id");
    let mut data = file.to_vec();
    data[at + 1..at + 17].copy_from_slice(&first.0);
    let got = salvage(&data).expect("recovers");
    let parts: Vec<&str> = got.lost.iter().map(|l| l.part.as_str()).collect();
    assert!(parts.contains(&"document.blocks[1]"), "{:?}", got.lost);
    let ids: Vec<_> = got.document.blocks.iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![original.blocks[0].id, original.blocks[2].id]);
    // The inserts of the lost definition go, each said; the others stay.
    let of_second = original
        .entities
        .iter()
        .filter(|e| matches!(e, kentos_contracts::Entity::Insert(i) if i.block == second))
        .count();
    assert!(of_second > 0);
    assert_eq!(got.objects_kept, original.entities.len() - of_second, "{:?}", got.lost);
    assert_eq!(got.lost.iter().filter(|l| l.part.starts_with("entities[")).count(), of_second);
}
