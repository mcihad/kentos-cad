# 1880 — Destek matrisi: altı biçimin özellik özellik ne koruduğu (`file.exchange.matrix`)

Tarih: 2026-11-07. plan/map-cad.md MAP-17, ikinci tur. Durum: taslak. Plan satırı: “Tam/koşullu destek matrisi nesne ve
özellik düzeyinde olsun: eğri, Z/M, metin/blok, stil, ilişki, kimlik, kalite ve kaynak.” İlgili:
- [ADR 1660](1660-exchange-round-trip-check.md): dışa aktarma kaybı denetimi; matrisin her sütunu onun bir biçimdeki
  cevabıdır.

## Dayanak

- **Plan:** dönüşen, korunan, yaklaşıklaşan ve kaybolan bilgi her aktarımda açıklanmalı; bir okuyucunun bulunması
  kusursuz gidiş-dönüş sayılmamalı.
- **KentOS bugün:** `file.exchange.check` tek biçimi denetliyordu. ADR 1660'ın biçim tablosu örnek çizim için elle
  yazılmıştı. Açık çizim için altı biçimi yan yana gösteren bir şey yoktu.
- **Bağımsız sınama:** ADR 1660'ın GDAL (`ogrinfo`) ve ezdxf denetimi her biçimin cevabını dosyanın içeriğine bağlar.
  Matris o cevaplardan, aşağıdaki kurallarla okunur.

## Karar

1. **`file.exchange.matrix` v1** (headless dosya komutu, salt okunur):
   - **Girdi:** `formats` (boş: altısı, `gpkg, parquet, fgb, gml, shp, dxf` sırasıyla; iki kez istenen bir kez denenir),
     `layers`, `tolerance` (denetiminkiler).
   - Her biçim için `file.exchange.check`'in gidiş-dönüşü yapılır; geçici klasörü denetim siler; açık çizim değişmez.
   - **Çıktı:** tolerans; satırlar; her biçimin denetim cevabı (`checks`).
2. **Satırlar ve hücre kuralları** (hücre: biçim, durum, sayı, ayrıntı; durum `kept`, `approximated`, `converted`,
   `added`, `lost`):
   - `objects`: yazılmayan nesne varsa kaybolur (türleri ve sayıları);
   - `kinds`: tür değişimi varsa dönüşür (`circle → polygon`);
   - `curves`: kirişe dönen eğri varsa yaklaşık;
   - `z`: kot kaybı varsa kaybolur, yoksa eklenen kot varsa eklenir;
   - `m`: ölçü kaybı varsa kaybolur;
   - `attributes`: öznitelik kaybı varsa kaybolur, yoksa değişen varsa dönüşür, yoksa eklenen varsa eklenir (adlarıyla);
   - `style`, `identity`: kayıp varsa kaybolur;
   - `position`: toleransı aşan köşe varsa yaklaşık (en büyük kayma), yoksa korunur (en büyük kayma ayrıntıda);
   - `extra`: eşleşmeyen geri okunan varsa eklenir.
3. **Retler:** denetiminkiler (`invalid_tolerance`, `layer_not_found`, yazıcının kendi reddi, `exchange_unreadable`,
   `file_unwritable`). Bir biçimin reddi yolunu `formats[i].` altında taşır. Yeni kod yoktur.
4. **Masaüstü:** Dışa aktarma kaybı denetimi penceresinde **Tüm biçimler**. Matris tablosu: satırlar özellikler,
   sütunlar biçimler; hücre Türkçe (Korunur, Yaklaşık, Dönüşür, Eklenir, Kaybolur, ayrıntıyla). Denetle tek biçime
   döner. Form tek etiket sütunlu (Biçim, Tolerans).
5. **Sınır:** blok, ilişki ve tablo nesneleri çizimde varsa satırlara kendiliğinden girer (yazılmayan ya da türü
   değişen). Örnek çizimde bulunmadıklarından bu dilimde ayrıca sınanmadılar.

## Kanıt

- **Rust** (`kentos-headless`, `exchange_matrix::tests`):
  - örnek çizimin matrisi ADR 1660'ın tablosudur (yazı DXF dışında kaybolur, daire her biçimde başka türe döner,
    GeoParquet ve Shapefile eğriyi kirişler, GeoParquet, GML ve DXF M'yi kaybeder, kimliği yalnız GeoPackage ve
    GeoParquet korur, köşeler toleransın içinde);
  - biçimler bir kez denenir; denetimin retleri yoluyla gelir.
- **Bağımsız** (`python/tests/test_exchange_check.py`): GDAL ve ezdxf testleri (altı biçim) aynen geçer.
  `MatrixFromTheChecks`:
  - matrisin her denetim cevabı, `file.exchange.check`'in o biçimdeki cevabıyla aynıdır;
  - her hücre ADR'nin kurallarıyla, Python'da ayrıca yazılmış hâliyle tutar.
- **Masaüstü** (`exchange_check_window_tests::all_formats_give_the_support_matrix`): Tüm biçimler altı sütunlu matrisi
  verir; hücre sözleri; Denetle tek biçime döner; çizim değişmez. Resimler `.run/gui-evidence/destek-matrisi-1880/`
  altındadır.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("teslim.kcad")
m = cad.file.exchange.matrix(doc)
for row in m.rows:
    print(row.aspect, [(c.format, c.state) for c in row.cells])
```

MCP: `{"tool": "file.exchange.matrix", ...}`.

## Sonra

- Blok, ilişki ve tablo nesnelerinin örnek çizime eklenip satırlarının bağımsız sınanması.
- Teslim paketinin bu matrisi ve kayıp raporunu taşıması.
- Kurum profillerine göre biçim önerisi (matrisin hangi satırlarının o kurum için önemli olduğu).
