//! The layer tree read back: layers with their fields, choices, snapping and styles, and a point style.

use super::*;

// ── The layer tree (§6.5) ───────────────────────────────────────────────

pub(super) fn layer(r: &mut Reader<'_>, has: Features) -> Result<LayerNode, KcadError> {
    let (mut id, mut name, mut kind, mut style) = (None, None, None, None);
    let (mut locked, mut visible, mut children, mut expanded) = (None, None, None, None);
    let (mut snap, mut fields, mut service, mut feed) = (None, None, None, None);
    let (mut time, mut scenario, mut replaces) = (None, None, None);
    let mut filter = None;
    let (mut actions, mut card, mut compose) = (None, None, None);
    let (mut plot, mut frozen) = (None, None);
    let mut table_styles = Vec::new();
    let mut subtypes: Option<(usize, kentos_contracts::LayerSubtypes)> = None;
    let mut source_crs: Option<kentos_contracts::LayerSourceCrs> = None;
    let mut dataset: Option<kentos_contracts::DatasetInfo> = None;
    let mut derivation: Option<kentos_contracts::LayerDerivation> = None;
    let mut geometry: Option<kentos_contracts::LayerGeometry> = None;
    let mut contingents: Option<(usize, Vec<kentos_contracts::FieldGroup>)> = None;
    let mut rules: Option<(usize, Vec<kentos_contracts::LayerRule>)> = None;
    let mut indexes: Option<(usize, Vec<kentos_contracts::LayerIndex>)> = None;
    map(r, |r, key| {
        match key {
            "id" => id = Some(text(r)?),
            "name" => name = Some(text(r)?),
            "frozen" if has.frozen => {
                if !r.bool()? {
                    return Err(r.fail(
                        Code::BadValue,
                        "frozen yalnız true yazılır; çözülmüş katmanda alan yoktur",
                    ));
                }
                frozen = Some(true);
            }
            "card" if has.attachments => card = Some(info::info_card(r)?),
            "actions" if has.attachments => actions = Some(info::layer_actions(r)?),
            "compose" if has.compose => compose = Some(compose::layer_compose(r)?),
            // Schema 72 (docs/adr/0413 §3): the table's rules, read whole and checked.
            "tableStyles" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::TableStyle> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("koşullu biçim okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "koşullu biçim yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; kural yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some((_, problem)) = kentos_contracts::table_styles_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                table_styles = list;
            }
            // Schema 72 (docs/adr/0422 §4): the subtypes, read whole; checked with the fields below.
            "subtypes" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let read: kentos_contracts::LayerSubtypes = serde_json::from_value(value.clone())
                    .map_err(|e| {
                    let words = e.to_string();
                    let code = if words.contains("unknown field") {
                        Code::UnknownField
                    } else {
                        Code::BadValue
                    };
                    r.fail_at(code, at, &format!("alt türler okunamadı: {words}"))
                })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "alt türler yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                subtypes = Some((at, read));
            }
            // Schema 72 (docs/adr/0423 §4): the value combinations, read whole; checked below.
            "contingents" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let read: Vec<kentos_contracts::FieldGroup> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("değer bileşimleri okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "değer bileşimleri yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if read.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; grup yoksa alan hiç yazılmaz",
                    ));
                }
                contingents = Some((at, read));
            }
            // Schema 72 (docs/adr/0438 §1): the record rules, read whole; checked below.
            "rules" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let read: Vec<kentos_contracts::LayerRule> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("kayıt kuralları okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kayıt kuralları yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if read.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; kural yoksa alan hiç yazılmaz",
                    ));
                }
                rules = Some((at, read));
            }
            // Schema 136 (docs/adr/0758): the indexes, read whole; checked below.
            "indexes" if has.layer_indexes => {
                let at = r.position();
                let value = r.opaque()?;
                let read: Vec<kentos_contracts::LayerIndex> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("dizinler okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "dizinler yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if read.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; dizin yoksa alan hiç yazılmaz",
                    ));
                }
                indexes = Some((at, read));
            }
            // Schema 102 (docs/adr/0622 §5): what the layer holds.
            "geometry" if has.layer_geometry => {
                let at = r.position();
                geometry = Some(match text(r)?.as_str() {
                    "point" => kentos_contracts::LayerGeometry::Point,
                    "line" => kentos_contracts::LayerGeometry::Line,
                    "area" => kentos_contracts::LayerGeometry::Area,
                    "annotation" => kentos_contracts::LayerGeometry::Annotation,
                    other => {
                        return Err(r.fail_at(
                            Code::BadValue,
                            at,
                            &format!(
                                "katmanın türü “{other}” değil: point, line, area ya da annotation"
                            ),
                        ));
                    }
                });
            }
            // Schema 106 (docs/adr/0633 §3): the layer as a dataset and what it was derived from, read whole.
            "dataset" if has.datasets => {
                let at = r.position();
                let value = r.opaque()?;
                let read: kentos_contracts::DatasetInfo = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("veri kümesi bilgisi okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "veri kümesi bilgisi yazıcının biçiminde değil (boş bir alan yazılmış)",
                    ));
                }
                if let Some(problem) = kentos_contracts::dataset_problem(&read) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                dataset = Some(read);
            }
            "derivation" if has.datasets => {
                let at = r.position();
                let value = r.opaque()?;
                let read: kentos_contracts::LayerDerivation = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("türetim kaydı okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "türetim kaydı yazıcının biçiminde değil (boş bir alan yazılmış)",
                    ));
                }
                if let Some(problem) = kentos_contracts::derivation_problem(&read) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                // GIS data 1 (docs/adr/8057 §15): the extension that made it and where its code ran.
                if has.gis < crate::GIS_WITH_EXTENSION_DERIVATIONS
                    && (read.extension.is_some() || read.environment.is_some())
                {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "türetim kaydında extension ve environment CBS verisi sürüm 1'den önce yok",
                    ));
                }
                derivation = Some(read);
            }
            // Schema 98 (docs/adr/0583 §1): the system its objects came from, read whole and checked.
            "sourceCrs" if has.source_crs => {
                let at = r.position();
                let value = r.opaque()?;
                let read: kentos_contracts::LayerSourceCrs = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("kaynak sistemi okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kaynak sistemi yazıcının biçiminde değil (boş bir alan yazılmış)",
                    ));
                }
                if let Some(problem) = kentos_contracts::layer_crs_problem(&read) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                source_crs = Some(read);
            }
            "plot" if has.look => {
                if r.bool()? {
                    return Err(r.fail(
                        Code::BadValue,
                        "plot yalnız false yazılır; yazdırılan katmanda alan yoktur",
                    ));
                }
                plot = Some(false);
            }
            "filter" if has.filters => filter = Some(filter::layer_filter(r)?),
            "time" if has.temporal => time = Some(temporal::layer_time(r, has.recorded_time)?),
            "scenario" if has.temporal => scenario = Some(temporal::scenario(r)?),
            "replaces" if has.temporal => replaces = Some(text(r)?),
            "snap" if has.layer_snap => snap = Some(layer_snap(r)?),
            "fields" if has.layer_fields => {
                fields = Some(layer_fields(
                    r,
                    has.forms,
                    has.field_kinds,
                    has.withheld_fields,
                    has.field_columns,
                    has.table_constraints,
                )?)
            }
            "service" if has.services => service = Some(services::service_layer(r)?),
            "feed" if has.services => feed = Some(services::feature_feed(r)?),
            "type" => {
                kind = Some(named(
                    r,
                    &[
                        ("group", LayerNodeType::Group),
                        ("layer", LayerNodeType::Layer),
                    ],
                )?)
            }
            "style" => style = Some(layer_style(r, has.labels, has.patterns)?),
            "locked" => locked = Some(r.bool()?),
            "visible" => visible = Some(r.bool()?),
            "children" => children = Some(list(r, |r, _| layer(r, has))?),
            "expanded" => expanded = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if !table_styles.is_empty() && kind == Some(LayerNodeType::Group) {
        return Err(r.fail(
            Code::BadValue,
            "grubun koşullu biçimi olmaz; yalnız katmanın olur",
        ));
    }
    if let Some((at, read)) = &subtypes {
        if kind == Some(LayerNodeType::Group) {
            return Err(r.fail_at(
                Code::BadValue,
                *at,
                "grubun alt türleri olmaz; yalnız katmanın olur",
            ));
        }
        let none = Vec::new();
        let fields_now: &Vec<_> = fields.as_ref().unwrap_or(&none);
        if let Some((_, problem)) = kentos_contracts::subtypes_problem(fields_now, read) {
            return Err(r.fail_at(Code::BadValue, *at, &problem));
        }
    }
    if let Some((at, read)) = &contingents {
        if kind == Some(LayerNodeType::Group) {
            return Err(r.fail_at(
                Code::BadValue,
                *at,
                "grubun değer bileşimleri olmaz; yalnız katmanın olur",
            ));
        }
        let none = Vec::new();
        let fields_now: &Vec<_> = fields.as_ref().unwrap_or(&none);
        if let Some((_, problem)) = kentos_contracts::contingents_problem(
            fields_now,
            subtypes.as_ref().map(|(_, s)| s),
            read,
        ) {
            return Err(r.fail_at(Code::BadValue, *at, &problem));
        }
    }
    if let Some((at, read)) = &rules {
        if kind == Some(LayerNodeType::Group) {
            return Err(r.fail_at(
                Code::BadValue,
                *at,
                "grubun kayıt kuralları olmaz; yalnız katmanın olur",
            ));
        }
        let none = Vec::new();
        let fields_now: &Vec<_> = fields.as_ref().unwrap_or(&none);
        if let Some((_, problem)) = kentos_contracts::layer_rules_problem(fields_now, read) {
            return Err(r.fail_at(Code::BadValue, *at, &problem));
        }
    }
    if let Some((at, read)) = &indexes {
        if kind == Some(LayerNodeType::Group) {
            return Err(r.fail_at(
                Code::BadValue,
                *at,
                "grubun dizinleri olmaz; yalnız katmanın olur",
            ));
        }
        let none = Vec::new();
        let fields_now: &Vec<_> = fields.as_ref().unwrap_or(&none);
        if let Some((_, problem)) = kentos_contracts::layer_indexes_problem(fields_now, read) {
            return Err(r.fail_at(Code::BadValue, *at, &problem));
        }
    }
    if source_crs.is_some() && kind == Some(LayerNodeType::Group) {
        return Err(r.fail(
            Code::BadValue,
            "grubun kaynak sistemi olmaz; yalnız katmanın olur",
        ));
    }
    if geometry.is_some() && kind == Some(LayerNodeType::Group) {
        return Err(r.fail(
            Code::BadValue,
            "grubun türü olmaz; nokta, çizgi, alan ya da açıklama yalnız katmanın türüdür",
        ));
    }
    if (dataset.is_some() || derivation.is_some()) && kind == Some(LayerNodeType::Group) {
        return Err(r.fail(
            Code::BadValue,
            "grubun veri kümesi bilgisi ve türetim kaydı olmaz; yalnız katmanın olur",
        ));
    }
    Ok(LayerNode {
        dataset,
        derivation,
        source_crs,
        geometry,
        rules: rules.map(|(_, c)| c).unwrap_or_default(),
        indexes: indexes.map(|(_, c)| c).unwrap_or_default(),
        contingents: contingents.map(|(_, c)| c).unwrap_or_default(),
        subtypes: subtypes.map(|(_, s)| s),
        table_styles,
        plot,
        frozen,
        id: required(r, id, "id")?,
        name: required(r, name, "name")?,
        kind: required(r, kind, "type")?,
        visible: required(r, visible, "visible")?,
        locked: required(r, locked, "locked")?,
        expanded: required(r, expanded, "expanded")?,
        style: required(r, style, "style")?,
        children: required(r, children, "children")?,
        snap: match (snap, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun keneti olmaz; kenet yalnız katmanındır",
                ));
            }
            (snap, _) => snap,
        },
        fields: match (fields, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun alanları olmaz; alanlar yalnız katmanındır",
                ));
            }
            (fields, _) => fields.unwrap_or_default(),
        },
        service: match (service, kind) {
            (Some(_), _) if feed.is_some() => {
                return Err(r.fail(
                    Code::BadValue,
                    "katman hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz",
                ));
            }
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun servisi olmaz; servis yalnız katmanındır",
                ));
            }
            (service, _) => service,
        },
        feed: match (feed, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun veri kaynağı olmaz; kaynak yalnız katmanındır",
                ));
            }
            (feed, _) => feed,
        },
        time: match (time, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun zamanı olmaz; zaman yalnız katmanındır",
                ));
            }
            (time, _) => time,
        },
        scenario: match (scenario, kind) {
            (Some(_), Some(LayerNodeType::Layer)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "katman senaryo olmaz; yalnız grup senaryodur",
                ));
            }
            (scenario, _) => scenario,
        },
        replaces: match (replaces, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grup bir katmanın yerine geçmez; yalnız katman geçer",
                ));
            }
            (replaces, _) => replaces,
        },
        filter: match (filter, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun süzgeci olmaz; süzgeç yalnız katmanındır",
                ));
            }
            (filter, _) => filter,
        },
        actions: match (actions, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun eylemi olmaz; eylemler yalnız katmanındır",
                ));
            }
            (actions, _) => actions.unwrap_or_default(),
        },
        card: match (card, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun bilgi kartı olmaz; kart yalnız katmanındır",
                ));
            }
            (card, _) => card,
        },
        compose: match (compose, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun katman çizimi olmaz; katman çizimi yalnız katmanındır",
                ));
            }
            (compose, _) => compose,
        },
    })
}

