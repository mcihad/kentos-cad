//! A text in an embedded SHX font (ADR 1125): its style names a `.shx` (and perhaps a Big Font) the project embeds
//! (ADR 0930, 1120), so its letters are the font's own strokes, the same open polylines on the screen, the CPU
//! picture and the PDF. Never a disk or network lookup; every character resolves or the whole text is refused with
//! the reason, and the caller says so (a finding), never drawing a substitute as if it were the original.
use kentos_commands::{ProjectSettings, ProjectStyles, TextAlign, TextEntity, TextStyleDef, shape_resource};
use kentos_geometry_core::{
    Vec2,
    jsmath::{cos, sin, tan},
    text::{
        fonts,
        paragraph::{PITCH, rise},
        shx::{ShapeFile, TextChar, draw_text_advance, text_chars},
    },
};

/// An attached SHX resource's bytes by the file a definition names (only its basename, ADR 0930).
pub fn attached(resources: &ProjectStyles, file: &str, what: &str) -> Result<Vec<u8>, String> {
    let file = shape_resource::source_name(file);
    let id = shape_resource::id(file);
    let mut items = resources
        .items
        .iter()
        .filter(|i| shape_resource::is_resource(i) && i["id"].as_str() == Some(id.as_str()));
    let item = items
        .next()
        .ok_or_else(|| format!("{what} için “{file}” kaynağı projeye eklenmemiş."))?;
    if items.next().is_some() {
        return Err(format!("“{file}” SHX kaynağı yinelenmiş."));
    }
    shape_resource::read(item)
}

/// Whether a style's font is an SHX file (by the core's rule, `fonts::resolve`).
pub fn is_shx(style: &TextStyleDef) -> bool {
    style
        .font_file
        .as_deref()
        .is_some_and(|f| fonts::resolve(f).1 == fonts::FontMatch::Shx)
}

/// A style's SHX font and Big Font, read from the project's resources.
pub struct Fonts {
    name: String,
    primary: Vec<u8>,
    big: Option<Vec<u8>>,
}

/// One line of SHX characters: open strokes in capital heights from the pen's start (1 is the text's height), and
/// where the pen ends (its advance).
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub paths: Vec<Vec<[f64; 2]>>,
    pub advance: f64,
}

impl Fonts {
    /// The style's fonts; an error names the file that is not embedded, broken or not a text font.
    pub fn of(style: &TextStyleDef, resources: &ProjectStyles) -> Result<Fonts, String> {
        let font_file = style
            .font_file
            .as_deref()
            .ok_or_else(|| format!("“{}” yazı stilinin yazı fontu yok.", style.name))?;
        let what = format!("“{}” yazı stilinin SHX yazı fontu", style.name);
        let primary = attached(resources, font_file, &what)?;
        let file = ShapeFile::read(&primary).map_err(|e| format!("{what}: {e}"))?;
        if file.font().is_none() || file.kind() == kentos_geometry_core::text::shx::Kind::BigFont {
            return Err(format!("{what} (“{font_file}”) bir yazı fontu değil."));
        }
        let big = match style.big_font_file.as_deref() {
            Some(file) => {
                let what = format!("“{}” yazı stilinin Big Font'u", style.name);
                let bytes = attached(resources, file, &what)?;
                let big = ShapeFile::read(&bytes).map_err(|e| format!("{what}: {e}"))?;
                if big.kind() != kentos_geometry_core::text::shx::Kind::BigFont {
                    return Err(format!("{what} (“{file}”) bir Big Font değil."));
                }
                Some(bytes)
            }
            None => None,
        };
        Ok(Fonts {
            name: style.name.clone(),
            primary,
            big,
        })
    }

    /// The fonts read once, for the lines of one text.
    pub fn parsed(&self) -> Result<Parsed<'_>, String> {
        let what = format!("“{}” yazı stilinin SHX yazı fontu", self.name);
        Ok(Parsed {
            name: &self.name,
            primary: ShapeFile::read(&self.primary).map_err(|e| format!("{what}: {e}"))?,
            big: match &self.big {
                Some(b) => Some(ShapeFile::read(b).map_err(|e| format!("Big Font: {e}"))?),
                None => None,
            },
        })
    }

    /// One line of `text`; see [`Parsed::run`].
    pub fn run(&self, text: &str) -> Result<Run, String> {
        self.parsed()?.run(text)
    }
}

/// A style's fonts as read.
pub struct Parsed<'a> {
    name: &'a str,
    primary: ShapeFile<'a>,
    big: Option<ShapeFile<'a>>,
}

