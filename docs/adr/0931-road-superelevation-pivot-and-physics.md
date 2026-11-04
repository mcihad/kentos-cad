# 0931 — KentOS Yol: deverin dönme ekseni, göreli eğim ve yanal sürtünme

Tarih: 2026-11-04. plan/road.md Y27 (ROAD-DEVER-01, 07, 11). Durum: Y27.1 kabul; Y27.2–Y27.3 bu belgeyle ilerler.

## Dayanak

- **AASHTO ve eyalet kılavuzları** (NDDOT ders notları ve MDT Ek G; 4 Kasım 2026'da okundu, slaytların resimleri
  incelendi): deverin dört kazanılma yöntemi vardır.
  1. Eksen etrafında: eksen profili sabittir, kenarlar E/2 iner ve çıkar; varsayılan budur.
  2. İç kenar etrafında: iç kenar sabittir, eksen ve dış kenar yükselir.
  3. Dış kenar etrafında: dış kenar sabittir, eksen ve iç kenar alçalır.
  4. Düz eğimle dış kenar etrafında (bölünmüş yolda).

  Kenar etrafında döndürme hendek ya da orta refüj sığ kalırsa seçilir.
  - **Göreli eğim:** kaplama kenarı ile dönme ekseni arasındaki boyuna eğim farkıdır, Δ = (w·n₁·e)/L. En kısa geçiş
    boyu L = (w·N₁·e_d / G)·b_w; G en büyük göreli eğimdir (AASHTO tablosu 25 km/s'te 1:128'den 130 km/s'te 1:286'ya
    iner).
  - Nokta kütle dengesi V²/(gR) = (e + f)/(1 − e·f) yazılır; yaklaşığı e + f ≈ V²/(127R)'dir.
- **Civil 3D:** dönme ekseni (axis of rotation) yalnız LaneSuperelevationAOR şeridiyle tam çalışır. Her yan ayrı grup
  kurulur; ötelemeli montajda hesap yapılmaz
  ([About Assemblies That Support Superelevation Axis of Rotation](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-7787EDE9-5A9A-4EB1-BC92-3FA95AB9D77E.htm)).
- **OpenRoads Designer:** şeritte pivot ve dönme noktası vardır; pivot sabit kalır, dönme noktası dever oranına kadar
  düşey kayar. Calculate Superelevation'ın pivot yöntemleri Crown, Inside Edge, Outside Edge, Left Edge, Right Edge,
  Divided Inside ve Centerline'dır. Centerline dışındaki yöntemlerde dever oranı “yarıçap ± pivotun ötelemesi”nden
  hesaplanır ([Calculate Superelevation](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/406410/GUID-4501852A-6031-438C-B0B9-C888735D9806.html)).
  FHWA'nın ORD dever bölümü bu oturumda 403 verdi.
- **Netcad NETPRO:** dever çizelgesi ve diyagramı vardır. Dönme ekseni seçeneklerinin çevrimiçi belgesi bulunamadı.
- **İstatistik:** sürtünme katsayısı tabloları sürücü konforu ölçümlerinden gelir (AASHTO). KentOS tablo vermez; sınır
  standart profilinindir (ROAD-DEC-01). Bu madde istatistiksel bir hesap gerektirmez.

## Karar

### §1 Çekirdek (`kentos_road::superelevation_physics`)

- **`Pivot`:** `Centerline` (öntanımlı, önceki davranış), `LeftEdge`, `RightEdge`, `InsideEdge`, `OutsideEdge`.
  - İç kenar, eğimi düşük olan yandır. Eşit eğimde (normal bombe) kalkış iki yanda da sıfırdır, sol alınır.
- **Eksenin kalkışı:** tutulan kenar normal bombedeki kotunu korur. Kalkış = W·(n − s)/100'dür; W o yanın şerit
  genişliği (genişleme dahil), n normal eğimi, s şimdiki eğimidir. Kesitin ekseni profil kotu + kalkıştır.
- **Göreli eğim:** kenarın dönme eksenine göre kotunun km'ye göre türevidir.
  - Eksen etrafında W·s/100.
  - Kenar etrafında, öbür kenar için W_o·s_o − W_p·s_p; bütün genişlik döner.
  - Eğim ve genişlik km'de parça parça doğrusaldır, türev her aralıkta doğrusaldır. En büyüğü uçlardadır; kesin.
- **Yanal sürtünme ihtiyacı:** f = (V²/(gR) − e)/(1 + e·V²/(gR)), g = 9,80665.
  - e, merkeze doğru eğimi en az olan yanınkidir. Dış yan ters eğimliyse e eksidir.
  - Yaklaşık V²/(127R) − e, e·f çarpımını atar. Örnek: 50 km/s, R 80 m, %8 → kesin 0,1627, yaklaşık 0,1661.

### §2 Kayıt ve komut (Y27.2)

- Dever çizelgesinin `pivot`'u `.kcad` şema 165'tedir. ADR 0930'un dersi: yayımlanmış ya da sahibin elindeki şemaya alan
  eklenmez.
- Kesit takımları ve koridorlar kesiti ekseni kalkmış kotla kurar.
- `road.superelevations.check` yalnız okur. Girdi: çizelge, tip kesit (şerit genişlikleri için), `speed` (km/s),
  `mostFriction`, `mostRelative`. Cevap:
  - göreli eğim satırları;
  - her kurbun en büyük sürtünme ihtiyacı;
  - sınırı aşan bulgular.

### §3 Masaüstü (Y27.3)

Dever penceresinde Dönme ekseni seçimi ve Denetim bölümü bulunur (hız, sürtünme ve göreli eğim sınırı). Pencere ortak
düzene taşınır (DESIGN.md §7.19).

## Doğrulama

23 ortak durum (`scripts/fixtures/road_superelevation_physics_cases.py`):

- Kalkış kesirlerle.
- Göreli eğim her aralığın uçları ve ortasından geçen parabolün uçlardaki eğimiyle. Rust çarpım kuralıyla hesaplar,
  referans parabol uydurur; iki yol farklıdır.
- Sürtünme mpmath'le 40 basamakta.

Rust testi `tests/all/superelevation_physics.rs`'tir.

## Açıklar

- Divided Inside (bölünmüş yolun iç kenarları), düz eğimli tek yöne dever.
- Dever oranının pivotun ötelemesiyle düzeltilen yarıçaptan hesaplanması (OpenRoads), standart profilinden dever ve
  sürtünme önerisi (ROAD-DEC-01).
