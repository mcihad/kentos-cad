# ADR 9335: Zaman ayarları tarih tutan alanı önerir ve UTC'yi söyler; Kot noktası ekle ile Kot oku ayrılır; Mesh sor meshini sorar

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  19, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:**
  - [ADR 0210](0210-temporal-layers-and-scenarios.md) (zamansal katman, sürgü);
  - [ADR 0057](0057-desktop-drawing-tools-3.md) (Kot noktası);
  - [ADR 0140](0140-new-drawing-and-editing-tools.md) (Koordinat oku);
  - [ADR 0552](0552-point-cloud-areas-meshes-and-virtual-clouds.md) (Mesh sor);
  - CLAUDE.md §5 (Y doğu, X kuzey).

## Bağlam

- **Zaman:** GPX katmanında Zaman ayarları ilk alan olarak `Ad`'ı seçti ve 10 tarih ayrıştırma hatası gösterdi.
  - `Zaman` elle seçilince 9 zamanlı nokta ve 1 zamansız çizgi bulundu.
  - Kaynak zamanlar `Z` ekli UTC iken sürgü başlığında saat dilimi yoktu.
  - Kod, alanı yalnız adına bakarak tahmin ediyordu (“tarih”, “start”, “date”…). `Zaman` adı listede yoktu, bu yüzden ilk
    ad seçiliyordu.
- **Arazi:** Kot noktası tıklanan yerde değer istedi ve `Kot noktaları`na yeni nokta yazdı. İçe alınan GNSS noktasının
  kotunu sorgulamak isteyen için ad, eylemi söylemiyordu.
- **Mesh sor:** veri yokken örnek koordinat ve kapalı Çalıştır gösterdi; nedenini ve yolunu söylemedi. Yer “X (doğu)”,
  “Y (kuzey)” diye soruluyordu; Ölçme ve Türk ölçmeciliği Y'yi doğu, X'i kuzey yazar.
- **İncelemenin kabulü:**
  - Geçerli tarih alanları ve doğrulanan örnek değerler öne çıksın; doğru alan önerilsin; zamansız geometri ve saat
    dilimi açık olsun.
  - Kot noktası ekle ile salt okunur Kot oku ayrılsın.
  - Mesh sor'da mesh seçimi ya da Mesh ekle yönlendirmesi olsun; Y (doğu) / X (kuzey) sırası açık olsun.

## Araştırma

