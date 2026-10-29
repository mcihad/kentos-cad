# ADR 0741: İşlemler araçlarının açık seçenekleri ve yazmayan adımın değişmezi

- Durum: Kabul edildi (29 Ekim 2026)
- Madde: plan/gis.md CBS-41'in dilimi
- İlgili: [ADR 0548](0548-coverage-matrix-and-run-equivalence.md) (kapsam matrisi), [ADR 0652](0652-every-processing-tool-through-the-command.md)
  (her aracın ortak durumu), [ADR 0253](0253-command-protocol.md) (komut protokolü), [ADR 0508](0508-processing-tools-as-commands.md)
  (İşlemler araçları komut olarak)

## Bağlam

CBS-41'in ADR 0652'den kalan işi “açık seçenek hücreleri”ydi. `scripts/gis/coverage.py` her İşlemler aracının her
seçeneğini (bir değer listesinin her değeri, açık ya da kapalı bir anahtar) bir hücre sayar; hücre, aracın o değerle
koşan bir ortak durumu varsa denenmiş sayılır. 29 Ekim'de 153 aracın 788 hücresinin 501'i denenmişti; 287'si hiçbir
durumda koşmuyordu: raster işlemlerinin istatistikleri, bindirme ve örnekleme kuralları, yüzey analizinin rampaları ve
güneşlenmenin ayları, Tablodan nokta'nın 24 koordinat sistemi, Koordinat sistemine dönüştür'ün 23 kaynak sistemi,
kriging modelleri, çekirdekler ve birimler, “Çukurları doldur” kapalı hidroloji, uzaktan algılamanın indisleri,
hemen her raster aracının “Çizime ekle” kapalı çalışması.

İkinci boşluk koşucudaydı. Ortak komut durumları (`fixtures/commands/v1`) her adımın cevabını denetliyordu ama planın,
doğrulamanın, sorgunun ve reddedilen çalıştırmanın çizimi gerçekten değiştirmediğini hiçbir yer söylemiyordu: bir
komutun `plan`'ı yanlışlıkla belgeye yazsa cevap doğru göründüğü için test geçerdi.

