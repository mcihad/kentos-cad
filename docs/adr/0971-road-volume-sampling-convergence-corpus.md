# 0971 — Kübajın km aralığına yakınsaması: bağımsız korpus

Tarih: 2026-11-05. Durum: kabul (Y66). Kapsam ROAD-QA-03.

## Sorun

ROAD-QA-03: "Known prism/ramp/mixed-sign volumes ve farklı station sampling convergence korpusunu oluştur."

- **Var olan:** Bilinen hacimler `fixtures/road/v1/volume.json`'da vardı (ADR 0906): düz prizma (2 m dolgu, 1,5 m kazı),
  eğik düzlem ve işaret değiştiren kontrol örneği (d = x − 5, 125 m³ kazı, 125 m³ dolgu).
- **Eksik:** Km aralığı küçüldükçe kesitlerden bulunan hacmin gerçek hacme nasıl yaklaştığı hiçbir yerde sınanmıyordu.
  Ortalama alan ile prizmoidal formülün hangi hızla yaklaştığı bilinmiyordu.

## Araştırma

Kesitten hacim, alanın km boyunca integralinin sayısal kuralıdır:

- **Ortalama alan:** L·(A₀ + A₁)/2, yamuk kuralıdır. Hatası −(b − a)·h²·A″(ξ)/12'dir; alan düzgünse h²'yle küçülür.
- **Prizmoidal:** L·(A₀ + 4A_m + A₁)/6, Simpson kuralıdır. Hatası h⁴'le küçülür; üçüncü dereceye kadar polinomda
  tamdır.
- **Kırık:** Alanın türevi aralığın içinde sıçrarsa iki kural da h²'ye düşer. Örnek: dolgunun bittiği yer bir km'ye
  denk gelmez; A_dolgu'nun eğimi W·H′'den 0'a atlar.

Bunlar sayısal analizin bilinen sonuçlarıdır (ör. Burden ve Faires, *Numerical Analysis*, §4.3–4.4). Yol ders
kitaplarında prizmoidal düzeltme aynı sonuçtan çıkar.

## Karar

`scripts/fixtures/road_volume_convergence_cases.py` KentOS kodu olmadan çalışır.

**Kurulum:**

- yol düz, zemin 100 m'de düz;
- platform yatay, W = 7 m;
- dolgu şevi 2, yarma şevi 1,5.

**Alan ve tam hacim:**

- Kırmızı kot zeminin H üstündeyse dolgu alanı H·(W + 2H), altındaysa yarma alanı |H|·(W + 1,5|H|)'dir.
- Kırmızı kot ders kitabının eğim ve simetrik parabollarıdır; kesirlerle hesaplanır.
- Alan, km'nin parçalı polinomudur. Tam hacim bu polinomun integralidir. Parçalar kurp uçlarından ve H'nin işaret
  değiştirdiği yerden (ikinci derece kökü, mpmath 50 basamak) bölünür.

**Toplamlar:** Her h için (40, 20, 10, 5, 2,5, 1,25 m) ortalama alan ve prizmoidal toplamları ile görülen mertebeler,
log₂(e_h/e_{h/2}), yazılır. Referans mertebeleri kendisi sınar.

| Durum | Tam hacim | Ortalama alan mertebesi | Prizmoidal |
|---|---|---|---|
| Dolguda tepe kurbu (L 160) | dolgu 26 159,627 m³ | 2,0045 → 2,00002 | 4,0000 |
| Rampa zemini km 80'de keser | kazı 720, dolgu 10 800 m³ | tam 2 | **tam** (hata 0): alan ikinci derece, kırık km'de |
| Çukur kurbunda yarma–dolgu, kırık km'ler arasında | kazı 3 090,744, dolgu 3 319,841 m³ | 1,78–2,31 | 0,42–3,00; ortalama ≈ 2 |

Rampanın tam hacmi elle de bulunur: kazı ∫₀² u(7 + 1,5u) du / 0,025 = 720 m³, dolgu ∫₀⁶ u(7 + 2u) du / 0,025 = 10 800
m³.

`crates/shared/road/tests/all/volume_convergence.rs`, KentOS'un kendi parçalarıyla her h'de iki toplamı kurar:
`Vertical`, düz zeminde `template::build`, `volume::end_areas` ve `volume::prismoidal`. Toplamlar referansınkiyle 10⁻⁹
bağıl içinde aynıdır. Yakınsamanın kendisini referans sınar.

**Kullanıcıya düşen:** Üçüncü durum, ADR 0939'un uyarlamalı örneklemesinin neden yarma–dolgu geçişini ayrıca aradığını
gösterir. Kırık iki km arasında kalınca prizmoidal formül ortalama alandan iyi değildir. Doğruluk km aralığını
küçültmekten değil, kırığı bir km yapmaktan gelir.

## Ölçüm

- Referans `--check` ile günceldir.
- Rust testi geçer.
- Komut ve pencere değişmedi; bu bir doğrulama korpusudur, ekranda görünen bir şey yoktur.

## Açık

- **Kurpta yakınsama:** Eğrilik düzeltmesi (ADR 0934) ile kurpta yakınsama bu korpusta değil.
  `curve-volumes.json` yalnız sabit kesitli dönel taramayı sınar.
