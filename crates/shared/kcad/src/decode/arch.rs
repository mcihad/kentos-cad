//! The architecture settings as a payload keeps them: storeys, details, modules, spaces' lists and the wall
//! styles, with their schemas' checks.

use super::*;

/// `wallStyles`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn wall_styles(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Vec<kentos_contracts::WallStyleDef>, KcadError> {
    let (l, at): (Vec<kentos_contracts::WallStyleDef>, usize) =
        objects::json_part(r, "duvar stilleri")?;
    if l.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş duvar stili listesi yazılmaz"));
    }
    if let Some(problem) = kentos_contracts::wall_styles_problem(&l) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    if !has.wall_joins
        && l.iter()
            .any(|s| s.components.iter().any(|c| c.priority.is_some()))
    {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "bileşenin önceliği şema 76'dan beri yazılır",
        ));
    }
    if !has.building_parts && l.iter().any(|s| s.components.iter().any(|c| c.wrap)) {
        return Err(r.fail_at(Code::UnknownField, at, "uçta dönen bileşen şema 138 ister"));
    }
    if !has.materials && l.iter().any(|s| s.components.iter().any(|c| c.material_id.is_some())) {
        return Err(r.fail_at(Code::UnknownField, at, "malzemeye bağlı bileşen şema 148 ister"));
    }
    Ok(l)
}

/// `arch`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn arch(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Option<kentos_contracts::ArchSettings>, KcadError> {
    let (a, at): (kentos_contracts::ArchSettings, usize) = objects::json_part(r, "mimari ayarlar")?;
    if a.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş mimari ayar yazılmaz"));
    }
    if !has.space_choices && a.has_space_choices() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin mahal listeleri ve kat önekleri şema 112'den beri yazılır",
        ));
    }
    if !has.plan_cut && a.has_plan_cut() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin kesme yüksekliği şema 217'den beri yazılır",
        ));
    }
    if !has.line_groups && a.has_line_groups() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin çizgi grupları şema 218'den beri yazılır",
        ));
    }
    if let Some((field, words)) = a.problem() {
        return Err(r.fail_at(Code::BadValue, at, &format!("{field}: {words}")));
    }
    if !has.level_state && a.has_level_state() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin etkin katı ve kat görünümü şema 120'den beri yazılır",
        ));
    }
    if !has.module_scope && kentos_contracts::modules_have_scope(&a.modules) {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "oda modülünün kolonu, merdiveni, rampası, döşemesi, mahal ayırıcısı ve mutfak dizisi şema 245'ten beri yazılır",
        ));
    }
    if !has.module_storeys && kentos_contracts::modules_have_storeys(&a.modules) {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "oda modülü nesnesinin katı şema 152'den beri yazılır",
        ));
    }
    if !has.instance_storeys && kentos_contracts::modules_have_instance_storeys(&a.modules) {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "tek katlı modül örneği şema 155'ten beri yazılır",
        ));
    }
    if !has.option_combos && !a.option_combos.is_empty() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "tasarım seçeneklerinin bileşimleri şema 154'ten beri yazılır",
        ));
    }
    if !has.options && a.has_options() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin tasarım seçenekleri şema 149'dan beri yazılır",
        ));
    }
    if !has.module_units && kentos_contracts::modules_have_units(&a.modules) {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "oda modülü örneğinin bölümü şema 145'ten beri yazılır",
        ));
    }
    if !has.modules && a.has_modules() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin oda modülleri şema 142'den beri yazılır",
        ));
    }
    if !has.materials && a.has_materials() {
        return Err(r.fail_at(Code::UnknownField, at, "projenin malzemeleri şema 148 ister"));
    }
    if !has.sprinklers && a.has_sprinklers() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "yağmurlama sistemi (sprinklers) şema 226 ister",
        ));
    }
    if !has.local_rules && a.has_local_rules() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "yerel imar yönetmeliği (localRules) şema 225 ister",
        ));
    }
    if !has.site_plan && a.has_site() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "vaziyet planı (site: parsel, kenarları, ±0,00 kotu) şema 224 ister",
        ));
    }
    if !has.land_shares && a.program.has_land_shares() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "bina programının eklentileri, niteliği, değer katsayısı ve arsa payı paydası şema 223 ister",
        ));
    }
    if !has.zoning && (a.has_zoning() || a.program.has_emsal()) {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "parselin imar durumu (zoning) ve kullanımın emsal sınıfı şema 222 ister",
        ));
    }
    if !has.opening_specs && a.has_opening_specs() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "kapı ve pencere tipinin kasa ve kanadı (material) ve U değeri (uValue) şema 229 ister",
        ));
    }
    if !has.opening_codes && a.has_opening_codes() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "kapı ve pencere tipinin doğrama kodu (code) şema 220 ister",
        ));
    }
    if !has.opening_types && a.has_opening_types() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin kapı ve pencere tipleri şema 139'den beri yazılır",
        ));
    }
    if !has.slide_more && a.has_xox() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "üç kanatlı sürmenin iki yana açılan tipi (XOX) şema 153 ister",
        ));
    }
    if !has.multi_slide && a.has_multi_slide() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "üç ve dört kanatlı sürme kapı ve pencere tipleri şema 150 ister",
        ));
    }
    if !has.special_doors && a.has_special_doors() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "döner, cebe giren ve katlanır kapı tipleri şema 144 ister",
        ));
    }
    if !has.building_program && !a.program.is_empty() {
        return Err(r.fail_at(Code::UnknownField, at, "bina programı şema 140 ister"));
    }
    if !has.program_shares && a.program.has_shares() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "bina programının payları ve binaların katları şema 147 ister",
        ));
    }
    if !has.audit_exceptions && a.has_audit_exceptions() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "mimari denetimin istisnaları şema 130'dan beri yazılır",
        ));
    }
    if !has.phases && a.phase_view.is_some() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin tadilat görünümü şema 82'den beri yazılır",
        ));
    }
    if !has.renovation_view
        && a.phase_view == Some(kentos_contracts::PhaseView::Renovation)
    {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "projenin Tadilat projesi görünümü (renovation) şema 231 ister",
        ));
    }
    if !has.spaces && a.space_method.is_some() {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "projenin ölçme yöntemi şema 78'den beri yazılır",
        ));
    }
    Ok(Some(a))
}
