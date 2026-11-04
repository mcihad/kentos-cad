# 0941 — Kübajın belirsizlik bütçesi: ölçme, kayma, yöntem ve aritmetik

Tarih: 2026-11-04. Durum: kabul (Y35.1 çekirdek; Y35.2 komut, Y35.3 pencere). Kapsam ROAD-VOL-14.

## Sorun

Kübaj tek bir sayı veriyor: kazı 23.408,7 m³. Bu sayının ne kadar güvenilir olduğu hiçbir yerde söylenmiyor. Oysa
sayıya hakediş bağlanıyor.

ROAD-VOL-14 şunu ister: "Quantity report numeric/mesh/measurement error bileşenleri ve kabul edilen bounds taşısın; FP64
hesap survey kalitesini artırmış sayılmasın."

Hatanın dört kaynağı var:

- **Arazi ölçüsünün rastgele hatası:** her noktanın kotu σ ile şaşar.
- **Sistematik kayma:** bir datum ya da GNSS anten yüksekliği bütün kotları β kadar kaydırır.
- **Hesap yöntemi:** kesitler arasını ortalama alan ya da prizmoidal formül tahmin eder.
- **İkili sayı aritmetiği.**

## Dayanak

- **Varyansların yayılımı:** doğrusal bir fonksiyonun varyansı `Var(Σ wⱼ·εⱼ) = Σ wⱼ²·σⱼ²`'dir (εⱼ bağımsız). Bu,
  belirsizlik ifadesi kılavuzlarının (GUM) birinci mertebe yayılımıdır.
- **Toplamanın yuvarlama sınırı:** n terimin binary64 toplamı en çok `n·u·Σ|vᵢ|` şaşar, u = 2⁻⁵³ (Higham, *Accuracy and
  Stability of Numerical Algorithms*, özyinelemeli toplama).

## Karar

### §1 Duyarlılık: her arazi noktası hacmi ne kadar oynatır

- d = z_tasarım − z_arazi ve arazi her üçgende doğrusaldır: z = Σ φⱼ·zⱼ (φ köşelerin ağırlık fonksiyonları).
- Kazı = ∫ max(−d, 0) ve dolgu = ∫ max(d, 0) olduğundan:
  - ∂kazı/∂zⱼ = ∫_{d<0} φⱼ dA;
  - ∂dolgu/∂zⱼ = −∫_{d>0} φⱼ dA.
- Sıfır çizgisi de kayar, ama orada d = 0 olduğu için katkısı ikinci mertebedir.
- Bindirmenin her parçası dışbükeydir ve φ üstünde doğrusaldır. Bu yüzden ∫φ = alan·φ(ağırlık merkezi) kesin
  eşitliktir.
- Uygulama: `kentos_road::volume::sensitivity`. Ağırlıkların toplamı plan alanını verir (Σφ = 1).

### §2 Bileşenler (`kentos_road::volume_uncertainty`)

- **Rastgele:** `σ_V = σ·√Σwⱼ²`, kazı, dolgu ve net için ayrı.
  - Net hacim (kazı − dolgu) kotlara **tam doğrusal** bağlıdır; σ'sı kesindir.
  - Kazı ve dolgu ayrı ayrı yalnız sıfır çizgisinden uzakta doğrusaldır. Sıfır çizgisinin 2σ yakınındaki alan payı
    raporlanır; pay %50'yi aşınca ikisinin σ'sı yalnız yaklaşık sayılır (§ Ölçüm).
- **Sistematik:** `β·Σwⱼ = β·A`. Ortalanarak azalmaz, olduğu gibi eklenir.
- **%95 sınırı:** `|β·A| + 1,96·σ`.
- **Aritmetik:** `n·u·Σ|vᵢ|`. Herhangi bir yol için bir litrenin çok altındadır. Bu, FP64'ün ölçü kalitesini
  artırmadığını sayıyla gösterir.

σ, β ve kabul payı projenindir; KentOS sayı vermez.

## Ortak durumlar ve ölçüm

`scripts/fixtures/road_volume_uncertainty_cases.py` KentOS kodu olmadan çalışır.

- **Ağırlıklar:** Y8'in kesirli bindirme başvurusunun üstünde, kesir aritmetiğiyle ayrı yazıldı.
- **Türev denetimi:** her ağırlık, o köşe 10⁻⁶ m kaldırılıp indirilerek hesaplanan kesin hacimlerin merkezi farkıyla
  karşılaştırılır.
- **İstatistik denetimi:** kotlar 4000 kez N(0, σ) ile bozulur (sabit tohum) ve hacimlerin standart sapması ölçülür.

21 durum. Rust ağırlıkları, alanları ve σ'ları 1e-9 ile aynı bulur
(`crates/shared/road/tests/all/volume_uncertainty.rs`).

**Monte Carlo ile doğrusal yayılım (4000 deneme):**

| Durum | Yakın alan payı | σ kazı (yayılım / MC) | σ dolgu | σ net |
|---|---|---|---|---|
| Düz arazi, düz dolgu | 0,00 | — | 7,862 / 7,811 | 7,862 / 7,811 |
| Dalgalı arazi, düz platform | 0,28 | 12,197 / 12,173 | 8,884 / 8,977 | 17,834 / 17,921 |
| Kenarlarla hizalı tasarım | 0,50 | 4,310 / 4,293 | 4,310 / 4,251 | 7,546 / 7,445 |
| Büyük σ (0,2 m), ±0,5 m arazi | 1,00 | **10,718 / 14,035** | **32,496 / 29,254** | 38,997 / 39,203 |
| Tasarım arazinin 1 cm üstünde | 1,00 | **0,000 / 3,282** | **7,546 / 4,910** | 7,546 / 7,461 |

- **Net hacmin σ'sı:** 21 durumun hepsinde Monte Carlo'yla %2 içinde tutar.
- **Kazı ve dolgunun σ'sı:** yakın alan payı %50'ye kadar tutar, pay %100 olunca %31'e varan sapma gösterir.

Kabul sınırı bu yüzden %50 seçildi.

## Açık

- **Komut (Y35.2):** σ ve β ile hacmin bileşenleri, yöntem farkı, aritmetik sınırı, kabul.
- **Pencere (Y35.3).**
- **Uzamsal korelasyon:** komşu noktaların hataları ilişkiliyse (aynı istasyondan ölçülmüşlerse) σ büyür. Bunun için
  bir kovaryans modeli gerekir.
- **Nokta başına ayrı σ:** GNSS ile total station noktaları birlikte kullanıldığında.
