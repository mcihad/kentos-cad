//! The catalog's capabilities are true (docs/adr/0253 §7): every code a
//! shared case answers with is listed for its command, every listed code a
//! case reaches or is named here with the reason no case does; and for the
//! commands that pick objects by kind, an object of each kind is tried: a
//! listed kind is taken, another is refused with the listed code.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use kentos_commands::capabilities::{KINDS, PROTOCOL_CODES, capabilities_of};
use kentos_commands::{CallOp, DocumentSnapshotV2, FILE_COMMANDS};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{DESKTOP_COMMANDS, ExecutionContext, registry};
use serde_json::{Value, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// Every code the shared cases answer with, by command.
fn reached() -> BTreeMap<String, BTreeSet<String>> {
    fn walk(v: &Value, out: &mut BTreeSet<String>) {
        match v {
            Value::Object(map) => {
                if map.contains_key("status")
                    && let Some(code) = map
                        .get("error")
                        .and_then(|e| e.get("code"))
                        .and_then(Value::as_str)
                {
                    out.insert(code.to_owned());
                }
                if let Some(Value::Array(ws)) = map.get("warnings") {
                    for w in ws {
                        if let Some(code) = w.get("code").and_then(Value::as_str) {
                            out.insert(code.to_owned());
                        }
                    }
                }
                map.values().for_each(|x| walk(x, out));
            }
            Value::Array(items) => items.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for dir in ["fixtures/commands/v1", "fixtures/commands/v1/desktop"] {
        let mut files: Vec<PathBuf> = std::fs::read_dir(root().join(dir))
            .expect("the folder")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for file in files {
            let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).expect("reads"))
                .expect("json");
            let command = v["command"].as_str().expect("command").to_owned();
            // A step that runs another command (its `command`) reaches that command's codes, the ones its catalog
            // lists; a code it does not list is the file's command's (a hatch's cutout gone by a delete step).
            for case in v["cases"].as_array().into_iter().flatten() {
                match case["steps"].as_array() {
                    Some(steps) => {
                        for step in steps {
                            let Some(id) = step["command"].as_str().filter(|id| *id != command)
                            else {
                                walk(step, out.entry(command.clone()).or_default());
                                continue;
                            };
                            let listed: BTreeSet<String> = capabilities_of(id)
                                .map(|c| c.refusals.into_iter().chain(c.warnings).collect())
                                .unwrap_or_default();
                            let mut got = BTreeSet::new();
                            walk(step, &mut got);
                            for code in got {
                                let owner = if listed.contains(&code) { id } else { &command };
                                out.entry(owner.to_owned()).or_default().insert(code);
                            }
                        }
                    }
                    None => walk(case, out.entry(command.clone()).or_default()),
                }
            }
        }
    }
    out
}

/// The listed codes no shared case reaches, and why.
const UNREACHED: &[(&str, &str, &str)] = &[
    ("cad.survey.baselines", "network_too_large", "1500 bilinmeyenin (500 istasyon) üstü bir ortak durumda taşınmaz; sınır geometry-core'un gnss::adjust'ında MAX_UNKNOWNS ile denetlenir (ADR 1380 §Bütçe)."),
    ("map.survey.network", "network_too_large", "1500 bilinmeyenin (500 işaret) üstü bir ortak durumda taşınmaz; arşivin ağı aynı çekirdekten (gnss::adjust, MAX_UNKNOWNS) geçer (ADR 1380 §Bütçe, ADR 1740)."),
    ("map.survey.edit", "network_too_large", "1500 bilinmeyenin (500 işaret) üstü bir ortak durumda taşınmaz; ağın kabulü map.survey.network'ün çözümlemesinden geçer (ADR 1740)."),
    ("map.survey.network", "invalid_covariance", "Arşivin kuralı (map_survey_problem) pozitif tanımlı olmayan baz kovaryansını yazdırmaz; varyans çarpanı 0'dan büyük olduğundan ölçeklenmiş kovaryans, σ'ları 0'dan büyük olduğundan bilinen noktanınki de pozitif tanımlı kalır (ADR 1740)."),
    ("map.survey.edit", "invalid_covariance", "Ağın çekirdeğe giden kovaryansları arşivin kuralıyla hep pozitif tanımlıdır (map.survey.network'ün gerekçesi); bazın kendi kovaryans reddi invalid_survey_value'dur (ADR 1740)."),
    ("map.features.reconcile", "invalid_profile", "Okunamayan proje profili ortak profil okuyucusundan (profiles::profile_only) gelir; cad.profiles.check'in ve .apply'ın ortak durumlarında sınanır (ADR 1760)."),
    ("map.features.code", "invalid_profile", "Okunamayan proje profili ortak profil okuyucusundan (profiles::profile_only) gelir; cad.profiles.check'in ve .apply'ın ortak durumlarında sınanır."),
    ("cad.standards.install", "no_standard", "Bağlı standart olmayan çizim review_take::installing_needs_a_readable_bound_standard testinde sınanır."),
    ("cad.standards.install", "invalid_standard", "Okunamayan bağlı standart review_take::installing_needs_a_readable_bound_standard testinde sınanır."),
    ("cad.standards.palettes", "invalid_standard", "cad.settings.set okunmayan paketi bağlamaz; dosyada kalmış okunmayan paketin reddi review_take::palettes_need_a_readable_bound_standard testinde sınanır."),
    ("cad.standards.install", "block_refused", "KentOS kitaplıklarının blokları kurallara uyar; kurallara uymayan bir proje kitaplığı office_profiles::a_library_block_that_breaks_the_block_rules_is_refused testinde sınanır."),
    ("cad.linetypes.edit", "pattern_too_long_for_lin", "12 öğeyi aşan desenin LIN dışa aktarımı review_take::a_long_pattern_is_kept_and_left_out_of_a_lin_file testinde sınanır."),
    ("cad.drawing.take", "unknown_block", "Tanımı kaybolmuş blok yerleştirmesi review_take::an_insert_whose_definition_the_drawing_lost_is_not_taken testinde sınanır."),
    ("cad.drawing.take", "block_definition_differs", "Tanımı değişmiş bloğun yerleştirmesini alma review_take::a_changed_definition_makes_the_insert_note_stale_and_the_take_warns testinde sınanır."),
    ("cad.review.notes.edit", "too_many_review_replies", "200 yanıt sınırı review_take::replies_and_notes_stop_at_their_limits testinde sınanır."),
    ("cad.review.notes.edit", "too_many_review_notes", "10 000 not sınırı review_take::replies_and_notes_stop_at_their_limits testinde sınanır."),
    // Baskı stili tablosu (docs/adr/1130).
    ("cad.plotstyles.import", "plot_style_file_too_large", "4 MiB'tan büyük bir tablo ortak durum olarak repoya konmaz; sınır plot_style_files_beyond_the_limit_and_unreadable_files_are_refused testinde sınanır."),
    ("cad.plotstyles.import", "book_unreadable", "Ortak kurulum geçersiz kitap vermez; kitap cad.sheets.details'in aynı read_project_sheets yolundan okunur ve orada korunur (ADR 1090)."),
    ("cad.plotstyles.import", "file_unreadable", "Okuma izni olmayan dosya ortak durumda kurulamaz (repo izinleri taşımaz); plot_style_files_beyond_the_limit_and_unreadable_files_are_refused testinde sınanır."),
    // Veri çıkarımı (docs/adr/1030).
    ("cad.extract.run", "extraction_too_large", "Bir milyon oluşum sınırı ortak durumda yazılamayacak kadar büyük bir çizim ister; sınır kentos_formats::extract::Walk::counted'te tek yerdedir ve çıkarımı kesmeden reddeder."),
    ("cad.extract.table", "extraction_too_large", "cad.extract.run ile aynı sınır ve aynı kod; tablo komutu aynı çıkarımı çağırır."),
    ("cad.extract.run", "extraction_too_deep", "Belgeler blok iç içeliğini 16 düzeyde keser (kentos_contracts::blocks); bu uyarı yalnız bu kurala uymayan bir dosyaya karşı savunmadır."),
    ("cad.extract.table", "extraction_too_deep", "cad.extract.run ile aynı savunma uyarısı."),
    ("cad.tables.edit", "extraction_too_deep", "cad.extract.run ile aynı savunma uyarısı; yenileme aynı çıkarımı çağırır."),
    ("cad.extract.table", "xref_unloaded", "Yüklü olmayan dış referans uyarısı cad.extract.run.json'da verilir; tablo komutu aynı çıkarımın uyarılarını aynen aktarır."),
    ("cad.tables.edit", "xref_unloaded", "Yüklü olmayan dış referans uyarısı cad.extract.run.json'da verilir; yenileme aynı çıkarımın uyarılarını aynen aktarır."),
    ("cad.tables.edit", "dimension_overrides", "Elle yazılmış ölçü uyarısı cad.extract.run.json'un metraj durumlarında ve cad.extract.table.json'da verilir; yenileme aynı çıkarımın uyarılarını aktarır."),
    ("road.volumes.query", "volume_curve_past_centre", "Kesitin kurbun merkezini geçmesi için iç yanının yarıçaptan (ortak çizimde 150 m) uzun olması gerekir; ortak çizimin arazisi buna yetmez. Kural kentos_road::volume::effective'tedir ve tests/all/curve_volumes.rs'de (merkezi geçen kesit durumu) sınanır; kübaj aynı fonksiyonu kullanır."),
    ("cad.standards.resolve", "invalid_standard", "Standart paketinin geçersizliği standards_cmd::project_standards_resolve_by_order testinde sınanır."),
    ("cad.standards.resolve", "no_standard", "Boş belge listesi standards_cmd::project_standards_resolve_by_order testinde sınanır."),
    ("cad.standards.conflicts", "invalid_standard", "Okunamayan bağlı belge standards_cmd::project_standard_conflicts_report_sources_and_fields_without_mutating testinde sınanır."),
    ("cad.entities.delete", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.transform", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.edit", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.hatches.edit", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.trim", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.extend", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.fillet", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.chamfer", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.stretch", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.break", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.join", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.entities.lengthen", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.changeset.apply", "hatch_association_lost", "Ortak işlem raporu tarama bağını kaybettiğinde uyarır; kaynak silme ve geri alma drawing_inspect::deleted_source_warns_and_undo_restores_association testinde ve arayüzde sınanır."),
    ("cad.drawing.inspect", "inspection_scope_limit", "drawing_inspect::caps_and_stale_revision_are_explicit_and_read_only testinde sınır ve revizyon reddi veya kısmi sonuç doğrulanır; ortak durum dosyası küçük girdiler kullanır."),
    ("cad.drawing.inspect", "inspection_truncated", "drawing_inspect::caps_and_stale_revision_are_explicit_and_read_only testinde sınır ve revizyon reddi veya kısmi sonuç doğrulanır; ortak durum dosyası küçük girdiler kullanır."),
    ("cad.drawing.inspect", "invalid_revision", "drawing_inspect::caps_and_stale_revision_are_explicit_and_read_only testinde sınır ve revizyon reddi veya kısmi sonuç doğrulanır; ortak durum dosyası küçük girdiler kullanır."),
    ("cad.drawing.inspect", "revision_conflict", "drawing_inspect::caps_and_stale_revision_are_explicit_and_read_only testinde sınır ve revizyon reddi veya kısmi sonuç doğrulanır; ortak durum dosyası küçük girdiler kullanır."),
    ("road.junctions.overlap","invalid_junction","Kavşak road_junctions::read ile okunur; okunamayan kavşak (kollar kesişmiyor, paralel) road.junctions.query'nin durumlarında aynı kodla sınanır (ADR 0910 §2)."),
    ("road.junctions.clearance","profile_curves_overlap","Kolların boykesitleri road.junctions.tie ile aynı yoldan (road_junctions::leg_profiles) kurulur; kurpları çakışan boykesit orada sınanır (ADR 0918 §3)."),
    ("road.terrains.coverage","terrain_unbuildable","Yüzey road_terrains::make ile kurulur; kurulamayan yüzey (aynı yerde farklı kotlar, kesişen kırık hatlar) road.terrains.query ve .create'in durumlarında aynı kodla sınanır (ADR 0900 §11)."),
    ("road.terrains.lowconfidence","terrain_unbuildable","Yüzey road_terrains::make ile kurulur; kurulamayan yüzey road.terrains.query ve .create'in durumlarında aynı kodla sınanır (ADR 0900 §11)."),
    ("road.corridors.water","corridor_folded","Komut koridorun kendi uyarılarını road_corridors::make ile aynen geçirir; katlanan şerit road.corridors.query'nin dar kurp durumunda sınanır (ADR 0905)."),
    ("road.terrains.density","terrain_unbuildable","Yüzey road_terrains::make ile kurulur; kurulamayan yüzey road.terrains.query ve .create'in durumlarında aynı kodla sınanır (ADR 0900 §11)."),
    ("road.terrains.sparse","terrain_unbuildable","Yüzey road_terrains::make ile kurulur; kurulamayan yüzey road.terrains.query ve .create'in durumlarında aynı kodla sınanır (ADR 0900 §11)."),
    ("road.terrains.repair","terrain_unbuildable","Bırakılacak yer yüzeyin bir nokta kaydı olmalı; kaydı olan ama kurulamayan yüzey (aynı yerde farklı kotlar, kesişen kırık hatlar) road.terrains.query ve .create'in durumlarında sınanır (ADR 0900 §11); onarım onu aynı kodla reddeder (ADR 0916 §6)."),
    ("road.drainage.query","invalid_culvert","Menfezin girişi koridorundan okunur; koridoru okunamayan menfezin reddi road.culverts.query'nin durumlarında sınanır (ADR 0909 §2)."),
    ("cad.pointclouds.register","too_many_cloud_points","Bir bulut en çok 400 000 voksel noktası verir (REGISTER_MOST_POINTS) ve bir çağrı en çok 200 milyon nokta okur; ortak durumun taramaları 2 400 voksel noktası verir, okuma sınırı cad.pointclouds.region'ın durumlarında sınanır (ADR 0552, 1800)."),
    ("cad.pointclouds.overlap","too_many_cloud_points","Bir çağrı en çok 200 milyon nokta okur (POINTCLOUD_MOST_COUNTED); ortak durumun dört taraması bu sınırın çok altında, sınır cad.pointclouds.region'ın durumlarında sınanır (ADR 0552, 1500)."),
    ("cad.pointclouds.register","cloud_source_unreadable","Taramalar betiğin yazdığı metin dosyalarıdır, her durumda okunur; okunamayan kaynağın uyarısı pointcloud_areas::Scan'ındır ve cad.pointclouds.region'ın durumlarında sınanır (ADR 0552, 1800)."),
    ("cad.pointclouds.overlap","cloud_source_unreadable","Taramalar betiğin yazdığı metin dosyalarıdır, her durumda okunur; okunamayan kaynağın uyarısı pointcloud_areas::Scan'ındır ve cad.pointclouds.region'ın durumlarında sınanır (ADR 0552, 1500)."),
    ("road.terrains.cloud","too_many_cloud_points","Bir çağrı en çok 200 milyon nokta okur (POINTCLOUD_MOST_COUNTED); ortak durumun bulutu bu sınırın çok altında, sınır cad.pointclouds.region'ın durumlarında sınanır (ADR 0552)."),
    ("map.survey.edit","survey_full","GNSS arşivi en çok 1000 kampanya, 10 000 oturum, 100 000 çözüm, 10 000 nokta ve 10 000 dayanan tutar; sınır sözleşmenin map_survey_problem kuralındadır, o kadar kayıtlı çizim ortak durumda taşınmaz (ADR 1340 §2)."),
    ("map.survey.edit","geoid_missing","Projenin kotları ortometrikse kabul noktası projenin geoit modeliyle taşınır; model bu cihazda yoksa zincirin reddi aynen döner ve cad.crs.transform'un durumlarında sınanır (ADR 0631 §1)."),
    ("map.survey.edit","no_system","Koordinat sistemi olmayan (yerel) harita projesine nokta zincirle taşınamaz; zincirin reddi aynen döner ve cad.crs.transform'un durumlarında sınanır (ADR 1230 §1)."),
    ("map.survey.edit","velocity_missing","Kampanyanın hız ızgarası bu cihazın kitaplığında yoksa zincirin reddi aynen döner; ret cad.crs.transform'un durumlarında sınanır (ADR 0631 §3)."),
    ("map.work.edit","work_full","Harita işi en çok 1000 öğe ve 10 000 türetme kaydı tutar; sınır sözleşmenin map_work_problem kuralındadır; 1000 öğelik çizim ortak durumda taşınmaz (ADR 1200 §1)."),
    ("road.project.rebuild","road_drawings_full","Yeniden üretme kaydı yerinde yazar, sayıyı artırmaz; sınır sözleşmenin a_drawing_record_is_judged testinde (ADR 0911 §3)."),
    ("road.alignments.label","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.profiles.draw","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.sections.draw","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.profiles.draw","invalid_road_corridor","Şev ve platform satırları koridoru road_corridors::make ile kurar; bu ret koridor kurulamayınca gelir ve road.corridors.draw/query durumlarında sınanır (ADR 0989)."),
    ("road.profiles.draw","no_terrain","Şev ve platform satırları koridoru road_corridors::make ile kurar; bu ret koridor kurulamayınca gelir ve road.corridors.draw/query durumlarında sınanır (ADR 0989)."),
    ("road.sheets.plan","unknown_corridor","Pafta, şev ve platform satırlarını road.profiles.draw'ın denetiminden geçirir; bu ret onun corridor durumlarında sınanır (ADR 0989)."),
    ("road.sheets.plan","invalid_label","Pafta, şev ve platform satırlarını road.profiles.draw'ın denetiminden geçirir; bu ret onun corridor durumlarında sınanır (ADR 0989)."),
    ("road.sheets.plan","invalid_road_corridor","Pafta, şev ve platform satırlarını road.profiles.draw'ın denetiminden geçirir; bu ret onun corridor durumlarında sınanır (ADR 0989)."),
    ("road.sheets.plan","no_terrain","Pafta, şev ve platform satırlarını road.profiles.draw'ın denetiminden geçirir; bu ret onun corridor durumlarında sınanır (ADR 0989)."),
    ("road.profiles.draw","invalid_haul","Hafriyat satırları Brükneri road_hauls::read ve kübajı road_volumes::compute_on ile okur; bu ret onlar okunamayınca gelir ve road.hauls.query durumlarında sınanır (ADR 0995)."),
    ("road.profiles.draw","unknown_volume","Hafriyat satırları Brükneri road_hauls::read ve kübajı road_volumes::compute_on ile okur; bu ret onlar okunamayınca gelir ve road.hauls.query durumlarında sınanır (ADR 0995)."),
    ("road.sheets.plan","unknown_haul","Pafta, hafriyat satırlarını road.profiles.draw'ın denetiminden geçirir; bu ret onun haul durumlarında ya da road.hauls.query'de sınanır (ADR 0995)."),
    ("road.sheets.plan","invalid_haul","Pafta, hafriyat satırlarını road.profiles.draw'ın denetiminden geçirir; bu ret onun haul durumlarında ya da road.hauls.query'de sınanır (ADR 0995)."),
    ("road.sheets.plan","unknown_volume","Pafta, hafriyat satırlarını road.profiles.draw'ın denetiminden geçirir; bu ret onun haul durumlarında ya da road.hauls.query'de sınanır (ADR 0995)."),
    ("road.corridors.draw","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.guardrails.draw","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.culverts.draw","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.walls.draw","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.platforms.draw","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.platforms.draw","invalid_platform","Çizimin denetimi platformu okuyarak yapar; okunamayan platform (kurulamayan yüzey, tesviye edilemeyen sınır) road.platforms.query'nin durumlarında, sınır retleri road.platforms.edit'te sınanır; tanımı geçerli bir platform yazıldıktan sonra yüzeyin bozulması ayrı bir sahne ister."),
    ("road.junctions.draw","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.terrains.contours","road_drawings_full","Proje en çok 1000 yol çizimi tutar; sınır sözleşmenin a_drawing_record_is_judged testinde, kaydı geri alan adım road_drawings birim testinde sınanır (ADR 0911 §1)."),
    ("road.stakeout.send", "stakeout_unsolved", "Ortak çizimde kesitsiz km yalnız yüzeysiz koridorda olur, o da hiç nokta vermez ve gönderme reddedilir; uyarının metni road.stakeout.query'nin durumlarında sınanır, gönderme cetvelin uyarılarını olduğu gibi taşır (ADR 0908 §9)."),
    ("cad.blocks.convert", "recognition_scope_limit", "Toplam 204000 köşe blocks_convert::aggregate_vertex_limit_is_inherited_from_the_same_recognition_gate_without_a_partial_write testinde oluşturulup ortak aramanın ret kodu, değişmez snapshot ve Undo yokluğu sınanır (ADR0871)."),
    ("cad.blocks.convert", "block_refused", "Yazmadan önce aynı tanım block_rules ile doğrulanır; add_block reddi yalnız savunmadır. Yeni tanım saf analitik üyelerden ve yeni UUID ile oluşur; yinelenen ad ortak durumda duplicate_block olur (ADR0871)."),
    ("cad.blocks.convert", "layer_not_found", "Kaynak ve aday nesneler geçerli belgeden okunur; kayıp katman bir dosya bütünlüğü hatasıdır. Dönüşüm katmanı istemciden ayrı almaz; ortak katman denetiminin savunma kodudur (ADR0871)."),
    ("cad.blocks.convert", "not_a_layer", "Geçerli belge nesnesi bir gruba çizilmez; dönüşüm katmanını kaynağından alır. Grup katman savunması ortak checks::layer yolundadır; kilitli katmanın gerçek reddi dönüşüm ortak durumunda sınanır (ADR0871)."),
    ("cad.blocks.find", "recognition_scope_limit", "50 001 nesnelik liste ortak JSON'a şişirilmez; blocks_find::limits_and_cancellation_keep_the_source_exact testinde üretilip ret ve değişmezlik sınanır (ADR 0866)"),
    ("cad.parameters.edit", "parameter_kind", "Birim boyutu ön denetimi parameter_dimensions.rs içinde validate/plan/execute eşitliği, bağımlı üs, eski pi, atomiklik ve Undo ile sınanır; ortak ADR0336 durumları eski sayısal grameri korur (ADR0840)."),
    ("cad.constraints.query", "parameter_units_legacy", "Eski pi ifadesi Number türünde olan V2 dosyası parameter_dimensions.rs içinde salt okunur uyarı, yeniden adlandırma, açık onarım ve Undo ile sınanır; yeni komutlar bu uyumsuz ifadeyi oluşturamaz (ADR0840)."),
    (
        "cad.entities.isolate", "invalid_input",
        "MAX_SELECT_ITEMS sınırından bir fazla UID, entities_select/isolation.rs içindeki isolation_limit_is_atomic testinde; büyük tekrarlı liste ortak JSON durumuna yazılmaz (ADR 0820).",
    ),
    ("cad.walls.edit", "constraint_conflict", "Bağların çelişkisi cad.constraints.edit.arch.json'un cad.walls.edit adımlarında (sabit duvarı kaydırmak, kilitli katmandaki bağlı duvar) verilir; o dosyanın komutu cad.constraints.edit olduğundan sayım onu orada bulur (ADR 0770 §4)."),
    ("cad.walls.edit", "constraints_relaxed", "Gevşeterek düzenleme cad.constraints.edit.arch.json'un cad.walls.edit adımında (relaxConstraints) verilir, uyarıları \"$any\" ile karşılaştırılır (ADR 0770 §4)."),
    ("cad.walls.edit", "constraints_removed", "Kenar sayısı değişen duvarın bağları ADR 0337'deki gibi gider; uyarı kentos-domain'in constraints testlerinde sınanır (ADR 0770 §4)."),
    ("cad.walls.edit", "invalid_constraint", "Mesafe'nin keep'inin bağı kuralına uymazsa invalid_constraint verilir; komutun kurduğu bağ hep uyduğu için yalnız savunmadır, aynı ret cad.constraints.edit.arch.json'un altı ret durumunda verilir (ADR 0770 §5)."),
    ("cad.openings.edit", "opening_type_not_found", "Tipe döndür, Tipi yeniden uygula ve Tipe uygula açıklığın izlediği tipi projenin tablosunda arar; tip silinirken izleyenler ondan ayrıldığı (opening_type_in_use, detach) için komutlarla tipsiz izleyen kalmaz. Ret yalnız elle yazılmış bir dosyanın açıklığına karşı savunmadır; aynı arama cad.openingtypes.edit'in update ve cad.openings.query'nin type durumlarında verilir (ADR 0772)."),
    ("cad.program.check", "cancelled", "Durdurma işin kendisine sorulur; ortak durum iş başlatmaz. program_scenes.rs'in kabul testi check_watched'ı durdurup Cancelled cevabını sınar (ADR 0773 §5)."),
    ("cad.addresses.compare", "register_cut", "Uyarı 250 000 satırdan uzun kayıtta verilir; o boyda bir dosya ortak durumlara konmaz, addresses_compare.rs'in a_register_longer_than_is_read_says_so testi sınırdan uzun bir kayıtla sınar (ADR 0736)."),
    ("file.export.block", "dimension_set_owned", "Dosya yazımı ve mevcut dosyanın korunumu headless dimension_set_exports.rs içinde gerçek validate/plan/execute + açık detach sonrası byte doğrulamasıyla sınanır (ADR0695)."),
    ("cad.entities.copy", "dimension_set_owned", "Bu komutun ortak durum dosyası otomatik ölçü tarifi kurmaz; V2 tarifli dimension_set_copy.rs her validate/plan/execute yolunun yazmadan ret ve açık detach sonrasını sınar (ADR0695)."),
    ("cad.entities.array", "dimension_set_owned", "Bu komutun ortak durum dosyası otomatik ölçü tarifi kurmaz; V2 tarifli dimension_set_copy.rs her validate/plan/execute yolunun yazmadan ret ve açık detach sonrasını sınar (ADR0695)."),
    ("cad.entities.transform", "dimension_set_owned", "Bu komutun ortak durum dosyası otomatik ölçü tarifi kurmaz; V2 tarifli dimension_set_copy.rs her validate/plan/execute yolunun yazmadan ret ve açık detach sonrasını sınar (ADR0695)."),
    ("cad.blocks.define", "dimension_set_owned", "Bu komutun ortak durum dosyası otomatik ölçü tarifi kurmaz; V2 tarifli dimension_set_copy.rs her validate/plan/execute yolunun yazmadan ret ve açık detach sonrasını sınar (ADR0695)."),

    ("cad.slabs.create", "invalid_attribute", "Katmanın alanlarına göre öznitelik her oluşturan komutun ortak denetimidir (checks::object_attrs); uyumsuz alan cad.polygon.create ve cad.entities.create ortak durumlarında sınanır (ADR 0199 §2, ADR 0722)."),
    ("cad.slabs.create", "attribute_required", "Katmanın zorunlu alanı her oluşturan komutun ortak denetimidir (checks::object_attrs); cad.polygon.create ortak durumlarında sınanır (ADR 0199 §2, ADR 0722)."),
    ("cad.spaces.separators","invalid_attribute","Oluşturma mevcut polyline::plan_in/checks::object_attrs yolunu kullanır; alan doğrulaması cad.polyline.create ve cad.polygon.create bağımsız durumlarında sınanır (ADR 0679)."),
    ("cad.spaces.separators","attribute_required","Zorunlu alan denetimi değiştirilmeden mevcut polyline::plan_in/checks::object_attrs yoluna gider (ADR 0679)."),
    ("cad.spaces.separators","slots_exhausted","Document::add_many bütün toplu yazmayı reddeder; son slot ve atomiklik domain the_last_slot_is_given_and_then_adding_is_refused_whole testinde sınanır (ADR 0679)."),
    ("cad.spaces.discover", "space_not_enclosed", "Bulunan seed aynı değişmez duvar ağı üzerinde mevcut spaces_create yazıcısıyla yeniden denetlenir. Eşzamanlı geometri değişimi mümkün değildir; bu savunma korunur, asıl kapanmama yolu cad.spaces.create ortak durumlarında sınanır (ADR 0671)."),
    ("cad.walls.partition", "attribute_required", "Bölmenin öznitelikleri değişmeden walls_create::plan_in/execute ve checks::object_attrs yoluna gider; zorunlu/uyumsuz alanlar cad.walls.create ve cad.polygon.create ortak başvurularında sınanır (ARCH-DRAW-02)."),
    ("cad.walls.partition", "invalid_attribute", "Bölmenin öznitelikleri değişmeden walls_create::plan_in/execute ve checks::object_attrs yoluna gider; zorunlu/uyumsuz alanlar cad.walls.create ve cad.polygon.create ortak başvurularında sınanır (ARCH-DRAW-02)."),
    ("cad.walls.partition", "coincident_points", "Yalnız iki noktalı düz yol ve eksen hizası üretilir; yolun kaynakta çakışması çekirdekte invalid_wall olur. Kalan çok küçük/büyük sayı ve yüz ötelemesi savunması mevcut walls::build/drawable yolundadır; onun ortak duvar başvuruları korunur (ARCH-DRAW-02)."),
    ("cad.walls.partition", "wall_collapses", "Yalnız iki noktalı düz yol ve eksen hizası üretilir; yolun kaynakta çakışması çekirdekte invalid_wall olur. Kalan çok küçük/büyük sayı ve yüz ötelemesi savunması mevcut walls::build/drawable yolundadır; onun ortak duvar başvuruları korunur (ARCH-DRAW-02)."),
    ("cad.walls.partition", "wall_join_acute", "Yeni duvar mevcut walls_create planı ve yazmasıyla eklenir; mevcut walls::join_warnings davranışı cad.walls.create ortak birleşim başvurularında sınanır. Bölmenin seçilmiş iki sınırdaki T ve mahal/kapı korunumu ayrı native kabulündedir (ARCH-DRAW-02)."),
    ("cad.walls.partition", "wall_join_crowded", "Yeni duvar mevcut walls_create planı ve yazmasıyla eklenir; mevcut walls::join_warnings davranışı cad.walls.create ortak birleşim başvurularında sınanır. Bölmenin seçilmiş iki sınırdaki T ve mahal/kapı korunumu ayrı native kabulündedir (ARCH-DRAW-02)."),
    ("cad.walls.partition", "wall_join_overlap", "Yeni duvar mevcut walls_create planı ve yazmasıyla eklenir; mevcut walls::join_warnings davranışı cad.walls.create ortak birleşim başvurularında sınanır. Bölmenin seçilmiş iki sınırdaki T ve mahal/kapı korunumu ayrı native kabulündedir (ARCH-DRAW-02)."),
    ("cad.walls.partition", "wall_join_unresolved", "Yeni duvar mevcut walls_create planı ve yazmasıyla eklenir; mevcut walls::join_warnings davranışı cad.walls.create ortak birleşim başvurularında sınanır. Bölmenin seçilmiş iki sınırdaki T ve mahal/kapı korunumu ayrı native kabulündedir (ARCH-DRAW-02)."),
    (
        "cad.walls.rectangle", "coincident_points",
        "Türetilmiş dört köşe mevcut walls::path_numbers ile denetlenir; çok büyük sonlu koordinatta yuvarlanıp aynı yere gelen köşeler bu miras ret yolundadır. Olağan dört çeyrek/uzak koordinat başvuruları farklı köşeler üretir (ARCH-DRAW-01).",
    ),
    (
        "cad.walls.rectangle", "wall_collapses",
        "Mevcut walls::drawable yüz ötelemesi güvenlik denetimi korunur. Dikdörtgende iç boşluğun kapanması yeni komutta invalid_wall/frame olur; kalan sayısal öteleme hataları ortak wall_collapses yolunu korur (ARCH-DRAW-01).",
    ),
    (
        "cad.walls.rectangle", "invalid_attribute",
        "walls_create::plan_in ve execute aynı checks::object_attrs yolunu çalıştırır; alan doğrulaması cad.walls.create ve cad.polygon.create ortak başvurularında sınanır (ARCH-DRAW-01).",
    ),
    (
        "cad.walls.rectangle", "attribute_required",
        "walls_create::plan_in ve execute aynı checks::object_attrs yolunu çalıştırır; zorunlu alanlar cad.walls.create ve cad.polygon.create ortak başvurularında sınanır (ARCH-DRAW-01).",
    ),
    (
        "cad.walls.rectangle", "wall_join_overlap",
        "Yeni kapalı duvar mevcut walls_create::plan_in ve execute ile üretilir; başka duvarlarla birleşim uyarısı ortak walls::join_warnings yoludur ve cad.walls.create başvurularında sınanır (ADR 0340, ARCH-DRAW-01).",
    ),
    (
        "cad.walls.rectangle", "wall_join_crowded",
        "Yeni kapalı duvar mevcut walls_create::plan_in ve execute ile üretilir; başka duvarlarla birleşim uyarısı ortak walls::join_warnings yoludur ve cad.walls.create başvurularında sınanır (ADR 0340, ARCH-DRAW-01).",
    ),
    (
        "cad.walls.rectangle", "wall_join_acute",
        "Yeni kapalı duvar mevcut walls_create::plan_in ve execute ile üretilir; başka duvarlarla birleşim uyarısı ortak walls::join_warnings yoludur ve cad.walls.create başvurularında sınanır (ADR 0340, ARCH-DRAW-01).",
    ),
    (
        "cad.walls.rectangle", "wall_join_unresolved",
        "Yeni kapalı duvar mevcut walls_create::plan_in ve execute ile üretilir; başka duvarlarla birleşim uyarısı ortak walls::join_warnings yoludur ve cad.walls.create başvurularında sınanır (ADR 0340, ARCH-DRAW-01).",
    ),
    (
        "cad.addresses.normalize",
        "layer_hidden",
        "uyarı kapı katmanının ortak denetiminindir (checks::layer_at), cad.circle.create'in ve cad.arc.create'in durumlarında; adres durumlarının katmanları görünür (ADR 0647 §2)",
    ),
    (
        "cad.sensors.ingest",
        "source_unreachable",
        "adres ağ ister, ortak durumların koşucusu ağa çıkmaz; python/tests/test_sensors.py yerel bir SensorThings sunucusuyla sınar (ADR 0646 §4)",
    ),
    (
        "cad.sensors.ingest",
        "layer_hidden",
        "uyarı yazılan katmanın ortak denetiminindir (checks::layer_at), cad.circle.create'in ve cad.arc.create'in durumlarında; sensör durumlarının katmanları görünür (ADR 0646 §4)",
    ),
    (
        "cad.stac.prepare",
        "crs_unofficial",
        "ED50'de varlık veren bir STAC kataloğu yok; ED50'nin resmî olmayan yolu cad.rasters.reproject'in ED50 UTM 36N durumunda (ADR 0644 §3)",
    ),
    (
        "cad.gis.detach",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); bağın koptuğu dosya yerinde güncellenir, yazılamayınca reddedilir (ADR 0634 §2)",
    ),
    (
        "file.import.gpkg",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); reddedilenler dosyasının geçici adla yazılıp yerine konması ve yazılamayınca hiçbir şey bırakmaması import_merge.rs'in a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing testinde (ADR 0628 §6)",
    ),
    (
        "file.import.fgb",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); reddedilenler dosyasının geçici adla yazılıp yerine konması ve yazılamayınca hiçbir şey bırakmaması import_merge.rs'in a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing testinde (ADR 0628 §6)",
    ),
    (
        "file.import.csv",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); reddedilenler dosyasının geçici adla yazılıp yerine konması ve yazılamayınca hiçbir şey bırakmaması import_merge.rs'in a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing testinde (ADR 0628 §6)",
    ),
    (
        "file.import.gml",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); reddedilenler dosyasının geçici adla yazılıp yerine konması ve yazılamayınca hiçbir şey bırakmaması import_merge.rs'in a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing testinde (ADR 0628 §6)",
    ),
    (
        "file.import.parquet",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); reddedilenler dosyasının geçici adla yazılıp yerine konması ve yazılamayınca hiçbir şey bırakmaması import_merge.rs'in a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing testinde (ADR 0628 §6)",
    ),
    (
        "file.import.shp",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); reddedilenler dosyasının geçici adla yazılıp yerine konması ve yazılamayınca hiçbir şey bırakmaması import_merge.rs'in a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing testinde (ADR 0628 §6)",
    ),
    (
        "file.import.geojson",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); reddedilenler dosyasının geçici adla yazılıp yerine konması ve yazılamayınca hiçbir şey bırakmaması import_merge.rs'in a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing testinde (ADR 0628 §6)",
    ),
    (
        "file.import.kml",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (klasörü olmayan yol invalid_file_path ile önceden reddedilir); reddedilenler dosyasının geçici adla yazılıp yerine konması ve yazılamayınca hiçbir şey bırakmaması import_merge.rs'in a_rejects_file_that_cannot_be_written_refuses_and_leaves_nothing testinde (ADR 0628 §6)",
    ),
    (
        "file.import.fgb",
        "duplicate_key",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.fgb",
        "merge_key_not_found",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.gml",
        "duplicate_key",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.gml",
        "merge_key_not_found",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.shp",
        "duplicate_key",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.shp",
        "merge_key_not_found",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.kml",
        "duplicate_key",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.kml",
        "merge_key_not_found",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.shp",
        "merge_key_required",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.kml",
        "merge_key_required",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "file.import.kml",
        "rows_rejected",
        "birleştirme bütün dosya içe aktarmalarının tek yoludur (import_merge::merge; GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet file_import_gpkg::run_read'den, Shapefile, GeoJSON ve KML file_import_gis::write_checked'ten); kodları file.import.merge.json'da GeoPackage, GeoParquet, CSV ve GeoJSON'la verilir (ADR 0628 §5)",
    ),
    (
        "cad.polygon.regular",
        "area_clipped",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.polygon.regular",
        "area_covered",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.polygon.regular",
        "corners_joined",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.polygon.regular",
        "neighbours_locked",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.sector.create",
        "area_clipped",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.sector.create",
        "area_covered",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.sector.create",
        "corners_joined",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.sector.create",
        "neighbours_locked",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.revcloud.create",
        "area_clipped",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.revcloud.create",
        "area_covered",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.revcloud.create",
        "corners_joined",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.revcloud.create",
        "neighbours_locked",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); cad.polygon.create.area-rules.json'da Kapalı alan ve Dikdörtgen'le, dik köşeli alanların hücre başvurusuyla sınanır; eğri kenarlı şekil o başvuruya girmez (ADR 0622 §9)",
    ),
    (
        "cad.entities.adjoin",
        "area_covered",
        "bölge kaçınılan katmanın alanına bütünüyle düşmez: bölgeyi komşular kapatır; reddin kendisi cad.polygon.create.area-rules.json'da (ADR 0622 §9)",
    ),
    (
        "cad.entities.adjoin",
        "neighbours_locked",
        "kilitli komşu Topoloji'nin ortak yolundadır; cad.polygon.create.area-rules.json'da sınanır (ADR 0622 §9)",
    ),
    (
        "cad.entities.adjoin",
        "too_few_corners",
        "çekirdeğin bulduğu bölgenin en az üç köşesi vardır; ret cad.polygon.create'in denetimidir (ADR 0622 §9)",
    ),
    (
        "cad.entities.adjoin",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.entities.adjoin",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.processing.batch",
        "crs_mismatch",
        "toplu işin sonucu cad.processing.run'ın yazıcısıyla yazılır (batch_sink.rs → write_sink): sistemi başka dosya orada sınanır (ADR 0549 §1, 0593 §1)",
    ),
    (
        "cad.processing.batch",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz; yazıcı cad.processing.run'ınkiyle aynı (ADR 0593 §1)",
    ),
    (
        "cad.processing.batch",
        "input_too_large",
        "kaynaktan okumanın sınırı cad.processing.run'ınkiyle aynı stage'te, a_source_input_stops_at_its_cap testinde (ADR 0592 §1, 0593 §2)",
    ),
    (
        "cad.processing.batch",
        "export_losses",
        "kayıplar cad.processing.run'ın yazıcısının sözü (sink_answer); toplu iş aynı yazıcıdan geçer (ADR 0593 §1)",
    ),
    (
        "file.import.dwg",
        "polyline_geometry_loss",
        "DWG codec bridge ve plan/execute değişmezliği headless::dwg::polyline_integrity testinde gerçek binary ile sınanır; ortak bağımsız ASCII korpusu file.import.polyline-policy.json içindedir.",
    ),
    (
        "file.import.dxf",
        "cad_file_too_large",
        "256 MiB sınırı; bu boyutta veri ortak korpusa alınmaz, preflight sınırlar (CAD-IO-09/19)",
    ),
    (
        "file.import.dxf",
        "unsupported_cad_format",
        "yanlış DWG seçimi preflight'ın biçim testinde sınanır",
    ),
    (
        "file.import.dxf",
        "unsupported_cad_version",
        "bütün ilan edilmiş ASCII sürümleri ve gelecekteki bilinmeyen sürüm preflight testinde sınanır",
    ),
    (
        "file.import.dxf",
        "unsupported_cad_encoding",
        "binary, UTF-16 ve desteklenmeyen kod sayfası preflight testinde sınanır",
    ),
    (
        "file.import.dwg",
        "cad_file_too_large",
        "256 MiB sınırı adaptörde; bu boyutta dosya ortak korpusa alınmaz (ADR 0586)",
    ),
    (
        "file.import.dwg",
        "block_units_differ",
        "blok birimleri ortak DXF yolu; DXF ortak durumları ve adaptör korpusu sınar (ADR 0586)",
    ),
    (
        "file.import.dwg",
        "unit_assumed",
        "birimsiz R14 adaptör testi ve ortak DXF varsayılan birim testleri sınar (ADR 0586)",
    ),
    (
        "file.import.dwg",
        "import_losses",
        "atlanan kayıtlar codec raporundan gelir; ileri kaynaklar üretim korpusuyla ayrıca kabul edilecek (ADR 0586)",
    ),
    (
        "cad.sources.edit",
        "import_losses",
        "ADR 0629 §4'ten beri ortak dosyaların geniş tam sayısı, dilimli zamanı ve ikili değeri kayıpsız gelir; kalan kayıplar (1 MiB'ı aşan ikili, sonlu olmayan sayı) geopackage::tests::what_a_cell_cannot_keep_is_said ile sınanır",
    ),
    (
        "file.import.csv",
        "import_losses",
        "ADR 0629 §4'ten beri ortak dosyaların geniş tam sayısı, dilimli zamanı ve ikili değeri kayıpsız gelir; kalan kayıplar (1 MiB'ı aşan ikili, sonlu olmayan sayı) geopackage::tests::what_a_cell_cannot_keep_is_said ile sınanır",
    ),
    (
        "file.import.fgb",
        "import_losses",
        "ADR 0629 §4'ten beri ortak dosyaların geniş tam sayısı, dilimli zamanı ve ikili değeri kayıpsız gelir; kalan kayıplar (1 MiB'ı aşan ikili, sonlu olmayan sayı) geopackage::tests::what_a_cell_cannot_keep_is_said ile sınanır",
    ),
    (
        "cad.sheets.set",
        "book_unreadable",
        "çekirdeğin okuyamadığı pafta kitabını hiçbir komut yazmaz (cad.sheets.edit kitabı çekirdekle denetler); kurulum kitap vermez (ADR 0347 §1)",
    ),
    (
        "cad.crs.transform",
        "no_system",
        "sistemsiz yerel projede tarafın projeninki olması; durum dosyasının tek kurulumu TUREF / TM30 projesidir, ret crs_transform.rs'in side'ındadır; Python SDK'nın test_crs.py'si yerel projede sınar (ADR 0519 §3)",
    ),
    (
        "cad.revisions.mark",
        "invalid_attribute",
        "bulutun öznitelikleri (Revizyon, Tarih, Açıklama) katmanın alanlarıyla cad.revcloud.create'in yolundan (polygon::ruled, checks::object_attrs) denetlenir; cad.polygon.create'in durumlarında (ADR 0199 §2, 0721 §4)",
    ),
    (
        "cad.revisions.mark",
        "attribute_required",
        "katmanın zorunlu alanı bulutun yazıldığı cad.polygon.create yolunun ortak denetimidir (checks::object_attrs), onun durumlarında (ADR 0199 §2, 0721 §4)",
    ),
    (
        "cad.constructions.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.entities.create'in durumlarında; yapımlar onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.constructions.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.entities.create'in durumlarında; yapımlar onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.objects.derive",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.entities.create'in durumlarında; türetilen nesneler onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.survey.points",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.entities.create'in durumlarında; noktalar onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.survey.points",
        "invalid_attribute",
        "katmanın alanına uymayan değer: ortak denetim cad.entities.create'in durumlarında; noktanın yazdığı Ad, Tür ve Z (m) metindir (ADR 0070, 0622 §9)",
    ),
    (
        "cad.roads.edit",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.entities.create'in durumlarında; yollar onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.roads.edit",
        "invalid_attribute",
        "katmanın alanına uymayan değer: ortak denetim cad.entities.create'in durumlarında; yolun yazdığı Tür ve Genişlik metindir (ADR 0198, 0622 §9)",
    ),
    (
        "cad.roads.edit",
        "degenerate_shape",
        "çekirdek iki noktalı temiz eksenden ve iki kenardan her zaman halka kurar; girdiyle ulaşılmaz, savunma (ADR 0198, 0622 §9)",
    ),
    (
        "cad.annotations.write",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.entities.create'in durumlarında; açıklamalar onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.annotations.write",
        "invalid_attribute",
        "katmanın alanına uymayan değer: ortak denetim cad.entities.create'in durumlarında; açıklamanın yazdığı tek öznitelik Km'dir ve adı sabittir (ADR 0189, 0622 §9)",
    ),
    (
        "cad.objects.derive",
        "invalid_attribute",
        "katmanın alanına uymayan değer: ortak denetim cad.entities.create'in durumlarında; boş öznitelik adı da bu koddur ve durumda (ADR 0622 §9)",
    ),
    (
        "cad.shapes.edit",
        "degenerate_shape",
        "çekirdeğin sonucu sözleşmenin geometrisine çevrilemezse verilir; çekirdek yalnız çizgi, çoklu çizgi ve alan döndürür, girdiyle ulaşılmaz (ADR 0622 §9)",
    ),
    (
        "cad.parcels.create",
        "not_a_layer",
        "parselin katmanı sabittir (parsel): yeni projede ve aynı adımda açılan hep katmandır; parsel adında bir grup ancak elle kurulur, denetimi cad.polygon.create'in ortak yolundadır (checks::layer, ADR 0622 §9)",
    ),
    (
        "cad.parcels.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında; parsel onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.parcels.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında; parsel onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.parcels.create",
        "neighbours_locked",
        "Çakışma ve Topoloji her alan yazan komutun ortak yoludur (polygon::ruled_areas, area_rules.rs); kilitli komşu cad.polygon.create.area-rules.json'da sınanır (ADR 0622 §9)",
    ),
    (
        "cad.crs.transform",
        "no_datum_link",
        "projenin WGS 84'e yolu olmayan kendi datumu; kayıttaki sistemlerle kurulamaz, geometri-core'un datum testi (Gap::NoLink) sınar (ADR 0168 §2, 0519 §3)",
    ),
    (
        "cad.sector.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında; dilim ve bulut cad.polygon.create'in yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.sector.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında; dilim ve bulut onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.revcloud.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında; dilim ve bulut cad.polygon.create'in yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.revcloud.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında; dilim ve bulut onun yolundan yazar (ADR 0199 §2, 0622 §9)",
    ),
    (
        "cad.grids.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.grids.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.columns.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.columns.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.stairs.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.stairs.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.ramps.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.ramps.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.spaces.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.spaces.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.openings.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.openings.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.walls.create",
        "invalid_attribute",
        "katmanın alanlarına göre öznitelik: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in ve cad.entities.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.walls.create",
        "attribute_required",
        "katmanın zorunlu alanı: her oluşturan komutun ortak denetimi (checks::object_attrs), cad.polygon.create'in durumlarında (ADR 0199 §2)",
    ),
    (
        "cad.walls.edit",
        "wall_join_overlap",
        "birleşmeyen duvarların uyarısı her duvar komutunun ortak kuralıdır (walls::join_warnings); cad.walls.create'in durumlarında dördü de, cad.walls.edit'te dar köşe (ADR 0340 §2)",
    ),
    (
        "cad.walls.edit",
        "wall_join_crowded",
        "birleşmeyen duvarların uyarısı her duvar komutunun ortak kuralıdır (walls::join_warnings); cad.walls.create'in durumlarında (ADR 0340 §2)",
    ),
    (
        "cad.walls.edit",
        "wall_join_unresolved",
        "birleşmeyen duvarların uyarısı her duvar komutunun ortak kuralıdır (walls::join_warnings); cad.walls.create'in durumlarında (ADR 0340 §2)",
    ),
    (
        "cad.entities.delete",
        "relation_too_large",
        "on bin alt kayıt gerektirir; ortak durum dosyası için çok büyük, sınır delete.rs'in MOST_FOLLOWED'ıdır ve relation_effects onu sayar (ADR 0440 §1)",
    ),
    (
        "cad.entities.hatch",
        "hatch_text_cycle",
        "taramanın kendi bağlı etiketi: durum dosyasının kurulumu kalıcı kimlik taşımaz, yazı taramaya bağlanamaz; kentos-domain'in hatch_texts testinde (ADR 0332)",
    ),
    (
        "cad.entities.hatch",
        "invalid_region",
        "1000'den çok ek nokta: durum dosyasında bin noktalık bir girdi yazmaya değmez; ret sınırın kendisidir.",
    ),
    (
        "cad.entities.offset",
        "approximate_offset",
        "yaklaşık kopyanın köşe sayısı algoritmaya bağlıdır; offset_bounded testleri analitik paralelin uzaklığı ve hata bütçesini bağımsız denetler; offset::tests, typed Python ve MCP açık onayı/raporu/Undo'yu sınar (ADR 0822)",
    ),
    (
        "cad.entities.relate",
        "too_many_pairs",
        "bir milyondan çok çift için binlerce kimlik gerekir; durum dosyasını büyütmeye değmez, sınır entities_relate.rs'in MAX_RELATE_PAIRS denetimidir (ADR 0501 §3)",
    ),
    (
        "cad.entities.select",
        "too_many_points",
        "100 001 noktalı ya da kimlikli durum dosyayı megabaytlarca büyütür; bu dosyanın selections_beyond_the_limit testinde",
    ),
    (
        "cad.meshes.region",
        "too_many_mesh_values",
        "2 milyon değer ya da 100 000'i aşan yüz listesi yüz binlerce yüzlü bir mesh dosyası ister (megabaytlarca); değer sınırının aynı denetimi cad.meshes.profile'ın üç meshli durumunda (ADR 0645 §7)",
    ),
    (
        "cad.meshes.query",
        "too_many_mesh_values",
        "2 milyon değer için yüz binlerce adımlı bir zaman ekseni gerekir (megabaytlarca dosya); sınır meshes.rs'in MESH_MOST_VALUES denetimidir, kendi birim testinde (ADR 0552 §2)",
    ),
    (
        "*",
        "slots_exhausted",
        "4 milyarı aşkın yuvanın tükenmesi ortak durumla kurulamaz; masaüstünün kendi testinde (fixtures/commands/README.md)",
    ),
    (
        "*",
        "block_too_large",
        "milyon parçalı tanım ortak durum dosyasını megabaytlarca büyütür; kural kentos-contracts'ın blocks testinde ve KCAD'in broken/block-too-large.kcad'inde (ADR 0316 §4)",
    ),
    (
        "*",
        "block_refused",
        "beklenmez: komutlar belgenin blok kurallarını önce kendileri denetler",
    ),
    (
        "*",
        "layer_refused",
        "beklenmez: komutun denetimleri belgenin reddedeceği katmanı önce yakalar",
    ),
    (
        "cad.units.change",
        "sheets_kept",
        "ortak durumun kurulumu v1 JSON'dur, pafta kitabı taşımaz; units.rs'in kendi testinde (kitaplı çizim)",
    ),
    (
        "cad.entities.edit",
        "invalid_hatch",
        "kural cad.entities.create'in durumlarında; düzenleme aynı denetimden geçer",
    ),
    (
        "cad.entities.edit",
        "invalid_paragraph",
        "kural cad.entities.create'in durumlarında; düzenleme aynı denetimden geçer",
    ),
    (
        "cad.entities.edit",
        "invalid_refs",
        "kural cad.entities.create'in ve cad.dimensions.reassociate'in durumlarında",
    ),
    (
        "cad.entities.edit",
        "invalid_style",
        "kural cad.entities.create'in ve cad.styles.edit'in durumlarında",
    ),
    (
        "cad.entities.edit",
        "unknown_style",
        "kural cad.entities.create'in durumlarında; düzenleme aynı denetimden geçer",
    ),
    (
        "cad.layers.service",
        "layer_has_objects",
        "ortak durumu yok; masaüstünün Harita servisi penceresi nesneli katmanı önermez",
    ),
    (
        "cad.parameters.edit",
        "*",
        "çözümün ve kümenin retleri (çelişki, kilitli katman, tekillik, ters dönme, yakınsamama, kümenin kuralı) cad.constraints.edit'in durumlarında; parametre komutu aynı constraints_edit::settle'dan geçer (ADR 0336 §1)",
    ),
    (
        "cad.constraints.edit",
        "not_converged",
        "beklenmez: en az değişikliğin Gauss–Newton'u kısıtların tuttuğu bir çözüme ya ulaşır ya çelişki ya da tekillik bulur; 64 adımı aşan bir durum kararlı yazılamaz (ADR 0334 §4)",
    ),
    (
        "cad.entities.stretch",
        "neighbour_invalid",
        "beklenmez: Esnet köşeleri yalnız taşır, komşunun köşe sayısı değişmez; uyarı çekirdeğin kuralı değişirse diye yazılır (ADR 0254 §2)",
    ),
    (
        "cad.entities.join",
        "join_failed",
        "beklenmez: çekirdek yalnız boş zinciri reddeder, komut onu nothing_to_join ile önce yakalar (ADR 0254 §4)",
    ),
    (
        "cad.entities.array",
        "constraints_not_copied",
        "ilişkili dizinin ve kopyalanmayan nesneyi anan kısıtın uyarısı; Kopyala'nın durumları (cad.constraints.edit.follow.json) aynı constraint_copy'den geçer (ADR 0337 §3)",
    ),
    (
        "cad.forms.submit",
        "source_readonly",
        "commit'in dosyası cad.sources.sync'in with'iyle aynı yoldan yazılır (sources::commit); salt okunur tablonun reddi cad.sources.sync.together.json'da, dosyanın izni fixture'ın yazılabilirliğine bağlı (ADR 0532 §1)",
    ),
    (
        "cad.forms.submit",
        "source_changed",
        "commit'in dosyası cad.sources.sync'in with'iyle aynı yoldan yazılır; dosyanın katman onunla eşleştikten sonra değişmesi cad.sources.sync.together.json'da, bağın sürümü kurulumda dosyanın kendi özetidir (ADR 0532 §1)",
    ),
    (
        "cad.forms.submit",
        "geometry_mismatch",
        "commit'in dosyası cad.sources.sync'in with'iyle aynı yoldan yazılır; tablonun tutamadığı geometri cad.sources.sync.together.json'da (ADR 0532 §1)",
    ),
    (
        "cad.forms.submit",
        "file_unwritable",
        "dosyaya yazan execute ortak durumlarda yok (fixture dosyası değişirdi); geçici kopyada source_relations.rs sınar (ADR 0532 §1)",
    ),
    (
        "cad.processing.run",
        "crs_mismatch",
        "dosyada çizimin sistem kimliğiyle başka bir sistem olması: ortak durumların dosyaları (qgis.gpkg 5256) çizimin 5254'ünü taşımaz; sistemi başka bir dosyayı geçici klasörde processing_sink.rs kurar ve sınar (ADR 0549 §1)",
    ),
    (
        "cad.sources.relate",
        "file_unwritable",
        "dosyaya yazan execute ortak durumlarda yok (fixture dosyası değişirdi); geçici kopyada source_relations.rs sınar (ADR 0532 §2)",
    ),
    (
        "cad.sources.sync",
        "file_unwritable",
        "dosyaya yazan execute ortak durumlarda yok (fixture dosyası değişirdi); geçici kopyada sources.rs sınar",
    ),
    (
        "cad.sources.edit",
        "key_not_unique",
        "anahtarı yinelenen tablonun uyarısı: ortak durumların tabloları sabit; yinelenen anahtarlı yeniden yazılmış tabloyu geçici kopyada sources.rs sınar (ADR 0533 §1)",
    ),
    (
        "cad.sources.edit",
        "label_pins_lost",
        "alanın değişmesinde iğne yalnız park yeri dolunca (100 000 nesne) ya da satırın kimliği başka bir kayıt olunca kaybolur; ortak durumların dosyası sabit ve katman değişmişken alan değişmez; aynı kod cad.sources.sync'in durumlarında ve sources.rs'in geçici kopya testlerinde verilir (ADR 0578 §3)",
    ),
    (
        "cad.sources.sync",
        "label_pins_parked",
        "okumada iğnenin park edilmesi, nesnenin satırının alanın dışına çıkmasını (dosyanın yazılması) ister; ortak durumlar dosyaya yazmaz: kural cad.sources.edit'in alan durumlarında, dosyanın yazılmasıyla olanı sources.rs'in geçici kopya testinde sınanır (ADR 0578 §3)",
    ),
    (
        "cad.sources.sync",
        "label_pins_restored",
        "okumada park edilmiş iğnenin geri verilmesi, satırın alana girmesini (dosyanın yazılmasını) ister; ortak durumlar dosyaya yazmaz: kural cad.sources.edit'in alan durumlarında, geçici kopyada sources.rs'te sınanır (ADR 0578 §3)",
    ),
    (
        "cad.sources.edit",
        "label_pins_moved",
        "pinin başka satıra anahtarıyla taşınması kaynağın yeniden yazılmasını ister (satır kimlikleri değişir); ortak durumlar dosyaya yazmaz, geçici kopyada sources.rs sınar (ADR 0533 §1)",
    ),
    (
        "cad.sources.edit",
        "too_many_sources",
        "1000 bağlı katmanlık kurulum durum dosyasını büyütür; sınır sources.rs'te ve sözleşmenin sources_problem'inde",
    ),
    (
        "cad.sources.edit",
        "no_row_key",
        "view ve sorgu katmanı yalnız veritabanında; ev sahibinin sağlayıcısı uygulamanın testlerinde kayıtlı değil, durum kentos-postgis'in postgis_cases.py ile koşan testlerinde (ADR 0582)",
    ),
    (
        "cad.sources.edit",
        "row_key_not_unique",
        "view ve sorgu katmanı yalnız veritabanında; ev sahibinin sağlayıcısı uygulamanın testlerinde kayıtlı değil, durum kentos-postgis'in postgis_cases.py ile koşan testlerinde (ADR 0582)",
    ),
    (
        "cad.processing.run",
        "input_too_large",
        "bir milyon kaydı aşan kaynak durum dosyasını büyütür; sınır sources_query.rs'in stage'inde, a_source_input_stops_at_its_cap testinde (ADR 0592 §1)",
    ),
    (
        "cad.sources.query",
        "order_too_large",
        "200 000 kaydı aşan tablo durum dosyasını büyütür; sınır geçici dosyada sources_query.rs'in an_order_here_keeps_at_most_two_hundred_thousand_records'ında (ADR 0591 §2)",
    ),
    (
        "cad.sources.query",
        "no_row_key",
        "view ve sorgu katmanı yalnız veritabanında; ev sahibinin sağlayıcısı uygulamanın testlerinde kayıtlı değil, durum kentos-postgis'in postgis_cases.py ile koşan testlerinde (ADR 0582)",
    ),
    (
        "cad.sources.query",
        "row_key_not_unique",
        "view ve sorgu katmanı yalnız veritabanında; ev sahibinin sağlayıcısı uygulamanın testlerinde kayıtlı değil, durum kentos-postgis'in postgis_cases.py ile koşan testlerinde (ADR 0582)",
    ),
    (
        "cad.sources.sync",
        "source_others",
        "satır düzeyinde yazma yalnız veritabanında; aynı tabloyu ikinci düzenleyici (QGIS) değiştirir, kentos-postgis'in postgis_cases.py ile koşan testlerinde (ADR 0582)",
    ),
    (
        "cad.sources.sync",
        "source_reference",
        "yabancı anahtarı yalnız veritabanı zorlar: KentOS GeoPackage'a SQLite'ın foreign_keys anahtarı kapalı yazar; ret kentos-postgis'in postgis_cases.py ile koşan testinde (ADR 0755 §5)",
    ),
    (
        "cad.sources.sync",
        "constraints_unread",
        "kısıtlar tablonun okunduğu dosyadan ve bağlantıdan okunur: tablo okunup kısıtları okunamayan bir ortak durum kurulamaz; uyarının metni sources/constraints.rs'in unread'inde (ADR 0755 §1)",
    ),
    (
        "cad.sources.edit",
        "constraints_unread",
        "kısıtlar tablonun okunduğu dosyadan ve bağlantıdan okunur: tablo okunup kısıtları okunamayan bir ortak durum kurulamaz; uyarının metni sources/constraints.rs'in unread'inde (ADR 0755 §1)",
    ),
    (
        "cad.sources.sync",
        "rows_denied",
        "yetki ve satır güvenliği yalnız veritabanında: GeoPackage'ın satır hakkı yoktur; kentos-postgis'in postgis_cases.py ile koşan testi satır güvenliği politikalı bir hesapla sınar (ADR 0585)",
    ),
    (
        "cad.sources.describe",
        "source_unreachable",
        "veritabanına ulaşılıp ulaşılmadığı ev sahibinin sağlayıcısına bağlı (Python kaydeder, uygulamanın testleri kaydetmez); sağlayıcısız ret sources.rs'te, ulaşılamayan veritabanı kentos-postgis'in testlerinde (ADR 0505)",
    ),
    (
        "cad.sources.edit",
        "source_unreachable",
        "veritabanına ulaşılıp ulaşılmadığı ev sahibinin sağlayıcısına bağlı (Python kaydeder, uygulamanın testleri kaydetmez); sağlayıcısız ret sources.rs'te, ulaşılamayan veritabanı kentos-postgis'in testlerinde (ADR 0505)",
    ),
    (
        "cad.sources.sync",
        "source_unreachable",
        "veritabanına ulaşılıp ulaşılmadığı ev sahibinin sağlayıcısına bağlı (Python kaydeder, uygulamanın testleri kaydetmez); sağlayıcısız ret sources.rs'te, ulaşılamayan veritabanı kentos-postgis'in testlerinde (ADR 0505)",
    ),
    (
        "cad.sources.reconcile",
        "file_unwritable",
        "yazılamayan dosya ortak durumda kurulamaz; salt okunur kopyada katmanın yanının da geri alındığını reconcile.rs'in a_file_that_cannot_be_written_takes_nothing_into_the_layer'ı sınar (ADR 0538)",
    ),
    (
        "cad.sources.reconcile",
        "field_base_missing",
        "tabanın alan kaydı olmayan nesne (eski sürümde bağlanmış ya da kayıtları 8 MiB'a sığmayan katman) ortak durum setup'ında üretilemez: özetleri yeniden hesaplamak KentOS'un özet kuralını kopyalamak olurdu; reconcile.rs'in a_base_without_field_records_conflicts_as_a_whole_and_says_so'su kaydı kaldırıp sınar (ADR 0553 §1)",
    ),
    (
        "cad.sources.reconcile",
        "invalid_extent",
        "bağdaki alan sınırı tablonun sistemine çevrilemediğinde; ortak durumların sistemleri birbirine çevrilir (ADR 0510, 0538)",
    ),
    (
        "cad.sources.reconcile",
        "source_changed",
        "dosya uzlaştırmanın okuması ile yazması arasında değişirse; iki adım arası tek çağrının içinde ortak durumda kurulamaz, yazmanın denetimi cad.sources.sync'inkidir (ADR 0538 §2)",
    ),
    (
        "cad.sources.reconcile",
        "source_unreachable",
        "veritabanına ulaşılıp ulaşılmadığı ev sahibinin sağlayıcısına bağlı (Python kaydeder, uygulamanın testleri kaydetmez); sağlayıcısız ret sources.rs'te (ADR 0505, 0538)",
    ),
    (
        "cad.processing.tools",
        "tools_unavailable",
        "her ev sahibi işlem araçlarını kaydeder, ortak durumların koşucusu da; kaydedilmeyen ortamın reddi uygulamanın kendi testinde (processing_run.rs)",
    ),
    (
        "cad.processing.run",
        "tools_unavailable",
        "her ev sahibi işlem araçlarını kaydeder, ortak durumların koşucusu da; kaydedilmeyen ortamın reddi uygulamanın kendi testinde (processing_run.rs)",
    ),
    (
        "cad.processing.batch",
        "tools_unavailable",
        "her ev sahibi işlem araçlarını kaydeder, ortak durumların koşucusu da; kaydedilmeyen ortamın reddi uygulamanın kendi testinde (processing_run.rs)",
    ),
    (
        "cad.pipes.check",
        "tools_unavailable",
        "her ev sahibi işlem araçlarını kaydeder, ortak durumların koşucusu da; ağı kuran araçların reddi cad.processing.run'ınkiyle aynıdır (ADR 1640)",
    ),
    (
        "cad.pipes.check",
        "invalid_network",
        "ağ kurulurken yalnız projede olmayan kimlik reddedilir; komut ağı önce adıyla ya da kimliğiyle bulur (unknown_network), bu ret yalnız aradaki bir değişime karşıdır (ADR 1640)",
    ),
    (
        "cad.sources.sync",
        "invalid_extent",
        "bağdaki alan sınırı tablonun sistemine çevrilemediğinde; ortak durumların sistemleri birbirine çevrilir (ADR 0510)",
    ),
    (
        "cad.processing.run",
        "file_unwritable",
        "yazılamayan disk ortak durumda kurulamaz (plan dosya yazmaz); geçici adla yazılıp yerine konan dosya kentos-processing'in sink testinde",
    ),
    (
        "cad.dimensions.create",
        "underlay_measure",
        "PDF altlığının çizgisine bağlanan ölçü; altlık PDF dosyasından yüklenir, ortak durumların çizimleri dosyasızdır: xrefs.rs'in a_dimension_on_a_pdf_underlay_is_said_to_be_read_off_paper'ı (ADR 0351 §6)",
    ),
    (
        "cad.templates.apply", "*",
        "Paket yalnız sabit doğrulanmış A3 ekleme işlemleri üretir; alttaki sheets_edit çekirdeğinin savunma retleri mirastır (çekirdek testleri ve sheets.edit başvuruları). Okunamayan mevcut kitap ve stale revision, templates_safe::unreadable_book_and_stale_revision_are_atomic_refusals testinde validate/plan/execute ile yazmasız doğrulanır (ADR 0659).",
    ),
    (
        "cad.sheets.create", "*",
        "Şablon okuyucusunun ve aynı cad.sheets.edit çekirdeğinin retleri; template.rs/validate.rs testleri ve 22 bağımsız oluşturma durumu (ADR 0592).",
    ),
    (
        "cad.sheets.levels", "*",
        "Şablon okuyucusunun ve cad.sheets.edit çekirdeğinin retleri (cad.sheets.create'inkiler gibi; çekirdeğin testleri); kat retleri ve levels_have_sheets, not_fixed_map, bad_drawing_info ortak durumlarda (ADR 0718).",
    ),
    (
        "cad.sheets.details", "*",
        "Şablon okuyucusunun ve cad.sheets.edit çekirdeğinin retleri (cad.sheets.levels'inkiler gibi; çekirdeğin testleri); detay, tablo parçası ve yerleşim retleri ortak durumlarda (ADR 1090).",
    ),
    (
        "cad.navigator.read",
        "book_unreadable",
        "Ortak kurulum geçersiz kitap vermez; kitap cad.sheets.set'in aynı set_of yolundan okunur ve orada korunur (ADR 0778).",
    ),
    (
        "cad.sheets.fit",
        "book_unreadable",
        "Ortak kurulum geçersiz kitap vermez; kitap okuma aynı sheets_edit/sheets_set yolunda korunur (ADR 0592).",
    ),
    (
        "cad.sheets.edit",
        "*",
        "pafta çekirdeğinin kendi retleri; çekirdeğin testleri (crates/shared/sheet) ve cad.sheets.edit'in ilk ikisi",
    ),
    (
        "cad.stac.search",
        "stac_unavailable",
        "ortak durumların koşucusu kayıtlı cevaplı bir ağ kaydeder; ağsız ev sahibi stac_search.rs'in birim testinde (ADR 0528 §2)",
    ),
    (
        "cad.sources.edit",
        "network_share",
        "ağ dosya sistemi (NFS, SMB, sshfs) ortak durumda kurulamaz; bağlama noktasının okunuşu file_place.rs'in testlerinde Linux'un ve macOS'un tablolarıyla sınanır (ADR 0627 §8)",
    ),
    (
        "cad.rasters.gpkgread",
        "file_unwritable",
        "yazılamayan disk ya da geri okununca aynı çıkmayan GeoTIFF ortak durumda kurulamaz (plan dosya yazmaz); geçici adla yazılıp geri okunarak yerine konması gpkg_tiles_files.rs'te sınanır (ADR 0556 §3)",
    ),
    (
        "cad.profiles.import",
        "file_unwritable",
        "yazılamayan klasör ortak durumda kurulamaz (plan dosya yazmaz); klasörün yerinde dosya olması ve eski dosyanın olduğu gibi kalması, yazılan dosyanın geri okunması ve yarım dosyanın kalmaması profile_files.rs'te sınanır (ADR 0558 §3)",
    ),
    (
        "cad.rasters.gpkgwrite",
        "file_unwritable",
        "yazılamayan disk ya da geri okununca aynı çıkmayan GeoPackage ortak durumda kurulamaz (plan dosya yazmaz); geçici adla yazılıp geri okunarak yerine konması ve yarım dosyanın kalmaması gpkg_tiles_files.rs'te sınanır (ADR 0556 §4)",
    ),
];

