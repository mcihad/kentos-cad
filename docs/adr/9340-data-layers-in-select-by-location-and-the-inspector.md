# ADR 9340: Konuma göre seç veri katmanlarını girdi alır, Denetçi çizim nesnelerini ve kaynak satırlarını ayrı sayar

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026 (`gis-eksik-27-11-2026.md`, madde
  11 ve 12, P2). Sahibin sözü: “Gis eksik 27 11 dosyasındaki maddeleri tamamla”.
- **İlgili:**
  - [ADR 0200](0200-spatial-and-attribute-queries.md) §2 (Konuma göre seç);
  - [ADR 9101](9101-data-layers.md) (veri katmanı: satırlar kaynaktadır);
  - [ADR 9130](9130-statistics-and-charts-over-data-layers.md) (satırların kaynaktan okunması);
  - [ADR 9133](9133-select-by-expression-on-data-layers.md) (İfadeyle seç veri katmanında);
  - [ADR 9326](9326-one-record-on-every-surface.md) (seçili kaynak satırı Öznitelikler'de).

## Bağlam

- **Madde 11:** yalnız GeoPackage'a bağlı çizgi ve nokta katmanları görünürken Konuma göre seç “Görünen katmanlarda
  uygun nesne yok” dedi.
  - İşlem araçları girdilerini çizimin nesnelerinden alır. Veri katmanının satırları kaynaktadır; araç onları
    görmüyordu.
  - Yalnız İfadeyle seç veri katmanında kaynağa soruyordu (ADR 9133).
- **Madde 12:** bir, üç ve üç satırlı üç GeoPackage katmanı olan proje yeniden açıldı. Seçim yokken Denetçi
  “Nesne sayısı 0” ve “Veri katmanları 3 katman” gösterdi; satır sayısını “Kaynak durumu”na gönderiyordu.
- **İncelemenin kabulü:**
  - Başvuru ve hedef kaynak katmanlarına bağlansın. 30–40–50 m üçgeninin uçları kesişmeyle seçilsin, başka katmanda
    5 m ötedeki kontrol noktası dışarıda kalsın. Sonuç katman ve anahtarla Tablo'ya ve Denetçi'ye yansısın.
  - Yerel çizim nesneleri ve kaynaktaki özellik sayısı ayrı etiketlensin. Ucuz sayım gösterilsin; bilinmeyen değer 0
    sunulmasın.

## Araştırma

- **QGIS, Select by location:** girdi ve karşılaştırma katmanı sağlayıcısı ne olursa olsun bir vektör katmanıdır
  (dosya, veritabanı, servis). Sonuç girdi katmanının seçimidir; öznitelik tablosu seçili satırları gösterir
  ([Select by location](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/vectorselection.html#select-by-location)).
- **QGIS, nesne sayısı:** Katmanlar panelinin “Show Feature Count”u sayıyı sağlayıcıya sordurur; büyük veritabanı
  katmanında yavaş olabileceği için isteğe bağlıdır
  ([Layers panel](https://docs.qgis.org/latest/en/docs/user_manual/introduction/general_tools.html#layers-panel)).
- **KentOS kararı:**
  - Araç kendi hesabını değiştirmez. Satırlar okunur ve çizimin bir kopyasına nesne olarak konur; araç orada
    `cad.processing.run` ile koşar. Sonuç katman ve anahtarla geri döner.
  - Satır sayısını kaynağın kendisi sayar (`count`, kestirim değil), arayüzün iş parçacığı dışında. Bilinmezken
    “sayılıyor…” denir.

## Karar

### 1. Konuma göre seç veri katmanlarını girdi alır (madde 11)

- **Hangi katmanlar:** Seçilecek nesneler ya da Başvuru nesneleri bir veri katmanını kapsıyorsa:
  - Katman: o veri katmanı;
  - Görünen: görünür veri katmanları;
  - Tümü: bütün veri katmanları;
  - Seçili: tablonun satır seçimi.
- **Kopya** (`gis/processing/data_location.rs`): bu katmanların satırları kaynaktan okunur (ADR 9130'un yolu) ve
  çizimin bir kopyasına nesne olarak eklenir. Çizimin tuttuğu satırlar onun nesnesidir, iki kez eklenmez. Kopya çizim
  değişmedikçe saklanır.
- **Pencere:** girdiler kopyada sayılır: “5 nesne (nokta, çoklu çizgi)”, “1 çoklu çizgi; “ucgen” katmanında”.
  - Bir kaynak okunurken girdi satırı bunu söyler ve Çalıştır bekler.
  - Okunamayan kaynak adıyla söylenir.
- **Çalıştır:** araç kopyada `cad.processing.run` ile koşar. Çizimin seçimi ve tablonun seçili satırları Seçim
  biçimi'nin başlangıcıdır. Sonuç katman ve anahtarla döner:
  - çizimin nesneleri çizimin seçimine;
  - bir veri katmanının satırları tablosunun seçimine. Tablo o katmana gelir, harita satırları çizer.
  - Tablo bir katmanı gösterir: Seçilecek nesneler'in katmanını, yoksa en çok satırı seçileni. Öbür katmanlarda
    ilişkiyi sağlayanlar sonuçta adıyla ve sayısıyla söylenir.
- **Sonuç satırı:** “3 nesne ilişkiyi sağladı; “koseler” katmanında 3 satır tabloda seçildi (anahtar: 1, 2, 3).”

### 2. Denetçi çizim nesnelerini ve kaynak satırlarını ayırır (madde 12)

Seçim yokken, projede veri katmanı varsa (`gis/source_counts.rs`, Denetçi'nin Çizim bölümü):

- **Çizimdeki nesne:** projenin kendi nesneleri (eski “Nesne sayısı”). Veri katmanı yoksa satırın adı “Nesne
  sayısı” kalır.
- **Kaynaktaki satır:** veri katmanlarının satırları, kaynaklarının saydığı.
  - Sayılırken “sayılıyor…”, hiçbir zaman 0 değil.
  - Sayılamayan katman “6 (+ 1 katman sayılamadı)” biçiminde ayrıca söylenir.
- **Veri katmanları:** katman sayısı; altında her katmanın satırı (adı ve “3 satır”, sayılırken “sayılıyor…”,
  okunamazsa nedeni). En çok sekiz katman gösterilir; fazlası “… N katman daha” satırındadır.
- **Önbellek:** sayım katmanın kaynağı ve tablosu, çizimin oturumu ve kaynağa yazdıklarıyla saklanır. Kaynağı
  değişen katman yeniden sayılır.

### 3. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Konuma göre seç'te veri katmanları | yok: Mimari'de veri katmanı yoktur (araç çizimin nesnelerinde kalır) | var: CBS tabloları altlık olarak eklenebilir | var: aynı gerekçe | var |
| Denetçi'nin Çizimdeki nesne, Kaynaktaki satır ve katman satırları | yok: aynı gerekçe (Nesne sayısı kalır) | var | var | var |

### 4. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Masaüstü aracı `cad.processing.run` ile koşturur. Betik veri katmanının satırlarını `cad.layers.rows` ile okur, `cad.processing.run`'a aynı yoldan verir. |
| MCP | var | Aynı komutlar. |
| Yapay zekâ ajanı | var | Aynı komutlar. |
| Eklenti | yok | Seçim ve Denetçi arayüzün sunumudur; eklentinin işlem aracı kendi girdilerini alır (ADR 9310). |

## Açık

- **Öteki işlem araçları:** yalnız Konuma göre seç veri katmanlarını kopya yoluyla alır. Tampon, Kesiştir gibi
  araçlar veri katmanında hâlâ çizimin nesnelerini sayar; aynı yol onlara ayrı dilimde açılır.
- **Büyük tablo:** kopya satırları bütün olarak okur (ADR 9130). Çok büyük tablolarda kaynağın kendi mekânsal sorgusu
  daha uygundur.
- **Noktalar paneli:** incelemenin söylediği kaynak noktalarında Noktalar panelinin kapsamı bu dilimde yapılmadı.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `data_location_tests` (yalnız veri katmanları: `ucgen` 30–40–50 m üçgeni, `koseler` üç köşe, `kontrol` 5 m batıda
    nokta):
    - Seçilecek nesneler Tümü 5 nesneyi sayıyor, Başvuru `ucgen` 1 nesne;
    - Kesişen `koseler`'in tablosunda 1, 2, 3'ü seçiyor, `kontrol` dışarıda, çizimin seçimi boş;
    - Uzaklıkta 6 m kontrol noktasını da alıyor ve katmanını ayrıca söylüyor; 4 m'de yine dışarıda.
  - `source_counts_tests` (kaydedilip yeniden açılan proje):
    - Çizimdeki nesne 0, Kaynaktaki satır 7;
    - satırlar `qa_secim` 1, `qa_yollar` 3, `qa_noktalar` 3;
    - dosyası taşınan katman “sayılamadı”, toplam “6 (+ 1 katman sayılamadı)”;
    - veri katmanı olmayan projede “Nesne sayısı” kalıyor.
  - `-- processing:: properties source_counts data_location data_rows module_tests ribbon_tests history::text catalog
    settings`: 383 geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır (`shell/approved_layer.rs`,
    `shell/gpu_frame.rs`, `gis/data_selection.rs`; bu dilimden değil).
- **Takımlar:** `cargo test -p kentos-desktop-cbs` 63, `cargo test -p kentos-cad` (izler) 37 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9340/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-konuma-gore-sec-veri` (CBS paketi): girdi 5 nesne, sonuç satırı, tabloda üç seçili köşe;
  - `02-denetci-kaynak-satirlari`: Çizimdeki nesne 0, Kaynaktaki satır 7, katman satırları.