- **QGIS, katman zaman özellikleri:** katman bir tarih/saat alanıyla (ya da başlangıç ve bitiş alanlarıyla) zamansal
  yapılır; birikimli seçenek vardır
  ([Temporal Properties](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#temporal-properties),
  [Temporal controller](https://docs.qgis.org/latest/en/docs/user_manual/map_views/map_view.html#the-temporal-controller-panel)).
- **KentOS kararı:**
  - KentOS'ta tarih değerleri metin öznitelikleridir; alanın türü tarihi söylemez. Bu yüzden pencere her alanın
    değerlerinden okunabilen tarihleri sayar ve öneriyi buna dayandırır.
  - Anlar UTC'dir (ADR 0210 §3): `Z` ya da `+03:00` gibi bir dilim UTC'ye çevrilir, dilimsiz değer yazıldığı gibi alınır.
    Bu arayüzde söylenir.

## Karar

### 1. Zaman ayarları

- **Alanlar:** her alanın değerlerinden kaçının tarih olarak okunduğu, kaçının dolu olduğu ve okunan ilk değer
  sayılır.
  - Seçicilerde tarih tutan alanlar öne gelir, en çok tutan önce; öbürleri kendi sıralarıyla gelir.
  - Her seçeneğin altında “9 tarih · ör. 12.05.2024 09:31 UTC”, “tarih değil (10 değer)” ya da “boş” yazar.
- **Öneri:** katmanın ayarı yoksa pencere en çok tarih tutan alanla açılır. Eşitlerde adı başlangıç ya da zaman gibi
  olan önce gelir; hiçbir alan tarih tutmuyorsa eski ad kuralı uygulanır (adlara “zaman”, “time” eklendi).
- **Özet:**
  - “1 zamansız nesne sürgü açıkken de hep görünür.” (ADR 0210 §4'ün kuralı);
  - “Saatler UTC'dir: “Z” ya da “+03:00” gibi dilimi yazılan değer UTC'ye çevrilir, dilimsiz değer yazıldığı gibi
    alınır.”
- **Sürgü:** adım günün altındayken anın yanında “UTC” yazar (“12.05.2024 09:39 – 09:40 UTC”).

### 2. Kot noktası ekle ve Kot oku

- **Kot noktası ekle:** Kot noktası'nın adı budur: şeritte, ipucunda ve isteminde. Açıklaması kot okumak için Kot
  oku'yu gösterir. Noktanın kendi türü (“Tür: Kot noktası”) değişmez.
- **Kot oku** (`tool.readElevation`): Koordinat oku'nun okumasıdır, kot önde:
  - kenetlenen noktanın ya da çizgi, alan köşesinin kotu: “Kot oku: Z=1101.870 (Y=…, X=…)”;
  - kotu olmayan yerde nedeni ve yolu: “Kot oku: tıklanan yerin kotu yok (Y=…, X=…). Kotlu bir noktaya ya da köşeye
    kenetleyin; altta yükseklik rasteri varsa değeri ayrıca söylenir.”;
  - altındaki rasterlerin değerleri Koordinat oku'daki gibi sorulur;
  - imleçte Y, X ve Z; her tıklama bir okumadır; Esc bitirir; çizime bir şey yazılmaz.
- Başka bir komutun üstünde de çalışır (Koordinat oku gibi saydam araçtır).
- **Şeritteki yeri:** Kot noktası ekle'nin yanı:
  - CBS'de Analiz › Arazi;
  - Yol'da Arazi › Arazi;
  - Harita'da Plan › Plan yazıları'nın taşması (Giriş ve Plan doludur).
  - Simgesi Koordinat oku'nunkidir.

### 3. Mesh sor

- **Mesh:** pencerenin başında sorulan mesh vardır.
  - Hiç mesh yoksa neden yazar: “Projede mesh yok: sorulacak bir mesh olmadan Çalıştır bekler. UGRID NetCDF ya da 2DM
    dosyasını Mesh ekle ile ekleyin.” Yanında **Mesh ekle…** düğmesi vardır; Çalıştır kapalıdır.
  - Tek mesh adıyla yazılır.
  - Birden çoksa “Tüm meshler (N)” ya da biri seçilir; seçilen mesh komuta kimliğiyle verilir.
  - Mesh adı etiketinden, yoksa dosyasının adından, yoksa katmanından gelir; yanında veri seti ve ağ yazar.
- **Yer:** önce **Y (doğu)**, sonra **X (kuzey)** sorulur. Okunamayan yerin iletisi “Yerin Y'si (doğu) ve X'i (kuzey) sayı
  olmalı.” olur.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Zaman ayarlarının alan önerisi, zamansız ve UTC satırları, sürgüde UTC | yok: Mimari'de zamansal katman yoktur | var | var | var |
| Kot noktası ekle adı ve Kot oku | yok: Mimari'de Kot noktası yoktur | var | var | var |
| Mesh sor'un mesh seçicisi, Mesh ekle ve Y/X sırası | yok: Mimari'de Mesh sor yoktur | var | var | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Alan önerisi pencerenin sunumudur; katmanın zamanı `cad.layers.time` ile verilir. Kot oku bir şey yazmaz (`catalog/v1/operations.json`'da düzeyi `query`); betik noktanın kotunu nesneden okur. Mesh sorgusu `cad.meshes.query` (ve alan, hat) ile aynıdır; mesh kimliği `uids` ile verilir. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Kot oku ve yüzey:** Kot oku kenetlenen noktanın kotunu ve rasterin değerini söyler. Arazi yüzeyinin (TIN) o yerdeki
  kotunu okumaz.
- **Zaman dilimi:** dilimsiz değerin dilimini seçmek (yerel saat) yoktur; dilimsiz değer yazıldığı gibi alınır.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Etkileşim katmanı:** `kot_oku_says_a_points_elevation_first_and_writes_nothing`:
  - 12,5 kotlu noktada “Kot oku: Z=12.500 (Y=487005.000, X=4420005.000)”, rasterler soruluyor;
  - boş yerde neden ve yol;
  - çizime bir şey yazılmıyor, araç sonraki tıklamayı bekliyor.
  - Kot noktası ekle'nin istemleri ve kilit iletisi yeni adla.
- **Masaüstü:**
  - `temporal::layer::tests`: GPX benzeri katmanda pencere `Zaman` ile açılıyor; sıra `Zaman`, `Ad`; “9 tarih · ör.
    12.05.2024 09:31 UTC” ve “tarih değil (10 değer)”; özet 9 zamanlı, 1 zamansız, zamansız ve UTC satırları,
    okunamayan değer yok.
  - `area_tests::mesh_sor_names_its_mesh_and_says_when_there_is_none`: meshsiz çizimde mesh listesi boş; örnek çizimin
    16 meshi dosya adı ve ağıyla, Tüm meshler bütün meshlere soruyor, biri seçilince yalnız onun kimliği gidiyor.
  - `-- temporal:: area_tests catalog ribbon_tests module_tests history::text settings`: 170 geçti. Kırmızı tek sınama
    bilinen bağımlılık tabanıdır (`shell/approved_layer.rs`, `shell/gpu_frame.rs`, `gis/data_selection.rs`; bu dilimden
    değil).
- **Takımlar:** `cargo test -p kentos-interaction` 1094, `cargo test -p kentos-cad` (izler) 37, `cargo test -p
  kentos-desktop-cbs` 63 geçti.
- **Kataloglar:**
  - `python3 scripts/ui/ribbon.py --check` güncel;
  - `catalog/v1/modules.json` `KENTOS_WRITE_MODULES=1` ile yeniden yazıldı (Kot oku Harita, Yol ve CBS'de);
  - `python3 scripts/cad/manifest.py --check`: 1096 satır tutarlı.
- **Resim** (`.run/gui-evidence/kullanim-9335/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-zaman-ayarlari`: Başlangıç alanı Zaman, özetin üç satırı;
  - `02-zaman-surgusu-utc`: “12.05.2024 09:39 – 09:40 UTC”;
  - `03-kot-oku`: CBS'de Analiz › Arazi (Kot noktası ekle, Kot oku, Mesh sor), GNSS noktasında Z 1101.870;
  - `04-mesh-yok`: neden, Mesh ekle…, kapalı Çalıştır, Y (doğu) / X (kuzey);
  - `05-mesh-sor`: Tüm meshler (16).
