//! `map.survey.edit` v1 (plan/map-cad.md MAP-03, docs/adr/1340 §4): a map project's GNSS archive changed — campaigns
//! and sessions added (a session from a RINEX, NMEA or GPX file), receiver solutions solved and others recorded,
//! solutions excluded and taken back, a mark's point accepted (its object drawn) and made again (its older result
//! kept, its object moved), uses recorded, records removed; one undo step each, whole or not at all. The shared cases
//! are `fixtures/commands/v1/map.survey.edit.json`.

use std::path::Path;

use kentos_commands::{
    AcceptMethod, CommandResult, CommandWarning, Entity, EntityBase, EntityId, GnssPoint, MapSurvey, MapSurveyEdit,
    MapSurveyEdited, MapSurveyNetwork, MapSurveyOperation as Op, PointEntity, PointRevision, SolutionKind,
    SurveyAdjustment, SurveyAntenna, SurveyBaseline, SurveyCampaign, SurveyFile, SurveyFileFormat, SurveyGrid,
    SurveyKnown, SurveyPoint, SurveyReceiver, SurveyResult, SurveySession, SurveySolution, SurveyUse, TimeSystem, Vec2,
    baseline_numbers_problem, known_numbers_problem, next_survey_id, survey_text_ok, survey_time,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::map_survey::{compute, fail, m3, map_project, point_why, present, stale_count};

const SHORT: usize = kentos_commands::MOST_SURVEY_TEXT;

fn text(v: Option<&str>, most: usize, path: &str, words: &str) -> Result<(), Stop> {
    match v {
        Some(t) if !survey_text_ok(t, most) => Err(fail(
            codes::INVALID_SURVEY_TEXT,
            format!("{words} boşlukla başlamayan ve bitmeyen 1–{most} karakter olmalı."),
            path,
        )),
        _ => Ok(()),
    }
}

fn value(path: &str, message: String) -> Stop {
    fail(codes::INVALID_SURVEY_VALUE, message, path)
}

fn nothing() -> Stop {
    fail(codes::NOTHING_TO_SET, "Değişecek bir şey yok.".into(), "operation")
}

fn full(what: &str) -> Stop {
    fail(codes::SURVEY_FULL, format!("GNSS arşivinde en çok {what} olur."), "operation")
}

fn unknown(what: &str, id: &str, path: &str) -> Stop {
    fail(codes::UNKNOWN_SURVEY_RECORD, format!("GNSS arşivinde “{id}” kimlikli {what} yok."), path)
}

fn epoch_ok(e: Option<f64>, path: &str) -> Result<(), Stop> {
    match e.and_then(kentos_commands::epoch_problem) {
        Some(why) => Err(value(path, format!("Epok: {why}."))),
        None => Ok(()),
    }
}

fn grid_ok(g: Option<&SurveyGrid>, path: &str) -> Result<(), Stop> {
    match g {
        Some(g) if !survey_text_ok(&g.name, SHORT) || !kentos_commands::is_sha256_text(&g.sha256) => Err(value(
            path,
            "Hız ızgarasının adı boş olmamalı ve SHA-256'sı küçük harfli 64 onaltılık rakam olmalı.".into(),
        )),
        _ => Ok(()),
    }
}

fn time_ok(t: Option<&str>, path: &str) -> Result<(), Stop> {
    match t {
        Some(t) if survey_time(t).is_none() => Err(value(
            path,
            format!("“{t}” geçerli bir zaman değil; YYYY-AA-GGTss:dd:sn[.kesir]Z biçiminde yazın."),
        )),
        _ => Ok(()),
    }
}

fn receiver_ok(r: Option<&SurveyReceiver>) -> Result<(), Stop> {
    let Some(r) = r else { return Ok(()) };
    let bad = !survey_text_ok(&r.kind, SHORT)
        || r.serial.as_deref().is_some_and(|t| !survey_text_ok(t, SHORT))
        || r.firmware.as_deref().is_some_and(|t| !survey_text_ok(t, SHORT));
    if bad {
        return Err(fail(
            codes::INVALID_SURVEY_TEXT,
            "Alıcının türü, seri no'su ve sürümü boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.".into(),
            "operation.receiver",
        ));
    }
    Ok(())
}

fn antenna_ok(a: Option<&SurveyAntenna>) -> Result<(), Stop> {
    match a.and_then(SurveyAntenna::problem) {
        Some((field, why)) => Err(value(&format!("operation.antenna.{field}"), format!("Anten: {why}."))),
        None => Ok(()),
    }
}

/// The file at `path` (absolute, there) and its SHA-256.
fn file_at(path: &str, at: &str) -> Result<(Vec<u8>, String), Stop> {
    if path.trim().is_empty() || path.trim() != path || !Path::new(path).is_absolute() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!("“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin."),
            at,
        ));
    }
    if !Path::new(path).is_file() {
        return Err(fail(codes::FILE_NOT_FOUND, format!("“{path}” dosyası yok."), at));
    }
    let bytes = std::fs::read(path).map_err(|_| fail(codes::FILE_NOT_FOUND, format!("“{path}” dosyası okunamadı."), at))?;
    let sha = crate::map_work::file_sha(path)
        .ok_or_else(|| fail(codes::FILE_NOT_FOUND, format!("“{path}” dosyası okunamadı."), at))?;
    Ok((bytes, sha))
}

fn frame_ok(srid: u32) -> Result<(), Stop> {
    let geographic = kentos_project::crs::system(srid)
        .is_some_and(|s| s.kind == "geographic" && s.transform_system().is_some());
    if !geographic {
        return Err(fail(
            codes::UNKNOWN_FRAME,
            format!("EPSG:{srid} kayıttaki coğrafi bir sistem değil; çözümün çerçevesini (4326 WGS 84, 5252 TUREF) verin."),
            "operation.srid",
        ));
    }
    Ok(())
}

fn applied_ok(v: f64) -> Result<(), Stop> {
    if !(v.is_finite() && (0.0..=kentos_commands::MOST_ANTENNA_HEIGHT).contains(&v)) {
        return Err(value("operation.antennaApplied", "Uygulanmış anten yüksekliği 0 ile 100 m arasında olmalı.".into()));
    }
    Ok(())
}

/// What the operation does, worked out: the archive after, the object to draw or move, the undo step's name, the
/// output and the warnings.
struct Change {
    survey: MapSurvey,
    label: &'static str,
    id: Option<String>,
    point_revision: Option<u32>,
    observations: u32,
    removed: u32,
    /// New accepted points' objects and their records' places; accepted points' objects moved.
    adds: Vec<(Entity, usize)>,
    updates: Vec<(Slot, Entity)>,
    /// The records a file brought or the points a network made.
    records: Vec<String>,
    warnings: Vec<CommandWarning>,
    /// A network's error ellipses and the use they are recorded in, its place in the archive (docs/adr/2960).
    drawn: Vec<Entity>,
    drawn_use: Option<usize>,
}

impl Change {
    fn new(survey: MapSurvey, label: &'static str) -> Self {
        Change {
            survey,
            label,
            id: None,
            point_revision: None,
            observations: 0,
            removed: 0,
            adds: Vec::new(),
            updates: Vec::new(),
            records: Vec::new(),
            warnings: Vec::new(),
            drawn: Vec::new(),
            drawn_use: None,
        }
    }
}

/// A session's file read: its format and what fills the session.
struct Read {
    file: SurveyFile,
    header: Option<kentos_commands::RinexHeader>,
    observations: Vec<GnssPoint>,
    skipped: Vec<String>,
}

