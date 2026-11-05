# 0966 — Boykesitte noktada eğrilik

Tarih: 2026-11-05. Durum: kabul (Y60). Kapsam ROAD-PROFILE-12.

## Sorun

ROAD-PROFILE-12: "Analitik point/grade/curvature query, profile report ve sheet üretimi aynı komutlardan çalışsın."

Durum şuydu:

- **`road.profiles.query`:** Bir km'de kırmızı kotu ve eğimi veriyordu. Eğriliği yalnız kurp başına K ve H olarak
  veriyordu (ADR 0961); bir noktadaki eğrilik sorulamıyordu.
- **Görünüşler penceresi:** Km'yi üç görünüşte eşliyordu ama o km'deki kırmızı kotu söylemiyordu.

## Araştırma

Bu işte iki tanım kullanılır; ikisi de kabul görmüştür:

- **Eğimin değişim hızı (AASHTO Green Book, düşey kurplar):** r = (g₂ − g₁)/L. Parabolde z″'nün kendisidir ve kurp
  boyunca sabittir.
- **Düzlem eğrisinin eğrilik yarıçapı (diferansiyel geometri):** R = (1 + z′²)^{3/2} / |z″|.
  - Dairede bu, dairenin yarıçapıdır.
  - Parabolde yalnız z′ = 0'da L/|Δg| = 100·K'ye, yani ADR 0961'in H'sine eşittir. Başka yerde biraz büyüktür.

Yeni bir formül yoktur.

## Karar

`road.profiles.query`'nin her km satırına (`ProfileAt`) iki alan eklendi:

- `rate` (1/m): z″, işaretli; tepede eksi, düz eğimde 0.
- `radius` (m): (1 + z′²)^{3/2}/|z″|; düz eğimde yok.

Çekirdek bunları zaten hesaplıyordu (`Vertical::at` → kot, eğim, z″); komut onları yalnız göstermiyordu.

**Aynı komuttan okunanlar:**

- **Görünüşler penceresi:** Kaydırıcının km'sindeki kırmızı kotu, eğimi ve yarıçapı alt satırda yazar ("km 1+340.000:
  kırmızı kot 109,95 · eğim %1,40 · R 1200 m"; düz eğimde "düz eğim"). Bunları `road_profiles::point_at`'ten okur;
  `road.profiles.query`'nin kullandığı işlev de odur.
- **Satırdaki iki km:** Satır kesitin km'sini ayrıca söyler. Gösterilen kesit en yakın kesittir, kırmızı kot ise km'nin
  kendisinde okunur. İkisinin karışmaması için kırmızı kot kendi km'siyle yazılır.
- **Boykesit raporu:** Kurp başına K, H ve tepe noktası aynı komutun `curves` alanındadır.
- **Pafta:** `road.sheets` boykesiti `road_profiles_draw::view` ile çizer; `road.profiles.draw`'ın çizdiği de odur.

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_profile_command_cases.py` düşey referanstan (`road_vertical_cases.py`, kesirler ve mpmath) z″'yü
alır; dairede kapalı biçim R²/h³'tür. Yeni durum "Noktada eğrilik: eğimde, parabolde, dairede, asimetrik kollarda":

| km | Yer | z″ (1/m) | R (m) |
|---|---|---|---|
| 100 | düz eğim | 0 | yok |
| 150, 170 | parabol v1 (L 80) | 7,353·10⁻⁴ (= A/L) | 1360,3 / 1361,4 (1/r = 1360 = 100·K, K 13,6) |
| 320, 340, 360 | daire v2 (R 1200) | −8,33·10⁻⁴ | **1200,0** her noktada |
| 500 | asimetrik v3, ilk kol | 6,16·10⁻⁴ | 1624,5 |
| 540 | asimetrik v3, ikinci kol | 1,256·10⁻³ | 796,9 (1/r = 795,9 = ADR 0961'in H'si) |

Daire, formülün çapraz sınamasıdır: genel eğrilik formülü her noktada dairenin yarıçapını verir.

Rust'ta ve Python'da geçer.

### Pencere testi

`road_views_window_tests::the_red_line_at_the_km_is_the_query_s`, kaydırıcıyı 21 km'ye götürür. Her birinde pencerenin
okuduğu kot, eğim, z″ ve R ile `road.profiles.query`'nin yanıtı aynıdır.

## Açık

- **Boykesit çiziminin etiketi:** Kurp etiketi hâlâ "K=…" yazar; birim ve H etikette yok (ADR 0961'in açık maddesi).
