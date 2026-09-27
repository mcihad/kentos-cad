# İfade motoru, web (WASM yolu): eski (2026-09-27, 23cc8f7)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Linux 7.0.0-34-generic; Node v24.16.0, WASM `--profile wasm`. 10 koşu (3 ısınma), p50 / p95 ms.

Test `apps/web/scripts/perf/expression.test.ts`: nokta nesneleri (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa), deponun ölçü yanıtında 0…1000 m² alan. Süre sayfanın tabloyu kurmasını (`exprTable`), çekirdeğin değerlendirmesini (`exprEvaluate`) ve bütün değerlerin okunmasını kapsar.

| İfade | Biçim | 100000 nesne | 1000000 nesne |
|---|---|---|---|
| `Nitelik = 'Arsa' ve $alan > 500` | bool | 16.9 / 18.2 | 169.5 / 179.2 |
| `'P' \|\| doldur($sıra, 5)` | text | 18.0 / 18.6 | 179.7 / 227.0 |
| `metin($alan, 2) \|\| ' m²'` | text | 19.9 / 20.8 | 196.2 / 295.9 |
| `yuvarla($alan, 2)` | number | 7.2 / 7.8 | 71.5 / 77.4 |
| `$alan / 10000 > 0.05 ve $uzunluk < 400` | bool | 12.0 / 12.5 | 116.7 / 127.0 |
| `Parsel * 2 + 1` | number | 13.8 / 15.4 | 176.8 / 240.0 |