fn read_session_file(path: &str) -> Result<Read, Stop> {
    let at = "operation.file";
    let (bytes, sha) = file_at(path, at)?;
    if kentos_formats::gnss::rinex::is_rinex(&bytes) {
        let header = kentos_formats::gnss::rinex::read_header(&bytes)
            .map_err(|why| fail(codes::UNREADABLE_GNSS_FILE, format!("“{path}”: {why}"), at))?;
        return Ok(Read {
            file: SurveyFile { path: path.to_owned(), revision: sha, format: SurveyFileFormat::Rinex },
            header: Some(header),
            observations: Vec::new(),
            skipped: Vec::new(),
        });
    }
    let read = kentos_formats::gnss::read(&bytes);
    if read.points.is_empty() {
        let why = read.problems.first().map_or_else(
            || "RINEX gözlem dosyası, GPX ya da NMEA kaydı değil".to_owned(),
            |p| p.message.clone(),
        );
        return Err(fail(codes::UNREADABLE_GNSS_FILE, format!("“{path}” dosyasında okunan konum yok: {why}"), at));
    }
    let format = if read.format == "gpx" { SurveyFileFormat::Gpx } else { SurveyFileFormat::Nmea };
    Ok(Read {
        file: SurveyFile { path: path.to_owned(), revision: sha, format },
        header: None,
        observations: read.points,
        skipped: read.problems.into_iter().map(|p| p.message).collect(),
    })
}

fn file_name(path: &str) -> &str {
    Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path)
}

/// A session made of what was given and what its file says.
#[allow(clippy::too_many_arguments)]
fn new_session(
    survey: &MapSurvey,
    campaign: &str,
    mark: Option<&String>,
    file: Option<&String>,
    start: Option<&String>,
    end: Option<&String>,
    time_system: Option<TimeSystem>,
    receiver: Option<&SurveyReceiver>,
    antenna: Option<&SurveyAntenna>,
    operator: Option<&String>,
) -> Result<(SurveySession, Vec<CommandWarning>), Stop> {
    if survey.campaign(campaign).is_none() {
        return Err(unknown("arazi ölçümü", campaign, "operation.campaign"));
    }
    text(mark.map(String::as_str), SHORT, "operation.mark", "İşaretin adı")?;
    text(operator.map(String::as_str), SHORT, "operation.operator", "Operatörün adı")?;
    receiver_ok(receiver)?;
    antenna_ok(antenna)?;
    time_ok(start.map(String::as_str), "operation.start")?;
    time_ok(end.map(String::as_str), "operation.end")?;
    let read = match file {
        Some(path) => Some(read_session_file(path)?),
        None => None,
    };
    let mut warnings = Vec::new();
    let header = read.as_ref().and_then(|r| r.header.clone()).unwrap_or_default();
    let mut s = SurveySession {
        id: String::new(),
        campaign: campaign.to_owned(),
        mark: mark.cloned().or(header.marker.clone()).unwrap_or_default(),
        start: start.cloned().or(header.start.clone()),
        end: end.cloned().or(header.end.clone()),
        time_system: time_system.or(header.time_system),
        receiver: receiver.cloned().or(header.receiver.clone()),
        antenna: antenna.cloned().or(header.antenna.clone()),
        operator: operator.cloned(),
        file: read.as_ref().map(|r| r.file.clone()),
        rinex: read.as_ref().and_then(|r| r.header.as_ref()).map(|h| h.facts.clone()),
        observations: read.as_ref().map(|r| r.observations.clone()).unwrap_or_default(),
    };
    if s.mark.is_empty() {
        return Err(fail(
            codes::INVALID_SURVEY_TEXT,
            "Oturumun işareti verilmedi ve dosya söylemiyor; işaretin adını verin.".into(),
            "operation.mark",
        ));
    }
    if !survey_text_ok(&s.mark, SHORT) {
        return Err(fail(
            codes::INVALID_SURVEY_TEXT,
            "Dosyadaki işaret adı kullanılamıyor; işaretin adını verin.".into(),
            "operation.mark",
        ));
    }
    if let Some(h) = &header.antenna
        && let Some((field, why)) = h.problem()
        && antenna.is_none()
    {
        return Err(value(&format!("operation.file.antenna.{field}"), format!("RINEX başlığının anteni: {why}.")));
    }
    // An NMEA log's or a GPX file's first and last full times (both UTC by their standards).
    if read.as_ref().is_some_and(|r| r.header.is_none()) {
        let times: Vec<&str> =
            s.observations.iter().filter_map(|o| o.time.as_deref()).filter(|t| survey_time(t).is_some()).collect();
        if s.start.is_none() && s.end.is_none() && !times.is_empty() {
            s.start = times.first().map(|t| (*t).to_owned());
            s.end = times.last().map(|t| (*t).to_owned());
            s.time_system = s.time_system.or(Some(TimeSystem::Utc));
        }
    }
    if let (Some(a), Some(b)) = (s.start.as_deref().and_then(survey_time), s.end.as_deref().and_then(survey_time))
        && b < a
    {
        return Err(value("operation.end", "Bitiş başlangıçtan önce olamaz.".into()));
    }
    if s.observations.len() > kentos_commands::MAX_SESSION_OBSERVATIONS {
        return Err(value(
            "operation.file",
            format!("Dosyada {} konum var; bir oturumda en çok 86400 konum olur.", s.observations.len()),
        ));
    }
    if let Some(r) = &read {
        if r.header.is_some() {
            let mut differs = Vec::new();
            if let (Some(m), Some(h)) = (mark, &header.marker)
                && m != h
            {
                differs.push(format!("işaret ({h})"));
            }
            if let (Some(a), Some(h)) = (antenna, &header.antenna) {
                if a.height != h.height {
                    differs.push(format!("anten yüksekliği ({} m)", m3(h.height)));
                }
                if let Some(k) = &h.kind
                    && a.kind.as_ref() != Some(k)
                {
                    differs.push(format!("anten türü ({k})"));
                }
            }
            if let (Some(a), Some(h)) = (receiver, &header.receiver)
                && a.kind != h.kind
            {
                differs.push(format!("alıcı ({})", h.kind));
            }
            if let (Some(a), Some(h)) = (start, &header.start)
                && survey_time(a) != survey_time(h)
            {
                differs.push(format!("başlangıç ({h})"));
            }
            if let (Some(a), Some(h)) = (end, &header.end)
                && survey_time(a) != survey_time(h)
            {
                differs.push(format!("bitiş ({h})"));
            }
            if !differs.is_empty() {
                warnings.push(CommandWarning {
                    code: codes::RINEX_DIFFERS.into(),
                    message: format!(
                        "RINEX başlığı başka değer söylüyor; verilen yazıldı: {}.",
                        differs.join(", ")
                    ),
                    path: Some("operation.file".into()),
                });
            }
        }
        if let Some(first) = r.skipped.first() {
            warnings.push(CommandWarning {
                code: codes::GNSS_LINES_SKIPPED.into(),
                message: format!(
                    "“{}” dosyasının {} satırı okunmadı; okunan {} konum yazıldı. İlki: {first}",
                    file_name(&r.file.path),
                    r.skipped.len(),
                    s.observations.len()
                ),
                path: Some("operation.file".into()),
            });
        }
    }
    Ok((s, warnings))
}

fn point_entity(layer: &str, r: &SurveyResult, mark: &str, id: &str, campaign: &str) -> Entity {
    let mut attrs = std::collections::BTreeMap::new();
    attrs.insert("Ad".to_owned(), mark.to_owned());
    attrs.insert("Kabul".to_owned(), id.to_owned());
    attrs.insert("Revizyon".to_owned(), r.revision.to_string());
    attrs.insert("Yöntem".to_owned(), r.method.title().to_owned());
    attrs.insert("Çözüm".to_owned(), made_of(r));
    attrs.insert("Arazi ölçümü".to_owned(), campaign.to_owned());
    Entity::Point(PointEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer.to_owned(),
            color: None,
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: Vec2 { x: r.x, y: r.y },
        z: r.z,
        parts: None,
    })
}

fn campaign_name(survey: &MapSurvey, r: &SurveyResult) -> String {
    crate::map_survey::campaign_of_result(survey, r).map(|c| c.name.clone()).unwrap_or_default()
}

/// What an accepted point's object says it was made of: its solutions, or its network adjustment.
fn made_of(r: &SurveyResult) -> String {
    r.adjustment.clone().unwrap_or_else(|| r.solutions.join(", "))
}

fn layer_ok(doc: &Document, layer: &str, at: &str, words: &str) -> Result<(), Stop> {
    if doc.layers().get(layer).is_none() {
        return Err(fail(codes::LAYER_NOT_FOUND, format!("“{layer}” kimlikli katman çizimde yok."), at));
    }
    if doc.layers().is_locked(layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{}” katmanı kilitli: {words} Kilidi açın.", doc.layers().path(layer)),
            at,
        ));
    }
    Ok(())
}

