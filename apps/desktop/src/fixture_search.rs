//! Donatı araması (docs/adr/8024; ADV-12): KentOS's library's furniture and fixtures (Islak hacim, Mobilya, Mutfak)
//! found in Komut ara by their names, tags and categories, and by the words people say for them (karyola, tuvalet,
//! eviye, duşakabin …; the Bloklar panel's search knows the words too). Enter on one takes that one block in, unless
//! the drawing has it, and starts Blok ekle with it: in KentOS Mimari Duvara hizala is on (docs/adr/0702), so a click
//! by a wall stands its back on the wall's face. Before, Komut ara found only the Kitaplık command, which took all 38
//! of the library's blocks in and left the user to find the one in the Bloklar panel.

use std::sync::OnceLock;

use kentos_commands::BlockEditOperation;
use kentos_interaction::Level;
use serde_json::Value;

use crate::app::App;
use crate::exchange::apply::fold_turkish;

/// The library's categories that furnish a plan (the doors, windows and symbols have their own tools).
const KINDS: [&str; 3] = ["Islak hacim", "Mobilya", "Mutfak"];

/// Words people say for what the library names otherwise, folded: each with the library's word it means.
const WORDS: &[(&str, &str)] = &[
    ("KARYOLA", "YATAK"),
    ("BAZA", "YATAK"),
    ("SOMYA", "YATAK"),
    // A wardrobe by its tag, not every cupboard (Dolaplı lavabo, Alt dolap).
    ("ELBISE DOLABI", "GARDIROP"),
    ("TUVALET", "KLOZET"),
    ("ALAFRANGA", "KLOZET"),
    ("REZERVUAR", "KLOZET"),
    ("WC", "KLOZET"),
    ("EVIYE", "EVYE"),
    ("DUSAKABIN", "DUS"),
    ("DUS KABINI", "DUS"),
    ("JAKUZI", "KUVET"),
    ("CEKYAT", "KANEPE"),
    ("DIVAN", "KANEPE"),
    ("SEDIR", "KANEPE"),
    ("PISUAR", "PISUVAR"),
    ("KOMIDIN", "KOMODIN"),
    ("MAKINA", "MAKINE"),
    ("TELEVIZYON", "TV"),
    ("BANYO DOLABI", "DOLAPLI LAVABO"),
    ("SET USTU", "OCAK"),
    ("KITAP", "KITAPLIK"),
];

/// One of the library's furniture and fixtures: its name, category and tags (folded for the search too).
#[derive(Clone, Debug)]
pub(crate) struct Fixture {
    pub name: String,
    pub category: String,
    folded: Vec<String>,
    tags: Vec<String>,
}

/// The library's furniture and fixtures, read once.
pub(crate) fn fixtures() -> &'static [Fixture] {
    static ALL: OnceLock<Vec<Fixture>> = OnceLock::new();
    ALL.get_or_init(|| {
        let text = kentos_native_application::blocks_library::BUILTIN_LIBRARIES[0].1;
        let file: Value = serde_json::from_str(text).unwrap_or_default();
        file["blocks"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|b| {
                let name = b["name"].as_str()?.to_owned();
                let category = b["card"]["category"].as_str()?.to_owned();
                if !KINDS.iter().any(|k| category.starts_with(&format!("{k}/"))) {
                    return None;
                }
                let tags = b["card"]["tags"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|t| t.as_str().map(fold_turkish))
                    .collect();
                Some(Fixture {
                    folded: vec![fold_turkish(&name), fold_turkish(&category)],
                    name,
                    category,
                    tags,
                })
            })
            .collect()
    })
}

/// The library's words `query` means (folded): itself, and what a word people say for it means. A word begun with at
/// least three letters counts (karyo → karyola → yatak).
pub(crate) fn meant(query: &str) -> Vec<String> {
    let q = fold_turkish(query);
    if q.is_empty() {
        return Vec::new();
    }
    let mut out = vec![q.clone()];
    for (said, means) in WORDS {
        if (*said == q || (q.chars().count() >= 3 && said.starts_with(&q)))
            && !out.iter().any(|w| w == means)
        {
            out.push((*means).to_owned());
        }
    }
    out
}

/// Whether a block's name or tags hold one of the words `query` means (the Bloklar panel's search, docs/adr/8024).
pub(crate) fn names_it(query: &str, name: &str, tags: &[String]) -> bool {
    let words = meant(query);
    let name = fold_turkish(name);
    words.iter().skip(1).any(|w| {
        name.contains(w.as_str()) || tags.iter().any(|t| fold_turkish(t).contains(w.as_str()))
    })
}

