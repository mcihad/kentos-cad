# İfade motoru, web (WASM yolu): yeni (2026-09-27, 4d2060a)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Linux 7.0.0-34-generic; Node v24.16.0, WASM `--profile wasm`. 10 koşu (3 ısınma), p50 / p95 ms.

Test `apps/web/scripts/perf/expression.test.ts`: nokta nesneleri (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa), deponun ölçü yanıtında 0…1000 m² alan. Süre sayfanın tabloyu kurmasını (`exprTable`), çekirdeğin değerlendirmesini (`exprEvaluate`) ve bütün değerlerin okunmasını kapsar.

| İfade | Biçim | 100000 nesne | 1000000 nesne |
|---|---|---|---|
| `Nitelik = 'Arsa' ve $alan > 500` | bool | 9.0 / 10.9 | 84.5 / 94.4 |
| `'P' \|\| doldur($sıra, 5)` | text | 13.8 / 19.1 | 135.3 / 238.9 |
| `metin($alan, 2) \|\| ' m²'` | text | 17.1 / 18.7 | 179.3 / 280.8 |
| `yuvarla($alan, 2)` | number | 3.2 / 3.7 | 31.4 / 34.0 |
| `$alan / 10000 > 0.05 ve $uzunluk < 400` | bool | 3.8 / 4.1 | 39.5 / 40.8 |
| `Parsel * 2 + 1` | number | 10.2 / 11.8 | 151.3 / 231.3 |
