# ADR 8134: Çatının yanıcılığı — BYKHY 28/2 (BROOF) ve 28/3

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (Kalan: "çatının Broof sınıfı"),
  sıra B15; ADR [8091](8091-material-reaction-to-fire.md) §6 ("Broof (TS EN 13501-5): çatı kaplamasının dış alev
  yayılımı sınıfı ve çatıyla ilgili maddeler").
- **Bağlam:** ADR [8091](8091-material-reaction-to-fire.md) (malzemenin yanıcılık sınıfı, Ek-2/Ç, `cad.arch.performance`'ın
  malzeme işlemi), [8071](8071-geometry-accuracy.md) §8 (çatının katmanları), [8087](8087-shafts-elevators-roofs.md)
  (çatıların Ek-3/B dayanımı, yüksek bina 4/ccc), [8078](8078-regulation-rule-set-by-application-date.md) (kural seti).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 28/2, 28/3 (10.8.2009-2009/15316 sayılı BKK ile değişik,
  RG 9.9.2009/27344) ve Ek-2/Ç'nin çatı satırı; TS EN 13501-5.
- **Sözleşme:** `ArchMaterial.roof_class` (beyan ve dayanağı), `roof_class_problem`; `cad.arch.performance` v1'in malzeme
  işlemine `roofClass` (verilmezse kalır, null kaldırır); `kentos.auditFacts` 1'in `building` bölümüne `roofs`. **`.kcad`:**
  belge şeması değişmedi; mimari verisi 23 (`ARCH_WITH_ROOF_CLASSES`, `settings.arch.materials[].roofClass`). **Web:** TS
  türleri yeniden üretildi (`ArchMaterial`, `PerformanceOperation`); pencere TODOS.md §0.2'de.

## §1. Araştırma (18 Kasım 2026)

Yönetmeliğin metni derlenmiş resmî metinden (pdftotext; ADR 8081'in kopyası) okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 28/2 | "Çatı kaplamalarının BROOF sınıfı malzemelerden, çatı kaplamaları altında yer alan yüzeyin veya yalıtımın en az zor alevlenici malzemelerden olması gerekir. Ancak, çatı kaplaması olarak yanmaz malzemelerin kullanılması durumunda üzerine çatı kaplaması uygulanan yüzeyin en az normal alevlenen malzemelerden olmasına izin verilir." | Kaplama katmanının malzemesinin dış yangın sınıfı BROOF; kaplama tahtası, su ve ısı yalıtımı en az zor alevlenici; istisna yalnız yüzeyin (kaplama tahtası) (K322, K323) |
| BYKHY 28/3 | "Yüksek binalarda ve bitişik nizam yapılarda; a) Çatıların oturdukları döşemelerin yatay yangın kesici niteliğinde, b) Çatı taşıyıcı sistemi ve çatı kaplamalarının yanmaz malzemeden, olması gerekir." | b) yüksek binada mertek ve kaplama yanmaz; nizam çizimde yok (K324). a) döşemenin kendi Ek-3/B dayanımı `dayanim`'da |
| BYKHY 28/4 (RG 20.11.2021) | 21/5'in alanlarında (orman) kaplama, dere, baca kenarı, taşıyıcı hiç yanmaz | Okunmaz: binanın orman alanında olduğu çizimde yok (§6) |
| BYKHY Ek-2/Ç (çatı) | "Çatı kaplamaları için yanıcılık sınıfları — Dış alev yayılımına dayanıklı: BROOF (TS EN 13501-5)"; test edilmeden BROOF sayılan ürünler (arduvazlar, Ek-2/C'ye uyan taş, beton, kil, seramik, çelik kiremitler …) | Sınıf beyandır: kiremit adı sınıf sayılmaz; müellif "BROOF" yazar, dayanağını (ürün beyanı, Ek-2/Ç listesi) söyler |
| BYKHY Ek-2/Ç (döşeme dışı) | Hiç yanmaz A1; zor yanıcı A2-s1,d0; zor alevlenici, normal alevlenici, kolay alevlenici satırları | 8091'in tablosu (`kaplamalar.ozellik`); "yanmaz" tanımlı değil (K325) |
| TS EN 13501-5, sınıflandırma (aktaran [Measurlabs](https://measurlabs.com/blog/en-13501-5-external-fire-exposure-testing-for-roof-coverings/), [Efectis](https://efectis.com/en/?p=81353)) | Dört deney yöntemi (t1 yanan kor, t2 kor ve rüzgâr, t3 ışınımla, t4 iki aşamalı); sınıf deney yöntemiyle yazılır: BROOF(t2) | Yazım: B, C, D, E ya da F, ROOF, ardından (t1)…(t4) ya da hiçbiri (Ek-2/Ç'nin yazdığı gibi yalnız BROOF). Hangi harfin hangi yöntemde verildiği kaynaklarda kesin değil: denetlenmez (K321) |

## §2. Karar

- **K321 — Dış yangın sınıfı beyandır** (`ArchMaterial.roof_class`, `Declared`: değeri ve dayanağı); yoksa eksik.
  `cad.arch.performance`'ın malzeme işlemi `roofClass`'ı dayanımla ve yanıcılık sınıfıyla birlikte ya da ayrı yazar:
  verilmeyen kalır, null kaldırır, hiçbiri verilmezse `nothing_to_set`; okunmayan sınıf `invalid_declared`
  (`operation.roofClass.value`; yanıcılık sınıfı önce okunur). Malzemeler penceresinde "Dış yangın sınıfı" alanı.
- **K322 — 28/2'nin kaplaması:** çatının kaplama katmanının (katmansız çatıda çatının kendi malzemesi: tek kaplama)
  malzemesinin dış yangın sınıfı BROOF ile başlamıyorsa kaldı; malzemesi ya da beyanı yoksa, katmanlarında kaplama yoksa
  eksik veri.
- **K323 — 28/2'nin alttaki katmanları:** kaplamanın altındaki kaplama tahtası (yüzey), su yalıtımı ve ısı yalıtımı en az
  zor alevlenici. Kaplama hiç yanmazsa (A1) tahta normal alevlenici olabilir; kaplama zor yanıcıysa (A2-s1,d0) istisnanın
  tutup tutmadığı uzman incelemesi; kaplamanın sınıfı yoksa tahta normal alevleniciyse eksik veri. Lata ve mertek 28/2'nin
  katmanı değildir.
- **K324 — 28/3-b:** yüksek binada (4/ccc: bina yüksekliği 21,50 m'den, yapı yüksekliği 30,50 m'den fazla) çatının mertek
  katmanı ve kaplaması yanmaz: hiç yanmaz tutar, zor yanıcı uzman incelemesi, öbürü kalır, bilinmeyen eksik veri. Binanın
  nizamı çizimde yok: yüksek olmayan binada yanmaz olmayan çatı uzman incelemesidir (bitişik nizamdaysa kalır); bilinmeyen
  sınıf söylenmez.
- **K325 — "Yanmaz":** yönetmelik terimi tanımlamaz; Ek-2/Ç hiç yanmaz (A1) ve zor yanıcı (A2-s1,d0) der. Hiç yanmaz
  yanmazdır; zor yanıcının yanmaz sayılıp sayılmadığı yazılı değil, sonucu ona bağlı çatı uzman incelemesidir. Değer
  uydurulmaz.
- **K326 — Kural seti:** 28/2 ve 28/3'ün metni 9.9.2009'dan (2009/15316); daha eski başvuruda çatı varsa uzman incelemesi.
  Paket `tr.kentos.mimari-mevzuat` 0.27.0, yeni katkı `catilar` ("Çatıların yanıcılığı (BYKHY 28/2, 28/3)").
- **Dosya:** mimari verisi 23 (`ARCH_WITH_ROOF_CLASSES`); daha küçük sürüm diyen dosyadaki dış yangın sınıfı okunurken
  reddedilir.

## §3. Uygulama

- **Sözleşme:** `materials.rs` (`roof_class`), `performance.rs` (`roof_class_problem`), `cad_arch_performance.rs`
  (`roof_class`).
- **Dosya:** `kentos_kcad::{ARCH_WITH_ROOF_CLASSES, uses_roof_classes}`, `decode.rs`'in ret iletisi.
- **Uygulama:** `arch_performance` malzeme dalı. **Masaüstü:** `materials_window.rs` (`roof_class`, `Event::RoofClass`,
  `declared_of`, alan ve ipucu).
- **Olgular:** `RoofFacts`, `RoofLayerFacts` (katmanın türü dosyanın yazdığı gibi, malzemesi, malzemenin iki sınıfı);
  `audit_facts::building::roofs`.
- **Paket:** `catilar.py` (`_madde_2`, `_madde_3`), kural setine 28/2 ve 28/3 (metinleri ve Ek-2/Ç), `denetim.cati_denetimi`,
  bildirim, değişiklikler.

## §4. Sınamalar

- **Sözleşme:** `a_roof_class_is_a_letter_roof_and_its_test_method` (BROOF, BROOF(t1…t4), CROOF(t3), DROOF(t4), EROOF(t3),
  FROOF(t4) okunur; Broof, AROOF, GROOF, BROOF(t5), BROOF(T1), boşluklu, eksik parantezli okunmaz). **Dosya:**
  `a_material_s_roof_class_round_trips_under_arch_data_23`.
- **Ortak durumlar** (`scripts/fixtures/arch_performance_cases.py`, KentOS kodu olmadan): `cad.arch.performance`'a 8 yeni
  durum (BROOF(t1), belgeyle yalnız BROOF, yanıcılıkla birlikte, kaldırma; Broof, t5, boş, önce yanıcılık sınıfı reddi);
  85 durum geçti.
- **Kural** (`fixtures/standards/v1/catilar.json`, `scripts/fixtures/roof_fire_cases.py`, beklenenler 28/2, 28/3 ve
  Ek-2/Ç'nin metninden elle): 23 bağımsız durum (tam çatı, CROOF, beyansız, malzemesiz, katmansız, yalnız BROOF, A1 kaplamanın
  normal tahtası, yanıcı kaplamanın normal tahtası, zor yanıcı kaplama, EPS yalıtım, su yalıtımı beyansız ve malzemesiz,
  kaplama sınıfsız, lata ve mertek, kaplama katmanı yok, yüksek binada yanmaz, ahşap mertek, mertek sınıfsız, zor yanıcı
  kaplama, yapı yüksekliğiyle yüksek, iki çatı, çatı yok, 2009 öncesi başvuru); ortak yüz (`mimari.denetle`'de iki madde).
  Paketin 17 sınama dosyası yeşil.
- **Masaüstü** (`roof_fire_tests::a_roof_s_covering_and_layers_are_read_by_28_2_and_28_3`): iki katlı çizimde 1. katın
  kırma çatısı (kil kiremit, lata, çam tahta, taş yünü, çam mertek): olgularda beş katman; beyansız 28/2 eksik veri, 28/3
  kapsam dışı; Malzemeler'den kiremit A1 ve BROOF(t1), çam D-s2,d0, taş yünü A1 → 28/2 geçti, 28/3 uzman incelemesi (çam
  mertek, nizam); Broof Uygula'dan önce reddedilir; kiremit CROOF(t3) → 28/2 kaldı. `regulation_dates_tests`'in kural seti
  satırında kapsam dışı 28 → 30 (çatısız binada iki yeni madde). Malzeme, denetim, mevzuat ve ortak süzgeçler yeşil (201).
- **Resimler:** `.run/gui-evidence/cati-yanicilik-8134/` (`roof_fire_tests::screens`): Malzemeler'de kiremidin Yanıcılık
  ve Dış yangın sınıfı; Denetim'de kural seti satırı ve 28/3'ün bulgusu; açık ve koyu.

## §5. Kalanlar

- **28/3-a:** çatının oturduğu döşemenin yatay yangın kesici olması (Ek-3/B 10, EI 30): hangi döşemenin çatının altında
  olduğu olgularda yok.
- **28/4:** binanın orman alanında (21/5) olduğu beyanı gelince.
- **Bitişik nizam:** projenin nizamı İmar durumu'na yazılırsa 28/3 yüksek olmayan binada da kesinleşir (ADR 8091 §6'nın
  26/4'ü de).
- **Yanmaz:** zor yanıcının yanmaz sayılıp sayılmadığı için bakanlığın yorumu ya da sahibin kararı gelince.
