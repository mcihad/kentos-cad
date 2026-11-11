//! The model quality control form (Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında
//! Yönetmelik Md. 15, EK-9 Tablo 9.1, Mimari; docs/adr/8047 §9): its 21 questions, answered from the model KentOS
//! writes (`crate::arch::building`), what it left out and the künye. A question the model answers by how it is made
//! (its names, classes, predefined types, property sets, version) is yes; one only the drawing can fail (the project's
//! words, its code, its rooms' numbers, its system, its losses) is yes or no with the reason; one KentOS does not judge
//! (the ministry's file size, the development level, the area tables, overlaps and duplicates Mimari denetim's other
//! groups read) is asked of the architect.

use crate::arch::{Particulars, Why, Written};
use crate::building::Building;
use crate::crs::{Crs, turef_meridian};

/// A question's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Evet, and how the model makes it so.
    Yes(String),
    /// Hayır, and why.
    No(String),
    /// KentOS does not judge it: the architect answers, where to look.
    Ask(String),
}

/// One of the form's questions: its number, its group, its discipline (Genel, Mimari), its title and its words, as
/// Tablo 9.1 has them, and its answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub no: u8,
    pub group: &'static str,
    pub discipline: &'static str,
    pub title: &'static str,
    pub question: &'static str,
    pub answer: Answer,
}

/// Tablo 9.1's questions: number, group, discipline, title, words.
pub const QUESTIONS: [(u8, &str, &str, &str, &str); 21] = [
    (
        1,
        "GENEL KONTROL",
        "Genel",
        "Dosya Boyutu",
        "Dosya boyutu, Bakanlıkça belirlenen kısıtlamalara uygun mu?",
    ),
    (
        2,
        "GENEL KONTROL",
        "Genel",
        "Proje Bilgileri",
        "Proje bilgileri Yönetmeliğin 13 üncü maddesinde belirtilen esaslara göre dolduruldu mu?",
    ),
    (
        3,
        "İSİMLENDİRME",
        "Genel",
        "Proje ve Dosya İsimlendirmesi",
        "Proje ve dosya isimlendirmesi Yönetmeliğin 6 ncı maddesine uygun mu?",
    ),
    (
        4,
        "İSİMLENDİRME",
        "Mimari",
        "Mahal No ve Mahal İsmi",
        "Mahal numaraları ve mahal isimlendirmesi Yönetmeliğin 10 uncu Maddesine uygun mu?",
    ),
    (
        5,
        "İSİMLENDİRME",
        "Genel",
        "Varlıklar",
        "Varlıkların isimlendirmesi Yönetmeliğin 11 inci maddesine uygun mu?",
    ),
    (
        6,
        "MODEL YAPILANMASI",
        "Genel",
        "Koordinat Sistemi",
        "Model doğru koordinat sisteminde konumlandırıldı mı? Bu koordinat verileri Yönetmeliğin 13 üncü maddesinde \
         verilen ilgili proje öznitelikleri kullanılarak atandı mı?",
    ),
    (
        7,
        "MODEL YAPILANMASI",
        "Genel",
        "Gelişim seviyesi",
        "Modelde yer alan varlıklar Yönetmeliğin 14 üncü maddesinde verilen gelişim seviyesine uygun olarak modellendi \
         mi?",
    ),
    (
        8,
        "MODEL YAPILANMASI",
        "Genel",
        "IFC Sınıfları",
        "Modelde yer alan varlıklar Yönetmeliğin 12 inci maddesinde tanımlanan “Zorunlu IFC Sınıfları” listesindeki \
         doğru sınıflar kullanılarak tanımlandı mı?",
    ),
    (
        9,
        "MODEL YAPILANMASI",
        "Genel",
        "IFC Sınıfları",
        "Yönetmeliğin 13 üncü maddesinde belirtilen projedeki mevcut “Zorunlu IFC Sınıfları” için zorunlu öznitelikler \
         ve özellikler tanımlandı mı?",
    ),
    (
        10,
        "MODEL YAPILANMASI",
        "Genel",
        "Emsal hesabı",
        "Mahal (ifcSpace) ve Mekansal Zon (ifcSpatialZone) alanları emsal hesap tablolarıyla uyumlu mu?",
    ),
    (
        11,
        "MODEL YAPILANMASI",
        "Genel",
        "IFC Sınıfları",
        "Modelde “Proje”, “Saha”, “Bina” ve en az bir “Bina Katı” ile ilgili IFC Sınıfları bulunuyor mu?",
    ),
    (
        12,
        "DİSİPLİN KONTROLLERİ",
        "Mimari",
        "Mahal",
        "Mimari modelde “Mahal” (ifcSpace) IFC Kategorisi yer alıyor mu?",
    ),
    (
        13,
        "DİSİPLİN KONTROLLERİ",
        "Mimari",
        "Mahal",
        "Mahaller diğer mahallerle örtüşmeyecek şekilde ve doğru katta modellendi mi?",
    ),
    (
        14,
        "DİSİPLİN KONTROLLERİ",
        "Mimari",
        "Mahal",
        "Mahaller kapalı olarak ve üst sınırları doğru tanımlandı mı?",
    ),
    (
        15,
        "DİSİPLİN KONTROLLERİ",
        "Genel",
        "Varlıklar",
        "Varlıklar doğru katta veya doğru kotta modellendi mi?",
    ),
    (
        16,
        "DİSİPLİN KONTROLLERİ",
        "Genel",
        "Varlıklar",
        "Modelden yinelenen varlıklar kaldırıldı mı?",
    ),
    (
        17,
        "TESLİM VE FORMAT",
        "Genel",
        "IFC Sınıfları",
        "Model IFC’ye uygun olarak oluşturuldu mu (IfcWall, IfcSlab, IfcWindow vb. sınıfları doğru atandı mı)?",
    ),
    (
        18,
        "TESLİM VE FORMAT",
        "Genel",
        "IFC Versiyonu",
        "Model dosyası, teslime ilişkin kısıtlamalarda tanımlanan IFC versiyonunda teslim edildi mi? Dosya uzantısı IFC \
         mi?",
    ),
    (
        19,
        "TESLİM VE FORMAT",
        "Genel",
        "Ön Tanımlı Tip",
        "Varlıklar IFC’de doğru Ön Tanımlı Tip (Predefined Type) kullanılarak temsil edildi mi?",
    ),
    (
        20,
        "TESLİM VE FORMAT",
        "Genel",
        "Özellikler",
        "Özellikler doğru P_set’ler altında listelendi mi?",
    ),
    (
        21,
        "TESLİM VE FORMAT",
        "Genel",
        "PDF ve IFC uyumu",
        "Teslim edilen PDF paftaları BIM modelinden üretildi mi?",
    ),
];

