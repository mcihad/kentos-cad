//! The project's survey quality in words (plan/map-cad.md MAP-18, docs/adr/2400): what a sheet's quality note says
//! about the points the map project accepted from its GNSS archive (MAP-03, docs/adr/1340 and 1740). A sheet's texts
//! read it (`@konum_dogrulugu`, `@yukseklik_dogrulugu`, `@ag_dengelemesi`, `@olcme_kalitesi`), so the title block says
//! what the archive holds and never what someone typed once (the plan: “datum/epok ve kalite notları aynı proje
//! bilgisinden gelsin”).
//!
//! - **Position:** the greatest point position error of the accepted points that have a σ, σP = √(σN² + σE²) (1σ;
//!   KentOS's rule since docs/adr/0203), in centimetres to a millimetre.
//! - **Height:** the greatest σ up.
//! - **Network:** the network adjustments the accepted points were made by, in the archive's order: their class, whether
//!   the rule package held, m0 and the degrees of freedom.
//! - **Note:** the three in one sentence, with the points whose σ is not recorded counted.
//!
//! The words are what a sheet shows: the archive keeps the values themselves. A project without accepted points says
//! none of them; nothing is guessed.
//!
//! A map on a sheet says its own points' (docs/adr/2800, [`survey_facts_where`]): the host asks which accepted points
//! its frame shows on the ground; the network part names only the adjustments those points were made by. A map that
//! shows none says so in the note; its values are empty.

use kentos_contracts::{MapSurvey, ProjectSettings, SurveyPoint};

/// The facts' variable names and what each holds. Not reserved: a project's or a sheet's own variable of a name comes
/// first.
pub const SURVEY_VARIABLES: [(&str, &str); 4] = [
    ("konum_dogrulugu", "Kabul edilen GNSS noktalarının en büyük nokta konum hatası (1σ, √(σK² + σD²))"),
    ("yukseklik_dogrulugu", "Kabul edilen GNSS noktalarının en büyük yükseklik hatası (1σ)"),
    ("ag_dengelemesi", "Noktaların dayandığı ağ dengelemeleri: sınıf, kural paketi, m0, serbestlik"),
    ("olcme_kalitesi", "Ölçme kalite notu: nokta sayısı, konum ve yükseklik hatası, ağ dengelemesi"),
];

/// The facts; an empty text where the project does not say it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurveyFacts {
    /// “±0,5 cm”.
    pub position: String,
    /// “±0,9 cm”.
    pub height: String,
    /// “da-1, C3 sınıfı, kurallar sağlandı (bohhbuy-2018-gnss@1), m0 0,98, serbestlik 3”.
    pub network: String,
    /// The sentence.
    pub note: String,
}

/// A length in centimetres to a millimetre with a decimal comma, “±0,5 cm” (what a sheet shows).
fn centimetres(metres: f64) -> String {
    format!("±{:.1} cm", metres * 100.0).replace('.', ",")
}

fn comma(v: f64, decimals: usize) -> String {
    format!("{v:.decimals$}").replace('.', ",")
}

/// The note of a map that shows none of the accepted points (docs/adr/2800).
pub const NONE_SHOWN: &str = "Haritanın içinde GNSS ile kabul edilmiş nokta yok.";

/// The project's survey facts: every accepted point's.
pub fn survey_facts(settings: &ProjectSettings) -> SurveyFacts {
    survey_facts_where(settings, |_| true)
}

/// The survey facts of the accepted points `shown` takes (docs/adr/2800: those a map's frame shows). With accepted
/// points but none taken, the note is [`NONE_SHOWN`] and the values are empty; without accepted points, nothing.
pub fn survey_facts_where(settings: &ProjectSettings, shown: impl Fn(&SurveyPoint) -> bool) -> SurveyFacts {
    let Some(survey) = settings.map_survey.as_ref().filter(|s| !s.points.is_empty()) else {
        return SurveyFacts::default();
    };
    let points: Vec<&SurveyPoint> = survey.points.iter().filter(|p| shown(p)).collect();
    if points.is_empty() {
        return SurveyFacts { note: NONE_SHOWN.to_owned(), ..SurveyFacts::default() };
    }
    let sigmas: Vec<[f64; 3]> = points.iter().filter_map(|p| p.result.sigma).collect();
    let position = sigmas.iter().map(|s| s[0].hypot(s[1])).reduce(f64::max);
    let height = sigmas.iter().map(|s| s[2]).reduce(f64::max);
    let network = networks(survey, &points);
    let mut note = format!("GNSS ile {} nokta", points.len());
    match (position, height) {
        (Some(p), Some(h)) => {
            note.push_str(&format!(": en büyük nokta konum hatası {}, yükseklik hatası {} (ortalama hata)", centimetres(p), centimetres(h)));
        }
        _ => note.push_str(": doğrulukları kayıtlı değil"),
    }
    let short = short_networks(survey, &points);
    if !short.is_empty() {
        note.push_str(&format!("; ağ dengelemesi {short}"));
    }
    note.push('.');
    let without = points.len() - sigmas.len();
    if without > 0 && !sigmas.is_empty() {
        note.push_str(&format!(" {without} noktanın doğruluğu kayıtlı değil."));
    }
    SurveyFacts {
        position: position.map(centimetres).unwrap_or_default(),
        height: height.map(centimetres).unwrap_or_default(),
        network,
        note,
    }
}