impl Parsed<'_> {
    /// One line of `text` (`\M+cXXXX` a Big Font code, ADR 1120 §3). Every character resolves or the line fails:
    /// none becomes a gap. A Unicode character the font lacks is not looked up in the Big Font: the Big Font file
    /// names no code page (ADR 1125 §3).
    pub fn run(&self, text: &str) -> Result<Run, String> {
        let what = format!("“{}” yazı stilinin SHX yazı fontu", self.name);
        let (primary, big) = (&self.primary, &self.big);
        let mut run = Vec::new();
        for c in text_chars(text) {
            match c {
                TextChar::Char(ch) => {
                    let glyph = primary.glyph(ch).map_err(|e| {
                        if big.is_some() && !ch.is_ascii() {
                            format!(
                                "{what}: {e} “{ch}” Big Font'tan aranmadı: Big Font dosyası kod sayfasını bildirmez, \
                                 Unicode karakter bir Big Font koduna doğrulanabilir biçimde çevrilemez. Karakteri \
                                 \\M+ koduyla yazın ya da onu içeren bir unifont seçin."
                            )
                        } else {
                            format!("{what}: {e}")
                        }
                    })?;
                    run.push((primary, glyph));
                }
                TextChar::Big(code) => {
                    let big = big.as_ref().ok_or_else(|| {
                        format!(
                            "“{}” yazı stilinin Big Font'u yok; \\M+ kodlu {code:04X} karakteri çizilemez.",
                            self.name
                        )
                    })?;
                    run.push((big, big.big_glyph(code)?));
                }
            }
        }
        let (paths, end) = draw_text_advance(&run).map_err(|e| format!("{what}: {e}"))?;
        Ok(Run {
            paths: paths
                .into_iter()
                .map(|p| p.into_iter().map(|v| [v.x, v.y]).collect())
                .collect(),
            advance: end.x,
        })
    }
}

/// A text laid out in its SHX font: its strokes on the ground (metres), its box (the outline a text's box has,
/// ADR 0145: from 0.23 of its height under its last baseline to 1.15 over its first) and its box's width.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub paths: Vec<Vec<Vec2>>,
    pub outline: Vec<Vec2>,
    pub width: f64,
}

/// The style a text follows when that style's font is an SHX file and the text does not set its own face; none for
/// any other text (it is drawn as texts always were).
pub fn style_of<'a>(settings: &'a ProjectSettings, t: &TextEntity) -> Option<&'a TextStyleDef> {
    let id = t.face.text_style.as_deref()?;
    let style = settings.text_styles.iter().find(|s| s.id == id)?;
    (is_shx(style) && !t.face.overrides.iter().any(|o| o == "font")).then_some(style)
}

/// `t` in its style's embedded SHX font; none when its style's font is no SHX file. An error says why it cannot be
/// drawn so (its font or Big Font not embedded, a character the fonts lack, a form SHX text does not take here).
pub fn place(settings: &ProjectSettings, resources: &ProjectStyles, t: &TextEntity) -> Option<Result<Placed, String>> {
    let style = style_of(settings, t)?;
    Some(Fonts::of(style, resources).and_then(|f| lay_out(&f, t)))
}

/// A block's text piece as an insert places it (`shape`, the store's `Shape::Text`; `shown`, the words it shows: an
/// attribute's value or its default) made a text for [`place`]/[`place_cached`] (docs/adr/1020): its point,
/// alignment, height, turn, width, box, runs and face are the placed piece's. None for any other shape.
pub fn piece_text(shape: &kentos_geometry_core::entity::Shape, shown: &str) -> Option<TextEntity> {
    let kentos_geometry_core::entity::Shape::Text { face, .. } = shape else {
        return None;
    };
    // The smallest text the contract reads; its fields then come from the placed piece.
    let Ok(kentos_commands::Entity::Text(mut base)) = serde_json::from_value::<kentos_commands::Entity>(serde_json::json!(
        {"kind": "text", "id": 0, "layerId": "", "attrs": {}, "p": {"x": 0.0, "y": 0.0}, "text": "", "height": 1.0,
         "rotation": 0.0}))
    else {
        return None;
    };
    base.face = crate::geometry::contract_face(face.clone());
    let template = kentos_commands::Entity::Text(base);
    let kentos_commands::Entity::Text(mut t) = crate::geometry::with_shape(&template, shape.clone())? else {
        return None;
    };
    shown.clone_into(&mut t.text);
    Some(t)
}

