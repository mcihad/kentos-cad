# KCAD v2 masaüstünde kaydet ve aç: before (2026-09-26, aa567ea)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release` (lto thin). 3 koşu, her hücre ortanca. Test `apps/desktop/src/perf.rs`: her işlem kendi sürecinde çalışır, bellek o sürecin en yüksek yerleşik belleğinin (VmHWM) işlemden önceki düzeyin üstündeki payıdır. Dosyalar `.run/perf`'e yazılır (yerel disk, `fsync` dahil).

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`crates/shared/kcad/tests/measure.rs` ile aynı). “Anlık görüntü” kaydın arayüz iş parçacığındaki payıdır (`to_snapshot_v2`); “yazma” doğrulamalı kodlama, geçici dosya, `fsync`, geri okuma ve yer değiştirmedir (`document::write`). “Açma” dosyanın okunması, çözülmesi ve belgenin kurulmasıdır (`Document::read`); “sahne” ilk karenin arayüz iş parçacığında kurulan sahnesidir (çizgiler ve eğriler, bütün çizim görünürken).

| Parsel | Dosya | Anlık görüntü | Yazma | Kayıtta bellek artışı | Açma | Açışta bellek artışı | Sahne | Açık çizimle süreç |
|---|---|---|---|---|---|---|---|---|
| 25000 | 12.0 MB | 21 ms | 181 ms | 80 MB | 78 ms | 57 MB | 25 ms | 43 MB |
| 50000 | 24.1 MB | 41 ms | 350 ms | 168 MB | 155 ms | 112 MB | 54 ms | 81 MB |
| 100000 | 48.3 MB | 86 ms | 693 ms | 338 MB | 308 ms | 224 MB | 107 ms | 155 MB |
| 200000 | 96.6 MB | 176 ms | 1418 ms | 641 MB | 635 ms | 448 MB | 212 ms | 304 MB |
