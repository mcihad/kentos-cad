# ADR 8124: B7'nin kalanları — pencerelerde beyan, türetilenin gerekçesi, acil çıkış önerisi, IFC-02'nin son alanları

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B7 (BIM-07),
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-02. Önceki dilimler: ADR [8068](8068-fire-performance-material-declarations.md)
  §8 (pencerelerde alan yok, dayanak seçilmiyor, türetilenin açıklaması yok), [8076](8076-ifc-mandatory-fields-complete.md)
  §7 (Tag politikası, ad ve soyad, ölçüm anotasyonu, baskın kotu olmayan kat, çatı altındaki kat, duvar kaplaması).
- **Bağlam:** [8035](8035-fire-escape-routes-distances-load-exits.md) (kaçış yolları), [8090](8090-declared-load-bearing-and-escape-roof.md)
  (beyan edilen roller), [8091](8091-material-reaction-to-fire.md) (malzemenin yanıcılık sınıfı),
  [8082](8082-fire-resistance-of-elements.md) (Ek-3 olguları), [8123](8123-far-zone-under-the-roof.md) (çatı altındaki zon).
- **Planın listesi eskimişti:** BIM-07'nin "Kalan"ındaki asansör kapasitesi ve kaplama kalınlıkları ADR
  [8070](8070-missing-building-elements.md) §5 (11) ve §6 (K44) ile, duvar kaplaması ADR 8076 K57 ile, formun 9. sorusu ADR
  8076 §5 ile yapılmıştı. Bu dilim kalanları bitirir.
- **`.kcad`:** mimari verisi 20 (`ARCH_WITH_DECLARED_READINGS`). **Sözleşme:** `ArchInfo.external`, `DeclaredExternal`,
  `DeclaredBasis::read`; `cad.arch.performance` v1'e `objects.external` ve `exits` işlemi; hata kodu `uid_in_both_lists`;
  `kentos.auditFacts` 1'in CAD olgularına `exits`. **Paket:** dijital-proje 0.12.0 (`cikis`). **Web:** TS türleri yeniden
  üretildi; web durduğu için arayüzü yok (TODOS.md §0.2).

## §1. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Graphisoft, Archicad 28 Yardım, "Archicad Classifications and Categories in IFC" | Konum (dış, iç, tanımsız) ve taşıyıcı işlev kullanıcının verdiği kategorilerdir; IFC'nin dışarıda ve taşıyıcı özelliğine çıkar; içe aktarmada dosyanınki kategori olur | KentOS bunları çizimden okur (katın dış çizgisi, döşemenin rolü, çerçevenin kolonu) ve gerekçesini söyler; okuma elemanda beyanla değişir (K274, K275) |
| buildingSMART Forum, "TAG. What does it mean and what is it for?" | Tag elemanın örnek düzeyindeki kimliğidir (seri ya da konum numarası); yazılımlar onu tutarsız doldurur, kimi kullanıcının verdiği numarayı (eleman kimliği, örnek işareti) kimi uygulamanın anahtarını yazar | Önce kullanıcının numarası, yoksa tipin kodu, yoksa kalıcı kimlik (K278) |
| IFC 4.3 `Pset_DoorCommon.FireExit`, `Pset_StairCommon.FireExit` | Nesnenin yangında çıkış olarak tasarlanıp tasarlanmadığı (evet, hayır); ülkenin yapı yönetmeliğine göre | Beyan müellifindir; KentOS'un kaçış yolları önerir, uyuşmazlığı söyler (K276) |
| BYKHY 4/u; ADR 8035 | Kaçış uzaklığı en yakın kat çıkışına kadardır; KentOS'un yolları kaçış merdiveninin mahalini, açık merdiveni ve ±0,00 ve altında dışarıya açılan kapıyı çıkış sayar | Önerinin gerekçesi bu dört okumadır |
| EK-7 Tablo 7.2 (PDF s. 223) | `FamilyName`, `GivenName`; not: soyad ismin ilk ya da son kısmında olabilir | Soyadı künyede ayrı alan; adı soyadının başında ya da sonunda aranır (K279) |
| EK-6 Tablo 6.8 (PDF s. 45–46) | `ElevationOfSSLRelative`: "Seviye değişkenlik gösteriyorsa ve belirgin biri baskın kot bulunmuyorsa, bu özellik girilmeyebilir" | O katta sorulmaz (K280) |
| EK-6 Tablo 6.58 (PDF s. 98–99) | Duvar kaplama kalınlığı mahalin `Pset_SpaceCoveringRequirements`'inde ya da `IfcCovering` ile | Eğri duvarın yüzü de okunur (K281) |
| EK-7 Tablo 7.9 (PDF s. 233); IFC 4.3 `IfcAnnotationTypeEnum.SURVEY` | Anotasyon ölçüm/nirengi noktalarını, çizgilerini, eş yükselti eğrilerini kapsar; SURVEY "survey points, survey lines or survey areas"; Name, Description, PredefinedType | Projenin kabul edilmiş ölçüm noktaları sahada SURVEY noktasıdır; ölçüm yoksa sınıf yok, satır sorulmaz (K282) |