/// The adjustments the points name, in the archive's order.
fn used<'a>(survey: &'a MapSurvey, points: &'a [&SurveyPoint]) -> impl Iterator<Item = &'a kentos_contracts::SurveyAdjustment> {
    survey
        .adjustments
        .iter()
        .filter(move |a| points.iter().any(|p| p.result.adjustment.as_deref() == Some(a.id.as_str())))
}

fn rules_word(passed: Option<bool>) -> Option<&'static str> {
    passed.map(|ok| if ok { "kurallar sağlandı" } else { "kurallar sağlanmadı" })
}

/// “da-1, C3 sınıfı, kurallar sağlandı (bohhbuy-2018-gnss@1), m0 0,98, serbestlik 3; da-2, …”.
fn networks(survey: &MapSurvey, points: &[&SurveyPoint]) -> String {
    used(survey, points)
        .map(|a| {
            let mut words = vec![a.id.clone()];
            if let Some(c) = &a.class {
                words.push(format!("{c} sınıfı"));
            }
            if let Some(r) = rules_word(a.rules_passed) {
                words.push(match &a.rules {
                    Some(package) => format!("{r} ({package})"),
                    None => r.to_owned(),
                });
            }
            if let Some(m0) = a.m0 {
                words.push(format!("m0 {}", comma(m0, 2)));
            }
            words.push(format!("serbestlik {}", a.dof));
            words.join(", ")
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The note's short form: “da-1 (C3, kurallar sağlandı)”.
fn short_networks(survey: &MapSurvey, points: &[&SurveyPoint]) -> String {
    used(survey, points)
        .map(|a| {
            let more: Vec<String> =
                a.class.iter().cloned().chain(rules_word(a.rules_passed).map(str::to_owned)).collect();
            if more.is_empty() { a.id.clone() } else { format!("{} ({})", a.id, more.join(", ")) }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The map project of `fixtures/kcad/v2/map-survey-network.json`: kn-1 accepted as a mean (no σ), kn-2 by the
    /// network adjustment da-1 (σ 4.0, 3.5, 9.0 mm; C3, the rules held, m0 0.98, 3 degrees of freedom). By hand:
    /// √(4.0² + 3.5²) = 5.315 mm.
    fn settings() -> ProjectSettings {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../../../fixtures/kcad/v2/map-survey-network.json")).expect("the file");
        serde_json::from_value(v["settings"].clone()).expect("settings")
    }

    #[test]
    fn the_network_project_s_quality() {
        let f = survey_facts(&settings());
        assert_eq!(f.position, "±0,5 cm");
        assert_eq!(f.height, "±0,9 cm");
        assert_eq!(f.network, "da-1, C3 sınıfı, kurallar sağlandı (bohhbuy-2018-gnss@1), m0 0,98, serbestlik 3");
        assert_eq!(
            f.note,
            "GNSS ile 2 nokta: en büyük nokta konum hatası ±0,5 cm, yükseklik hatası ±0,9 cm (ortalama hata); ağ dengelemesi da-1 \
             (C3, kurallar sağlandı). 1 noktanın doğruluğu kayıtlı değil."
        );
    }

    #[test]
    fn points_without_a_sigma_and_no_archive() {
        let mut s = settings();
        if let Some(m) = s.map_survey.as_mut() {
            m.points.retain(|p| p.result.sigma.is_none());
        }
        let f = survey_facts(&s);
        assert_eq!((f.position.as_str(), f.height.as_str(), f.network.as_str()), ("", "", ""));
        assert_eq!(f.note, "GNSS ile 1 nokta: doğrulukları kayıtlı değil.");
        s.map_survey = None;
        assert_eq!(survey_facts(&s), SurveyFacts::default());
    }

    /// docs/adr/2800: a map's own points. kn-2 alone carries da-1 and its σ; kn-1 alone has none and names no
    /// adjustment; none shown says so.
    #[test]
    fn a_map_says_only_the_points_it_shows() {
        let s = settings();
        let only = |id: &'static str| move |p: &SurveyPoint| p.id == id;
        let f = survey_facts_where(&s, only("kn-2"));
        assert_eq!((f.position.as_str(), f.height.as_str()), ("±0,5 cm", "±0,9 cm"));
        assert_eq!(f.note, "GNSS ile 1 nokta: en büyük nokta konum hatası ±0,5 cm, yükseklik hatası ±0,9 cm (ortalama hata); \
                            ağ dengelemesi da-1 (C3, kurallar sağlandı).");
        let f = survey_facts_where(&s, only("kn-1"));
        assert_eq!((f.position.as_str(), f.network.as_str()), ("", ""), "no σ, no adjustment");
        assert_eq!(f.note, "GNSS ile 1 nokta: doğrulukları kayıtlı değil.");
        let f = survey_facts_where(&s, |_| false);
        assert_eq!(f, SurveyFacts { note: NONE_SHOWN.to_owned(), ..SurveyFacts::default() });
    }
}
