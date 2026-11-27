# ADR 8174: Arazi: tabii ve tesviye zemin, kesitte ve hafriyatta

- **Durum:** 1. bölüm kabul edildi ve uygulandı (27 Kasım 2026): vaziyetin tabii ve tesviye zemin yüzeyleri, kesitte
  zemin çizgileri, dosyada Mimari verisi 43. Kalan: "tzk/tsk" kot işareti (2. bölüm), hafriyat hacmi ve zemini kuran
  araç (3. bölüm).
- **Plan:** [plan/mimari-bim.md](../../plan/mimari-bim.md) MB-12. Karar: [ADR 8171](8171-mimari-3d-section-quality-and-missing-tools.md)
  §2.6.
- **İstek** (sahip, 27 Kasım 2026): "mimari proje çizimlerinde birçok eksik aracımız var". Araştırma raporu (§5): arazi,
  bina oturumu ve kazı-dolgu piyasadaki araçlarda var, KentOS Mimari'de yok.
- **Sözleşmeler:**
  - `SitePlan`'a `naturalGround` ve `gradedGround` eklendi. İkisi de bir arazi yüzeyinin kimliği.
  - `SitePlan` kendi dosyasına taşındı: `crates/shared/contracts/src/arch/site.rs`.
  - `.kcad`'ın Mimari verisi 43 (`ARCH_WITH_GROUND`). Belge şeması değişmedi.

## §1 Sorun

- **Kesit araziyi bilmiyordu.** Tesviye zemin her kesitte ±0.00'da düz bir çizgiydi.
- **Tabii zemin hiç çizilmiyordu.** Her kesit "tabii zemin (modelde yok)" diye listeliyordu.
- **Yönetmelik ikisini de istiyor:**
  - PAİY 57/2-a: vaziyet planında ve kesitlerde tabii ve tesviye zemin.
  - Uygulama projesinde hafriyat hacmi.
  - Eğimli arsada düz ±0.00 çizgisi yanlış bir şey söylüyordu.

## §2 Araştırma

### Türkiye mevzuatı

Planlı Alanlar İmar Yönetmeliği'nin resmî metni (mevzuat.gov.tr, `7.5.23722`, son değişiklik 1.7.2026/33297) bu iş
için okundu.

- **Tanımlar (4. madde):**
  - Tabii zemin: "Arazinin hafredilmemiş veya doldurulmamış hali" (çççç).
  - Tesviye edilmiş zemin: "Bu Yönetmelikte belirtilen esaslara göre kazılarak veya doldurularak parsel tabi zemininin
    aldığı son zemin durumu" (gggg).
- **Kotlandırma:**
  - 10: yoldan kotlandırma esastır.
  - 11: tretuvar seviyesi kırmızı kotun 0,18 m üstüdür; kademe en çok 3,50 m.
  - 12/4: "Tabii zeminden kotlandırmada, ± 0.00 kotu binanın köşe kotlarının aritmetik ortalaması alınarak bulunur."
- **Bahçe tesviyesi:**
  - 15: tabii ya da tesviye zemin bağımsız bölüm için hafredilemez.
  - 16/2: yol cephesindeki bahçe yola doğru en fazla %2 eğimle tesviye edilir.
  - 18/1: ±0.00'ın üstündeki arka bahçe bu kota kadar kazılır.
  - 18/3: arka bahçeye en çok 2,00 m ve ±0.00'ı geçmeyen dolgu yapılabilir.
- **57/2-a:** vaziyet planında "±0.00 ve röper; tabii ve tesviye zemin", kesitlerde "tabii ve tesviye zemin";
  uygulamada hafriyat (`docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`).
- **Dijital Hazırlama Yönetmeliği, EK-4 4.30 ve Şekil 4.6:** tabii zemin kotu ve tesviye kotu bir işaretle gösterilir:
  "tzk 849.85 (-5.65)" üstte, "tsk 850.10 (-4.40)" altta. Önce mutlak kot, parantezde ±0.00'a göre farkı. Bütün
  disiplinlerin paftalarında kullanılır.

### Mevcut KentOS

- **Arazi yüzeyi** (TIN) Yol ve Harita'nın: `settings.road.surfaces` (`TerrainSurfaceDef`: nokta ve kırık hat
  kaynakları, sınır, boşluklar), `road.terrains.*` komutları.
  - Yüzeyin bir çizgi boyunca kesiti kesindir: `Surface::section`, üçgen kenarlarını kestiği yerlerde.
  - İki yüzey arasındaki hacim kesin üçgen ağı–üçgen ağı yöntemiyle hesaplanır (`road.terrains.compare`).
- **Komutların kapısı** (`holds_surfaces`): Harita ve Yol (ADR 1520).
  - 20'den fazla durum betiği ve 30 durum dosyası Mimari projesinin yüzeyi reddettiğini sınıyor.
  - Hepsi başka modüllerin, yani başka oturumların alanı.
- **Vaziyet planı** (ADR 8032) ±0.00'ın mutlak kotunu (`datum`) zaten tutuyor.

## §3 Karar

### §3.1 Model

