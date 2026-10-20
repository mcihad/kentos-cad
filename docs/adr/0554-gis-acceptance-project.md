# ADR 0554: CBS'nin ilk uçtan uca kabul projesi

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-44'ün bir dilimi. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0252 (CAD'in ilk kabul çizimi: aynı senaryo biçimi ve oynatıcıları);
  - 0199 (katman alanları), 0507 (Alan hesapla), 0200 (Konuma göre seç);
  - 0502, 0514, 0522, 0544, 0545 (GeoPackage, Shapefile, CSV, FlatGeobuf, GML).
- **Web:** ertelendi (ADR 0228).
- **Sonraya kalan:**
  - PostGIS'e yazma ve okuma, yetki ve geri yükleme kabulü (CBS-38, CBS-39'un tatbikatı);
  - aynı senaryonun MCP'de ve masaüstünün ajan bağlantısında oynatılması (bugün Python ve başsız oturum);
  - kesilen işlem ve yeniden açma senaryoları;
  - QGIS 4.2'nin kendi arayüzüyle açılış ve büyük veriyle performans kabulü.

## Bağlam

CBS-44'ün son kabulü şudur: bir parsel ve bina CBS projesi KentOS içinde oluşturulup düzenlenmeli, sorgulanıp doğrulanmalı ve başka
bir uygulamada veri kaybı olmadan açılmalıdır. Biçimlerin kendi testleri vardı, ama bir projenin baştan sona yolu ve dışarıda
okunuşu tek senaryoda yoktu.

## Karar

### 1. Senaryo: `fixtures/acceptance/v1/parcels.json`

ADR 0252'nin `kentos.acceptance` biçimiyle, `scripts/fixtures/acceptance_parcels.py` KentOS kodu olmadan yazar. Beklenen her değer
tanımdan hesaplanır: köşelerden alanlar, tipli alanların kanonik metni, hangi binanın hangi parsele değdiği, her biçimin neyi
taşıyabildiği.

Proje: CBS, TUREF / TM36 (EPSG:5256). Adımlar:
1. Parsel ve Bina katmanları ve tipli alanları (tam sayı, metin, ondalık sayı, tarih).
2. Yan yana üç parsel (ortak kenarlar) ve iki bina, öznitelikleriyle. İkinci parselin maliki boş metin, üçüncünün tescil tarihi yok
   (NULL): ikisi ayrıdır.
3. Yanlış girilen bir değer geri alınır.
4. Alan hesapla: `hesap_alani = round($alan, 2)`.
5. Konuma göre seç: üçüncü parsele değen bina.
6. Kaydet ve aç: alanlar, değerler ve ölçüler aynı.
7. GeoPackage, FlatGeobuf, GML, Shapefile ve CSV'ye dışa aktarma.
8. GeoPackage yeni bir projede açılır: katmanlar, satırlar, değerler ve alanlar eksiksiz.

### 2. Oynatıcı ve bağımsız okuma

- `python/tests/test_acceptance_gis.py` senaryoyu `kentos.cad` ile adım adım oynatır; başsız oturum, MCP sunucusunun da kullandığı
  yoldur.
- `KENTOS_WRITE_ACCEPTANCE=1` ile yazılan beş dosya `fixtures/acceptance/v1/out`'a gider.
- `acceptance_parcels.py --files` onları GDAL'ın OGR'ıyla okur (macOS'ta QGIS 4.2'nin Python'u):
  - katmanlar ve nesne sayıları, sistem EPSG:5256;
  - her satırın her alanı türüyle (sayılar sayı, tarih günü);
  - halkalar, başlangıç köşesinden ve yönden bağımsız, köşe köşe aynı; alanlar tam;
  - NULL ile boş metin GeoPackage ve FlatGeobuf'ta ayrı. dBASE ve CSV ayıramaz. GML'de KentOS NULL'u `xsi:nil`, boş metni boş öğe
    yazar ve ikisini ayrı okur, ama OGR boş öğeyi NULL okur (ADR 0545). Bu üçünde ikisi de boş sayılır;
  - Shapefile'da sütun adı dBASE'in on baytıyla (`hesap_alani` → `hesap_alan`, ADR 0514);
  - geometrisiz tabloların (GeoPackage'ın `kentos_uids`'i) sistemi yoktur, onlara bakılmaz.

Senaryonun her adımının beklenen değeri tanımdan: köşelerin alanları; tipli alanların kanonik metni (ondalık sayı en kısa yazımıyla,
`round($alan, 2)` 600, 600, 750); `cad.entities.set`'in öznitelik adımının adı “Değiştir”; üçüncü parsele değen bina, dikdörtgenlerin
köşelerinden.

## Doğrulama

- `python3 scripts/fixtures/acceptance_parcels.py --check`.
- `KENTOS_WRITE_ACCEPTANCE=1 .run/py/bin/python -m unittest discover -s python/tests -p "test_acceptance_gis.py"`.
- `python3 scripts/fixtures/acceptance_parcels.py --files`.
