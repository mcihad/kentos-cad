# 2380 — Dışa aktarma denetiminde bloklar, tablolar ve ilişkiler

Tarih: 2026-11-08. plan/map-cad.md MAP-17, üçüncü tur. Durum: taslak. Plan satırı: “Tam/koşullu destek matrisi nesne
ve özellik düzeyinde olsun: eğri, Z/M, metin/blok, stil, ilişki, kimlik, kalite ve kaynak.”

İlgili:
- [ADR 1660](1660-exchange-round-trip-check.md), dışa aktarma kaybı denetimi. “Sonra”sındaki “blok, ilişki ve tablo
  nesnelerinin aynı denetimle sınanması” maddesini bu ADR kapatır;
- [ADR 1880](1880-exchange-support-matrix.md), destek matrisi. “Sonra”sındaki “blok, ilişki ve tablo nesnelerinin
  örnek çizime eklenip satırlarının bağımsız sınanması” maddesini de bu ADR kapatır;
- [ADR 0405](0405-relations.md) projenin ilişkileri, [ADR 0627](0627-geopackage-tiles-coverages-links-and-profiles.md) §6
  GeoPackage'ın ilişkili tabloları (OGC 18-000).

## Dayanak

- **OGC GeoPackage Related Tables Extension** (OGC 18-000, sürüm 1.0, 8 Mayıs 2019;
  [belge](https://docs.ogc.org/is/18-000/18-000.html), 8 Kasım 2026'da okundu): ilişkiler `gpkgext_relations`
  tablosunda, kullanıcının eşleme tablolarıyla birlikte tutulur. Eşleme tablosu iki tablonun satırlarını birincil
  anahtarlarıyla bağlar, çoka çok ilişkiyi de taşır.
- **AutoCAD DXF Reference, TABLE**
  ([belge](https://help.autodesk.com/cloudhelp/2019/ENU/AutoCAD-DXF/files/GUID-D8CCD2F0-18A3-42BB-A64D-539114A07DA0.htm),
  8 Kasım 2026'da okundu): tablonun blok adı grup kodu 2'dedir, anonim bloğu `*T` ile başlar. Varlık bir blok
  başvurusudur (`AcDbBlockReference`).
- **KentOS bugün** (8 Kasım 2026'da denendi):
  - denetim çizimi, özgün çizimin boş bir kopyasına geri okuyordu. O kopyada özgünün blok tanımları ve ilişkileri de
    vardı. Okuyucu bir bloğu ya da ilişkiyi yeniden kurmasa bile geri okunmuş görünüyordu;
  - blok ekleri ve tablolar yalnız “yazılmadı” ya da “türü değişti” olarak sayılıyordu. Geri okunan bloğun tanımı,
    tablonun hücreleri ve projenin ilişkileri karşılaştırılmıyordu;
  - KentOS'un DXF yazıcısı tabloyu görünüşünü çizen anonim bir blok (`*U…`: çizgiler ve hücre yazıları) ve KentOS
    XDATA'sı (hücreler, boylar) olarak yazıyor. KentOS'un okuyucusu XDATA'dan tabloyu geri kuruyor. Başka bir program
    tabloyu çizgiler ve yazılar olarak görür;
  - GeoPackage yazıcısı ilişkiyi ilişkili tablolarla yazıyor (`kentos_references`, Malik → Parsel). GDAL bunu
    “Relationship: kentos_references (Association, Malik, Parsel)” diye okuyor.

## Karar

1. **Boş kopya:** okuyucu, ayarlar ve katmanlarla kurulmuş, nesnesi, **blok tanımı ve ilişkisi olmayan** bir kopyaya
   okur. Geri okunan bloklar ve ilişkiler böylece okuyucunun dosyadan kurduklarıdır.
2. **Yeni kayıplar** (`EXCHANGE_LOSSES` sonuna):
   - `blockChanged`: geri okunan ek başka bir blok gösteriyor. Bloğun adı, tabanı (toleransla), nesnelerinin türü ve
     yerleri (sırasıyla, toleransla) ya da ekin ölçeği, dönüklüğü, aynası değişmiştir. Ayrıntı bloğun adıdır;
   - `tableChanged`: tablonun hücreleri (`cells`), satır ve sütun boyları (`sizes`, toleransla) ya da birleşik hücreleri
     (`merges`) değişmiştir. Ayrıntı ilk değişendir;
   - `relationLost`: yazılan katmanlar arasındaki bir proje ilişkisi kayboldu. Okuyucu aynı katmanlar ve alanla bir
     ilişki kurmamıştır, ya da kurduğu ilişkinin anahtarları ana katmandaki nesnelere kalıcı kimlikle ulaşmıyordur.
     Ayrıntı ilişkinin adıdır; nesne kimliği yoktur.
3. **Yeni matris satırları** (`EXCHANGE_ASPECTS` sonuna):
   - `blocks` ve `tables`: yazılmayan ek ya da tablo varsa kaybolur (tür ve sayı). Yoksa başka türe dönen ya da değişen
     varsa dönüşür. Yoksa korunur;
   - `relations`: kaybolan ilişki varsa kaybolur (adlarıyla). Yoksa korunur.
4. **Eski çizimin cevabı değişmez.** `devir.json`'da blok, tablo ve ilişki yoktur. Yeni satırlar onda korunur der.
5. **Masaüstü** (Dışa aktarma kaybı denetimi):
   - Tüm biçimler matrisinde Bloklar, Tablolar ve İlişkiler satırları;
   - kayıp sözleri: Blok başka tanımla geri okundu, Tablo değişti (hücreler, satır ve sütun boyları, birleşik
     hücreler), İlişki geri okunmadı.
6. **Kapsam işareti:** ADR 1660'taki gibi **CAD + CBS** (her modül).

## Kanıt

- **Örnek çizim:** `fixtures/exchange/v1/devir-blok.json` (`scripts/fixtures/exchange_drawing.py`). İlk çizime
  eklenenler:
  - Baca'da “Rögar” bloğunun eki (daire ve harf) ve özniteliği;
  - Yazı'da iki satır, iki sütunlu bir tablo;
  - Malik katmanı ve “Parselin malikleri” ilişkisi: Malik'in noktası parselin kalıcı kimliğini `parsel_id`'de tutar.
- **Sonuçlar:**

  | Satır | GeoPackage | GeoParquet | FlatGeobuf | GML | Shapefile | DXF |
  |---|---|---|---|---|---|---|
  | Bloklar | kaybolur | kaybolur | kaybolur | kaybolur | kaybolur | korunur |
  | Tablolar | kaybolur | kaybolur | kaybolur | kaybolur | kaybolur | korunur |
  | İlişkiler | korunur | kaybolur | kaybolur | kaybolur | kaybolur | kaybolur |

  GeoParquet kalıcı kimliği ve anahtar alanını korur, ama ilişkinin tanımını taşımaz.
- **Rust** (`kentos-headless`, `exchange_parts::tests`):
  - satırlar biçim biçim tablodaki gibidir;
  - kurallar ısırır: dairesi ya da dönüklüğü başka blok değişmiş, bir hücresi ya da sütunu başka tablo değişmiş
    sayılır. Kurulmayan ilişki ve anahtarı bir kot noktasına giden ilişki kaybolmuş sayılır. Yazılmayan katmanlar
    arasındaki ilişki sayılmaz;
  - eski denetim ve matris testleri aynen geçer.
- **Bağımsız** (`python/tests/test_exchange_check.py`, `BlocksTablesRelations`), KentOS kodu yok:
  - **GDAL** GeoPackage'da `gpkgext_relations`'ı okur: taban tablo Malik, ilişkili tablo Parsel. Eşleme tablosunun tek
    satırı Malik'in noktasını parsele bağlar. Ek ve tablo dosyanın hiçbir tablosunda yoktur;
  - **ezdxf** DXF'te “Rögar” ekini yerinde okur. Bloğunda 0,75 yarıçaplı daire ve “R” yazısı vardır. Tablo anonim bir
    bloğun ekidir: çizgileri (3 + 3) ve hücre yazıları satır sırasıyla, KentOS XDATA'sında hücreleri;
  - matrisin her hücresi ADR'nin kurallarıyla Python'da ayrıca yazılmış hâliyle tutar (iki çizimde);
  - 10 test geçer.
- **MCP** (`crates/native/mcp/tests/exchange_parts.rs`): çizim `.kcad` olarak açılır. Matrisin üç satırı tablodaki
  gibidir; DXF denetimi ilişkiyi kaybolmuş, bloğu ve tabloyu değişmemiş sayar.
- **Masaüstü** (`exchange_check_window_tests::the_matrix_shows_blocks_tables_and_relations`): satırların durumları ve
  hücre sözleri (Kaybolur: blok 1, Kaybolur: Parselin malikleri, Korunur). Çapraz masaüstü denetimleri 136 geçer.
  Resimler `.run/gui-evidence/blok-tablo-iliski-2380/`.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("teslim.kcad")
m = cad.file.exchange.matrix(doc)
for row in m.rows:
    if row.aspect in ("blocks", "tables", "relations"):
        print(row.aspect, [(c.format, c.state, c.detail) for c in row.cells])
```

## Sonra

- DXF'te tablonun AutoCAD'in TABLE varlığı olarak yazılması. Başka programlar bugün tabloyu çizgiler ve yazılar olarak
  görür; matris bunu KentOS'un yazıcısıyla okuyucusunun birlikte yaptığı olarak “korunur” der.
- DXF'te ek özniteliklerinin ATTRIB olarak yazılması (bugün KentOS XDATA'sında).
- Teslim paketinin matrisi ve kayıp raporunu taşıması (ADR 1880 Sonra).
- Kurum profillerine göre biçim önerisi.
