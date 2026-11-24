# Canlı kamera, önce ve sonra (RM-05, 23 Kasım 2026)

**Ortam:** Intel Core i5-13500, UHD Graphics 770 (Mesa 26.0.8, Vulkan), `--release`, pencere 1440×900. Ölçüm
`perf::files` ([ADR 9201](../adr/9201-render-measurement.md)); donanım 5a5d99dc9'dan beri görünümü çalışma zamanı gibi
yalnız bir iletiden sonra kurar.

**Karşılaştırılan sürümler:** önce 5a5d99dc9, sonra canlı kamera ([ADR 9205](../adr/9205-live-camera-in-the-canvas.md)).
Ham sonuçlar: `files-desktop-rm05-{once,sonra}-2026-11-23.json`.

**Sütunların anlamı:**

- Tekerlek ve kaydırma: kare başına UI (arayüz iş parçacığı) süresi, p50, milisaniye. Hücre “önce → sonra”.
- Görünüm: bu karelerin kaçında `App::view` kuruldu.
- GPU: kare başına GPU süresi, p50, milisaniye.

| Dosya | Tekerlek UI | Tekerlek görünüm | Kaydırma UI | Kaydırma görünüm | Tekerlek GPU |
|---|---|---|---|---|---|
| sivas.kcad (1 M) | 92,8 → **0,2** | 32 → **1** | 1,4 → **0,2** | 40 → **0** | 108 → 105 |
| İmar planı NCZ (453 534) | 64,9 → **0,2** | 32 → **1** | 0,8 → **0,2** | 40 → **0** | 92 → 87 |
| SİVAS AVM DWG (601 619) | 40,8 → **0,2** | 32 → **1** | 2,0 → **0,2** | 40 → **0** | 22 → 22 |
| Suşehri DXF (71 820) | 23,6 → **0,2** | 32 → **1** | 0,8 → **0,2** | 40 → **0** | 50 → 50 |

**Bütçeyi aşan tekerlek kareleri** (60 Hz'de 16,7 ms):

| Dosya | Önce | Sonra |
|---|---|---|
| sivas.kcad | 16 / 32 | **0** |
| NCZ | 32 / 32 | **1** |
| DWG | 16 / 32 | **0** |
| DXF | 16 / 32 | **0** |

**Hızlı el** (on dört çentik tek karede):

| Dosya | Patlama karesi UI | Resmin son değişimi |
|---|---|---|
| sivas.kcad | 45–96 → **0** | 224–251 → 329–546 ms |
| NCZ | 178–196 → **0** | 238–313 → 565–591 ms |
| DWG | 49–53 → **0** | 84–89 → 294–296 ms |
| DXF | 24–25 → **0** | 59–95 → 296–338 ms |

Resmin son hâline varış uzadı. Ağır iş (yazıların yerleşimi, eğrilerin bandı) artık görünüm 150 ms durunca, bir kez
yapılıyor; o kare 25–193 ms UI tutuyor. Bu işi arayüz iş parçacığından çıkarmak RM-07 ve RM-08..10'un işi.

**GPU değişmedi:** sahne aynı, yalnız kamerası canlı. Uzak görünümde sivas.kcad'in 105 ms'si ve NCZ'nin 87 ms'si
LOD'un işi (F3).
