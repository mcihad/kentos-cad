# ADR 8136: Korunumlu şaft ve yangın kesici — BYKHY Ek-3/B 8 ve 10

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (Kalan: "Ek-3'te korunumlu
  şaft beyanı ve yangın kesici"), sıra B15; ADR [8082](8082-fire-resistance-of-elements.md) §6, [8087](8087-shafts-elevators-roofs.md)
  §6, [8090](8090-declared-load-bearing-and-escape-roof.md) §6.
- **Bağlam:** ADR [8082](8082-fire-resistance-of-elements.md) (Ek-3/B ve Ek-3/C, `dayanim`), [8087](8087-shafts-elevators-roofs.md)
  (şaftın çevresi), [8090](8090-declared-load-bearing-and-escape-roof.md) ve [8124](8124-b7-remaining-declarations-and-ifc-fields.md)
  (rol beyanları), [8135](8135-lift-shaft-vent.md) (katlarca tek şaft, kuyunun mahalindeki beyan). Kurallar
  Python'dadır, Rust olguları verir (bim-plan K10).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 4/1-pp (yangın kesici), Ek-3/B 8 ve 10.
- **Sözleşme:** `DeclaredProtected`, `DeclaredFireStop`; `ArchInfo.protected`, `ArchInfo.fire_stop`;
  `cad.arch.performance` v1'in nesne işlemine `protected` ve `fireStop` (verilmezse kalır, null kaldırır);
  `kentos.auditFacts` 1'e `building.shafts[].protected`, `building.elements[].fireStop`. **`.kcad`:** belge şeması
  değişmedi; mimari verisi 25 (`ARCH_WITH_FIRE_STOPS`). **Web:** TS türleri yeniden üretildi; Öznitelikler'in satırları
  TODOS.md §0.2'de.

## §1. Sorun

Ek-3/B'nin iki satırı okunmuyordu: 8 (korunumlu şaftlar REI 120) ve 10 (yangın kesici EI 30). İkisinin de modelde ne
rolü ne öğesi vardı (8090 §6): bir şaftın korunumlu olduğunu ya da bir duvarın yangın kesici olduğunu çizim
söylemez.

## §2. Araştırma (18 Kasım 2026)

Yönetmeliğin metni derlenmiş resmî metinden (pdftotext; ADR 8081'in kopyası) ve Ek-3/B'nin tablosu PDF'in 91. sayfasından
okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY Ek-3/B 8 | "Korunumlu Şaftlar (korunumlu yangın merdiveni yuvaları ve acil durum asansör kuyuları hariç)" REI 120, "Ayrı ayrı her bir yüzey" | Korunumlu beyanlı şaftın duvarları REI 120 (K334) |
| BYKHY Ek-3/B 10 | "Yangın Kesici" EI 30, "Ayrı ayrı her bir yüzey" | Yangın kesici beyanlı duvar ve döşeme EI 30 (K335) |
| BYKHY 4/1 (tanımlar) | "dd) Korunumlu koridor veya hol", "ee) Korunumlu merdiven" tanımlı; korunumlu şaft tanımlı değil. "pp) Yangın kesici: Bina içinde, yangının ve dumanın ilerlemesini ve yayılmasını belirlenmiş bir süre için durduran, yatay veya düşey konumlu elemanı" | İkisi de müellifin beyanıdır (K332, K333); yangın kesici yatay ya da düşey eleman: duvar ve döşeme |
| BYKHY 25/3 | Yüksek binada düşey tesisat şaftı ve bacanın duvarı en az 120, kapağı en az 90 dakika ve duman sızdırmaz | Ayrı madde (`kuyular`, 8087); korunumlu beyanı 25/3'ün yerine geçmez |
| BYKHY 87/8, 87/11 | Havalandırma kanalı korunmuş bir şafttan geçiyorsa şafta giriş ve çıkışta yangın damperi; tesisat bacası ve kanallar kompartımanın dayanımını azaltmayacak şekilde yalıtılır | Kanal ve damper modelde yok (§6) |
| BYKHY 20/4, 28/3-a | Pis su borusu kat geçişinde yangın kesici; yüksek binada çatının oturduğu döşeme yatay yangın kesici niteliğinde | 20/4 boru modelde yok; 28/3-a ayrı iş (§6) |
| IFC 4.3 `Pset_WallCommon.Compartmentation`, `Pset_SlabCommon.Compartmentation` (buildingSMART belgesi) | "Indication whether the object is designed to serve as a fire compartmentation" | Kompartıman duvarının işaretidir, yangın kesici değil: yangın kesici IFC'ye yazılmaz (§6) |

## §3. Karar

- **K332 — Korunumlu şaft beyandır:** şaft olan boşluk mahal (asansör kuyusu, tesisat şaftı, hava bacası; adı ŞAFT,
  BACA ya da TESİSAT ile başlayan sözcük; ışıklık değil: gökyüzüne açıktır) `ArchInfo.protected` taşır: Evet, Hayır
  ve dayanağı. Beyansız şaft korunumlu sayılmaz. Bir katındaki beyan katlarca bütün şafta geçer (8135'in yığını).
- **K333 — Yangın kesici beyandır:** duvar ya da döşeme (balkon ve kaplama değil; Ek-3'ün okuduğu elemanlar)
  `ArchInfo.fire_stop` taşır: Evet, Hayır ve dayanağı. Kapı ve pencere duvarı gibidir, beyan edilmez.
  `cad.arch.performance`'ın nesne işlemi ikisini öbür alanlar gibi yazar: verilmeyen kalır, null kaldırır; başka
  nesneye `field_not_for_kind`, dayanağı `operation.protected.basis`, `operation.fireStop.basis`. `.kcad` mimari verisi
  25; 25'ten küçük diyen dosyadaki beyanı `decode` reddeder.
- **K334 — Ek-3/B 8 (`dayanim`):** korunumlu şaftın her katındaki duvarları REI 120 (kendi satırları daha çok
  istiyorsa o); şaftın katlarında Evet ve Hayır birlikteyse uzman incelemesi; çevresinde duvar çizilmemişse eksik veri,
  açık yeri (5 cm'den uzun) varsa kaldı. Kapağın süresi bu satırda yok (yüksek binanınki 25/3'te).
- **K335 — Ek-3/B 10 (`dayanim`):** yangın kesici EI 30. Elemanın kendi satırı (kompartıman duvarı, döşeme, dış
  duvar…) E ve I ile en az 30 istiyorsa o yeter; istemiyorsa (taşıyıcı iç duvarın R'si gibi) 10'un isteği ayrıca okunur
  ve bulgusu ayrıdır: R 60 beyanlı taşıyıcı yangın kesici 2'yi tutar, 10'dan kalır.
- **K336 — Öznitelikler:** şaft mahalinde "Korunumlu" (Evet, Hayır, Bilinmiyor; Evet'te not: "Ek-3/B 8: duvarları en az
  REI 120; bir katta beyan yeter."), duvar ve döşemede "Yangın kesici" (Evet, Hayır, Bilinmiyor; Evet'te not:
  "Ek-3/B 10: en az EI 30, her yüzü."). Panelin dayanağı müellif beyanıdır; belge dayanağı komutla verilir.

## §4. Uygulama

- `crates/shared/contracts/src/arch/performance.rs` (`DeclaredProtected`, `DeclaredFireStop`), `arch/arch.rs`
  (`ArchInfo.protected`, `fire_stop`); `crates/shared/commands/src/arch/cad_arch_performance.rs`.
- `crates/native/application/src/arch/arch_performance.rs` (`protected_rated`, `protected_of`, `fire_stop_rated`,
  `fire_stop_of`, nesne işlemi).
- `crates/shared/kcad/src/lib.rs`, `decode.rs` (mimari verisi 25).
- `crates/shared/extensions/src/audit_facts/building.rs` (`ShaftFacts.protected`, `ElementFacts.fire_stop`);
  `crates/native/processing/src/audit_facts/building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/dayanim.py` (8 ve 10), `data/surumler.json` (Ek-3'ün metni);
  paket 0.29.0.
- `crates/native/interaction/src/core/properties.rs` (`edit_roles`); `apps/desktop/src/cad/properties/`
  (`Event::Protected`, `Event::FireStop`, `rows/space.rs`, `rows/arch.rs`).

## §5. Sınamalar

- **Ortak durumlar** (`cad.arch.performance`, 116; 17'si yeni): tesisat şaftı korunumlu, asansör kuyusu korunumlu değil
  (belgeyle), null kaldırır; ışıklık, oda, asansör holü `field_not_for_kind`; adsız belge; duvar ve döşeme yangın
  kesici, "değil" de yazılır, null kaldırır, sınıfla birlikte; balkon, kolon, kapı `field_not_for_kind`; kilitli
  katman; adsız belge. Beklenenler `scripts/fixtures/arch_performance_cases.py`'nin ADR'den yazılmış kurallarından.
- **`.kcad`:** `a_protected_shaft_and_a_fire_stop_round_trip_under_arch_data_25`.
- **Python** (`test_dayanim.py`, 46 durum; 15'i yeni, Ek-3/B'nin 8. ve 10. satırlarından elle): korunumlu şaft REI 120
  geçer, EI 120 kalır (R yok), beyansız eksik veri, açık yeri kalır, duvarsız eksik veri; katlarca tek şaftta yalnız 1.
  katın beyanı zemin katın REI 60 duvarlarını da kaldırır; katlarda farklı beyan uzman incelemesi; korunumlu değil ya
  da beyansız şaft kapsam dışı; yangın kesici EI 30 geçer, E 30 kalır, beyansız eksik veri; taşıyıcı R 60 yangın kesici
  10'dan kalır, REI 60 geçer; döşemenin REI 60'ı 10'u da karşılar.
- **Masaüstü** (`shaft_tests`): sekiz katlı büroda Tesisat şaftı Öznitelikler'den korunumlu: olgularda `protected`,
  dört duvarı beyansızken Ek-3/B 8'in eksik veri bulgusu, REI 120 ile kalkar; Ofis ile Hol arasındaki duvar yangın
  kesici: E 30 ile 10'un bulgusu, EI 30 ile kalkar; Hol'de Korunumlu satırı yok. "Yangın kesici" satırı yapı
  elemanının ana satırlarında, Dışarıda'nın altında (`BUILDING_MAIN`). Ortak süzgeçlerle (modül, şerit, geçmiş, katalog,
  ayarlar, Öznitelikler, beyanlar, mevzuat) 257 sınama yeşil.
- **Resimler:** `.run/gui-evidence/korunumlu-saft-8136/` (`shaft_tests::roles_screens`): şaftın ve duvarın
  Öznitelikler'i, Denetim'de 8'in bulgusu; açık ve koyu.

## §6. Kalanlar

- **28/3-a:** yüksek binada çatının oturduğu döşemenin yatay yangın kesici niteliği; çatının altındaki döşemenin
  bulunması ayrı iş (8134 §6).
- Korunumlu şaftın kapağı ve kanalların yangın damperi (87/8), tesisat bacalarının kompartıman hizası (87/11), pis su
  borusunun kat geçişindeki yangın kesici (20/4): kanal, damper ve boru modelde yok.
- IFC: yangın kesicinin ve korunumlu şaftın IFC özelliği yok (`Compartmentation` kompartımanındır); yazılmaz.
- Duvar ve döşeme stillerinin (tiplerinin) yangın kesici beyanı: şimdilik eleman başına.
