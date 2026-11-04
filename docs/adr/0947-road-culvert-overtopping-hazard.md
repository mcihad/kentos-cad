# 0947 — Menfezin su yüksekliği, yolun taşması ve taşan suyun tehlikesi

Tarih: 2026-11-05. Durum: kabul (Y41.1 çekirdek, Y41.2 komut, Y41.3 pencere). Kapsam ROAD-DRAIN-08.

## Sorun

Y16'nın drenajı (ADR 0929) bir menfezin yalnız dolu akış kapasitesini (Manning) veriyor. Asıl sorular cevapsız:

- Tasarım debisinde menfezin önünde su ne kadar yükselir?
- Su yolun sırtını aşar mı?
- Aşarsa yolun üstünden ne kadar su, ne derinlik ve ne hızla geçer? Yol hangi araçlar için kapanmalıdır?

ROAD-DRAIN-08 şunu ister: "Taşkın nedeniyle yol kapanması karar kuralı depth/velocity/time/vehicle class ve
uncertainty'yi açık alsın."

## Dayanak

- **FHWA HDS-5 (Hydraulic Design of Highway Culverts), giriş kontrolü:** X = Ku·Q/(A·√D) (SI'da Ku = 1,811; X ABD
  birimlerindeki değerdir) ile:
  - batmamış giriş: form 1 HW/D = Hc/D + K·X^M + Ks·S, form 2 HW/D = K·X^M;
  - batmış giriş: HW/D = c·X² + Y + Ks·S;
  - X ≤ 3,5 batmamış, X ≥ 4,0 batmış, arası doğrusal geçiş.
- **Katsayılar:** HDS-5 Tablo A.1'in yedi satırı (üç boru, dört kutu menfez girişi), tablonun bir yeniden yayımından
  (tbn2net, "Inlet Coefficients") alındı ve tek tek karşılaştırıldı. Özgün FHWA belgesiyle karşılaştırılamadı; tasarımda
  özgün tabloya bakılmalıdır.
- **Giriş kaybı katsayısı ke** (HDS-5 Tablo C.2) doğrulanamadı. KentOS ke vermez, kullanıcı yazar.
- **Çıkış kontrolü**, HDS-5'in dolu boru yaklaşımı: HW = TW' + (1 + ke + 2g·n²·L/R^(4/3))·V²/(2g) − S·L,
  TW' = max(TW, (dc + D)/2).
- **Taşan suyun tehlikesi:** AIDR'nin 7-3 kılavuzu (Avustralya, Smith ve Wasko 2012'den) altı sınıf verir. Sınırlar
  D·V, derinlik ve hız birlikte; kılavuzun PDF'inden okundu:

| Sınıf | Anlamı | D·V (m²/s) | D (m) | V (m/s) |
|---|---|---|---|---|
| H1 | araç, insan ve yapı için genelde güvenli | ≤ 0,3 | 0,3 | 2,0 |
| H2 | küçük araçlar için güvensiz | ≤ 0,6 | 0,5 | 2,0 |
| H3 | araçlar, çocuklar ve yaşlılar için güvensiz | ≤ 0,6 | 1,2 | 2,0 |
| H4 | araçlar ve insanlar için güvensiz | ≤ 1,0 | 2,0 | 2,0 |
| H5 | araçlar ve insanlar için güvensiz, yapılar hasar görebilir | ≤ 4,0 | 4,0 | 4,0 |
| H6 | araçlar ve insanlar için güvensiz, yapılar yıkılabilir | > 4,0 | — | — |

## Karar (`kentos_road::culvert_hydraulics`)

**Menfez:**

- kesit (daire ya da kutu, bir göz), göz sayısı, boy, eğim, Manning n;
- HDS-5 giriş katsayıları ve ke.
- Debi gözlere eşit bölünür.

**Kritik derinlik:**

- kutuda (q²/g)^(1/3), en çok kutunun yüksekliği;
- dairede A³/T = Q²/g ikiye bölmeyle (A³/T taça doğru sonsuza büyür).

**Su yüksekliği** (giriş tabanının üstünde):

- giriş ve çıkış kontrolünün büyüğüdür;
- debi sıfırsa kuyruk suyunun seviyesidir. HDS-5'in (dc + D)/2 kuralı akan bir boru içindir; sıfır debide anlamsız
  bir 0,3 m veriyordu (başvuruda görüldü).
