# ADR 0535: Raster analizinde alan ve ızgaraların kaydı

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-30'un ilk dilimi: yüzey analizinin sekiz aracı. Kapsamı ben
  belirledim.
- **Bağlam belgesi:** ADR 0231 (yüzey analizi, raster işinin şerit şerit motoru), 0529 (çalışma kaydı), 0515 (COG),
  0232 (interpolasyonun Kapsam'ı: rasterin ızgarası).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - aynı alan ve ızgara bağlamı öbür raster ailelerinde: raster işlemleri, hidroloji, uzaklık, uygunluk, uzaktan algılama
    ve interpolasyonun hücre boyu;
  - ortak bir analiz ortamı: hücre boyu, hizalama rasteri ve NoData, araç parametreleri yerine `cad.processing.run`'ın
    girdisinde;
  - tam geçiş isteyen araçlar (hidroloji, uzaklık) için diskte ara dosya;
  - bağlı ya da uzak raster sağlayıcısının (CBS-02, CBS-28) karo kenarlarının bağımsız karşılaştırması;
  - sensör ya da datum belirsizliğinin ve uygunluk ağırlığının raporu;
  - Renkli kabartma'nın sınırsız rampası alanın bandına göre gerilir; bütün rasterle aynı renk istenirse En küçük ve En büyük
    elle verilir.

## Bağlam

Yüzey analizinin araçları (ADR 0231) her zaman bütün rasteri çözümler. Büyük bir DEM'in yalnız bir parsel ya da proje
sınırındaki eğimi istendiğinde bütün raster hesaplanır ve yazılır. Rasteri önce kırpmak da sonucu bozar: kırpılan kenarın
hücreleri komşusunu kaybeder ve kenar kuralıyla hesaplanır. Plan, analizlerin aynı ızgara, NoData ve birim sözleşmesinde
güvenilir olmasını ve blok boyutunun sonucu sınırların ötesinde değiştirmemesini ister. Ayrıca işlem kaydının hangi
ızgaradan ne yazıldığını söylemesi gerekir.

## Karar

1. **Alan raster işinin girdisinde.** `kentos_raster::job::Spec`'in `window`'u (en küçük doğu, kuzey, en büyük doğu, kuzey):
   - alanın köşeleri afinin tersiyle piksele çevrilir, tam hücrelere genişletilir, rastere kırpılır (`window_of`);
   - hücre kenarından 1e-9 hücre içindeki bir kenar ötesindeki hücreyi almaz;
   - rasterin dışındaki alan reddedilir: “Analiz alanı rasterle kesişmiyor…”.
2. **Komşular kaynaktan okunur.**
   - İş yalnız pencerenin şeritlerini yürütür. Her şeridin bölgesi pencerenin hücreleri ve çevresindeki halodur; halo rasterin
     içindeyse kaynaktan okunur, dışındaysa okuyucu kenarı yineler (bütün rasterdeki gibi).
   - Satırın eksenleri ve enlemi kaynağın satırınındır: pencerenin değerleri bütün rasterin aynı hücrelerinin değerleriyle bit
     bit aynıdır.
   - Sonucun ızgarası kaynağın ızgarasıdır, başlangıcı pencerenin ilk hücresidir (`window_affine`; bütün raster için kaynağın
     afinidir).
3. **Araçlarda Kapsam.**
   - Sekiz aracın Kapsam'ı Bütün raster ya da Nesnelerin kutusu'dur. Nesnelerin kutusu, Alan nesneleri'nin (seçim ya da
     katman) kutusudur.
   - Sonucun raster nesnesi pencerenin ızgarasıyla eklenir.
   - Eş yükselti eğrileri pencerenin ızgarasında çıkar.
4. **Izgaralar kayıtta.**
   - Çalışma kaydının (ADR 0529) `grids`'i her rasteri yazar: girdi (dosya, afin, boy, bant, nodata, sistem ve çözümlenen
     hücreler `window`) ve sonuç (dosya, afin, boy).
   - Araç kaydı `RunResult.grids` ile verir; toplu çalıştırmanın kaydı satırların ızgaralarını toplar.
   - Kayıt `.kcad`'e yazılmaz.

## Doğrulama

- `cargo test -p kentos-raster --test all window`:
  - `terrain/v1/cases.json`'un 39 durumu (sınırsız rampalı Renkli kabartma dışında hepsi) iki alanla koşturulur: biri içeride,
    biri üst ve sağ kenarı aşan;
  - pencerenin her örneği bütün rasterin aynı hücresiyle bit bit eşleşir;
  - sonucun afini pencerenin ızgarasıdır;
  - dışarıdaki ve ters alan reddedilir.
- `python3 scripts/fixtures/surface_processing_cases.py --check`:
  - beş yeni İşlemler durumu: Eğim, kenarı aşan Gölgeli kabartma, Eğrilik, Güneşlenme ve dışarıdaki alanın reddi;
  - pencere KentOS kodu olmadan, afinin tersiyle bulunur;
  - `rasterOf`'un `window`'u bağımsız yüzey başvurusunun (gdaldem'le çapraz denetlenmiş) o hücrelerini ister.
  - `cargo test -p kentos-processing --test cases the_surface_cases` ile oynatılır.
- `cargo test -p kentos-processing --test batch a_raster_run_records_its_grids`: kayıtta girdinin penceresi ve sonucun
  ızgarası.
- `cargo test -p kentos-desktop slope_on_an_area`. Resimler:
  `KENTOS_SHOTS_ONLY=yuzey-alan cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` ve `yuzey-alan-cizim`
  (`.run/shots/arac-yuzey-alan-*`).