/// [`piece_text`] showing the piece's own words (a multi-line piece: its whole paragraph, docs/adr/1022).
pub fn placed_piece(shape: &kentos_geometry_core::entity::Shape) -> Option<TextEntity> {
    let kentos_geometry_core::entity::Shape::Text { text, .. } = shape else {
        return None;
    };
    piece_text(shape, text)
}

/// A dimension's value as a text for [`place_cached`] when its dimension style (`dim_style`, by id; absent: none)
/// names a text style whose font is an SHX file (docs/adr/1023): centred on its baseline at `at`, turned `rotation`
/// degrees, `height` high (ground), the value's words. None for any other dimension: it is drawn in its typeface.
pub fn dimension_value(
    settings: &ProjectSettings,
    dim_style: Option<&str>,
    at: kentos_commands::Vec2,
    rotation: f64,
    height: f64,
    words: &str,
) -> Option<TextEntity> {
    let dim = settings.dimension_styles.iter().find(|s| Some(s.id.as_str()) == dim_style)?;
    let style = kentos_commands::dimension_text_style(dim, &settings.text_styles).filter(|s| is_shx(s))?;
    let Ok(kentos_commands::Entity::Text(mut t)) = serde_json::from_value::<kentos_commands::Entity>(serde_json::json!(
        {"kind": "text", "id": 0, "layerId": "", "attrs": {}, "p": {"x": at.x, "y": at.y}, "text": words,
         "height": height, "rotation": rotation, "align": "baselineCenter"}))
    else {
        return None;
    };
    t.face.text_style = Some(style.id.clone());
    Some(t)
}

/// A leader's note as a text for [`place_cached`] when its leader style (`leader_style`, by id) names a text style
/// whose font is an SHX file (docs/adr/1025): from its left baseline at `at`, turned `rotation` degrees, `height`
/// high (ground). None for any other leader: its note is drawn in the typeface.
pub fn leader_note(
    settings: &ProjectSettings,
    leader_style: Option<&str>,
    at: kentos_commands::Vec2,
    rotation: f64,
    height: f64,
    words: &str,
) -> Option<TextEntity> {
    let leader = settings.leader_styles.iter().find(|s| Some(s.id.as_str()) == leader_style)?;
    let style = kentos_commands::leader_text_style(leader, &settings.text_styles).filter(|s| is_shx(s))?;
    let Ok(kentos_commands::Entity::Text(mut t)) = serde_json::from_value::<kentos_commands::Entity>(serde_json::json!(
        {"kind": "text", "id": 0, "layerId": "", "attrs": {}, "p": {"x": at.x, "y": at.y}, "text": words,
         "height": height, "rotation": rotation}))
    else {
        return None;
    };
    t.face.text_style = Some(style.id.clone());
    Some(t)
}

/// Words written in `face` from their left baseline at `at` (a table cell's line, docs/adr/1026) as a text for
/// [`place_cached`] when the face's style is an SHX one (and the face sets no font of its own); none otherwise.
pub fn words_in(
    settings: &ProjectSettings,
    face: &kentos_commands::TextFace,
    at: kentos_commands::Vec2,
    rotation: f64,
    height: f64,
    words: &str,
) -> Option<TextEntity> {
    let Ok(kentos_commands::Entity::Text(mut t)) = serde_json::from_value::<kentos_commands::Entity>(serde_json::json!(
        {"kind": "text", "id": 0, "layerId": "", "attrs": {}, "p": {"x": at.x, "y": at.y}, "text": words,
         "height": height, "rotation": rotation}))
    else {
        return None;
    };
    t.face = face.clone();
    style_of(settings, &t)?;
    Some(t)
}

/// The most texts [`place_cached`] keeps per thread before it starts again.
const CACHED: usize = 4096;