/// The künye's words EK-7 (Tablo 7.2–7.5) and EK-6 (Tablo 6.55: the address and the parcel) ask that it lacks, by
/// their labels in the künye; the project's code is question 3's (`has_project_code`).
pub fn project_info_missing(k: &Particulars) -> Vec<&'static str> {
    let fields: [(&Option<String>, &'static str); 10] = [
        (&k.architect, "mimar"),
        (&k.registration, "Mimarlar Odası sicil no"),
        (&k.office, "büronun adı"),
        (&k.office_tax_number, "büronun vergi kimlik no"),
        (&k.office_address, "büronun adresi"),
        (&k.province, "il"),
        (&k.district, "ilçe"),
        (&k.neighbourhood, "mahalle"),
        (&k.block, "ada"),
        (&k.parcel, "parsel"),
    ];
    fields
        .iter()
        .filter(|(v, _)| v.is_none())
        .map(|(_, l)| *l)
        .collect()
}

/// The project's code is the administration's (EK-1): none, or the künye's 000000, is no code.
pub fn has_project_code(k: &Particulars) -> bool {
    k.project_code
        .as_deref()
        .is_some_and(|c| c.chars().any(|ch| ch != '0'))
}

/// What is wrong with the model's system for EK-7 Tablo 7.6–7.7: none, a system that is no TUREF zone, no ±0.00
/// elevation; none when it is right.
pub fn system_problem(crs: Option<&Crs>, elevation: Option<f64>) -> Option<String> {
    match crs {
        None => Some("çizim yerel koordinatlarda".into()),
        Some(c) if turef_meridian(&c.name).is_none() => Some(format!(
            "projenin sistemi {} bir TUREF 3° dilimi (EPSG:5253–5259) değil",
            c.name
        )),
        Some(_) if elevation.is_none() => Some("±0.00'ın mutlak kotu (Vaziyet planı) yok".into()),
        Some(_) => None,
    }
}

/// What the model left out, `why`'s kinds with how many in the order the export says them: "2 duvar, 1 kapı".
pub fn left_words(written: &Written) -> String {
    let mut out: Vec<String> = Vec::new();
    for why in [Why::NoBody, Why::NoWall, Why::NoArea, Why::NotYet] {
        for what in [
            "duvar",
            "döşeme",
            "kolon",
            "kiriş",
            "çatı",
            "korkuluk",
            "merdiven",
            "rampa",
            "kapı",
            "pencere",
            "açıklık",
            "mahal",
        ] {
            let n = written
                .left
                .iter()
                .filter(|l| l.what == what && l.why == why)
                .count();
            if n > 0 {
                out.push(format!("{n} {what}"));
            }
        }
    }
    out.join(", ")
}

