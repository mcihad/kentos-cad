# ADR 8094: Taşıyıcı sistem beyanı — geleneksel kesme taş yığma bina, PAİY 22/1-m ve 28/1-d

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8079](8079-emsal-article-22-limits.md) §6 ("22/1-m: kesme taş yığma geleneksel yapının balkon ve taşlıkları"), ADR
  [8085](8085-zone-storey-heights.md) §6 ("28/1-d taşıyıcı sistem beyanıyla").
- **Bağlam:** ADR [8079](8079-emsal-article-22-limits.md) (22. maddenin sınırları tabloda, koşulları pakette, K66–K67),
  [8085](8085-zone-storey-heights.md) (bölge, K93–K97), [8083](8083-dated-storey-height-and-doors.md) (kat yüksekliği
  tarihli), [8069](8069-room-zone-far-model.md) (balkonların %20'si), [8078](8078-regulation-rule-set-by-application-date.md)
  (başvuru gününe göre kural seti), [8090](8090-declared-load-bearing-and-escape-roof.md) (duvarın taşıyıcılık beyanı).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 5/8, 20/8-j, 22/1-m, 28/1-d (RG 9.10.2021/31623; RG 31.12.2022/32060);
  Sivas Belediyesi İmar Yönetmeliği 28/1-d ve 22/k (RG 10.11.2022/32009); Dijital Proje Yönetmeliği EK-2 Tablo 2.5.
- **Web:** dokunulmadı (ADR 0228); TS türleri yeniden üretildi (`ArchSettings`, `BuildingStructure`, `StructuralSystem`).
  **`.kcad`:** belge şeması değişmedi; mimari verisi sürüm 15 (`settings.arch.structure`). **Sözleşme:**
  `ArchSettings.structure`; `cad.settings.set`'in `arch`'ı; `kentos.auditFacts` 1'in `building` bölümüne
  `structuralSystem` ve `traditional`, sınırlı kullanımlara `traditional`.

## §1. Araştırma (17 Kasım 2026)

Metinler bu bilgisayardaki derlenmiş PAİY'den (1.7.2026 derlemesi, `pdftotext`; ADR 8079 §1), değişiklik
yönetmeliklerinin yayımlanmış metinlerinden (alomaliye.com'un 9.10.2021 ve 31.12.2022 sayfaları) ve Sivas'ınki Resmî
Gazete'nin eski sayısından (WebFetch) okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 28/1-d (Ek: RG 9.10.2021/31623, 3 üncü madde; 4 üncü madde: yayımı günü) | Konut bölgelerinde; geleneksel mimari öğeler barındıran ve taşıyıcı sistemi kesme taş yığma bina olarak tasarlanan zemin dâhil 3 katı geçmeyen binalarda zemin ve normal katlarda 4.50 m | K130 |
| PAİY 22/1-m (Ek: RG 9.10.2021/31623, 2 nci madde) | Böyle binalarda taşıyıcı yığma taş iç duvarların 12,50 cm'den, dış duvarların 25 cm'den fazla olan kısımları; düşey taşıyıcıları da bulunabilen, bağımsız bölüm net alanının %20'sini geçmeyen balkonlar; bahçe ile irtibatı olup gerektiğinde basamakla çıkılan, girişle ya da çıkma veya balkon altında yer alabilen, düşey taşıyıcılı üst örtülü taşlık, dış sofa veya eyvan katlar alanına dâhil edilmez | K131, K132; duvarlar K133 |
| RG 31.12.2022/32060, 3 üncü madde; 10 uncu madde: yayımı günü | 22/1-m'den "binalarda, tasarım gereği taşıyıcı yığma taş iç duvarların … dış duvarların genişliği 25 cm'den fazla olan kısımları, bu" ibaresi kaldırıldı | 22/1-m'nin iki hâli: 9.10.2021 (duvarlarıyla), 31.12.2022 |
| RG 31.12.2022/32060, 1 inci madde; PAİY 5/8 | Kalın taş duvarların kısımları 5/8'in listesine girdi: "bu hesaba dâhil edilmeksizin emsal haricidir" (%30'un dışında) | K133 |
| PAİY 20/8-j (Ek: RG 9.10.2021/31623, 1 inci madde) | Taşıyıcı yığma taş dış duvarların tasarım gereği 25 cm'den fazla olan kısımları taban alanına dâhil edilmez | K133 |
| Sivas İmar Yön. 28/1-d | Konut bölgelerinde; geleneksel mimari çizgiler barındıran ve taşıyıcı sistemi kesme taş yığma bina olarak tasarlanan zemin dahil 3 katı geçmeyen binalarda zemin ve normal katlarda 4.50 m | Yerel katman (K130) |
| Sivas İmar Yön. 22/k | Aynı bina için duvarlar, balkonlar ve taşlık, dış sofa, eyvan; "toplamı %30'u geçmemek üzere" | Paket Sivas'ın 22'sini okumuyor (§6) |
| Dijital Proje Yön. EK-2 Tablo 2.5 | Statik disiplininin alt disiplinleri: Betonarme, Çelik, Yığma, Ahşap, Kompozit | Taşıyıcı sistemin seçenekleri; kesme taş yığma 22/1-m ve 28/1-d'nin adıyla ayrı (K129) |
| Ruhsat takımının künyesi (`tasiyici_sistem`, ADR 8037) | Serbest metin ("Betonarme karkas") | Kural serbest metni okumaz; beyan ayrı alan (§6) |
| 22/1-j'nin yorumu (web araması, 17 Kasım 2026) | "Emsal hesabına konu alanın %5'i"nin tabanı için yetkili bir yorum bulunmadı | 22/1-j kaynak bekliyor (§6) |

- **"Konut bölgeleri":** 28/1-c'nin bölgesi de ç'ninki de ("zemin katında ticaret yapılabilen konut bölgeleri") konut
  bölgesidir; d ikisinde de okunur. Bölge yazılmamış konut projesi K96'yla konut alanıdır.
- **"Zemin dâhil 3 kat":** zemin ve normal katlar (±0,00 ve üstündeki katlar, asma kat hariç: 28/1-c'nin 12.8.2023
  sayımı, ADR 8083); bodrum sayılmaz.
- **Taşlık, dış sofa, eyvan:** bendin tarifi bahçe ile irtibatı, basamakla çıkılmayı ve çıkma ya da balkon altında
  olmayı söyler: zemin kattaki alandır. "Sofa" tek başına iç holdür; bentteki "dış sofa"dır.

## §2. Karar

- **K129 — Taşıyıcı sistem projenin beyanıdır** (`ArchSettings.structure`, `BuildingStructure`): sistem (Betonarme,
  Çelik, Yığma, Kesme taş yığma, Ahşap, Kompozit) ve geleneksel mimari öğeler barındırıp barındırmadığı. Geleneksel
  öğeler yalnız kesme taş yığmada okunur; öbüründe `invalid_arch` (`structure.traditional`). Proje ayarları › Mimari ›
  Taşıyıcı sistem'de seçilir; başka sisteme geçince geleneksel öğeler düşer. Beyan projenin bütün binalarınındır (blok
  başına §6).
