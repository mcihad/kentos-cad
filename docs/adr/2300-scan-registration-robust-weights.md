# 2300 — Tarama kaydında kaba hatayı atan ağırlıklar

Tarih: 2026-11-08. plan/map-cad.md MAP-13, üçüncü tur. Durum: taslak. Plan satırı: “Otomatik kayıt/ICP sonucunda …
artık, güvenilir kapsam ve dönüşüm tarifi raporlansın.”

İlgili: [ADR 1800](1800-scan-registration-check.md), nokta-düzlem ICP ile tarama kaydı. Bu ADR onun “Sonra”sındaki
“kaba hatalı eşleşmeyi atan ağırlıklar (Huber, kırpılmış ICP)” maddesini kapatır.

## Dayanak

- **MathWorks, `robustfit`** ([belge](https://www.mathworks.com/help/stats/robustfit.html), 8 Kasım 2026'da okundu):
  - Huber ağırlığı w = 1 / max(1, |r|), r artığın ayar sabiti ve ölçekle bölünmüşüdür;
  - Huber'in öntanımlı ayar sabiti 1,345'tir. Yanıt normal dağılıyor ve kaba hata yoksa en küçük karelerin yaklaşık
    %95'i kadar verimlidir;
  - ölçek s = MAD / 0,6745 (1 / 0,6745 = 1,4826).
- **CloudCompare, ICP** ([wiki](https://www.cloudcompare.org/doc/wiki/index.php?title=ICP), 8 Kasım 2026'da okundu):
  “Final overlap” ile kaydedilen bulutun gerçekten örtüşen payı verilir; bulutların onda biri kadar örtüşmesiyle de
  kayıt yapılabilir. Kırpılmış ICP'nin (Chetverikov ve arkadaşları, “The Trimmed Iterative Closest Point
  Algorithm”, ICPR 2002) kullanıcıya açılmış biçimidir.
- **KentOS bugün** (ADR 1800):
  - her eşleşme normal denklemlere aynı ağırlıkla giriyordu;
  - araç, bitki ya da duvar yüzü gibi erişim içinde kalan yanlış eşleşmeler dönüşümü kendine doğru çekiyordu.

## Karar

1. **`robust` girdisi** (`cad.pointclouds.register`, isteğe bağlı). Her ICP adımında eşleşmeler çözümden önce
   ağırlıklanır: N = Σ w a aᵀ, Σ w a r. Ağırlığı 0 olan eşleşme toplama girmez.
   - **`huber`:**
     - |r| ≤ k·s ise ağırlık 1, ötesinde k·s / |r|;
     - s = 1,4826 × ortanca |r| (tek sayıda ortadaki, çift sayıda ortadaki ikisinin ortalaması). KentOS ortancayı
       sıfıra göre alır: oturmuş taramada düzlem artığının beklenen değeri sıfırdır. MathWorks'ün MAD'i ortancaya
       göredir ve kaldıraç düzeltmesi vardır; KentOS bu düzeltmeyi uygulamaz;
     - `k` verilmezse 1,345.
   - **`trimmed`:** en küçük |r|'li ⌈ratio·n⌉ eşleşme 1, kalanı 0 (eşitlikte önceki önce).
     - `ratio` 0'dan büyük, en çok 1 olmalıdır. Kullanıcının örtüşme tahminidir; KentOS ilk değer vermez.
2. **Rapor:** cevapta `robust` vardır: tür, `k` ya da `ratio`, son adımın ölçeği (Huber) ve ağırlığı 1'den küçük eşleşme
   sayısı (`lighter`).
   - RMS'ler, kapsam ve güç tanımlarını korur: bütün eşleşmelerden, ağırlıksız. Güç geometrinin tutuşudur; ağırlıklar
     onu değiştirmez.
3. **Retler** (`invalid_input`):
   - `robust.kind`: huber ya da trimmed değil;
   - `robust.k`: sonlu ve 0'dan büyük değil;
   - `robust.ratio`: kırpılmış ICP'de verilmemiş ya da (0, 1] dışında.
4. **Masaüstü** (Tarama kaydı):
   - **Ağırlık** satırı: Eşit, Huber (k, ilk değer 1,345), Kırpılmış (tutulan pay, ilk değer 1: kullanıcı örtüşmeyi
     söyleyene dek hiçbir şey kırpılmaz);
   - özette ağırlıkların satırı: ölçek ve hafifletilen ya da dışarıda bırakılan eşleşme sayısı.
5. **Kapsam işareti:** ADR 1800'deki gibi **Harita + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/pointcloud_register_cases.py`, KentOS kodu yok; numpy yalnız öz değerler için.
  - Yeni bulut `kayit-e`: B taramasının zemini yeniden örneklenip aynı biçimde kaydırılmış, üstünde zeminden 0,5 m
    yüksek 8 m × 6 m'lik bir yama var (park etmiş araç, alçak bitki). Yama erişim içinde kaba hatalı eşleşmeler verir.
  - Başvurunun kendi denetimi şudur: düz ICP kotta 1 cm'den çok kayar; Huber bunun üçte birinin altında; kırpılmış
    ICP 5 mm'nin altında.
  - Sonuçlar (gerçek kot ötelemesi −6,02 cm):

    | Kayıt | Kot ötelemesi | Ağırlığı birden küçük |
    |---|---|---|
    | düz | −8,45 cm | — |
    | Huber, k 1,345 | −6,11 cm | 542 eşleşme |
    | Huber, k 3 | −6,20 cm | 229 eşleşme |
    | kırpılmış, %90 | −6,03 cm | 255 eşleşme |

    Temiz taramada Huber düz ICP'ye yakın kalır.
  - 40 durum (31'den), dört ret dahil. Eski 31 durumun cevabı bayt bayt aynıdır.
  - Başvuru Homebrew'un `python3`'üyle çalıştırılır: uv'nin numpy tekerleği öz değerlerde 10⁻⁸ farklıdır.
  - Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/register_weights.rs`): durumun çizimi `.kcad` olarak açılır, bulutların dosyaları
  fikstürlerden. Üç kaydın ötelemeleri ve ağırlık raporu durumlarınkidir; paysız kırpılmış ICP reddedilir.
- **Masaüstü** (`register_window_tests.rs`, `the_weights_hold_the_fit_against_gross_mismatches`): Ağırlık satırından düz,
  Huber ve kırpılmış kayıt durumların ötelemelerini ve raporlarını verir. Çapraz masaüstü denetimleri 134 geçer.
  Resimler `.run/gui-evidence/tarama-agirlik-2300/`.

## Sonra

- Dönüşümün buluta uygulanması (yerleşim dönüşümü; `.kcad`, çizici ve bulut araçları birlikte).
- Çoklu tarama demeti, kalıcı kayıt kimliği, gerçek veriyle süre ve bellek (ADR 1800 Sonra).
- Kenar ve örtülme ayıklaması.
