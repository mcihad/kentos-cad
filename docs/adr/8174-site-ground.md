# ADR 8174: Arazi: tabii ve tesviye zemin, kesitte ve hafriyatta

- **Durum:** 1. ve 3. bölüm kabul edildi ve uygulandı (27 Kasım 2026).
  - 1. bölüm: vaziyetin tabii ve tesviye zemini, kesitte zemin çizgileri, dosyada Mimari verisi 43.
  - 3. bölüm: Arazi zemini aracı, ±0.00 bina köşelerinden, bahçe hafriyatı. Zeminin yeri değişti (§3.1).
  - Kalan: "tzk/tsk" kot işareti (2. bölüm).
- **Plan:** [plan/mimari-bim.md](../../plan/mimari-bim.md) MB-12. Karar: [ADR 8171](8171-mimari-3d-section-quality-and-missing-tools.md)
  §2.6.
- **İstek** (sahip, 27 Kasım 2026): "mimari proje çizimlerinde birçok eksik aracımız var". Araştırma raporunun 5.
  bölümü: arazi, bina oturumu ve kazı-dolgu piyasadaki araçlarda var, KentOS Mimari'de yok.
- **Sözleşmeler:**
  - `SitePlan`'a `naturalGround` ve `gradedGround` eklendi. İkisi de bir `SiteGround`: zeminin kotlu ölçü
    noktalarının katmanları (`layers`).
  - `SitePlan` kendi dosyasına taşındı: `crates/shared/contracts/src/arch/site.rs`.
  - `.kcad`'ın Mimari verisi 43 (`ARCH_WITH_GROUND`). Belge şeması değişmedi.
  - Yeni araç `tool.siteGround` (Arazi zemini); yeni komut yok.

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

- **Zemin vaziyetin kendisindedir:** `SitePlan.naturalGround` ve `SitePlan.gradedGround` birer `SiteGround`. Zemin,
  kotlu ölçü noktalarının durduğu katmanlardır. Yüzey kesit, ±0.00 ve hacim için okunurken o katmanların kotlu
  noktalarından kurulur. Kuran Yol'un kendi kurucusudur (`road_terrains::made_of`); üçgenleme ve kesit aynıdır.
- **Neden `settings.road.surfaces`'ta değil (3. bölümde değişti):**
  - 1. bölümde vaziyet, Yol'un ve Harita'nın yüzeylerinden birinin kimliğini tutuyordu.
  - Gerçek pencerede araç zemini kurunca kurtarma kopyası yazılamadı: "yol ayarları yalnız yol projesinin olur
    (cadWork road); arazi yüzeyleri harita projesinin de olur". Kural kodlayıcının ve çözücünün (ADR 1520).
  - Yani Mimari projesi yol ayarı tutamıyor; kullanıcı dosyasını kaydedemezdi. 1. bölümün sınamaları dosya yazmadığı
    için görmedi.
  - Kuralı Mimari'ye açmak, Yol'un ve Harita'nın kuralını üç yerde (kodlayıcı, çözücü, Python okuyucusu)
    değiştirirdi. Yol yüzeyinin düzenleme, bulut ve sınır alanları da Mimari'de hiçbir araçla yönetilemezdi.
  - Zemin vaziyete taşındı. 1. bölüm main'de değildi; şekli serbestçe değişti.
- **Kapı değişmedi:** `road.terrains.*` komutlarının kapısı (`holds_surfaces`) değiştirilmedi. Başka modüllerin
  durumlarına dokunulmadı.
- **Kurallar:**
  - Bir zemin en az 1, en çok 16 katmandan kurulur. Kimlik boş olamaz, her katman bir kez yazılır.
  - Tabii ve tesviye zemin aynı katmanlardan kurulamaz: hafriyat ikisinin farkıdır.
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
| MCP, Python, eklenti, yapay zekâ ajanı | var | Ayarın kendisi (`settings.arch.site`, `cad.settings.set`) ve kesit komutları; yeni komut yok. Python SDK'sında `SiteGround`. |

## §4 Kanıt (1. bölüm)

- **Kesit** (`crates/native/application/tests/all/site_ground.rs`): kesit binası, çevresine eğimli bir arazinin
  noktaları (z = 1000 + 0,1·x) eklenerek.
  - `the_natural_ground_is_drawn_along_the_line_over_plus_minus_zero`:
    - B-B'de `MM-CVRE-TABII` kesikli çoklu çizgiler var.
    - Her köşenin kotu, arazinin o plan noktasındaki kotu eksi 1000,50.
    - Tabii zemin eksik listelenmiyor.
  - `without_the_site_s_ground_the_graded_ground_is_flat_and_the_natural_ground_left_out`: önceki davranış.
