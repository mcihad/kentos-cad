# İfade motoru, native: yeni (2026-09-27, 4d2060a)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release`. 20 koşu (3 ısınma), p50 / p95 ms.

Test `crates/shared/expression/tests/perf.rs`: web'in ölçümündeki nesneler (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa, 0…1000 m² alan) tarayıcının gönderdiği tabloda (`rows`); ifade bir sütuna değerlendirilir, bütün değerler okunur. Tablonun kurulması dahil değildir. “Eski” aynı yapıdaki ağaç değerlendiricisidir (`tests/reference`, 23cc8f7'nin kodu), “yeni” sütun motoru; ikisi aynı tabloda aynı sütunu verir.

| İfade | Biçim | 100000 nesne: eski / yeni (p50) | kat | 1000000 nesne: eski / yeni (p50) | kat |
|---|---|---|---|---|---|
| `Nitelik = 'Arsa' ve $alan > 500` | Bool | 7.19 / 1.47 | 4.9× | 72.38 / 15.07 | 4.8× |
| `'P' \|\| doldur($sıra, 5)` | Text | 10.89 / 7.10 | 1.5× | 111.31 / 74.31 | 1.5× |
| `metin($alan, 2) \|\| ' m²'` | Text | 11.00 / 8.33 | 1.3× | 112.74 / 85.42 | 1.3× |
| `yuvarla($alan, 2)` | Number | 5.50 / 0.89 | 6.2× | 55.52 / 9.86 | 5.6× |
| `$alan / 10000 > 0.05 ve $uzunluk < 400` | Bool | 7.43 / 1.07 | 7.0× | 73.54 / 10.56 | 7.0× |
| `Parsel * 2 + 1` | Number | 5.73 / 3.04 | 1.9× | 62.82 / 33.19 | 1.9× |

Tek tek (`Expr::evaluate`, bir kapsamdan; stil motorunun bir sembolü çizerken yaptığı gibi), 100000 nesne, p50 ms:

| İfade | Eski | Yeni | Kat |
|---|---|---|---|
| `Nitelik = 'Arsa' ve $alan > 500` | 6.10 | 6.10 | 1.0× |
| `'P' \|\| doldur($sıra, 5)` | 10.08 | 9.76 | 1.0× |
| `metin($alan, 2) \|\| ' m²'` | 9.76 | 9.40 | 1.0× |
| `yuvarla($alan, 2)` | 4.90 | 4.85 | 1.0× |
| `$alan / 10000 > 0.05 ve $uzunluk < 400` | 6.80 | 4.88 | 1.4× |
| `Parsel * 2 + 1` | 4.38 | 4.11 | 1.1× |

p95 (eski / yeni):

- `Nitelik = 'Arsa' ve $alan > 500`: 100000 nesnede 7.40 / 1.58 ms; 1000000 nesnede 97.02 / 31.07 ms;
- `'P' \|\| doldur($sıra, 5)`: 100000 nesnede 15.19 / 10.85 ms; 1000000 nesnede 114.50 / 77.96 ms;
- `metin($alan, 2) \|\| ' m²'`: 100000 nesnede 11.85 / 9.40 ms; 1000000 nesnede 119.86 / 92.02 ms;
- `yuvarla($alan, 2)`: 100000 nesnede 5.82 / 1.05 ms; 1000000 nesnede 59.91 / 11.72 ms;
- `$alan / 10000 > 0.05 ve $uzunluk < 400`: 100000 nesnede 9.75 / 2.20 ms; 1000000 nesnede 80.82 / 14.96 ms;
- `Parsel * 2 + 1`: 100000 nesnede 5.94 / 4.60 ms; 1000000 nesnede 67.68 / 35.22 ms;
