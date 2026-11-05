# 0954 — Taşıma planının onayı ve eskimesi

Tarih: 2026-11-05. Durum: kabul (Y48). Kapsam ROAD-HAUL-08.

## Sorun

Taşıma planı (ADR 0924, 0925, 0944) her okunuşta yeniden kuruluyordu ve kaydı yoktu. Döküm alanının kapasitesi,
Brüknerin fiyatları ya da kübaj değiştiğinde, hakedişte ya da şantiyede dayanılan plan da sessizce değişiyordu.
Hangi planın konuşulduğu ve neyin değiştiği hiçbir yerde görünmüyordu. ROAD-HAUL-08: "Borrow/waste kapasite ve iş
takvimi değişince etkilenen taşıma planı stale olsun."

Araştırma: Trimble Business Center–HCE'nin koridor kütle taşıması engelleri, ödünç ve döküm alanlarını taşır ve
"what if" senaryoları kurar. Topcon MAGNET zamanlı kütle taşıma planı yapar. Montana DOT'un Civil 3D kütle taşıma
yordamı da incelendi. Bu kaynakların hiçbirinde onaylı bir planı saklayıp sonrakini ona göre okuyan bir kayıt
bulunamadı. Bu yüzden kübajın onayı (ADR 0906 §5, 0950) örnek alındı.

## Karar

### Onay

Brüknerin `approved` alanı (`HaulApprovalDef`, KCAD şema 172) planın o günkü hâlini tutar:

- **Kimlik:** `number` (1'den), `at` (gün), `note`.
- **Dayandığı sürümler:** Brüknerin `revision`'ı, kübajın `volumeRevision`'ı, kübaj onayının `volumeApproval`
  numarası (eski numarasız onay 1 sayılır).
- **Amaç:** `cost`; yanlışsa en küçük moment.
- **Planın sayıları:** `moment`, `moved`, varsa maliyetin `total`'ı.
- **Alanlar:** her alanın adı, kapasitesi ve kullanımı.

Biten onay `history`'ye geçer, her biri `ended` ile:

- Yeni bir onay eskisinin yerine geçtiğinde: o günle.
- Onay kaldırıldığında: `revoked`, gün ve neden.

Kurallar `RoadSettings::problem`'dedir:

- Günler YYYY-AA-GG biçimindedir; notlar en çok 500 karakter olur.
- Sayılar sonludur; en çok 1000 alan ve 200 geçmiş onay olur.
- Geçerli onayın bitişi olmaz, geçmiştekilerin olur.
- Numaralar artarak gider.

### Komutlar

- **`road.hauls.approve`:** Planı verilen amaçla (yoksa moment) bugünkü hâliyle onaylar, ya da `revoke` ile onayı
  kaldırır. Tek geri alma adımı "Taşıma planı onayı".
  - Kurulamayan plan onaylanmaz (`invalid_haul`).
  - Onaysız planın onayı kaldırılmaz (`haul_not_approved`).
- **`road.hauls.edit`:** Onayı ya da geçmişi olan Brükneri silmez; onay ve geçmiş kalır. Düzenleme onayı korur:
  plan okunduğunda neyin değiştiği söylenir.
- **`road.hauls.plan`:** Onaylı Brüknerde planı onayın amacıyla yeniden kurar. Pencere başka amaçla okuyorsa da
  karşılaştırma onaylanan amaçla yapılır. Çıktıda `approval` vardır: onay, `stale`, `reasons`, `momentChange` ve
  değişen alanlar (onaylı ve bugünkü kapasite ve kullanım). Eskiyen planda `haul_plan_stale` uyarısı da verilir.

### Eskime gerekçeleri

Her biri sözle, şu sırayla:

1. Brüknerin sürümü değişti.
2. Kübajın sürümü değişti.
3. Kübajın onayı değişti (onaysız ↔ n. onay).
4. Bir alanın kapasitesi değişti, alan kalktı ya da eklendi.
5. Planın kendisi değişti: moment, taşınan hacim ya da tutar, değişenler ayrı ayrı. İki sayı büyüğünün (en az 1)
   10⁻⁶'sından fazla ayrıştığında değişmiş sayılır.
6. Plan bugün kurulamıyor: onaylanan amaçla, örneğin fiyatlar kaldırıldıysa.

Sürüm gerekçesi plan aynı kalsa da söylenir. Onaylanan şeyin dayanağı değişmiştir ve kararı kullanıcı verir.

### Pencere ve çıktı

- **Taşıma planı penceresi:**
  - Onay bölümünde not alanı, altta Onayla ve Onayı kaldır.
  - Eskimişse uyarı şeridi gerekçeleriyle, değilse bilgi şeridi.
  - "Onay" listesi: alan başına onaylı ve bugünkü kapasite ve kullanım, durumu (aynı, değişti, kalktı, eklendi) ve
    taşıma momenti.
- **Brükner penceresi:** Onayı ve geçmişi olduğu gibi taşır.
- **Planı yaz (ADR 0953):** Özet sayfasında onayın numarası ve günü, onaylı amaç, taşınan, moment ve tutar, ve
  "güncel" ya da "eskidi: gerekçeler".

## Ölçüm

- **Bağımsız referans:** `scripts/fixtures/road_haul_approve_cases.py` (22 durum) ve `road_haul_plan_cases.py`'nin 9
  onay durumu. Planı HiGHS (scipy linprog) kurar, KentOS kodu kullanılmaz. Rust ve Python durumları geçer.
- **Moment ile ölçülen değişimler:**

  | Durum | Moment değişimi |
  |---|---|
  | D1 döküm alanı 10 000 → 14 000 m³ | −117 390,9 m³·m |
  | D2 kalktı, D3 eklendi | −270 506,8 m³·m |
  | Kübajın yarma katsayısı 1,3 | +270 943,5 m³·m, taşınan +2034,4 m³ |

- **Yalnız tutarı değişen plan:** Fazla taşıma fiyatı 800 → 5000 TL olunca moment aynı kaldı. İlk gerekçe metni
  "moment +0.0" diyordu; parçalı gerekçe "tutar +732835 TL" diyor.
- **Yuvarlama:** Tutar tam birime, moment 0,1 m³·m'ye yuvarlanır. Rust ve HiGHS'ın toplamları 1e-3 içinde
  birleşir; metin bu yüzden iki tarafta aynı çıkar.
- **Pencere bekçileri:** `the_window_approves_and_takes_the_approval_off`, `a_stale_approval_says_why`.

## Açık

- **İş takvimi:** Modelde Brükner ya da alan için bir takvim yok; kamyon filosu (ADR 0936) çağrı başına hesaplanır.
  Takvim modele girdiğinde gerekçelere eklenir.
- **Geçmiş:** Geçmiş onaylar dosyada durur ama pencerede ayrı bir geçmiş görünümü yok (kübajda var, ADR 0950).
