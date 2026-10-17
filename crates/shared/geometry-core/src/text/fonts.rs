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
    let n: String = lower
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if lower.ends_with(".shx") {
        return (None, FontMatch::Shx);
    }
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
        if n.starts_with(stem) {
            return (Some(id), FontMatch::Own);
        }
    }
    for (stem, id) in [
        ("arial", "arimo"),
        ("helvetica", "arimo"),
        ("liberationsans", "arimo"),
        ("cour", "courier-prime"),
    ] {
        if n.starts_with(stem) {
            return (Some(id), FontMatch::Equivalent);
        }
    }
    (None, FontMatch::Missing)
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
}
