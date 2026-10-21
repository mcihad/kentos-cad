# ADR 0570: Ulaşılamayanın nedeni

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-26'nın dördüncü dilimi: “erişilemeyen durak/çiftin nedeni source ID ile
  verilsin; OD matrisinde erişilemeyen çiftler sıfır maliyet sayılmamalı”.
- **Bağlam belgesi:** ADR 0209 (ağ analizi, rota, maliyet matrisi, en yakın tesis), 0521 (dönüşler, saat), 0557 (terminaller),
  0567 (profiller, tarihli kapanma), 0568 (gün ve saatle kapanma).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - En küçük kesim: bugün tek bir yol üzerindeki kapatanlar söylenir; iki ayrı yolu birden kapatanların en küçük kümesi değil.
  - “Başka” durumunun ayrıntısı: dönüş yasağının, kapalı düğümün ya da cihazın hangisi olduğu söylenmez.
  - Hizmet alanında ulaşılamayan kenarların nedeni.

## Bağlam

İki durak arasında yol yoksa araçlar şunu diyordu: “1. ve 2. duraklar arasında yol yok (kapalı kenar, tek yön ya da kopuk ağ)”. Bu söz
nedeni sayar ama nesneyi göstermez. ADR 0567 ve 0568'den sonra kapanmanın nedenleri çoğaldı: araç profili, gün, saat. Kullanıcının hangi
kenarın yolu kestiğini görmesi gerekir. Maliyet matrisi erişilemeyen çiftleri tabloya yazmıyordu (sıfır sayılmıyordu), ama bunu da
söylemiyordu.

## Karar

### 1. Gevşek yol

Yolu olmayan iki yer için çekirdek (`ops::network::blocked`) şu yolu bulur: her parçanın iki yöne de geçildiği, hiçbir kenarın ve düğümün
kapalı sayılmadığı ağda uzunlukça en kısa yol (gevşek Dijkstra). Bu yol yoksa iki yer ağın birbirine
bağlanmayan parçalarındadır (`Apart`).

### 2. Kapatanlar

Gevşek yolun her parçası gidildiği yönde sorunun maliyetiyle geçilemiyorsa kenarı kapatandır. Her kenar bir kez, yolun sırasıyla söylenir,
nesnesinin kimliği ve nedeniyle. Neden şu sırayla seçilir:

1. kapalı kenarlar ifadesi;
2. sorunun araç profili, günü ya da saati (ADR 0567, 0568; saat yalnız saati olmayan maliyette ve sorunun anında);
3. yön alanının kapalı değeri;
4. tek yön, ters yönde;
5. maliyetin değeri yok (alandan maliyette boş ya da okunamayan değer).

Yolun üstündeki engeller de sırasıyla kapatandır. Kapatanların kenarları açıksa yolu bir dönüş yasağı, kapalı düğüm ya da cihazın
terminal yolları kesiyordur (`Other`).

Bu, yolu kesen en küçük kesim değildir. Her kenar açık olsaydı alınacak en kısa yolu neyin kestiğini gösterir; tek tek kaldırılan
kapatanlar yolu açar.

### 3. Söz ve JSON

- **Rota reddi:** `Refusal::Unreachable` nedeni taşır. Sözü örneğin şöyledir: “1. ve 2. duraklar arasında yol yok: en kısa yolu
  kapatanlar: kenar 6 (o gün kapalı), kenar 5 (araç profiline kapalı), kenar 4 (tek yön, ters yönde).” En çok beş kapatan söylenir;
  kalanlar sayıyla eklenir.
- **JSON:** `{"error":"unreachable","between":[a,b],"blocked":{"kind":"by","edges":[{"id":6,"why":"date"},…],"barriers":[…]}}`.
  Kopuk ağda `kind` `apart`, öbür kesintide `other`'dır.
- **Başka sorular için:** `NetworkSession::blocked_between(a, b, reach, cost)` aynı nedeni verir.

### 4. İşlemler, araçlar ve Python

- **Maliyet matrisi:** sayı ve üst sınır yoksa her çiftin satırı vardır. Satırı olmayan çiftlerin sayısı uyarıyla söylenir: “tabloda
  yoklar (sıfır sayılmadı)”. İlk üçünün nedeni de söylenir.
- **En yakın tesis:** üst sınırsız soruda ulaşamayan olaylar için kuş uçuşu en yakın tesisle olan yolun nedeni söylenir.
- **En kısa yol aracı:** ret sözü komut satırında ve istemde nedeniyle gelir.
- **Python:** `route`'un `NetworkError`'ı nedeni taşır.

## Doğrulama

- **Bağımsız başvuru: `scripts/fixtures/network_blocked_cases.py --check`**
  - **Sahne:** birbirinden ayrı kenar zincirleri; her zincirde kenarlar kendi nedeniyle kapalıdır.
  - **Kapanma nedenleri:** ifade, yön alanı, tek yön, ağırlık sınırı, tarih, saat, boş ücret.
  - **Ek durumlar:** biri engelli bir zincir; kısa kenarı kapalı, uzun dolanması ters yönde tek yön olan bir kare.
  - **14 soru:** 9'unda yol yok, nedeniyle; 5'inde yol var (açık, tarih dışında, saat dışında, sürede bekleyen, kısa yolun tersi).
  - **Yöntem:** başvuru KentOS kodu olmadan yazılmış düz Dijkstra'dır. Önce açık kenarlarla yolu arar; yol yoksa her yolun gevşek en
    kısasını bulur ve tek olduğunu sınar.
- **Çekirdek:** `tests/all/network_access.rs` her sorunun JSON'unu başvurununkiyle karşılaştırır.
- **Python:** `test_network_blocked.py` sözleri karşılaştırır.
- **`network_processing_cases.py --check`:** sahnesindeki en yakın tesis soruları üst sınırsız durumda ulaşılamayan olay vermez. Başvuru
  bunu sınar.
- **Sınanmayan:** “Başka” durumu (dönüş yasağı, kapalı düğüm) ortak durumlarda yoktur.
