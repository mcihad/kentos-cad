//! Explicit access to the one model through a paper viewport. All lengths here are µm/metres.
use crate::{GroundPoint, Item, ItemKind, MapView, Result, SheetError, SheetBook};
use crate::projection::Transform;
use crate::units::{RectUm, sin_cos};

#[derive(Clone, Debug, PartialEq)]
pub struct Viewport {
    pub sheet: Option<String>,
    pub item: String,
    pub content: RectUm,
    pub center: GroundPoint,
    pub denominator: f64,
    pub annotation_scale: f64,
    pub layers: crate::kinds::MapLayers,
    pub layer_overrides: Vec<crate::kinds::LayerOverride>,
    pub rotation: i32,
    pivot: [f64; 2],
    frame_rotation: i32,
    clip: crate::clip::Clip,
}
impl Viewport {
    pub fn resolve(book: &SheetBook, sheet: &str, id: &str) -> Result<Self> {
        Self::resolve_with_views(book, sheet, id, &[])
    }
    pub fn resolve_with_views(
        book: &SheetBook,
        sheet: &str,
        id: &str,
        views: &[crate::viewport::NamedModelView],
    ) -> Result<Self> {
        let s = book
            .sheets
            .iter()
            .find(|s| s.id == sheet)
            .ok_or_else(|| SheetError::new("unknown_sheet", "Pafta bulunamadı."))?;
        let item = s
            .items
            .iter()
            .find(|i| i.id == id)
            .ok_or_else(|| SheetError::new("scope_mismatch", "Görünüm bu paftada değil."))?;
        let mut viewport = Self::of_with_views(item, views)?;
        viewport.sheet = Some(sheet.to_owned());
        Ok(viewport)
    }
    pub fn of(item: &Item) -> Result<Self> {
        Self::of_with_views(item, &[])
    }
    pub fn of_with_views(item: &Item, views: &[crate::viewport::NamedModelView]) -> Result<Self> {
        let ItemKind::Map(map) = &item.kind else {
            return Err(SheetError::new(
                "not_viewport",
                "Öğe bir model görünümü değil.",
            ));
        };
        let MapView::Fixed(_) = &map.view else {
            return Err(SheetError::new(
                "atlas_scope",
                "Atlas sayfası model düzenleme kapsamı olamaz.",
            ));
        };
        let view = crate::viewport::effective_fixed_view(item, views)?;
        let center = view
            .center
            .ok_or_else(|| SheetError::new("unplaced_viewport", "Görünümün model merkezi yok."))?;
        Ok(Self {
            sheet: None,
            item: item.id.clone(),
            content: crate::display::map::content_rect(item, map),
            center,
            denominator: view.denominator(),
            annotation_scale: map.annotation_scale.unwrap_or(view.denominator()),
            layers: map.layers.clone(),
            layer_overrides: map.layer_overrides.clone(),
            rotation: view.rotation,
            pivot: item.frame.center(),
            frame_rotation: item.rotation,
            clip: map.clip.clone(),
        })
    }
    /// The model boundary, unrotated from the paper's frame. Original model coordinates are retained.
    pub fn model_boundary(&self) -> Vec<[f64; 2]> {
        self.clip
            .paper_points(&self.content)
            .into_iter()
            .map(|p| self.transform().inverse(p))
            .collect()
    }
    pub fn contains_model(&self, point: [f64; 2]) -> bool {
        let p = self.transform().point(point);
        self.clip.contains(
            (p[0] - f64::from(self.content.left)) / f64::from(self.content.width),
            (p[1] - f64::from(self.content.top)) / f64::from(self.content.height),
        )
    }
    fn rotate(&self, point: [f64; 2], angle: i32) -> [f64; 2] {
        let (s, c) = sin_cos(angle);
        let (x, y) = (point[0] - self.pivot[0], point[1] - self.pivot[1]);
        [self.pivot[0] + x * c - y * s, self.pivot[1] + x * s + y * c]
    }
    pub fn transform(&self) -> Transform {
        Transform::paper(&self.content, self.center, self.denominator, self.rotation)
    }
    pub fn model_point(&self, paper: [f64; 2]) -> Option<[f64; 2]> {
        let p = self.rotate(paper, -self.frame_rotation);
        let x = (p[0] - f64::from(self.content.left)) / f64::from(self.content.width);
        let y = (p[1] - f64::from(self.content.top)) / f64::from(self.content.height);
        self.clip
            .contains(x, y)
            .then(|| self.transform().inverse(p))
    }
    /// Also for a captured drag ending beyond the boundary. A new press must use model_point.
    pub fn model_unclipped(&self, paper: [f64; 2]) -> [f64; 2] {
        self.transform()
            .inverse(self.rotate(paper, -self.frame_rotation))
    }
    pub fn paper_point(&self, model: [f64; 2]) -> [f64; 2] {
        self.rotate(self.transform().point(model), self.frame_rotation)
    }
    pub fn total_rotation(&self) -> i32 {
        crate::units::norm_mdeg(i64::from(self.rotation) + i64::from(self.frame_rotation))
    }
}

/// A paper-space pan and anchor-preserving zoom; model geometry is never scaled.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NavigateViewport {
    pub id: String,
    /// Screen-right/down displacement expressed as paper µm, after frame rotation.
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
    pub delta: [f64; 2],
    /// Greater than one zooms in.
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
    pub factor: f64,
    /// Absolute paper µm; absent: the content centre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
    pub anchor: Option<[f64; 2]>,
}
pub fn navigate(item: &Item, input: &NavigateViewport) -> Result<crate::ops::Op> {
    if matches!(&item.kind,ItemKind::Map(m) if m.source.is_link()) {
        return Err(SheetError::new(
            "linked_view",
            "Kaydırmak için önce canlı görünüm bağını Bu çizim yapın.",
        ));
    }
    let v = Viewport::of(item)?;
    if !input.delta.iter().all(|x| x.is_finite())
        || !input.factor.is_finite()
        || input.factor <= 0.
        || input
            .anchor
            .is_some_and(|p| !p.iter().all(|x| x.is_finite()))
    {
        return Err(SheetError::new(
            "bad_navigation",
            "Kaydırma, yakınlaşma ve odak sonlu ve geçerli olmalı.",
        ));
    }
    let scale = crate::projection::Scale::new(v.denominator / input.factor)
        .ok_or_else(|| SheetError::new("bad_scale", "Yakınlaşma ölçek sınırını aşıyor."))?;
    let anchor = input
        .anchor
        .unwrap_or_else(|| v.paper_point([v.center.x, v.center.y]));
    let a = v.model_unclipped(anchor);
    let b = v.model_unclipped([anchor[0] - input.delta[0], anchor[1] - input.delta[1]]);
    let center = GroundPoint {
        x: a[0] + (v.center.x - a[0]) / input.factor + (b[0] - a[0]) / input.factor,
        y: a[1] + (v.center.y - a[1]) / input.factor + (b[1] - a[1]) / input.factor,
    };
    crate::viewport::configure(
        item,
        &crate::viewport::SetViewport {
            id: input.id.clone(),
            center: Some(center),
            ratio: (input.factor != 1.).then_some(crate::viewport::ScaleRatio {
                paper: 1.,
                model: scale.denominator(),
            }),
            ..Default::default()
        },
    )
}
