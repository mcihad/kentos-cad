# İfade motoru, native: eski (2026-09-27, 23cc8f7)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release`. 20 koşu (3 ısınma), p50 / p95 ms.

Test `crates/shared/expression/tests/perf.rs`: web'in ölçümündeki nesneler (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa, 0…1000 m² alan) tarayıcının gönderdiği tabloda (`rows`); ifade bir sütuna değerlendirilir, bütün değerler okunur. Tablonun kurulması dahil değildir.

| İfade | Biçim | 100000 nesne | 1000000 nesne |
|---|---|---|---|
| `Nitelik = 'Arsa' ve $alan > 500` | Bool | 7.30 / 7.71 | 76.76 / 85.35 |
| `'P' \|\| doldur($sıra, 5)` | Text | 12.44 / 12.91 | 114.63 / 133.48 |
| `metin($alan, 2) \|\| ' m²'` | Text | 11.38 / 12.61 | 109.12 / 114.31 |
| `yuvarla($alan, 2)` | Number | 5.87 / 6.12 | 55.36 / 58.58 |
| `$alan / 10000 > 0.05 ve $uzunluk < 400` | Bool | 7.52 / 8.31 | 73.01 / 76.31 |
| `Parsel * 2 + 1` | Number | 5.71 / 6.34 | 57.69 / 61.03 |
