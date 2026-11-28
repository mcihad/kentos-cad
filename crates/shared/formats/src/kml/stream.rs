//! A layer's KML written placemark by placemark (docs/adr/9112 §1, §4): the document's styles named first, as KML
//! orders them, then each object moved into WGS 84 and written as it comes, with the style it is given. The bytes are
//! those [`super::write`] makes for a layer whose objects take one style; a layer of a million rows never stands whole
//! in memory. The first write that fails ends the writing; [`LayerStream::finish`] gives it back.

use std::fmt::Write as _;

use kentos_contracts::{Entity, ExportReport, Vec2};
use kentos_geometry_core::crs::{Choice, System, Unreached, transform_in};

use super::write::{KML, TEXT, has_heights, name_of, three_d, write_geometry};
use crate::blocks::Placing;
use crate::export_geom::{Geometry, geometry};
use crate::num::decimals;
use crate::report::Report;
use crate::wgs84::{WGS84, escape, say_unreached, tally};

/// A placemark style, its colours as KML writes them (`aabbggrr`).
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceStyle {
    /// A point's icon colour; none: no IconStyle.
    pub icon: Option<String>,
    /// The icon's size against KML's own (1 its own); none: KML's.
    pub icon_scale: Option<f64>,
    /// The line or an area's outline: its colour and width in pixels; none: an area without an outline.
    pub line: Option<(String, f64)>,
    /// An area's fill; none: an area not filled. Written only when the style is an area's.
    pub fill: Option<String>,
    /// It styles areas: a PolyStyle is written.
    pub area: bool,
}

impl PlaceStyle {
    fn write(&self, id: usize, out: &mut String) {
        let _ = write!(out, "<Style id=\"s{id}\">");
        if let Some(c) = &self.icon {
            let _ = write!(out, "<IconStyle><color>{c}</color>");
            if let Some(k) = self.icon_scale {
                let _ = write!(out, "<scale>{}</scale>", decimals(k, 2));
            }
            out.push_str("</IconStyle>");
        }
        if let Some((c, w)) = &self.line {
            let _ = write!(
                out,
                "<LineStyle><color>{c}</color><width>{}</width></LineStyle>",
                decimals(*w, 2)
            );
        }
        if self.area {
            let (color, fill) = match &self.fill {
                Some(c) => (c.as_str(), 1),
                None => ("00ffffff", 0),
            };
            let outline = u8::from(self.line.is_some());
            let _ = write!(
                out,
                "<PolyStyle><color>{color}</color><fill>{fill}</fill><outline>{outline}</outline></PolyStyle>"
            );
        }
        out.push_str("</Style>\n");
    }
}

/// One layer's objects written as a KML document, a placemark at a time.
pub struct LayerStream<W: std::io::Write> {
    out: W,
    from: System,
    choices: Vec<Choice>,
    blocks: Placing,
    styles: usize,
    rep: Report,
    unreached: Vec<(Unreached, u32)>,
    heights: u32,
    mixed: u32,
    buf: String,
    failed: Option<std::io::Error>,
}

impl<W: std::io::Write> LayerStream<W> {
    /// Begins the document `document` with its styles and the layer's folder `layer`; objects move from `from` into
    /// WGS 84 with `choices`.
    pub fn begin(
        out: W,
        document: &str,
        layer: &str,
        styles: &[PlaceStyle],
        from: System,
        choices: Vec<Choice>,
        blocks: Placing,
    ) -> Self {
        let mut s = LayerStream {
            out,
            from,
            choices,
            blocks,
            styles: styles.len(),
            rep: Report::default(),
            unreached: Vec::new(),
            heights: 0,
            mixed: 0,
            buf: String::with_capacity(256),
            failed: None,
        };
        let mut head = String::with_capacity(256 + styles.len() * 160);
        head.push_str(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document>\n<name>",
        );
        escape(document, &mut head);
        head.push_str("</name>\n");
        for (i, style) in styles.iter().enumerate() {
            style.write(i + 1, &mut head);
        }
        head.push_str("<Folder><name>");
        escape(layer, &mut head);
        head.push_str("</name>\n");
        s.emit(&head);
        s
    }

    fn emit(&mut self, text: &str) {
        if self.failed.is_none()
            && let Err(e) = self.out.write_all(text.as_bytes())
        {
            self.failed = Some(e);
        }
    }

