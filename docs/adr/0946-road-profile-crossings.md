# 0946 — Boykesitte kesişmeler: köprü altında açıklık, menfez üstünde örtü

Tarih: 2026-11-05. Durum: kabul (Y40). Kapsam ROAD-PROFILE-07 ve ROAD-PROFILE-14'ün açıklık kısıtı.

## Sorun

Yol, boykesitinde başka yapılarla kesişir:

- **Üstünden geçen yapı** (üst geçit, köprü, enerji hattı): kırmızı kot, yapının alt yüzünden en az bir açıklık kadar
  aşağıda kalmalıdır.
- **Altından geçen yapı** (menfez, boru): kırmızı kot, yapının üst yüzünden en az bir örtü kadar yukarıda olmalıdır.

Y25'in kırmızı kot önerisi (ADR 0926) bunları bilmiyordu. Araziye en yakın çizgi bir üst geçidin altında yeterli
açıklık bırakmayabiliyor, menfezin üstünde örtüsüz kalabiliyordu. Tasarımcı bunu ancak çizimde gözle görüyordu.

## Dayanak

- **Araştırma (5 Kasım 2026):**
  - Civil 3D boru ağlarındaki kesişen boruları boykesit görünüşünde gösterir ve boru ağları arasında girişim denetimi
    yapar; boykesitin kendisinde açıklık kısıtı bulunamadı.
  - OpenRoads için bir karşılık bulunamadı.
- **Eniyileme yazını:** düşey güzergâh eniyilemesini doğrusal ve karesel programlama üstüne kurar (Easa'nın doğrusal
  programı, Moreb'in karesel gösterimi, UBC'nin dışbükey modelleri).
- **Değerler projenindir.** Açıklık ve örtü standarttan ve yapının projesinden gelir; KentOS sayı vermez.
- **Doğrusallık:** bir istasyondaki tasarım kotu PVI kotlarında doğrusaldır (ADR 0926). Bu yüzden "z(s) ≤ alt yüz −
  açıklık" ve "z(s) ≥ üst yüz + örtü" doğrusal eşitsizliklerdir ve önerinin dışbükey karesel programına girer. Önerinin
  tek en iyi çözüm olması korunur.

## Karar

### Kayıt (KCAD şema 170)

`ProfileDef.crossings` alanı her kesişme için şunları tutar:

- ad, km;
- `side`: `above` (yapı yolun üstünde) ya da `below` (altında);
- `elevation`: yapının alt ya da üst yüzü;
- `clearance`: açıklık ya da örtü, 0–100 m.

Şema 169 ve öncesi bu alanı `unknown_field` ile reddeder. `road.profiles.edit` kesişmeleri yazar; boş liste kaldırır.

### Çekirdek

- `kentos_road::profile::crossings`: kesişmenin sınırı ve bir çizgiye karşı payı (pozitif: yetiyor).
- `propose::Rules.bounds`: istasyondaki kot için alt ve üst sınır. Etkin küme yönteminde sınırın satırı, tasarımın o
  istasyondaki katsayılarıdır.
- **Sınır tutuyor mu:** etkin kümedeyse tutuyordur. `Proposal.bound` bunu söyler.

### Komutlar

- **`road.profiles.propose`:**
  - kesişmeleri kısıt olarak alır;
  - her kesişmenin sınırını, önerilen kotu, payını ve tutup tutmadığını raporlar;
  - birlikte sağlanamayan kısıtlar `invalid_profile_proposal` ile reddedilir.
- **`road.profiles.check`:** açıklığı ya da örtüsü yetmeyen her kesişme `clearance` kuralıyla bulgu olur. Kesişmesi
  olan boykesit başka kural verilmeden de denetlenir.

### Pencereler

- **Boykesit:**
  - "Kesişmeler" tablosu: ad, km, yer, kot, açıklık ya da örtü ve şimdiki çizgiye göre pay;
  - resimde üst geçit tabla, menfez kutu olarak çizilir; sınır kesiklidir, yetiyorsa yeşil, yetmiyorsa kırmızıdır.
- **Kırmızı kot önerisi:**
  - "Kesişmeler" listesi: sınır, önerilen kot, pay, tutuyor;
  - tutan kesişmeleri söyleyen bir bant;
  - resimde sınırlar.

## Ortak durumlar ve ölçüm

Başvurular KentOS kodu olmadan çalışır.

- **Öneri** (`road_profile_propose_cases.py`): kesişmeler SLSQP'nin başlangıcına ve kesirlerle çözülen KKT
  sertifikasına doğrusal satır olarak eklenir.
- **Denetim** (`road_design_check_cases.py`): kesişmenin kotu düşey başvurunun kesin kotudur.
- **Düzenleme** (`road_profile_command_cases.py`).

Ölçümler:

- **Düz boykesit** (PVI'lar 0, 200, 420, 650 m):
  - kısıtsız öneri 300 m'de 107,910, 520 m'de 111,666 m'dir;
  - 300 m'deki üst geçit (≤ 107,0) ve 520 m'deki menfez (≥ 112,5) öneriyi tutar, ikisinin payı 0'dır;
  - 100 m'deki boru 1,420 m payla serbesttir.
- **Elle bulunan yapılamazlık:** üst geçitten menfeze çizgi 220 m'de 5,5 m tırmanmalıdır, yani en az %2,5. %3 sınırla
  öneri kurulur, son eğim sınırdadır. %2 sınırla iki uygulama da reddeder.
- **Başvurunun hatası:** ilk yazımında eğim sınırı yokken kesişme satırlarını eğim satırı sandı ve "en büyük çıkış"
  bayrağı koydu. Rust doğruydu; başvuru düzeltildi.

Ortak durumlar: öneri 31, denetim 23, düzenleme durumları. Hepsi Rust'ta ve Python tipli katmanında aynıdır.

Resimler `.run/gui-evidence/yol-y40/` altındadır.

## Açık

- Kesişmenin genişliği: köprü tablası tek istasyonda değil, bir aralıkta açıklık ister. Bugün tek istasyondur.
- Kesişen yolun kendi boykesitine bağlanma (kavşak kolları).
