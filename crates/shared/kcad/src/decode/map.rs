//! The surveying, cadastre and zoning settings as a payload keeps them: map work, GNSS survey, the parcel register,
//! the zoning plan, kept adjustments and monitoring reviews, with their schemas' checks.

use super::*;

/// `parcelRegister`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn parcel_register(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Option<kentos_contracts::ParcelRegister>, KcadError> {
    let (v, at): (kentos_contracts::ParcelRegister, usize) =
        objects::json_part(r, "parsel kayıtları")?;
    if v.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş parsel kayıtları yazılmaz"));
    }
    // Schema 249 (docs/adr/5020): stated lengths, drafts, an approval.
    if !has.parcel_drafts && v.has_drafts() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "parsel kayıtlarında lengths, drafts ya da belgenin approval'ı (şema 249'dan önce yok)",
        ));
    }
    if let Some(problem) = kentos_contracts::parcel_register_problem(&v) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(Some(v))
}

/// `zoningPlan`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn zoning_plan(
    r: &mut Reader<'_>,
) -> Result<Option<kentos_contracts::ZoningPlanData>, KcadError> {
    let (v, at): (kentos_contracts::ZoningPlanData, usize) =
        objects::json_part(r, "imar planı verisi")?;
    if v.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş imar planı verisi yazılmaz"));
    }
    if let Some(problem) = kentos_contracts::zoning_plan_problem(&v) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(Some(v))
}

/// `toolLayers` (schema 255, usage review UR-003): the layer each annotation tool draws on, checked as it is read.
pub(super) fn tool_layers(r: &mut Reader<'_>) -> Result<Vec<kentos_contracts::ToolLayer>, KcadError> {
    let (v, at): (Vec<kentos_contracts::ToolLayer>, usize) = objects::json_part(r, "araç katmanları")?;
    if v.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş araç katmanları yazılmaz"));
    }
    if let Some(problem) = kentos_contracts::tool_layers_problem(&v) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(v)
}

/// `monitoringReviews`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn monitoring_reviews(
    r: &mut Reader<'_>,
) -> Result<Vec<kentos_contracts::MonitoringReview>, KcadError> {
    let (v, at): (Vec<kentos_contracts::MonitoringReview>, usize) =
        objects::json_part(r, "izleme incelemeleri")?;
    if v.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş izleme incelemeleri yazılmaz"));
    }
    if let Some(problem) = kentos_contracts::monitoring_reviews_problem(&v) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(v)
}

/// `surveyAdjustments`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn survey_adjustments(
    r: &mut Reader<'_>,
) -> Result<Vec<kentos_contracts::KeptAdjustment>, KcadError> {
    let (v, at): (Vec<kentos_contracts::KeptAdjustment>, usize) =
        objects::json_part(r, "dengeleme kayıtları")?;
    if v.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş dengeleme kayıtları yazılmaz"));
    }
    if let Some(problem) = kentos_contracts::kept_adjustments_problem(&v) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(v)
}

/// `mapSurvey`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn map_survey(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Option<kentos_contracts::MapSurvey>, KcadError> {
    let (v, at): (kentos_contracts::MapSurvey, usize) = objects::json_part(r, "GNSS arşivi")?;
    if v.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş GNSS arşivi yazılmaz"));
    }
    if let Some((field, words)) = v.problem() {
        return Err(r.fail_at(Code::BadValue, at, &format!("{field}: {words}")));
    }
    // Schema 199 (docs/adr/1740): its baselines, known points, network adjustments and network points.
    if !has.gnss_network && v.has_network() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "GNSS arşivinin bazları, bilinen noktaları ve ağ dengelemeleri şema 199'dan önce yazılmaz",
        ));
    }
    // Schema 207 (docs/adr/2520): a known point's own velocity.
    if !has.known_velocities && v.has_velocities() {
        return Err(r.fail_at(Code::UnknownField, at, "bilinen noktada velocity (şema 207'den önce yok)"));
    }
    // Schema 248 (docs/adr/4920): baseline sessions, an adjustment's sessions, kept limits, a point's velocity.
    if !has.gnss_sessions && v.has_sessions() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "GNSS arşivinde baselineSessions, dengelemenin sessions'ı, tolerance ya da noktanın velocity'si (şema 248'den önce yok)",
        ));
    }
    Ok(Some(v))
}

