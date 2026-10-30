//! The tiles' glyphs (docs/adr/0800 §3): a manifest names one by its word
//! (`"icon": "mimari"`) or gives a picture file of its own. Each is drawn on
//! the web's 20 × 20 grid, as KentOS's icons are (DESIGN.md §6): 1.4 unit
//! lines in the tile's ink, round ends, fills only as deliberate faint
//! areas. Those of KentOS's icon set are reused as they are (the
//! instrument, the folded map, the page, the globe…); the building, the
//! road, the four tiles and the bell are the launcher's own.

/// A built-in glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Glyph {
    /// A building's front: KentOS Mimari.
    Mimari,
    /// A surveyor's instrument on its tripod (the web's `stakeout`): KentOS Harita.
    Harita,
    /// A folded map (the web's `modeGis`): KentOS GIS.
    Cbs,
    /// A road running off to the horizon: KentOS YOL.
    Yol,
    /// A page (the web's `document`): KentOS Rapor.
    Rapor,
    /// Four tiles: any program.
    #[default]
    Uygulama,
    /// A globe (the web's `crs`): a web address.
    Web,
    /// A terminal (the web's `terminal`): a script.
    Terminal,
    /// A table (the web's `table`).
    Tablo,
    /// A chart (the web's `chart`).
    Grafik,
    /// A folder (the web's `folder`).
    Klasor,
}

impl Glyph {
    /// Every glyph, the add dialog's order.
    pub const ALL: [Glyph; 11] = [
        Glyph::Uygulama,
        Glyph::Terminal,
        Glyph::Web,
        Glyph::Rapor,
        Glyph::Tablo,
        Glyph::Grafik,
        Glyph::Klasor,
        Glyph::Mimari,
        Glyph::Harita,
        Glyph::Cbs,
        Glyph::Yol,
    ];

    /// The manifest's word for it.
    pub const fn key(self) -> &'static str {
        match self {
            Glyph::Mimari => "mimari",
            Glyph::Harita => "harita",
            Glyph::Cbs => "cbs",
            Glyph::Yol => "yol",
            Glyph::Rapor => "rapor",
            Glyph::Uygulama => "uygulama",
            Glyph::Web => "web",
            Glyph::Terminal => "terminal",
            Glyph::Tablo => "tablo",
            Glyph::Grafik => "grafik",
            Glyph::Klasor => "klasor",
        }
    }

    /// What it shows, for a tip.
    pub const fn name(self) -> &'static str {
        match self {
            Glyph::Mimari => "Bina",
            Glyph::Harita => "Ölçme aleti",
            Glyph::Cbs => "Harita",
            Glyph::Yol => "Yol",
            Glyph::Rapor => "Belge",
            Glyph::Uygulama => "Uygulama",
            Glyph::Web => "Web",
            Glyph::Terminal => "Komut satırı",
            Glyph::Tablo => "Tablo",
            Glyph::Grafik => "Grafik",
            Glyph::Klasor => "Klasör",
        }
    }

    pub fn parse(key: &str) -> Option<Glyph> {
        Glyph::ALL.into_iter().find(|g| g.key() == key)
    }

    /// The words a manifest may write, for a refusal's sentence.
    pub fn known() -> String {
        Glyph::ALL.map(Glyph::key).join(", ")
    }

    /// Its drawing on the 20 × 20 grid.
    pub const fn svg(self) -> &'static str {
        match self {
            Glyph::Mimari => BUILDING,
            Glyph::Harita => INSTRUMENT,
            Glyph::Cbs => FOLDED_MAP,
            Glyph::Yol => ROAD,
            Glyph::Rapor => DOCUMENT,
            Glyph::Uygulama => APPS,
            Glyph::Web => GLOBE,
            Glyph::Terminal => TERMINAL,
            Glyph::Tablo => TABLE,
            Glyph::Grafik => CHART,
            Glyph::Klasor => FOLDER,
        }
    }
}

/// A building's front: a block of three floors and a lower wing, windows and a door.
const BUILDING: &str = concat!(
    r#"<path d="M2.5 17.5h15"/>"#,
    r#"<path d="M4.5 17.5V3.5h7.5v14M12 7.5h3.5v10"/>"#,
    r#"<path d="M6.6 6.5h.8M9.1 6.5h.8M6.6 9.5h.8M9.1 9.5h.8M6.6 12.5h.8M9.1 12.5h.8M13.4 10.5h.6M13.4 13.5h.6"/>"#,
    r#"<path d="M7 17.5v-2.5h2v2.5"/>"#,
);

/// The web's `stakeout` (aplikasyon): a surveyor's instrument, its sight's
/// cross, on its tripod.
const INSTRUMENT: &str = concat!(
    r#"<circle cx="10" cy="6.5" r="3.3"/>"#,
    r#"<path d="M10 3.2v6.6M6.7 6.5h6.6M10 9.8 5.8 17M10 9.8l4.2 7.2M10 9.8V17"/>"#,
);