fn why_unreached(command: &str, code: &str) -> Option<&'static str> {
    UNREACHED
        .iter()
        .find(|(c, k, _)| (*c == "*" || *c == command) && (*k == "*" || *k == code))
        .map(|(_, _, why)| *why)
}

#[test]
fn every_code_a_case_reaches_is_listed_and_every_listed_one_is_reached_or_explained() {
    let reached = reached();
    let mut problems = Vec::new();
    // The change set answers with its steps' codes too.
    let every: BTreeSet<String> = DESKTOP_COMMANDS
        .iter()
        .filter_map(|(id, _)| capabilities_of(id))
        .flat_map(|c| c.refusals.into_iter().chain(c.warnings))
        .collect();
    for (id, _) in DESKTOP_COMMANDS {
        let Some(c) = capabilities_of(id) else {
            problems.push(format!("{id}: yetenekleri yok"));
            continue;
        };
        let listed: BTreeSet<String> = c.refusals.iter().chain(&c.warnings).cloned().collect();
        let got = reached.get(*id).cloned().unwrap_or_default();
        for code in &got {
            // The protocol's codes any call may answer with (the constraints' follow, docs/adr/0337) need no listing.
            let ok = listed.contains(code)
                || PROTOCOL_CODES.contains(&code.as_str())
                || (*id == "cad.changeset.apply" && every.contains(code));
            if !ok {
                problems.push(format!("{id}: durumlar {code} veriyor, katalog anmıyor"));
            }
        }
        for code in &listed {
            if !got.contains(code) && why_unreached(id, code).is_none() {
                problems.push(format!(
                    "{id}: {code} anılıyor ama hiçbir durum onu vermiyor ve gerekçesi yok"
                ));
            }
        }
    }
    // Every command of the catalog the desktop or the headless host runs has its capabilities.
    for (id, _) in FILE_COMMANDS {
        assert!(capabilities_of(id).is_some(), "{id}");
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// An object of every kind, each in a drawing of the shared samples that has it.
fn one_of_each_kind() -> BTreeMap<&'static str, (Document, Uuid)> {
    let mut out = BTreeMap::new();
    let dir = root().join("fixtures/kcad/v2");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("the folder")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_name().is_none_or(|n| n != "expected.json")
        })
        .collect();
    files.sort();
    for file in files {
        let Ok(snapshot) = serde_json::from_str::<DocumentSnapshotV2>(
            &std::fs::read_to_string(&file).expect("reads"),
        ) else {
            continue;
        };
        let Ok(doc) = Document::from_snapshot_v2(snapshot) else {
            continue;
        };
        let wanted: Vec<(&'static str, Uuid)> = KINDS
            .iter()
            .filter(|k| !out.contains_key(*k))
            .filter_map(|k| {
                doc.entities()
                    .find(|e| e.kind() == *k)
                    .and_then(|e| doc.uid(kentos_domain::Slot(e.base().id)))
                    .map(|u| (*k, u))
            })
            .collect();
        for (kind, uid) in wanted {
            out.insert(kind, (doc.clone(), uid));
        }
    }
    out
}

