# 1700 — Uygulama kontrolü: ölçülen noktalar tasarım noktası, yüzeyi ve çizgisine karşı (`cad.asbuilt.check`)

Tarih: 2026-11-07. plan/map-cad.md MAP-19, ilk dilim. Alt maddeler:
- “Gerçekleşen ölçüm geldiğinde tasarım kimliğiyle veya önerilen mekânsal eşlemeyle karşılaştır. Yatay/düşey/normal
  fark, kapsam, ölçüm belirsizliği ve seçilmiş kabul profilini raporla; en yakın nokta her zaman doğru varlık değildir”;
- kabul satırı: “eksik varlık, yanlış eşleme, kot farkı ve düşük kaliteli çözümü ayır. Eski tasarım revizyonuyla ölçülen
  kayıt daha sonra yeni tasarıma sessiz bağlanmasın.”

Durum: taslak. İlgili ADR'ler: [ADR 1520](1520-map-project-terrain-surfaces.md) (Harita'da yüzey) ve
[ADR 1340](1340-gnss-campaign-archive.md) (GNSS arşivi).

## Dayanak

- **Trimble Business Center:**
  - **Point Conformance Report:** ölçülen noktayı tasarım noktasıyla kimliğine ya da yakınlığına (arama yarıçapı) göre
    eşler; yatay farkı ve isteğe göre düşey farkı verir.
  - **String Conformance Report:** noktaları bir çizgiye göre yatay ve düşey toleransla denetler.
  - **As-staked points:** proje ayarındaki yatay ve düşey toleransı aşanları işaretler.
  - Bir kullanıcı sorusu, ölçülen noktanın tasarım çizgisine göre karşılaştırılmasının hazır bir rapor olmadığını
    gösteriyor.

  Kaynaklar:
  - [Run a Point Conformance Report](https://help.fieldsystems.trimble.com/tbc/29971.htm);
  - [Run a String Conformance Report](https://help.fieldsystems.trimble.com/tbc/30020.htm);
  - [Run a Point Comparison Report](https://help.fieldsystems.trimble.com/tbc/21022.htm);
  - [As-staked point details](https://help.fieldsystems.trimble.com/trimble-access/latest/en/stakeout-as-staked-point-details.htm).
- **Türk kaynakları:** aplikasyon kontrolü için yatay ve düşey sayısal sınır bulunamadı (arama 7 Kasım 2026).
  **Karar:** sınırlar kullanıcınındır (idarenin şartnamesi, sözleşmenin kabul profili). KentOS değer koymaz; verilmeyen
  sınır denetlenmez.
- **KentOS bugün:** aplikasyon cetveli ve alete gönderme vardı (Yol). Sahadan dönen ölçümü tasarımla karşılaştıran
  bir şey yoktu.

## Karar

1. **`cad.asbuilt.check` v1, salt okunur. Girdi:**
   - `measured`: ölçülen noktaların katmanı;
   - `design` (tasarım noktaları), `surface` (Harita/Yol yüzeyi), `lines` (tasarım çizgileri): en az biri;
   - `field`: ad özniteliği (yoksa `Ad`);
   - `search`: yerle eşleme yarıçapı;
   - `horizontal`, `vertical`: toleranslar;
   - `qualityField` ve `quality`: σ ve sınırı;
   - `expectedDesign`: saha kaydının yapıldığı tasarımın özeti.
2. **Eşleme:**
   - Adı aynı olan tek tasarım noktası. Aynı adda iki tasarım noktası varsa belirsizdir.
   - Ad tutmazsa ve yarıçap verilmişse: başka bir ölçülen noktanın adıyla almadığı tasarım noktalarından yarıçap
     içindeki tek nokta, **öneri** (`place`, işaret `proposed`). İki ya da daha çok aday belirsizdir ve hiçbirine
     eşlenmez (“en yakın nokta her zaman doğru varlık değildir”).
3. **Farklar:**
   - Ölçülen − tasarım: doğu, kuzey, kot, yatay uzaklık.
   - Yüzeye göre: ölçülen kot − yüzey.
   - Çizgiye göre: en yakın tasarım çizgisinin en yakın noktasına uzaklık, yönünün solu artı. Yaylı kenarda yaya
     uzaklıktır. Yalnız hiçbir tasarım noktasının almadığı ve (yarıçap verilmişse) bir çizgiye yarıçap içinde olan
     noktalar için hesaplanır: bordürün noktaları ölçülür, yanındaki aplikasyon noktaları değil.
4. **Durum:**
   - `ambiguous`.
   - `unmatched`: ne bir tasarım noktası ne bir çizgi alıyorsa. Yalnız yüzey verilmişse her nokta yüzeyle denetlenir.
   - Aksi hâlde `outside` (bir sınır aşıldı) ya da `within`.
   - İşaretler: `proposed`, `outsideHorizontal`, `outsideVertical`, `outsideSurface`, `outsideLine`, `lowQuality`
     (σ > sınır; durumu değiştirmez: zayıf çözüm yapım hatası değildir), `noHeight`, `offSurface`.
   - Ölçülmeyen tasarım noktaları `missing`'dedir. Eşlenenlerin yatay ve düşey KOH'u ile en büyükleri de verilir.
5. **Tasarım özeti:**
   - `designDigest`: tasarım noktalarının (ad, yer, kot; altı ondalık) ve çizgilerinin SHA-256'sı.
   - `expectedDesign` verilir ve tutmazsa `designChanged`. Karşılaştırma yine yapılır ama değişim söylenir: eski
     tasarımla ölçülen kayıt yeni tasarıma sessizce bağlanmaz.
   - Yüzeyin kendi özeti arazinindir (`road.terrains.query`).
6. **Retler:** `invalid_asbuilt_check` (yeni), `layer_not_found`, `not_a_road_project`, `unknown_terrain`,
   `terrain_unbuildable`.
7. **Masaüstü, Uygulama kontrolü penceresi** (Harita ve Yol: Ölçme › Saha; CBS: Ölçme › Noktalar menüsü):
   - Katmanlar adlarından tahmin edilir. Yüzeyin kaynak katmanları tahmine girmez.
   - Toleranslar ve kalite boştur.
   - Sonuç: özet, KOH, kalite uyarısı, ölçülmeyenler; Tümü | Dışındakiler süzgeci.
   - Tablo: Δdoğu, Δkuzey, Δkot, yüzey, çizgi, durum, işaretler (milimetreye, işaretli).
   - Dışındakileri seç ve CSV kaydet.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/asbuilt_cases.py` 26 durum üretir.
  - Bağımsız hesap:
    - tasarım yüzeyi düzlemdir: G = 100 + 0,02x + 0,01y;
    - eşleme, farklar, işaretli dik uzaklık, durum ve SHA-256 özeti elle hesaplanır;
    - KentOS kodu kullanılmaz.
  - Noktalar:
    - içinde; yatay dışında; düşey ve yüzeye göre dışında;
    - yarıçapla öneri; iki adayla belirsiz;
    - eşleşmeyen; kotsuz;
    - σ'sı yüksek ama içinde;
    - bordürden −3 cm (dışında) ve −1 cm (içinde).
  - Durumlar:
    - yalnız adla, yalnız yüzey, yalnız bordür (yarıçaplı ve yarıçapsız), toleranssız;
    - aynı ve değişmiş tasarım özeti;
    - CBS projesi, Yol projesi;
    - 13 ret.
  - Toleranslar durumların kendi değerleridir.

  Rust ve Python (.kcad gidiş-dönüşüyle) geçer.
- **Masaüstü** (`asbuilt_window_tests`):
  - katman tahmini;
  - sınırlar girilince ortak durumun sayıları;
  - sayı olmayan sınır;
  - Dışındakileri seç;
  - CSV;
  - Harita, Yol ve CBS sunar; Mimari sunmaz.

  Resimler `.run/gui-evidence/uygulama-kontrolu-1700/` altındadır.

## Sonra

- **Saha paketi:** seçilen revizyon, kimlikler, CRS/epok, kot referansı, kabul eşiği. Aplikasyonu alete gönder'in
  dosyasına tasarım özetinin yazılması.
- **İş türleri:** as-built, aplikasyon kontrolü ve dönemsel deformasyon ayrı olmalı; zaman serisi ve ortak kontrol
  korunmalı.
- **Kayıt:** sahadan gelen düzeltmenin yetkili incelemeyle kabul edilen gerçekleşen revizyon kaydı.
- **Kapsam:** çizgiler boyunca ölçülmeyen aralıklar.
- **Cihazlar:** canlı cihaz adaptörleri donanım kapısındadır.