/// The fixtures `query` finds with their ranks (lower is better; Komut ara's: an alias equal 0, a title beginning
/// with the text 2): the name itself −1, a name beginning with a word 1.5, holding one 2.5, a tag 3, the category 3.5.
/// Two letters at least.
pub(crate) fn search(query: &str) -> Vec<(f64, &'static Fixture)> {
    let words = meant(query);
    if words.first().is_none_or(|q| q.chars().count() < 2) {
        return Vec::new();
    }
    let mut found: Vec<(f64, &'static Fixture)> = fixtures()
        .iter()
        .filter_map(|f| {
            let (name, category) = (&f.folded[0], &f.folded[1]);
            let score = words
                .iter()
                .map(|w| {
                    if name == w {
                        -1.0
                    } else if name.starts_with(w.as_str()) {
                        1.5 + name.chars().count() as f64 / 100.0
                    } else if name.contains(w.as_str()) {
                        2.5
                    } else if f.tags.iter().any(|t| t.starts_with(w.as_str())) {
                        3.0
                    } else if category.contains(w.as_str()) {
                        3.5
                    } else {
                        f64::INFINITY
                    }
                })
                .fold(f64::INFINITY, f64::min);
            score.is_finite().then_some((score, f))
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found
}

/// Where a fixture is shown in the list: “Donatı › Islak hacim › Lavabo”.
pub(crate) fn detail(f: &Fixture) -> String {
    format!("Donatı › {}", f.category.replace('/', " › "))
}

impl App {
    /// A fixture of the library placed (docs/adr/8024): the drawing's block of its name, or the library's definition
    /// taken in alone (Blokları al, one undo step); Blok ekle starts with it.
    pub(crate) fn place_fixture(&mut self, name: &str) -> iced::Task<crate::app::Message> {
        let Some(doc) = self.document.as_ref() else {
            return iced::Task::none();
        };
        let own = doc.model.block_named(name).map(|b| b.id);
        let id = match own {
            Some(id) => id,
            None => {
                let text = kentos_native_application::blocks_library::BUILTIN_LIBRARIES[0].1;
                let mut file: Value = serde_json::from_str(text).unwrap_or_default();
                let key = fold_turkish(name);
                if let Some(blocks) = file["blocks"].as_array_mut() {
                    blocks.retain(|b| b["name"].as_str().is_some_and(|n| fold_turkish(n) == key));
                }
                let edit = kentos_commands::BlocksEdit {
                    library: Some(file.to_string()),
                    ..crate::blocks_files::input(BlockEditOperation::Import)
                };
                if self.blocks_command(edit).is_none() {
                    return iced::Task::none();
                }
                let Some(id) = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.block_named(name))
                    .map(|b| b.id)
                else {
                    return iced::Task::none();
                };
                id
            }
        };
        let to_wall = if self.memory.block_to_wall {
            " Duvarın yanına tıklayın: arkası duvara oturur, önü odaya bakar."
        } else {
            " Yerleştirme noktasına tıklayın."
        };
        let taken = if own.is_some() {
            "çizimdeki bloğu"
        } else {
            "kitaplıktan alındı"
        };
        self.say(Level::Success, format!("“{name}” {taken}.{to_wall}"));
        self.memory.block_insert = Some(id);
        self.start_tool(kentos_interaction::block_insert::ID)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(query: &str) -> Vec<&'static str> {
        search(query)
            .into_iter()
            .map(|(_, f)| f.name.as_str())
            .collect()
    }

    #[test]
    fn the_library_s_fixtures_are_found_by_name_tag_and_the_words_people_say() {
        assert_eq!(names("lavabo"), ["Lavabo", "Dolaplı lavabo"]);
        // The beds first, then the bedside table of their category.
        assert_eq!(
            names("karyola"),
            ["Çift kişilik yatak", "Tek kişilik yatak", "Komodin"]
        );
        assert_eq!(names("tuvalet").first(), Some(&"Klozet"));
        assert!(names("tuvalet").contains(&"Asma klozet"));
        assert_eq!(names("eviye"), ["Evye"]);
        assert_eq!(names("gardırop"), ["Dolap"]);
        assert!(names("duşakabin").contains(&"Köşe duş"));
        assert_eq!(names("l"), Vec::<&str>::new(), "one letter finds nothing");
        // The doors, windows and symbols are not fixtures: their own tools place them.
        assert!(names("kapı").is_empty());
        assert!(names("kuzey").is_empty());
    }

    #[test]
    fn the_bloklar_search_knows_the_words_too() {
        assert!(names_it("karyola", "Tek kişilik yatak", &[]));
        assert!(names_it("TUVALET", "Asma klozet", &[]));
        assert!(!names_it("karyola", "Kanepe", &["kanepe".to_owned()]));
        assert!(
            !names_it("lavabo", "Lavabo", &[]),
            "the word itself is the search's own"
        );
    }
}
