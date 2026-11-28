//! A style's colour text as the reader takes it (docs/adr/0635): `#rgb` … `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`,
//! `hsla()` and every colour name of CSS, each as a colour (`#rrggbb`) and its opacity. A colour a style says that is
//! no colour is told, never drawn black without a word (docs/adr/9401 §3).

use kentos_formats::sld::colour;
use serde_json::{Value, json};

use crate::{Prop, prop_of};

/// A CSS colour as a colour and its opacity.
pub(crate) fn css_in(text: &str) -> Option<(String, f64)> {
    let t = text.trim().to_ascii_lowercase();
    if t.starts_with('#') {
        return colour(&json!(t));
    }
    // A name: `transparent`, or one of CSS's colour names (the style specification permits them all). The table is the
    // SVG core's; this crate keeps none of its own.
    if !t.is_empty() && t.bytes().all(|b| b.is_ascii_lowercase()) {
        if t == "transparent" {
            return Some(("#000000".to_owned(), 0.0));
        }
        return kentos_svg_core::values::read_color(&t)
            .map(|c| (c.hex.to_ascii_lowercase(), c.alpha));
    }
    let (func, args) = t.split_once('(')?;
    let args = args.strip_suffix(')')?;
    let nums: Vec<&str> = args
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    let part = |s: &str, full: f64| -> Option<f64> {
        match s.strip_suffix('%') {
            Some(p) => p.parse::<f64>().ok().map(|p| p / 100.0 * full),
            None => s.trim_end_matches("deg").parse::<f64>().ok(),
        }
    };
    let alpha = |i: usize| -> Option<f64> {
        match nums.get(i) {
            Some(s) => part(s, 1.0).map(|a| a.clamp(0.0, 1.0)),
            None => Some(1.0),
        }
    };
    match func {
        "rgb" | "rgba" if nums.len() >= 3 => {
            let c = |i: usize| part(nums[i], 255.0).map(|x| x.round().clamp(0.0, 255.0) as u8);
            Some((
                format!("#{:02x}{:02x}{:02x}", c(0)?, c(1)?, c(2)?),
                alpha(3)?,
            ))
        }
        "hsl" | "hsla" if nums.len() >= 3 => {
            let h = part(nums[0], 360.0)?.rem_euclid(360.0) / 360.0;
            let s = part(nums[1], 1.0)?.clamp(0.0, 1.0);
            let l = part(nums[2], 1.0)?.clamp(0.0, 1.0);
            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;
            let channel = |mut t: f64| {
                if t < 0.0 {
                    t += 1.0;
                }
                if t > 1.0 {
                    t -= 1.0;
                }
                let v = if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 0.5 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                };
                (v * 255.0).round().clamp(0.0, 255.0) as u8
            };
            Some((
                format!(
                    "#{:02x}{:02x}{:02x}",
                    channel(h + 1.0 / 3.0),
                    channel(h),
                    channel(h - 1.0 / 3.0)
                ),
                alpha(3)?,
            ))
        }
        _ => None,
    }
}

/// A value as a colour's text: a text that reads as a colour, or a constant expression that gives one
/// (`["rgb", r, g, b]`, `["to-color", "red"]`). What is no colour is told with what the reader does without it (`then`).
fn text_of(v: &Value, key: &str, then: &str, lost: &mut Vec<String>, what: &str) -> Option<String> {
    let text = match v {
        Value::String(c) => c.clone(),
        // An expression the reader does not know is told there.
        other => match prop_of(other, key, lost, what)? {
            Prop::Const(Value::String(c)) => c,
            _ => {
                lost.push(format!(
                    "{what}: “{key}” değerindeki {other} bir renk değil; {then}"
                ));
                return None;
            }
        },
    };
    if css_in(&text).is_none() {
        lost.push(format!(
            "{what}: “{key}” değerindeki “{text}” rengi okunamadı; {then}"
        ));
        return None;
    }
    Some(text)
}