- Sıfır debide iki kontrolün su yüksekliği de öyledir: giriş 0, çıkış kuyruk suyunun seviyesi. Giriş denklemi
  sıfır debide Ks·S kadar eksi bir değer (−0,01 m) veriyordu; performans eğrisinin ilk satırında görüldü (Y41.3).

**Taşma:**

- su yüksekliği yol sırtını aşarsa sırt bir savaktır: Q = Cd·L·h^(3/2);
- **Cd ve taşma boyu L projenindir;** KentOS sayı vermez;
- menfez ve savak, ortak bir su yüksekliğinde birlikte debiyi taşır. İkisinin taşıdığı su yüksekliğiyle arttığı için
  ikiye bölmeyle bulunur.

**Sırtta:**

- akış kritik derinlikten geçer: y = (q²/g)^(1/3), v = √(g·y);
- bu yüzden **D·V = q**, yolun metresinden geçen debidir. Tehlike sınıfı q'dan doğrudan okunur.

## Ortak durumlar ve ölçüm

`scripts/fixtures/road_culvert_hydraulics_cases.py` KentOS kodu olmadan çalışır: mpmath ile 40 basamak.

- **Kritik derinlik** findroot'la (Illinois) bulunur, logaritmik biçimde (ln A³/T, derinlik boyunca 0'dan sonsuza iyi
  ölçeklidir). İlk denemede Anderson çözücüsü taça yakın takıldı.
- **Bir su yüksekliğindeki debi** ve **taşmanın bölüşümü** kendi denklemlerinin ikiye bölmesiyle bulunur.
- **Denetimler:** kutunun kritik derinliği kapalı biçimle karşılaştırılır; menfez + savak = debi; sırtta D·V = q;
  sınıfların sınırları.

18 durum var:

- üç boru ve dört kutu girişi;
- batmamış, geçiş ve batmış giriş;
- çıkış kontrolü (yüksek kuyruk suyu; uzun, pürüzlü, yatık menfez);
- iki gözlü ve üç borulu menfez;
- dört taşma;
- sıfır debi.

Rust su yüksekliğini, kontrolü, iki kontrolün su yüksekliğini, bölüşümü, sırttaki derinliği ve hızı ve sınıfı 1e-9
içinde aynı bulur (`crates/shared/road/tests/all/culvert_hydraulics.rs`).

