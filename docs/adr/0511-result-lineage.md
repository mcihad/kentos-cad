# ADR 0511: İşlem sonucunun soyu

- **Durum:** kabul edildi (2026-10-18). `plan/gis.md` CBS-10'un ilk dilimi: analiz damgası. Bir işlem aracının dosyaya
  yazdığı sonuç, neyden ve nasıl üretildiğini kendisi taşır.
- **Bağlam belgesi:** ADR 0508 (işlem araçları komut olarak, `sink`), 0503 (bağlı katman), 0502 (GeoPackage).
- **Sonraya kalan:**
  - çizimin katmanına yazılan sonucun soyu (katmanın kendi alanı; `.kcad` şeması değişir);
  - bağımlılık grafı ve “kaynak değişti, türev eski” göstergesi;
  - veri kümesinin kendi tanımı (başlık, sorumlu, lisans, kalite);
  - kurum kataloğu, OGC API Records, STAC.

## Bağlam

Plan ister: her türev girdi sürümlerini ve özetlerini, kapsamı, algoritmayı ve sürümünü, parametreleri, motoru ve üretim
zamanını taşısın; dosya başka cihazda açıldığında üretim yöntemi anlaşılsın; salt dosya adı soy ağacı sayılmasın.
`cad.processing.run`'ın `sink`'i sonucu bir GeoPackage'a yazar; bu dosya çizimden ayrılınca nereden geldiği
bilinmiyordu.

## Karar

1. **Girdinin içeriği:** cevabın `inputs`'unun her katmanı içeriğinin özetini taşır (`digest`: katmanın nesnelerinin tür,
   geometri ve özniteliklerinin kalıcı kimlikle SHA-256'sı, `sources::layer_digest`). Bağlı katmanda kaynağın kimliği ve
   okunan sürümü de yazılır.
2. **Dosyadaki soy:** `sink`'in her tablosu GeoPackage'ın metadata uzantısıyla (`gpkg_metadata`,
   `gpkg_metadata_reference`; `gpkg_extensions`'ta kayıtlı) bir belge taşır.
   - Kapsamı `dataset`, standardı `https://kentos.app/lineage/1`, türü `application/json`.
   - Tabloya `table` kapsamıyla, üretim zamanında bağlıdır.
   - Belge (`kentos.lineage` 1): tablo ve katman; araç (kimlik, ad); istek, girdiğin olduğu gibi (`sink` dışında:
     parametreler, seçim, görünüm); okunan katmanlar ve özetleri; çizimin sürümü; motor (`KentOS <sürüm>`); üretim
     zamanı (UTC).
3. Uzantı GeoPackage'ın standart uzantısıdır: QGIS ve GDAL tabloyu okur, metadata'yı gösterir ya da yok sayar; KentOS dışı
   bir okuyucu bozulmaz.

## Doğrulama

- `processing_sink_cases.py` G1'in üç dosyasını sqlite3 ile okur:
  - uzantının iki satırı;
  - tek metadata belgesi ve tabloya bağı;
  - zamanın biçimi;
  - araç ve istek testin verdiği gibi;
  - okunan katmanlar ve 64 haneli özetleri; değişmeden okunan parsellerin iki dosyada aynı özeti;
  - GDAL'ın `validate_gpkg.py`'si bir şey bulmaz.
- Ortak durumların planlarında `inputs` özetleriyle (`$any`: değer KentOS'un kanonik JSON'undan).
