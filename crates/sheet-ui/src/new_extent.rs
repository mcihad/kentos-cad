//! Yeni pafta's extent (docs/adr/9147): where the template's main map looks and at what scale, chosen in the window
//! of the sheet's questions before the sheet is made — the drawing area as it shows now, one layer's objects, the
//! whole drawing, or the project's plot scale at the drawing area's centre. A fitting choice fits its box with a 5 %
//! margin at the next standard scale, the preflight's own fix (`kentos_sheet::fit::map`); the fixed scale says the
//! scale the whole drawing would need and keeps the cut, deliberately. The main map is the template's first fixed map
//! of the drawing that is no overview; the others follow it as the template has them.

use kentos_sheet::ops::Op;
use kentos_sheet::template::{InstanceIds, InstanceOptions, PaperChoice, Template};
use kentos_sheet::{GroundPoint, Item, ItemKind, MapView};

use crate::designer::Context;

/// The four ways a new sheet's main map is placed, in the window's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Extent {
    /// What the drawing area shows now, fitted.
    View,
    /// One layer's objects, fitted.
    Layer,
    /// The whole drawing, fitted.
    Drawing,
    /// The project's plot scale at the drawing area's centre (what a new sheet did before).
    #[default]
    Fixed,
}

/// The choices' words, in the window's order.
pub const EXTENTS: [(Extent, &str); 4] = [
    (Extent::View, "Görünüm"),
    (Extent::Layer, "Katman"),
    (Extent::Drawing, "Çizimin tamamı"),
    (Extent::Fixed, "Sabit ölçek"),
];

/// Where the main map looks and its scale; none of either: the template's own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placement {
    pub center: Option<GroundPoint>,
    pub scale: Option<u32>,
}

/// A scale as the window says it: 1:1.000.
pub fn scale_words(scale: u32) -> String {
    let digits = scale.to_string();
    let mut out = String::from("1:");
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// What the window's picture shows (docs/adr/9187), in millimetres from the page's top left: the page, the main map's
/// frame on it, and the drawing's box and the chosen box (the drawing area's or the layer's) as the map would show
/// them: a box past the frame is the cut.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picture {
    pub page: [f64; 2],
    /// Left, top, width, height.
    pub frame: [f64; 4],
    pub drawing: Option<[f64; 4]>,
    pub chosen: Option<[f64; 4]>,
}

/// The picture of `placement` (docs/adr/9187): the template's main map at `paper` looking where the placement says.
pub fn picture(
    t: &Template,
    paper: Option<PaperChoice>,
    ctx: &Context,
    extent: Extent,
    layer: Option<usize>,
    placement: Placement,
) -> Option<Picture> {
    let (item, page) = made_main_map(t, paper, placement)?;
    let (center, scale) = match &item.kind {
        ItemKind::Map(m) => match &m.view {
            MapView::Fixed(v) => (v.center?, v.scale),
            _ => return None,
        },
        _ => return None,
    };
    if scale == 0 {
        return None;
    }
    let mm = |um: kentos_sheet::units::Um| f64::from(um) / 1000.0;
    let frame = [mm(item.frame.left), mm(item.frame.top), mm(item.frame.width), mm(item.frame.height)];
    // A metre of ground at 1:N is 1000 / N millimetres of paper; the paper's y runs down.
    let k = 1000.0 / f64::from(scale);
    let (cx, cy) = (frame[0] + frame[2] / 2.0, frame[1] + frame[3] / 2.0);
    let on_paper = |b: [f64; 4]| [cx + (b[0] - center.x) * k, cy - (b[3] - center.y) * k, (b[2] - b[0]) * k, (b[3] - b[1]) * k];
    Some(Picture {
        page: [mm(page.width), mm(page.height)],
        frame,
        drawing: ctx.drawing_bounds.map(on_paper),
        chosen: bounds_of(ctx, extent, layer).map(on_paper),
    })
}

/// The template's main map as the sheet would have it at `paper`, looking at `center` at `scale` (none: the
/// template's): its first fixed map of the drawing that is no overview.
fn main_map(t: &Template, paper: Option<PaperChoice>, place: Placement) -> Option<Item> {
    made_main_map(t, paper, place).map(|(item, _)| item)
}

