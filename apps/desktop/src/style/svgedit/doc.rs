//! The SVG editor's drawing (the web's `SvgDoc`, `style/svg/svgModel.ts`):
//! a canvas and its shapes back to front, the symbol's intended width in
//! mm, the preview's paper and the editor's guides. Shapes stay the SVG
//! core's JSON objects (`Obj`), field for field and in order, so what the
//! core does to them is the web's to the letter and the drawing's text (the
//! history, the unsaved check) matches the web's.

use std::sync::atomic::{AtomicU64, Ordering};

use kentos_geometry_core::api::json::{FromJson, Json, ToJson};
use kentos_svg_core::shape::Obj;

/// A guide: an endless line through (x, y) at `angle` degrees from the x axis (0 horizontal, 90 vertical).
#[derive(Clone, Debug, PartialEq)]
pub struct GuideLine {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

/// The drawing being edited.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub width: f64,
    pub height: f64,
    pub shapes: Vec<Obj>,
    /// Intended width of the drawing on the map in mm (Belge özellikleri).
    pub size_mm: Option<f64>,
    /// The preview's paper kept with the drawing; none: the theme's.
    pub background: Option<String>,
    /// Snap lines of the editor; the SVG file does not keep them.
    pub guides: Vec<GuideLine>,
}

static SEQ: AtomicU64 = AtomicU64::new(0);

/// A fresh shape (or group, or guide) id, as the web's `shapeId`: `s`, the
/// time and a sequence in base 36.
pub fn shape_id() -> String {
    fn base36(mut n: u128) -> String {
        const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut out = Vec::new();
        loop {
            out.push(DIGITS[(n % 36) as usize]);
            n /= 36;
            if n == 0 {
                break;
            }
        }
        out.reverse();
        String::from_utf8(out).unwrap_or_default()
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("s{}{}", base36(ms), base36(u128::from(seq)))
}

impl Drawing {
    /// A blank canvas.
    pub fn new(width: f64, height: f64) -> Drawing {
        Drawing {
            width,
            height,
            shapes: Vec::new(),
            size_mm: None,
            background: None,
            guides: Vec::new(),
        }
    }

    /// The drawing as the core reads it (`SvgDoc`): width, height, shapes,
    /// then what is set of sizeMm, background and guides.
    pub fn to_obj(&self) -> Obj {
        let mut o = Obj::default();
        o.set_num("width", self.width);
        o.set_num("height", self.height);
        o.set(
            "shapes",
            Json::Arr(self.shapes.iter().map(obj_tree).collect()),
        );
        if let Some(mm) = self.size_mm {
            o.set_num("sizeMm", mm);
        }
        if let Some(b) = &self.background {
            o.set_text("background", b);
        }
        if !self.guides.is_empty() {
            o.set(
                "guides",
                Json::Arr(
                    self.guides
                        .iter()
                        .map(|g| {
                            Json::Obj(vec![
                                ("id".into(), Json::Str(g.id.clone())),
                                ("x".into(), Json::Num(g.x)),
                                ("y".into(), Json::Num(g.y)),
                                ("angle".into(), Json::Num(g.angle)),
                            ])
                        })
                        .collect(),
                ),
            );
        }
        o
    }

    /// A drawing from the core's object (an import, the XML source).
    pub fn from_obj(o: &Obj) -> Result<Drawing, String> {
        let shapes = match o.get("shapes") {
            Json::Arr(items) => items
                .iter()
                .map(Obj::from_json)
                .collect::<Result<Vec<_>, _>>()?,
            _ => Vec::new(),
        };
        let guides = match o.get("guides") {
            Json::Arr(items) => items
                .iter()
                .map(|g| GuideLine {
                    id: match g.get("id") {
                        Json::Str(s) => s.clone(),
                        _ => shape_id(),
                    },
                    x: f64::from_json(g.get("x")).unwrap_or(0.0),
                    y: f64::from_json(g.get("y")).unwrap_or(0.0),
                    angle: f64::from_json(g.get("angle")).unwrap_or(0.0),
                })
                .collect(),
            _ => Vec::new(),
        };
        Ok(Drawing {
            width: o.opt_num("width").unwrap_or(100.0),
            height: o.opt_num("height").unwrap_or(100.0),
            shapes,
            size_mm: o.opt_num("sizeMm"),
            background: o.text("background").map(str::to_owned),
            guides,
        })
    }

    /// The drawing's text: what the history keeps and the unsaved check compares.
    pub fn text(&self) -> String {
        kentos_geometry_core::api::json::to_string(&self.to_obj())
    }

    /// A drawing from its text (the history).
    pub fn from_text(text: &str) -> Option<Drawing> {
        let json = Json::parse(text).ok()?;
        Drawing::from_obj(&Obj::from_json(&json).ok()?).ok()
    }

    pub fn shape(&self, id: &str) -> Option<&Obj> {
        self.shapes.iter().find(|s| s.text("id") == Some(id))
    }

    /// Whether a shape is drawn: every shape but the hidden ones counts for Kaydet.
    pub fn has_visible(&self) -> bool {
        self.shapes.iter().any(|s| !s.is("hidden"))
    }
}

/// A shape's JSON tree (fields left undefined are left out, as the text leaves them).
pub fn obj_tree(o: &Obj) -> Json {
    Json::Obj(
        o.0.iter()
            .filter_map(|(k, v)| v.clone().map(|v| (k.clone(), v)))
            .collect(),
    )
}

/// A shape's id.
pub fn id_of(s: &Obj) -> &str {
    s.text("id").unwrap_or("")
}

/// What the list calls a shape kind (`KIND`).
pub fn kind_name(kind: &str) -> &'static str {
    match kind {
        "rect" => "Dikdörtgen",
        "ellipse" => "Elips",
        "text" => "Yazı",
        _ => "Yol",
    }
}

/// A shape's name in the list: its own, or its kind (a text by its words) (`shapeName`).
pub fn shape_name(s: &Obj) -> String {
    if let Some(n) = s.text("name") {
        return n.to_owned();
    }
    if s.kind() == "text" {
        return format!("Yazı “{}”", s.text("text").unwrap_or(""));
    }
    kind_name(s.kind()).to_owned()
}

/// Whether two shapes are the same field for field (the web's JSON comparison).
pub fn same(a: &Obj, b: &Obj) -> bool {
    let mut x = String::new();
    let mut y = String::new();
    a.write_json(&mut x);
    b.write_json(&mut y);
    x == y
}
