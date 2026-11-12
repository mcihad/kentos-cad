//! Katman çizimi (docs/adr/0226 §3, §4): how a layer is composed into the
//! drawing: its opacity, its blending mode and the mask its labels and
//! markers cut into other layers. The rules here are the readers', the
//! command's and the server's. A mask's targets are other layers' ids: the
//! command checks them when it writes them; later a target the tree no
//! longer has is skipped, never refused ([`composes_problem`], [`mask_targets`]).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::layer::{LayerNode, LayerNodeType};

/// The largest mask growth, paper mm.
pub const MAX_MASK_SIZE: f64 = 10.0;
/// The most layers a mask cuts into.
pub const MAX_MASK_TARGETS: usize = 64;

/// How a layer's picture meets what is under it (docs/adr/0226 §4: the W3C's
/// separable blending modes, and addition).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Addition,
}

impl BlendMode {
    /// Every mode, in the list's order.
    pub const ALL: [BlendMode; 13] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Addition,
    ];

    /// The name a file writes.
    pub fn key(self) -> &'static str {
        match self {
            BlendMode::Normal => "normal",
            BlendMode::Multiply => "multiply",
            BlendMode::Screen => "screen",
            BlendMode::Overlay => "overlay",
            BlendMode::Darken => "darken",
            BlendMode::Lighten => "lighten",
            BlendMode::ColorDodge => "colorDodge",
            BlendMode::ColorBurn => "colorBurn",
            BlendMode::HardLight => "hardLight",
            BlendMode::SoftLight => "softLight",
            BlendMode::Difference => "difference",
            BlendMode::Exclusion => "exclusion",
            BlendMode::Addition => "addition",
        }
    }

    pub fn from_key(key: &str) -> Option<BlendMode> {
        BlendMode::ALL.into_iter().find(|m| m.key() == key)
    }

    /// The mode's number in the composing shaders (its place in [`BlendMode::ALL`]).
    pub fn index(self) -> u32 {
        BlendMode::ALL.iter().position(|m| *m == self).unwrap_or(0) as u32
    }

    /// Its name in the menus.
    pub fn label(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Çoğalt",
            BlendMode::Screen => "Ekran",
            BlendMode::Overlay => "Bindirme",
            BlendMode::Darken => "Koyulaştır",
            BlendMode::Lighten => "Açıklaştır",
            BlendMode::ColorDodge => "Renk soldurma",
            BlendMode::ColorBurn => "Renk yakma",
            BlendMode::HardLight => "Sert ışık",
            BlendMode::SoftLight => "Yumuşak ışık",
            BlendMode::Difference => "Fark",
            BlendMode::Exclusion => "Dışlama",
            BlendMode::Addition => "Toplama",
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// The mask a layer's labels and markers cut into other layers (docs/adr/0226 §3).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerMask {
    /// Its placed labels' boxes.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub labels: bool,
    /// Its markers' boxes.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub markers: bool,
    /// How far the shapes grow, paper mm (0–10); their corners round as far.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: f64,
    /// The layers it cuts into, by their ids.
    pub targets: Vec<String>,
}

impl LayerMask {
    /// What is wrong with the mask itself, when anything is.
    pub fn problem(&self) -> Option<String> {
        if !self.labels && !self.markers {
            return Some("maskenin kaynağı yok: etiketler ya da işaretler seçilmeli".to_owned());
        }
        if !self.size.is_finite() || !(0.0..=MAX_MASK_SIZE).contains(&self.size) {
            return Some(format!(
                "maskenin boyu 0 ile {MAX_MASK_SIZE} mm arasında olmalı"
            ));
        }
        if self.targets.is_empty() {
            return Some("maskelenecek katman yok".to_owned());
        }
        if self.targets.len() > MAX_MASK_TARGETS {
            return Some(format!("maske en çok {MAX_MASK_TARGETS} katmanı maskeler"));
        }
        let mut seen = HashSet::new();
        for t in &self.targets {
            if t.trim().is_empty() {
                return Some("maskelenen katmanlar arasında boş kimlik var".to_owned());
            }
            if !seen.insert(t.as_str()) {
                return Some(format!("“{t}” katmanı iki kez maskeleniyor"));
            }
        }
        None
    }
}

/// How a layer is composed into the drawing (Katman çizimi, docs/adr/0226 §3,
/// §4); absent, as it always was: opaque, normal, no mask.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerCompose {
    /// 0–1; absent, 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub opacity: Option<f64>,
    /// Absent, normal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub blend: Option<BlendMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mask: Option<LayerMask>,
}

impl LayerCompose {
    /// Whether it says nothing (opaque, normal, no mask): a writer leaves it out.
    pub fn is_plain(&self) -> bool {
        self.opacity.is_none_or(|o| o == 1.0)
            && self.blend.is_none_or(|b| b == BlendMode::Normal)
            && self.mask.is_none()
    }

    /// The same without what says nothing; None when nothing is left.
    pub fn normalized(&self) -> Option<LayerCompose> {
        let out = LayerCompose {
            opacity: self.opacity.filter(|o| *o != 1.0),
            blend: self.blend.filter(|b| *b != BlendMode::Normal),
            mask: self.mask.clone(),
        };
        (!out.is_plain()).then_some(out)
    }

