# 4060 — Dikiş ölçümünde dönük birinci görüntü: ızgaranın kendisi

Tarih: 2026-11-09. plan/map-cad.md MAP-12, yedinci tur. Durum: taslak. ADR 3660'ın “Sonra”sındaki “Dönük birinci görüntü
(ızgaranın kendisi dönük: yerler onun pikselleriyle)”.

İlgili:
- [ADR 2460](2460-orthophoto-seam-measured-in-the-images.md): dikiş ölçümü görüntülerin kendisinden;
- [ADR 3260](3260-orthophoto-seam-bicubic-and-arcs.md): çift kübik piksel altı;
- [ADR 3660](3660-orthophoto-seam-resampled-onto-the-first-grid.md): ikinci görüntünün birincinin ızgarasına yeniden
  örneklenmesi.

## Dayanak

- **GDAL**, [Raster Data Model](https://gdal.org/en/stable/user/raster_data_model.html) (9 Kasım 2026'da okundu),
  Affine GeoTransform:
  - Xgeo = GT(0) + Xpixel·GT(1) + Yline·GT(2), Ygeo = GT(3) + Xpixel·GT(4) + Yline·GT(5);
  - kuzeye bakan görüntüde GT(2) ve GT(4) sıfırdır, GT(1) piksel genişliği, GT(5) piksel yüksekliğidir;
  - (GT(0), GT(3)) sol üst pikselin sol üst köşesidir; sol üst pikselin merkezi (0,5; 0,5)'tir.
- **ArcGIS Pro'nun Resample'ı ve Keys 1981:** ADR 3660'ın okuması (CUBIC, a = −1/2).
- **KentOS bugün** (ADR 3660): yeniden örneklemede ızgara birinci görüntünündü ve kuzeye bakması gerekiyordu. Dönük
  birinci “Dönük görüntüyü ikinci verin.” ile reddediliyordu; kullanıcı görüntülerin sırasını, dolayısıyla kaymanın
  işaretini değiştirmek zorundaydı.

## Karar

1. **Dönük birinci ızgaranın kendisidir** (`cad.orthophoto.compare`, `seam.resample`; komut sürümü değişmez):
   - yer, birinci görüntünün içine düştüğü pikselin merkezidir. Piksel, birincinin afininin tersiyle bulunur:
     sütun = ⌊(t5·dx − t2·dy)/det⌋, satır = ⌊(−t4·dx + t1·dy)/det⌋;
   - birincinin pikselleri yere afinin kendisiyle gider (GDAL'ın sırası). Yerin, eşleşen yerin ve ikinciye taşınan blok
     merkezlerinin yeri böyle bulunur;
   - örtüşmenin kutusunda birincinin kutusu dört köşesinindir; pay birincinin uzun piksel kenarıyla. Penceresi birincinin
     piksellerinden taşan yer örtüşmenin dışındadır;
   - ikinci görüntü (kuzeye bakan ya da başka piksel boyunda) ADR 3660'taki gibi kübik evrişimle bu ızgaraya örneklenir;
   - eşleme ve piksel altı (parabol ya da çift kübik) birincinin pikselleriyledir. Kayma yerde, doğu ve kuzey olarak
     verilir.
2. **Kuzeye bakan birincide hesap aynen kalır:** aynı formüller, aynı sıra. Eski durumlar bayt bayt değişmez.
3. **İstenmeden:** dönük birinci `resample` olmadan eskisi gibi “…resample verin” ile reddedilir. “Dönük görüntüyü ikinci
   verin” reddi kalktı. Tersinir olmayan dönüşümlü birinci yeni bir retle söylenir.
4. **Cevap:** `seam.turned` (birinci dönük) ve notta “Birinci görüntü dönük: yerler ve eşleme onun pikselleriyle; kayma
   doğu ve kuzey olarak verildi.”
5. **Masaüstü** (Ortofoto karşılaştırması):
   - Yeniden örnekleme açıklaması: “birinci dönükse ızgara onun pikselleridir”;
   - özet: “Birinci görüntü dönük: ikincisi onun ızgarasına kübik evrişimle yeniden örneklendi; yerler ve eşleme onun
     pikselleriyle, kayma doğu ve kuzey olarak.”
6. **Kapsam işareti:** ADR 1780 gibi **CAD + CBS** (raster bağlam sekmesi).

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/orthophoto_compare_cases.py`, KentOS kodu yok):
  - başvurunun ölçümü dönük birinci ızgarayı alır (aynı formüllerle elle);
  - ADR 3660'ın 9° dönük sentetik görüntüsü birinci, kuzeye bakan 0,10 m'lik ya da 0,08 m'lik görüntü ikinci. Bilinen
    kayma ikincininki eksi birincininki. Temiz yerler bilinen kaymadan şu kadar sapıyor (piksel):

    | Durum | Temiz yer | En büyük sapma |
    |---|---|---|
    | dönük birinci, parabol | 9 | 0,053 |
    | dönük birinci, çift kübik | 9 | 0,0075 |
    | dikiş hattı boyunca, 0,08 m'lik ikinci, çift kübik | 2 | 0,0083 |

    Dönük ızgarada parabol 0,05 piksel sınırını geçiyor; durum gerçeği beklemeyen durum olarak yazıldı (ADR 3260'taki
    11 piksellik pencere gibi), çift kübik geçmiyor;
  - ret “yeniden örneklemede birinci görüntü dönük” yerine “dönük birinci görüntü, yeniden örnekleme istenmeden”;
  - 56 durum (53'ten). Eski 52 durum ve görüntüler bayt bayt aynı; Rust ve Python 1e-9 içinde.
- **Bağımsız Python testi** (`python/tests/test_orthophoto_turned_grid.py`), testin kendi ters afiniyle:
  - her yer dönük görüntünün bir piksel merkezidir, penceresi görüntünün içindedir;
  - ortalama kayma −s'nin 3 mm içinde; sıra değişince (ADR 3660'ın durumu) s;
  - `resample`'sız ret.
- **MCP** (`crates/native/mcp/tests/orthophoto_seam.rs`): üç yeni durumun yerleri, sayıları, `resampled` ve `turned`'ü;
  eski sekiz durum aynen.
- **Masaüstü** (`ortho_compare_window_tests::a_turned_first_image_is_the_grid_as_the_shared_cases_say`): kutu kapalıyken
  ileti `resample`'ı söyler; açılınca parabolde ve çift kübikte cevap durumunkidir, özet dönük ızgarayı söyler. Pencere
  testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/donuk-birinci-4060/`.

## Sonra

- Görüntüden türeyen nesnenin kaynağa izi (MAP-01 kararı bekliyor).
- Mozaiğin dikiş çokgenlerinden hatlar (mozaik veri kümesinin dikiş çokgenleri girdisi gerekir).
- Web (TODOS §0.2).
