# 3440 — İmar denetiminin cevabında yola giren bölgeler

Tarih: 2026-11-09. plan/map-cad.md MAP-09, altıncı tur. Durum: taslak. ADR 3040'ın “Sonra”sındaki “bölgelerin komutun
cevabına girmesi (Python ve MCP'de çizilebilir halkalar; ortak durumların üreteciyle)”.

İlgili:
- [ADR 2240](2240-zoning-decision-inside-road-width.md): yol genişliği içine giren karar;
- [ADR 2640](2640-zoning-parcel-inside-road-width.md): yola giren parsel;
- [ADR 3040](3040-road-width-overlaps-hatched.md): yola giren kısımların çizimde taranması.

## Dayanak

- **ArcGIS Pro, Export Topology Errors** (ADR 2200'ün okuması): topoloji hataları yalnız listede kalmaz; hatanın
  geometrisi (alan hataları için çokgen) ayrı bir sınıfa yazılır ve başka araçlarla işlenebilir.
- **ArcGIS Pro, Error Inspector** (ADR 3040): “Must Not Overlap With” kuralının hatası iki katmanın nesnelerinin bindiği
  yerdir, haritada yeriyle görülür.
- **KentOS bugün** (ADR 3040):
  - yol genişliği ve yola giren parsel bulgularının bölgelerini masaüstü yan işlevle (`with_regions`) okuyup tarıyordu;
  - komutun cevabı yalnız alanı veriyordu. Python ve MCP hangi kısmın yola girdiğini çizemiyordu;
  - ortak durumlar bölgeleri sınamıyordu.

## Karar

1. **`ZoningFinding.regions`** (ek alan; komut sürümü 1 kalır):
   - `roadWidth` ve `roadParcel` bulguları çekirdeğin kuralının saydığı örtüşme parçalarını taşır. Bunlar alanı
     toplanan parçalardır: tolerans altındaki ince parça yoktur;
   - öbür türlerde alan yoktur.
2. **Biçim** (`ZoningRegion`: `outer` ve `holes`, her biri `RingGeometry`):
   - dış halka saat yönünün tersine, delikler saat yönündedir;
   - her halka en alttaki köşesinden başlar (en alttakilerin en batıdakinden);
   - düz giden bir köşe (iki kenarı da düz, dönüş yok) ve üst üste gelen köşe atılır;
   - delikler ve bölgeler ilk köşelerine göre sıralıdır (kuzey, sonra doğu);
   - yay varsa bulge'ı yazılır.

   Biçim bağımsız başvurunun aynısını yazabilmesi içindir: bir halkanın başladığı köşe ve köşe sayısı çekirdeğin
   bindirmesine bağlı kalmaz.
3. **Masaüstü:**
   - Bulguları göster eskisi gibi aynı bölgeleri tarar (`with_regions`, aynı hesaptan);
   - birden çok parçalı bulgunun Alan hücresi parçaları sayar: “1200.00 m² (2 parça)”. Sütun buna göre genişledi.
4. Yasal sonuç ya da resmî uygunluk iddia edilmez (ADR 2640).
5. **Kapsam işareti:** ADR 2640'taki gibi **Harita + CBS**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/zoning_check_cases.py`, KentOS kodu yok):
  - sayılan her parçanın sınırı hücrelerinden izlenir (koordinat sıkıştırmasıyla kesin), aynı biçimde yazılır;
  - yol bulgularının hepsi (43 bulgu) bölgeleriyle;
  - yeni durum: U biçimli yolun iki kolu konut kararına iki yerden girer. Tek bulgu, iki bölge, alttaki önce
    (47 durum);
  - Rust ve Python geçer.
- **Python** (`python/tests/test_zoning_regions.py`):
  - yol bulgularının bölgeleri başvurununki, halkalar saat yönünün tersine;
  - alanlarının toplamı bulgunun alanı;
  - öbür bulgularda bölge yok.
- **MCP** (`crates/native/mcp/tests/zoning_regions.rs`): iki yolun genişlediği durumda bütün bulguların bölgeleri
  durumunki.
- **Masaüstü** (`zoning_window_tests::a_road_entering_a_decision_twice_carries_two_regions`):
  - cevapta iki bölge, alttaki önce;
  - Alan hücresi “(2 parça)”;
  - Bulguları göster iki parçayı tarar;
  - resimler `.run/gui-evidence/bolgeler-3440/`.

  İmar penceresinin öbür testleri ve çapraz denetimler geçer.

## Sonra

- Bölgelerin hata katmanına yazılması (topoloji bulguları gibi, ADR 2200).
- Plan değişikliği taslağı; sembolün karardan seçilmesi; resmî şema ve eşleme (MAP-09'un öbür maddeleri).
- Web (TODOS §0.2).