/// The form's 21 answers for the model `b`, what was `written` of the drawing, its künye `k`.
pub fn ek9(b: &Building, written: &Written, k: &Particulars) -> Vec<Row> {
    let missing = project_info_missing(k);
    let system = system_problem(b.crs.as_ref(), b.site_elevation);
    QUESTIONS
        .iter()
        .map(|&(no, group, discipline, title, question)| {
            let answer = match no {
                1 => Answer::Ask("Bakanlığın dosya boyutu sınırı; KentOS bilmiyor.".into()),
                2 if missing.is_empty() => Answer::Yes("EK-7'nin proje, mimar ve büro bilgileri künyeden.".into()),
                2 => Answer::No(format!("künyede eksik: {}", missing.join(", "))),
                3 if has_project_code(k) => Answer::Yes(format!("dosya adı EK-2'ye göre: {}.ifc", b.file)),
                3 => Answer::No("proje kodu (EK-1) yok; dosya ve IFC projesi kodsuz".into()),
                4 if written.unnamed.is_empty() => Answer::Yes("her mahalin numarası ve adı var.".into()),
                4 => Answer::No(format!("{} mahalin numarası ya da adı yok", written.unnamed.len())),
                5 => Answer::Yes("adlar EK-5 §5.5'e göre: MM-<kod>-<açıklama>.".into()),
                6 => match &system {
                    None => Answer::Yes(format!(
                        "{} (EK-7 Tablo 7.6), ±0.00 {:.2} m.",
                        b.crs.as_ref().map_or("", |c| c.name.as_str()),
                        b.site_elevation.unwrap_or(0.0)
                    )),
                    Some(why) => Answer::No(why.clone()),
                },
                7 => Answer::Ask("EK-8'in gelişim seviyesi; KentOS değerlendirmiyor.".into()),
                8 | 17 => Answer::Yes("sınıflar EK-5 Tablo 5.1'e göre.".into()),
                9 => Answer::No(
                    "yangın dayanım sınıfı (FireRating), acil çıkış (FireExit) ve beton sınıfı çizimde yok".into(),
                ),
                10 => Answer::Ask("Mahallerin net alanı yazıldı; emsal tablosuyla karşılaştırma mimarın.".into()),
                11 => Answer::Yes(format!("proje, saha, bina ve {} kat.", b.storeys.len())),
                12 if b.spaces.is_empty() => Answer::No("modelde mahal yok".into()),
                12 => Answer::Yes(format!("{} mahal.", b.spaces.len())),
                13 => Answer::Ask("Mimari denetim › Mahaller ve Kat gruplarına bakın.".into()),
                14 => Answer::Yes("mahal kapalı sınırından katının ya da kendi yüksekliğine.".into()),
                15 => Answer::Ask("Mimari denetim › Kat grubuna bakın.".into()),
                16 => Answer::Ask("Mimari denetim › Duvarlar grubuna (üst üste duvar) bakın.".into()),
                18 => Answer::Yes("IFC 4.3 (IFC4X3_ADD2, EK-5 §5.4), uzantı .ifc.".into()),
                19 => Answer::Yes("her sınıfın ön tanımlı tipi yazıldı.".into()),
                20 => Answer::Yes("özellikler IFC'nin Pset_… ve EK-6'nın setlerinde.".into()),
                21 if written.left.is_empty() => {
                    Answer::Yes("paftalar ve model aynı çizimden; her nesne modelde.".into())
                }
                _ => Answer::No(format!("modele girmeyen: {}", left_words(written))),
            };
            Row {
                no,
                group,
                discipline,
                title,
                question,
                answer,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kunye() -> Particulars {
        let s = |v: &str| Some(v.to_owned());
        Particulars {
            project_code: s("260123"),
            architect: s("Ayşe Yılmaz"),
            registration: s("41234"),
            office: s("Örnek Mimarlık"),
            office_tax_number: s("1234567890"),
            office_address: s("Sivas"),
            province: s("Sivas"),
            district: s("Merkez"),
            neighbourhood: s("Esentepe"),
            block: s("1234"),
            parcel: s("5"),
            ..Particulars::default()
        }
    }

    #[test]
    fn the_kunye_s_missing_words_are_named() {
        assert!(project_info_missing(&kunye()).is_empty());
        let mut k = kunye();
        k.registration = None;
        k.office_tax_number = None;
        assert_eq!(
            project_info_missing(&k),
            ["Mimarlar Odası sicil no", "büronun vergi kimlik no"]
        );
        k.project_code = Some("000000".into());
        assert!(!has_project_code(&k));
    }

    #[test]
    fn a_system_must_be_a_turef_zone_with_its_elevation() {
        let tm33 = crate::crs::projected("EPSG:5255", None, None, [0.0; 3]);
        assert_eq!(system_problem(Some(&tm33), Some(1285.4)), None);
        assert!(system_problem(Some(&tm33), None).unwrap().contains("±0.00"));
        let utm = crate::crs::projected("EPSG:32636", None, None, [0.0; 3]);
        assert!(
            system_problem(Some(&utm), Some(1.0))
                .unwrap()
                .contains("TUREF")
        );
        assert!(system_problem(None, None).unwrap().contains("yerel"));
    }

    #[test]
    fn the_form_has_twenty_one_answers() {
        let b = Building::default();
        let rows = ek9(&b, &Written::default(), &kunye());
        assert_eq!(rows.len(), 21);
        assert!(
            rows.iter()
                .enumerate()
                .all(|(i, r)| usize::from(r.no) == i + 1)
        );
        assert!(
            matches!(rows[1].answer, Answer::Yes(_)),
            "the künye is whole"
        );
        assert!(matches!(rows[5].answer, Answer::No(_)), "no system");
        assert!(matches!(rows[11].answer, Answer::No(_)), "no room");
        assert!(
            matches!(rows[20].answer, Answer::Yes(_)),
            "nothing left out"
        );
    }
}