thread_local! {
    static PLACED: std::cell::RefCell<std::collections::HashMap<u64, std::rc::Rc<Result<Placed, String>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// [`place`], kept per thread by everything it reads (the text's place and words, its style's fonts and the fonts'
/// bytes by their SHA-256): a view drawn again does not read the fonts again. Same answer as [`place`].
pub fn place_cached(
    settings: &ProjectSettings,
    resources: &ProjectStyles,
    t: &TextEntity,
) -> Option<std::rc::Rc<Result<Placed, String>>> {
    use std::hash::{Hash, Hasher};
    let style = style_of(settings, t)?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let sha = |file: Option<&str>| {
        file.and_then(|f| {
            let id = shape_resource::id(shape_resource::source_name(f));
            resources
                .items
                .iter()
                .find(|i| shape_resource::is_resource(i) && i["id"].as_str() == Some(id.as_str()))
                .and_then(|i| i["sha256"].as_str())
        })
    };
    (&style.name, &style.font_file, &style.big_font_file).hash(&mut h);
    (sha(style.font_file.as_deref()), sha(style.big_font_file.as_deref())).hash(&mut h);
    (&t.text, t.align.map(|a| a as u8), t.path.is_some(), t.paragraph.runs.len()).hash(&mut h);
    (t.paragraph.layout.is_some(), &t.face.overrides).hash(&mut h);
    for v in [
        t.p.x,
        t.p.y,
        t.height,
        t.rotation,
        t.width_factor.unwrap_or(1.),
        t.face.oblique.unwrap_or(0.),
        t.paragraph.box_width.unwrap_or(-1.),
        t.paragraph.line_spacing.unwrap_or(1.),
    ] {
        v.to_bits().hash(&mut h);
    }
    let key = h.finish();
    PLACED.with(|c| {
        if let Some(p) = c.borrow().get(&key) {
            return Some(p.clone());
        }
        let placed = std::rc::Rc::new(Fonts::of(style, resources).and_then(|f| lay_out(&f, t)));
        let mut c = c.borrow_mut();
        if c.len() >= CACHED {
            c.clear();
        }
        c.insert(key, placed.clone());
        Some(placed)
    })
}

/// The lines of `t` (its breaks, then words wrapped to its box) in `fonts`, placed as its alignment puts it.
pub fn lay_out(fonts: &Fonts, t: &TextEntity) -> Result<Placed, String> {
    if t.path.is_some() {
        return Err("Eğri boyunca yazı SHX fontuyla çizilmiyor; düz yazıya çevirin ya da yazı stilini değiştirin.".into());
    }
    if !t.paragraph.runs.is_empty() {
        return Err("Harf biçimli (kalın, eğik, renkli, üst/alt simge) yazı SHX fontuyla çizilmiyor; biçimleri kaldırın.".into());
    }
    if t.paragraph.layout.is_some() {
        return Err("Girinti, sekme, sütun ya da Unicode yerleşim profilli yazı SHX fontuyla çizilmiyor.".into());
    }
    if t.text.contains('\t') {
        return Err("Sekmeli yazı SHX fontuyla çizilmiyor.".into());
    }
    let h = t.height;
    let wf = t.width_factor.unwrap_or(1.);
    let k = tan(t.face.oblique.unwrap_or(0.).to_radians());
    if !(h.is_finite() && h > 0. && wf.is_finite() && wf > 0. && k.is_finite()) {
        return Err("Yazının yüksekliği, genişlik çarpanı ya da eğikliği geçersiz.".into());
    }
    // Lines: its breaks, then (in a box) words wrapped where the next would pass the box, as the legacy layout
    // wraps (`paragraph::wrap`); a line's trailing spaces take no room.
    let fonts = fonts.parsed()?;
    let mut lines: Vec<Run> = Vec::new();
    for para in t.text.split('\n') {
        let para = para.strip_suffix('\r').unwrap_or(para);
        let Some(bw) = t.paragraph.box_width else {
            lines.push(fonts.run(para.trim_end_matches(' '))?);
            continue;
        };
        // The first line from the paragraph's start, each next one from its first word.
        let b = para.as_bytes();
        let (mut start, mut filled, mut i) = (0, None, 0);
        loop {
            while i < b.len() && b[i] == b' ' {
                i += 1;
            }
            if i >= b.len() {
                break;
            }
            let word = i;
            while i < b.len() && b[i] != b' ' {
                i += 1;
            }
            if let Some(end) = filled
                && fonts.run(&para[start..i])?.advance * h * wf > bw
            {
                lines.push(fonts.run(para[start..end].trim_end_matches(' '))?);
                start = word;
            }
            filled = Some(i);
        }
        lines.push(fonts.run(para[start..].trim_end_matches(' '))?);
    }
    let widths: Vec<f64> = lines.iter().map(|l| l.advance * h * wf).collect();
    let width = t
        .paragraph
        .box_width
        .unwrap_or_else(|| widths.iter().copied().fold(0., f64::max));
    let pitch = PITCH * h * t.paragraph.line_spacing.unwrap_or(1.);
    let n = lines.len();
    let along = t.align.map_or(0., align_along);
    let up = t.align.map_or(0., align_up);
    let r = t.rotation.to_radians();
    let (u, v) = (Vec2::new(cos(r), sin(r)), Vec2::new(-sin(r), cos(r)));
    let (dx, dy) = (along * width, rise(up, h, n, pitch));
    let o = Vec2::new(t.p.x - u.x * dx - v.x * dy, t.p.y - u.y * dx - v.y * dy);
    // A point of the text's own frame (along its baseline, up from it), its slant leaning it as its box leans.
    let at = |x: f64, y: f64| {
        let x = x + y * k;
        Vec2::new(o.x + u.x * x + v.x * y, o.y + u.y * x + v.y * y)
    };
    let mut paths = Vec::new();
    for (i, (line, w)) in lines.iter().zip(&widths).enumerate() {
        let shift = along * (width - w);
        let base = -(i as f64) * pitch;
        for p in &line.paths {
            paths.push(p.iter().map(|q| at(shift + q[0] * h * wf, base + q[1] * h)).collect());
        }
    }
    let top = 1.15 * h;
    let bottom = -0.2 * top - (n.saturating_sub(1)) as f64 * pitch;
    Ok(Placed {
        paths,
        outline: vec![at(0., bottom), at(width, bottom), at(width, top), at(0., top)],
        width,
    })
}

/// The contract's alignment as the core's shares (`text::TextAlign::along`/`up`, ADR 0145 §1).
fn align_along(a: TextAlign) -> f64 {
    core_align(a).along()
}
fn align_up(a: TextAlign) -> f64 {
    core_align(a).up()
}
fn core_align(a: TextAlign) -> kentos_geometry_core::text::TextAlign {
    use kentos_geometry_core::text::TextAlign as C;
    match a {
        TextAlign::BaselineCenter => C::BaselineCenter,
        TextAlign::BaselineRight => C::BaselineRight,
        TextAlign::BottomLeft => C::BottomLeft,
        TextAlign::BottomCenter => C::BottomCenter,
        TextAlign::BottomRight => C::BottomRight,
        TextAlign::MiddleLeft => C::MiddleLeft,
        TextAlign::MiddleCenter => C::MiddleCenter,
        TextAlign::MiddleRight => C::MiddleRight,
        TextAlign::TopLeft => C::TopLeft,
        TextAlign::TopCenter => C::TopCenter,
        TextAlign::TopRight => C::TopRight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn resources() -> ProjectStyles {
        let item = |name: &str, bytes: &[u8]| shape_resource::item(name, bytes).unwrap();
        ProjectStyles {
            items: vec![
                item("kentos-uni.shx", include_bytes!("../../../../../fixtures/shx/v1/kentos-uni.shx")),
                item("kentos-big.shx", include_bytes!("../../../../../fixtures/shx/v1/kentos-big.shx")),
            ],
            categories: Vec::new(),
        }
    }

    fn style(fields: Value) -> TextStyleDef {
        let mut s = json!({"id":"shx","name":"SHX","font":"barlow","fontFile":"kentos-uni.shx","bigFontFile":"kentos-big.shx"});
        for (k, v) in fields.as_object().unwrap() {
            if v.is_null() {
                s.as_object_mut().unwrap().remove(k);
            } else {
                s[k] = v.clone();
            }
        }
        serde_json::from_value(s).unwrap()
    }

    fn text(fields: &Value) -> TextEntity {
        let mut t = json!({"id":1,"layerId":"l","attrs":{},"textStyle":"shx"});
        for (k, v) in fields.as_object().unwrap() {
            t[k] = v.clone();
        }
        serde_json::from_value(t).unwrap()
    }

    /// ADR 1125 §6: every shared case of `fixtures/shx/v1/expected.json` (written by
    /// scripts/fixtures/shx_font_reference.py's own reader and layout, no KentOS code) is drawn where the reference
    /// puts it: alignment, turn, width factor, slant, lines, spacing, box wrap, Big Font, far coordinates.
    #[test]
    fn texts_are_placed_as_the_independent_reference_places_them() {
        let expected: Value =
            serde_json::from_str(include_str!("../../../../../fixtures/shx/v1/expected.json")).unwrap();
        let cases = expected["texts"].as_array().unwrap();
        assert!(cases.len() >= 20, "{}", cases.len());
        let fonts = Fonts::of(&style(json!({})), &resources()).unwrap();
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let placed = lay_out(&fonts, &text(&case["text"])).unwrap_or_else(|e| panic!("{name}: {e}"));
            let tol = 1e-9 * (1. + case["text"]["p"]["x"].as_f64().unwrap().abs());
            assert!((placed.width - case["width"].as_f64().unwrap()).abs() < 1e-12, "{name}: width");
            let want = case["paths"].as_array().unwrap();
            assert_eq!(placed.paths.len(), want.len(), "{name}: strokes");
            for (got, want) in placed.paths.iter().zip(want) {
                let want = want.as_array().unwrap();
                assert_eq!(got.len(), want.len(), "{name}");
                for (g, w) in got.iter().zip(want) {
                    let (x, y) = (w[0].as_f64().unwrap(), w[1].as_f64().unwrap());
                    assert!((g.x - x).abs() < tol && (g.y - y).abs() < tol, "{name}: {g:?} vs [{x}, {y}]");
                }
            }
        }
    }

    /// What SHX does not draw is refused with its reason, never a gap or a substitute (ADR 1125 §3).
    #[test]
    fn what_the_fonts_cannot_draw_is_refused_by_name() {
        let r = resources();
        let fonts = Fonts::of(&style(json!({})), &r).unwrap();
        let with = |extra: Value| {
            let mut t = json!({"p":{"x":0.,"y":0.},"height":1.,"rotation":0.});
            for (k, v) in extra.as_object().unwrap() {
                t[k] = v.clone();
            }
            text(&t)
        };
        let refused = |t: TextEntity| lay_out(&fonts, &t).unwrap_err();
        assert!(refused(with(json!({"text":"SAU"}))).contains("U+0041"));
        // A Unicode letter the unifont lacks is not looked for in the Big Font: no code page.
        assert!(refused(with(json!({"text":"Sç"}))).contains("kod sayfasını bildirmez"));
        assert!(refused(with(json!({"text":"S\\M+19999"}))).contains("9999"));
        assert!(refused(with(json!({"text":"S\tU"}))).contains("Sekmeli"));
        assert!(refused(with(json!({"text":"SU","runs":[{"start":0,"end":1,"bold":true}]}))).contains("biçimli"));
        assert!(refused(with(json!({"text":"SU","path":{"pts":[{"x":5.,"y":1.}]}}))).contains("Eğri"));
        // No Big Font: a `\M+` code is refused; a font not embedded names its file.
        let plain = Fonts::of(&style(json!({"bigFontFile":null})), &r).unwrap();
        assert!(lay_out(&plain, &with(json!({"text":"\\M+18140"}))).unwrap_err().contains("Big Font'u yok"));
        assert!(Fonts::of(&style(json!({"fontFile":"romans.shx"})), &r).err().unwrap().contains("romans.shx"));
        assert!(Fonts::of(&style(json!({"bigFontFile":"chinese.shx"})), &r).err().unwrap().contains("chinese.shx"));
        // A Big Font is no text font, a unifont no Big Font.
        assert!(Fonts::of(&style(json!({"fontFile":"kentos-big.shx"})), &r).is_err());
        assert!(Fonts::of(&style(json!({"bigFontFile":"kentos-uni.shx"})), &r).is_err());
    }

    /// Only a style naming an `.shx` takes this path, and a text that sets its own face keeps it; the cache gives
    /// the same answer as the layout and follows a change of the text.
    #[test]
    fn the_style_decides_and_the_cache_follows_the_text() {
        let sample: Value =
            serde_json::from_str(include_str!("../../../../../fixtures/kcad/v2/shx-fonts.json")).unwrap();
        let mut settings: ProjectSettings = serde_json::from_value(sample["settings"].clone()).unwrap();
        settings.text_styles = vec![
            style(json!({})),
            style(json!({"id":"ttf","name":"TTF","fontFile":"arial.ttf","bigFontFile":null})),
        ];
        let r = resources();
        let mut t = text(&json!({"p":{"x":1.,"y":2.},"height":2.,"rotation":0.,"text":"SU"}));
        let direct = lay_out(&Fonts::of(&settings.text_styles[0], &r).unwrap(), &t).unwrap();
        assert_eq!(place_cached(&settings, &r, &t).unwrap().as_ref().as_ref().unwrap(), &direct);
        t.text = "US".into();
        assert_ne!(place_cached(&settings, &r, &t).unwrap().as_ref().as_ref().unwrap(), &direct);
        t.face.text_style = Some("ttf".into());
        assert!(place(&settings, &r, &t).is_none());
        t.face.text_style = Some("shx".into());
        t.face.overrides = vec!["font".into()];
        assert!(place(&settings, &r, &t).is_none());
    }
}
