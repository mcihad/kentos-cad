//! The enumerations as the contract's serde names them, as the writer writes
//! them (the test below holds the two equal).

use kentos_contracts::{
    AngleUnit, AreaUnit, DimensionStyle, DrawingFont, HatchPatternType, LabelInk, LabelPlacement,
    LineType, PointSymbol, Workspace,
};

pub(crate) fn area_unit(u: AreaUnit) -> &'static str {
    match u {
        AreaUnit::M2 => "m2",
        AreaUnit::Donum => "donum",
        AreaUnit::Ha => "ha",
    }
}

pub(crate) fn angle_unit(u: AngleUnit) -> &'static str {
    match u {
        AngleUnit::Grad => "grad",
        AngleUnit::Deg => "deg",
    }
}

pub(crate) fn workspace(w: Workspace) -> &'static str {
    match w {
        Workspace::Hybrid => "hybrid",
        Workspace::Cad => "cad",
        Workspace::Gis => "gis",
        Workspace::Plan3d => "plan3d",
        Workspace::Disaster => "disaster",
    }
}

pub(crate) fn drawing_font(f: DrawingFont) -> &'static str {
    match f {
        DrawingFont::Barlow => "barlow",
        DrawingFont::Arimo => "arimo",
        DrawingFont::Overpass => "overpass",
        DrawingFont::Quicksand => "quicksand",
        DrawingFont::ArchitectsDaughter => "architects-daughter",
        DrawingFont::CourierPrime => "courier-prime",
        DrawingFont::PlexMono => "plex-mono",
    }
}

pub(crate) fn line_type(t: LineType) -> &'static str {
    match t {
        LineType::Continuous => "continuous",
        LineType::Dashed => "dashed",
        LineType::Dashdot => "dashdot",
        LineType::Dotted => "dotted",
    }
}

pub(crate) fn point_symbol(s: PointSymbol) -> &'static str {
    match s {
        PointSymbol::Ring => "ring",
        PointSymbol::Cross => "cross",
        PointSymbol::Triangle => "triangle",
    }
}

pub(crate) fn label_ink(i: LabelInk) -> &'static str {
    match i {
        LabelInk::Fg => "fg",
        LabelInk::FgDim => "fg-dim",
        LabelInk::Label => "label",
    }
}

pub(crate) fn label_placement(p: LabelPlacement) -> &'static str {
    match p {
        LabelPlacement::Center => "center",
        LabelPlacement::Corner => "corner",
        LabelPlacement::Beside => "beside",
        LabelPlacement::Along => "along",
    }
}

pub(crate) fn dimension_style(s: DimensionStyle) -> &'static str {
    match s {
        DimensionStyle::Aligned => "aligned",
        DimensionStyle::Linear => "linear",
        DimensionStyle::Angular => "angular",
        DimensionStyle::Radius => "radius",
        DimensionStyle::Diameter => "diameter",
    }
}

pub(crate) fn hatch_pattern(t: HatchPatternType) -> &'static str {
    match t {
        HatchPatternType::Solid => "solid",
        HatchPatternType::Lines => "lines",
        HatchPatternType::Cross => "cross",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::LayerNodeType;
    use serde::Serialize;
    use serde_json::Value;

    fn serde_name<T: Serialize>(v: T) -> String {
        match serde_json::to_value(v) {
            Ok(Value::String(s)) => s,
            other => panic!("not a string: {other:?}"),
        }
    }

    #[test]
    fn enumerations_are_written_as_the_contract_names_them() {
        for u in [AreaUnit::M2, AreaUnit::Donum, AreaUnit::Ha] {
            assert_eq!(area_unit(u), serde_name(u));
        }
        for u in [AngleUnit::Grad, AngleUnit::Deg] {
            assert_eq!(angle_unit(u), serde_name(u));
        }
        for w in [
            Workspace::Hybrid,
            Workspace::Cad,
            Workspace::Gis,
            Workspace::Plan3d,
            Workspace::Disaster,
        ] {
            assert_eq!(workspace(w), serde_name(w));
        }
        for f in [
            DrawingFont::Barlow,
            DrawingFont::Arimo,
            DrawingFont::Overpass,
            DrawingFont::Quicksand,
            DrawingFont::ArchitectsDaughter,
            DrawingFont::CourierPrime,
            DrawingFont::PlexMono,
        ] {
            assert_eq!(drawing_font(f), serde_name(f));
        }
        for t in [
            LineType::Continuous,
            LineType::Dashed,
            LineType::Dashdot,
            LineType::Dotted,
        ] {
            assert_eq!(line_type(t), serde_name(t));
        }
        for s in [PointSymbol::Ring, PointSymbol::Cross, PointSymbol::Triangle] {
            assert_eq!(point_symbol(s), serde_name(s));
        }
        for i in [LabelInk::Fg, LabelInk::FgDim, LabelInk::Label] {
            assert_eq!(label_ink(i), serde_name(i));
        }
        for p in [
            LabelPlacement::Center,
            LabelPlacement::Corner,
            LabelPlacement::Beside,
            LabelPlacement::Along,
        ] {
            assert_eq!(label_placement(p), serde_name(p));
        }
        for s in [
            DimensionStyle::Aligned,
            DimensionStyle::Linear,
            DimensionStyle::Angular,
            DimensionStyle::Radius,
            DimensionStyle::Diameter,
        ] {
            assert_eq!(dimension_style(s), serde_name(s));
        }
        for t in [
            HatchPatternType::Solid,
            HatchPatternType::Lines,
            HatchPatternType::Cross,
        ] {
            assert_eq!(hatch_pattern(t), serde_name(t));
        }
        for k in [LayerNodeType::Group, LayerNodeType::Layer] {
            let written = match k {
                LayerNodeType::Group => "group",
                LayerNodeType::Layer => "layer",
            };
            assert_eq!(written, serde_name(k));
        }
    }
}