- **Dosya:** `arch_elements::a_site_s_ground_round_trips_under_arch_data_43`. Mimari verisi 43 ile aynen okunur; aynı
  zemin iki kez yazılırken reddedilir (3. bölümde: aynı katmanlar).
- **Derleme:** yalnız Mimari ve CBS ile (Yol ve Harita olmadan) uygulama derleniyor.
- **Sınamalar:**
  - Sözleşme (1679 + 39 + 13), `kentos-kcad` (291).
  - Uygulama: yalnız main'deki `gis/lineage` modül sınaması kırmızı.

## §5 Arazi zemini aracı (3. bölüm)

- **Akış:** kotlu bir ölçü noktasına tıklanır. O noktanın katmanındaki kotlu noktalardan zemin kurulur ve vaziyetin
  tabii zemini olur. Tür (T) tesviye zemine geçer. Araç açık kalır; ikinci zemin hemen kurulur.
- **Söyledikleri:**
  - Kurulan zemin: üçgen sayısı ve kot aralığı ("Tabii zemin kuruldu: 432 üçgen, kot 998,52–1004,22 m."). Düz
    zeminde tek kot yazılır.
  - ±0.00 yoksa onu bulur ve söyler (aşağıda). Bina bulunamazsa kotun Proje ayarları › İmar durumu'ndan verileceğini
    söyler.
  - İki zemin ve parsel varsa bahçe hafriyatı: kazı ve dolgu ayrı, m³; kapsanan alan, m².
  - Az nokta (üçten az): kurulamadığını söyler, hiçbir şeyi değiştirmez.
  - Diğer zeminin katmanı: reddeder, hiçbir şeyi değiştirmez. Dosyaya yazılamayacak bir ayar hiç kurulmaz.
- **±0.00 (PAİY 12/4):** bina köşelerinin tabii zemin kotlarının aritmetik ortalaması, cm'ye yuvarlanmış.
  - Bina, oturum katının duvarlarının dış hattıdır. Oturum katı: ±0.00'da ya da üstünde, duvarı olan en alçak kat
    (zemin kat); yoksa altındakilerin en yükseği. Katsız projede bütün duvarlar.
  - Dış hat bir duvarın kapalı yanından bulunur, ilk kapalı yandan. L biçimli binada da doğrudur. Duvar
    noktalarının ortalaması L'nin dışına düşebiliyordu, bu yüzden kullanılmadı.
  - Köşe, doğrultunun değiştiği köşedir. Düz giden bir kenarın ara noktası ortalamayı kaydırırdı, sayılmaz.
  - ±0.00 zaten verilmişse dokunulmaz.
- **Hafriyat (PAİY 57/2-a, 16–18):** tesviye zeminin tabii zeminden farkı. Parselin içinde, bina oturumu dışında
  ölçülür: bahçe tesviyesi. Kazı, tesviye zeminin tabii zeminin altında kaldığı yerdir. Hacim Yol'un kesin
  üçgen ağı–üçgen ağı ölçüsüyle bulunur (`volume_materials::measure`). Bina kazısı (bodrum, temel çukuru) sayılmaz.
- **Ayardır, geri alma adımı değildir:** Yol'un yüzeyleri gibi. Aynı zemin yeniden kurulunca eskisinin yerini alır.
- **Yeri:**
  - Şeritte Proje panelinin açılır listesi, Parsel ve bahçe mesafeleri'nin ardından.
  - Kısa adı AZ.
  - Takma adları ARAZİZEMİNİ, TABİİZEMİN, TESVİYEZEMİN, HAFRİYAT, KAZIDOLGU, TOPOSURFACE. "ARAZİ" Yol'un Arazi
    yüzeyleri penceresinin adıdır; kullanılmadı.
- **Kısa adlar düzeltildi:** Tekil temel TK, Sürekli temel SR. ADR 8172'deki TT ve ST, KentOS'ta zaten Geçici izleme
  noktası'nın ve Yazı stilleri'nin adıydı. `module_names` sınaması yakaladı; bu iş koşulana dek görülmemişti.

## §6 Kanıt (3. bölüm)