- **Bağ:** vaziyet tabii ve tesviye zemini adlandırır: `SitePlan.naturalGround`, `SitePlan.gradedGround`. İkisi de
  projenin arazi yüzeylerinden birinin kimliği.
- **Yüzeyin yeri:** yüzey Yol'un ve Harita'nın aynı tanımıyla (`settings.road.surfaces`) tutulur. Doğrulaması, dosya
  biçimi ve bağımsız okuyucusu hazırdır.
- **Kapı değişmedi:** `road.terrains.*` komutlarının kapısı (`holds_surfaces`) değiştirilmedi. Mimari'de yüzeyi kendi
  aracı kurar (3. bölüm). Başka modüllerin durumlarına dokunulmadı.
- **Kurallar:**
  - Kimlikler boş olamaz.
  - Tabii ve tesviye zemin aynı yüzey olamaz: hafriyat ikisinin farkıdır.
- **Kot:** yüzeyin kotları mutlaktır; kesitte ±0.00'a göre (`datum`) yazılır. `datum` yoksa zemin çizilmez.

### §3.2 Kesitte

- **Vaziyet zemin yüzeyleri ve `datum` verdiğinde:**
  - Tabii zemin (`MM-CVRE-TABII`) ince ve kesikli çizilir.
  - Tesviye zemin (`MM-CVRE-TESVIYE`) kalın çizilir.
  - İkisi de yüzeyin kesit çizgisi boyunca kesin kesitidir, binanın iki yanında 2 m'ye kadar.
  - Çizgi, kesilen elemanların içinden geçtiği yerde ve yüzeyin dışında kalan kısımda kesilir.
- **Yalnız tabii zemin verildiğinde** tesviye zemin eskisi gibi ±0.00'da düz çizgidir.
- **Hiçbiri yoksa** kesit eskisi gibi çizilir ve tabii zemini eksik olarak listeler.
- **Yeri:** kod `sections/ground.rs`'tedir. `sections.rs` zemini ona bırakır.
- **Derleme:** Mimari derlemesi arazi çekirdeğini açar (`mod-arch` → `terrain`). Yol ve Harita olmadan da derlenir.

### §3.3 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | Vaziyet ve kesit Mimari'nin. |
| Harita, Yol | değişmedi | Yüzeyler zaten onların; komutları ve kapıları aynı. |
| CBS | yok | Bina modeli yok. |
| MCP, Python, eklenti, yapay zekâ ajanı | var | Ayarın kendisi (`settings.arch.site`) ve kesit komutları; yeni komut yok. |

## §4 Kanıt (1. bölüm)

- **Kesit** (`crates/native/application/tests/all/site_ground.rs`): kesit binası, çevresine eğimli bir arazinin
  noktaları (z = 1000 + 0,1·x) eklenerek.
  - `the_natural_ground_is_drawn_along_the_line_over_plus_minus_zero`:
    - B-B'de `MM-CVRE-TABII` kesikli çoklu çizgiler var.
    - Her köşenin kotu, arazinin o plan noktasındaki kotu eksi 1000,50.
    - Tabii zemin eksik listelenmiyor.
  - `without_the_site_s_ground_the_graded_ground_is_flat_and_the_natural_ground_left_out`: önceki davranış.
- **Dosya:** `arch_elements::a_site_s_ground_round_trips_under_arch_data_43`. Mimari verisi 43 ile aynen okunur; aynı
  yüzey iki zemin olarak yazılırken reddedilir.
- **Derleme:** yalnız Mimari ve CBS ile (Yol ve Harita olmadan) uygulama derleniyor.
- **Sınamalar:**
  - Sözleşme (1679 + 39 + 13), `kentos-kcad` (291).
  - Uygulama: yalnız main'deki `gis/lineage` modül sınaması kırmızı.

## Kalanlar

- **2. bölüm:** EK-4 Şekil 4.6'nın "tzk/tsk" kot işareti. Vaziyette bir noktaya tıklanınca iki yüzeyin kotunu mutlak
  ve ±0.00'a göre yazar. Bina köşelerinin tabii zemin ortalaması (PAİY 12/4) vaziyet raporunda.
- **3. bölüm:**
  - Mimari'de zemini kuran araç: nokta katmanından tabii zemin, çizilen kotlu sınır ve noktalardan tesviye zemin.
  - Hafriyat hacmi: iki yüzeyin farkı, parselin içinde, kazı ve dolgu ayrı.
- **Görünüş:** zemin altındaki kesikli kısım hâlâ ±0.00'a göre; tesviye yüzeyine göre değil.

## Uygulama

- `crates/shared/contracts/src/arch/site.rs` (yeni; `SitePlan`, `SiteEdge`, `SiteEdgeKind` `arch.rs`'ten taşındı),
  `naturalGround`, `gradedGround`, kurallar.
- `crates/native/application/src/arch/sections/ground.rs` (yeni), `sections.rs` (zemin oraya), `Cargo.toml`
  (`mod-arch` → `terrain`).
- `crates/shared/kcad/src/{lib.rs, decode.rs}`, `tools/kcad/kcad.py`: Mimari verisi 43.
- Sınamalar: `site_ground.rs`, `arch_elements.rs`; `SitePlan`'ı alan alan kuran üç sınama `..Default::default()` aldı.
