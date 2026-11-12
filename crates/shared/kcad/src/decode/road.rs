//! The road project's settings as a payload keeps them (docs/specs/kcad-v2.md): terrains, alignments, profiles,
//! sections, corridors, volumes, hauls and their schemas' checks.

use super::*;

/// `road`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn road(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Option<kentos_contracts::RoadSettings>, KcadError> {
    let (mut v, at): (kentos_contracts::RoadSettings, usize) =
        objects::json_part(r, "yol ayarları")?;
    // Schema 191 (docs/adr/0977): an older file's grades were percent.
    if !has.road_grades {
        v.grades_from_percent();
    }
    if v.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş yol ayarı yazılmaz"));
    }
    // Schema 196 (docs/adr/1560): a surface's edits.
    if !has.surface_edits && v.surfaces.iter().any(|s| !s.edits.is_empty()) {
        return Err(r.fail_at(Code::UnknownField, at, "yüzey düzenlemeleri şema 196 ister"));
    }
    // Schema 214 (docs/adr/3920): a surface's edit that pastes another surface.
    if !has.surface_pastes && crate::encode::road_has_pastes(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "yüzey yapıştırma şema 214 ister"));
    }
    if !has.road_pavement && road_has_pavement(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "tip kesitin üstyapısı ve kübajın kuralı şema 166 ister"));
    }
    if !has.road_ove && road_has_ove(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "aks noktasının ovesi şema 192 ister"));
    }
    if !has.road_speed && road_has_speed(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "aksın proje hızı şema 200 ister"));
    }
    if !has.road_approval_sources && road_has_approval_sources(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "kübaj onayının kaynakları ve algoritması şema 204 ister"));
    }
    if !has.road_cloud_conversions && road_has_cloud_conversions(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "bulut kaydının çevrilen kotları şema 203 ister"));
    }
    if !has.road_cloud_heights && road_has_cloud_heights(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "bulut kaydının düşey sistemi şema 202 ister"));
    }
    if !has.road_places && road_has_places(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "aksın uçlarında gidilen yerler şema 201 ister"));
    }
    if !has.road_pivot && road_has_pivot(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "deverin dönme ekseni şema 167 ister"));
    }
    if !has.volume_curve && road_has_volume_curve(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "kübajın kurp düzeltmesi şema 169 ister"));
    }
    if !has.corridor_sampling && road_has_corridor_sampling(&v) {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "koridorun düşey kiriş yüksekliği ve örnekleme denetimi şema 170 ister",
        ));
    }
    if !has.haul_sorting && road_has_haul_sorting(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "Brüknerin uygunsuz malzemesi ve engelleri şema 171 ister"));
    }
    if !has.profile_crossings && road_has_profile_crossings(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "boykesitin kesişmeleri şema 172 ister"));
    }
    if !has.volume_history && road_has_volume_history(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "kübajın numaralı onayı ve onay geçmişi şema 173 ister"));
    }
    if !has.volume_grid && road_has_volume_grid(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "kübajın ızgara yöntemi ve hücre boyu şema 179 ister"));
    }
    if !has.slope_transitions && road_has_slope_transitions(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "koridor bölgesinin şev geçişi şema 178 ister"));
    }
    if !has.cut_benches && road_has_cut_benches(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "tip kesitin yarma basamakları şema 177 ister"));
    }
    if !has.volume_states && road_has_volume_states(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "kübaj katsayısının yoğunlukları ve kaynağı şema 176 ister"));
    }
    if !has.haul_network && road_has_haul_network(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "Brüknerin taşıma ağı ve konumlu alanları şema 175 ister"));
    }
    if !has.haul_approval && road_has_haul_approval(&v) {
        return Err(r.fail_at(Code::UnknownField, at, "Brüknerin onaylı taşıma planı ve onay geçmişi şema 174 ister"));
    }
    if !has.section_breaks && road_has_section_breaks(&v) {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "kesit takımının kırpması, arazi kırıkları yöntemi ve kurp aralığı şema 168 ister",
        ));
    }
    if let Some((field, words)) = v.problem() {
        return Err(r.fail_at(Code::BadValue, at, &format!("{field}: {words}")));
    }
    Ok(Some(v))
}

