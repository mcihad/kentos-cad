# 2860 — Ortofoto dikiş ölçümünün çizilmiş bir dikiş hattı boyunca yapılması

Tarih: 2026-11-08. plan/map-cad.md MAP-12, dördüncü tur. Durum: taslak. ADR 2460'ın “Sonra”sındaki “çizilmiş bir dikiş
hattı boyunca ölçüm (bugün örtüşmenin bütünü)”.

İlgili:
- [ADR 1780](1780-two-orthophotos-compared.md): iki ortofotonun karşılaştırılması;
- [ADR 2460](2460-orthophoto-seam-measured-in-the-images.md): dikiş ölçümü görüntülerin kendisinden.

## Dayanak

- **ArcGIS Pro, mozaik veri kümesinin dikiş hatları**
  ([Mosaic dataset seamlines](https://pro.arcgis.com/en/pro-app/3.4/help/data/imagery/mosaic-dataset-seamlines-pro-.htm),
  [Build Seamlines](https://pro.arcgis.com/en/pro-app/3.4/tool-reference/data-management/build-seamlines.htm),
  8 Kasım 2026'da okundu):
  - her görüntünün mozaikte kullanılan bölümü bir dikiş çokgeniyle sınırlanır;
  - hatlar radyometri, kenar algılama, Voronoi ya da paralaks farkıyla hesaplanabilir, elle de düzenlenir;
  - iki görüntünün birleştiği yer dikiş hattıdır; harmanlama genişliği hattın çevresinde uygulanır.

  İki görüntünün kayması en çok mozaiğin göründüğü yerde, dikiş hattında önemlidir. Örtüşmenin hattan uzak yeri
  mozaikte görünmez.
- **KentOS bugün** (ADR 2460): dikiş ölçümü örtüşmenin bütününde, adım aralıklı bir ızgarada yapılıyordu. Çizilmiş bir
  hat okunmuyordu.

## Karar

1. **`seam.line`** (`cad.orthophoto.compare`, ek alan; komut sürümü değişmez): çizilmiş dikiş hattının kalıcı kimliği.
   Hat bir çizgi ya da düz kenarlı bir çoklu çizgidir; kapalı çoklu çizgi başına döner.
2. **Yerler:**
   - hattın başından `step` metrede bir: k'ıncı yer k × step uzaklıktadır, hattın sonu tam bir adıma düşerse o da
     yerdir;
   - yer, o uzaklıktaki hat noktasıdır. Uzunluğu olmayan kenar atlanır;
   - yalnız örtüşmenin içindeki yerler ölçülür (ızgaradaki gibi pencere ve arama payıyla), hattın sırasıyla;
     dışındakiler sayılır;
   - sonrası ADR 2460'ınkidir: yer birinci görüntünün pikselinin ortası, eşleme, piksel altı, durum.
3. **Cevap:**
   - her yerin `station`'ı: hat boyunca uzaklığı;
   - `seam.line`: kimlik, uzunluk, yer sayısı, örtüşme dışındaki yerler;
   - not hattın uzunluğunu ve dışarıda kalan yerleri söyler.
4. **Retler** (`invalid_input`, `seam.line`): hat çizimde yok; çizgi ya da çoklu çizgi değil; yaylı; uzunluğu yok;
   hiçbir yeri örtüşmede değil. Örtüşme ve piksel retleri (ADR 2460) önce gelir.
5. **Masaüstü** (Ortofoto karşılaştırması):
   - **Dikiş hattı** satırı: çizimde seçili tek çizgi ya da çoklu çizgi **Seçili çizgiyi al** ile alınır, **Hattı
     kaldır** ile örtüşmenin bütününe dönülür;
   - özet hattın uzunluğunu ve dışarıda kalan yerleri söyler; dikiş tablosunda **Hat (m)** sütunu.
6. **Kapsam işareti:** ADR 1780'deki gibi **CAD + CBS** (raster bağlam sekmesi).

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/orthophoto_compare_cases.py`, KentOS kodu yok):
  - ADR 2460'ın sentetik görüntüleri; örtüşmenin kuzeyinden güneyine kıvrımlı bir hat (15,31 m);
  - 16 yerden 9'u örtüşmede, 7'si dışarıda;
  - değişen bölgeden geçen 5 yerin korelasyonu düşük; öbür 4 yerin kayması bilinen sentetik kayma
    (0,034; −0,021 m);
  - dört ret: hat yok, nokta, yaylı, örtüşmeden geçmiyor;
  - 45 durum (40'tan). Eski durumlar aynı. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/orthophoto_seam.rs`): hat boyunca durumun yerleri, uzaklıkları, sayıları ve
  hattın bilgisi; eski iki durum aynen.
- **Masaüstü** (`ortho_compare_window_tests.rs`):
  - seçim yokken hat alınmaz, söylenir;
  - seçili hatla cevap durumunki, tablonun ilk yeri 4 m;
  - Hattı kaldır ızgaraya döner.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/dikis-hatti-2860/`.

## Sonra

- ~~Yaylı dikiş hattı (yay boyunca uzaklık).~~ ADR 3260.
- ~~Görüntü ara değeriyle (çift kübik) piksel altı~~ (ADR 3260); farklı piksel boyunda ya da dönük görüntülerde yeniden örnekleme
  (ADR 2460 Sonra).
- ~~Mozaiğin dikiş çokgenlerinden hatların kendiliğinden okunması.~~ ADR 4460.
- Web (TODOS §0.2).
