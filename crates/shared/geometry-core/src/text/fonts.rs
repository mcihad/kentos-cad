//! Which of KentOS's drawing typefaces a typeface named by a file or a family is drawn in (docs/adr/0324 §5): one
//! rule for a DXF's STYLE, a style's file and `cad.fonts.report`. KentOS draws, measures and embeds only its own
//! faces (the tables of `metrics.rs`, the TrueType files a PDF embeds as subsets); any other face is drawn in one of
//! them and said, never silently.

/// How a named typeface is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontMatch {
    /// One of KentOS's own faces by its name.
    Own,
    /// A face KentOS has a metric twin of (Arial, Helvetica and Liberation Sans as Arimo, Courier as Courier Prime):
    /// its letters as wide, its look close.
    Equivalent,
    /// An AutoCAD SHX line font (`romans.shx`, a big font): KentOS does not read SHX; the project's face takes it.
    Shx,
    /// A face KentOS does not have (Times, Calibri …): the project's face takes it.
    Missing,
}

impl FontMatch {
    pub fn name(self) -> &'static str {
        match self {
            FontMatch::Own => "own",
            FontMatch::Equivalent => "equivalent",
            FontMatch::Shx => "shx",
            FontMatch::Missing => "missing",
        }
    }

    /// Whether the face drawn is not the one named.
    pub fn substituted(self) -> bool {
        matches!(self, FontMatch::Shx | FontMatch::Missing)
    }
}

/// KentOS's face for a typeface named `name` (a file, `arialbd.ttf`, `romans.shx`, or a family, `Arial Narrow`), as
/// a `DrawingFont` id, and how; none for one drawn in the project's face.
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
    ] {
        if family_style(&n, stem) {
            return (Some(id), FontMatch::Own);
        }
    }
    for (stem, id) in [
        ("arial", "arimo"),
        ("helvetica", "arimo"),
        ("liberationsans", "arimo"),
        ("couriernew", "courier-prime"),
        ("courier", "courier-prime"),
        ("cour", "courier-prime"),
    ] {
        if family_style(&n, stem) {
            return (Some(id), FontMatch::Equivalent);
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
        assert_eq!(
            resolve("arialbd.ttf"),
            (Some("arimo"), FontMatch::Equivalent)
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
        assert_eq!(resolve("times.ttf"), (None, FontMatch::Missing));
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
        for name in [r"C:\Windows\Fonts\arialbd.ttf", "/fonts/Arial-BoldItalic.otf",
            "Arial Oblique", "LiberationSans-Regular.ttf", "Helvetica-Bold"] {
            assert_eq!(resolve(name), (Some("arimo"), FontMatch::Equivalent), "{name}");
        }
        assert_eq!(resolve("/fonts/Barlow-400-italic.ttf"), (Some("barlow"), FontMatch::Own));
        assert_eq!(resolve(r"C:\Fonts\ROMANS.SHX"), (None, FontMatch::Shx));
    }
}
