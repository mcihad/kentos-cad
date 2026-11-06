# 1480 — Ortofoto doğruluk denetimi: bağımsız denetim noktaları ve BÖHHBÜY sınırları (`cad.orthophoto.check`)

Tarih: 2026-11-06. plan/map-cad.md MAP-12 (alt maddeler: denetim noktası doğruluğu ve işleme raporu kaynakla; uyum ve
bağımsız denetim noktaları ayrı, RMSE ve yönlü hata; bir piksel okuması milimetre ölçü gibi sunulmaz; yanlış CRS).
Durum: taslak. İlgili: [ADR 0518](0518-georeference-record.md) (Raster oturt).

## Dayanak

- **BÖHHBÜY:** Büyük Ölçekli Harita ve Harita Bilgileri Üretim Yönetmeliği, Bakanlar Kurulu Kararı 2018/11962, RG
  26.06.2018/30460 Mükerrer; metin harita.gov.tr'den 6 Kasım 2026'da okundu.
  - **Madde 54(1):** yer örnekleme aralığı (YÖA) en çok 1/5000'de 30 cm, 1/2000'de 20 cm, 1/1000'de 10 cm,
    1/500'de 5 cm. **54(2):** uçuştan doğan ±%10 değişim kabul edilir.
  - **Madde 61(2):** blok dengelemede denetleme noktalarının KOH'ları X,Y'de ±0,75·YÖA, Z'de ±1·YÖA'dan küçük; en
    büyük farklar X,Y'de 1,5·YÖA, Z'de 2·YÖA. Bu madde fotogrametrik nirengiyi denetler, bu dilim değil.
  - **Madde 67(5):** üretilen ortofotonun doğruluğunda Madde 91'in ölçütleri aranır. **67(6):** mozaikteki
    ortofotolar arasındaki kayıklık en çok ±2·YÖA.
  - **Madde 91(1):** detay noktalarının en az %5'i GNSS ile bulunur. **91(2):** KOH_X,Y (cm) ≤
    ±(−1,665·10⁻⁶·S² + 0,01745·S − 1,166), KOH_Z (cm) ≤ ±1,33·KOH_X,Y. Buna göre sınır 1/500'de 7,14 cm, 1/1000'de
    14,62 cm, 1/2000'de 27,07 cm, 1/5000'de 44,46 cm.