- **K130 — 28/1-d** (`kat_yuksekligi`, paket 0.18.0): bölgenin bendi c ya da ç, bina beyanlı ve zemin dahil en çok 3
  katlıysa zemin ve normal katlarda 4,50 m (`most`); başvuru 9.10.2021'den önceyse bölgenin kendi bendi. Kural setinde
  28/1-d'nin "bent yok" hâli (1.10.2017–8.10.2021, `absent`) aday günleri daraltmaz. Sivas 28/1-d 4,50 m.
- **K131 — 22/1-m tabloda** (`kentos_domain::zoning`, K66'nın yolu): beyanlı binada adı ya da kullanımı Taşlık, Dış sofa
  ya da Eyvan olan mahal emsal harici (%30 içinde). Kendi alan sınırı yoktur; sınırlı kullanımlarda `traditional`
  (izin verilen = alanı). Balkonları 22/1-l'nin %20'siyle sayılır: 22/1-m'nin payı da %20; düşey taşıyıcılı balkonu
  KentOS l'de de ayırmaz.
- **K132 — 22/1-m'nin koşulları pakette** (`emsal`, K67'nin yolu): bina zemin dahil 3 kattan çoksa, mahal zemin katta (ya
  da onun asma katında) değilse ya da başvuru bendin eklendiği 9.10.2021'den önceyse kaldı
  (`paiy_traditional_not_exempt`); hepsi tutuyorsa bahçeyle bağlantısı çizimden okunmaz, uzman incelemesi. 22/1-m'nin
  satırı yalnız beyanlı binada yazılır; taşlığı yoksa balkonları varsa geçti, yoksa kapsam dışı. Güne bağlı emsal
  bulgusu aday günlerin bulgularının mahallerini seçer (önceden boştu).
