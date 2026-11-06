# ADR 1015: “Bu değer nereden geliyor?” baskıdaki mürekkebi de söyler

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12` açığı “plot profilinin baskı rengine
  etkisi” (ADR 1100 §5'in sınırı); ADR 1130 (CTB/STB baskı stili tabloları) bunu ölçülebilir kıldı.
- **Bağlam belgesi:** ADR 1100 §5, `crates/shared/sheet/src/plot_profile/apply.rs` (PDF'in mürekkep kuralı).

## 1. Karar

- `appearance_origin::explain`'in renk ve kalınlık parçaları `printed` satırları kazanır: baskı profili nesnenin
  katmanına bir kalem ya da stil veren (ya da varsayılan stili olan) her pafta için o paftanın bastığı değer
  (`<pafta> · “<profil>” profili: <değer>`), yalnız modelinkinden farklıysa. Kural PDF'inkidir:
  `plot_profile::apply::Resolved::model` ve `apply::color`; kalınlık profilin µm değerinden.
- Model rengi tema jetonuysa (`fg`, `ink`) palet burada çözülmez: profil sabit renk veriyorsa (siyah ya da renk) o
  renk, gri ton veriyorsa “gri tonda” söylenir; nesnenin rengini koruyan profil satır üretmez.
- Masaüstü Öznitelikler “Bu değer nereden geliyor?” bölümünde **Baskıda:** satırı. Baskı mürekkebi türetilmiş veridir;
  model değişmez.

## 2. Kanıt

`office_profiles::the_value_origin_says_what_the_sheets_plot_profile_prints`: bağımsız başvurunun `siyah.ctb`'si
(ADR 1130) içe alınır, kalem 1 Duvar katmanına verilip a-101 paftasına uygulanır; açıklama a-101'de siyah ve 0,5 mm
basıldığını söyler, profil yokken söylemez. Resim `deger-kokeni-baski`.

## 3. Sınırlar

Kâğıt öğesine (paper item) verilen stil ve görünüm penceresinin kendi ezmesi üstüne profilin bileşimi bu satırda
ayrı söylenmez; tema jetonunun kâğıttaki karşılığı çözülmez.
