# 1780 — İki ortofotonun karşılaştırılması: aynı noktaların kayması ve mozaik sınırı (`cad.orthophoto.compare`)

Tarih: 2026-11-07. plan/map-cad.md MAP-12, ikinci dilim. Durum: taslak. İlgili:
- [ADR 1480](1480-orthophoto-accuracy-check.md): Ortofoto doğruluk denetimi (`cad.orthophoto.check`). Bu ADR oradaki
  “Sonra: zamanları farklı iki ortofotonun aynı denetim noktalarıyla karşılaştırılması; mozaik kayıklığı (Madde
  67(6))” maddesini açar.

## Dayanak (7 Kasım 2026'da okundu)

- **BÖHHBÜY** (metni ADR 1480 için harita.gov.tr'den 6 Kasım 2026'da okundu):
  - **Madde 67(6):** mozaikteki ortofotolar arasındaki kayıklık en çok ±2·YÖA. Madde bir mozaiğin komşu görüntülerini
    konuşur; zamanları farklı iki görüntüye ayrı bir sınır yazmaz.
  - **Madde 67(5), 91(2):** her ortofotonun kendi doğruluğu; ADR 1480'in denetimi.
- **ASPRS Positional Accuracy Standards:**
  - Edition 1 (2014): ortofoto mozaiğinin dikiş hattındaki en büyük uyumsuzluk sınıfın RMSE'sinin iki katıdır. Örneğin
    I. sınıfta RMSE 1 piksel, uyumsuzluk 2 piksel
    ([Woolpert sunumu](https://woolpert.com/wp-content/uploads/2017/05/The-New-Standard-of-Map-Accuracy.pdf)).
  - Edition 2 (2024): aynı kural, sınıfın RMSE'sinin iki katı
    ([Penn State GEOG 892](https://courses.ems.psu.edu/geog892/book/export/html/708)).
  - Bunlar ikincil kaynaklardır; ASPRS'in kendi metni okunmadı. Türkiye'de BÖHHBÜY geçerlidir, ASPRS yalnız
    karşılaştırma içindir. İki kaynak da komşu görüntülerin farkına iki piksel (iki RMSE) der.
- **ArcGIS Pro ortho mapping:** blok dengelemede bazı yer kontrol noktaları denetim noktasına çevrilir; dengeleme raporu
  onların sapmasını gösterir ([Esri blog](https://www.esri.com/arcgis-blog/products/arcgis-pro/imagery/ortho-mapping-arcgis-block-adjustment)).
  Bu araç tek bir bloğu dengeler; iki zamanın farkını ölçmez.
- **Agisoft Metashape:**
  - İki dönem, iki parçada (chunk) değişmeyen yerlerdeki aynı ölçülmüş işaretlerle hizalanır.
  - Dönemler arasında tek adımlı fark yoktur; DEM farkı önerilir.
  - Kaynak [topluluk el kitabıdır](https://photogrammetrycommunity.github.io/metashape-expert-manual/topics/scripting/chunk-diff-volume-workflows/),
    Agisoft'un resmî belgesi değil.
- **Netcad:** iki ortofotonun noktalarla karşılaştırılmasının herkese açık belgesi bulunamadı.
- **Plan (MAP-12) kabul satırı:** zamanları farklı iki ortofotoyu bağımsız kontrol noktalarıyla karşılaştır.

## Karar

1. **`cad.orthophoto.compare` v1, salt okunur** (CAD + CBS: raster bağlam sekmesi).
   - **Girdi:**
     - iki raster: `first` eskisidir ya da mozaikte komşusuna göre bakılanı, `second` öbürü;
     - her birinde okunan noktaların katmanı (`firstPoints`, `secondPoints`);
     - isteğe bağlı arazi noktaları (`known`);
     - ad alanı (`field`, öntanımlı `Ad`), ölçek ve kural paketi.
   - **Eşleme:** noktalar adlarına göre eşlenir (her yanda bir adın ilki). Kayma, ikinci − birinci (doğu, kuzey) ve
     uzunluğudur.
   - **İstatistik:** `cad.orthophoto.check`'in kodu kullanılır:
     - KOH_X,Y, X ve Y'nin RMSE'si, radyal RMSE;
     - ortalamalar: görüntüler arasındaki sistematik fark;
     - en büyük kayma.

     Kot karşılaştırılmaz.
2. **Sınır veridir.**
   - Kural paketine `mosaic` (YÖA'nın katı) eklendi. KentOS'un paketi (`ortofoto-bohhbuy-2018.json`, sürüm 2) m. 67(6)'nın
     2'sini taşır.
   - Bir noktanın sınırı `mosaic` × iki görüntüden iri olanın YÖA'sıdır (YÖA, pikselin alanının karekökü). Yönetmelik iki
     farklı pikseli konuşmaz. İri olanı almak KentOS'un okumasıdır ve cevabın notunda söylenir.
   - Sınırı aşan nokta `over` ile işaretlenir. `passed` bütün noktaların sınır içinde olup olmadığıdır; nokta yoksa
     verilmez.
   - `mosaic`'i olmayan ya da pozitif olmayan paket `invalid_input` ile reddedilir: dayanağı yazılmadan sınır uydurulmaz.
     Paketin öbür alanları aynı kaldı; `.kcad` değişmedi.
3. **Her görüntünün kendi doğruluğu:** `known` verilirse her görüntü ayrıca `cad.orthophoto.check` ile denetlenir
   (`firstCheck`, `secondCheck`). Uyum noktaları her rasterin kendi oturtmasından gelir.
4. **Yorum sınırı:** iki zamanın farkı kendiliğinden deformasyon ya da yer değiştirme değildir; görüntülerin kendi
   doğruluğuyla birlikte okunur (cevabın notu). Ortalama kayma sistematik farktır.
5. **Retler:**
   - `invalid_uid`, `entity_not_found`;
   - `not_a_raster` (`first`, `second`);
   - `crs_mismatch` (rasterin sistemi projeninki değil);
   - `invalid_input`: aynı raster iki kez (kimliğin çevresindeki boşluk sayılmaz), ölçek 0, kimliği, kaynağı ya da
     `mosaic`'i olmayan paket;
   - `layer_not_found` (`firstPoints`, `secondPoints`, `known`).
6. **Masaüstü: Ortofoto karşılaştır…** (`raster.compare`):
   - **Simge:** `orthoCompare`, iki zamanın çerçevesi ve aradaki kayma oku. Seçenek sayfası
     `.run/gui-evidence/ortofoto-karsilastirma-1780/secenekler.png`.
   - **Yer:** raster seçilince Raster bağlam sekmesinde, Doğruluk'un altında “Karşılaştır” (Harita ve CBS).
   - **Açılış:**
     - Seçili rasterler çizimdeki sırayla alınır; biri eksikse çizimin öbür rasteri. Çizimde tek raster varsa pencere
       açılmaz ve neden söylenir.
     - Okunan katman, rasterin dosya adındaki yılı taşıyan katmandır (`ortofoto-2024.tif` → `okuma-2024`); yoksa sıradaki
       nokta katmanı.
     - Arazi katmanı adından bulunur (GNSS, arazi, denetim).
   - **Form:**
     - iki rasterin satırları eşittir: raster ve okunan noktalar;
     - arazi noktaları (“— yok —” seçilebilir) ve ölçek;
     - ad alanı.
   - **Özet:**
     - en büyük kayma sınırıyla, sistematik kayma, YÖA'lar;
     - her görüntünün KOH'u m. 91'in sınırıyla;
     - eşi olmayan noktalar, deformasyon notu.
   - **Tablo:** noktanın kayması, iki görüntünün kendi hataları ve durumu (cm).
   - **Düğmeler:** Yer değiştir iki görüntünün yerini değiştirir; CSV kaydet kaymaları yazar.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/orthophoto_compare_cases.py`, 29 durum; KentOS kodu yok. Sınır ve
  istatistik elle; her görüntünün denetimi ADR 1480'in kurallarıyla.
  - İki zaman, arazi noktalarıyla ve onlarsız; uygula ve doğrulama salt okunur.
  - Sınır ve YÖA:
    - 12 cm sistematik kayma üç noktayı sınırın dışına taşır;
    - kaba okunmuş nokta sınırı aşar;
    - ikinci görüntü 25 cm olunca aynı nokta sınırın içinde kalır;
    - 25 cm'lik görüntünün YÖA'sı 1/1000 için kabadır.
  - Paket ve sıra:
    - 1/500;
    - başka kural paketi (`mosaic` 1,5);
    - sıra değişince kaymanın işareti döner.
  - Adlar ve kimlikler: başka ad alanı, eşi olmayan noktalar, eşleşen nokta yok, raster sistemi 0, kimliğin çevresindeki
    boşluk.
  - 13 ret.

  Rust ve Python geçer. `cad.orthophoto.check`'in 27 durumu paketin yeni kaynağıyla ve `mosaic`'iyle yeniden yazıldı;
  ikisinde de geçer.
- **Masaüstü** (`ortho_compare_window_tests`, 3 test; yıl ayıklayıcısının testi ayrı):
  - pencere ortak durumlarla aynı sonucu verir: arazi noktalarıyla, onlarsız ve Yer değiştir'den sonra;
  - seçili raster önce gelir; tek raster yetmez;
  - CSV yazılır, çizime hiçbir şey yazılmaz;
  - Harita ve CBS sunar.

  Resimler `.run/gui-evidence/ortofoto-karsilastirma-1780/` altındadır. Ortak masaüstü denetimleri 133 testle geçer.
- **MCP:** `kentos-mcp` testleri geçer.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("mahalle.kcad")
r = cad.orthophoto.compare(doc, first=eski, second=yeni, first_points="okuma-2024", second_points="okuma-2026",
                           known="gnss")
print(r.limit, r.passed, r.shift.mean_x, r.shift.mean_y, r.first_check.passed_xy, r.second_check.passed_xy)
```

MCP: `{"tool": "cad.orthophoto.compare", ...}`; salt okunurdur.

## Sonra

- **Eşleme:** noktaların görüntüde kendiliğinden eşlenmesi (aday denetim noktaları, görüntü korelasyonu).
- **Dikiş hattı:** mozaik dikiş hattı boyunca kayıklığın örtüşme bandında kendiliğinden ölçülmesi.
- **Düşey:** DSM ve DTM karşılaştırması, kot farkı.
- **Değişen yerler:** yıkılan ya da yeni yapının ayırt edilmesi. Bugün değişmeyen noktaları kullanıcı seçer.
- **Rapor:** fotogrametri aracının kalite raporunun aktarımı (ADR 1480'den kalan).