/// `mapWork`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn map_work(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Option<kentos_contracts::MapWork>, KcadError> {
    let (v, at): (kentos_contracts::MapWork, usize) = objects::json_part(r, "harita işi")?;
    if v.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş harita işi yazılmaz"));
    }
    // Schema 193 (docs/adr/1201 §1): an item's file's relative path.
    if !has.map_work_relative && v.has_relative() {
        return Err(r.fail_at(Code::UnknownField, at, "harita işi öğesinin dosyasında relative (şema 193'ten önce yok)"));
    }
    // Schema 206 (docs/adr/2080): an approval's stamp; a derivation from an item without a file.
    if !has.work_stamps && v.items.iter().any(|i| i.approved.is_some()) {
        return Err(r.fail_at(Code::UnknownField, at, "harita işi öğesinde approved (şema 206'dan önce yok)"));
    }
    if !has.work_stamps && v.has_stamps() {
        return Err(r.fail_at(Code::BadValue, at, "dosyası olmayan öğeden türetme şema 206'dan önce yazılmaz"));
    }
    // Schema 210 (docs/adr/2880): an approval's objects' digests.
    if !has.approved_objects && v.has_approved_objects() {
        return Err(r.fail_at(Code::UnknownField, at, "harita işi öğesinde approvedObjects (şema 210'dan önce yok)"));
    }
    // Schema 211 (docs/adr/3280): an approval's objects' parts' digests.
    if !has.approved_parts && v.has_approved_parts() {
        return Err(r.fail_at(Code::UnknownField, at, "harita işi öğesinde approvedParts (şema 211'den önce yok)"));
    }
    // Schema 213 (docs/adr/3680): an approval's objects' values.
    if !has.approved_values && v.has_approved_values() {
        return Err(r.fail_at(Code::UnknownField, at, "harita işi öğesinde approvedValues (şema 213'ten önce yok)"));
    }
    // Schema 247 (docs/adr/4880): an item's typed layers' profile.
    if !has.work_profiles && v.has_profiles() {
        return Err(r.fail_at(Code::UnknownField, at, "harita işi öğesinde profile (şema 247'den önce yok)"));
    }
    if let Some((field, words)) = v.problem() {
        return Err(r.fail_at(Code::BadValue, at, &format!("{field}: {words}")));
    }
    Ok(Some(v))
}

/// Schema 85's heights (docs/adr/0519 §1): the kind (`ellipsoidal`, `orthometric`; not `unknown`, which a project
/// does not keep), the vertical system and the geoid model, checked by their rules.
pub(super) fn height_reference(r: &mut Reader<'_>) -> Result<HeightReference, KcadError> {
    let at = r.position();
    let (mut kind, mut vertical_srid, mut geoid) = (None, None, None);
    map(r, |r, key| {
        match key {
            "kind" => {
                let k_at = r.position();
                kind = Some(match text(r)?.as_str() {
                    "ellipsoidal" => HeightKind::Ellipsoidal,
                    "orthometric" => HeightKind::Orthometric,
                    other => {
                        return Err(r.fail_at(
                            Code::BadValue,
                            k_at,
                            &format!(
                                "“{other}” bir yükseklik türü değil (ellipsoidal, orthometric)"
                            ),
                        ));
                    }
                });
            }
            "verticalSrid" => vertical_srid = Some(r.uint(u64::from(u32::MAX))? as u32),
            "geoid" => {
                let (mut name, mut sha256) = (None, None);
                map(r, |r, key| {
                    match key {
                        "name" => name = Some(text(r)?),
                        "sha256" => sha256 = Some(text(r)?),
                        _ => return Err(unknown(r)),
                    }
                    Ok(())
                })?;
                geoid = Some(GeoidModel {
                    name: required(r, name, "name")?,
                    sha256,
                });
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let h = HeightReference {
        kind: required(r, kind, "kind")?,
        vertical_srid,
        geoid,
    };
    match h.problem() {
        Some((_, why)) => Err(r.fail_at(Code::BadValue, at, &why)),
        None => Ok(h),
    }
}

/// Schema 14's survey settings (docs/adr/0169 §3), checked whole: at least
/// one, k within [−1, 1], tolerances above zero; schema 15's traverse
/// tolerances too; schema 16's ground height within [−500, 9000] m and its
/// reduction to the grid only with one (docs/adr/0171); schema 28's a priori
/// standard deviations above zero, the parts per million and the centering
/// not below (docs/adr/0203 §1).
pub(super) fn survey_settings(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<SurveySettings, KcadError> {
    let at = r.position();
    let mut s = SurveySettings::default();
    map(r, |r, key| {
        match key {
            "index" => s.index = Some(r.float()?),
            "faceHz" => s.face_hz = Some(r.float()?),
            "faceSlope" => s.face_slope = Some(r.float()?),
            "refraction" => s.refraction = Some(r.float()?),
            "twoWay" if has.traverse_tolerances => s.two_way = Some(r.float()?),
            "traverseAngle" if has.traverse_tolerances => s.traverse_angle = Some(r.float()?),
            "traverseCoord" if has.traverse_tolerances => s.traverse_coord = Some(r.float()?),
            "groundHeight" if has.ground => s.ground_height = Some(r.float()?),
            "reduceToGrid" if has.ground => s.reduce_to_grid = Some(r.bool()?),
            "sigmaDirection" if has.survey_sigmas => s.sigma_direction = Some(r.float()?),
            "sigmaDistance" if has.survey_sigmas => s.sigma_distance = Some(r.float()?),
            "sigmaPpm" if has.survey_sigmas => s.sigma_ppm = Some(r.float()?),
            "sigmaCentering" if has.survey_sigmas => s.sigma_centering = Some(r.float()?),
            "sigmaZenith" if has.survey_sigmas => s.sigma_zenith = Some(r.float()?),
            "sigmaLevelling" if has.survey_sigmas => s.sigma_levelling = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match s.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(s),
    }
}
