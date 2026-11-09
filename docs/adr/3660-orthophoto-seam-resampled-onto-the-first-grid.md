# 3660 — Dikiş ölçümünde ikinci görüntünün birincinin ızgarasına yeniden örneklenmesi

Tarih: 2026-11-09. plan/map-cad.md MAP-12, altıncı tur. Durum: taslak. ADR 2460 ve 3260'ın “Sonra”sındaki “farklı piksel
boyunda ya da dönük görüntülerde birini öbürünün ızgarasına yeniden örnekleme; kübik evrişim bunun da çekirdeği
olabilir”.

İlgili:
- [ADR 2460](2460-orthophoto-seam-measured-in-the-images.md): dikiş ölçümü görüntülerin kendisinden;
- [ADR 2860](2860-orthophoto-seam-along-a-drawn-line.md): çizilmiş dikiş hattı boyunca;
- [ADR 3260](3260-orthophoto-seam-bicubic-and-arcs.md): çift kübik piksel altı ve yaylı hat.

## Dayanak

- **ArcGIS Pro**, [Resample (Data Management)](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/resample.html)
  (9 Kasım 2026'da okundu):
  - CUBIC, bir hücrenin yeni değerini en yakın 16 giriş hücresi merkezinden geçen düzgün bir eğri uydurarak bulur.
    Çıktıda girişin aralığı dışında değerler olabilir;
  - BILINEAR en yakın dört hücre merkezinin uzaklıkla ağırlıklı ortalamasıdır, sürekli veride kullanılır;
  - NEAREST başlıca ayrık veri içindir (arazi kullanımı sınıflaması gibi), hücre değerlerini değiştirmez.
- **Keys 1981** (ADR 3260'ın okuması): kübik evrişim çekirdeği, a = −1/2; 4 × 4 piksel.
- **GDAL'ın gdalwarp sayfası** TLS sertifikası doğrulanamadığı için okunamadı; dayanağa girmedi.
- **KentOS bugün:**
  - dikiş ölçümü pikselleri aynı boyda ve kuzeye bakan iki görüntü istiyordu, başkası `invalid_input` ile
    reddediliyordu (ADR 2460);
  - çift kübik piksel altı ikinci görüntüyü kendi ızgarasında kübik evrişimle örnekliyordu (ADR 3260).

## Karar

1. **`seam.resample`** (`cad.orthophoto.compare`, ek alan; komut sürümü değişmez). İki görüntünün pikselleri farklı
   boydaysa ya da ikincisi dönükse, ikinci görüntü birincinin ızgarasına yeniden örneklenir. Verilmezse bu görüntüler
   bugünkü gibi reddedilir; ileti `resample`'ı söyler.
2. **Örnekleme:**
   - ızgara birinci görüntünündür, o kuzeye bakmalıdır. Dönük birinci `seam.resample`'da reddedilir: “Dönük görüntüyü
     ikinci verin.”;
   - her yerde, birincinin yerin penceresini ve aramasını kapsayan piksel bloğu kullanılır (çift kübik piksel altında bir
     piksel fazlası);
   - bloğun her piksel merkezi ikincinin pikseline ikincinin dönüşümünün tersiyle taşınır: sütun = (t5·dx − t2·dy)/det −
     ½, satır = (−t4·dx + t1·dy)/det − ½. Değeri orada kübik evrişimle (Keys, a = −1/2, ArcGIS'in CUBIC'i gibi 16 piksel)
     okunur;
   - bloğu (kübiğin 4 × 4'ü dahil) ikincinin piksellerinin dışına taşan yer örtüşmenin dışındadır. Izgarada ölçülmez,
     dikiş hattında dışarıda sayılır. Örtüşmenin kutusu iki görüntünün kutularının kesişimidir; ikincinin kutusu dört
     köşesinindir;
   - eşleme, piksel altı (parabol ya da çift kübik) ve kayma birincinin ızgarasındadır. Çift kübik, ikinciyi aynı yolla
     birincinin pikselinin kesirlerinde örnekler.
3. **Cevap:**
   - `seam.resampled`: yeniden örneklendi (ızgaralar farklı ve istendi). Aynı ızgaradaki görüntülerde `resample` bir şey
     değiştirmez;
   - notlarda: “İkinci görüntü birincinin ızgarasına kübik evrişimle yeniden örneklendi (Keys, a = −1/2); kayma
     birincinin pikselleriyle ölçüldü.”
4. **Masaüstü** (Ortofoto karşılaştırması): **Yeniden örnekleme** satırında “İkinciyi birincinin ızgarasına örnekle”
   kutusu ve açıklaması. Özet örneklemeyi söyler.
5. **Kapsam işareti:** ADR 1780 gibi **CAD + CBS** (raster bağlam sekmesi).

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/orthophoto_compare_cases.py`, KentOS kodu yok):
  - iki yeni sentetik GeoTIFF bayt bayt yazıldı. Biri 0,08 m pikselli, sahneyi (0,046; 0,012) m kaydırıyor. Öbürü 9°
    dönük, (−0,028; 0,039) m kaydırıyor; dönüşü ModelTransformationTag'de taşıyor;
  - yeniden örnekleme başvuruda aynı formüllerle yeniden yazıldı. Temiz yerler bilinen kaymadan şu kadar sapıyor
    (piksel):

    | Durum | Temiz yer | En büyük sapma |
    |---|---|---|
    | 0,08 m pikselli | 7 | 0,047 |
    | 9° dönük | 8 | 0,017 |
    | dönük, dikiş hattı boyunca, çift kübik | 2 | 0,001 |

    Dönük görüntüde dikiş hattının 8 yerinden 3'ü ikincinin pikselleri yetmediği için dışarıda;
  - aynı ızgaradaki görüntüde `resample` ADR 2460'ın durumunu aynen verir;
  - ret durumları: farklı piksel (yeni ileti), dönük birinci;
  - 53 durum (48'den). Rust ve Python'da 1e-9 içinde aynı; eski durumlar değişmedi.
- **Bağımsız Python testi** (`python/tests/test_orthophoto_resample.py`):
  - `resample`'sız ret;
  - ortalama kayma bilinen kaymanın 3 mm (0,03 piksel) içinde;
  - sıra değişince (0,08 m'lik görüntü ızgara olur, birinci ona örneklenir) kayma tersine döner, aynı sınır içinde;
  - dönük görüntünün kayması kendi bilinen kaymasıdır.
- **MCP** (`crates/native/mcp/tests/orthophoto_seam.rs`): üç yeni durumun yerleri, sayıları ve `resampled`'ı; eski beş
  durum aynen.
- **Masaüstü** (`ortho_compare_window_tests::yeniden_ornekleme_gives_the_shared_cases_places`):
  - kutu kapalıyken ileti `resample`'ı söyler;
  - açılınca her iki görüntüde cevap durumunkidir, not örneklemeyi söyler.

  Pencere testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/yeniden-ornekleme-3660/`.

## Sonra

- Görüntüden türeyen nesnenin kaynağa izi (MAP-01 kararı); mozaiğin dikiş çokgenlerinden hatlar.
- Dönük birinci görüntü (ızgaranın kendisi dönük: yerler onun pikselleriyle).
- Web (TODOS §0.2).
