# İfade motoru, web (WASM yolu): tipli (2026-09-27, cdbb36f)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Linux 7.0.0-34-generic; Node v24.16.0, WASM `--profile wasm`. 10 koşu (3 ısınma), p50 / p95 ms.

Test `apps/web/scripts/perf/expression.test.ts`: nokta nesneleri (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa), deponun ölçü yanıtında 0…1000 m² alan. Süre sayfanın tabloyu kurmasını (`exprTable`), çekirdeğin değerlendirmesini (`exprEvaluate`) ve bütün değerlerin okunmasını kapsar.

| İfade | Biçim | 100000 nesne | 1000000 nesne |
|---|---|---|---|
| `Nitelik = 'Arsa' ve $alan > 500` | bool | 9.4 / 10.2 | 86.4 / 105.7 |
| `'P' \|\| doldur($sıra, 5)` | text | 13.8 / 16.6 | 142.3 / 251.9 |
| `metin($alan, 2) \|\| ' m²'` | text | 17.8 / 18.9 | 172.4 / 274.6 |
| `yuvarla($alan, 2)` | number | 3.5 / 4.3 | 30.7 / 38.8 |
| `$alan / 10000 > 0.05 ve $uzunluk < 400` | bool | 4.2 / 4.5 | 38.5 / 40.2 |
| `Parsel * 2 + 1` | number | 10.4 / 11.7 | 145.1 / 255.1 |

Geometri deposundaki kareler (10 × 10 … 13 × 13 m; ADR 0100 §3), p50 ms: “depoda” ifade deponun içinde değerlendirilir ve geometri değerlerini şekillerden okur (`CoreStore.evaluateExpression`); “ölçü kaydıyla” deponun `measures` yanıtı alınır ve `exprEvaluate`'e verilir (sayfanın bugünkü yolu, iki kopya dahil).

| İfade | Yol | 100000 nesne |
|---|---|---|
| `$alan > 500` | depoda | 19.2 |
| `$alan > 500` | ölçü kaydıyla | 28.1 |
| `yuvarla($alan, 2)` | depoda | 17.3 |
| `yuvarla($alan, 2)` | ölçü kaydıyla | 24.9 |
| `$merkez_y` | depoda | 19.8 |
| `$genişlik * $yükseklik` | depoda | 21.7 |
