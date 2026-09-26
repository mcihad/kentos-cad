# Masaüstü: bulut projesinin yerel kopyası, ölçüm (26 Eylül 2026)

- **Ne:** ADR 0043'ün yerel kopyası (`crates/native/cloud`, `Replica`); ölçüm `crates/native/cloud/tests/replica_perf.rs`.
- **Ortam:** Intel Core i5-11300H, Linux, ext4 NVMe (`KENTOS_PERF_DIR=.run`; `/tmp` bellekte olduğundan kullanılmadı), sürüm derlemesi (`--release`), commit `8570cf9` üstü.
- **Çizim:** 20 köşeli, üç öznitelikli, etiketli parseller (masaüstünün kayıt/açılış ölçümüyle aynı).

| Parsel | Kopyala (reset) | 100 adım (append, her biri diske işlenir) | Aç (load) | Topla (compact) | Kopya boyu |
|---:|---:|---:|---:|---:|---:|
| 10 000 | 68 ms | 131 ms | 42 ms | 64 ms | 5,2 MB |
| 100 000 | 584 ms | 145 ms | 392 ms | 609 ms | 52,6 MB |

- **Adım:** bir adım (10 nesne) yaklaşık 1,4 ms'de diske işlenir; süre projenin büyüklüğünden bağımsızdır.
- **Açılış:** bağlantısız açılış 100 bin parselde yaklaşık 0,4 s.
- **Kopyala ve topla:** doğrulanmış KCAD v2'yi ve sürümleri yazıp diske işler. Arayüz iş parçacığının dışında yapılır.
- Aynı ölçüm bellekteki `/tmp`'de adımları 2–3 ms'de gösterir. Diske işleme orada bedava olduğundan o sayılar kullanılmadı.

Çalıştırmak için:

```text
KENTOS_PERF_DIR=$PWD/.run cargo test --release -p kentos-cloud --test replica_perf -- --ignored --nocapture
```