/// A colour key's values settled once, as a layer is read (docs/adr/0560 “Boya değerleri”, docs/adr/9401 §3): every
/// value the key can take must be a colour: a constant, each arm and the fallback of a `match`, the base and each
/// stop of a `step`, a field's fallback. An arm's or a stop's own constant expression becomes its colour's text, so
/// every class reads a text. The first value that is no colour is told and the answer is false: the caller leaves
/// the value out (its default stays), and the layer when the colour is its own.
pub(crate) fn settle(p: &mut Prop, key: &str, lost: &mut Vec<String>, what: &str) -> bool {
    let colour =
        |v: &mut Value, lost: &mut Vec<String>| match text_of(v, key, "değer alınmadı", lost, what)
        {
            Some(text) => {
                *v = Value::String(text);
                true
            }
            None => false,
        };
    match p {
        Prop::Const(v) => colour(v, lost),
        // A field's colour with no fallback says none.
        Prop::Data { fallback, .. } => fallback.is_null() || colour(fallback, lost),
        Prop::Match { arms, fallback, .. } => {
            arms.iter_mut().all(|(_, v)| colour(v, lost)) && colour(fallback, lost)
        }
        Prop::Step { base, stops, .. } => {
            colour(base, lost) && stops.iter_mut().all(|(_, v)| colour(v, lost))
        }
        Prop::Metres(_) => {
            lost.push(format!("{what}: “{key}” bir renk değil; değer alınmadı"));
            false
        }
    }
}