/// A baseline file's vectors (records without their id, campaign and file yet), its format and what was not read: a
/// GVX 1.0 file, else KentOS's baseline text (docs/adr/1740).
fn read_baselines(bytes: &[u8]) -> Result<(Vec<SurveyBaseline>, SurveyFileFormat, Vec<CommandWarning>), Stop> {
    use kentos_formats::gnss::gvx;
    let blank = |from: String, to: String, d: [f64; 3], covariance: [f64; 6], source: String| SurveyBaseline {
        id: String::new(),
        campaign: String::new(),
        from,
        to,
        d,
        covariance,
        start: None,
        end: None,
        solution: None,
        software: None,
        epochs: None,
        satellites: None,
        file: None,
        source: Some(source),
        recipe: None,
        excluded: None,
        adjust: true,
    };
    let mut warnings = Vec::new();
    if gvx::is_gvx(bytes) {
        let read = gvx::read(bytes).map_err(|why| fail(codes::UNREADABLE_GNSS_FILE, why, "operation.file"))?;
        if !read.warnings.is_empty() {
            warnings.push(CommandWarning {
                code: codes::GVX_CORRELATIONS_IGNORED.into(),
                message: read.warnings.join(" "),
                path: Some("operation.file".into()),
            });
        }
        let out = read
            .vectors
            .into_iter()
            .map(|v| SurveyBaseline {
                start: v.start,
                end: v.end,
                solution: v.solution,
                software: v.software,
                epochs: v.epochs,
                satellites: v.satellites,
                ..blank(v.from, v.to, v.d, v.covariance, v.id)
            })
            .collect();
        return Ok((out, SurveyFileFormat::Gvx, warnings));
    }
    let (enc, bom) = kentos_formats::text::sniff(bytes);
    let body = kentos_formats::text::decode(&bytes[bom..], enc);
    let (stations, vectors) = crate::survey_baselines::read_text(&body)
        .map_err(|stop| fail(codes::UNREADABLE_GNSS_FILE, stop.into_error().message, "operation.file"))?;
    if vectors.is_empty() {
        return Err(fail(
            codes::UNREADABLE_GNSS_FILE,
            "Dosyada baz satırı (B;…) yok; GVX 1.0 dosyası ya da KentOS baz metni verin.".into(),
            "operation.file",
        ));
    }
    if !stations.is_empty() {
        warnings.push(CommandWarning {
            code: codes::BASELINE_TEXT_STATIONS.into(),
            message: format!(
                "Baz metninin {} K/N satırı arşive alınmadı; bilinen noktaları çerçevesi ve epokuyla Bilinen nokta ekle ile girin.",
                stations.len()
            ),
            path: Some("operation.file".into()),
        });
    }
    let out = vectors.into_iter().enumerate().map(|(k, v)| blank(v.from, v.to, v.d, v.cov, format!("B{}", k + 1))).collect();
    Ok((out, SurveyFileFormat::Text, warnings))
}

/// The network adjusted and accepted (docs/adr/1740): refused while a stage has a blunder or fails its model test or a
/// mark does not reach the project; else `da-N` and each new mark's point accepted on `layer` or made again, its
/// older result kept, in one step.
fn network(
    doc: &Document,
    now: &MapSurvey,
    mut next: MapSurvey,
    input: &MapSurveyNetwork,
    layer: Option<&str>,
    ellipses: Option<&kentos_commands::NetworkEllipses>,
) -> Result<Change, Stop> {
    let a = crate::map_survey_network::analyse(doc, input, "operation.")?;
    if let Some(id) = &a.out.current {
        return Err(fail(
            codes::NOTHING_TO_SET,
            format!("Ağ “{id}” dengelemesinden beri değişmedi; noktaları güncel."),
            "operation",
        ));
    }
    for st in [&a.out.free, &a.out.constrained] {
        if st.blunders > 0 {
            let which = if std::ptr::eq(st, &a.out.free) { "Serbest dengelemede" } else { "Bilinen noktalara dayalı dengelemede" };
            return Err(fail(
                codes::NETWORK_BLUNDER,
                format!(
                    "{which} {} uyuşumsuz ölçü var; en kötüsü “{}”. Dışlayıp nedenini yazın ya da yeniden işleyin, sonra yeniden dengeleyin.",
                    st.blunders,
                    st.worst.clone().unwrap_or_default()
                ),
                "operation",
            ));
        }
    }
    for st in [&a.out.free, &a.out.constrained] {
        if st.passed == Some(false) {
            let m0 = st.m0.unwrap_or(0.0);
            return Err(fail(
                codes::NETWORK_TEST_FAILED,
                format!(
                    "Ağın model testi (χ²) geçmedi: m0 = {m0:.2}. Varyans çarpanını yaklaşık {:.2} (m0²·çarpan) yapıp yeniden dengeleyin.",
                    m0 * m0 * a.scale
                ),
                "operation.scale",
            ));
        }
    }
    for n in &a.new {
        if let Err(e) = &n.placed {
            return Err(Stop::failed(e.clone()));
        }
    }
    if a.new.is_empty() {
        return Err(fail(
            codes::NOTHING_TO_SET,
            "Ağda bilinen nokta olmayan işaret yok; dengelenecek nokta yok.".into(),
            "operation",
        ));
    }
    let layer = layer.map_or_else(|| doc.layers().active().to_owned(), str::to_owned);
    if let Some(with) = ellipses {
        crate::network_ellipses::check(doc, with)?;
    }
    if now.adjustments.len() >= kentos_commands::MAX_SURVEY_ADJUSTMENTS {
        return Err(full("1000 ağ dengelemesi"));
    }
    let da = next_survey_id(now.adjustments.iter().map(|x| x.id.as_str()), "da-");
    let mut change = Change::new(MapSurvey::default(), "Ağ dengelemesini kabul et");
    let mut layer_checked = false;
    for n in &a.new {
        let Ok(placed) = &n.placed else { continue };
        let mut result = SurveyResult {
            revision: 1,
            solutions: Vec::new(),
            method: AcceptMethod::Network,
            lat: n.lat,
            lon: n.lon,
            height: n.height,
            srid: a.out.srid,
            epoch: a.out.epoch,
            target_epoch: placed.target,
            antenna: Vec::new(),
            sigma: Some(n.sigma),
            x: placed.x,
            y: placed.y,
            z: placed.z,
            chain: placed.chain.clone(),
            reasons: Vec::new(),
            adjustment: Some(da.clone()),
        };
        match now.points.iter().find(|p| p.mark == n.mark) {
            Some(old) => {
                let slot = doc.slot_of(Uuid::from_bytes(old.uid.0)).filter(|_| present(doc, &old.uid));
                let Some((slot, entity)) = slot.and_then(|s| doc.get(s).map(|e| (s, e.clone()))) else {
                    return Err(fail(
                        codes::ENTITY_NOT_FOUND,
                        format!("“{}” noktasının nesnesi çizimde yok; geri alın ya da kaydı çıkarıp ağı yeniden kabul edin.", old.id),
                        "operation",
                    ));
                };
                let on = entity.base().layer_id.clone();
                if doc.layers().is_locked(&on) {
                    return Err(fail(
                        codes::LAYER_LOCKED,
                        format!("“{}” katmanı kilitli: “{}” noktası yeni yerine taşınamaz. Kilidi açın.", doc.layers().path(&on), old.id),
                        "operation",
                    ));
                }
                if old.history.len() >= kentos_commands::MAX_POINT_HISTORY {
                    return Err(full("100 eski sonuç (bir noktanın)"));
                }
                result.revision = old.result.revision + 1;
                result.reasons = vec![format!("“{da}” ağ dengelemesiyle üretildi.")];
                let mut moved = entity.clone();
                if let Entity::Point(pt) = &mut moved {
                    pt.p = Vec2 { x: result.x, y: result.y };
                    pt.z = result.z;
                    pt.base.attrs.insert("Revizyon".into(), result.revision.to_string());
                    pt.base.attrs.insert("Yöntem".into(), result.method.title().into());
                    pt.base.attrs.insert("Çözüm".into(), made_of(&result));
                }
                if let Some(p) = next.points.iter_mut().find(|p| p.id == old.id) {
                    p.history.push(p.result.clone());
                    p.result = result;
                }
                change.updates.push((slot, moved));
                change.records.push(old.id.clone());
            }
            None => {
                if !layer_checked {
                    layer_ok(doc, &layer, "operation.layer", "kabul edilen nokta kilitli katmana çizilmez.")?;
                    layer_checked = true;
                }
                if next.points.len() >= kentos_commands::MAX_SURVEY_POINTS {
                    return Err(full("10 000 kabul edilen nokta"));
                }
                let id = next_survey_id(next.points.iter().map(|x| x.id.as_str()), "kn-");
                next.points.push(SurveyPoint {
                    id: id.clone(),
                    mark: n.mark.clone(),
                    uid: EntityId([0; 16]),
                    result,
                    history: Vec::new(),
                });
                change.records.push(id);
            }
        }
    }
    let (class, rules) = match &a.class {
        Some((c, r)) => (Some(c.clone()), Some(r.clone())),
        None => (None, None),
    };
    next.adjustments.push(SurveyAdjustment {
        id: da.clone(),
        baselines: a.baselines.clone(),
        known: a.known.clone(),
        hold: a.hold.clone(),
        scale: a.scale,
        class,
        rules,
        rules_passed: a.out.rules.as_ref().map(|r| r.passed),
        free_m0: a.out.free.m0,
        free_dof: a.out.free.dof,
        m0: a.out.constrained.m0,
        dof: a.out.constrained.dof,
        srid: a.out.srid,
        epoch: a.out.epoch,
    });
    // Each point's error ellipse, recorded as made from the points at their revisions now (docs/adr/2960).
    if let Some(with) = ellipses {
        let mut points = Vec::new();
        for n in &a.new {
            let Ok(placed) = &n.placed else { continue };
            let Some(e) = a.out.stations.iter().find(|s| s.mark == n.mark).and_then(|s| s.ellipse.as_ref()) else {
                continue;
            };
            let at = Vec2 { x: placed.x, y: placed.y };
            let turn = crate::network_ellipses::convergence(doc, at, &n.mark)?;
            change.drawn.push(crate::network_ellipses::ellipse(&n.mark, &da, at, e, with, turn));
            // Docs/adr/3360: and its height's half-interval as a bar, recorded with it.
            if with.heights {
                change.drawn.push(crate::network_ellipses::height_bar(&n.mark, &da, at, e, with));
            }
            if let Some(p) = next.points.iter().find(|p| p.mark == n.mark) {
                points.push(PointRevision { point: p.id.clone(), revision: p.result.revision });
            }
        }
        if !change.drawn.is_empty() {
            if next.uses.len() >= kentos_commands::MAX_SURVEY_USES {
                return Err(full("10 000 dayanan kaydı"));
            }
            let id = next_survey_id(next.uses.iter().map(|u| u.id.as_str()), "kb-");
            next.uses.push(SurveyUse { id, operation: format!("Hata elipsleri ({da})"), uids: Vec::new(), points });
            change.drawn_use = Some(next.uses.len() - 1);
        }
    }
    // The new points' objects, drawn once the archive after knows their records' places and campaign.
    for (at, p) in next.points.iter().enumerate() {
        if p.uid == EntityId([0; 16]) {
            let entity = point_entity(&layer, &p.result, &p.mark, &p.id, &campaign_name(&next, &p.result));
            change.adds.push((entity, at));
        }
    }
    change.warnings = crate::map_survey_network::warnings_of(&a.out, "operation.");
    change.id = Some(da);
    change.survey = next;
    Ok(change)
}

