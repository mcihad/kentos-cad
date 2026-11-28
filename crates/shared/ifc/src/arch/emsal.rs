//! An emsal area's state as IFC writes it (docs/adr/8069; EK-6 Tablo 6.60): its zone's state, its words and the
//! clauses it rests on (Planlı Alanlar İmar Yönetmeliği 4/1-şş, 21, 22/1, 5/8).

/// An emsal area of a storey (docs/adr/8069; EK-6 Tablo 6.60): its storey's level id, how its rooms count, its rings
/// (even-odd, the rooms' contours together) and, counted in, the part of its storey's balconies past their units' 20 %
/// (m²; PAİY 22/1-l) and of its other limited uses past their limits (docs/adr/8079): storage annexes past their
/// units' 50 % (22/1-k), caretaker flats past 70 m² together (22/1-ç), library rooms past 50 m² (22/1-f), the
/// shelter past the building's needed shelter area (22/1-g, docs/adr/8149), the
/// residents' common rooms under the ground past 5 % of katlar alanı and 1000 m² (22/1-j, docs/adr/8147); and, within
/// its rings, a traditional cut stone masonry building's stone walls past 25 cm and 12.50 cm, out apart from the 30 %
/// (5/8, docs/adr/8095); and its rooms' part 5/8's basement list moves apart from the 30 % (docs/adr/8148).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EmsalArea {
    pub level: Option<String>,
    pub count: kentos_contracts::EmsalCount,
    pub rings: Vec<Vec<[f64; 2]>>,
    pub moved: f64,
    pub storage: f64,
    pub caretaker: f64,
    pub library: f64,
    pub shelter: f64,
    pub amenity: f64,
    pub walls: f64,
    pub apart: f64,
    /// Its rooms' own emsal states' bases, each once (docs/adr/8125): an emsal increase's description.
    pub bases: Vec<String>,
    /// The emsal table's area of its storey's rooms counting so, their contours' (docs/adr/8125; EK-9/10); none: not
    /// known.
    pub table: Option<f64>,
}

/// The limited uses' parts an emsal zone names (docs/adr/8079): each clause's words up to its part's area, and how a zone
/// counted in ends them (Turkish vowel harmony: “balkon da”, “eklentisi de”).
fn limited_parts(a: &EmsalArea) -> [(f64, &'static str, &'static str); 6] {
    [
        (
            a.moved,
            "22/1-l: bağımsız bölüm net alanının %20'sini aşan {} m² balkon",
            " da dahil",
        ),
        (
            a.storage,
            "22/1-k: bağımsız bölüm net alanının %50'sini aşan {} m² depo eklentisi",
            " de dahil",
        ),
        (
            a.caretaker,
            "22/1-ç: kapıcı dairelerinin toplam 70 m²'sini aşan {} m²",
            " de dahil",
        ),
        (
            a.library,
            "22/1-f: kütüphanenin 50 m²'sini aşan {} m²",
            " de dahil",
        ),
        (
            a.shelter,
            "22/1-g: sığınağın gerekli alanını aşan {} m²",
            " de dahil",
        ),
        (
            a.amenity,
            "22/1-j: bodrumdaki ortak mekânların sınırını aşan {} m²",
            " de dahil",
        ),
    ]
}

/// An emsal zone's state as EK-6 Tablo 6.60 writes it and the clauses it rests on. A limited use's part past its limit
/// is named in its storey's zone counted in (the room stays in its own zone, docs/adr/8069 K41), or, on a storey with
/// none, in the zone the room is in, as counted in.
pub(super) fn emsal_state(a: &EmsalArea) -> (&'static str, &'static str, String) {
    use kentos_contracts::EmsalCount as C;
    let with_parts = |mut words: String, counted_in: bool| {
        for (part, clause, tail) in limited_parts(a) {
            if part > 0.0 {
                words.push_str("; ");
                words.push_str(&clause.replace("{}", &format!("{part:.2}")));
                words.push_str(if counted_in { tail } else { " emsale dahil" });
            }
        }
        // The stone walls' part within the zone's rings is out apart from the 30 % (docs/adr/8095).
        if a.walls > 0.0 {
            words.push_str(&format!(
                "; 5/8: {:.2} m² taşıyıcı yığma taş duvarların kalın kısmı emsal harici, %30 dışında",
                a.walls
            ));
        }
        // The basement's rooms 5/8's list moves apart from the 30 % (docs/adr/8148).
        if a.apart > 0.0 {
            words.push_str(&format!(
                "; 5/8: bodrumda {:.2} m² emsal harici, %30 dışında",
                a.apart
            ));
        }
        words
    };
    match a.count {
        C::Included => (
            "DAHIL",
            "emsale dahil alan",
            with_parts("Planlı Alanlar İmar Yönetmeliği 4/1-şş, 21: katlar alanına dahil".to_owned(), true),
        ),
        C::Exempt => (
            "HARIC",
            "emsal harici alan",
            with_parts(
                "Planlı Alanlar İmar Yönetmeliği 22/1: katlar alanına dahil edilmeyen; 5/8: emsale esas alanın %30'u içinde"
                    .to_owned(),
                false,
            ),
        ),
        C::Outside => (
            "HARIC",
            "emsal harici alan (%30 dışında)",
            "Planlı Alanlar İmar Yönetmeliği 5/8: yangın merdiveni, korunumlu koridor, yangın güvenlik holü; %30 sınırının dışında".to_owned(),
        ),
        // EK-6 Tablo 6.60: "DIGER olması durumunda açıklama Description bölümüne girilir" (docs/adr/8125).
        C::Bonus => ("DIGER", "emsal artışı alanı", format!("Emsal artışı: {}", a.bases.join("; "))),
    }
}