- **K133 — Kalın taş duvarlar sonraki dilim:** 20/8-j (taban alanı), 5/8 (31.12.2022'den %30 dışında) ve 22/1-m'nin
  9.10.2021 hâli (%30 içinde) duvarın taşıyıcılık beyanını (ADR 8090), dış ya da iç oluşunu ve 25/12,50 cm'yi aşan payın
  geometrisini ister; emsal zonlarıyla birlikte ayrı yapılır.
- **Dosya:** mimari verisi 15 (`ARCH_WITH_STRUCTURE`).

## §3. Uygulama

- **Sözleşme:** `StructuralSystem` (`ALL`, `label`, `name`), `BuildingStructure` (`traditional_cut_stone`, `problem`),
  `ArchSettings::{has_structure, traditional_cut_stone}`; `cad.settings.set` belgesi ve kataloğu.
- **Dosya:** `kentos_kcad::{ARCH_WITH_STRUCTURE, uses_structure}`, `arch_version`, `MODULE_VERSIONS`; okuyucunun reddi.
- **Alan hesabı:** `ZoningLimit::Traditional`, `limits::says_traditional`, `Room::traditional`; `apply`'ın sınırı
  olmayan kullanımı.
- **Olgular:** `BuildingFacts::{structural_system, traditional}`; SDK'nın `factsSchema`'sı yeniden yazıldı.
- **Paket** (`tr.kentos.mimari-mevzuat` 0.18.0): `kat_yuksekligi` (`GELENEKSEL`, `KONUT_BENTLERI`, `geleneksel`, günün
  bendi), `emsal` (`geleneksel`, `_zemin_mahalleri`); `data/surumler.json`'a PAİY 22/1-m (yok, 9.10.2021, 31.12.2022),
  PAİY 28/1-d (yok, 9.10.2021), Sivas 28/1-d; Denetim'in `emsal` katkısı `paiy_traditional_` kodunu da alır.
- **Masaüstü:** Proje ayarları › Mimari › Taşıyıcı sistem (`structure_view`; `ArchStructure`, `ArchTraditional`).

## §4. Sınamalar

- **Sözleşme:** `the_structure_is_written_by_its_names`. **Dosya:** `the_building_s_structure_round_trips_under_arch_data_15`.
- **Paket** (metinden elle; kural kodundan üretilmedi):
  - `fixtures/standards/v1/kat-yuksekligi.json`, 9 yeni, 44 durum: 4,40 m'lik iki kat 2026'da geçer, 4,60 kalır; 2020
    başvurusunda 28/1-c'nin 3,60'ından kalır; dört katlı binada 28/1-c; geleneksel öğesiz kesme taşta 28/1-c'nin 4,00'ı;
    ticaret alanında 28/1-a; zemin katında ticaretli konut alanında d; Sivas'la ikisi geçer; tarihsiz eksik veri.
  - `fixtures/standards/v1/emsal.json`, 7 yeni, 30 durum: zemin kattaki taşlık uzman incelemesi; dört katlı binada ve 1.
    kattaki eyvanda kaldı; 2020 başvurusunda bent yok, kaldı; yalnız balkonlu binada satır geçti; 2022 başvurusunda
    9.10.2021 hâli; tarihsiz eksik veri (bulgu taşlığı seçer).
- **Masaüstü** (`cut_stone_tests`, `dated_sizes_tests`'in iki katlı evi; Hol'ü Taşlık, katlar 4,40 m, 200 m² parsel,
  emsal 1,00; elle): beyansız 28/1-c'nin 4,00'ından iki kat kalır, katlar alanı 100 m² (Taşlık 40 + WC 10 + yatak odası
  50; ara duvarın cephe köşeleri yarı yarıya); Proje ayarları'nda Kesme taş yığma ve geleneksel öğeler seçilip
  kaydedilince 28/1-d geçer, olgularda taşlık 40 m² `traditional`, katlar alanı 60, emsal harici 40 m²; 22/1-m uzman
  incelemesi; Betonarme seçilince geleneksel öğeler düşer, katlar alanı yine 100. **Resimler:**
  `.run/gui-evidence/kesme-tas-8094/`.

## §5. Arayüz

- Proje ayarları › Mimari › **Taşıyıcı sistem**: "Sistem" seçimi (Yazılmamış, Betonarme, Çelik, Yığma, Kesme
  taş yığma, Ahşap, Kompozit); Kesme taş yığmada "Geleneksel mimari öğeler barındırıyor" anahtarı; açıklaması Denetim'in
  ve Emsal ve TAKS tablosunun ne okuduğunu söyler.
- Denetim: "Taşlık, dış sofa ve eyvan emsal harici (40.00 m²; zemin dahil 2 katlı geleneksel kesme taş yığma bina, zemin
  kat): bahçeyle bağlantısı ve gerektiğinde basamakla çıkıldığı çizimden okunmuyor — PAİY 22/1-m (31.12.2022)".

## §6. Sıradaki adımlar

- **Kalın taş duvarların payı** (K133): 20/8-j, 5/8 ve 22/1-m'nin 9.10.2021 hâli; tablo, TAKS ve emsal zonları — yapıldı: ADR [8095](8095-thick-stone-walls-out-of-emsal.md).
- **Sivas 22/k:** yerel yönetmeliğin kesme taş yığma bendi — duvarları yapıldı: ADR [8096](8096-sivas-22k-thick-walls-within-30.md).
- **Blok başına taşıyıcı sistem:** bina programının binasında (ADR 8072); bugün projenin bütün binalarının — yapıldı:
  ADR [8145](8145-structure-per-building.md).
- **Künye:** ruhsat takımının `tasiyici_sistem` metni boşsa beyanın adıyla dolabilir — yapıldı: ADR
  [8146](8146-kunye-structural-system.md).
- **22/1-j:** "emsal hesabına konu alanın %5'i"nin tabanı (çizimin katlar alanı mı, planın izin verdiği alan mı) kaynak
  bekliyor — yapıldı: ADR [8147](8147-basement-common-rooms-22-1-j.md) (yönetmeliğin 21/2–3'teki kullanımı: katlar
  alanı).
