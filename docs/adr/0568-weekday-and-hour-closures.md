# ADR 0568: Haftanın günü ve saatle kapanma

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-26'nın üçüncü dilimi; [ADR 0567](0567-vehicle-profiles-and-dated-closures.md)'nin
  “sonraya kalan” listesinin ilk maddesi.
- **Bağlam belgesi:** ADR 0521 (zaman profili ve saati, ilk giren ilk çıkar), 0567 (araç profilleri, tarihli kapanma, `Graph::vary`),
  0265 (`.kcad` şemasının altı adımı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - Yedi günden uzun yolculuk: saat haftayı yineler (sorunun gününden önceki gün ve sonraki beş gün; §3).
  - Saatli kapanmanın uzunluk ve alan maliyetinde yol boyunca izlenmesi: bu maliyetlerin saati yoktur, kapanma sorunun anında okunur (§2).
  - Dış yönlendirici, bağlı katmanın revizyonu, profilin dönüş kuralları (ADR 0567'nin listesi).

## Bağlam

ADR 0567'nin kapanması gün gündür: kenar ilk ve son gün arasındaki her gün bütünüyle kapalıdır. Gerçek kapanmaların çoğu ise yinelenir:
okul önündeki sokak hafta içi sabahları, pazar yeri cumartesileri, köprü her gece 22:00'den 06:00'ya kapanır. Saat yalnızca ADR 0521'in zaman
profiliyle yazılabiliyordu: bir dönemde kenarın hızını sıfır yapan alan. Ancak bu ayar her gün aynıdır ve her kenar için ayrı alan ister.

Ağın bir saati vardır: hız maliyetinin zaman profili kenarın hızını günün saatine göre verir. Kenar kapalıyken hız sıfırdır; araç bekler ya da
başka yoldan gider (ilk giren ilk çıkar, ADR 0521 §4, §5). Saatli kapanma aynı saatin bir haftalık hâlidir. Arama yöntemi değişmez.

## Karar

### 1. Kapanmanın günleri ve saatleri

Ağın `closures`'ında iki isteğe bağlı alan daha vardır: `days` ve `hours` (`kentos_contracts::NetworkClosures`, `.kcad` şema 96).
İlk ve son gün alanlarıyla birlikte dört alan birbirinden ayrı olmalıdır.

- **Günler:** kenarın değeri günlerden ya da günlerin aralığından oluşur, aralarında virgül ya da noktalı virgül vardır (`Pzt-Cum`,
  `Cmt, Paz`, `1-5; 7`). Bir gün şöyle yazılır: tam Türkçe adı (`Çarşamba`; işaretsiz yazım `carsamba` de okunur), kısa adı (`Pzt`, `Sal`,
  `Çar`, `Per`, `Cum`, `Cmt`, `Paz`) ya da 1 (pazartesi) ile 7 (pazar) arası sayısı. Büyük küçük harf Türkçe kuralla ayrılmaz. `Cum-Pzt`
  hafta sonunu dolanır. Boş değer her gün demektir.
- **Saatler:** kenarın değeri `H:MM-H:MM` biçiminde pencerelerden oluşur, aralarında virgül ya da noktalı virgül vardır
  (`07:00-09:00, 17:00-19:00`). Bir pencere `[başlangıç, bitiş)` aralığıdır. Bitişi başlangıcından önce olan pencere ertesi güne uzanır
  (`22:00-06:00`) ve başladığı günün kapanmasıdır. `24:00` günün sonudur. Başı sonuyla aynı olan pencere okunmaz. Boş değer bütün gün
  demektir.
- **Kapanmanın günü:** kapanma bir `d` gününde, `d` ilk ve son gün arasındaysa (boş sınır açık uçlu) ve haftanın günü günler alanında
  yazılıysa uygulanır. Dört değeri de boş olan kenar kapanmaz.

Okunamayan gün ya da saat ADR 0567 §3 gibidir: kenarı kapatmaz ve söylenir.

### 2. Saat yalnız saatle okunur

- **Saatsiz kapanma:** ADR 0567'deki gibidir. Kenar sorunun gününde bütün gün kapalıdır; artık haftanın gününe de bakılır.
- **Saatli kapanma, kalkış saatiyle sorulan soruda:**
  - Hız maliyetinin (dakikalar) saatinde kenarın hızı pencerelerde sıfırdır. Araç bekler ya da dolaşır.
  - Uzunluk ve alan maliyetlerinin saati yoktur. Bu maliyetlerde sorunun anında pencerenin içinde kalan kenardan geçilmez.
- **Saatli kapanma, saatsiz soruda:** uygulanmaz. Kaç kenarda böyle olduğu söylenir (`Varied.hours_unasked`).

### 3. Hafta

Saatli kapanması olan bir soruda, hız maliyetinin saati sorunun günü için bir haftalık saate çevrilir (`Clock::week`). Haftanın düzeni şöyledir:

- 0. dakika sorunun gününün gece yarısıdır.
- Sonraki beş gün birbirini izler.
- Altıncı gün sorunun bir önceki günüdür; geriye doğru arama 0'ın öncesine bu güne iner (varış saatine göre soru).

Her kenarın günlük adımları (zaman profili ve en yüksek hızla sınırlanmış hızı) yedi kez yinelenir ve pencereleri sıfır hızla üstüne
yazılır. Saat bu yedi günlük süreyi yineler. Graf yeniden kurulmaz. `Graph::vary`, ADR 0567 §4'teki gibi maliyetlerle birlikte saati de değiştirir.

`Access`'in `time`'ı yalnız ağın kapanmasının saat alanı varsa ve soruda bir gün varsa doldurulur. Böylece saatli kapanması olmayan bir
ağda saati değiştirmek grafı yeniden değiştirmez. Saat soruların anahtarıdır:

- `NetworkSession::set_time` saati koyar ve erişimi yeniden uygular.
- `set_access` o anki saati okur. Saatin ve günün hangi sırayla verildiği sonucu değiştirmez.

### 4. Arayüz ve otomasyon

- **Ağlar penceresi:** Tarihli kapanma'nın altında Gün ve saat satırı: günlerin ve saatlerin alanı. Form kuralları
  `kentos_interaction::network::form`'dadır. İlk ve son gün alanı olmadan gün ya da saat yazılırsa form bunu söyler.
- **Araçlar:** Saat (Z) seçeneği artık saatli kapanması olan ağda da sunulur (`NetworkDef::timed`). Tarih (T) ve Profil (P) değişmez.
- **İşlemler:** Tarih ve Kalkış saati'yle saatli kapanan kenarlar sayıyla söylenir. Saat verilmeden uygulanmayan kapanmalar uyarıdır.
- **Python:**
  - `Network.time` saati koyar ve `Network.closed`'u günceller.
  - `closed` sayıları: `byHours` ve `hoursUnasked`.
  - `set_time` yerel modülde erişimin JSON'unu verir.
- **`.kcad` şema 96:**
  - Kapanmanın `days`'i ve `hours`'u (docs/specs/kcad-v2.md §6.4.8).
  - Bağımsız yazıcı ve okuyucu.
  - Şema 95'te bu iki alan bilinmeyen alandır.

## Doğrulama

- **`scripts/fixtures/network_timed_closures_cases.py --check`:**
  - Sahne: 4 × 3'lük yol ızgarası. Hız maliyeti sabah 07:00–09:00 yarıya iner (ADR 0521'in profili).
  - Kenarlar: hafta içi sabah kapanan, her gece kapanan, hafta sonu bütün gün kapanan, üç gün boyunca iki saat kapanan, çarşamba gece
    yarısından sonra kapanan; okunamayan bir gün ve bir saat değeri.
  - 18 soru: gün ve saatle, saatsiz, günsüz; uzunlukla ve süreyle; gece yarısını ve önceki günün gece penceresini geçen yolculuklar,
    pencereye saniyeler kala giren yolculuk.
  - Başvuru KentOS kodu olmadan yazılmış, en erken varışı arayan bir Dijkstra'dır. Kenarın dakikalarını, girildiği andan başlayarak hızın
    değişimleri ve pencereler boyunca yürüyerek bulur. Pencereler gerçek günler üzerindedir; haftalık yineleme yoktur.
  - Her sorunun en ucuz yolu tektir.
- **Çekirdek** (`tests/all/network_access.rs`): iki tur oynar. Saatin haftalık hâli `clock.rs`'in testinde, gün ve saat okuyucuları
  `access.rs`'in testindedir.
- **Python** (`test_network_timed_closures.py`): dakikası tam olan 15 soruyu oynar; tarihi saatten önce ve sonra verir.
- **`kcad_v2_reference.py --check`:** `network-timed-closures.kcad` ve 5 bozuk örnek.
- **Ağ kuralları:** 7 yeni durum (`network_rules_cases.py`).
- **Ağlar formu:** 3 yeni durum (`network_form_cases.py`).