/// Schema 26's fields of a layer (docs/adr/0199 §1), checked whole
/// (`layer_fields_problem`): a list that is not empty, each field by its
/// kind's rules, no name twice.
pub(super) fn layer_fields(
    r: &mut Reader<'_>,
    ids: bool,
    wide: bool,
    withheld: bool,
    columns: bool,
    constraints: bool,
) -> Result<Vec<LayerField>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        layer_field(r, ids, wide, withheld, columns, constraints)
    })?;
    match layer_fields_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}

/// A layer's field; `ids`: schema 72's stable `id` is read (docs/adr/0401 §2).
pub(super) fn layer_field(
    r: &mut Reader<'_>,
    ids: bool,
    wide: bool,
    withholds: bool,
    columns: bool,
    constraints: bool,
) -> Result<LayerField, KcadError> {
    let mut id = None;
    let mut withheld = None;
    let mut column = None;
    let (mut name, mut kind, mut alias, mut length, mut scale) = (None, None, None, None, None);
    let (mut min, mut max, mut values, mut default, mut must) = (None, None, None, None, None);
    let mut many = None;
    let (mut split, mut merge) = (None, None);
    let mut domain = None;
    let (mut unique, mut min_out, mut max_out, mut unit) = (None, None, None, None);
    let count = |r: &mut Reader<'_>| r.uint(u64::from(u32::MAX)).map(|n| n as u32);
    map(r, |r, key| {
        match key {
            "id" if ids => id = Some(text(r)?),
            "max" => max = Some(text(r)?),
            "min" => min = Some(text(r)?),
            // A date and time and a time of day are schema 72's (docs/adr/0412 §4), the kinds of docs/adr/0629 §2
            // schema 104's.
            "kind" => {
                let kinds: Vec<(&str, LayerFieldKind)> = LayerFieldKind::ALL
                    .into_iter()
                    .filter(|k| {
                        (ids || !matches!(k, LayerFieldKind::DateTime | LayerFieldKind::Time))
                            && (wide || !k.is_wide())
                    })
                    .map(|k| (k.name(), k))
                    .collect();
                kind = Some(named(r, &kinds)?)
            }
            "name" => name = Some(text(r)?),
            "alias" => alias = Some(text(r)?),
            "scale" => scale = Some(count(r)?),
            "length" => length = Some(count(r)?),
            "values" => values = Some(list(r, |r, _| field_choice(r, ids))?),
            "default" => default = Some(text(r)?),
            "required" => must = Some(r.bool()?),
            "multiple" if ids => many = Some(r.bool()?),
            // Schema 72 (docs/adr/0430): the value list it links.
            "domain" if ids => domain = Some(text(r)?),
            // Schema 72 (docs/adr/0438 §3, §4): unique, the range's left-out ends, the unit.
            "unique" if ids => unique = Some(r.bool()?),
            "minExclusive" if ids => min_out = Some(r.bool()?),
            "maxExclusive" if ids => max_out = Some(r.bool()?),
            "unit" if ids => unit = Some(text(r)?),
            // Schema 107 (docs/adr/0638 §5): its values do not leave the drawing.
            "withheld" if withholds => withheld = Some(r.bool()?),
            // Schema 133 (docs/adr/0753): the bound table's column, what the table makes and what it refers to.
            "column" if columns => column = Some(field_column(r, constraints)?),
            // Schema 72 (docs/adr/0419): the split and merge policies.
            "split" if ids => {
                let all: Vec<(&str, kentos_contracts::SplitPolicy)> =
                    kentos_contracts::SplitPolicy::ALL
                        .into_iter()
                        .map(|p| (crate::encode::split_name(p), p))
                        .collect();
                split = Some(named(r, &all)?)
            }
            "merge" if ids => {
                let all: Vec<(&str, kentos_contracts::MergePolicy)> =
                    kentos_contracts::MergePolicy::ALL
                        .into_iter()
                        .map(|p| (crate::encode::merge_name(p), p))
                        .collect();
                merge = Some(named(r, &all)?)
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if must == Some(false) || many == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "required ve multiple yalnız true yazılır; değilse yazılmaz",
        ));
    }
    if unique == Some(false) || min_out == Some(false) || max_out == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "unique, minExclusive ve maxExclusive yalnız true yazılır; değilse yazılmaz",
        ));
    }
    if withheld == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "withheld yalnız true yazılır; değilse yazılmaz",
        ));
    }
    Ok(LayerField {
        withheld: withheld == Some(true),
        id,
        name: required(r, name, "name")?,
        alias,
        kind: required(r, kind, "kind")?,
        length,
        scale,
        min,
        max,
        values,
        required: must == Some(true),
        default,
        multiple: many == Some(true),
        split,
        merge,
        domain,
        unique: unique == Some(true),
        min_exclusive: min_out == Some(true),
        max_exclusive: max_out == Some(true),
        unit,
        column,
    })
}

