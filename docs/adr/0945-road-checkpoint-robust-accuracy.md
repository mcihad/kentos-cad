# 0945 — Kontrol noktalarıyla yüzeyin doğruluğu: tek bir RMSE yetmez

Tarih: 2026-11-05. Durum: kabul (Y39.1 çekirdek). Kapsam ROAD-SURVEY-05.

## Sorun

Y18'in kalite komutu (ADR 0916 §4) kontrol noktalarının yüzeyden farkını sınıf sınıf ortalama, standart sapma ve RMSE
olarak verir. ROAD-SURVEY-05 şunu ister: "Sert zemin ve bitki kaplı alan için uygun checkpoint sınıfları/istatistikleri
ayrı göster; yalnız tek RMSE sayısına dayanma."

Tek bir RMSE üç şeyi saklar:

- **Aykırı birkaç nokta** RMSE'yi büyütür. Yüzey iyi olsa da kötü görünür.
- **Bitkili zeminde** farklar çarpıktır: lazer bitkinin üstüne düşer, yüzey yukarıda kalır. Normal dağılım varsayan
  "1,96 × RMSE" %95'i söylemez.
- **Az noktayla** RMSE'nin kendisi belirsizdir. Kaç nokta olduğu söylenmeden "RMSE 5 cm" bir şey demez.

## Dayanak

- **Höhle ve Höhle (2009),** *Accuracy assessment of digital elevation models by means of robust statistical methods*,
  ISPRS J. Photogramm. 64, 398–406: dağılımı normal olmayan yükseklik modelinin doğruluğu medyan, NMAD ve |Δh|'nin
  örneklem yüzdelikleriyle (%68,3 ve %95) değerlendirilmelidir.
  - NMAD = 1,4826 · medyan(|Δh − medyan(Δh)|).
  - 1,4826 = 1/Φ⁻¹(3/4): normal dağılımda NMAD standart sapmadır, ama aykırı noktaları görmez.
- **ASPRS Konum Doğruluğu Standartları, 2. baskı (2023):**
  - doğruluk RMSE ile bildirilir, %95 güven düzeyi bırakılmıştır;
  - en az 30 kontrol noktası gerekir;
  - kontrol noktalarının kendi ölçme hatası ürünün doğruluğuna katılır.
  Bir üretici kılavuzu (LP360) birleşik hatayı √(RMSE_ürün² + RMSE_ölçü²) diye verir. Bu formül ASPRS metninin
  kendisinden doğrulanamadı; KentOS onu seçenek olarak, bu açıklamayla sunar.
- **RMSE'nin güven aralığı:** farklar normal ve yansız ise n·RMSE²/σ² serbestlik derecesi n olan χ²'dir. Bu yüzden σ,
  %95 ile [√(n·RMSE²/χ²₀,₉₇₅,ₙ), √(n·RMSE²/χ²₀,₀₂₅,ₙ)] aralığındadır.

## Karar (`kentos_road::accuracy`)

**Farklar** (kontrol noktası eksi yüzey) için:

- sayı, ortalama;
- RMSE ve %95 aralığı;
- medyan ve NMAD;
- |Δ|'nin %68,3 ve %95 yüzdelikleri: Hyndman–Fan'ın 7. tanımı, sıralı değerler arasında doğrusal (numpy ve R'nin
  öntanımlısı);
- çarpıklık g₁ = m₃/m₂^{3/2} ve basıklık fazlası g₂ = m₄/m₂² − 3, merkezî momentlerle (mₖ = Σ(d − d̄)ᵏ/n). İkisi de
  normal dağılımda 0'dır;
- **uzak noktalar:** |d − medyan| > 3 · NMAD. İşaretlenir, çıkarılmaz; çıkarmak kullanıcının kararıdır (Y18.6'nın
  onarımı);
- kontrol noktalarının kendi RMSE'si verilirse birlikte √(RMSE² + s²).

**Hesap ayrıntıları:**

- χ² yüzdeliği kendi yazılımımızdır: düzenli alt eksik gamma P(a, x), x < a + 1'de serisiyle, üstünde Lentz'in sürekli
  kesriyle; yüzdelik ikiye bölmeyle; ln Γ Lanczos ile (g = 7, 9 terim).
- **Yayılımı olmayan farklar:** hepsi eşit farkların ortalaması yuvarlanınca m₂ 0 değil ~1e-36 çıkar. m₂ ≤ 1e-12 · RMSE²
  ise yayılım yok sayılır; çarpıklık ve basıklık 0'dır.

## Ortak durumlar ve ölçüm

`scripts/fixtures/road_accuracy_cases.py` KentOS kodu olmadan çalışır: numpy'nin medyanı ve yüzdeliği, scipy'nin χ²
yüzdeliği, çarpıklığı ve basıklığı.

- **15 dağılım:** normal, kaymalı, bitkili (sağa çarpık), üç aykırılı, düzgün, iki ve üç nokta, hepsi aynı, yarısı sıfır,
  kendi doğruluğu olan, 1000 nokta, tek yönlü sistematik, ağır kuyruklu (t₃), tam sayılar, 30 nokta.
- **36 χ² yüzdeliği:** k = 1…1000.

Rust χ²'yi 1e-10, bütün ölçüleri 1e-9 içinde aynı bulur ve uzak noktaları aynı seçer
(`crates/shared/road/tests/all/accuracy.rs`).

**Ölçüm (başvurunun durumlarından):**

| Dağılım | RMSE (%95 aralığı) | Medyan | NMAD | %95 yüzdelik |
|---|---|---|---|---|
| Normal, σ 5 cm, 40 nokta | 4,6 (3,8–5,9) cm | 0,4 cm | 4,2 cm | 9,5 cm |
| Normal ve üç aykırı | **18,8** cm | −0,5 cm | **3,8** cm | 45,8 cm |
| Bitkili zemin (çarpıklık 1,65) | 9,7 cm | 3,9 cm | 5,4 cm | **25,1** cm |
| İki nokta | 2,2 (**1,2–14,1**) cm | — | — | — |
| Tek yönlü sistematik | 9,8 cm | 9,6 cm | **1,5** cm | 11,2 cm |

Bu tablodan çıkanlar:

- **Üç aykırı nokta RMSE'yi beşe katlıyor.** NMAD yüzeyin asıl dağılımını gösteriyor.
- **Bitkili zeminde %95 yüzdelik (25 cm),** normal dağılım varsayımının 1,96 × RMSE'sinden (19 cm) büyük.
- **İki noktalı RMSE'nin aralığı 12 kat geniş.** ASPRS'in 30 nokta istemesinin sayısal karşılığı budur.
- **Sistematik kaymada küçük NMAD ile büyük medyan** hatanın rastgele değil kayma olduğunu söyler.

## Açık

- Yatay doğruluk (x, y) kontrol noktaları.
- Sınıf sınıf kabul ölçütleri projenindir; KentOS sayı vermez.
