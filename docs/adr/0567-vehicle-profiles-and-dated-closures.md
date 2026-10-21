# ADR 0567: Araç profilleri ve tarihli kapanmalar

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-26'nın ikinci dilimi; [ADR 0521](0521-turns-levels-and-time-profiles.md)'in
  “kalan” listesinden iki madde.
- **Bağlam belgesi:** ADR 0209 (ağ analizi, graf, aramalar), 0521 (dönüşler, seviyeler, zaman profili), 0562 (terminaller her
  yerde), 0199 (katman alanları), 0265 (`.kcad` şemasının altı adımı), 0253 (komut protokolü).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - Haftanın günlerine ya da saat aralığına bağlı kapanma (pazar günleri pazar yeri, gece kapanan köprü). Bu dilimin kapanması
    gün gündür; saat ADR 0521'in zaman profilinde kalır.
  - Dış yönlendirici (Valhalla, OSRM) bağdaştırıcısı: bölgesel ölçek ihtiyacı ölçülünce.
  - Bağlı katmanın (GeoPackage, PostGIS) revizyonu değişince grafın ve maliyetlerinin yenilenmesi: bugün masaüstü ağı çizim
    değişince yeniden kurar; bağlı kaynağın sürümü ayrıca izlenmiyor.
  - Profilin dönüş kuralları (kamyonun sola dönüş yasağı) ve aksı başına yük.

## Bağlam

ADR 0521'den sonra bir ağ her soruya aynı yanıtı veriyordu: 3,5 m'lik köprünün altından kamyon da geçiyor, yaya otoyola giriyor,
tek yönlü sokakta yaya da yönü izliyordu. Kapanma ya ağın “kapalı kenarlar” ifadesiyle (her soruda, her gün) ya da zaman
profilinin sıfır hızıyla (her gün aynı saatte) yazılabiliyordu; “3–5 Kasım arası kazı” yazılamıyordu.

Ağ çizimden bir kez kurulur ve birçok soruya yanıt verir (ADR 0209 §10). Profil ve tarih grafın parçalarını, düğümlerini, dönüşlerini
değiştirmez; yalnız hangi kenardan hangi yöne ve hangi maliyetle geçildiğini değiştirir. Bu yüzden her profil ve gün için grafı
yeniden kurmak (100 000 parçada yüzlerce milisaniye) gerekmez.

## Karar

### 1. Araç profili

Ağın tanımında `profiles` (`kentos_contracts::NetworkProfile`, en çok 8): her biri

- `name`: kırpılmış, en çok 40 karakter, ağda Türkçe büyük küçük harf ayrımı olmadan bir kez;
- `limits` (en çok 8): kenar alanı ve aracın değeri (`agirlik` 18, `yukseklik` 4). Kenarın alanı değerin altında bir sayıysa
  kenar araca kapalıdır; eşit değer geçer; boş alan sınır değildir;
- `forbidden` (en çok 8): kenar alanı ve 1–16 değer (`tur` = yaya, bisiklet). Kenarın değeri listedeyse (büyük küçük harf ayrımı
  olmadan) kapalıdır;
- `topSpeed` (km/sa, 0 < v ≤ 300): her hızla maliyetin hızı en çok budur; parçanın dakikası kendisininkinin ve uzunluğunun bu
  hızla dakikasının büyüğüdür; zaman profilinin her basamağı da bu hızla sınırlanır;
- `bothWays`: tek yönlü kenarda iki yöne aynı maliyetle gider (yaya). Yön alanının kapalı değeri ya da kapalı kenarlar ifadesiyle
  kapalı kenar kapalı kalır.

### 2. Tarihli kapanma

Ağın tanımında `closures`: kenarın kapanmasının ilk ve son gününü tutan iki ayrı alan (`from`, `to`). Değer `YYYY-MM-DD` (ardından
saat gelebilir: `2026-11-03T08:30` o gündür). Bir gün `d` için kenar `a ≤ d ≤ b` ise kapalıdır; boş ilk gün her zamandan, boş son gün
süresiz demektir; ikisi de boşsa kenar kapanmaz. Soruda gün yoksa kapanma uygulanmaz.