Üreticilerin yolu (resmî belgeler, 29 Ekim 2026'da okundu):

- QGIS'in Processing testleri (`python/plugins/processing/tests/README.md`): her test bir aracın bir çalıştırmasıdır
  (algoritma, parametreler, beklenen sonuç); vektör sonuçlar beklenen GML ile, rasterler sağlama toplamıyla
  karşılaştırılır; tolerans (`compare`) “yalnız gerektiğinde” ve köşe başına verilir. Parametre değerlerinin her biri için
  ayrı test istenmez; kapsamı ölçen bir araç da yoktur.
  <https://github.com/qgis/QGIS/blob/master/python/plugins/processing/tests/README.md>

KentOS'un kararı QGIS'ten bir adım öte: her seçenek değerinin en az bir bağımsız başvurulu durumu olur ve kapsam matrisi
bunu sayar; toleranslar başvurunun kuralından gelir, tek tek gerekçeli.

## Karar

### 1. Yazmayan adım çizimi değiştirmez

Rust'ın ortak durum koşucusu (`crates/native/application/tests/all/fixtures.rs`, `Held`) ve Python'unki
(`python/tests/test_command_cases.py`, `held`, `writes_nothing`) her adımdan önce çizimin revizyonunu, kirli işaretini,
geri alma ve yineleme durumunu tutar. Şu adımlardan sonra dördü aynı olmalı:

- `validate` ve `plan`;
- kataloğun `effect: query` dediği komutun `execute`'u;
- cevabı `failed`, `conflict` ya da `needsInput` olan `execute`.

Komutların kaydı (`kentos.calls/1`, ADR 0253) bu dörde girmez: sorgu ve ret kayda not düşebilir; kayıt çizimin değil,
çağrıların defteridir. İlk koşuda 164 sorgu adımı kayda not düşüyordu (blok sorgusu, form doğrulaması, GIS türevinin
denetimi); hiçbiri revizyonu, kirliliği ya da geri almayı değiştirmiyordu.

### 2. Açık hücreler bağımsız başvurularla kapanır

Her aile kendi başvurusunda (KentOS kodu olmadan) genişledi; yeni durumlar mevcut çizimlerin üstünde, gerektiğinde yeni
rasterle ya da katmanla:

| Aile | Başvuru | Durumlar |
|---|---|---|
| Raster işlemleri | `raster_ops_cases.py` (sınıf rasterleri, en az, aralık, çoğunluk, azınlık, çeşitlilik), `raster_ops_processing_cases.py` | 16 → 70 |
| Raster ve vektör | `raster_vector_cases.py` (`cizgi-aralik-bir`), `raster_vector_processing_cases.py` | 13 → 23 |
| Yüzey analizi | `terrain_cases.py` (Gri, Spektral, Mavi-kırmızı, Sıcaklık rampaları; on bir ay aralığı), `contour_cases.py` (alanlı eğriler) | 20 → 46 |
| Uzaktan algılama | `remote_cases.py`'nin durumları, yeni Kaba bant | 26 → 42 |
| Uygunluk | `suitability_cases.py` (`bulanik-or-64`, `ahp-tutarli-raster-64`) | 21 → 37 |
| İnterpolasyon, yoğunluk | `interpolation_cases.py` (dönük ızgarada TIN, doğal komşu, spline, kriging, çekirdek; dekar, m², m/m²) | 16 → 37 |
| Hidroloji | `hydrology_cases.py` (çukur doldurmasız birikim, nemlilik, döküm noktası, havza, havzalar) | 20 → 32 |
| Uzaklık ve maliyet | `distance_cases.py` (`koridor-yukseklik`) | 18 → 22 |
| Mekânsal istatistik | `spatial_stats_cases.py`, `spatial_inference_cases.py`, `spatial_stability_cases.py` | 62 → 68 |
| Doğrusal referans | `lrs_processing_cases.py` (başlangıç köşeleri, kalibrasyonun uçları ve boşlukları, katmandan olaylar) | 17 → 27 |
| Yakınlık, dönüştürme, geometri | `proximity_cases.py`, `convert_cases.py`, `geoprocess_cases.py` (sağ tampon) | 51 → 58 |
| Genelleştirme, üretim, ağ | `generalize_cases.py`, `generate_cases.py`, `network_processing_cases.py` (tesise doğru hizmet alanı) | 53 → 56 |
| Seçim ve öznitelik | `spatial_query_cases.py` (ekle, çıkar, seçimin içinden; Boşalt; en az, en çok; Yalnız boşlara) | 29 → 37 |
| Çok boyutlu, nokta bulutu | `multidim_cases.py` (En küçük), `pointcloud_ops_cases.py` (COPC'nin sıralı kayıtları) | 35 → 43 |
| Tablodan nokta | yeni `table_points_systems_cases.py`: her sistemde iki yer, PROJ'un adlı EPSG yollarıyla | 0 → 25 |
| Koordinat sistemine dönüştür | yeni `reproject_systems_cases.py`: her kaynak sistemde bir parsel | 0 → 24 |

“Çizime ekle” kapalı her raster aracı aynı çalıştırmanın kopyasıdır (`raster_ops_processing_cases.quiet`): dosya aynı
yazılır, katman, nesne ve geri alma adımı yoktur.

İki sistem ailesinin toleransı bir milimetredir: datum yolları aynı EPSG işlemleridir ama PROJ ile çekirdek onları kendi
sıralarıyla birleştirir (`crs_sweep.py`'nin bilinen farkı milimetrenin onda biri). Dönük ızgaradaki spline'ın sınırı
10⁻¹²'dir (ölçülen 2,2·10⁻¹³; tepenin değil alanın değerleri 12 × 12 sistemlerde bir basamak daha yitirir).

### 3. Denetimler yeni durumlara göre genişledi

- Yüzey araçlarının alan penceresi çok bantlı rasteri hücre hücre keser (renkli kabartmanın dört bandı yan yana).
- Renkli kabartmanın rampası alanın kendi yüksekliklerine gerilir; alanlı durum aralığı yazılı rampayla denetlenir, alan
  ile bütün raster aynı hücreleri verir.
- COPC'nin dizini noktaları sıralar: COPC çıktısında sınıflar noktaların X, Y, Z ve sınıf sırasıyla, kayıtlar
  baytlarının sırasıyla karşılaştırılır (`clouds.rs`, `classes_hash(sort=True)`, `sortedRecords`).
- Doğrusal referansta ölçüsü bilinmeyen çizginin `ma`'sı ya da `mb`'si hiç yazılmaz (null değil).

## Doğrulama

- `cargo test -p kentos-native-application --test all every_command_case`: bütün ortak komut durumları değişmezle geçer.
- `.run/py/bin/python -m unittest python.tests.test_command_cases`: aynı değişmez Python'da.
- `cargo test -p kentos-processing --test cases`: 22 ailenin durumları, iki yeni test
  (`the_table_point_system_cases_do_what_they_say`, `the_reproject_system_cases_do_what_they_say`) ve pencereyle
  komutun eşdeğerliği.
- `cargo test -p kentos-raster --test all`, `-p kentos-pointcloud --test all`, `-p kentos-formats --test all multidim`:
  başvuruların çekirdek denetimleri yeni durum sayılarıyla.
- Başvurular `--check` ile yeniden üretilir; GDAL, GRASS, PROJ ve MDAL çapraz denetimleri değişmeden geçer.
- `.run/py/bin/python scripts/gis/coverage.py --check`: İşlemler araçlarında 788 hücrenin 781'i denenir (287 → 7 açık).

## Sonuçlar

- Bir İşlemler aracının her seçeneği, yedisi dışında, bağımsız başvurusu olan bir durumla koşar; yeni seçenek kapsam
  matrisinde açık görünür.
- Plan, doğrulama, sorgu ve ret çizime yazarsa ortak durumlar düşer.
- Yeni üretim kodu yok; değişen yalnız başvurular, durumlar ve denetimler.

## Kalan

- Köşe noktalarını numarala'nın beş hücresi (En kuzey, İlk çizilen, Seçilen noktaya en yakın; ortak köşesiz; yalnız
  yazı) ve Kenar uzunluklarını yaz'ın ortak kenarsız çalışması: durumları web'in kaydettiği `cases.json`'dadır, bağımsız
  Python başvurusu yoktur; başvuru yazılınca kapanır.
- Çizgi yoğunluğu, dönük rasterin ızgarasında: çizimin çizgileri ızgaranın uzağında, sonuç bütünüyle boş olurdu; ızgaraya
  yakın çizgiler ya da ikinci bir ızgara gerekir.
- Komutların seçenek hücreleri (5894'ün 4141'i açık): ayrı dilim.