## §2. Karar

- **K272 — Pencerelerde beyan:** Kapı ve pencere tipleri penceresinin tip formunda "Yangın dayanım sınıfı" ve "Dayanak
  belgesi". Uygula tipin değişikliğinden sonra beyanı `cad.arch.performance` `openingType` ile yazar (kendi adımı "Yangın
  ve performans", Malzemeler'deki gibi, ADR 8091); okunmayan dayanak Uygula'dan önce söylenir. Proje ayarları › Mimari ›
  Yangın güvenliği'nde binanın "Yangın koruma sınıfı" ve "Dayanak belgesi"; Kaydet onu `cad.settings.set`'in mimari
  ayarıyla yazar (tek adım). Malzemenin dayanımı ve yanıcılık sınıfı Malzemeler'de vardı (ADR 8091).
- **K273 — Dayanak tek satır:** boş ya da "müellif beyanı" müellif beyanıdır; öbürü belgenin adı, günü sonunda parantezde
  ("Test raporu 41 (2026-02-28)"). Gün biçimindeki parantez gerçek bir gün olmalıdır, başka parantez adın parçasıdır
  (`DeclaredBasis::read`, `field`). Öznitelikler'de nesnenin kendi sınıfının altında "Dayanak belgesi"; sınıfın değeri
  değişince dayanağı kalır. Rollerin (acil çıkış, taşıyıcı, dışarıda) dayanağı arayüzde müellif beyanıdır; belge
  komutla (Python, MCP) verilir.
- **K274 — Türetilenin gerekçesi:** Öznitelikler'de "Dışarıda" ve kolonun "Taşıyıcı" çizimin okumasını gerekçesiyle
  söyler: duvar katın dış çizgisinde olduğu için dışarıda ya da değmediği için içeride (Ek-3 olgularının ve IFC'nin aynı
  okuması: katın duvar ve kolon gövdelerinin dış çizgisi, `Document::fire_elements`); döşeme kat döşemesi olduğu için
  içeride, balkon ya da teras döşemesi olduğu için dışarıda; çatı hep dışarıda; kapı ve pencere duvarı gibi; kolon
  çerçevenin kolonu olduğu için taşıyıcı, dikme taşıyıcı değil (`Document::outside_reading`, `bearing_reading`).
- **K275 — Örnekte değiştirme:** duvarın ve kaplama olmayan döşemenin `arch.external`'ı (evet, hayır, dayanağı),
  dikme olmayan kolonun `arch.loadBearing`'i. Beyan çizimin okumasını geçer: IFC'de `IsExternal` ve
  `Pset_ColumnCommon.LoadBearing`, Ek-3 olgularında dış duvar ve çerçeve. Kapı ve pencere duvarını izler; çatıya ve
  açıklığa dışarıda beyanı verilmez. Seçimde "Kendiliğinden (Evet)" beyanı kaldırır.
- **K276 — Acil çıkış önerisi:** kaçış yolları kat çıkışı saydığı kapıyı ve merdiveni nedeniyle verir
  (`escape::ExitWhy`: dışarıya açılan kapı, kaçış merdiveninin mahaline açılan kapı, açık merdiven, kaçış merdiveninin
  mahalindeki merdiven). Çizimin olgusu `cad.exits`: her kapı ve merdiven, beyanı ve yolun okuması. Kural (dijital-proje
  0.12.0, `cikis`): beyanı olmayanlar için tek bilgi bulgusu, adaylar adıyla ve nedeniyle; düzeltmesi önerinin hepsini
  tek adımda yazar. Beyan yolla uyuşmuyorsa nesne başına uyarı, düzeltmesi önerinin yönünde. Yolların görmediği bir
  çıkış olabilir (dış merdiven, çizilmemiş mahal): karar müellifindir.
