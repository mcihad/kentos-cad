# ADR 8135: Asansör kuyusunun havalandırması — BYKHY 62/4'ün ilk cümlesi

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (Kalan: "62/4'ün kuyu
  havalandırması"), sıra B15.
- **Bağlam:** ADR [8087](8087-shafts-elevators-roofs.md) (şaftlar ve asansörler: 25/3, 41/9, 62/2, 62/4'ün kapıları,
  62/6, 62/7), [8092](8092-basement-lift-protected-corridor.md) (62/4'ün ikinci cümlesi, bodrumdaki korunmuş koridor),
  [8086](8086-compartments-across-storeys.md) (katlar arası açıklıkların yığını), [8068](8068-fire-performance-material-declarations.md)
  ve [8124](8124-b7-remaining-declarations-and-ifc-fields.md) (`cad.arch.performance`, beyan ve dayanağı), [8078](8078-regulation-rule-set-by-application-date.md)
  (kural seti). Kurallar Python'dadır, Rust olguları verir (bim-plan K10).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 62/4 (RG 19.12.2007/26735; fıkranın bu cümlesi
  değişmedi).
- **Sözleşme:** `DeclaredShaftVent` (alan, basınçlandırma, dayanak), `ArchInfo.shaft_vent`; `cad.arch.performance` v1'in
  nesne işlemine `shaftVent` (verilmezse kalır, null kaldırır); `kentos.auditFacts` 1'in `building.shafts[]`'ına `area`,
  `stack`, `vent`. **`.kcad`:** belge şeması değişmedi; mimari verisi 24 (`ARCH_WITH_SHAFT_VENTS`,
  `entities[].arch.shaftVent`). **Web:** TS türleri yeniden üretildi (`DeclaredShaftVent`, `ArchInfo`,
  `PerformanceOperation`); Öznitelikler'in satırları TODOS.md §0.2'de.

## §1. Sorun

62/4'ün kapıları (8087) ve bodrum cümlesi (8092) okunuyordu; ilk cümlesi, asansör kuyusunun havalandırma ve dumandan
arındırma bacası ya da basınçlandırılması, okunmuyordu. Kuyunun bacası planda çizilmez (kuyunun tepesinde, çatıda ya da
cephededir), basınçlandırma bir tesisat sistemidir: ikisi de müellifin beyanıdır. Ölçülebilen yarısı kuyunun alanıdır.

## §2. Araştırma (18 Kasım 2026)

Yönetmeliğin metni derlenmiş resmî metinden (pdftotext; ADR 8081'in kopyası) okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 62/4, ilk cümle | "Asansör kuyusunda en az 0.1 m2 olmak üzere kuyu alanının 0.025 katı kadar bir havalandırma ve dumandan arındırma bacası bulundurulur veya kuyular basınçlandırılır." Fıkrada değişiklik işareti yalnız 2015/7401 sayılı BKK'nın otopark cümlesinde | İstenen: en büyüğü 0,1 m² ile kuyu alanının 0,025 katı; baca ya da basınçlandırma (K329). Sayılar kural setinde (`ventLeast`, `ventShare`) |
| BYKHY 62/4, ikinci ve üçüncü cümle | Bodruma da inen asansöre bodrumda korunmuş koridordan ya da yangın güvenlik holünden ulaşılır; asansör kapıları kullanım alanına açılamaz | Değişmedi (8087, 8092); maddenin satırı üç cümlenin sonucudur (K330) |
| BYKHY 160/3 | Mevcut yapılarda aynı kural ("bulundurulması veya kuyuların basınçlandırılması gerekir") | Okunmaz: KentOS'un denetimi yeni yapının ruhsat projesidir (§6) |
| BYKHY 62/3 | Aynı kuyuda en çok 3 kabin; 4 kabinde ikişerli gruplar 60 dakikayla ayrılır | Okunmaz: kabin sayısı kuyunun mahalinde yok (§6) |
| EN 81-1:1998 5.2.3, notu (aktaran S. Russett, "Elevator shaft natural ventilation", CIBSE, [PDF](https://www.cibse.org/media/egyfesfi/elevator-shaft-ventilation-sr-summer-08.pdf)) | Kuyu uygun biçimde havalandırılır, başka mahalleri havalandırmaz; ilgili yönetmelik yoksa kuyunun tepesinde yatay kesitin en az %1'i kadar açıklık önerilir. Yazı, İngiltere yönetmeliğinin bu havalandırmayı bilerek kaldırdığını aktarır | Türkiye'de ilgili yönetmelik vardır (62/4: %2,5 ve en az 0,1 m²); standardın önerisi okunmaz |
| EN 81-20:2014 (EN 81-1'in yerini aldı; [KONE bilgi notu](https://www.kone.com.au/Images/pdf_Safety%20Standard%20EN81-20%20Fact%20Sheet_tcm46-29613.pdf)) | Kuyunun ve makine yerinin havalandırması ekipmanın ısısına göre | Alan oranı vermez; kural setine girmez |
| Basınçlandırma (TS EN 12101-6, basınç farkı sistemleri) | Sistemin tasarımı ve deneyi standarttadır | Çizimde yok: basınçlandırıldığı beyan edilir, tasarımı uzmanındır (§6) |

## §3. Karar

- **K327 — Beyan kuyunun mahalindedir:** asansör kuyusu olan mahal (boşluk ya da değil; adı ya da kullanımı Asansör, holü,
  önü, lobisi, sahanlığı ya da bekleme yeri değil: `room_names::shaft_of`) `ArchInfo.shaft_vent` taşır:
  bacanın serbest alanı (m², 0'dan büyük, en çok 100), basınçlandırıldığı ya da ikisi, ve dayanağı (müellif ya da belge).
  Kuyunun bir katındaki mahali bütün kuyu için söyler. `cad.arch.performance`'ın nesne işlemi `shaftVent`'i yazar:
  verilmeyen kalır, null kaldırır; başka nesneye `field_not_for_kind`; ne alan ne basınçlandırma, ya da sınır dışı alan
  `invalid_declared` (`operation.shaftVent.area`), dayanak `operation.shaftVent.basis`. Basınçlandırılmaz (false)
  yazılmaz. `.kcad` mimari verisi 24; 24'ten önceki okuyucu böyle bir belgeyi reddeder.
- **K328 — Katlarca tek kuyu:** ardışık katlardaki aynı türden iki şaft, birinin ortası öbürünün dış çizgisindeyse, tek
  kuyudur (8086'nın yığını gibi); adı en alttaki mahalin kalıcı kimliğidir (`Enclosure.stack`, olguların
  `shafts[].stack`'i; katsız şaft kendi başınadır). Kuyunun alanı mahallerinin dış çizgisinin alanıdır (kuyu duvar
  yüzlerine çizilir); kural katlarındaki en büyüğünü okur.
- **K329 — Kural (`kuyular.havalandirma`, `bykhy_elevator_shaft_vent`):** istenen = max(0,1; 0,025 × kuyu alanı).
  Basınçlandırılan kuyu geçer; bacası istenene eşit ya da büyükse geçer (karşılaştırma 1e-9 m² kayan nokta payıyla);
  küçükse kaldı. Beyan yoksa ya da kuyunun alanı ölçülemediyse (0) eksik veri; kuyunun katlarında iki farklı beyan
  varsa uzman incelemesi. Bir kuyuya tek bulgu, bütün katlarındaki mahalleriyle: "“Asansör”: havalandırma bacası 0.2 m²
  (kuyu alanı 10.64 m², en az 0.266 m² baca); 62/4 …".
- **K330 — 62/4'ün kapsamı ve satırı:** asansör kuyusu ya da asansör kapısı varsa madde kapsamdadır (kapısız kuyu da);
  satır üç cümlenin bulgularından (`_sonuc`): biri kaldıysa kaldı, sonra eksik veri, sonra uzman incelemesi. Maddenin
  başlığı "Asansör kuyusunun havalandırması ve kapıları"; kural setindeki metni üç cümlenin tamamı. 2015'in otopark
  cümlesi okunmaz (8087).
- **K331 — Öznitelikler:** asansör kuyusu mahalinde "Baca alanı" (havalandırma ve dumandan arındırma bacasının serbest
  alanı, m²; boş bacayı kaldırır, basınçlandırma kalır; "Havalandırma bacası" etiket sütununa sığmadı) ve
  "Basınçlandırılır" (Evet, Hayır; bacanın alanı kalır; ikisi de yoksa beyan kalkar), altında iki not (panelin notu iki
  satırdır): "Kuyu bu katta 10.64 m²; bir katta beyan yeter." ve "62/4: alanın 0,025 katı, en az 0,1 m²." İstenen değer
  panelde hesaplanmaz: hesap kuralındır (K10). Panelin dayanağı müellif beyanıdır; belge dayanağı
  komutla (Python, MCP) verilir.

## §4. Uygulama

- `crates/shared/contracts/src/arch/performance.rs` (`DeclaredShaftVent`, `MAX_SHAFT_VENT`, `problem`),
  `arch/arch.rs` (`ArchInfo.shaft_vent`); `crates/shared/commands/src/arch/cad_arch_performance.rs` (`shaftVent`),
  `cad_arch_audit.rs` ve `catalog/arch_project.rs` (bulgu kodu).
- `crates/native/application/src/arch/arch_performance.rs` (`shaft_vent_rated`, `shaft_vent_of`, nesne işlemi).
- `crates/shared/kcad/src/lib.rs`, `decode.rs` (mimari verisi 24).
- `crates/native/domain/src/separations.rs` (`Enclosure.area`, `stack`; `stack_shafts`);
  `crates/shared/extensions/src/audit_facts/building.rs` (`ShaftFacts.area`, `stack`, `vent`; `ShaftVentFacts`);
  `crates/native/processing/src/audit_facts/building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/kuyular.py` (`havalandirma`, `onu`), `data/surumler.json`
  (62/4'ün metni, başlığı, `ventLeast`, `ventShare`); paket 0.28.0.
- `crates/native/interaction/src/core/properties.rs` (`edit_shaft_vent`); `apps/desktop/src/cad/properties/`
  (`ArchField::ShaftVent`, `Event::ShaftPressurized`, `rows/space.rs`).

## §5. Sınamalar

- **Ortak durumlar** (`cad.arch.performance`, 99; 14'ü yeni): asansör kuyusuna 0,12 m², belgeyle basınçlandırma, ikisi,
  basınçlandırılmaz demek yazılmaz, null kaldırır; hole, odaya, duvara `field_not_for_kind`; kuyuya yangın sınıfı
  önce okunur; ne alan ne basınçlandırma, 0, 120 m², basınçlandırılsa da −0,1, adsız belge `invalid_declared`.
  Beklenenler `scripts/fixtures/arch_performance_cases.py`'nin ADR'den yazılmış kurallarından.
- **`.kcad`:** `a_lift_shaft_s_vent_round_trips_under_arch_data_24` (yazılır, okunur; 23'ün okuyucusu reddeder).
- **Olgular:** `fixtures/audit-facts/v1/mimari.json` yeniden yazıldı; fark yalnız asansör boşluğunun alanı (6.365 m² =
  1.90 × 3.35, duvar boyları 1.70 ve 3.15'in köşeleriyle) ve yığını (kendisi). SDK'nın Python sınamaları (mimari ve proje
  olguları) geçer.
- **Python** (`test_kuyular.py`, 42 durum; 11'i yeni, elle): 4,4 m² kuyuya 0,11 m² geçer; 6 m²'ye 0,12 kalır (0,15
  ister); 2,4 m²'ye 0,08 kalır, tam 0,1 geçer (en az 0,1); basınçlandırılan geçer; beyansız eksik veri; üç katlık
  kuyuda tek beyan bütün kuyuya geçer ve kuyunun en büyük alanı okunur (4,8 m²: 0,11 kalır); katlarda farklı beyan uzman
  incelemesi; iki ayrı kuyudan beyansızı tek bulgu; alanı ölçülemeyen eksik veri. Eski 13 asansörlü durum 4 m² kuyuya
  0,12 m² baca beyanıyla aynı sonucu verir; kapısız kuyunun 62/4'ü kapsam dışından geçti'ye döndü.
- **Masaüstü** (`shaft_tests`): olgularda kuyunun alanı 10.64 m², yığını kendisi, beyanı yok; sekiz katlı büroda
  beyansız 62/4 eksik veri, 0,27 m² ile geçer; üç katlık kuyuda 1. katın mahalinde Öznitelikler'den 0.2 yazılınca tek
  bulgu üç mahalle kaldı, basınçlandırılınca geçer, alan silinince basınçlandırma kalır, o da kalkınca beyan yok; Hol'de
  satır yok. Bodrumdaki asansör (8092) basınçlandırılmış kurulur: sınaması koridorundur. `regulation_dates_tests`'in
  satırında kapısız kuyuların 62/4'ü eksik veri. Ortak süzgeçlerle (modül, şerit, geçmiş, katalog, ayarlar,
  Öznitelikler) 247 sınama yeşil; `regulation_audit_tests` ve `declared_readings_tests` de.
- **Resimler:** `.run/gui-evidence/kuyu-havalandirmasi-8135/` (`shaft_tests::vent_screens`): 1. katın kuyusunun
  Öznitelikler'i, Denetim'de 62/4'ün bulgusu; açık ve koyu.

## §6. Kalanlar

- Bacanın nereye açıldığı (dış hava, dumansız yer) ve basınçlandırma sisteminin tasarımı (TS EN 12101-6) çizimde yok:
  beyanın dayanağı ve uzman incelemesi.
- 62/3 (bir kuyuda en çok üç kabin) ve 62/2'nin makine dairesi: kabin sayısı ve makine dairesi mahalde yok.
- 160/3 (mevcut yapılar): mevcut yapı denetimi KentOS'un kapsamında değil.
