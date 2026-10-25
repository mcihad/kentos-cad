//! Shape markers as sprite icons (docs/adr/0635 §9): a shape the `circle` layer cannot draw (a square, a star, a
//! cross …) goes out as a `symbol` layer whose `icon-image` names an icon of the style's sprite, the marker itself in
//! the layer's `metadata.kentos:marker`; the writer lists the icons (`Written.icons`) and its host draws them into the
//! sprite (its PNG and JSON, at 1× and 2×). Read back, such a layer is its marker again; an icon of another sprite
//! (pixels only) is said lost.

use std::cell::RefCell;

use serde_json::{Value, json};

/// An icon of the sprite: a shape marker in screen pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteIcon {
    /// Its name in the sprite (`icon-image`).
    pub id: String,
    pub shape: String,
    /// Its size, CSS pixels.
    pub size: f64,
    /// `#rrggbb(aa)`; none, not filled.
    pub fill: Option<String>,
    /// `#rrggbb(aa)`; none, not outlined.
    pub stroke: Option<String>,
    /// CSS pixels.
    pub stroke_width: f64,
}

thread_local! {
    /// The icons of the style being written.
    static ICONS: RefCell<Vec<SpriteIcon>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn clear() {
    ICONS.with(|i| i.borrow_mut().clear());
}

pub(super) fn take() -> Vec<SpriteIcon> {
    ICONS.with(|i| std::mem::take(&mut *i.borrow_mut()))
}

/// FNV-1a, 64 bits: an icon's name from what it draws.
fn fnv(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The icon of a shape marker (its size and outline in pixels, its colours fixed): its name, listed once.
pub(super) fn icon(
    shape: &str,
    size: f64,
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: f64,
) -> SpriteIcon {
    let (size, stroke_width) = (r3(size), r3(stroke_width));
    let key = format!(
        "{shape}|{size}|{}|{}|{stroke_width}",
        fill.as_deref().unwrap_or("-"),
        stroke.as_deref().unwrap_or("-")
    );
    let id = format!("kentos-{shape}-{:08x}", fnv(&key) as u32);
    let icon = SpriteIcon {
        id: id.clone(),
        shape: shape.to_owned(),
        size,
        fill,
        stroke,
        stroke_width,
    };
    ICONS.with(|i| {
        let mut list = i.borrow_mut();
        if !list.iter().any(|x| x.id == id) {
            list.push(icon.clone());
        }
    });
    icon
}

/// The marker metadata a KentOS icon layer carries.
pub(super) fn metadata(icon: &SpriteIcon) -> Value {
    let mut m = json!({ "shape": icon.shape, "size": icon.size, "unit": "px" });
    if let Some(f) = &icon.fill {
        m["fill"] = json!(f);
    }
    if let Some(s) = &icon.stroke {
        m["stroke"] = json!(s);
        m["strokeWidth"] = json!(icon.stroke_width);
    }
    m
}

/// Whether a style layer is an icon KentOS wrote (its marker in the metadata).
pub(super) fn is_kentos_icon(l: &super::StyleLayer) -> bool {
    l.kind == "symbol"
        && l.layout.contains_key("icon-image")
        && l.metadata
            .get("kentos:marker")
            .is_some_and(Value::is_object)
}

/// The shape marker of a KentOS icon layer: its metadata, turned by `icon-rotate`.
pub(super) fn marker_of(l: &super::StyleLayer) -> Value {
    let mut m = l
        .metadata
        .get("kentos:marker")
        .cloned()
        .unwrap_or_else(|| json!({}));
    m["id"] = json!("p");
    m["type"] = json!("shape");
    if let Some(r) = l
        .layout
        .get("icon-rotate")
        .and_then(Value::as_f64)
        .filter(|r| *r != 0.0)
    {
        // MapLibre turns the icon clockwise, KentOS counter-clockwise.
        m["rotation"] = json!(-r);
    }
    m
}