/// A field's bound column (schema 133, docs/adr/0753): `made` and `refers`, from schema 134 `check` (docs/adr/0755 §4),
/// at least one; their texts are checked with the field's (`field_problem`).
pub(super) fn field_column(
    r: &mut Reader<'_>,
    constraints: bool,
) -> Result<FieldColumn, KcadError> {
    let (mut made, mut refers, mut check) = (None, None, None);
    map(r, |r, key| {
        match key {
            "made" => made = Some(text(r)?),
            "check" if constraints => check = Some(text(r)?),
            "refers" => {
                let (mut table, mut column, mut label) = (None, None, None);
                map(r, |r, key| {
                    match key {
                        "table" => table = Some(text(r)?),
                        "column" => column = Some(text(r)?),
                        "label" => label = Some(text(r)?),
                        _ => return Err(unknown(r)),
                    }
                    Ok(())
                })?;
                refers = Some(ColumnRefers {
                    table: required(r, table, "table")?,
                    column: required(r, column, "column")?,
                    label,
                });
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if made.is_none() && refers.is_none() && check.is_none() {
        return Err(r.fail(
            Code::BadValue,
            "column boş olamaz: made, refers ya da check yazılır; hiçbiri yoksa column yazılmaz",
        ));
    }
    Ok(FieldColumn {
        made,
        refers,
        check,
    })
}

/// A value of a field's list; `later`: schema 72's `retired` is read (docs/adr/0418).
pub(super) fn field_choice(r: &mut Reader<'_>, later: bool) -> Result<FieldChoice, KcadError> {
    let (mut code, mut label, mut retired) = (None, None, None);
    let (mut description, mut from, mut until) = (None, None, None);
    map(r, |r, key| {
        match key {
            "code" => code = Some(text(r)?),
            "label" => label = Some(text(r)?),
            "retired" if later => retired = Some(r.bool()?),
            // Schema 72 (docs/adr/0438 §5): what it means and its days.
            "description" if later => description = Some(text(r)?),
            "from" if later => from = Some(text(r)?),
            "until" if later => until = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if retired == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "retired yalnız true yazılır; değilse yazılmaz",
        ));
    }
    Ok(FieldChoice {
        code: required(r, code, "code")?,
        label: required(r, label, "label")?,
        retired: retired == Some(true),
        description,
        from,
        until,
    })
}

/// A layer's own snapping (docs/adr/0163 §4): `off` (true only) or `kinds`.
pub(super) fn layer_snap(r: &mut Reader<'_>) -> Result<LayerSnap, KcadError> {
    let mut snap = LayerSnap::default();
    let mut off = None;
    map(r, |r, key| {
        match key {
            "off" => off = Some(r.bool()?),
            "kinds" => snap.kinds = Some(list(r, |r, _| text(r))?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if off == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "off yalnız true yazılır; kenet kapalı değilse alan yazılmaz",
        ));
    }
    snap.off = off == Some(true);
    match snap.problem() {
        Some(problem) => Err(r.fail(Code::BadValue, &problem)),
        None => Ok(snap),
    }
}

/// A line type by its name (a layer style's, a dimension's lines'; §6.4).
pub(crate) fn line_type_named(r: &mut Reader<'_>) -> Result<LineType, KcadError> {
    named(
        r,
        &[
            ("continuous", LineType::Continuous),
            ("dashed", LineType::Dashed),
            ("dashdot", LineType::Dashdot),
            ("dotted", LineType::Dotted),
        ],
    )
}

pub(super) fn layer_style(
    r: &mut Reader<'_>,
    engine: bool,
    patterns: bool,
) -> Result<LayerStyle, KcadError> {
    let (mut fill, mut color, mut label, mut point_) = (None, None, None, None);
    let mut pattern = None;
    let mut labels = None;
    let (mut line_type, mut renderer, mut line_weight, mut pick_interior) =
        (None, None, None, None);
    map(r, |r, key| {
        match key {
            "fill" => fill = Some(text(r)?),
            "color" => color = Some(text(r)?),
            "label" => label = Some(labels::label_style(r, engine)?),
            "labels" if engine => labels = Some(labels::layer_labels(r)?),
            "point" => point_ = Some(point_style(r)?),
            "lineType" => line_type = Some(line_type_named(r)?),
            "linePattern" if patterns => pattern = Some(text(r)?),
            "renderer" => {
                if r.next_is_null() {
                    return Err(r.fail(
                        Code::BadValue,
                        "null yazılmaz; çizici yoksa alan hiç yazılmaz",
                    ));
                }
                renderer = Some(r.opaque()?);
            }
            "lineWeight" => line_weight = Some(r.float()?),
            "pickInterior" => pick_interior = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LayerStyle {
        line_pattern: pattern,
        color: required(r, color, "color")?,
        line_type: required(r, line_type, "lineType")?,
        line_weight: required(r, line_weight, "lineWeight")?,
        fill,
        point: point_,
        label,
        pick_interior,
        renderer,
        labels,
    })
}

pub(super) fn point_style(r: &mut Reader<'_>) -> Result<PointStyle, KcadError> {
    let (mut size, mut symbol) = (None, None);
    map(r, |r, key| {
        match key {
            "size" => size = Some(r.float()?),
            "symbol" => {
                symbol = Some(named(
                    r,
                    &[
                        ("ring", PointSymbol::Ring),
                        ("cross", PointSymbol::Cross),
                        ("triangle", PointSymbol::Triangle),
                    ],
                )?)
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(PointStyle {
        symbol: required(r, symbol, "symbol")?,
        size: required(r, size, "size")?,
    })
}
