# 1077 — GeoJSON dışa aktarımı: RFC 7946 varsayılan, seçim hatırlanır, tek katman tek satır

Tarih: 2026-11-20. gis-usage-report.md GIS-011. Durum: kabul edildi.

İlgili: [ADR 0046](0046-geojson-and-shapefile.md) (GeoJSON), [ADR 0222](0222-kml-gpx-and-explicit-transformation.md) §7 (WGS 84'e çevirme).

## Sorun

TM33 projesinde GeoJSON dışa aktarımı başlangıç ayarıyla metre koordinatlarını eski `crs` üyesiyle yazıyordu; WGS 84
seçimi bir sonraki açılışta sıfırlanıyordu. Pencere “Öznitelikler metin özellik olarak yazılır” diyordu ama tanımlı tam
sayı alanı JSON sayısı yazılıyordu. Tek park seçiliyken “Seçili (1)”, “Bütün katmanlar (1)”, işaretli tek katman ve
sonuç sayısı aynı anda görünüyordu.

## Dayanak

- [RFC 7946 §4](https://www.rfc-editor.org/rfc/rfc7946#section-4): GeoJSON'ın koordinat sistemi WGS 84 boylam,
  enlemdir; `crs` üyesi tanımlı değildir.
- GDAL GeoJSON sürücüsü ([belgesi](https://gdal.org/en/stable/drivers/vector/geojson.html)): `RFC7946=YES` ile
  WGS 84'e çevirip yazar; eski biçim projeksiyonlu koordinatı `crs` üyesiyle adlandırır.

## Karar

1. Projeksiyonlu projede (EPSG:4326 dışında, dönüşümü bilinen) Koordinatlar seçimi iki seçeneklidir:
   **GeoJSON (RFC 7946, WGS 84)** (varsayılan; boylam, enlem derece, `crs` yok) ve **Proje koordinatları (eski
   GeoJSON)** (projenin sisteminde metre, eski `crs` üyesiyle; açıklaması birimi ve uyumluluğu söyler). Son seçim
   oturum boyunca bir sonraki dışa aktarımın başlangıcıdır.
2. Öznitelik açıklaması gerçeği söyler: tanımlı sayı alanları JSON sayısı, evet/hayır alanları true/false, ötekiler
   metin.
3. Kapsamın nesneleri tek katmandaysa katman tablosu yerine tek satır: “1 nesne · Parsel · WGS 84”. Birden çok
   katmanda ayrıntılı seçim kalır.

## Sınama

- `gis_tests::a_geojson_export_is_rfc_7946_by_default_and_keeps_the_choice`: TM36 projesinde seçili parsel
  varsayılanla WGS 84 derece koordinatlı, `crs` üyesiz yazılır; proje koordinatları bir kez seçilince sonraki açılış
  onunla gelir. Resim `.run/gui-evidence/gis-011/`.
- `gis_tests::a_geojson_export_writes_what_the_reader_takes_back`: proje koordinatları seçilince eski `crs` üyesi
  (EPSG::5256) yazılır.
