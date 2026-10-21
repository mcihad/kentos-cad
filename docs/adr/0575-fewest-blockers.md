# ADR 0575: Ulaşılamayanda en az kapatan

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-26'nın kalanından: “en küçük kesim”. ADR 0570'in §1'inin yerini alır.
- **Bağlam belgesi:** ADR 0570 (ulaşılamayanın nedeni), 0567 ve 0568 (profil, tarihli ve saatli kapanma).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - “Başka” durumunun ayrıntısı (dönüş yasağı, kapalı düğüm ya da cihaz) ve hizmet alanında ulaşılamayan kenarların nedeni, ADR 0570'teki
    gibi.
  - Yolu kesen iki kenar kümesinin ikisi de en az kapatanlıysa, uzunlukça kısa olan söylenir. Öbürü söylenmez.

## Bağlam

ADR 0570 yolu olmayan iki yer için şunu söylüyordu: her kenar açık olsaydı alınacak uzunlukça en kısa yol ve onu kapatanlar. Bu yol
kullanıcıya en az iş çıkaran yol olmayabilir. Düz yolda iki kapalı kenar, biraz dolanan yolda tek kapalı kenar varsa iki kenar söyleniyordu.
Oysa tek bir kapanmanın kalkması yolu açar.

## Karar

### 1. En az kapatanlı yol

Çekirdek (`ops::network::blocked`'ın `fewest`'i) iki yer arasında, her parçanın iki yöne de gidildiği ve hiçbir düğümün kapalı
sayılmadığı ağda şu yolu arar:

- önce en az kapatanı olan yol;
- bunlardan uzunlukça en kısası;
- eşitlikte numarası küçük yay.

Kapatanlar şöyle sayılır:

- **Kenar:** parçası gidildiği yönde sorunun maliyetiyle geçilemiyorsa bir kapatandır. Bir kenarın birbiri ardınca gidilen parçaları onu bir
  kez sayar; bunun için arama düğümler üzerinde değil, yaylar (parçanın gidildiği yön) üzerindedir.
- **Durak:** bir parçanın ucundaki durak o parçadan hiçbir şey gitmez; parçanın kapanması yolu kesmez.
- **Engel:** gidilen parçanın üstündeki her engel bir kapatandır.

Bulunan yolun kapatanları ADR 0570'teki gibi söylenir: yolun sırasıyla, kenar bir kez, nesnesinin kimliği ve nedeniyle. Nedenlerin
sırası değişmez. Yol yoksa durum `Apart`, kapatan yoksa `Other`'dır.

### 2. Söz

- **Ret sözü:** “1. ve 2. duraklar arasında yol yok: yolu açmak için en az kalkması gerekenler: kenar 19 (yön alanı kapalı).”
- **Başka durumu:** “en az kapatanı olan yolun kenarları açık; …”
- **JSON:** `blocked` biçimi değişmez.

## Doğrulama

- **Bağımsız başvuru: `scripts/fixtures/network_blocked_cases.py --check`.** KentOS kodu olmadan yazılmış, sözlük sırasıyla
  (kapatan sayısı, uzunluk) çalışan düz bir Dijkstra'dır ve en iyi yolun tek olduğunu sınar.
  - ADR 0570'in 14 sorusunun cevabı değişmedi.
  - **Yeni soru (`en-az-kapatan`):** düz yolda iki kenar ifadeyle kapalı, dolanan yolda tek kenarın yön alanı kapalı. Cevap dolanan yolun tek
    kenarıdır (19). Eski kural 16 ve 17'yi verirdi.
- **Çekirdek:** `tests/all/network_access.rs` 15 sorunun JSON'unu başvurununkiyle karşılaştırır.
- **Python:** `test_network_blocked.py` sözü karşılaştırır.
- **İşlemler:** `cargo test -p kentos-processing --test cases network`.
- **Sınanmayan:** bir kenarın birden çok parçasının kapanmayı bir kez sayması ortak durumlarda yok; başvurunun kenarları tek parçalıdır.
  Kural koddadır.