    /// Writes an object as a placemark with the style at `style` (none: none); one it cannot write is left out and
    /// said.
    pub fn put(&mut self, e: &Entity, style: Option<usize>) {
        if self.failed.is_some() {
            return;
        }
        let mut g = match e {
            Entity::Text(t) => {
                self.rep.note("Yazı", TEXT, 0);
                Geometry::Point(t.p, None)
            }
            _ => match geometry(e, &self.blocks, &mut self.rep, &KML) {
                Some(g) => g,
                None => return,
            },
        };
        let mut why = None;
        let (from, choices) = (&self.from, &self.choices);
        let moved = g.map(&mut |p| match transform_in(
            from,
            &WGS84,
            kentos_geometry_core::Vec2::new(p.x, p.y),
            choices,
        ) {
            Ok(t) => Some(Vec2 {
                x: t.point.x,
                y: t.point.y,
            }),
            Err(u) => {
                why = Some(u);
                None
            }
        });
        if !moved {
            if let Some(u) = why {
                tally(&mut self.unreached, u);
            }
            return;
        }
        let z = three_d(&g);
        if z {
            self.heights += 1;
        } else if has_heights(&g) {
            self.mixed += 1;
        }
        let mut out = std::mem::take(&mut self.buf);
        out.clear();
        out.push_str("<Placemark>");
        if let Some(n) = name_of(e) {
            out.push_str("<name>");
            escape(n, &mut out);
            out.push_str("</name>");
        }
        if let Some(i) = style.filter(|i| *i < self.styles) {
            let _ = write!(out, "<styleUrl>#s{}</styleUrl>", i + 1);
        }
        let attrs = &e.base().attrs;
        if !attrs.is_empty() {
            out.push_str("<ExtendedData>");
            for (k, v) in attrs {
                out.push_str("<Data name=\"");
                escape(k, &mut out);
                out.push_str("\"><value>");
                escape(v, &mut out);
                out.push_str("</value></Data>");
            }
            out.push_str("</ExtendedData>");
        }
        write_geometry(&g, z, &mut out);
        out.push_str("</Placemark>\n");
        self.emit(&out);
        self.buf = out;
        self.rep.count(e.kind());
    }

    /// Ends the document: the writer back (or the first write that failed) and what was said.
    pub fn finish(mut self) -> (Result<W, std::io::Error>, ExportReport) {
        self.emit("</Folder>\n</Document>\n</kml>\n");
        say_unreached(&mut self.rep, std::mem::take(&mut self.unreached));
        self.rep.note_n(
            "Kot",
            "kotları deniz seviyesinden yükseklik (absolute) olarak, olduğu gibi yazıldı",
            0,
            self.heights,
        );
        self.rep.note_n(
            "Kotsuz köşe",
            "bir kısım köşesi kotsuz nesne yere oturtularak, kotsuz yazıldı",
            0,
            self.mixed,
        );
        let report = self.rep.export();
        match self.failed {
            Some(e) => (Err(e), report),
            None => (Ok(self.out), report),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{KmlLayer, KmlWriteInput};

    fn wgs84() -> System {
        crate::wgs84::WGS84
    }

    /// The layer's objects written a placemark at a time are the bytes the whole writer makes for them with the one
    /// style it finds.
    #[test]
    fn a_layer_streamed_is_the_whole_writers_bytes() {
        let text = r#"[
          {"kind":"point","id":1,"layerId":"k","attrs":{"Ad":"P1","no":"7"},"p":{"x":37.01,"y":39.75}},
          {"kind":"polygon","id":2,"layerId":"k","attrs":{"ada":"12"},"pts":[{"x":37.0,"y":39.7},{"x":37.0,"y":39.71},{"x":37.01,"y":39.71},{"x":37.01,"y":39.7}]}
        ]"#;
        let entities: Vec<Entity> = serde_json::from_str::<crate::wire::Objects>(text)
            .expect("objects")
            .0;
        let polygon: Vec<Entity> = entities[1..].to_vec();
        let input = KmlWriteInput {
            entities: polygon.clone(),
            layers: vec![KmlLayer {
                id: "k".into(),
                name: "Parseller".into(),
                path: Vec::new(),
                color: "#d9c27a".into(),
                line_weight: None,
            }],
            name: "Belge".into(),
            blocks: Vec::new(),
            kmz: false,
        };
        let (whole, _) = super::super::write::write(&input, &wgs84(), &[]);
        let style = PlaceStyle {
            icon: None,
            icon_scale: None,
            line: Some(("ff7ac2d9".into(), 0.94)),
            fill: Some("667ac2d9".into()),
            area: true,
        };
        let mut s = LayerStream::begin(
            Vec::new(),
            "Belge",
            "Parseller",
            &[style],
            wgs84(),
            Vec::new(),
            Placing::new(&[]),
        );
        for e in &polygon {
            s.put(e, Some(0));
        }
        let (out, report) = s.finish();
        let streamed = out.expect("written");
        assert_eq!(
            String::from_utf8_lossy(&streamed),
            String::from_utf8_lossy(&whole)
        );
        assert_eq!(report.counts.values().sum::<u32>(), 1);
    }
}
