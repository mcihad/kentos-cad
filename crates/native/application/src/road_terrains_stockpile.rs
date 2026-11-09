//! `road.terrains.stockpile` v1 (plan/map-cad.md MAP-15, docs/adr/3140): a stockpile's volume inside a closed boundary
//! over a base drawn from the boundary's outer corners (the lowest, highest or average of their heights on the
//! surface, the plane fitted to them, or a height given; Pix4D's volume base surfaces), the computation the core's
//! (`kentos_road::stockpile`). Reads only. The shared cases are `fixtures/commands/v1/road.terrains.stockpile.json`.

use kentos_commands::{CommandResult, CommandWarning, TerrainsStockpile, TerrainsStockpiled};
use kentos_domain::Document;
use kentos_road::stockpile::{self, Base, BaseError};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_STOCKPILE, message, path)
}

/// The base asked for.
fn base_of(input: &TerrainsStockpile) -> Result<Base, Stop> {
    Ok(match input.base.as_str() {
        "lowest" => Base::Lowest,
        "highest" => Base::Highest,
        "average" => Base::Average,
        "fit" => Base::Fit,
        "elevation" => match input.elevation {
            Some(z) if z.is_finite() => Base::Elevation(z),
            _ => return Err(invalid("Kotla taban (elevation) için sonlu bir kot verin.".into(), "elevation")),
        },
        other => {
            return Err(invalid(
                format!("“{other}” bir taban değil: lowest, highest, average, fit ya da elevation olmalı."),
                "base",
            ));
        }
    })
}

/// The volume now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &TerrainsStockpile) -> Result<(TerrainsStockpiled, Vec<CommandWarning>), Stop> {
    let base = base_of(input)?;
    terrains::terrain_project(doc)?;
    let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
    let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
    let made = terrains::make(doc, &def);
    let surface = made.built()?;
    let region = crate::road_terrains_coverage::boundary_of(doc, &input.boundary)?;
    let heights = stockpile::corner_heights(surface, &region);
    let corners = match (&heights, base) {
        (Ok(h), _) => h.clone(),
        (Err(_), Base::Elevation(_)) => Vec::new(),
        (Err(BaseError::Off(k)), _) => {
            let p = region.outer[*k];
            return Err(invalid(
                format!(
                    "Sınırın {}. köşesi ({:.3}, {:.3}) yüzeyin dışında; taban köşelerin yüzeydeki kotlarından çizilir. Sınırı yüzeyin içine çizin ya da kotla taban verin.",
                    k + 1,
                    p[0],
                    p[1]
                ),
                "boundary",
            ));
        }
        (Err(BaseError::Collinear), _) => Vec::new(),
    };
    let plane = stockpile::base_plane(base, &region.outer, &corners).map_err(|_| {
        invalid("Sınırın köşeleri bir doğru üstünde; eğimli taban düzlemi çizilemez.".into(), "base")
    })?;
    let m = stockpile::measure(surface, &region, plane);
    let uncovered = (m.area - m.covered).max(0.0);
    let mut warnings = Vec::new();
    if uncovered > 1e-6 * m.area.max(1.0) {
        warnings.push(CommandWarning {
            code: codes::TERRAIN_NO_DATA.into(),
            message: format!(
                "“{}” yüzeyi sınırın {:.1} m²'sini örtmüyor (%{:.1}); orada hacim ölçülmedi.",
                def.name,
                uncovered,
                100.0 * uncovered / m.area
            ),
            path: Some("boundary".into()),
        });
    }
    let said = match base {
        Base::Lowest => "köşelerin en alçak kotunda yatay düzlem".to_owned(),
        Base::Highest => "köşelerin en yüksek kotunda yatay düzlem".to_owned(),
        Base::Average => "köşelerin ortalama kotunda yatay düzlem".to_owned(),
        Base::Fit => "köşelere en küçük kareler düzlemi (düşey farklar)".to_owned(),
        Base::Elevation(_) => "verilen kotta yatay düzlem".to_owned(),
    };
    let notes = vec![
        format!(
            "Taban: {said}; tabanın üstü kazı (yığın), altı dolgu (Pix4D'nin hacim tabanları). Sınırın {} köşesi.",
            region.outer.len()
        ),
        "Hacim yüzeyin üçgenleriyle sınırın kesişiminde kesin tümlenir: doğrusal fark, parçanın alanı ve momentleriyle.".to_owned(),
    ];
    Ok((
        TerrainsStockpiled {
            surface: id,
            base: input.base.clone(),
            plane,
            corners,
            area: m.area,
            covered: m.covered,
            uncovered,
            cut: m.cut,
            fill: m.fill,
            total: m.cut + m.fill,
            cut_area: m.cut_area,
            fill_area: m.fill_area,
            notes,
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsStockpile) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsStockpile) -> CommandResult<TerrainsStockpiled> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsStockpile) -> CommandResult<TerrainsStockpiled> {
    plan(cx, &input)
}

/// The volume for a window, or why it does not read; with its warnings' words.
pub fn preview(doc: &Document, input: &TerrainsStockpile) -> Result<(TerrainsStockpiled, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