/// The code `command` answers `validate` of `input` with: none when it is taken.
fn refusal(doc: &mut Document, command: &str, input: Value) -> Option<String> {
    let mut cx = ExecutionContext::new(doc);
    let answer = registry::run(&mut cx, command, CallOp::Validate, input).expect("runs");
    match answer["status"].as_str() {
        Some("failed") | Some("conflict") => answer["error"]["code"].as_str().map(str::to_owned),
        _ => None,
    }
}

/// The form a target's note names, given to a sample that is not in it
/// before it is tried: Çoklu tara takes a closed curve (an open one is
/// refused with `not_closed`, as its note says).
type Form = fn(&mut kentos_commands::Entity);
fn curved_space(e: &mut kentos_commands::Entity) {
    if let kentos_commands::Entity::Space(s) = e {
        s.space.parts = vec![serde_json::from_value(json!({"outer":{
            "pts":[{"x":-1.,"y":0.},{"x":1.,"y":0.}],"bulges":[1.,1.]}})).unwrap()];
        s.space.seed = None;
        s.space.columns = kentos_commands::SpaceColumns::Ignore;
    }
}
// Kind capabilities need a geometry that fits the requested operation. In particular a two-vertex arc
// cannot stand in for a straight corner, and a straight polyline cannot prove the radius target's arc support.
fn straight_path(e: &mut kentos_commands::Entity) {
    if let kentos_commands::Entity::Polyline(p) | kentos_commands::Entity::Polygon(p)=e {
        p.pts=vec![kentos_commands::Vec2{x:0.,y:0.},kentos_commands::Vec2{x:5.,y:0.},kentos_commands::Vec2{x:5.,y:5.}];
        p.bulges=None;p.parts=None;p.holes=None;p.widths=None;p.zs=None;p.ms=None;
    }
}
fn curved_path(e: &mut kentos_commands::Entity) {
    straight_path(e);
    if let kentos_commands::Entity::Polyline(p) | kentos_commands::Entity::Polygon(p)=e {p.bulges=Some(vec![1.,0.,0.]);}
}
const FORMS: &[(&str, &str, Form)] = &[
    ("cad.blocks.find", "text", |e| {
        if let kentos_commands::Entity::Text(t)=e {
            t.label_of=None;t.label_scale=None;t.annotative=None;t.path=None;
            t.paragraph=Default::default();t.text="TEST".into();t.height=1.;t.rotation=0.;
        }
    }),
    ("cad.dimensions.create", "polyline", curved_path),
    ("cad.dimensions.create", "polygon", curved_path),
    ("cad.entities.fillet", "polyline", straight_path),
    ("cad.entities.chamfer", "polyline", straight_path),
    ("cad.points.construct", "polyline", straight_path),
    ("cad.spaces.separators","polyline",|e|if let kentos_commands::Entity::Polyline(p)=e {
        p.pts=vec![kentos_commands::Vec2{x:0.,y:0.},kentos_commands::Vec2{x:5.,y:0.}];
        p.bulges=None;p.closed=false;p.parts=None;p.holes=None;p.widths=None;p.zs=None;p.ms=None;p.space_separator=None;
    }),
    ("cad.dimensions.create", "space", curved_space),
    // Kat ver: a polyline takes a level as a room separator (docs/adr/0717).
    ("cad.levels.assign", "polyline", |e| {
        if let kentos_commands::Entity::Polyline(p) = e {
            p.space_separator = Some(Default::default());
        }
    }),
    ("cad.entities.hatch", "spline", |e| {
    if let kentos_commands::Entity::Spline(s) = e {
        s.closed = true;
    }
})];

