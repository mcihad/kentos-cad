# ADR 9139: GeoPackage teslimi ne gittiğini söyler; salt okunur dosya ezilmez; eğri türleri adıyla kaydedilir

- **Durum:** kabul edildi (2026-11-22); adım VY-38. Plan: [plan/veri.md](../../plan/veri.md) VY-38.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  6. maddesi (P1):
  - dışa aktarmada seçim varken ilk kapsam “seçili”ydi;
  - “Tüm proje” üç tablo yazdı; kişi formların, çalışma alanının ve ilişkilerin de gittiğini sanabilirdi;
  - “katman tablosu oluştur”, “verinin kopyasını dışa aktar” ve “projeyi kaynakta sakla”nın etkileri ayrılmalı;
  - ad çakışması, salt okunur dosya ve Türkçe tablo adları sınanmalı; teslim başka bir okuyucuda doğrulanmalı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `file.export.*` var olan salt okunur dosyanın üzerine yazmaz (`file_unwritable`).

## §1 Bağlam

- Pencerenin özeti yalnız “N nesne, K katman yazılır: …” diyordu. Geometrisi olmayan nesneler (yazı, ölçü) de bu
  sayıya giriyor, sonra yazılmıyordu.
- Dosya bitmiş bir kopya olarak yazılıp hedefin üzerine taşınır. Klasör yazılabilirse işletim sistemi salt okunur
  dosyayı da değiştirmeye izin verir; bu yüzden salt okunur hedef eziliyordu.
- **GDAL'ın uyarısı.** Teslim edilen dosyayı GDAL'ın `ogrinfo`'su okurken uyardı: karma (GEOMETRY) katmanlar
  `gpkg_geom_GEOMETRY` uzantısıyla kaydedilmişti, okuyucu bu adı tanımıyor ve “veri eksik okunabilir” diyordu.
  GeoPackage'ın eğri türleri uzantısı adı yazılan eğri türünden kurar (Ek F.1): `gpkg_geom_CIRCULARSTRING`,
  `…COMPOUNDCURVE`, `…CURVEPOLYGON`, `…MULTICURVE`, `…MULTISURFACE`.
- **Araştırma.**
  - Başvuru alınan CBS uygulamasında yeni katman oluşturma, var olan bir pakete tablo ekler.
  - Dışa aktarma verinin kopyasını başka bir dosyaya yazar, projeyi o dosyaya bağlamaz.
  - Kaynaklar: [katman oluşturma kılavuzu](https://documentation.qgis.org/3.44/en/docs/user_manual/managing_data_source/create_layers.html),
    [GDAL GeoPackage sürücüsü](https://gdal.org/en/stable/drivers/vector/gpkg.html) (değerlendirmenin bağlantıları).

## §2 Karar

1. **Özet** (`exchange/gpkg_out.rs`). Dosya sorulmadan önce pencere şunları söyler:
   - yazılacak her katman nesne sayısıyla; boş şablon katmanı “boş tablo”;
   - dosyaya giden: her katman bir tablo, alanları türleriyle; katmanın stili; iki ucu da yazılan ilişkiler (sayısıyla);
   - geometrisi olmadığı için yazılmayan nesneler türleriyle (“3 yazı, 2 ölçü”), uyarı olarak;
   - dosyaya gitmeyen: formlar, birleşik görünümler, çalışma alanı (görünüm, paftalar); bunları proje dosyası tutar;
   - bunun verinin bir kopyası olduğu, proje bu dosyaya bağlanmadığı. Projeyi verisiyle bir GeoPackage'ta saklamak
     için Dosya › Veri kaynağı, bir pakete kalıcı yeni tablo için Veri › Yeni katman › Türüyle.

   Seçim varken ilk kapsamın “Seçili nesneler” olması korunur: özet ne yazılacağını sayılarıyla söyler.
2. **Salt okunur hedef.** Bütün dosyaya aktarmaların ortak hedef denetimi (`headless::file_commands::target`) var olan
   salt okunur dosyayı `file_unwritable` ile reddeder: “… salt okunur; üzerine yazılmadı. Dosyanın yazma iznini açın
   ya da başka bir ad verin.” Dosya olduğu gibi kalır.
3. **Eğri türleri.** GeoPackage yazıcısı bir tablonun satırlarında yazdığı her eğri türünü kendi adıyla kaydeder
   (`Geometry::curved_type`). `gpkg_geom_GEOMETRY` yazılmaz.
4. **Türkçe adlar.** Katmanın adı tablonun adıdır, Türkçe harfleriyle (“Yol ekseni çğüşıö”).

## §3 Kanıt

- **Masaüstü** (`exchange/gpkg_tests/delivery.rs`):
  - özet katmanları sayılarıyla, gidenleri, geometrisiz kalanları, gitmeyenleri ve kopya olduğunu söylüyor;
  - Türkçe adlı katman aynı adlı tablo oluyor, satır sayısı ve sistemi doğru;
  - eğri türleri adlarıyla kayıtlı, `gpkg_geom_GEOMETRY` yok;
  - salt okunur dosya reddediliyor, baytları aynı kalıyor.
- **Bağımsız okuyucu:** aynı teslimi GDAL'ın `ogrinfo`'su uyarısız okudu:
  - Parsel (Curve Polygon), Bina, “Yol ekseni çğüşıö” 5 nesne, TUREF sistemi, `layer_styles`;
  - `sqlite3` kayıtları: Bina'da CIRCULARSTRING ve CURVEPOLYGON, Parsel'de CURVEPOLYGON, yolda COMPOUNDCURVE.
- GeoPackage çekirdeği, uygulamanın GeoPackage, kaynak, birleşim ve sonuç yeri sınamaları (121), başsız komutlar ve
  masaüstünün aktarma sınamaları (174) geçiyor.
- Görünür değişiklik özet satırlarıdır; sınamada okundu, resim çekilmedi.

## §4 Kalan

- Son onayda hedef dosyanın adı ve var olan/yeni tablo ayrımı: dosya işletim sisteminin penceresinde seçildiği için
  özet ondan önce. Dosya seçildikten sonra kısa bir onay ayrı iştir.
- Yarıda kesilen dışa aktarmanın sınaması (işlem sonucu yerinin durdurma sınamaları var; `file.export.gpkg`'nin
  durdurulması yok).
- Web (TODOS.md §0.2).
