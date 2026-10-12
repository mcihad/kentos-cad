//! An address written (docs/adr/0229 §9): its parts as the data writes them,
//! with a space between: the neighbourhood (“ Mahallesi” added when it says
//! no MAHALLE or KÖY of its own), the street, “No:” and the door, the
//! postcode, “district/province” (one alone when the other is missing).

use super::locator::{Locator, Texts};

/// A part after the ones written, a space between.
fn push_part(out: &mut String, text: &str) {
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(text);
}

/// The address's text, written into one string (a search writes twenty).
pub fn label(t: &Texts) -> String {
    let mut out = String::with_capacity(
        t.neighbourhood.len()
            + t.street.len()
            + t.door.len()
            + t.postcode.len()
            + t.district.len()
            + t.province.len()
            + 24,
    );
    if !t.neighbourhood.is_empty() {
        push_part(&mut out, &t.neighbourhood);
        if !Locator::tagged_hood(&t.neighbourhood) {
            out.push_str(" Mahallesi");
        }
    }
    if !t.street.is_empty() {
        push_part(&mut out, &t.street);
    }
    if !t.door.is_empty() {
        push_part(&mut out, "No:");
        out.push_str(&t.door);
    }
    if !t.postcode.is_empty() {
        push_part(&mut out, &t.postcode);
    }
    match (t.district.is_empty(), t.province.is_empty()) {
        (false, false) => {
            push_part(&mut out, &t.district);
            out.push('/');
            out.push_str(&t.province);
        }
        (false, true) => push_part(&mut out, &t.district),
        (true, false) => push_part(&mut out, &t.province),
        (true, true) => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_reads_in_the_turkish_order() {
        let t = Texts {
            street: "Atatürk Bulvarı".into(),
            door: "12".into(),
            neighbourhood: "Kızılay".into(),
            district: "Çankaya".into(),
            province: "Ankara".into(),
            postcode: "06420".into(),
            ..Texts::default()
        };
        assert_eq!(
            label(&t),
            "Kızılay Mahallesi Atatürk Bulvarı No:12 06420 Çankaya/Ankara"
        );
        let t = Texts {
            neighbourhood: "Gölbaşı Köyü".into(),
            district: "Polatlı".into(),
            ..Texts::default()
        };
        assert_eq!(label(&t), "Gölbaşı Köyü Polatlı");
    }
}