/// Where a used record is held: the accepted point using a solution of these sessions.
fn used_by<'a>(survey: &'a MapSurvey, sessions: &[&str]) -> Option<(&'a str, &'a str)> {
    survey
        .solutions
        .iter()
        .filter(|s| sessions.contains(&s.session.as_str()))
        .find_map(|s| survey.point_using(&s.id).map(|p| (s.id.as_str(), p.id.as_str())))
}

#[allow(clippy::too_many_lines)]
fn check(doc: &Document, input: &MapSurveyEdit) -> Result<Change, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    map_project(doc)?;
    let now = doc.map_survey().cloned().unwrap_or_default();
    let mut next = now.clone();
    match &input.operation {
        Op::Campaign { name, work, operator, epoch, velocity } => {
            text(Some(name), SHORT, "operation.name", "Arazi ölçümünün adı")?;
            work_ok(doc, work.as_deref())?;
            text(operator.as_deref(), SHORT, "operation.operator", "Operatörün adı")?;
            epoch_ok(*epoch, "operation.epoch")?;
            grid_ok(velocity.as_ref(), "operation.velocity")?;
            if now.campaigns.len() >= kentos_commands::MAX_SURVEY_CAMPAIGNS {
                return Err(full("1000 arazi ölçümü"));
            }
            let id = next_survey_id(now.campaigns.iter().map(|c| c.id.as_str()), "kp-");
            next.campaigns.push(SurveyCampaign {
                id: id.clone(),
                name: name.clone(),
                work: work.clone(),
                operator: operator.clone(),
                epoch: *epoch,
                velocity: velocity.clone(),
            });
            Ok(Change { id: Some(id), ..Change::new(next, "Arazi ölçümü ekle") })
        }
        Op::SetCampaign { campaign, name, work, operator, epoch, velocity } => {
            let Some(old) = now.campaign(campaign) else {
                return Err(unknown("arazi ölçümü", campaign, "operation.campaign"));
            };
            text(name.as_deref(), SHORT, "operation.name", "Arazi ölçümünün adı")?;
            work_ok(doc, work.as_deref())?;
            text(operator.as_deref(), SHORT, "operation.operator", "Operatörün adı")?;
            epoch_ok(*epoch, "operation.epoch")?;
            grid_ok(velocity.as_ref(), "operation.velocity")?;
            let mut c = old.clone();
            if let Some(v) = name {
                c.name = v.clone();
            }
            if work.is_some() {
                c.work = work.clone();
            }
            if operator.is_some() {
                c.operator = operator.clone();
            }
            if epoch.is_some() {
                c.epoch = *epoch;
            }
            if velocity.is_some() {
                c.velocity = velocity.clone();
            }
            if c == *old {
                return Err(nothing());
            }
            if let Some(slot) = next.campaigns.iter_mut().find(|x| x.id == *campaign) {
                *slot = c;
            }
            Ok(Change { id: Some(campaign.clone()), ..Change::new(next, "Arazi ölçümünü değiştir") })
        }
        Op::Session { campaign, mark, file, start, end, time_system, receiver, antenna, operator } => {
            let (mut s, warnings) = new_session(
                &now,
                campaign,
                mark.as_ref(),
                file.as_ref(),
                start.as_ref(),
                end.as_ref(),
                *time_system,
                receiver.as_ref(),
                antenna.as_ref(),
                operator.as_ref(),
            )?;
            if now.sessions.len() >= kentos_commands::MAX_SURVEY_SESSIONS {
                return Err(full("10 000 oturum"));
            }
            s.id = next_survey_id(now.sessions.iter().map(|x| x.id.as_str()), "os-");
            let id = s.id.clone();
            let observations = u32::try_from(s.observations.len()).unwrap_or(u32::MAX);
            next.sessions.push(s);
            Ok(Change { id: Some(id), observations, warnings, ..Change::new(next, "GNSS oturumu ekle") })
        }
        Op::SetSession { session, mark, start, end, time_system, receiver, antenna, operator } => {
            let Some(old) = now.session(session) else {
                return Err(unknown("oturum", session, "operation.session"));
            };
            text(mark.as_deref(), SHORT, "operation.mark", "İşaretin adı")?;
            text(operator.as_deref(), SHORT, "operation.operator", "Operatörün adı")?;
            receiver_ok(receiver.as_ref())?;
            antenna_ok(antenna.as_ref())?;
            time_ok(start.as_deref(), "operation.start")?;
            time_ok(end.as_deref(), "operation.end")?;
            let mut s = old.clone();
            if let Some(m) = mark {
                s.mark = m.clone();
            }
            if start.is_some() {
                s.start = start.clone();
            }
            if end.is_some() {
                s.end = end.clone();
            }
            if time_system.is_some() {
                s.time_system = *time_system;
            }
            if receiver.is_some() {
                s.receiver = receiver.clone();
            }
            if antenna.is_some() {
                s.antenna = antenna.clone();
            }
            if operator.is_some() {
                s.operator = operator.clone();
            }
            if s == *old {
                return Err(nothing());
            }
            if s.mark != old.mark
                && let Some((_, p)) = used_by(&now, &[session.as_str()])
            {
                return Err(fail(
                    codes::SURVEY_IN_USE,
                    format!("“{session}” oturumunun çözümü “{p}” noktasında; oturumun işareti değişmez."),
                    "operation.mark",
                ));
            }
            if let (Some(a), Some(b)) = (s.start.as_deref().and_then(survey_time), s.end.as_deref().and_then(survey_time))
                && b < a
            {
                return Err(value("operation.end", "Bitiş başlangıçtan önce olamaz.".into()));
            }
            if let Some(slot) = next.sessions.iter_mut().find(|x| x.id == *session) {
                *slot = s;
            }
            Ok(Change { id: Some(session.clone()), ..Change::new(next, "Oturumu değiştir") })
        }
        Op::Solve { session, srid, antenna_applied, epoch, from, to, fixes } => {
            let Some(s) = now.session(session) else {
                return Err(unknown("oturum", session, "operation.session"));
            };
            frame_ok(*srid)?;
            applied_ok(*antenna_applied)?;
            epoch_ok(*epoch, "operation.epoch")?;
            let n = s.observations.len();
            if n == 0 {
                return Err(fail(
                    codes::NO_OBSERVATIONS,
                    format!("“{session}” oturumunda ham ölçüm yok; NMEA ya da GPX dosyasıyla eklenen oturum çözülür."),
                    "operation.session",
                ));
            }
            let a = from.unwrap_or(0) as usize;
            let b = to.map_or(n - 1, |t| t as usize);
            if a > b || b >= n {
                return Err(value("operation.to", format!("Aralık oturumun 0–{} konumları içinde olmalı.", n - 1)));
            }
            let taken: Vec<&GnssPoint> = s.observations[a..=b]
                .iter()
                .filter(|o| fixes.is_empty() || o.fix.as_ref().is_some_and(|f| fixes.contains(f)))
                .collect();
            if taken.is_empty() {
                return Err(fail(
                    codes::NO_OBSERVATIONS,
                    "Seçilen aralıkta ve fix'lerde ham ölçüm yok.".into(),
                    "operation.fixes",
                ));
            }
            // A position without an ellipsoidal height (GPX's `ele`, a GGA without its geoid separation) is left out;
            // none left is refused.
            let taken: Vec<&GnssPoint> = taken.into_iter().filter(|o| o.ellipsoidal.is_some()).collect();
            if taken.is_empty() {
                return Err(fail(
                    codes::NO_ELLIPSOIDAL_HEIGHT,
                    "Seçilen ham ölçümlerin hiçbirinin elipsoit yüksekliği yok (GPX'in yüksekliğinin datumu bilinmez); başka aralık ya da fix seçin.".into(),
                    "operation.from",
                ));
            }
            let (mut lat, mut lon, mut h) = (0.0, 0.0, 0.0);
            for o in &taken {
                lat += o.lat;
                lon += o.lon;
                h += o.ellipsoidal.unwrap_or(0.0);
            }
            let k = taken.len() as f64;
            let first_fix = taken[0].fix.clone();
            let fix = taken.iter().all(|o| o.fix == first_fix).then_some(first_fix).flatten();
            let satellites = taken.iter().filter_map(|o| o.satellites).min();
            let hdop = taken.iter().filter_map(|o| o.hdop).fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| if v > m { v } else { m })));
            if now.solutions.len() >= kentos_commands::MAX_SURVEY_SOLUTIONS {
                return Err(full("100 000 çözüm"));
            }
            let id = next_survey_id(now.solutions.iter().map(|x| x.id.as_str()), "cz-");
            next.solutions.push(SurveySolution {
                id: id.clone(),
                session: session.clone(),
                kind: SolutionKind::Receiver,
                lat: lat / k,
                lon: lon / k,
                height: h / k,
                srid: *srid,
                epoch: *epoch,
                antenna_applied: *antenna_applied,
                fix,
                epochs: Some(u32::try_from(taken.len()).unwrap_or(u32::MAX)),
                satellites,
                hdop,
                sigma: None,
                covariance: None,
                correction: None,
                age: None,
                report: None,
                recipe: None,
                from: Some(u32::try_from(a).unwrap_or(u32::MAX)),
                to: Some(u32::try_from(b).unwrap_or(u32::MAX)),
                fixes: fixes.clone(),
                excluded: None,
                adjust: false,
            });
            Ok(Change { id: Some(id), ..Change::new(next, "Alıcı çözümü") })
        }
        Op::Solution {
            session,
            kind,
            lat,
            lon,
            height,
            srid,
            antenna_applied,
            epoch,
            fix,
            epochs,
            satellites,
            hdop,
            sigma,
            covariance,
            correction,
            age,
            report,
            recipe,
        } => {
            if now.session(session).is_none() {
                return Err(unknown("oturum", session, "operation.session"));
            }
            frame_ok(*srid)?;
            text(fix.as_deref(), SHORT, "operation.fix", "Fix")?;
            text(correction.as_deref(), SHORT, "operation.correction", "Düzeltme kaynağı")?;
            text(recipe.as_deref(), kentos_commands::MOST_RECIPE_TEXT, "operation.recipe", "Yeniden işleme tarifi")?;
            let mut s = SurveySolution {
                id: String::new(),
                session: session.clone(),
                kind: *kind,
                lat: *lat,
                lon: *lon,
                height: *height,
                srid: *srid,
                epoch: *epoch,
                antenna_applied: *antenna_applied,
                fix: fix.clone(),
                epochs: *epochs,
                satellites: *satellites,
                hdop: *hdop,
                sigma: *sigma,
                covariance: *covariance,
                correction: correction.clone(),
                age: *age,
                report: None,
                recipe: recipe.clone(),
                from: None,
                to: None,
                fixes: Vec::new(),
                excluded: None,
                adjust: false,
            };
            if let Some((field, why)) = kentos_commands::solution_numbers_problem(&s) {
                return Err(value(&format!("operation.{field}"), format!("Çözüm: {why}.")));
            }
            if let Some(path) = report {
                let (_, sha) = file_at(path, "operation.report")?;
                s.report = Some(SurveyFile { path: path.clone(), revision: sha, format: SurveyFileFormat::Report });
            }
            if now.solutions.len() >= kentos_commands::MAX_SURVEY_SOLUTIONS {
                return Err(full("100 000 çözüm"));
            }
            s.id = next_survey_id(now.solutions.iter().map(|x| x.id.as_str()), "cz-");
            let id = s.id.clone();
            next.solutions.push(s);
            Ok(Change { id: Some(id), ..Change::new(next, "Çözüm ekle") })
        }
        Op::Exclude { solution, reason } if solution.starts_with("bz-") || solution.starts_with("bn-") => {
            text(Some(reason), kentos_commands::MOST_REASON_TEXT, "operation.reason", "Dışlama nedeni")?;
            let was = match (now.baseline(solution), now.known_point(solution)) {
                (Some(b), _) => b.excluded.clone(),
                (_, Some(k)) => k.excluded.clone(),
                _ => return Err(unknown("baz ya da bilinen nokta", solution, "operation.solution")),
            };
            if was.as_ref() == Some(reason) {
                return Err(nothing());
            }
            if let Some(b) = next.baselines.iter_mut().find(|x| x.id == *solution) {
                b.excluded = Some(reason.clone());
            }
            if let Some(k) = next.known.iter_mut().find(|x| x.id == *solution) {
                k.excluded = Some(reason.clone());
            }
            let label = if solution.starts_with("bz-") { "Bazı dışla" } else { "Bilinen noktayı dışla" };
            Ok(Change { id: Some(solution.clone()), ..Change::new(next, label) })
        }
        Op::Include { solution } if solution.starts_with("bz-") || solution.starts_with("bn-") => {
            let was = match (now.baseline(solution), now.known_point(solution)) {
                (Some(b), _) => b.excluded.clone(),
                (_, Some(k)) => k.excluded.clone(),
                _ => return Err(unknown("baz ya da bilinen nokta", solution, "operation.solution")),
            };
            if was.is_none() {
                return Err(nothing());
            }
            if let Some(k) = now.known_point(solution)
                && let Some(other) = now.known.iter().find(|x| x.mark == k.mark && x.excluded.is_none())
            {
                return Err(value(
                    "operation.solution",
                    format!("“{}” işaretinin dışlanmamış bilinen noktası var (“{}”); önce onu dışlayın.", k.mark, other.id),
                ));
            }
            if let Some(b) = next.baselines.iter_mut().find(|x| x.id == *solution) {
                b.excluded = None;
            }
            if let Some(k) = next.known.iter_mut().find(|x| x.id == *solution) {
                k.excluded = None;
            }
            Ok(Change { id: Some(solution.clone()), ..Change::new(next, "Dışlamayı kaldır") })
        }
        Op::Adjust { solution, adjust } if solution.starts_with("bz-") || solution.starts_with("bn-") => {
            let Some(old) = now.baseline(solution) else {
                if now.known_point(solution).is_some() {
                    return Err(value(
                        "operation.solution",
                        format!("“{solution}” bilinen nokta; bilinen nokta hep dengelemeye girer, çıkarmak için dışlayın."),
                    ));
                }
                return Err(unknown("baz", solution, "operation.solution"));
            };
            if *adjust && old.excluded.is_some() {
                return Err(fail(
                    codes::SOLUTION_EXCLUDED,
                    format!("“{solution}” bazı dışlanmış; dengelemeye girmez. Önce dışlamayı kaldırın."),
                    "operation.adjust",
                ));
            }
            if old.adjust == *adjust {
                return Err(nothing());
            }
            if let Some(b) = next.baselines.iter_mut().find(|x| x.id == *solution) {
                b.adjust = *adjust;
            }
            Ok(Change { id: Some(solution.clone()), ..Change::new(next, "Dengeleme seçimi") })
        }
        Op::Baselines { campaign, file } => {
            if now.campaign(campaign).is_none() {
                return Err(unknown("arazi ölçümü", campaign, "operation.campaign"));
            }
            let (bytes, sha) = file_at(file, "operation.file")?;
            let (vectors, format, mut warnings) = read_baselines(&bytes)?;
            if now.baselines.len() + vectors.len() > kentos_commands::MAX_SURVEY_BASELINES {
                return Err(full("100 000 baz"));
            }
            let mut ids = Vec::new();
            for v in vectors {
                let id = next_survey_id(next.baselines.iter().map(|x| x.id.as_str()), "bz-");
                let b = SurveyBaseline {
                    id: id.clone(),
                    campaign: campaign.clone(),
                    file: Some(SurveyFile { path: file.clone(), revision: sha.clone(), format }),
                    ..v
                };
                if let Some((field, why)) = baseline_numbers_problem(&b) {
                    return Err(value(
                        "operation.file",
                        format!("“{}” bazı ({}): {why}.", b.source.clone().unwrap_or_default(), field),
                    ));
                }
                next.baselines.push(b);
                ids.push(id);
            }
            let first = ids.first().cloned();
            let mut c = Change { id: first, records: ids, ..Change::new(next, "Baz ekle") };
            c.warnings.append(&mut warnings);
            Ok(c)
        }
        Op::Baseline { campaign, from, to, d, covariance, start, end, solution, recipe } => {
            if now.campaign(campaign).is_none() {
                return Err(unknown("arazi ölçümü", campaign, "operation.campaign"));
            }
            text(Some(from), SHORT, "operation.from", "Bazın başlangıç işareti")?;
            text(Some(to), SHORT, "operation.to", "Bazın bitiş işareti")?;
            if from == to {
                return Err(value("operation.to", "Bazın iki ucu aynı işaret olamaz.".into()));
            }
            time_ok(start.as_deref(), "operation.start")?;
            time_ok(end.as_deref(), "operation.end")?;
            if let (Some(a), Some(b)) = (start.as_deref().and_then(survey_time), end.as_deref().and_then(survey_time))
                && b < a
            {
                return Err(value("operation.end", "Bitiş başlangıçtan önce olamaz.".into()));
            }
            text(recipe.as_deref(), kentos_commands::MOST_RECIPE_TEXT, "operation.recipe", "Yeniden işleme tarifi")?;
            if now.baselines.len() >= kentos_commands::MAX_SURVEY_BASELINES {
                return Err(full("100 000 baz"));
            }
            let id = next_survey_id(now.baselines.iter().map(|x| x.id.as_str()), "bz-");
            let b = SurveyBaseline {
                id: id.clone(),
                campaign: campaign.clone(),
                from: from.clone(),
                to: to.clone(),
                d: *d,
                covariance: *covariance,
                start: start.clone(),
                end: end.clone(),
                solution: *solution,
                software: None,
                epochs: None,
                satellites: None,
                file: None,
                source: None,
                recipe: recipe.clone(),
                excluded: None,
                adjust: true,
            };
            if let Some((field, why)) = baseline_numbers_problem(&b) {
                return Err(value(&format!("operation.{field}"), format!("Baz: {why}.")));
            }
            next.baselines.push(b);
            Ok(Change { id: Some(id), ..Change::new(next, "Baz ekle") })
        }
        Op::Known { mark, lat, lon, height, srid, epoch, sigma, velocity, source } => {
            text(Some(mark), SHORT, "operation.mark", "İşaretin adı")?;
            text(source.as_deref(), kentos_commands::MOST_REASON_TEXT, "operation.source", "Kaynak")?;
            let k = SurveyKnown {
                id: next_survey_id(now.known.iter().map(|x| x.id.as_str()), "bn-"),
                mark: mark.clone(),
                lat: *lat,
                lon: *lon,
                height: *height,
                srid: *srid,
                epoch: *epoch,
                sigma: *sigma,
                velocity: *velocity,
                source: source.clone(),
                excluded: None,
            };
            if let Some((field, why)) = known_numbers_problem(&k) {
                return Err(value(&format!("operation.{field}"), format!("Bilinen nokta: {why}.")));
            }
            frame_ok(*srid)?;
            if let Some(other) = now.known.iter().find(|x| x.mark == *mark && x.excluded.is_none()) {
                return Err(value(
                    "operation.mark",
                    format!("“{mark}” işaretinin dışlanmamış bilinen noktası var (“{}”); önce onu dışlayın.", other.id),
                ));
            }
            if now.known.len() >= kentos_commands::MAX_SURVEY_KNOWN {
                return Err(full("10 000 bilinen nokta"));
            }
            let id = k.id.clone();
            next.known.push(k);
            Ok(Change { id: Some(id), ..Change::new(next, "Bilinen nokta ekle") })
        }
        Op::Network { campaign, scale, class, rules, hold, epoch, layer, ellipses } => network(
            doc,
            &now,
            next,
            &MapSurveyNetwork {
                campaign: campaign.clone(),
                scale: *scale,
                class: class.clone(),
                rules: rules.clone(),
                hold: hold.clone(),
                epoch: *epoch,
            },
            layer.as_deref(),
            ellipses.as_ref(),
        ),
        Op::Exclude { solution, reason } => {
            let Some(old) = now.solution(solution) else {
                return Err(unknown("çözüm", solution, "operation.solution"));
            };
            text(Some(reason), kentos_commands::MOST_REASON_TEXT, "operation.reason", "Dışlama nedeni")?;
            if old.excluded.as_ref() == Some(reason) {
                return Err(nothing());
            }
            if let Some(s) = next.solutions.iter_mut().find(|x| x.id == *solution) {
                s.excluded = Some(reason.clone());
            }
            Ok(Change { id: Some(solution.clone()), ..Change::new(next, "Çözümü dışla") })
        }
        Op::Include { solution } => {
            let Some(old) = now.solution(solution) else {
                return Err(unknown("çözüm", solution, "operation.solution"));
            };
            if old.excluded.is_none() {
                return Err(nothing());
            }
            if let Some(s) = next.solutions.iter_mut().find(|x| x.id == *solution) {
                s.excluded = None;
            }
            Ok(Change { id: Some(solution.clone()), ..Change::new(next, "Dışlamayı kaldır") })
        }
        Op::Adjust { solution, adjust } => {
            let Some(old) = now.solution(solution) else {
                return Err(unknown("çözüm", solution, "operation.solution"));
            };
            if *adjust && old.excluded.is_some() {
                return Err(fail(
                    codes::SOLUTION_EXCLUDED,
                    format!("“{solution}” çözümü dışlanmış; dengelemeye girmez. Önce dışlamayı kaldırın."),
                    "operation.adjust",
                ));
            }
            if old.adjust == *adjust {
                return Err(nothing());
            }
            if let Some(s) = next.solutions.iter_mut().find(|x| x.id == *solution) {
                s.adjust = *adjust;
            }
            Ok(Change { id: Some(solution.clone()), ..Change::new(next, "Dengeleme seçimi") })
        }
        Op::Accept { mark, solutions, method, layer } => {
            text(Some(mark), SHORT, "operation.mark", "İşaretin adı")?;
            if let Some(p) = now.points.iter().find(|p| p.mark == *mark) {
                return Err(fail(
                    codes::ALREADY_ACCEPTED,
                    format!("“{mark}” işaretinin kabul edilen noktası var (“{}”); Yeniden üret ile güncelleyin.", p.id),
                    "operation.mark",
                ));
            }
            let method = method.unwrap_or(if solutions.len() == 1 { AcceptMethod::Single } else { AcceptMethod::Mean });
            let result = compute(doc, &now, mark, solutions, method, "operation.solutions")?;
            let layer = layer.clone().unwrap_or_else(|| doc.layers().active().to_owned());
            layer_ok(doc, &layer, "operation.layer", "kabul edilen nokta kilitli katmana çizilmez.")?;
            if now.points.len() >= kentos_commands::MAX_SURVEY_POINTS {
                return Err(full("10 000 kabul edilen nokta"));
            }
            let id = next_survey_id(now.points.iter().map(|x| x.id.as_str()), "kn-");
            let entity = point_entity(&layer, &result, mark, &id, &campaign_name(&now, &result));
            next.points.push(SurveyPoint {
                id: id.clone(),
                mark: mark.clone(),
                uid: EntityId([0; 16]),
                result,
                history: Vec::new(),
            });
            let at = next.points.len() - 1;
            Ok(Change {
                id: Some(id),
                point_revision: Some(1),
                adds: vec![(entity, at)],
                ..Change::new(next, "Noktayı kabul et")
            })
        }
        Op::Regenerate { point, solutions, method } => {
            let Some(old) = now.point(point) else {
                return Err(unknown("kabul edilen nokta", point, "operation.point"));
            };
            let slot = doc.slot_of(Uuid::from_bytes(old.uid.0)).filter(|_| present(doc, &old.uid));
            let Some((slot, entity)) = slot.and_then(|s| doc.get(s).map(|e| (s, e.clone()))) else {
                return Err(fail(
                    codes::ENTITY_NOT_FOUND,
                    format!("“{point}” noktasının nesnesi çizimde yok; geri alın ya da kaydı çıkarıp noktayı yeniden kabul edin."),
                    "operation.point",
                ));
            };
            let layer = entity.base().layer_id.clone();
            if old.result.method == AcceptMethod::Network && solutions.is_none() {
                return Err(value(
                    "operation.point",
                    format!("“{point}” ağ dengelemesiyle üretildi; ağı yeniden dengeleyip kabul edin ya da çözümlerini verin."),
                ));
            }
            let ids = solutions.clone().unwrap_or_else(|| old.result.solutions.clone());
            let method = method.unwrap_or(if old.result.method == AcceptMethod::Network {
                if ids.len() == 1 { AcceptMethod::Single } else { AcceptMethod::Mean }
            } else {
                old.result.method
            });
            let mut reasons = point_why(doc, &now, old);
            // Nothing it rests on moved and nothing else is asked: the same point (judged by its inputs, not by
            // recomputing its numbers).
            if reasons.is_empty() && ids == old.result.solutions && method == old.result.method {
                return Err(nothing());
            }
            let mut result = compute(doc, &now, &old.mark, &ids, method, "operation.solutions")?;
            if ids != old.result.solutions {
                reasons.push(format!(
                    "Çözümler {} iken {} oldu.",
                    old.result.solutions.join(", "),
                    ids.join(", ")
                ));
            }
            if method != old.result.method {
                reasons.push(format!("Yöntem {} iken {} oldu.", old.result.method.title(), method.title()));
            }
            if doc.layers().is_locked(&layer) {
                return Err(fail(
                    codes::LAYER_LOCKED,
                    format!("“{}” katmanı kilitli: nokta yeni yerine taşınamaz. Kilidi açın.", doc.layers().path(&layer)),
                    "operation.point",
                ));
            }
            if old.history.len() >= kentos_commands::MAX_POINT_HISTORY {
                return Err(full("100 eski sonuç (bir noktanın)"));
            }
            result.revision = old.result.revision + 1;
            result.reasons = reasons;
            let mut moved = entity.clone();
            if let Entity::Point(pt) = &mut moved {
                pt.p = Vec2 { x: result.x, y: result.y };
                pt.z = result.z;
                pt.base.attrs.insert("Revizyon".into(), result.revision.to_string());
                pt.base.attrs.insert("Yöntem".into(), result.method.title().into());
                pt.base.attrs.insert("Çözüm".into(), made_of(&result));
            }
            let revision = result.revision;
            if let Some(p) = next.points.iter_mut().find(|p| p.id == *point) {
                p.history.push(p.result.clone());
                p.result = result;
            }
            Ok(Change {
                id: Some(point.clone()),
                point_revision: Some(revision),
                updates: vec![(slot, moved)],
                ..Change::new(next, "Noktayı yeniden üret")
            })
        }
        Op::Use { uids, points, operation } => {
            text(Some(operation), SHORT, "operation.operation", "İşlem")?;
            if points.is_empty() {
                return Err(value("operation.points", "Dayanan en az bir kabul edilen noktaya dayanır.".into()));
            }
            let mut revs = Vec::new();
            for (i, p) in points.iter().enumerate() {
                let at = format!("operation.points[{i}]");
                let Some(found) = now.point(p) else {
                    return Err(unknown("kabul edilen nokta", p, &at));
                };
                if points[..i].contains(p) {
                    return Err(value(&at, format!("“{p}” noktası iki kez verilmiş.")));
                }
                revs.push(PointRevision { point: p.clone(), revision: found.result.revision });
            }
            if uids.is_empty() {
                return Err(fail(
                    codes::NO_ENTITIES,
                    "Dayanan nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.".into(),
                    "operation.uids",
                ));
            }
            let mut ids = Vec::new();
            for (i, u) in uids.iter().enumerate() {
                let at = format!("operation.uids[{i}]");
                if !checks::is_uid_text(u) {
                    return Err(fail(codes::INVALID_UID, format!("“{u}” geçerli bir nesne kimliği değil."), &at));
                }
                let Some(uuid) = Uuid::parse_str(u).ok().filter(|x| doc.slot_of(*x).is_some()) else {
                    return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{u}” kimlikli nesne çizimde yok."), &at));
                };
                let id = EntityId(*uuid.as_bytes());
                if let Some(other) = now.uses.iter().find(|x| x.uids.contains(&id)) {
                    return Err(fail(
                        codes::SURVEY_IN_USE,
                        format!("“{u}” nesnesi “{}” dayanan kaydında; önce o kaydı çıkarın.", other.id),
                        &at,
                    ));
                }
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            if now.uses.len() >= kentos_commands::MAX_SURVEY_USES {
                return Err(full("10 000 dayanan"));
            }
            let id = next_survey_id(now.uses.iter().map(|x| x.id.as_str()), "kb-");
            next.uses.push(SurveyUse { id: id.clone(), operation: operation.clone(), uids: ids, points: revs });
            Ok(Change { id: Some(id), ..Change::new(next, "Dayanan kaydı") })
        }
        Op::Remove { id } => {
            let removed: usize;
            let at = "operation.id";
            if now.campaign(id).is_some() {
                let sessions: Vec<&str> =
                    now.sessions.iter().filter(|s| s.campaign == *id).map(|s| s.id.as_str()).collect();
                if let Some((s, p)) = used_by(&now, &sessions) {
                    return Err(fail(
                        codes::SURVEY_IN_USE,
                        format!("“{id}” kaydının “{s}” çözümü “{p}” noktasında; önce noktayı çıkarın ya da yeniden üretin."),
                        at,
                    ));
                }
                if let Some((b, a)) = now
                    .baselines
                    .iter()
                    .filter(|b| b.campaign == *id)
                    .find_map(|b| now.adjustments.iter().find(|a| a.baselines.contains(&b.id)).map(|a| (b.id.as_str(), a.id.as_str())))
                {
                    return Err(fail(
                        codes::SURVEY_IN_USE,
                        format!("“{id}” kaydının “{b}” bazı “{a}” ağ dengelemesinde; önce dengelemeyi çıkarın."),
                        at,
                    ));
                }
                let before = now.sessions.len() + now.solutions.len() + now.campaigns.len() + now.baselines.len();
                next.solutions.retain(|s| !sessions.contains(&s.session.as_str()));
                next.sessions.retain(|s| s.campaign != *id);
                next.baselines.retain(|b| b.campaign != *id);
                next.campaigns.retain(|c| c.id != *id);
                removed =
                    before - (next.sessions.len() + next.solutions.len() + next.campaigns.len() + next.baselines.len());
            } else if now.session(id).is_some() {
                if let Some((s, p)) = used_by(&now, &[id.as_str()]) {
                    return Err(fail(
                        codes::SURVEY_IN_USE,
                        format!("“{id}” kaydının “{s}” çözümü “{p}” noktasında; önce noktayı çıkarın ya da yeniden üretin."),
                        at,
                    ));
                }
                let before = now.sessions.len() + now.solutions.len();
                next.solutions.retain(|s| s.session != *id);
                next.sessions.retain(|s| s.id != *id);
                removed = before - (next.sessions.len() + next.solutions.len());
            } else if now.solution(id).is_some() {
                if let Some(p) = now.point_using(id) {
                    return Err(fail(
                        codes::SURVEY_IN_USE,
                        format!("“{id}” çözümü “{}” noktasında; önce noktayı çıkarın ya da yeniden üretin.", p.id),
                        at,
                    ));
                }
                next.solutions.retain(|s| s.id != *id);
                removed = 1;
            } else if now.point(id).is_some() {
                if let Some(u) = now.uses.iter().find(|u| u.points.iter().any(|p| p.point == *id)) {
                    return Err(fail(
                        codes::SURVEY_IN_USE,
                        format!("“{id}” noktasına “{}” dayanan kaydı dayanıyor; önce onu çıkarın.", u.id),
                        at,
                    ));
                }
                next.points.retain(|p| p.id != *id);
                removed = 1;
            } else if now.uses.iter().any(|u| u.id == *id) {
                next.uses.retain(|u| u.id != *id);
                removed = 1;
            } else if now.baseline(id).is_some() || now.known_point(id).is_some() {
                if let Some(a) = now.adjustments.iter().find(|a| a.baselines.contains(id) || a.known.contains(id)) {
                    return Err(fail(
                        codes::SURVEY_IN_USE,
                        format!("“{id}” kaydı “{}” ağ dengelemesinde; önce dengelemeyi çıkarın ya da kaydı dışlayın.", a.id),
                        at,
                    ));
                }
                next.baselines.retain(|b| b.id != *id);
                next.known.retain(|k| k.id != *id);
                removed = 1;
            } else if now.adjustment(id).is_some() {
                if let Some(p) = now.point_by(id) {
                    return Err(fail(
                        codes::SURVEY_IN_USE,
                        format!("“{}” noktası “{id}” ağ dengelemesiyle üretildi; önce noktayı yeniden üretin ya da çıkarın.", p.id),
                        at,
                    ));
                }
                next.adjustments.retain(|a| a.id != *id);
                removed = 1;
            } else {
                return Err(unknown("kayıt", id, at));
            }
            Ok(Change {
                id: Some(id.clone()),
                removed: u32::try_from(removed).unwrap_or(u32::MAX),
                ..Change::new(next, "Arşivden çıkar")
            })
        }
    }
}