### 3. Okunamayan değer

Sayı olmayan sınır ya da tarih olmayan gün kenarı kapatmaz ve söylenir (`Varied.unread`: kenarın nesnesi). Okuma ilk kapatan
nedende durur (kapatılmış kenarın sonraki değerleri okunmaz); bu bağımsız başvuruda da böyledir. Profil kenarı kapattıysa tarih
okunmaz.

### 4. Graf bir kez kurulur, soru maliyetleri değiştirir

`Graph::vary(Access { profile, day })`: ilk değişiklikte tanımlı maliyetler ve zaman profilleri saklanır, sonra her parça için
kenarının kapanması, iki yön ve en yüksek hız saklanandan yeniden hesaplanır; aynı erişim yeniden sorulursa iş yapılmaz, erişimsiz
soru tanımlı ağa döner. Parçalar, düğümler, dönüş tablosu, terminaller ve Denetle değişmez. Kenarın profil ve kapanma alanları
grafın kurulmasında bir kez alınır (`EdgeIn.facts`, kuralların `access.facts` sırasıyla); kenar başına okumalar ancak tanımda profil ya
da kapanma varsa yapılır.

`NetworkSession::set_access(profil adı, gün)`: bilinmeyen profili (“Ağda “Bisikletli” araç profili yok; profiller: …”) ve okunamayan
günü (“Tarih “3.11.2026” okunamadı; 2026-11-03 gibi yazın.”) reddeder; erişim değişince imlece yol ağacını bırakır.

### 5. Arayüz ve otomasyon

- **Ağlar penceresi:** Araç profilleri (ad; sınırlar `agirlik 18; yukseklik 4`; yasaklar `tur = yaya, bisiklet`; en yüksek hız;
  Tek yönlü kenara uyar / İki yöne gider) ve Tarihli kapanma (iki alan). Form kuralları `kentos_interaction::network::form`,
  ortak durumlar `fixtures/network/v1/form.json`.
- **Araçlar:** En kısa yol ve Hizmet alanı'nda Profil (P; profili olan ağda) ve Tarih (T; kapanması olan ağda; gün değer alanına
  yazılır, T kapatır ve son günü yeniden açar). Soru ağın iş parçacığına profil ve günle gider.
- **İşlemler:** Hizmet alanları, En yakın tesis ve Maliyet matrisi'nde Araç profili ve Tarih; kapanan kenarlar sayılarıyla söylenir.
- **Python:** `Network.profile`, `Network.date`, `Network.closed`.
- **`.kcad` şema 95:** ağın `profiles`'ı ve `closures`'ı (docs/specs/kcad-v2.md §6.4.8); bağımsız yazıcı ve okuyucu; şema 94'te bilinmeyen
  alan.

## Doğrulama

- `scripts/fixtures/network_access_cases.py --check`: 4 × 3'lük yol ızgarası (köşeleri metrelerle kaydırılmış, iki yolun uzunluğu
  eşit değil), dört profil ve kapanmalar; 26 soru (profil, gün, uzunluk ve süre) ve 3 ret. Başvuru KentOS kodu olmadan düz Dijkstra'dır;
  her sorunun en ucuz yolunun tek olduğunu sınar. Çekirdek `tests/all/network_access.rs` iki tur oynar (erişim değişip geri döner)
  ve erişimsiz sorunun tanımlı ağa döndüğünü sınar; Python `test_network_access.py` aynı dosyayı çizim, tanım ve `kentos.network`
  yoluyla oynar.
- `kcad_v2_reference.py --check`: `network-profiles.kcad` ve 15 bozuk örnek (iki şema 94 örneği dahil).
- Ağlar formunun 9 yeni durumu (`network_form_cases.py`).
