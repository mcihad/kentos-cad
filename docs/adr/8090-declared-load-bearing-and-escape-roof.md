# ADR 8090: Beyan edilen roller — taşıyıcı duvar (Ek-3/B 2) ve kaçış yolu olan çatı (4-a)

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, BIM-07; ADR
  [8082](8082-fire-resistance-of-elements.md) §6 ("Taşıyıcı duvar (Ek-3/B 2): duvarın taşıyıcı olduğu beyanı modelde
  yok; not (3)'ün uzman incelemesini de kapatır"; "Çatılar: 4-a kaçış yolu olan bölüm REI 30").
- **Bağlam:** ADR [8068](8068-fire-performance-material-declarations.md) (beyanlar: sınıf ve acil çıkış,
  `cad.arch.performance`), [8082](8082-fire-resistance-of-elements.md) (Ek-3), [8087](8087-shafts-elevators-roofs.md)
  (çatılar 4-b, 4-c), [8076](8076-ifc-mandatory-fields-complete.md) (IFC'nin zorunlu ve isteğe bağlı alanları).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik Ek-3/B 2, 4-a, 5, 6, 7 ve Ek-3/C not (3); Mimarlık ve
  Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik EK-6 Tablo 6.133 (`Pset_WallCommon.LoadBearing`,
  isteğe bağlı).
- **Web:** TS türleri yeniden üretildi (`ArchInfo`, `DeclaredBearing`, `PerformanceOperation`). **`.kcad`:** belge şeması
  değişmedi; mimari verisi sürüm 13 (`arch.loadBearing`, çatının `arch.exit`'i). **Sözleşme:** `ArchInfo.loadBearing`,
  `DeclaredBearing`; `cad.arch.performance` v1'in `objects` işlemine `loadBearing`, `exit` çatıda da;
  `kentos.auditFacts` 1'in öğelerine `loadBearing`, `escape`.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Ek-3/B 2 | Yük taşıyıcı duvar (aşağıdaki maddelerde de açıklanmayan duvar): R, Ek-3/C kadar; ayrı ayrı her bir yüzey | Taşıyıcı beyan edilen iç duvar R; dış, kompartıman ve korunumlu merdiven duvarı kendi satırıyla |
| Ek-3/B 5, 6, 7 | Dış duvar ve kompartıman duvarı REI, Ek-3/C kadar (kompartımanda en az 60) | R yalnız taşıyıcı duvarda anlamlıdır: taşıyıcıysa REI, değilse EI (ADR 8082'nin EI'si beyan yokken kalır) |
| Ek-3/C not (3) | Taşıyıcı sistemin parçası olmayan eleman 120'den 90'a | Taşıyıcı olmadığı beyan edilen dış duvarda 90 istenir; beyan yokken 90–120 arası uzman incelemesi (ADR 8082) |
| Ek-3/B 4-a, 4-c | Kaçış yolu teşkil eden her bölüm REI 30 (alt yüzeyden); dıştan yangına maruz kalan çatı (yük taşıyıcı değil) EI, Ek-3/C | Kaçış yolu olan çatı yük (insan) taşır: 4-c değil 4-a, sabit REI 30 |
| EK-6 Tablo 6.133 | `Pset_WallCommon.LoadBearing` (Taşıyıcı), isteğe bağlı | Beyan edilmişse yazılır; beyan yoksa yazılmaz (tahmin edilmez) |
| ADR 8068 | Acil çıkış rolü (`DeclaredExit`, EK-6 `FireExit`) kapıda ve merdivende; beyanın dayanağı (müellif, belge) | Aynı kip: taşıyıcılık `DeclaredBearing` (evet, hayır, dayanağı); çatının kaçış yolu rolü `exit` (Öznitelikler'de "Kaçış yolu") |

## §2. Karar

- **K116 — Taşıyıcılık beyanı:** duvarın (giydirme cephe değil) kendi `arch.loadBearing`'i (evet, hayır, dayanak);
  yoksa bilinmiyor. Duvar stili taşımaz (stil işleminin alan yokken sildiği anlamı değişirdi: §6).
- **K117 — Çatının kaçış yolu rolü:** `arch.exit` çatıda da; Öznitelikler'de "Kaçış yolu". IFC'de çatının
  `FireExit` alanı yok: yazılmaz.
- **K118 — Ek-3 rolleri okur** (`dayanim`, paket 0.14.0): taşıyıcı iç duvar R (2); taşıyıcı dış duvar REI (5, not (3)
  yok); taşıyıcı olmayan dış duvar 120(3)'te EI 90; kompartıman duvarı taşıyıcıysa REI (6–7, 2'nin satırından önce);
  kaçış yolu olan çatı REI 30 (4-a, Ek-3/C'den bağımsız). Bölme duvara (taşıyıcı değil, dış ve kompartıman değil) süre
  istenmez.
- **K119 — IFC:** beyan edilen taşıyıcılık `Pset_WallCommon.LoadBearing`.
- **Dosya:** mimari verisi 13 (`ARCH_WITH_DECLARED_ROLES`): eski okuyucu reddeder, rolsüz çizim eskisi gibi yazılır.

## §3. Uygulama

- **Sözleşme:** `performance.rs` (`DeclaredBearing`), `ArchInfo.load_bearing`; `cad_arch_performance.rs`.
- **Dosya:** `kentos_kcad::{ARCH_WITH_DECLARED_ROLES, uses_declared_roles}`, `MODULE_VERSIONS`.
- **Uygulama:** `arch_performance` (`load_bearing_rated`, `load_bearing_of`, `exit_rated` çatıda, `exit_of`).
- **Masaüstü:** Öznitelikler'de duvarın "Taşıyıcı" ve çatının "Kaçış yolu" seçimi (Evet, Hayır, Bilinmiyor; bir geri al
  adımı "Yangın ve performans"); `properties::edit_performance`'a taşıyıcılık.
- **IFC:** `kentos_ifc::arch` beyanlar bölümü.
- **Olgular:** `ElementFacts.load_bearing`, `escape`; şema yeniden yazıldı.

## §4. Sınamalar

- **Komut durumları** (`fixtures/commands/v1/cad.arch.performance.json`, 10 yeni, 52 durum;
  `scripts/fixtures/arch_performance_cases.py`, çizime düz bir çatı eklendi): duvar taşıyıcı, stilsiz duvar taşıyıcı
  değil sınıfıyla, null kaldırır, kolona, kapıya ve çatıya taşıyıcılık reddedilir, belgesi adsız, kilitli katman, çatı
  kaçış yolu, çatıya sınıf ve kaçış yolu birlikte.
- **Dosya:** `the_declared_roles_round_trip_under_arch_data_13`. **IFC:**
  `a_wall_s_declared_load_bearing_role_is_its_common_set_s` (evet, hayır, beyansız yok).
- **Paket** (`fixtures/standards/v1/dayanim.json`, 7 yeni durum, metinden elle): taşıyıcı iç duvar EI 60 kalır, REI 60
  geçer; bölme duvar kapsam dışı; 32 m yağmurlamalı büroda taşıyıcı olmayan dış duvar EI 90 geçer, taşıyıcı olan REI 120
  ister; kaçış yolu çatısı EI 60 kalır; taşıyıcı kompartıman duvarı EI 90 REI ister.
- **Masaüstü** (`resistance_tests::a_load_bearing_wall_is_read_by_ek_3_b_2`): iç duvar Öznitelikler'de Bilinmiyor,
  Evet seçilince Ek-3/B 2 eksik veri, REI 60 ile geçer, Hayır ile bölme. **Resimler:** `.run/gui-evidence/tasiyici-8090/`.

## §5. Arayüz

- Öznitelikler › duvar: "Taşıyıcı" (Evet, Hayır, Bilinmiyor) "Yangın dayanımı"nın altında; çatı: "Kaçış yolu".

## §6. Sıradaki adımlar

- **Duvar stilinin taşıyıcılığı:** stil işleminin "alan yoksa sil" anlamını "alan yoksa kalır"a çevirmek bir sözleşme
  kararıdır (sürüm 2); o zaman stil "Taşıyıcı duvar" diye işaretlenir.
- **Korunumlu şaft (Ek-3/B 8, REI 120)** ve **yangın kesici (10)**: modelde rolü ve öğesi yok ([ADR
  8136](8136-protected-shafts-and-fire-stops.md)'da yapıldı: şaft mahalinin ve duvarın, döşemenin beyanı).
- **Döşemenin, kolonun taşıyıcılığı:** Ek-3/B 1 zaten çerçeveyi (kolon, kiriş) taşıyıcı sayar; perde duvar beyanı
  taşıyıcılıkla gelir.
