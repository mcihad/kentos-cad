# ADR 8052: Faz kapısı — 2+1 gerçek pencerede bugünkü araçlarla

- **Durum:** sürüyor (12 Kasım 2026); `plan/architecture-advanced.md` §7 "Kabul ölçüleri ve faz kapıları", ADV-26. M51
  ile iş sırası (M1…M51) bitti; §7'nin faz kapısı kabul izlerinin Mimari modülünde, şeritten ve kenetle oynanmasını ve
  sayaç raporunun başlangıçla (ADR 8000: 147 etkileşim, KLM 177,6 s; hedef ≤ 80 ve ≤ 100 s) karşılaştırılmasını ister.
- **Bağlam belgesi:** ADR 8000 (gerçek pencere düzeneği, 2+1 tabanı), ADR 8018 (seçenekler imlecin yanında), ADR 8021
  (Oda aracı), ADR 8003 (seçenek harfi sözleşmesi), ADR 8033 (konutun mahalleri, PAİY 29).

## Bağlam

2+1'in başlangıç senaryosu (`fixtures/interaction/real/daire-2p1.txt`, 7 Kasım) Dikdörtgen duvar, dört Ölçülü bölme,
Kapı, Pencere, beş Mahal (adı panelden yazılarak), Mutfak dizisi, Duvar ölçüleri, Döşeme, Hepsini etiketle, geçici
ölçüyle değişiklik ve iki çizelgeyle çizildi. O günden beri Oda aracı (M21: duvarlar, mahal ve etiketi tek adımda),
imleç yanı seçenekler (M18), iki harfli adlar (M3) ve yeni şerit (M15) geldi.

Yeni senaryoyu gerçek pencerede kurarken üç kusur çıktı:

- **Oda'nın "A" harfi çalışmıyordu.** Komut çubuğu "Mahal adı: Adsız … A" gösteriyor, ama komut satırına yazılan A
  "“A” seçeneği şu adımda kullanılamıyor" diyordu: aracın yazılan seçenekleri B, D, E, S, R, O ve K'ydı. Ardından
  yazılan ad komut satırında komut adı gibi okunuyordu. ADR 8003'ün sözleşmesi, gösterilen her harfin yazılınca
  çalışmasıdır.
- **Yeni projede mahal adı listesi boştu.** Oda'nın ve Mahal'in ad listesi projenin mahal adlarıdır (Proje ayarları ›
  Mimari); yeni Mimari projesinde liste yalnız "Adsız"dı, her odanın adı elle yazılıyordu.
- **Dört yanı duvarlı yerde Oda odayı yazmıyordu.** Daireyi oda oda çizerken sonuncusu (Banyo: iki yanı kabuk, öbür
  ikisi koridorun ve yatak odasının duvarı) için Dikdörtgen duvarın reddi ("bütün kenarları var olan duvarlarda;
  çizilecek yeni duvar kalmadı") mahali ve etiketi de götürüyordu.

Ayrıca çipten bir seçim yapıldıktan sonra klavye odağı komut satırında kalır: ↓ ve Tab (imleç kutusunun tuşları, ADR
8018) çizime değil komut satırına gider. Bu dilimde değiştirilmedi; senaryo değerleri komut satırına yazar.

## Karar

### 1. Oda'nın adı A ile yazılır

`WallRectangle` (Oda) yazılan "A"yı sorar ("mahalin adını yazın (Adsız: adsız)"), sonraki sözcükler mahalin adıdır:
projenin adlarından biriyse onun yazılışı, değilse yazıldığı gibi; "Adsız" adı kaldırır. Liste ve imleç kutusunun
yolundan (`choose_option("A", …)`) geçer. İlk ve ikinci köşede çalışır; G geri alır.

### 2. Yeni Mimari projesi mahal adlarıyla açılır

`kentos_contracts::NEW_PROJECT_SPACE_NAMES` (20 ad) yeni Mimari projesinin `arch.space_names`'idir: Planlı Alanlar İmar
Yönetmeliği 29/1'in konutta istediği piyesler (oturma odası, yatak odası, mutfak, banyo, tuvalet) ve 29/3'ün hol ve
koridoru, denetimin onları tanıdığı sözcüklerle (`room_names::kind_of`: Salon → Oturma odası, WC → Tuvalet, Koridor →
Hol), ardından bir konut planının öbür mahalleri (Antre, Giyinme odası, Kiler, Çamaşırlık, Depo, Balkon, Teras). Yalnız
yeni projeler: var olan çizimin listesine dokunulmaz (plan §8).

### 3. Duvarları duran yerde Oda yalnız mahali ve etiketi yazar

Oda, dikdörtgenin planı "çizilecek yeni duvar kalmadı" (`nothing_to_set`) derse duvar komutunu atlar; mahal ve etiketi
aynı "Oda" adımında yazar ("Oda eklendi: mahali (Banyo) ve etiketi; duvarları zaten vardı"). Önizleme bu durumda ret
göstermez. Başka bir ret (stil, kat, katman) eskisi gibi söylenir ve hiçbir şey yazılmaz.

## Doğrulama

- `cargo test -p kentos-desktop --lib building_context_tests::oda`: komut satırında A ve ad ilk köşede ve ikinci köşede;
  Adsız adı kaldırır; liste Kiler'i sunar, listede olmayan Arşiv de alınır; Dikdörtgen duvarın duvarlarına çizilen
  Oda duvar eklemeden 12 m²'lik mahali ve etiketini yazar, tek geri almayla ikisi gider.
- `cargo test -p kentos-project`: yeni Mimari projesinin 20 adı, ayarların denetiminden geçer; Salon, Yatak odası,
  Mutfak, Banyo, WC ve Koridor denetimin türleri olarak okunur.