- **K277 — Komut:** `cad.arch.performance` v1'in `exits` işlemi (`yes`, `no`: her biri müellif beyanı; ikisi de boşsa
  `no_entities`, bir nesne ikisinde `uid_in_both_lists`, kapı ve merdivenden başkası `field_not_for_kind`, kilitli katman
  `layer_locked`); `objects`'e `external`; taşıyıcılık kolonda da. Ortak durumlar 52'den 77'ye. "Kolona taşıyıcılık"
  durumu artık reddedilmez, yazılır (K275'in sonucu).
- **K278 — Tag:** elemanın kendi numarası (kapının ve pencerenin numarası, kolonun ve kirişin etiketi), yoksa tipinin
  doğrama kodu (kapı ve pencere tipi), yoksa KentOS kimliği.
- **K279 — Ad ve soyad:** künyede "Mimarın soyadı (IFC modelinde)" ayrı alan (`mimar_soyadi`, kişisel bilgi). Soyadı
  "Mimar (müellif) adı soyadı"nın sonunda ya da başında aranır (büyük küçük harf ayrılmaz); ad kalan kelimelerdir. Soyadı
  yoksa ya da adı soyadında bulunmazsa ikisi de yazılmaz: son kelimeyi soyadı saymak tahmindi. Form künyenin kutusunu
  söyler.
- **K280 — Girilmeyebilen satırlar:** katın iki ya da daha çok bilinen taşıyıcı döşeme kotu varsa ve hiçbiri baskın
  değilse o katın `ElevationOfSSLRelative`'i sorulmaz. Mahalleri bir çatının altına yükselen katın yüksekliği sabit
  değildir: brüt ve net yüksekliği sorulmaz, binanın `Height` ve `EavesHeight`'ı koşulla ayrılır. Nedeni yazılan
  modelin katındadır (`Storey::unentered`, dosyaya yazılmaz); okunan dosyada neden bilinmez, eskisi gibi eksik sayılır.
- **K281 — Eğri duvarın kaplaması:** mahalin yay kenarı bir duvarın yay yüzünde ise (aynı merkez, yüzün yarıçapı: yolun
  yarıçapı yüzün uzaklığı kadar, yay saat yönünün tersine dönüyorsa sol yüz içeride; kenarın ortası yayın içinde) o yüzün
  kaplaması okunur. Duvardan ayrı çizilmiş mahal (kenarları yüzlerde değil) değer almaz; onarım mahali duvarlarından
  yeniden bulmayı söyler. Tolerans büyütülmedi (1 µm).
- **K282 — Ölçüm anotasyonu:** EK-7 Tablo 7.9 projede kabul edilmiş ölçüm noktası varken uygulanır (`mapSurvey.points`).
  Her nokta sahada bir `IfcAnnotation` SURVEY'dir: adı işareti, açıklaması kaynağı (noktanın kimliği, sonucun sürümü,
  yöntemi, çözümleri), yeri nokta nesnesinin çizimdeki yeri, kotu sonucun kotundan ±0,00'ın mutlak kotu çıkarılarak.
  ±0,00'ın kotu ya da noktanın kotu bilinmiyorsa nokta yazılmaz (kot tahmin edilmez). Ölçüm yoksa sınıf modelde yoktur,
  satırları sorulmaz (K51: IDS'te en az sayısı 0).

- **K283 — Ek-3'ün dış duvarı çerçeveli katta da:** Öznitelikler'in gerekçesini yazarken bulundu: alanın okuması
  (`Document::fire_elements`, Ek-3 olguları) kesit binasının bütün dış duvarlarını "içeride" okuyordu; IFC modeli dışarıda
  yazıyordu. Neden: birleştirilmemiş duvar gövdesinin uçları köşe kolonlarının içinde kalır, köşeleri dış çizgide değildir.
  Şimdi duvarın kenarlarının ortası da dış çizgiyle sınanır (aynı 0,1 mm). Kesit binasında modelin ve alanın okuması duvar
  duvar aynı; Ek-3'ün dış duvar satırları (5, not (3)) çerçeveli binada artık okunur.

## §3. Uygulama

- **Sözleşme:** `crates/shared/contracts/src/arch/performance.rs` (`DeclaredExternal`, `DeclaredBasis::{read, field}`),
  `arch.rs` (`ArchInfo.external`), `core/codes.rs` (`UID_IN_BOTH_LISTS`), `core/particulars.rs` (`mimar_soyadi` kişisel);
  `crates/shared/commands/src/arch/cad_arch_performance.rs`, `catalog/arch_project.rs` (özet, örnek).
- **Dosya:** `kentos_kcad::{ARCH_WITH_DECLARED_READINGS, uses_declared_readings}`, `decode.rs`.
- **Uygulama:** `arch_performance` (`external_rated`, `external_of`, kolonun taşıyıcılığı, `exits`), `arch_ifc::surveys`.
- **Alan:** `kentos_domain::derived_roles` (okuma ve gerekçe), `fire_elements` (beyan, `outline`, kenar ortaları), `escape`
  (`ExitWhy`, `exits_by`), `room_walls` (yay kenarı).
- **IFC:** `kentos_ifc::arch` (beyan edilen dışarıda ve taşıyıcı, `tag_of`, tipin kodu, `Particulars::names`, katın
  `unentered`'i), `building.rs` (`SurveyPoint`, `IFCANNOTATION`, `Storey::unentered`), `required.rs` (`Carrier::unentered`).
- **Olgular:** `kentos_processing::audit_facts::cad::exits`, `kentos_extensions::audit_facts::cad::CadExit`.
- **Masaüstü:** Öznitelikler (`rows/arch.rs` `performance_rows`, `Event::External`, `ArchField::FireBasis`),
  `opening_types_window.rs`, `shell/project/arch.rs` (Yangın güvenliği), künye (`permit_set.rs`).
- **Paket:** `catalog/standards/dijital-proje` 0.12.0: `cikis.py` (`mimari.denetle` ve eklenti denetimi `cikis`),
  `zorunlu.py` (onarımların yeri), `form.py` (künyenin soyadı kutusu).

## §4. Sınamalar

- **Komut durumları** (`fixtures/commands/v1/cad.arch.performance.json`, 77; `scripts/fixtures/arch_performance_cases.py`
  ADR'den, KentOS kodu olmadan): duvar içeride, döşeme belgesiyle dışarıda, null okumaya döner, kapıya ve çatıya
  dışarıda ret, dikmeye taşıyıcılık ret, kolon taşıyıcı değil yazılır; `exits`: evet ve hayır birlikte, yalnız hayır,
  hayırı evete çevirir, boş, iki listede, pencere, çatı, kilitli katman, olmayan kapı.
- **Sözleşme, dosya, IFC, alan:** `a_basis_reads_from_one_line`; `the_declared_readings_round_trip_under_arch_data_20`;
  `the_architect_s_names_come_from_the_family_name_given_apart`,
  `a_storey_s_varying_level_and_its_height_under_a_roof_are_not_asked`; `walls_and_slabs_read_by_the_drawing_and_say_why`,
  `a_room_along_a_curved_wall_sees_its_face_s_covering`.
- **Uygulama** (`arch_ifc`): `a_declared_reading_is_written_over_the_drawing_s` (duvar içeride, penceresi onu izler,
  döşeme dışarıda, kolon taşıyıcı değil; Ek-3 olguları aynı; gerekçe), `an_element_s_tag_is_its_number_else_its_type_s_code_else_its_id`,
  `an_accepted_survey_point_is_a_survey_annotation_of_the_site`.
- **Paket:** `catalog/standards/tests/dijital-proje/test_cikis.py` (5), `test_zorunlu.py`.
- **Olguların başvurusu** (`fixtures/audit-facts/v1/mimari.json`, `KENTOS_WRITE_SDK=1`): fark okundu. Yeni olan `cad.exits`
  (kapı kendi duvarının katında, kimliğe göre sıralı: çizimin sırası olgu değildir). Dosyanın kendi nesnelerinin (proje,
  saha, bina, katlar) eksik satırlarındaki kimlikler `""` yerine türetilmiş kimlikler: ADR 8117'den beri böyleydi, başvuru
  o zaman yeniden yazılmamıştı (sınaması o günden beri düşüyordu). Olguların şeması
  (`crates/shared/extensions/sdk/kentos-audit-facts.schema.json`) yeniden yazıldı.
- **Kesit binasında model ve alan** (`a_declared_reading_is_written_over_the_drawing_s`): her duvarın IFC'deki
  `IsExternal`'ı alanın okumasıyla aynı (K283).
- **Masaüstü** (`declared_readings_tests`): Öznitelikler'de duvarın okuması ve gerekçesi, beyanla değişmesi, penceresinin
  duvarını izlemesi, kolonun taşıyıcılığı, döşeme; dayanağın tek satırı ve sınıf değişince kalması, gün olmayan tarihin
  reddi; Kapı ve pencere tipleri'nden tipin sınıfı ve dayanağı (bir adım), okunmayan dayanağın Uygula'dan önce
  söylenmesi; Proje ayarları'ndan binanın sınıfı; Denetim'in acil çıkış önerisi ve düzeltmesinden sonra öneri kalmaması.
- **Resimler:** `.run/gui-evidence/b7-8124/`.

## §5. Kalanlar

- Gerçek pencerede kabul (B6'daki gibi) yapılmadı; resimler uygulamanın görünüm ağacından.
- Python SDK'sı katalogdan yeniden yazıldı (`scripts/python/sdk.py --check` temiz); yerel modülle `pnpm py:test`
  koşturulmadı. MCP araçları aynı katalogdan gelir; MCP sınamaları koşturulmadı.
- Acil çıkış önerisi Denetim'dedir; Öznitelikler nesne başına önermez (kaçış yolları her seçimde hesaplanmaz).
- IDS dosyası `ElevationOfSSLRelative`'in "girilmeyebilir" koşulunu söyleyemez; IfcTester baskın kotu olmayan katta eksik
  der (ADR 8076'nın bina yükseklikleri gibi).
- Ölçüm çizgileri ve alanları (yalnız noktalar); ölçüm arşivi harita projesinindir, mimari projeye noktanın gelmesi
  harita işidir.
