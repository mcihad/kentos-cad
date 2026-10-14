# ADR 0271: Kenet adayları arasında dolaşma, ipucu ve belirlenim

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-07`: “Kalabalık çizimde snap adayına cycling,
  tür/geometri ipucu, öncelik ve piksel aperture davranışını deterministik yap.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0029 (kenet deposu, ağırlıklar), 0163 (kenet ekleri), 0187 (seçimde Sıradakini seç), 0269 (kenet kapsamı),
  0023 (HiDPI).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Kalabalık bir köşede (parselin köşesi, ona yakın bir duvarın kesişimi, kenarın ortası) kenet tek bir aday seçer: ağırlıklı uzaklığı en
küçük olanı. İstenen başka bir adaysa kullanıcı imleci kaydırmak, büyütmek ya da kenet türlerini kapatmak zorundaydı. AutoCAD'de Tab
aynı yerdeki adaylar arasında dolaşır. İşaretin yazısı yalnız türü söylüyordu, hangi nesnenin noktası olduğunu söylemiyordu.

## Karar

### 1. Adayların sırası (çekirdek)

`Store::snap_candidates` imlecin açıklığındaki bütün adayları en iyiden sırayla verir; ilki her zaman `Store::snap_ex`'in seçtiğidir:

1. türlerin noktaları ağırlıklı uzaklıklarıyla (uzaklık × türün ağırlığı: uç nokta, nokta ve ekleme noktası 1; kesişim 1,02; merkez ve
   ağırlık merkezi 1,08; çeyrek 1,1; orta 1,15; teğet 1,2; dik 1,25; uzantı ve paralel 2), eşitlikte nesnelerin depoda karşılaşıldığı
   sırayla (çizimin sırası);
2. eğrinin üstünden merkez (tek seferlik Merkez, ADR 0269 §2);
3. en yakın nokta;
4. Karelaj düğümü.

Aynı nokta (bir köşe ki aynı zamanda kesişimdir; iki nesnenin ortak köşesi) bir kez sayılır, en iyi türüyle ve ilk nesnesiyle.
Aday toplarken kesişim çiftleri budanmaz (en iyiyi değiştiremeyen çiftler `snap_ex`'te atlanır; sonuç bit bit aynıdır).

### 2. Tab ile dolaşma (masaüstü)

- Nokta bekleyen bir komut (ya da sürüklenen tutamaç) çalışırken ve değer alanı kapalıyken **Tab** imlecin durduğu yerdeki sıradaki
  adaya, **Shift+Tab** öncekine geçer; sonda başa döner. Tek aday varsa Tab hiçbir şey yapmaz.
- Seçilen aday tıklamanın noktasıdır; imleç 0,75 mantıksal pikselden çok kımıldayınca ya da tıklama bırakılınca dolaşma biter ve en iyi
  aday yeniden gelir.
- Değer alanı açıkken Tab'ın anlamı değişmez (kilit, ADR 0166 §6).

### 3. İşaretin ipucu

Kenet işaretinin yazısı türü, nesnenin türünü ve dolaşırken sırayı söyler: “Uç nokta · kapalı alan”, “Kesişim · çizgi · 2/3 (Tab)”.
Nesnesi olmayan adayda (çizilmekte olan yol, uzantı, paralel, Karelaj) yalnız tür.

### 4. Açıklık ve belirlenim

- Kenet açıklığı mantıksal piksel olarak ayardır (`drafting.snapAperture`, varsayılan 11); dünyadaki boyu kameranın ölçeğiyle, HiDPI'dan
  bağımsızdır (aynı ekran görüntüsünde aynı aday). Kenetin ölçek aralığı ADR 0163 §5'tedir.
- Aynı çizim, aynı imleç ve aynı ayarlar her zaman aynı adayları aynı sırayla verir: rastgelelik ve karma tablo sırası yoktur; eşitlik
  çizimin sırasıyla çözülür.

## Sonuçlar

- Kalabalık köşede istenen aday imleç kımıldamadan Tab'la alınır; ipucu hangi nesnenin hangi noktası olduğunu söyler.
- Adayların sırası çekirdekte tektir ve bağımsız başvuruyla sınanır.

## Doğrulama

- `python3 scripts/fixtures/snap_coverage_cases.py --check` (`fixtures/snap/v1/coverage.json`'un `candidates`'i: kalabalık köşeler,
  ağırlıklar, aynı noktanın bir kez sayılması); `cargo test -p kentos-geometry-core --test all snap` (sıra ve her iki dosyanın her
  durumunda ilk adayın kenetle aynı olması).
- Etkileşim izi `fixtures/interaction/v1/snap-cycle.json` masaüstünde üç çeşitte (Tab, Shift+Tab, tıklama, kımıldayınca biten dolaşma).
- Resim: `KENTOS_SHOTS_ONLY=kenet-dolasma cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