- **Etkileşim** (`crates/native/interaction/tests/all/ground_tool.rs`, 3 sınama). Kurulum:
  - 12 × 9 m bina; tabii zemin eğimli, z = 1000 + 0,1·x; tesviye zemin düz, 1001; parsel 24 × 21 m.
  - Tabii zemin 112 üçgen.
  - ±0,00 = 1000,60 m: köşe kotları 1000 ve 1001,2.
  - Tesviye zemin 98 üçgen. Bahçe hafriyatı kazı 65,40 m³, dolgu 223,80 m³, 396,00 m². Elle hesap: dolgu
    21·12,8 − 9·5, kazı 21·3,2 − 9·0,2, alan 504 − 108.
  - Yol ayarı yok. Aynı katman reddedildi. Az nokta ve boş tıklama sınandı.
- **Uygulama** (`crates/native/application/tests/all/site_ground.rs`, 4 sınama):
  - L biçimli binada 6 köşe; 1. katın 4 köşesi alınmadı; ±0,00 = 1001,4.
  - Mimari projesi zemini kaydediyor ve geri okuyor. Aynı katman tesviye zemin olarak reddediliyor.
- **Dosya** (`arch_elements::a_site_s_ground_round_trips_under_arch_data_43`):
  - Mimari projesinde iki zemin aynen okunuyor.
  - Reddedilenler: aynı katmanlar, katmansız zemin, boş kimlik, iki kez yazılan katman.
  - Python okuyucusu zeminli dosyayı geçerli buldu (`kcad.py validate`).
- **Gerçek pencere** (`.run/gui-evidence/mimari-bim/{once,sonra}/mb12-*`). Dosya: kesit binası ve çerçeve; 247 ölçü
  noktası (her 3 m'de, eğimli ve hafif dalgalı); 96 tesviye noktası; parsel.
  - Tabii zemin 432 üçgen, kot 998,52–1004,22.
  - ±0,00 = 1000,81.
  - Tesviye zemin 154 üçgen, 1000,60.
  - Bahçe hafriyatı kazı 727,69 m³, dolgu 105,08 m³ (766,69 m²).
  - A-A kesitinde önce zemin ±0,00'da düzdü. Sonra tabii zemin eğim boyunca kesikli, tesviye zemin −0,21'de.
  - Kurtarma kopyası hatası (§3.1) kalktı.

## Kalanlar

- **2. bölüm:** EK-4 Şekil 4.6'nın "tzk/tsk" kot işareti. Vaziyette bir noktaya tıklanınca iki zeminin kotunu mutlak
  ve ±0.00'a göre yazar.
- **Bina kazısı:** bodrum ve temel çukuru, oturum içinde tabii zeminden temel tabanına kadar.
- **Planda zemin:** yükselti eğrileri ya da üçgenler. Hangi katmandan kurulduğu Öznitelikler'de görünmüyor.
- **Zeminin araçları:** zemin bir katmanın bütün kotlu noktalarından kurulur. Sınır, kırık hat ve düzenleme yok;
  Yol'un yüzey araçları Mimari'de yok.
- **Görünüş:** zemin altındaki kesikli kısım hâlâ ±0.00'a göre; tesviye yüzeyine göre değil.

## Uygulama

- `crates/shared/contracts/src/arch/site.rs` (yeni; `SitePlan`, `SiteEdge`, `SiteEdgeKind` `arch.rs`'ten taşındı),
  `SiteGround`, `naturalGround`, `gradedGround`, kurallar.
- 3. bölüm:
  - `crates/native/application/src/arch/site_ground.rs` (yeni): zeminin yüzeyi, `with_ground`, bina köşeleri,
    `datum_from_corners`, `earthwork`.
  - `crates/native/interaction/src/arch/ground_tool.rs` (yeni): araç.
  - Katalog: `additions.json` (tanım ve simge), `operations.json`, `modules.json`, `names-mimari.json` (AZ; TK, SR),
    `scripts/ui/ribbon.py`. Üretilenler: `ribbon.json`, `commandCatalog.json`, Python SDK, CAD el kitabı, simge
    listesi.
  - "Kesit çıkar"ın tanımı ve kesitin "tabii zemin eksik" gerekçesi artık Arazi zemini'ni anıyor.
- `crates/native/application/src/arch/sections/ground.rs` (yeni), `sections.rs` (zemin oraya), `Cargo.toml`
  (`mod-arch` → `terrain`).
- `crates/shared/kcad/src/{lib.rs, decode.rs}`, `tools/kcad/kcad.py`: Mimari verisi 43.
- Sınamalar: `site_ground.rs`, `arch_elements.rs`; `SitePlan`'ı alan alan kuran üç sınama `..Default::default()` aldı.
