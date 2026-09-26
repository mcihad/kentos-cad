# KCAD v2 masaüstünde kaydet ve aç: after (2026-09-26, 35caac8)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release` (lto thin). 3 koşu, her hücre ortanca. Test `apps/desktop/src/perf.rs`: her işlem kendi sürecinde çalışır, bellek o sürecin en yüksek yerleşik belleğinin (VmHWM) işlemden önceki düzeyin üstündeki payıdır. Dosyalar `.run/perf`'e yazılır (yerel disk, `fsync` dahil).

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`crates/shared/kcad/tests/measure.rs` ile aynı). “Arayüzde” kaydın arayüz iş parçacığındaki payıdır (önce `to_snapshot_v2`, ADR 0030'dan beri nesneleri paylaşan kopya `Document::clone`); “yazma” kaydın kendi iş parçacığındaki payıdır: anlık görüntü (ADR 0030'dan beri), doğrulamalı kodlama, geçici dosya, `fsync`, geri okuma ve yer değiştirme (`document::write`). “Açma” dosyanın okunması, çözülmesi ve belgenin kurulmasıdır (`Document::read`); “sahne” ilk karenin arayüz iş parçacığında kurulan sahnesidir (çizgiler ve eğriler, bütün çizim görünürken).

| Parsel | Dosya | Arayüzde | Yazma | Kayıtta bellek artışı | Açma | Açışta bellek artışı | Sahne | Açık çizimle süreç |
|---|---|---|---|---|---|---|---|---|
| 25000 | 12.0 MB | 2 ms | 131 ms | 80 MB | 76 ms | 46 MB | 26 ms | 44 MB |
| 50000 | 24.1 MB | 4 ms | 257 ms | 160 MB | 154 ms | 92 MB | 57 ms | 81 MB |
| 100000 | 48.3 MB | 9 ms | 524 ms | 319 MB | 312 ms | 184 MB | 113 ms | 155 MB |
| 200000 | 96.6 MB | 18 ms | 1049 ms | 637 MB | 627 ms | 367 MB | 220 ms | 305 MB |
