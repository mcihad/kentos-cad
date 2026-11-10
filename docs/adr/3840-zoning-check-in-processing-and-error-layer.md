# 3840 — İmar denetimi İşlem araç kutusunda; yola giren bölgeler hata katmanında

Tarih: 2026-11-09. plan/map-cad.md MAP-09, yedinci tur. Durum: taslak. ADR 3440'ın “Sonra”sındaki “Bölgelerin hata
katmanına yazılması (topoloji bulguları gibi, ADR 2200)”.

İlgili:
- [ADR 2240](2240-zoning-decision-inside-road-width.md), [ADR 2640](2640-zoning-parcel-inside-road-width.md): yola giren
  karar ve parsel;
- [ADR 3440](3440-zoning-regions-in-the-answer.md): bölgelerin komutun cevabında;
- [ADR 1980](1980-topology-check-in-processing.md), [ADR 2200](2200-topology-findings-on-an-error-layer.md): topoloji
  denetimi İşlem araç kutusunda ve bulguları hata katmanında.

## Dayanak

- **ADR 2200'ün okuması** (8 Kasım 2026): QGIS'in Check validity aracı hataları ayrı bir çıktı katmanına yazar; ArcGIS
  Pro'nun Export Topology Errors aracı nokta, çizgi ve alan hatalarını kaynak ve hedef nesnenin kimliği ve kuralın
  açıklamasıyla ayrı sınıflara yazar. Hata geometrisi alan olduğunda alan olarak yazılır.
- **ADR 3440:** yola giren karar ve parsel bulguları sayılan örtüşme parçalarını (`regions`) taşır. Masaüstünün
  Bulguları göster'i bunları ekranda tarar ama çizime yazmaz; bölge paftada, GeoPackage'da ya da başka bir programda
  gösterilemiyordu.
- **Plan (MAP-09):** denetim sonuçları Processing, Python ve MCP'den aynı yoldan alınabilmeli. İmar denetimi İşlem araç
  kutusunda yoktu.

## Karar

1. **Yeni İşlem aracı “İmar planını denetle”** (`zoning.check`, Geometri grubu, Topolojiyi kurallarla denetle'nin
   yanında; masaüstünün `processing.run.zoning.check`'i):
   - denetim `cad.zoning.check`'inkidir (`zoning_check::with_regions`): aynı bulgular, sözler, alanlar ve bölgeler;
   - parametreler: Tolerans (m, öntanımlı 0,01), Bulguları hata katmanına yaz (öntanımlı kapalı), Hata katmanı
     (öntanımlı yeni “İmar bulguları”);
   - tablo: Tür, Nesneler, Alan, Bölge (sayısı), İleti; bulgulu nesneler kimlik sırasıyla seçilir; sayı çıktısı.
2. **Hata katmanı** (kutu açıkken): her bulgunun her bölgesi bir alan olarak yazılır:
   - dış halka ve delikleri cevabınkidir (ADR 3440'ın biçimi);
   - öznitelikler: Tür, Nesneler (kalıcı kimlikler), Bulgu alanı, Bölge (“1/2”), İleti;
   - katmanın görünüşü: kırmızı (`#E5484D`) 0,35 mm çizgi, soluk dolgu (`#E5484D26`). Altındaki karar ve parsel
     okunur kalır;
   - bölgesi olmayan bulgular (değer verilmemiş, plan notu yok …) tabloda kalır, özet sayar: “…; 3 bulgunun 4
     bölgesi hata katmanına yazıldı, 2 bulgunun yeri yok (tabloda).”;
   - tek geri alma adımı “İmar planını denetle”; yazınca seçim yapılmaz. Kapalıyken çizim değişmez.
3. **Araç = komut:** yazan `cad.processing.run`'dır, aracın kimliği ve `write` ile. Python ve MCP aynı yolu kullanır
   (ADR 2200 §5).
4. **Tür sözleri** (Yola giren karar, Yola giren parsel …) masaüstünün İmar denetimi penceresinden ortak uygulama
   katmanına taşındı (`zoning_check::kind_word`). Pencere ile araç aynı sözü söyler.
5. Yasal sonuç ya da resmî uygunluk iddia edilmez (ADR 2640).
6. **Kapsam işareti:** ADR 2640'taki gibi **Harita + CBS**.

## Kanıt

- **İşlem durumları** (`scripts/fixtures/zoning_processing_cases.py` → `fixtures/processing/v1/zoning.json`,
  `zoning-yollar.kcad`, `zoning-u-yol.kcad`; KentOS kodu yok):
  - çizimler ve bulgular `cad.zoning.check`'in ortak durumlarındandır (kendi bağımsız başvurusu);
  - betik tabloyu, seçimi, özeti, katmanı ve her alanın özniteliklerini, alanını (shoelace, delikler çıkarılarak) ve
    delik sayısını yazar;
  - üç durum: kutu kapalı (yedi bulgu, sekiz nesne seçilir); kutu açık (yedi alan); U biçimli yolun tek bulgusu iki
    alan (1/2, 2/2);
  - durumlar doğrudan ve çizimin okuma kopyasında koşar (`the_zoning_cases_do_what_they_say`); öntanımlar testi iki
    çizimin öntanımlarını ve aracın öntanımlarını denetler. Processing'in bütün testleri geçer.
- **Python** (`python/tests/test_zoning_processing.py`): kutu kapalıyken seçim bulguların nesneleri; açıkken her alanın
  öznitelikleri ve alanı (testin kendi shoelace'i) durumun bölgesininki; tek geri alma.
- **MCP** (`crates/native/mcp/tests/zoning_errors.rs`): kutu kapalı sekiz seçim, ekleme yok; açıkken özet, adım, katman
  ve yedi alanın öznitelikleri durumunki; geri alma çizimi eski hâline döndürür.
- **Masaüstü** (`processing::geometry_tests::the_zoning_regions_go_to_an_error_layer_in_one_step`): U yolunda tek bulgu,
  iki alan, tek geri alma. Araç kutusunun sayısı 156 oldu: test ADR 1980'in ve ADR 3100'ün araçlarından beri 153'te
  kalmıştı, açıklamasıyla düzeltildi. İşlem, imar ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/imar-hata-3840/`.

  Not: `processing::sink_tests::a_bound_input_is_read_whole_into_a_file_and_the_drawing_stays` çok testli bir koşuda bir
  kez “readonly database” ile düştü, tek başına geçer: aynı GeoPackage'a yazan testlerin yarışı, bu değişiklikle
  ilgisiz.

## Sonra

- Plan değişikliği taslağı; ~~sembolün karardan seçilmesi~~ (ADR 4240); resmî şema ve eşleme (MAP-09'un öbür
  maddeleri).
- ~~Hata katmanının GeoPackage'a yazılması (İşlem penceresinin Sonucun yeri ile; ADR 2200 Sonra)~~
  ([ADR 4640](4640-zoning-symbol-mismatch-and-error-geopackage.md)).
- Web (TODOS §0.2).