    /// What is wrong with it alone, when anything is: an opacity outside
    /// 0–1, a broken mask, or nothing said at all.
    pub fn problem(&self) -> Option<String> {
        if let Some(o) = self.opacity
            && (!o.is_finite() || !(0.0..=1.0).contains(&o))
        {
            return Some("saydamlık 0 ile 1 arasında olmalı".to_owned());
        }
        if let Some(problem) = self.mask.as_ref().and_then(LayerMask::problem) {
            return Some(problem);
        }
        if self.opacity.is_none() && self.blend.is_none() && self.mask.is_none() {
            return Some("katman çizimi boş".to_owned());
        }
        None
    }
}

/// What is wrong with the layer tree's compositions, when anything is
/// (docs/adr/0226 §3, §4): one on a group, or one broken. A mask's target
/// the tree does not have as another layer (deleted, merged, another
/// editor's change) is not refused: it is skipped when the mask is drawn,
/// so a tree never turns unwritable under it. The command checks its
/// targets when it writes them.
pub fn composes_problem(tree: &[LayerNode]) -> Option<String> {
    for n in tree {
        if let Some(c) = &n.compose {
            if n.kind != LayerNodeType::Layer {
                return Some(format!(
                    "“{}” bir grup; katman çizimi yalnız katmanın olur",
                    n.name
                ));
            }
            if let Some(problem) = c.problem() {
                return Some(format!("“{}”: {problem}", n.name));
            }
        }
        if let Some(problem) = composes_problem(&n.children) {
            return Some(problem);
        }
    }
    None
}

/// The layers a mask cuts into as drawn: its targets the tree has as other
/// layers, in its order; the rest are skipped (docs/adr/0226 §3).
pub fn mask_targets<'a>(tree: &'a [LayerNode], own: &str, mask: &LayerMask) -> Vec<&'a LayerNode> {
    fn find<'a>(tree: &'a [LayerNode], id: &str) -> Option<&'a LayerNode> {
        tree.iter().find_map(|n| {
            if n.id == id {
                Some(n)
            } else {
                find(&n.children, id)
            }
        })
    }
    mask.targets
        .iter()
        .filter(|t| t.as_str() != own)
        .filter_map(|t| find(tree, t))
        .filter(|n| n.kind == LayerNodeType::Layer)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(targets: &[&str]) -> LayerMask {
        LayerMask {
            labels: true,
            markers: false,
            size: 1.0,
            targets: targets.iter().map(|t| (*t).to_owned()).collect(),
        }
    }

    #[test]
    fn a_composition_says_what_is_wrong() {
        let c = |opacity: Option<f64>| LayerCompose {
            opacity,
            blend: None,
            mask: None,
        };
        assert_eq!(c(Some(0.5)).problem(), None);
        assert_eq!(
            c(Some(1.5)).problem().as_deref(),
            Some("saydamlık 0 ile 1 arasında olmalı")
        );
        assert_eq!(c(None).problem().as_deref(), Some("katman çizimi boş"));
        let m = LayerMask {
            labels: false,
            ..mask(&["a"])
        };
        assert!(m.problem().is_some_and(|p| p.contains("kaynağı yok")));
        assert!(
            mask(&["a", "a"])
                .problem()
                .is_some_and(|p| p.contains("iki kez"))
        );
        assert!(
            mask(&[])
                .problem()
                .is_some_and(|p| p.contains("maskelenecek katman yok"))
        );
    }

    #[test]
    fn modes_go_by_their_keys() {
        for m in BlendMode::ALL {
            assert_eq!(BlendMode::from_key(m.key()), Some(m));
            assert_eq!(serde_json::to_value(m).unwrap(), serde_json::json!(m.key()));
        }
        assert_eq!(BlendMode::Multiply.index(), 1);
    }

    fn node(id: &str, kind: LayerNodeType, compose: Option<LayerCompose>) -> LayerNode {
        let mut n: LayerNode = serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": []
        }))
        .unwrap();
        n.kind = kind;
        n.compose = compose;
        n
    }

    #[test]
    fn a_tree_skips_the_targets_it_lacks() {
        let masking = |targets: &[&str]| LayerCompose {
            mask: Some(mask(targets)),
            ..LayerCompose::default()
        };
        let tree = vec![
            node(
                "a",
                LayerNodeType::Layer,
                Some(masking(&["a", "b", "g", "yok"])),
            ),
            node("b", LayerNodeType::Layer, None),
            node("g", LayerNodeType::Group, None),
        ];
        // Itself, a group and a layer the tree lacks are not refused …
        assert_eq!(composes_problem(&tree), None);
        // … but skipped when drawn.
        let m = tree[0]
            .compose
            .as_ref()
            .and_then(|c| c.mask.as_ref())
            .unwrap();
        let drawn: Vec<&str> = mask_targets(&tree, "a", m)
            .iter()
            .map(|n| n.id.as_str())
            .collect();
        assert_eq!(drawn, ["b"]);
        // A group has none; a broken one is named by its layer.
        let tree = vec![node("g", LayerNodeType::Group, Some(masking(&["b"])))];
        assert!(composes_problem(&tree).is_some_and(|p| p.contains("bir grup")));
        let mut inner = node("g", LayerNodeType::Group, None);
        inner.children = vec![node("c", LayerNodeType::Layer, Some(masking(&[])))];
        assert!(
            composes_problem(&[inner])
                .is_some_and(|p| p.contains("“c”") && p.contains("maskelenecek katman yok"))
        );
    }

    #[test]
    fn a_plain_composition_is_left_out() {
        let plain = LayerCompose {
            opacity: Some(1.0),
            blend: Some(BlendMode::Normal),
            mask: None,
        };
        assert!(plain.is_plain());
        assert_eq!(plain.normalized(), None);
    }
}