/// [`main_map`] and the page it is on.
fn made_main_map(
    t: &Template,
    paper: Option<PaperChoice>,
    place: Placement,
) -> Option<(Item, kentos_sheet::units::RectUm)> {
    let ids = InstanceIds {
        sheet: "yeni".into(),
        master: t.master.as_ref().map(|_| "ana".into()),
        items: (0..t.sheet.items.len()).map(|i| format!("o{i}")).collect(),
        master_items: t
            .master
            .iter()
            .flat_map(|m| m.items.iter())
            .enumerate()
            .map(|(i, _)| format!("ao{i}"))
            .collect(),
    };
    let options = InstanceOptions {
        paper,
        name: None,
        center: place.center,
        scale: place.scale,
        values: Vec::new(),
    };
    let made = kentos_sheet::template::instantiate(t, &ids, &options).ok()?;
    let page = made.sheet.page.rect();
    made.sheet
        .items
        .into_iter()
        .find(|it| {
            matches!(&it.kind, ItemKind::Map(m)
                if matches!(m.view, MapView::Fixed(_)) && m.overview_of.is_none() && m.source.is_drawing())
        })
        .map(|item| (item, page))
}

/// Whether the template has a main map to place.
pub fn has_main_map(t: &Template) -> bool {
    main_map(t, None, Placement::default()).is_some()
}

/// The centre and scale `item` takes to hold `bounds`: the preflight's fit.
fn fitted(item: &Item, bounds: [f64; 4]) -> Option<(GroundPoint, u32)> {
    match kentos_sheet::fit::map(item, bounds, 5, true).ok()? {
        Op::SetItemProps(p) => {
            let view = &p.patch["kind"]["view"];
            let center: GroundPoint = serde_json::from_value(view["center"].clone()).ok()?;
            let scale = u32::try_from(view["scale"].as_u64()?).ok()?;
            Some((center, scale))
        }
        _ => None,
    }
}

/// The box a choice fits: the drawing area's, the chosen layer's, the drawing's; none for the fixed scale.
fn bounds_of(ctx: &Context, extent: Extent, layer: Option<usize>) -> Option<[f64; 4]> {
    match extent {
        Extent::View => {
            let (c, [w, h]) = (ctx.center?, ctx.view_size?);
            Some([c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0])
        }
        Extent::Layer => layer.and_then(|i| ctx.layer_bounds.get(i)).map(|(_, b)| *b),
        Extent::Drawing => ctx.drawing_bounds,
        Extent::Fixed => None,
    }
}

/// The layer's name a list of layer boxes names (its path), else its id.
pub fn layer_name(ctx: &Context, i: usize) -> String {
    let Some((id, _)) = ctx.layer_bounds.get(i) else {
        return String::new();
    };
    ctx.layers.iter().find(|(l, _)| l == id).map_or_else(|| id.clone(), |(_, name)| name.clone())
}

/// The placement a choice gives the template at `paper`, and the sentence that says it.
pub fn place(
    t: &Template,
    paper: Option<PaperChoice>,
    ctx: &Context,
    extent: Extent,
    layer: Option<usize>,
) -> (Placement, String) {
    let fixed = Placement {
        center: ctx.center,
        scale: ctx.capabilities.plot_scale,
    };
    let Some(item) = main_map(t, paper, fixed) else {
        return (fixed, "Şablonun yerleştirilecek bir haritası yok.".to_owned());
    };
    if extent == Extent::Fixed {
        let at = match item.kind {
            ItemKind::Map(ref m) => match &m.view {
                MapView::Fixed(v) => v.scale,
                _ => 0,
            },
            _ => 0,
        };
        let mut words = format!("Ana harita {}, çizim alanının ortasında.", scale_words(at));
        if let Some(b) = ctx.drawing_bounds {
            match kentos_sheet::preflight::drawing_overrun(&item, b) {
                Some(_) => {
                    let need = fitted(&item, b).map(|(_, s)| scale_words(s)).unwrap_or_default();
                    words.push_str(&format!(
                        " Çizimin bir kısmı dışarıda kalır; tamamı {need} ölçeğinde sığar. Kırpma bilerek korunur."
                    ));
                }
                None => words.push_str(" Çizimin tamamı içinde."),
            }
        }
        return (fixed, words);
    }
    let what = match extent {
        Extent::View => "Görünümdeki alan".to_owned(),
        Extent::Layer => match layer {
            Some(i) => format!("{} katmanının nesneleri", layer_name(ctx, i)),
            None => return (fixed, "Bir katman seçin.".to_owned()),
        },
        _ => "Çizimin tamamı".to_owned(),
    };
    match bounds_of(ctx, extent, layer).and_then(|b| fitted(&item, b)) {
        Some((center, scale)) => (
            Placement {
                center: Some(center),
                scale: Some(scale),
            },
            format!("{what} %5 payla sığar: ana harita {}.", scale_words(scale)),
        ),
        None => (fixed, format!("{what} sığdırılamadı: kapsamı bilinmiyor ya da desteklenen ölçeğe sığmıyor.")),
    }
}
