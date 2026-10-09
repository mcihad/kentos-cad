//! Tile address templates (docs/adr/0208 §4, §6): XYZ's `{z}`, `{x}`, `{y}`,
//! `{-y}`, `{s}`, `{r}` and `{quadkey}`; WMTS REST's `{TileMatrixSet}`,
//! `{TileMatrix}`, `{TileRow}`, `{TileCol}`, `{Style}` and its dimensions'
//! names (in any case, as the standard's examples differ); OGC API Tiles'
//! `{tileMatrix}`, `{tileRow}` and `{tileCol}`.

use kentos_geometry_core::geom::tiles::{TileRef, quadkey};

/// Replaces every `{name}` of `template` whose name `value` answers (in any
/// case); the others stay as written.
fn fill(template: &str, mut value: impl FnMut(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let name = &after[..close];
                match value(name) {
                    Some(v) => out.push_str(&v),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// An XYZ tile's address: `{y}` counts rows from the top, or from the
/// bottom when the layer says `y_flip` (TMS); `{-y}` from the bottom
/// always; `{s}` the subdomains in turn by the tile; `{r}` `@2x` on a high
/// density screen.
pub fn xyz(
    template: &str,
    t: TileRef,
    subdomains: &[String],
    y_flip: bool,
    retina: bool,
) -> String {
    let flipped = (1u64 << t.level.min(62))
        .saturating_sub(1)
        .saturating_sub(t.row);
    fill(template, |name| match name {
        "z" => Some(t.level.to_string()),
        "x" => Some(t.col.to_string()),
        "y" => Some(if y_flip { flipped } else { t.row }.to_string()),
        "-y" => Some(flipped.to_string()),
        "s" => (!subdomains.is_empty())
            .then(|| subdomains[((t.col + t.row) % subdomains.len() as u64) as usize].clone()),
        "r" => Some(if retina {
            "@2x".to_owned()
        } else {
            String::new()
        }),
        "quadkey" => Some(quadkey(t)),
        _ => None,
    })
}

/// A WMTS REST tile's address from its `ResourceURL` template.
pub fn wmts(
    template: &str,
    matrix_set: &str,
    matrix: &str,
    row: u64,
    col: u64,
    style: &str,
    dimensions: &[(String, String)],
) -> String {
    fill(template, |name| {
        let n = name.to_ascii_lowercase();
        match n.as_str() {
            "tilematrixset" => Some(matrix_set.to_owned()),
            "tilematrix" => Some(matrix.to_owned()),
            "tilerow" => Some(row.to_string()),
            "tilecol" => Some(col.to_string()),
            "style" => Some(style.to_owned()),
            _ => dimensions
                .iter()
                .find(|(d, _)| d.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone()),
        }
    })
}

/// An OGC API tile's address from its tile set's template.
pub fn ogc(template: &str, matrix: &str, row: u64, col: u64) -> String {
    fill(template, |name| match name {
        "tileMatrix" | "TileMatrix" => Some(matrix.to_owned()),
        "tileRow" | "TileRow" => Some(row.to_string()),
        "tileCol" | "TileCol" => Some(col.to_string()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(level: u32, col: u64, row: u64) -> TileRef {
        TileRef { level, col, row }
    }

    #[test]
    fn xyz_templates() {
        let osm = "https://tile.openstreetmap.org/{z}/{x}/{y}.png";
        assert_eq!(
            xyz(osm, t(3, 4, 2), &[], false, false),
            "https://tile.openstreetmap.org/3/4/2.png"
        );
        // TMS: rows from the bottom.
        assert_eq!(
            xyz(osm, t(3, 4, 2), &[], true, false),
            "https://tile.openstreetmap.org/3/4/5.png"
        );
        assert_eq!(
            xyz("https://x/{z}/{x}/{-y}", t(3, 4, 2), &[], false, false),
            "https://x/3/4/5"
        );
        let subs = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        assert_eq!(
            xyz(
                "https://{s}.x/{z}/{x}/{y}{r}.png",
                t(1, 1, 0),
                &subs,
                false,
                true
            ),
            "https://b.x/1/1/0@2x.png"
        );
        assert_eq!(
            xyz(
                "https://x/{quadkey}?k={unknown}",
                t(3, 3, 5),
                &[],
                false,
                false
            ),
            "https://x/213?k={unknown}"
        );
    }

    #[test]
    fn wmts_and_ogc_templates() {
        let rest =
            "https://x/wmts/{Layer}/{Style}/{TileMatrixSet}/{TileMatrix}/{TileRow}/{TileCol}.png";
        assert_eq!(
            wmts(
                rest,
                "TM30",
                "07",
                12,
                34,
                "default",
                &[("Layer".into(), "ortofoto".into())]
            ),
            "https://x/wmts/ortofoto/default/TM30/07/12/34.png"
        );
        assert_eq!(
            wmts(
                "https://x/{time}/{tilematrix}",
                "s",
                "m",
                0,
                0,
                "",
                &[("Time".into(), "2024".into())]
            ),
            "https://x/2024/m"
        );
        assert_eq!(
            ogc(
                "https://x/tiles/WebMercatorQuad/{tileMatrix}/{tileRow}/{tileCol}?f=mvt",
                "14",
                6209,
                9687
            ),
            "https://x/tiles/WebMercatorQuad/14/6209/9687?f=mvt"
        );
    }
}
