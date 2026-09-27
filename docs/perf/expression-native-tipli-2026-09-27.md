# İfade motoru, native: tipli (2026-09-27, cdbb36f)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release`. 20 koşu (3 ısınma), p50 / p95 ms.

Test `crates/shared/expression/tests/perf.rs`: web'in ölçümündeki nesneler (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa, 0…1000 m² alan) tarayıcının gönderdiği tabloda (`rows`); ifade bir sütuna değerlendirilir, bütün değerler okunur. Tablonun kurulması dahil değildir. “Eski” aynı yapıdaki ağaç değerlendiricisidir (`tests/reference`, 23cc8f7'nin kodu), “yeni” sütun motoru; ikisi aynı tabloda aynı sütunu verir.

| İfade | Biçim | 100000 nesne: eski / yeni (p50) | kat | 1000000 nesne: eski / yeni (p50) | kat |
|---|---|---|---|---|---|
| `Nitelik = 'Arsa' ve $alan > 500` | Bool | 7.45 / 1.49 | 5.0× | 74.31 / 14.95 | 5.0× |
| `'P' \|\| doldur($sıra, 5)` | Text | 10.82 / 7.10 | 1.5× | 112.22 / 74.11 | 1.5× |
| `metin($alan, 2) \|\| ' m²'` | Text | 10.92 / 8.34 | 1.3× | 114.61 / 86.92 | 1.3× |
| `yuvarla($alan, 2)` | Number | 5.42 / 0.89 | 6.1× | 54.21 / 9.64 | 5.6× |
| `$alan / 10000 > 0.05 ve $uzunluk < 400` | Bool | 7.40 / 0.99 | 7.5× | 74.10 / 10.51 | 7.1× |
| `Parsel * 2 + 1` | Number | 5.54 / 2.97 | 1.9× | 61.59 / 33.84 | 1.8× |

Tek tek (`Expr::evaluate`, bir kapsamdan; stil motorunun bir sembolü çizerken yaptığı gibi), 100000 nesne, p50 ms:

| İfade | Eski | Yeni | Kat |
|---|---|---|---|
| `Nitelik = 'Arsa' ve $alan > 500` | 6.01 | 6.06 | 1.0× |
| `'P' \|\| doldur($sıra, 5)` | 10.08 | 10.03 | 1.0× |
| `metin($alan, 2) \|\| ' m²'` | 9.96 | 9.48 | 1.1× |
| `yuvarla($alan, 2)` | 4.96 | 4.81 | 1.0× |
| `$alan / 10000 > 0.05 ve $uzunluk < 400` | 7.01 | 4.86 | 1.4× |
| `Parsel * 2 + 1` | 4.39 | 4.48 | 1.0× |

Tipli alan ve şekillerden geometri (ADR 0100 §3; yeni motor, p50 ms). `Kat` bir kez sayı sütunu (kullanıcı alanı), bir kez metin özniteliği; `$alan` bir kez şekillerden (`geometry::Shapes`, hesap dahil), bir kez deponun ölçü kaydıyla (kaydın hesabı dahil). Nesneler 10 × 10 … 13 × 13 m kareler.

| İfade | Yol | 100000 nesne | 1000000 nesne |
|---|---|---|---|
| `Kat * 2 + 1` | sayı alanı | 0.37 | 4.26 |
| `Kat * 2 + 1` | metin özniteliği | 2.15 | 22.17 |
| `$alan > 500` | şekillerden | 1.62 | 16.81 |
| `$alan > 500` | ölçü kaydıyla | 4.98 | 62.75 |
| `yuvarla($alan, 2)` | şekillerden | 2.04 | 21.01 |
| `yuvarla($alan, 2)` | ölçü kaydıyla | 5.33 | 66.52 |
| `$merkez_y` | şekillerden | 2.87 | 29.10 |
| `$genişlik * $yükseklik` | şekillerden | 2.94 | 29.63 |

p95 (eski / yeni):

- `Nitelik = 'Arsa' ve $alan > 500`: 100000 nesnede 7.81 / 1.61 ms; 1000000 nesnede 77.72 / 23.20 ms;
- `'P' \|\| doldur($sıra, 5)`: 100000 nesnede 11.48 / 7.67 ms; 1000000 nesnede 119.32 / 75.63 ms;
- `metin($alan, 2) \|\| ' m²'`: 100000 nesnede 11.81 / 8.88 ms; 1000000 nesnede 116.22 / 96.05 ms;
- `yuvarla($alan, 2)`: 100000 nesnede 5.71 / 1.13 ms; 1000000 nesnede 57.03 / 10.33 ms;
- `$alan / 10000 > 0.05 ve $uzunluk < 400`: 100000 nesnede 8.27 / 1.20 ms; 1000000 nesnede 76.35 / 11.92 ms;
- `Parsel * 2 + 1`: 100000 nesnede 5.68 / 3.14 ms; 1000000 nesnede 63.30 / 36.29 ms;