#[test]
fn a_listed_kind_is_taken_and_another_is_refused_with_the_listed_code() {
    let kinds = one_of_each_kind();
    assert_eq!(
        kinds.len(),
        KINDS.len(),
        "the samples lack a kind: {:?}",
        KINDS
            .iter()
            .filter(|k| !kinds.contains_key(*k))
            .collect::<Vec<_>>()
    );
    // The command, the operation its targets name (none: the whole), the input for an object.
    type Make = fn(&str) -> Value;
    let tries: &[(&str, Option<&str>, Make)] = &[
        ("cad.spaces.number",None,|u|json!({"uids":[u],"first":"TEST-001"})),
        // A level the sample drawing lacks: a kind with a level is refused for it, another for its kind (docs/adr/0717).
        ("cad.levels.assign", None, |u| json!({ "uids": [u], "level": "yok" })),
        ("cad.spaces.separators",Some("set"),|u|json!({"operation":{"kind":"set","uids":[u]}})),
        ("cad.spaces.separators",Some("clear"),|u|json!({"operation":{"kind":"clear","uids":[u]}})),
        (
            "cad.entities.offset",
            None,
            |u| json!({ "uids": [u], "by": { "kind": "distance", "distance": 0.5, "side": { "x": 1.0e9, "y": 1.0e9 } } }),
        ),
        // Sabit on a circle: a circle and an arc have the part, another kind is refused for it (docs/adr/0335).
        ("cad.constraints.edit", Some("circle"), |u| {
            json!({ "edit": { "kind": "add", "constraints": [{ "rule": { "kind": "fix",
                    "a": { "uid": u, "element": { "kind": "circle" } } } }] } })
        }),
        // Duvarlardan bul with a seed far away: a space is refused for the seed, another kind is not one (docs/adr/0342 §4).
        (
            "cad.spaces.edit",
            None,
            |u| json!({ "uids": [u], "operation": { "kind": "set", "name": "Oda" } }),
        ),
        // Çevir: an opening turns, another kind is not one (docs/adr/0341 §4).
        (
            "cad.openings.edit",
            None,
            |u| json!({ "uids": [u], "operation": { "kind": "flip", "what": "side" } }),
        ),
        (
            "cad.dimensions.reassociate",
            None,
            |u| json!({ "uid": u, "points": [{ "point": "a" }] }),
        ),
        // Kaldır: a picture takes it (nothing to take away writes nothing), another kind is not one (docs/adr/0622 §9).
        (
            "cad.images.clip",
            None,
            |u| json!({ "uid": u, "clip": { "kind": "remove" } }),
        ),
        (
            "cad.entities.fillet",
            Some("vertex"),
            |u| json!({ "corner": { "kind": "vertex", "uid": u, "index": 1 }, "radius": 0.001 }),
        ),
        (
            "cad.entities.chamfer",
            Some("vertex"),
            |u| json!({ "corner": { "kind": "vertex", "uid": u, "index": 1 }, "distance": 0.001 }),
        ),
        (
            "cad.entities.transform",
            None,
            |u| json!({ "uids": [u], "transform": { "kind": "move", "dx": 1.0, "dy": 0.0 } }),
        ),
        ("cad.blocks.find", None, |u| json!({"source":[u]})),
        (
            "cad.entities.array",
            None,
            |u| json!({ "uids": [u], "layout": { "kind": "grid", "rows": 1, "cols": 2, "dx": 1.0, "dy": 0.0 } }),
        ),
        // A window around everything: a kind that stretches has a corner in it.
        ("cad.entities.stretch", None, |u| {
            json!({
                "window": { "min": { "x": -1.0e12, "y": -1.0e12 }, "max": { "x": 1.0e12, "y": 1.0e12 } },
                "dx": 1.0, "dy": 0.0, "uids": [u]
            })
        }),
        // A point off the object: a kind that breaks is refused for the point (break_failed), not for its kind.
        (
            "cad.entities.break",
            None,
            |u| json!({ "uid": u, "first": { "x": 0.0, "y": 0.0 } }),
        ),
        // A seed alone: a kind that chains has nothing to join, another is not joinable. (Two
        // objects meeting end to end, the whole's target, cannot be made of one sample object.)
        (
            "cad.entities.join",
            Some("chain"),
            |u| json!({ "uids": [u], "chain": { "kind": "objects", "uids": [u] } }),
        ),
        // A layer that is not there: a kind that divides is refused for the layer, after its points.
        (
            "cad.entities.divide",
            None,
            |u| json!({ "uid": u, "division": { "kind": "parts", "count": 2 }, "layerId": "yok" }),
        ),
        (
            "cad.entities.hatch",
            Some("objects"),
            |u| json!({ "region": { "kind": "objects", "uids": [u] }, "pattern": { "name": "Dolu" }, "layerId": "yok" }),
        ),
        // A circle's or an arc's edge: a kind that has one is refused for the layer, or for a dimension
        // that does not form there, not for its kind.
        ("cad.dimensions.create", Some("radius"), |u| {
            json!({ "measure": { "kind": "radius", "of": { "uid": u, "near": { "x": 0.0, "y": 0.0 } } },
                    "at": { "x": 1.0e9, "y": 1.0e9 }, "layerId": "yok" })
        }),
        // Çoklu çizgiye çevir: a NURBS curve takes it, a polyline is refused for the edit (not its kind).
        (
            "cad.curves.edit",
            None,
            |u| json!({ "uid": u, "edit": { "kind": "toPolyline", "tolerance": 1.0 } }),
        ),
        // Nesne: a kind that gives axes sets the UCS, another is refused for its kind.
        (
            "cad.ucs.set",
            Some("object"),
            |u| json!({ "to": { "kind": "object", "uid": u, "at": { "x": 0.0, "y": 0.0 } } }),
        ),
        // Paralel: a kind with a straight edge gives the point, another is refused for its kind (docs/adr/0270).
        ("cad.points.construct", Some("parallel"), |u| {
            json!({ "constructions": [{ "kind": "parallel", "through": { "kind": "at", "at": { "x": 0.0, "y": 0.0 } },
                    "edge": { "uid": u, "near": { "x": 0.0, "y": 0.0 } }, "distance": 1.0 }] })
        }),
        ("cad.dimensions.create", Some("arcLength"), |u| {
            json!({ "measure": { "kind": "arcLength", "of": { "uid": u, "near": { "x": 0.0, "y": 0.0 } } },
                    "at": { "x": 1.0e9, "y": 1.0e9 }, "layerId": "yok" })
        }),
    ];
    let mut problems = Vec::new();
    for (command, operation, make) in tries {
        let c = capabilities_of(command).expect("listed");
        let target = c
            .targets
            .iter()
            .find(|t| t.operation.as_deref() == *operation)
            .unwrap_or_else(|| panic!("{command}: no target {operation:?}"));
        let code = target.refused_with.clone().expect("a refusing code");
        for kind in KINDS {
            let taken = if target.kinds.iter().any(|k| k == "*") {
                !target.except.iter().any(|k| k == kind)
            } else {
                target.kinds.iter().any(|k| k == kind)
            };
            let (doc, uid) = &kinds[kind];
            let mut doc = doc.clone();
            // An object on a locked layer is refused for its layer first: unlock it.
            let layer = doc
                .get(doc.slot_of(*uid).expect("there"))
                .map(|e| e.base().layer_id.clone())
                .unwrap_or_default();
            doc.set_layer_locked(&layer, false);
            if let Some((_, _, form)) = FORMS.iter().find(|(c, k, _)| c == command && *k == kind) {
                let slot = doc.slot_of(*uid).expect("there");
                let mut e = doc.get(slot).expect("there").clone();
                form(&mut e);
                assert!(
                    doc.update(slot, e),
                    "{command}: {kind} could not be given its form"
                );
            }
            let got = refusal(&mut doc, command, make(&uid.to_string()));
            let refused_for_kind = got.as_deref() == Some(code.as_str());
            if taken && refused_for_kind {
                problems.push(format!(
                    "{command}: {kind} listede ama {code} ile reddedildi"
                ));
            }
            if !taken && !refused_for_kind {
                problems.push(format!(
                    "{command}: {kind} listede değil ama {code} ile reddedilmedi ({got:?})"
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// `cad.entities.select`'s `too_many_points`, which no shared case reaches (see [`UNREACHED`]): a fence of more than
/// 100 000 points and a list of more than 100 000 objects are refused at the list, before any is read.
#[test]
fn selections_beyond_the_limit_are_refused() {
    let (mut doc, _) = one_of_each_kind().into_values().next().expect("a drawing");
    let n = kentos_commands::MAX_SELECT_ITEMS + 1;
    let points: Vec<Value> = (0..n)
        .map(|i| json!({ "x": i as f64, "y": (i % 7) as f64 }))
        .collect();
    let fence = json!({ "region": { "kind": "fence", "points": points, "tolerance": 0 } });
    assert_eq!(
        refusal(&mut doc, "cad.entities.select", fence).as_deref(),
        Some("too_many_points")
    );
    let uids = vec![json!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"); n];
    let list = json!({ "region": { "kind": "objects", "uids": uids } });
    assert_eq!(
        refusal(&mut doc, "cad.entities.select", list).as_deref(),
        Some("too_many_points")
    );
}

/// `cad.plotstyles.import`'s `plot_style_file_too_large` and `file_unreadable`, which no shared case reaches (see
/// [`UNREACHED`]): a table past 4 MiB is refused before it is read, a file the user may not read says so.
#[test]
fn plot_style_files_beyond_the_limit_and_unreadable_files_are_refused() {
    let (mut doc, _) = one_of_each_kind().into_values().next().expect("a drawing");
    let dir = std::env::temp_dir().join(format!("kentos-ctb-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let big = dir.join("buyuk.ctb");
    let mut bytes = b"PIAFILEVERSION_2.0,CTBVER1,compress\r\npmzlibcodec".to_vec();
    bytes.resize(4 * 1024 * 1024 + 1, 0);
    std::fs::write(&big, &bytes).expect("writes");
    assert_eq!(
        refusal(&mut doc, "cad.plotstyles.import", json!({ "path": big })).as_deref(),
        Some("plot_style_file_too_large")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let shut = dir.join("kapali.ctb");
        std::fs::write(&shut, b"PIAFILEVERSION_2.0,CTBVER1").expect("writes");
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o000)).expect("chmod");
        // A user who may read anything (root) reads it anyway; the refusal is then the reader's.
        if std::fs::read(&shut).is_err() {
            assert_eq!(
                refusal(&mut doc, "cad.plotstyles.import", json!({ "path": shut })).as_deref(),
                Some("file_unreadable")
            );
        }
        let _ = std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o644));
    }
    let _ = std::fs::remove_dir_all(dir);
}