/// The settings; `has` says whether the schema has the drawing unit (11 and
/// up), the second coordinate system (12 and up), the project's own
/// systems and datum choices (13 and up) and the survey settings (14).
/// Whether road settings carry schema 166's fields: a template's pavement or a volume's rule (docs/adr/0975 §2).
pub(crate) fn road_has_pavement(road: &kentos_contracts::RoadSettings) -> bool {
    road.templates.iter().any(|t| t.pavement.is_some()) || road.volumes.iter().any(|v| v.rule.is_some())
}

/// Whether road settings carry schema 204's fields: a volume approval's sources or algorithm (docs/adr/1041).
pub(crate) fn road_has_approval_sources(road: &kentos_contracts::RoadSettings) -> bool {
    road.volumes
        .iter()
        .flat_map(|v| v.approved.iter().chain(&v.history))
        .any(|a| a.sources.is_some() || a.algorithm.is_some())
}

/// Whether road settings carry schema 203's fields: a cloud record's heights as the call said them (docs/adr/1032).
pub(crate) fn road_has_cloud_conversions(road: &kentos_contracts::RoadSettings) -> bool {
    road.surfaces.iter().any(|s| s.clouds.iter().any(|c| c.heights.is_some()))
}

/// Whether road settings carry schema 202's fields: a cloud record's vertical system (docs/adr/1029).
pub(crate) fn road_has_cloud_heights(road: &kentos_contracts::RoadSettings) -> bool {
    road.surfaces.iter().any(|s| s.clouds.iter().any(|c| c.vertical.is_some()))
}

/// Whether a road project's settings hold what schema 246 brought (docs/adr/4740): a sheet's contour lines kept as a
/// drawing, a layer as a drawing's source, drawing again whole.
pub(crate) fn road_has_sheet_contour_drawings(r: &kentos_contracts::RoadSettings) -> bool {
    r.rebuild_whole
        || r.drawings.iter().any(|d| {
            d.command == "road.terrains.sheetcontours" || d.sources.iter().any(|s| s.kind == kentos_contracts::RoadSourceKind::Layer)
        })
}

/// Whether road settings carry schema 201's fields: an alignment's places beyond its ends (docs/adr/1028).
pub(crate) fn road_has_places(road: &kentos_contracts::RoadSettings) -> bool {
    road.alignments.iter().any(|a| a.start_place.is_some() || a.end_place.is_some())
}

/// Whether road settings carry schema 200's fields: an alignment's design speed or a PI's own (docs/adr/0985).
pub(crate) fn road_has_speed(road: &kentos_contracts::RoadSettings) -> bool {
    road.alignments.iter().any(|a| a.design_speed.is_some() || a.pis.iter().any(|p| p.speed.is_some()))
}

/// Whether road settings carry schema 192's field: an alignment PI's ove (docs/adr/0979).
pub(crate) fn road_has_ove(road: &kentos_contracts::RoadSettings) -> bool {
    road.alignments.iter().any(|a| a.pis.iter().any(|p| p.ove))
}

/// Whether road settings carry schema 167's field: a superelevation's axis of rotation (docs/adr/0931 §2).
pub(crate) fn road_has_pivot(road: &kentos_contracts::RoadSettings) -> bool {
    road.superelevations.iter().any(|s| s.pivot.is_some())
}

/// Whether road settings carry schema 168's fields: a sample line set's clip, breaks method or a range's curve step
/// (docs/adr/0933 §3).
pub(crate) fn road_has_section_breaks(road: &kentos_contracts::RoadSettings) -> bool {
    road.sections.iter().any(|s| {
        s.clip
            || matches!(
                s.stations,
                kentos_contracts::SectionStations::Breaks { .. }
                    | kentos_contracts::SectionStations::Range { curve: Some(_), .. }
            )
    })
}

