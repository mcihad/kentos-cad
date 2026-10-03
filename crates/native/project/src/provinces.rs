//! Türkiye's 81 provinces with the centre of each provincial capital (the
//! web's `geo/provinces.ts`, docs/adr/0165 §3), read from the file the web
//! writes (fixtures/crs/v1/provinces.json): a new project's start view and
//! the TM zone it suggests. The positions are city centres to about a
//! hundredth of a degree, for a view and a zone, never a measurement.

use std::sync::OnceLock;

/// A province: its plate code, name and centre (degrees north and east).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Province {
    pub code: u32,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

/// The 81, by their plate codes.
pub fn provinces() -> &'static [Province] {
    static ALL: OnceLock<Vec<Province>> = OnceLock::new();
    ALL.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct File {
            provinces: Vec<Province>,
        }
        serde_json::from_str::<File>(include_str!("../../../../fixtures/crs/v1/provinces.json"))
            .map(|f| f.provinces)
            .unwrap_or_default()
    })
}

/// The province of a plate code.
pub fn province(code: u32) -> Option<&'static Province> {
    provinces().iter().find(|p| p.code == code)
}

/// `s.toLocaleLowerCase('tr-TR')`: I → ı, İ → i, then the default mapping.
fn lower_tr(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'I' => 'ı',
            'İ' => 'i',
            c => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// Provinces whose name starts with or holds the query, Turkish case folded
/// (“iz” finds İzmir), or whose plate code it begins (the exact one first);
/// all for none (the web's `searchProvinces`).
pub fn search(query: &str) -> Vec<&'static Province> {
    let q = lower_tr(query.trim());
    if q.is_empty() {
        return provinces().iter().collect();
    }
    if q.chars().all(|c| c.is_ascii_digit()) {
        let exact: Option<u32> = q.parse().ok();
        let begins: Vec<&Province> = provinces()
            .iter()
            .filter(|p| {
                p.code.to_string().starts_with(&q) || format!("{:02}", p.code).starts_with(&q)
            })
            .collect();
        let (first, rest): (Vec<_>, Vec<_>) =
            begins.into_iter().partition(|p| Some(p.code) == exact);
        return first.into_iter().chain(rest).collect();
    }
    let names: Vec<(String, &Province)> =
        provinces().iter().map(|p| (lower_tr(&p.name), p)).collect();
    let starts = names
        .iter()
        .filter(|(n, _)| n.starts_with(&q))
        .map(|(_, p)| *p);
    let holds = names
        .iter()
        .filter(|(n, _)| !n.starts_with(&q) && n.contains(&q))
        .map(|(_, p)| *p);
    starts.chain(holds).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The web's cases (`geo/provinces.test.ts`).
    #[test]
    fn the_81_are_found_as_on_the_web() {
        assert_eq!(
            provinces().iter().map(|p| p.code).collect::<Vec<_>>(),
            (1..=81).collect::<Vec<_>>()
        );
        let names = |q: &str| {
            search(q)
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(names("iz"), ["İzmir", "Denizli", "Rize"]);
        assert_eq!(names("IĞ"), ["Iğdır", "Elazığ"]);
        assert_eq!(names("06"), ["Ankara"]);
        assert_eq!(
            search("3")
                .iter()
                .take(3)
                .map(|p| p.code)
                .collect::<Vec<_>>(),
            [3, 30, 31]
        );
        assert_eq!(search("  ").len(), 81);
        assert_eq!(province(34).map(|p| p.name.as_str()), Some("İstanbul"));
    }
}