- **Pix4D kalite raporu:** GCP'lerin hatası modelin GCP'lere uyumunu gösterir. Bağımsız doğruluk denetim noktalarıyla
  (3D checkpoints: X, Y, Z'de hata, sigma, RMS) ölçülür. GCP RMS'i 2,5·GSD'yi geçerse uyarı verilir
  ([Quality report](https://support.pix4d.com/hc/en-us/articles/360040686852),
  [GCP/CP](https://support.pix4d.com/hc/en-us/articles/16173344040733)).
- **ASPRS Positional Accuracy Standards, Edition 2 (2023/2024):** tek ölçü RMSE'dir (%95 kaldırıldı). Tam uygunluk
  için en az 30 denetim noktası istenir; denetim noktalarının kendi doğruluğu ürüne katılır
  ([LIDAR Magazine](https://lidarmag.com/2023/10/06/the-asprs-positional-accuracy-standards-edition-2/)).
  Türkiye'de BÖHHBÜY geçerlidir; ASPRS yalnız karşılaştırma içindir.
- **KentOS bugün:** Raster oturt kontrol ve denetim noktalarını ayırır, oturtma artıklarını gösterir (ADR 0518).
  Üreticiden gelen bir ortofotoyu bağımsız arazi noktalarıyla ve Türk mevzuatının sınırıyla denetleyen bir iş yoktur.

## Karar

1. **Kural paketi veridir:** `catalog/v1/rules/ortofoto-bohhbuy-2018.json`. Paket şunları taşır:
   - kimlik, sürüm, kurum, kaynak ve yürürlük;
   - Madde 91'in katsayıları ve Z çarpanı;
   - Madde 54'ün YÖA tablosu ve ±%10.

   Komut başka bir paket de alabilir (`rules`). Hiçbir oran kodda sabit değildir.
2. **`cad.orthophoto.check` v1, salt okunur (CAD + CBS: raster bağlam sekmesi).**
   - **Eşleme:** girdi rasterdir; arazide ölçülen noktaların katmanı ile görüntüde okunan noktaların katmanı adlarına
     (`Ad`) göre eşlenir.
   - **Noktaların rolü:** rasterin oturtmasında (`georef`) kontrol noktası olarak kullanılmış ad, **uyum noktasıdır**
     ve doğruluğa sayılmaz; öbürleri **bağımsız denetim noktasıdır**.
   - **Hata ve istatistik:** her noktanın hatası görüntü − arazidir.
     - **KOH_X,Y:** √(Σ(dx² + dy²) / 2n), yani X ve Y'nin ortak karesel ortalaması. Yönetmelik bu formülü yazmaz;
       KentOS'un yorumudur, ADR'de ve cevapta söylenir.
     - **Öbür ölçüler:** X ve Y'nin RMSE'si, radyal RMSE, ortalama (yönlü) hata, en büyük hata, KOH_Z (iki yanda kot
       olan noktalarda).
   - **Sınırlar:** 91(2)'den ölçeğe göre; YÖA (pikselin alanının karekökü) 54'ten.
   - **Piksel uyarısı:** KOH_X,Y yarım pikselden küçükse `belowPixel` uyarısı verilir: sonuç görüntüyü okuma
     inceliğinin altındadır, iyimser olabilir.
   - **Sistem:** rasterin sistemi projeninkinden başkaysa `crs_mismatch` ile reddedilir.
   - **Koordinatlar:** görüntünün koordinatları hiç değişmez.
3. **Bilinmeyen sayı:** noktaların sayısı ve dağılımının yeterliği (Madde 91(1): en az %5) komutça bilinmez. Cevabın
   notlarında söylenir, elle gözden geçirilir.
4. **Pencere Ortofoto doğruluğu…** (`raster.accuracy`, ikon `orthoAccuracy`, seçenekleri sahibine önceden gönderildi):
   - **Yeri:** raster seçilince Raster bağlam sekmesinde, Raster oturt'un yanında (Harita ve CBS).
   - **Form:** katmanlar bir satırda, ad alanı ve ölçek bir satırda.
   - **Özet:** KOH sınırla ve madde numarasıyla, KOH_Z, YÖA, uyum noktaları ayrı, yarım piksel uyarısı.
   - **Tablo ve dışa aktarım:** noktaların cm cinsinden hataları kayan veri tablosunda; CSV kaydet.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/orthophoto_check_cases.py` 27 durum üretir. Kurallar elle uygulanır; noktalar
  sentetiktir. Durumlar şunları kapsar:
  - 1/1000 içinde, 1/500 dışında, 1/2000, 1/5000, YÖA'sız ölçek;
  - başka kural paketi;
  - kaba hatalı nokta, yarım piksel altı KOH, kaba ve %10 içindeki YÖA;
  - oturtmasız raster, sistemi 0 olan raster, başka ad alanı, eşi olmayan noktalar, eşleşmeyen katmanlar, yalnız uyum
    noktaları, kotsuz noktalar;
  - retler.

  Rust ve Python geçer.
- **Masaüstü:** `ortho_window_tests.rs` sınar:
  - seçimsiz reddi ve katmanların tahminini;
  - 8 denetim ve 2 uyum noktasını, 1/1000 geçer ve 1/500 kalır sonucunu;
  - CSV'yi, salt okunurluğu, Harita ve CBS'yi.

  Resimler `.run/gui-evidence/ortofoto-dogruluk-1480/` (raster sekmesindeki yeri dahil).

## Sonra

- Zamanları farklı iki ortofotonun aynı denetim noktalarıyla yan yana karşılaştırılması; mozaik kayıklığı
  (Madde 67(6)).
- Görüntüde noktaları otomatik eşleme ve aday denetim noktaları.
- Fotogrametri aracının kalite raporunu (Pix4D, Metashape) kaynak kaydına bağlama.
- Görüntüden türeyen nesnenin kaynağa izlenmesi; DSM ve DTM denetimi; erişilemeyen ya da çevrimdışı kaynak.