/// The map's work item a campaign names: a campaign item of the project.
fn work_ok(doc: &Document, work: Option<&str>) -> Result<(), Stop> {
    let Some(w) = work else { return Ok(()) };
    let found = doc.map_work().and_then(|mw| mw.item(w)).is_some_and(|i| i.kind == kentos_commands::WorkKind::Campaign);
    if !found {
        return Err(fail(
            codes::UNKNOWN_WORK_ITEM,
            format!("Harita işinde “{w}” kimlikli arazi ölçümü öğesi yok."),
            "operation.work",
        ));
    }
    Ok(())
}

fn output(doc: &Document, c: &Change, uid: Option<String>, revision: String) -> MapSurveyEdited {
    MapSurveyEdited {
        id: c.id.clone(),
        uid,
        point_revision: c.point_revision,
        observations: c.observations,
        removed: c.removed,
        ellipses: c.drawn.iter().filter(|e| matches!(e, Entity::Ellipse(_))).count() as u32,
        bars: c.drawn.iter().filter(|e| matches!(e, Entity::Polyline(_))).count() as u32,
        stale: stale_count(doc, &c.survey),
        records: c.records.clone(),
        revision,
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: MapSurveyEdit) -> CommandResult<MapSurveyEdited> {
    let mut change = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let label = change.label;
    let adds = std::mem::take(&mut change.adds);
    let updates = std::mem::take(&mut change.updates);
    let drawn = change.drawn.clone();
    let drawn_use = change.drawn_use;
    let single = adds.len() + updates.len() == 1;
    let mut survey = change.survey.clone();
    let written = cx.doc.transact(label, |doc| -> Result<Option<String>, Stop> {
        let mut uid = None;
        for (entity, at) in adds {
            let (_, text, _) = checks::add(doc, entity)?;
            let id = Uuid::parse_str(&text).map(|u| EntityId(*u.as_bytes())).unwrap_or(EntityId([0; 16]));
            if let Some(p) = survey.points.get_mut(at) {
                p.uid = id;
            }
            uid = Some(text);
        }
        for (slot, entity) in updates {
            uid = doc.uid(slot).map(|u| u.to_string());
            doc.update(slot, entity);
        }
        // The error ellipses after the points, their ids in the use that records them (docs/adr/2960).
        for entity in drawn {
            let (_, text, _) = checks::add(doc, entity)?;
            let id = Uuid::parse_str(&text).map(|u| EntityId(*u.as_bytes())).unwrap_or(EntityId([0; 16]));
            if let Some(u) = drawn_use.and_then(|k| survey.uses.get_mut(k)) {
                u.uids.push(id);
            }
        }
        doc.set_map_survey(Some(survey.clone()), label);
        Ok(uid.filter(|_| single))
    });
    let uid = match written {
        Ok(uid) => uid,
        Err(stop) => return stop.into(),
    };
    change.survey = survey;
    let output = output(cx.doc, &change, uid, cx.doc.revision().to_string());
    CommandResult::Completed { output, warnings: change.warnings }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &MapSurveyEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &MapSurveyEdit) -> CommandResult<MapSurveyEdited> {
    match check(cx.doc, input) {
        Ok(c) => {
            let uid = match (c.adds.len(), c.updates.as_slice()) {
                (0, [(slot, _)]) => cx.doc.uid(*slot).map(|u| u.to_string()),
                _ => None,
            };
            let output = output(cx.doc, &c, uid, cx.doc.revision().to_string());
            CommandResult::Completed { output, warnings: c.warnings }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: MapSurveyEdit) -> CommandResult<MapSurveyEdited> {
    crate::call::recorded(cx, kentos_commands::MAP_SURVEY_EDIT, kentos_commands::MAP_SURVEY_EDIT_VERSION, input, write)
}