**Örnek** (1,2 m'lik beton boru, kare ağızlı başlık; sırt girişin 2,2 m üstünde, 40 m taşma boyu, Cd 1,66):

| Debi | Yoldan geçen | Sırtta derinlik ve hız | Sınıf |
|---|---|---|---|
| 5 m³/s | 1,20 m³/s | 0,045 m, 0,67 m/s | H1 |
| 30 m³/s | 25,65 m³/s | 0,347 m, 1,85 m/s | H4 |
| 15 m³/s, taşma boyu 8 m | 10,31 m³/s | 0,553 m, 2,33 m/s | H5 |

Yarı debi (15 m³/s), 8 m'lik bir çukurda toplanınca, 40 m'ye yayılan 30 m³/s'den daha tehlikelidir: H4 yerine H5.
Taşma boyu, yani yol boykesitinin çukurunun genişliği, menfez kadar önemlidir.

## Komut: `road.culverts.hydraulics` (Y41.2)

Kayıtlı bir menfezi bir debide okur. Yalnız okur.

- **Girdi:** menfez, debi (m³/s), kuyruk suyu (yoksa 0), giriş (HDS-5'in yedi satırından biri), giriş kaybı ke,
  Manning n (yoksa menfezinki), savak (Cd, boy, kret; kret yoksa yol), eğri (2–200 nokta, 0'dan 1,5 × debiye).
- **Menfezden okunan:** giriş ve çıkış ucunun akış kotu, gövde boyu (açıyla), eğim = (giriş − çıkış) / boy, göz sayısı.
  Hepsi `road.culverts.query`'nin okuduğudur.
- **Kret yoksa** gövdenin menteşeleri arasındaki yol üstünün en yüksek kotudur: su yolu sırtından aşar.
- **Retler:** `not_a_road_project`, `unknown_culvert`, `invalid_culvert_hydraulics` (debi 0–10000, kuyruk suyu 0–100,
  ke 0–2, n yoksa ya da 0–0,1 dışındaysa, Cd 0–3, savak boyu 0–10000, kret giriş kotunun altındaysa, eğri 2–200),
  `invalid_culvert`.
- **Uyarı:** `culvert_overtopped` yolu aşan debiyi, kret üstündeki suyu ve tehlike sınıfını söyler; okumanın uyarıları
  (`culvert_stale`, `culvert_cover`, `corridor_stale`, `corridor_unsolved`) geçer.
- KentOS ke, n, Cd ve taşma boyu için sayı vermez.

`scripts/fixtures/road_culvert_hydraulics_command_cases.py` 30 durum yazar. Menfezin okunuşu `road_culvert_cases.py`'nin,
hidrolik yukarıdaki 40 basamaklı başvurunun; KentOS kodu yok. Rust ve Python 1e-6 içinde aynı bulur.

Ortak çizimin kutu menfezi (2 × 1,5 m, iki göz, 21,94 m, %1; giriş 106,589, yol kreti 109,846; 30°–75° kanat, ke 0,4,
n 0,013; savak Cd 1,66):

| Debi, savak boyu | Su kotu | Kontrol | Yoldan | Sırtta derinlik, hız | Sınıf |
|---|---|---|---|---|---|
| 4 m³/s, savaksız | 107,390 | çıkış ((dc + D)/2) | — | — | — |
| 12 m³/s, 60 m | 108,155 | giriş | 0 (krete 1,69 m) | — | — |
| 60 m³/s, 60 m | 110,313 | giriş | 31,79 m³/s | 0,306 m, 1,73 m/s | H2 |
| 60 m³/s, 10 m | 111,230 | giriş | 27,04 m³/s | 0,907 m, 2,98 m/s | H5 |

Aynı debi 10 m'lik bir çukurda toplanınca hız 2 m/s'yi aşar ve sınıf H2'den H5'e çıkar.

## Pencere: Menfez → Hidrolik (Y41.3)

Araştırma (5 Kasım 2026): FHWA'nın HY-8'i bir geçidi iki tabloyla gösterir. Geçit özetinde toplam debi, menfezden ve
yoldan geçen debi ve su kotu vardır. Menfez özetinde ayrıca giriş ve çıkış kontrolünün su derinliği bulunur. Yanında su
kotu–debi performans eğrisi durur.

Menfez penceresi iki sayfa oldu: **Gövde** (eski resim ve uçlar) ve **Hidrolik**.

- **Alanlar:** debi, kuyruk suyu, giriş (kesite göre üç boru ya da dört kutu satırı), ke, n, savak Cd, savak boyu,
  kret kotu. Savak Cd ile savak boyu birlikte yazılır.
- **Okuma satırı:** su yüksekliği ve kotu, kontrol ve iki kontrolün su yüksekliği, menfezden geçen debi. Yol
  aşılmıyorsa krete kalan pay; aşılıyorsa yoldan geçen debi ve sırttaki derinlik, hız, sınıf.
- **Taşmada** D·V ile sınıf ayrıca uyarı renginde yazılır, H3'ten yukarısı tehlike renginde. Kapatma kararı
  tasarımcınındır.
- **Performans eğrisi:** 13 nokta. Su kotu düz çizgi, giriş ve çıkış kontrolü kesikli, yol kreti kırmızı. Taşan noktalar
  sınıf rengiyle işaretlenir, sorulan debi daireyle gösterilir. Yanında HY-8'in özet tablosu durur: toplam, menfez, yol,
  su kotu, giriş HW, çıkış HW, kontrol, tehlike.
- **Dar pencere:** eğri genişliğin 5/9'unu alır, tablo kendi içinde kayar. Önce tablonun sabit sütunları eğriyi
  1100 px'lik pencerede yaklaşık 210 px'e sıkıştırıyordu.
- Hidrolik kayıtlı menfezi okur; yazılıp kaydedilmemiş ölçüler okunmaz. Bunu pencere de söyler.

## Açık

- Zaman: hidrografla taşmanın süresi.
- Belirsizlik: debinin ve Cd'nin aralığı.
- Uzun yaklaşık yöntem yerine HDS-5'in kısmi dolu çıkış kontrolü (su yüzü eğrisi).