/// The colour a label's halo or a heat map's stop says with `v`; none when it says none. One it says that is no
/// colour is told with what the reader does without it (`then`), and is none.
pub(crate) fn said(
    v: Option<&Value>,
    key: &str,
    then: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<(String, f64)> {
    let text = text_of(v.filter(|v| !v.is_null())?, key, then, lost, what)?;
    css_in(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StyleLayer, parse, renderer_of};

    fn read(layers: &str) -> crate::Read {
        let text = format!(r#"{{ "version": 8, "sources": {{}}, "layers": [{layers}] }}"#);
        let style = parse(&text).expect("parses");
        let all: Vec<&StyleLayer> = style.layers.iter().collect();
        renderer_of(&all).expect("reads")
    }

    /// Every colour name of CSS reads; the names the reader knew before read as they did.
    #[test]
    fn every_colour_name_reads() {
        assert_eq!(css_in("steelblue"), Some(("#4682b4".into(), 1.0)));
        assert_eq!(css_in(" RebeccaPurple "), Some(("#663399".into(), 1.0)));
        assert_eq!(css_in("transparent"), Some(("#000000".into(), 0.0)));
        for (name, rgb) in [
            ("black", "#000000"),
            ("white", "#ffffff"),
            ("red", "#ff0000"),
            ("green", "#008000"),
            ("lime", "#00ff00"),
            ("blue", "#0000ff"),
            ("yellow", "#ffff00"),
            ("cyan", "#00ffff"),
            ("aqua", "#00ffff"),
            ("magenta", "#ff00ff"),
            ("fuchsia", "#ff00ff"),
            ("gray", "#808080"),
            ("grey", "#808080"),
            ("orange", "#ffa500"),
            ("purple", "#800080"),
            ("pink", "#ffc0cb"),
            ("brown", "#a52a2a"),
        ] {
            assert_eq!(css_in(name), Some((rgb.into(), 1.0)), "{name}");
        }
        for none in [
            "",
            "no-such-colour",
            "colourless",
            "rgb(1,2",
            "currentcolor",
            "none",
        ] {
            assert_eq!(css_in(none), None, "{none}");
        }
    }

    /// A colour said by a name was drawn black without a word (docs/adr/9401 §3): a fill, its outline, a circle and
    /// the arms of a `match`, an arm's own constant expression among them.
    #[test]
    fn a_named_colour_is_drawn_as_itself() {
        let r = read(
            r#"{ "id": "su", "type": "fill", "source": "s", "paint": { "fill-color": "steelblue", "fill-outline-color": "navy" } }"#,
        );
        let layers = &r.renderer["symbols"]["fill"]["layers"];
        assert_eq!(layers[0]["color"], "#4682b4");
        assert_eq!(layers[1]["color"], "#000080");
        assert!(r.lost.is_empty(), "{:?}", r.lost);
        let r = read(
            r#"{ "id": "nokta", "type": "circle", "source": "s", "paint": { "circle-color": "tomato", "circle-stroke-color": "white", "circle-stroke-width": 2 } }"#,
        );
        let m = &r.renderer["symbols"]["marker"]["layers"][0];
        assert_eq!(
            (&m["fill"], &m["stroke"]),
            (&json!("#ff6347"), &json!("#ffffff"))
        );
        assert!(r.lost.is_empty(), "{:?}", r.lost);
        let r = read(
            r#"{ "id": "tur", "type": "fill", "source": "s", "paint": { "fill-color":
                ["match", ["get", "tur"], "a", "olive", "b", ["rgb", 1, 2, 3], "c", ["to-color", "crimson"], "teal"] } }"#,
        );
        let of = |v: &Value| v["fill"]["layers"][0]["color"].clone();
        assert_eq!(of(&r.renderer["categories"][0]["symbols"]), "#808000");
        assert_eq!(of(&r.renderer["categories"][1]["symbols"]), "#010203");
        assert_eq!(of(&r.renderer["categories"][2]["symbols"]), "#dc143c");
        assert_eq!(of(&r.renderer["other"]), "#008080");
        assert!(r.lost.is_empty(), "{:?}", r.lost);
    }

    /// A layer whose own colour is no colour is left out, as an expression the reader does not know leaves it out
    /// (docs/adr/0560): it would draw something else. Another colour that is none keeps its default. Both are told.
    #[test]
    fn a_colour_that_does_not_read_is_told() {
        let r = read(
            r#"{ "id": "su", "type": "fill", "source": "s", "paint": { "fill-color": "olive", "fill-outline-color": "bogus" } },
               { "id": "yol", "type": "line", "source": "s", "paint": { "line-color": "no-such-colour" } },
               { "id": "tur", "type": "fill", "source": "s", "paint": { "fill-color": ["match", ["get", "tur"], "a", "olive", "b", "bogus", "gray"] } },
               { "id": "sayi", "type": "fill", "source": "s", "paint": { "fill-color": "teal", "fill-outline-color": 5 } }"#,
        );
        assert_eq!(
            r.lost,
            [
                "“su” katmanı: “fill-outline-color” değerindeki “bogus” rengi okunamadı; değer alınmadı",
                "“yol” katmanı: “line-color” değerindeki “no-such-colour” rengi okunamadı; değer alınmadı",
                "“yol” katmanı: “line-color” okunamadığı için katman alınmadı",
                "“tur” katmanı: “fill-color” değerindeki “bogus” rengi okunamadı; değer alınmadı",
                "“tur” katmanı: “fill-color” okunamadığı için katman alınmadı",
                "“sayi” katmanı: “fill-outline-color” değerindeki 5 bir renk değil; değer alınmadı",
            ]
        );
        // The two fills that read, each without its outline.
        let layers = r.renderer["symbols"]["fill"]["layers"]
            .as_array()
            .expect("layers");
        let colours: Vec<&Value> = layers.iter().map(|l| &l["color"]).collect();
        assert_eq!(colours, [&json!("#808000"), &json!("#008080")]);
    }

    /// A label's halo and a heat map's stop.
    #[test]
    fn a_halo_and_a_stop_that_do_not_read_are_told() {
        let r = read(
            r#"{ "id": "ad", "type": "symbol", "source": "s", "layout": { "text-field": "{ad}" },
                "paint": { "text-color": "darkslategray", "text-halo-color": "bogus", "text-halo-width": 1 } }"#,
        );
        let style = &r.labels.as_ref().expect("a label class").labels["classes"][0]["style"];
        assert_eq!(style["color"], "#2f4f4f");
        assert_eq!(
            r.lost,
            ["“ad” etiketi: “text-halo-color” değerindeki “bogus” rengi okunamadı; siyah alındı"]
        );
        // The ramp keeps the colours that read: it was all transparent before.
        let r = read(
            r#"{ "id": "isi", "type": "heatmap", "source": "s", "paint": { "heatmap-color":
                ["interpolate", ["linear"], ["heatmap-density"], 0, "transparent", 0.5, "bogus", 1, "crimson"] } }"#,
        );
        assert_eq!(r.renderer["ramp"], json!(["#00000000", "#dc143c"]));
        assert!(
            r.lost.iter().any(|l| l
                .contains("“heatmap-color” değerindeki “bogus” rengi okunamadı; durak atlandı")),
            "{:?}",
            r.lost
        );
    }
}