/// The web's `modeGis`: a map folded in three.
const FOLDED_MAP: &str = concat!(
    r#"<path d="M2.5 5.8 7 3.8l6 2 4.5-2v10.4L13 16.2l-6-2-4.5 2z" fill="currentColor" fill-opacity=".14"/>"#,
    r#"<path d="M7 3.8v10.4M13 5.8v10.4"/>"#,
);

/// A road to the horizon: its edges meeting far off, its middle line dashed.
const ROAD: &str = concat!(
    r#"<path d="M2.5 17.5 7.6 2.5M17.5 17.5 12.4 2.5"/>"#,
    r#"<path d="M10 3.4v2M10 8.2v2.6M10 13.8v3.7"/>"#,
);

/// The web's `document`.
const DOCUMENT: &str =
    r#"<path d="M5 2.5h6.5L15.5 6.5v11H5z"/><path d="M11.5 2.5v4h4M7.5 10h5.5M7.5 13h5.5"/>"#;

/// Four tiles: the launcher's own page of apps, and an app without a glyph of its own.
pub const APPS: &str = concat!(
    r#"<rect x="3" y="3" width="5.8" height="5.8" rx="1.4"/>"#,
    r#"<rect x="11.2" y="3" width="5.8" height="5.8" rx="1.4"/>"#,
    r#"<rect x="3" y="11.2" width="5.8" height="5.8" rx="1.4"/>"#,
    r#"<rect x="11.2" y="11.2" width="5.8" height="5.8" rx="1.4"/>"#,
);

/// The web's `crs`: a globe.
const GLOBE: &str = r#"<circle cx="10" cy="10" r="7"/><path d="M3 10h14M10 3c-2.5 2-2.5 12 0 14M10 3c2.5 2 2.5 12 0 14"/>"#;

/// The web's `terminal`.
const TERMINAL: &str = r#"<rect x="2.5" y="4" width="15" height="12" rx="1"/><path d="m6 8 2.5 2L6 12M10.5 12.5h3.5"/>"#;

/// The web's `table`.
const TABLE: &str = r#"<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7.5h15M2.5 12h15M7.5 7.5v9"/>"#;

/// The web's `chart`.
const CHART: &str = concat!(
    r#"<rect x="2" y="2.5" width="16" height="15" rx="1.5"/>"#,
    r#"<path d="M5 14.8V11.7a1.2 1.2 0 0 1 2.4 0V14.8M8.8 14.8V9.2a1.2 1.2 0 0 1 2.4 0V14.8M12.6 14.8V12.7a1.2 1.2 0 0 1 2.4 0V14.8" stroke-width="1.2"/>"#,
    r#"<path d="M4.6 8.3 9.6 5.2l5.8 2" stroke-width="1.3"/>"#,
);

/// The web's `folder`.
const FOLDER: &str = r#"<path d="M2.5 15.5v-10h5l1.5 2h8.5v8z"/>"#;

/// The bell of the notifications: its dome on a flared rim, and its clapper.
pub const BELL: &str = concat!(
    r#"<path d="M5.5 13.5V9a4.5 4.5 0 0 1 9 0v4.5l1.5 1.5h-12z"/>"#,
    r#"<path d="M8.3 17.2a1.8 1.8 0 0 0 3.4 0"/>"#,
);

/// The web's `settings`: KentOS's settings are sliders.
pub const SETTINGS: &str = r#"<path d="M3.5 6h8M15.5 6h1M3.5 14h1M8.5 14h8"/><circle cx="13.5" cy="6" r="2"/><circle cx="6.5" cy="14" r="2"/>"#;

/// The web's `externalOpen`: a web address opens outside.
pub const EXTERNAL: &str =
    r#"<path d="M10.5 3.5h6v6M16.5 3.5 9 11"/><path d="M14.5 12v4.5h-11v-11H8"/>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_parses_whole_and_its_word_goes_both_ways() {
        for glyph in Glyph::ALL {
            assert_eq!(Glyph::parse(glyph.key()), Some(glyph));
            let markup = glyph.svg();
            let elements = markup.matches('<').count();
            assert_eq!(
                kentos_ui::icon::svg_elements(markup),
                elements,
                "{glyph:?}: every element of its drawing is read"
            );
        }
        for markup in [BELL, SETTINGS, EXTERNAL, APPS] {
            assert_eq!(
                kentos_ui::icon::svg_elements(markup),
                markup.matches('<').count()
            );
        }
        assert_eq!(Glyph::parse("Mimari"), None);
        assert!(Glyph::known().contains("terminal"));
    }
}
