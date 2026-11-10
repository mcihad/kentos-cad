//! Doğramaların ısıl geçirgenliği (docs/adr/8038; M38): TS 825:2024 "Binalarda ısı yalıtım kuralları", made binding by
//! Tebliğ MHGM-2025/1 (RG 20.2.2025/32819, in force from 1.4.2025), asks a window's Up of at most 1.8 W/m²K in every
//! climate zone; the standard's recommended U values are binding beside its energy limits. Mimari denetim's Mevzuat
//! group says a window type the drawing uses whose U value is above it, once for the type with its windows. A type
//! that says no U value is not judged: the maker's figure is not the drawing's to guess.

use kentos_commands::{Entity, OpeningForm, codes};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;

use crate::regulation_audit::{Article, Said, Source};

/// TS 825:2024's recommended U value of a window, W/m²K, the same in its six zones.
pub const WINDOW_U_MAX: f64 = 1.8;

pub const TS825_ARTICLE: Article = Article {
    source: Source::Ts825,
    article: "tavsiye edilen U değerleri (pencere 1,8 W/m²K)",
    gazette: "20.2.2025/32819, Tebliğ MHGM-2025/1",
};

/// A U value as a finding writes it: “2,4”.
fn u_text(v: f64) -> String {
    let t = format!("{v:.2}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    t.replace('.', ",")
}

/// The windows' findings (docs/adr/8038), each said through `push(said, slots, at)`.
pub(crate) fn findings(doc: &Document, push: &mut dyn FnMut(Said, Vec<Slot>, V)) {
    for t in doc.opening_types() {
        let Some(u) = t.u_value else {
            continue;
        };
        if t.form != OpeningForm::Window || u <= WINDOW_U_MAX + 1e-9 {
            continue;
        }
        let windows: Vec<(Slot, V)> = doc
            .entities()
            .filter_map(|e| match e {
                Entity::Opening(o)
                    if o.opening.follows_type()
                        && o.opening.style.as_deref() == Some(t.id.as_str()) =>
                {
                    let p = &o.opening.place;
                    let at = if p.is_empty() {
                        V::new(0.0, 0.0)
                    } else {
                        let n = p.len() as f64;
                        V::new(
                            p.iter().map(|q| q.x).sum::<f64>() / n,
                            p.iter().map(|q| q.y).sum::<f64>() / n,
                        )
                    };
                    Some((Slot(e.base().id), at))
                }
                _ => None,
            })
            .collect();
        let Some(&(_, at)) = windows.first() else {
            continue;
        };
        let named = match &t.code {
            Some(c) => format!("{c} {}", t.name),
            None => t.name.clone(),
        };
        push(
            Said {
                code: codes::WINDOW_U_HIGH,
                message: format!(
                    "{named} ({} pencere): U {} W/m²K > {} W/m²K — {}",
                    windows.len(),
                    u_text(u),
                    u_text(WINDOW_U_MAX),
                    TS825_ARTICLE.short()
                ),
                repair: "Daha iyi yalıtan bir doğrama ve cam seçin (TS 825:2024 Ek A.3'te örneğin PVC 5 odacıklı, çift camlı \
                         Low-E 16 mm: 1,6 W/m²K) ve tipin U değerini Kapı ve pencere tipleri'nde yazın."
                    .into(),
                rule: TS825_ARTICLE.named(),
                binding: None,
            },
            windows.iter().map(|(s, _)| *s).collect(),
            at,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_u_value_is_written_with_a_comma_and_no_trailing_zero() {
        assert_eq!(u_text(2.4), "2,4");
        assert_eq!(u_text(1.8), "1,8");
        assert_eq!(u_text(1.25), "1,25");
        assert_eq!(u_text(3.0), "3");
    }
}