/// Whether road settings carry schema 169's field: an earthwork volume's curve correction (docs/adr/0934 §2).
pub(crate) fn road_has_volume_curve(road: &kentos_contracts::RoadSettings) -> bool {
    road.volumes.iter().any(|v| v.curve)
}

/// Whether road settings carry schema 170's fields: a corridor's vertical chord height or sampling check
/// (docs/adr/0939).
pub(crate) fn road_has_corridor_sampling(road: &kentos_contracts::RoadSettings) -> bool {
    road.corridors.iter().any(|c| c.vertical_sag.is_some() || c.refine.is_some())
}

/// Whether road settings carry schema 173's fields: a volume's numbered or ended approval, its history (docs/adr/0950).
pub(crate) fn road_has_volume_history(road: &kentos_contracts::RoadSettings) -> bool {
    road.volumes.iter().any(|v| {
        !v.history.is_empty() || v.approved.as_ref().is_some_and(|a| a.number != 0 || a.ended.is_some())
    })
}

/// Whether road settings carry schema 179's fields: an earthwork volume on a grid or its cell (docs/adr/0974).
pub(crate) fn road_has_volume_grid(road: &kentos_contracts::RoadSettings) -> bool {
    road.volumes.iter().any(|v| v.method == kentos_contracts::VolumeMethod::Grid || v.cell.is_some())
}

/// Whether road settings carry schema 178's field: a corridor region's slope transition (docs/adr/0960).
pub(crate) fn road_has_slope_transitions(road: &kentos_contracts::RoadSettings) -> bool {
    road.corridors.iter().any(|c| c.regions.iter().any(|r| r.transition.is_some()))
}

/// Whether road settings carry schema 177's fields: a template side's cut or fill slope benches (docs/adr/0958).
pub(crate) fn road_has_cut_benches(road: &kentos_contracts::RoadSettings) -> bool {
    let benched = |s: &kentos_contracts::TemplateSideDef| s.bench.is_some() || s.fill_bench.is_some();
    road.templates.iter().any(|t| benched(&t.left) || t.right.as_ref().is_some_and(benched))
}

/// Whether road settings carry schema 176's fields: a volume factor row's densities or source (docs/adr/0957).
pub(crate) fn road_has_volume_states(road: &kentos_contracts::RoadSettings) -> bool {
    road.volumes.iter().any(|v| v.factors.iter().any(|f| f.densities.is_some() || f.source.is_some()))
}

/// Whether road settings carry schema 175's fields: a mass haul diagram's network or a site off the road (docs/adr/0955).
pub(crate) fn road_has_haul_network(road: &kentos_contracts::RoadSettings) -> bool {
    road.hauls.iter().any(|h| h.network.is_some() || h.sites.iter().any(|s| s.at.is_some()))
}

/// Whether road settings carry schema 174's fields: a mass haul diagram's approved plan or its history (docs/adr/0954).
pub(crate) fn road_has_haul_approval(road: &kentos_contracts::RoadSettings) -> bool {
    road.hauls.iter().any(|h| h.approved.is_some() || !h.history.is_empty())
}

/// Whether road settings carry schema 172's field: a profile's crossings (docs/adr/0946).
pub(crate) fn road_has_profile_crossings(road: &kentos_contracts::RoadSettings) -> bool {
    road.profiles.iter().any(|p| !p.crossings.is_empty())
}

/// Whether road settings carry schema 171's fields: a mass haul diagram's unsuitable cut or barriers (docs/adr/0944).
pub(crate) fn road_has_haul_sorting(road: &kentos_contracts::RoadSettings) -> bool {
    road.hauls.iter().any(|h| !h.unsuitable.is_empty() || !h.barriers.is_empty())
}
