# 0948 — Taşkında yolun kapanma süresi, araç sınıfları ve belirsizlik

Tarih: 2026-11-05. Durum: kabul (Y42.1 çekirdek). Kapsam ROAD-DRAIN-08'in açık kalanı (süre, araç sınıfı, belirsizlik).

## Sorun

ADR 0947 menfezin bir debide yolu aşıp aşmadığını, sırttaki derinliği, hızı ve tehlike sınıfını veriyor. ROAD-DRAIN-08
ayrıca süreyi ve belirsizliği ister. Taşkın bir debi değil, zamanla yükselip inen bir hidrograftır. Sorular:

- Yol kaç saat su altında kalır?
- Hangi araç sınıfına kaç saat kapalıdır? Ne zaman kapanır, ne zaman açılır?
- Debi ve savak katsayısı belirsizken bu süreler hangi aralıktadır?

## Dayanak

- **Austroads, Guide to Road Design Part 5 / 5B** (araştırma 5 Kasım 2026, içindekiler ve Queensland eğitim sunumu):
  "time of submergence", yolun derinliği ne olursa olsun su altında kaldığı süredir. Kapanma süresi ve yeniden açılma
  ayrı başlıklardır. Hesap yöntemi belgenin kendisinden okunamadı; burada süre doğrudan hidrograftan ölçülür.
- **Araç kararlılığı:** Shand ve ark. (2011), ARR Project 10'un duran araç için ara ölçütleri. UNSW WRL 2017/07
  raporu (Tablo 2-1) aynı tabloyu verir ve deneylerle ölçütleri uygun bulur. Rapordan okunan sınırlar:

| Sınıf | Durgun suda derinlik (m) | Hız (m/s) | D·V (m²/s) |
|---|---|---|---|
| Küçük binek (< 4,3 m, < 1250 kg) | 0,3 | 3,0 | ≤ 0,3 |
| Büyük binek (> 4,3 m, > 1250 kg) | 0,4 | 3,0 | ≤ 0,45 |
| Büyük 4×4 (> 4,5 m, > 2000 kg) | 0,5 | 3,0 | ≤ 0,6 |

  Derinlik, hız ya da D·V sınırı aşılınca araç güvensizdir. Rapor, gerçek taşkında derinlik ve hızın gözle
  kestirilemeyeceği uyarısını yapar: bu bir tasarım ölçütüdür, sahada karar aracı değildir.
- **Tehlike sınıfları** ADR 0947'nin (AIDR 7-3) sınıflarıdır.

## Karar (`kentos_road::culvert_closure`)

**Tekdüzelik.** Yoldan geçen debi Cd·L·h^1.5'tir. Kret üstündeki su yüksekliği h toplam debiyle artar, yoldan geçen de
onunla birlikte artar. Sırtta akış kritik derinlikten geçer: derinlik, hız ve D·V metre başına debiyle artar. Her ölçüt
bunlara bir üst sınırdır ve bir kez aşılınca debi arttıkça aşık kalır (tehlike sınıflarının sınırları da sınıftan
sınıfa azalmaz). Yani her ölçüt bir **eşik toplam debiden** yukarıda tutar.

**Eşik:**

- Su altında kalmanınki: menfezin, su kretteyken geçirdiği debi.
- Ötekiler kret üstündeki su yüksekliği üzerinde, ölçütün kendisiyle ikiye bölünerek bulunur. Eşik debi, menfezin o
  yükseklikteki debisi artı yolunkidir.
- İlk sürüm toplam debiyi ikiye bölüyordu; her adımı bir performans hesabıydı ve 12 durum hata ayıklama derlemesinde
  499 s sürdü. Su yüksekliği üzerinde 0,10 s.

**Süre.** Hidrograf noktaları arasında doğrusaldır. Eşiğin üstündeki süre her parçada kesin bulunur; aralıklar
başlangıç ve bitiş saatleriyle verilir.

**Belirsizlik.** Debi çarpanı ve Cd için birer aralık alınır. Bütün süreler ikisiyle birlikte artar:

- debi çarpanı her noktayı büyütür;
- büyük Cd verilen debide suyu daha düşük yükseklikte yoldan geçirir, menfez daha az, yol daha çok taşır ve her eşik
  düşer.

Bu yüzden aralık örneklenmez, **kesindir**: en kısa süre en küçük çarpan ve Cd'de, en uzunu ikisinin en büyüğündedir.
Su altında kalma eşiği Cd'ye bağlı değildir.

**Hesaba katılmayanlar:** menfezin önünde biriken su (öteleme tepe debiyi düşürürdü), debiyle yükselen kuyruk suyu,
savak boyunun belirsizliği (süreler onunla tekdüze değil).

## Ortak durumlar ve ölçüm

`scripts/fixtures/road_culvert_closure_cases.py` KentOS kodu olmadan ve **başka bir yoldan** hesaplar. Her ölçüt sırtta
metre başına debiye bir sınırdır: q* = min(p, √g·d^1.5, v³/g). Eşik toplam debi, menfezin kret + (q*/Cd)^(2/3)
yüksekliğindeki debisi artı L·q*'dir; debi üzerinde arama yoktur. Hidrolik ADR 0947'nin 40 basamaklı mpmath
başvurusudur.

12 durumda 16 taşkın var:

- üçgen, taban debili, iki tepeli, düz tepeli ve başta yüksek hidrograf;
- aşmayan taşkın, dar çukur, yüksek kuyruk suyu, kutu menfez;
- iki üç noktalı aralık.

Rust eşikleri ve süreleri 1e-7 içinde aynı bulur (`crates/shared/road/tests/all/culvert_closure.rs`).

**Bulgular:**

- **Sırtta sınıflar metre başına debiye iner.** Kritik akışta araçların D·V sınırı derinlik ve hız sınırından önce
  gelir: q* küçük binekte 0,3, büyük binekte 0,45, 4×4'te 0,6 m²/s. H2 eşiği küçük binekle aynıdır. H3 ile H4 aynı
  debide başlar; aralarındaki tek fark olan derinlik sınırı sırtta etkisizdir.
- 1,2 m boru, sırt girişin 2,2 m üstünde, 40 m taşma boyu, Cd 1,66; üçgen hidrograf, tepe 30 m³/s 2. saatte, taban 8
  saat:

| Ölçüt | Eşik debi | Süre |
|---|---|---|
| Su altında | 3,71 m³/s | 7,01 sa |
| Küçük binek | 16,11 m³/s | 3,70 sa |
| Büyük binek | 22,22 m³/s | 2,07 sa |
| Büyük 4×4 | 28,33 m³/s | 0,45 sa |

- **Belirsizliği debi belirler.** Aynı taşkında çarpan 0,8–1,2 ve Cd 1,5–1,8 iken küçük binek 2,62–4,43 sa, 4×4
  0–1,71 sa kapalıdır. Cd'nin ±%10'u eşiği yalnız binde 1–2 kaydırır (16,13 / 16,09 m³/s): eşiği menfezin
  kapasitesi belirler.
- Yüksek kuyruk suyu (1,8 m) yolu daha erken su altında bırakır: eşik 3,71'den 2,97 m³/s'ye iner.

## Açık

- Komut, Python/MCP ve pencere (Y42.2, Y42.3).
- Menfez önündeki biriktirmeyle hidrograf ötelemesi; debiyle değişen kuyruk suyu.
- Yayalar için sınırlar (ARR'nin insan güvenliği ölçütleri) ve yeniden açılma süresi.
