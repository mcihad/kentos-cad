//! Which drawing typeface a typeface named by a file or a family is drawn in (docs/adr/0324 §5): one rule for a DXF's
//! STYLE, a style's file and `cad.fonts.report`. KentOS draws, measures and embeds its own faces (the tables of
//! `metrics.rs`, the TrueType files a PDF embeds as subsets) and the families its fonts folders gave (docs/adr/9313);
//! any other face is drawn in one of them and said, never silently.

/// How a named typeface is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontMatch {
    /// One of KentOS's own faces by its name.
    Own,
    /// A face KentOS has a metric twin of (Helvetica and Liberation Sans as Arial, Liberation Serif and Tinos as Times
    /// New Roman, Courier as Courier Prime): its letters as wide, its look close.
    Equivalent,
    /// An SHX line font (`romans.shx`, a big font): KentOS does not read SHX; the project's face takes it.
    Shx,
    /// A face KentOS does not have (Times, Calibri …): the project's face takes it.
    Missing,
    /// A family a fonts folder gave (docs/adr/9313): drawn in its own faces.
    Folder,
}

impl FontMatch {
    pub fn name(self) -> &'static str {
        match self {
            FontMatch::Own => "own",
            FontMatch::Equivalent => "equivalent",
            FontMatch::Shx => "shx",
            FontMatch::Missing => "missing",
            FontMatch::Folder => "folder",
        }
    }

    /// Whether the face drawn is not the one named.
    pub fn substituted(self) -> bool {
        matches!(self, FontMatch::Shx | FontMatch::Missing)
    }
}

/// KentOS's face for a typeface named `name` (a file, `arialbd.ttf`, `romans.shx`, or a family, `Arial Narrow`), as
/// a `DrawingFont` id (a folder family's name), and how; none for one drawn in the project's face.
pub fn resolve(name: &str) -> (Option<&'static str>, FontMatch) {
    let lower = name.trim().to_lowercase();
    // A source may carry either Windows or Unix font paths. Keep the source
    // string on the style; only match the leaf here, without touching disk.
    let leaf = lower.rsplit(['/', '\\']).next().unwrap_or(&lower);
    if leaf.ends_with(".shx") {
        return (None, FontMatch::Shx);
    }
    let family = [".ttf", ".otf", ".ttc", ".woff2", ".woff"]
        .iter()
        .find_map(|ext| leaf.strip_suffix(ext))
        .unwrap_or(leaf);
    let n: String = family
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    for (stem, id) in [
        ("barlow", "barlow"),
        ("arimo", "arimo"),
        ("overpass", "overpass"),
        ("quicksand", "quicksand"),
        ("architectsdaughter", "architects-daughter"),
        ("courierprime", "courier-prime"),
        ("ibmplexmono", "plex-mono"),
        ("plexmono", "plex-mono"),
        // docs/adr/9311: Arial and Times New Roman by their names and their files (arial.ttf, times.ttf …).
        ("arial", "arial"),
        ("timesnewroman", "times-new-roman"),
        ("times", "times-new-roman"),
    ] {
        if family_style(&n, stem) {
            return (Some(id), FontMatch::Own);
        }
    }
    for (stem, id) in [
        ("helvetica", "arial"),
        ("liberationsans", "arial"),
        ("liberationserif", "times-new-roman"),
        ("tinos", "times-new-roman"),
        ("couriernew", "courier-prime"),
        ("courier", "courier-prime"),
        ("cour", "courier-prime"),
    ] {
        if family_style(&n, stem) {
            return (Some(id), FontMatch::Equivalent);
        }
    }
    // A family the fonts folders gave, by its name or its files' (`Roboto`, `Roboto-Bold.ttf`; docs/adr/9313).
    for family in super::families::installed_families() {
        let stem: String = family.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if !stem.is_empty() && family_style(&n, &stem) {
            return (Some(family), FontMatch::Folder);
        }
    }
    (None, FontMatch::Missing)
}

/// Exact family plus known style/weight labels; a prefix is not a metric
/// guarantee (Arial Narrow, Helvetica Neue and Barlow Condensed differ).
fn family_style(name: &str, family: &str) -> bool {
    name.strip_prefix(family).is_some_and(|style| matches!(style,
        "" | "regular" | "normal" | "roman" | "medium" | "semibold" |
        "bold" | "italic" | "oblique" | "bolditalic" | "boldoblique" |
        "400" | "500" | "600" | "700" | "400italic" |
        "bd" | "bi" | "i" | "b" | "z"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_resolve_by_the_rule() {
        assert_eq!(resolve("Barlow-Bold.ttf"), (Some("barlow"), FontMatch::Own));
        assert_eq!(
            resolve("IBM Plex Mono"),
            (Some("plex-mono"), FontMatch::Own)
        );
        assert_eq!(resolve("arialbd.ttf"), (Some("arial"), FontMatch::Own));
        assert_eq!(resolve("Times New Roman"), (Some("times-new-roman"), FontMatch::Own));
        assert_eq!(
            resolve("LiberationSerif-Bold.ttf"),
            (Some("times-new-roman"), FontMatch::Equivalent)
        );
        assert_eq!(
            resolve("couri.ttf"),
            (Some("courier-prime"), FontMatch::Equivalent)
        );
        assert_eq!(
            resolve("Courier Prime"),
            (Some("courier-prime"), FontMatch::Own)
        );
        assert_eq!(resolve("romans.shx"), (None, FontMatch::Shx));
        assert_eq!(resolve("times.ttf"), (Some("times-new-roman"), FontMatch::Own));
        assert_eq!(resolve("calibri.ttf"), (None, FontMatch::Missing));
        assert!(FontMatch::Shx.substituted() && !FontMatch::Equivalent.substituted());
    }

    #[test]
    fn related_names_are_not_proof_of_equal_metrics() {
        for name in ["Arial Narrow", "Arial Black", "Helvetica Neue", "Barlow Condensed",
            "Barlow Semi Condensed", "Overpass Mono", "Arimo Display", "Courtney.ttf",
            "Courier New Condensed", "Arial-custom.ttf", "NotArial.ttf"] {
            assert_eq!(resolve(name), (None, FontMatch::Missing), "{name}");
        }
    }

    #[test]
    fn file_paths_and_recognized_styles_share_the_family_rule() {
        for name in [r"C:\Windows\Fonts\arialbd.ttf", "/fonts/Arial-BoldItalic.otf", "Arial Oblique"] {
            assert_eq!(resolve(name), (Some("arial"), FontMatch::Own), "{name}");
        }
        for name in ["LiberationSans-Regular.ttf", "Helvetica-Bold"] {
            assert_eq!(resolve(name), (Some("arial"), FontMatch::Equivalent), "{name}");
        }
        assert_eq!(resolve("/fonts/Barlow-400-italic.ttf"), (Some("barlow"), FontMatch::Own));
        assert_eq!(resolve(r"C:\Fonts\ROMANS.SHX"), (None, FontMatch::Shx));
    }
}
