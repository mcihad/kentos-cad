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
        "cad.crs.transform",
        "no_datum_link",
        "projenin WGS 84'e yolu olmayan kendi datumu; kayıttaki sistemlerle kurulamaz, geometri-core'un datum testi (Gap::NoLink) sınar (ADR 0168 §2, 0519 §3)",
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
        "eğrinin kopyası yaklaşıktır, kesin bir beklentiyle yazılamaz; geometri-core'un offset_notes testinde eğriye uzaklığıyla denetlenir (ADR 0293)",
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
        "cad.sources.sync",
        "key_not_unique",
        "anahtarı yinelenen tablonun uyarısı: ortak durumların tabloları sabit; yinelenen anahtarlı yeniden yazılmış tabloyu geçici kopyada sources.rs sınar (ADR 0533 §1)",
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
        "cad.rasters.gpkgread",
        "file_unwritable",
        "yazılamayan disk ya da geri okununca aynı çıkmayan GeoTIFF ortak durumda kurulamaz (plan dosya yazmaz); geçici adla yazılıp geri okunarak yerine konması gpkg_tiles_files.rs'te sınanır (ADR 0556 §3)",
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
const FORMS: &[(&str, &str, Form)] = &[("cad.entities.hatch", "spline", |e| {
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
