# 4080 — Onaylı hâline getir: silinen ve değişen nesnelerin onayın değerleriyle geri gelmesi

Tarih: 2026-11-09. plan/map-cad.md MAP-01, sekizinci tur. Durum: taslak. ADR 3680'in “Sonra”sındaki “Silinen nesneyi
onaylanan hâline geri getirme komutu (değerden; komut, Python ve MCP bütünlüğüyle)”.

İlgili:
- [ADR 0026](0026-cloud-sync-persistent-ids.md): silinen kimlik yeniden oluşturulabilir;
- [ADR 2880](2880-map-work-objects-changed-since-approval.md): onaydan sonra değişen nesneler;
- [ADR 3680](3680-map-work-approved-values.md): onayın nesnelerin değerlerini tutması (şema 213).

## Dayanak

- **ArcGIS REST API**, [Restore Rows](https://developers.arcgis.com/rest/services-reference/enterprise/restore-rows/)
  (Version Management Server, 9 Kasım 2026'da okundu):
  - işlem, satırları ortak ata sürümünden geri getirir;
  - istekte satırlar katmanıyla ve nesne kimlikleriyle (`objectIds`) verilir;
  - yalnız bir uzlaştırmanın bulduğu silme–güncelleme çakışmalarında çağrılmalıdır.

  Yani geri getirme, kaydedilmiş bir hâlden, nesnenin kendi kimliğiyle yapılır.
- **ArcGIS Pro'nun arşiv geçmişi** (ADR 3680'in okuması): eski hâl saklanmadıkça “ne idi” söylenemez.
- **KentOS bugün** (ADR 3680):
  - onay her nesnenin değerini tutuyordu ve `map.work.query` silinen nesneyi onaylanan hâliyle söylüyordu;
  - ama geri getiren bir komut yoktu (ADR 3680, Sınırlar). Kullanıcı silineni yeniden çizmek ya da geri almanın
    geçmişinde aramak zorundaydı; yeniden çizilen nesne yeni bir kimlik alır, türetme kayıtları ve onay onu tanımaz.

## Karar

1. **`map.work.restore`** (yeni komut, v1; “Onaylı hâline getir”): onaylı bir harita işi öğesinin nesnelerini onayın
   tuttuğu değerlerle geri getirir. Tek geri alma adımıdır, hepsi ya da hiçbiri:
   - onaydan sonra çizimden silinen onaylı nesne aynı kalıcı kimlikle, onaylanan değeriyle yeniden oluşturulur
     (`restored`). Çizimin bir sonraki yuvasına girer (ADR 0026: silinen kimlik yeniden oluşturulabilir);
   - çizimde olup özeti onaylanandan farklı olan nesne (değişen ya da öğenin katmanlarından başka katmana taşınan)
     yuvasında onaylanan değerine döner (`reverted`);
   - onaydan sonra eklenen nesneler yerinde kalır, sayısı söylenir (`added`). Silinmeleri ayrı bir karardır;
   - `asApproved`: geri getirmeden sonra öğenin nesneleri onaylananlarla aynı mı (eklenen yoksa evet).
2. **`uids`:** verilirse yalnız o onaylı nesneler. Her biri öğenin onaylı nesnelerinden biri ve silinmiş ya da değişmiş
   olmalıdır; iki kez verilen bir kez sayılır. Verilmezse silinen ve değişenlerin hepsi.
3. **Retler**, sırayla:
   - `invalid_revision`, `revision_conflict`; `not_a_map_project`; `unknown_work_item` (`item`);
   - `work_not_approved`: öğe onaylı değil;
   - `no_approved_values`: dosyalı kaynak (onayı dosyasının revizyonunu tutar), nesne başına özet tutmayan onay (şema
     210'dan önce ya da 100 000'den çok nesne), değerleri tutmayan onay (şema 213'ten önce ya da 4 MiB'tan büyük).
     İletisi nedeni ve “öğeyi yeniden onaylayın”ı söyler;
   - `invalid_uid`, `not_approved_object` (`uids[i]`); `nothing_to_restore` (`uids[i]`: o nesne onaylandığı gibi;
     `item`: hiçbiri değişmemiş);
   - her geri gelen nesnenin onaylı katmanı: çizimde (`layer_not_found`), katman (`not_a_layer`), kilitsiz
     (`layer_locked`), türüne uygun (`layer_geometry_mismatch`). Değişen nesnenin şimdiki katmanı kilitli olmamalı
     (`layer_locked`). Yol `uids[i]`, ya da bütün öğede `item`;
   - uyarı: gizli katmana geri gelen nesne (`layer_hidden`).
4. **Yeni kodlar:** `work_not_approved`, `no_approved_values`, `not_approved_object`, `nothing_to_restore`.
5. **Masaüstü** (Harita işi):
   - onaylı öğenin Değişenler satırında, silinen ya da değişen varsa ↺ düğmesi: “Onaylı hâline getir: silinenler aynı
     kimlikle geri gelir, değişenler onaylanan değerine döner; eklenenler kalır”;
   - durum çubuğu ne yapıldığını söyler. Simge var olan geri al simgesidir, yeni simge yok.
6. **Kapsam işareti:** ADR 2880 ve 3680 gibi **yalnız Harita (CAD)**.

## Sınırlar

- Onaydan sonra eklenen nesneler silinmez; “öğeyi tam onaylı hâline döndür” isteği eklenenleri ayrıca silmeyi
  gerektirir.
- Nesnenin bağları (yazının, taramanın, ölçünün bağlı olduğu nesne) değerin içindedir. Bağlı olduğu nesne de silindiyse
  bağ ancak o da geri getirilince tutar; KentOS bunu ayrıca denetlemez.
- Onaylı katmanı silinmiş nesne için katman geri getirilmez; ret katmanı söyler.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/map_work_restore_cases.py`; KentOS kodu yok):
  - çizim map_work_cases.py'ninkidir. Tasarım yüzeyi onaylanır; onay, nesnelerin özetini, parçalarını ve değerlerini
    o betiğin kurallarıyla tutar;
  - her durumun çizimi onaydan sonra olanlardır: silinen, değişen (öznitelik, geometri), başka katmana taşınan,
    eklenen, kilitli, gizli, silinmiş ya da nokta katmanına çevrilmiş katman. Kalan nesneler dosya sırasıyla yeniden
    numaralanır (`.kcad` boşluk tutmaz);
  - 28 durum: geri gelen nesne bir sonraki yuvada onaylanan değeriyle, dönen nesne yuvasında; ikinci istekte
    `nothing_to_restore`; geri alınınca yine hepsi; plan, doğrulama, uyarı ve on beş ret;
  - Rust ve Python geçer; yetenekler testi her anılan kodun bir durumda verildiğini doğrular.
- **Python** (`python/tests/test_map_work_restore.py`), testin kendi özet hesabıyla ve bağımsız `.kcad` okuyucusuyla
  (tools/kcad):
  - değişen çizgi yuvasında, özeti onayınki;
  - silinen çizgi aynı kimlikle, değeri onaylanan değerin kendisi;
  - eklenen kalır; geri almada sorgu çizgiyi yine silinmiş söyler; ikinci istek `nothing_to_restore`.
- **MCP** (`crates/native/mcp/tests/map_work_restore.rs`): şema 213 örneğinde değişen ve silinen çizgi geri gelir, tasarım
  güncel; ikinci istek ve nesnesiz teslim paketi reddedilir. ADR 3680'in MCP testi değişmeden geçer.
- **Masaüstü** (`map_work_restore_tests::onayli_haline_getir_puts_back_the_deleted_and_the_changed_line`):
  - Değişenler 1/1/1 → 0/1/0, durum çubuğunun cümlesi, silinen çizgi onaylanan değeriyle;
  - tek geri alma eski hâle döndürür.

  Harita işi testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/onayli-haline-4080/`. Geri gelen çizgi
  GNSS ölçümünün türetme kaydında da adlı olduğundan, kayıt yeniden tamamlanır (“Güncel değil” 2 → 1).

## Sonra

- ~~Geometri farkının çizimde gösterilmesi (onaylanan hâlin hayalet çizimi).~~ ADR 4480.
- Dosyası olan öğede nesne başına fark; tipli katman profili (MAP-01'in kalanı).
- Onaydan sonra eklenenleri de silen “tam onaylı hâl” seçeneği (kullanıcının kararı gerekir).
- Web (TODOS §0.2).
