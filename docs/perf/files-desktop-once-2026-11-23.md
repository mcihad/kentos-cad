# Gerçek dosyalar, taban ölçümü (RM-01, 23 Kasım 2026)

**Ortam:**

- Makine: Intel Core i5-13500, Intel UHD Graphics 770, Mesa 26.0.8, Vulkan.
- Çizim: pencere açılmadan, gerçek GPU'da.
- Derleme: `--release` (lto thin).
- Pencere: 1440×900. Sahibin ayarları geçerliydi: gerçek boyutlu yazılar, çizgi kalınlıkları açık, MSAA 1.

**Ölçüm:** `apps/desktop/src/shell/perf/files.rs` (plan/render.md RM-01, [ADR 9201](../adr/9201-render-measurement.md)). Ham
sonuç: `files-desktop-once-2026-11-23.json`.

**Sütunların anlamı:**

- UI: arayüz iş parçacığının payı.
- Görünüm: UI içinde `App::view`'ın payı.
- GPU: kare çizilip geri okunana kadar geçen süre.
- Süreler milisaniye, p50 / p95.

| Dosya | Nesne | Açılış | Tekerlek UI | Tekerlek görünüm (p95) | Tekerlek GPU | Kaydırma GPU | İmleç UI | Seçim UI (görünüm) | En yüksek bellek |
|---|---|---|---|---|---|---|---|---|---|
| sivas.kcad | 999 975 | 4,7 s | 95 / 103 | 99,5 | 108 / 116 | 13 / 15 | 0,9 / 1,2 | 4 (0) | 2 752 MB |
| SIVAS_UIP_2025_06_HAZIRAN.NCZ | 453 534 | 3,7 s | 71 / 180 | 115,9 | 38 / 154 | 31 / 32 | 0,9 / 0,9 | 1 (0) | 2 561 MB |
| SİVAS AVM_2015.06.11.dwg | 601 619 | 23,6 s | 42 / 43 | 37,5 | 22 / 24 | 12 / 13 | 2,0 / 2,0 | 106 (65) | 3 715 MB |
| deneme_suşehri.dxf | 71 820 | 1,6 s | 24 / 24 | 16,9 | 50 / 53 | 13 / 15 | 0,8 / 0,9 | 8 (5) | 636 MB |

**Tekerlek kareleri bütçeyi aşıyor.** 60 Hz'de bir karenin bütçesi 16,7 ms.

| Dosya | Bütçeyi aşan |
|---|---|
| sivas.kcad | 32 karenin 16'sı |
| NCZ | 32 karenin 32'si |
| DWG | 32 karenin 16'sı |
| DXF | 32 karenin 16'sı |

**Hızlı el:** on dört çentik tek karede geliyor; ölçülen, bu karenin süresi ve resmin son değiştiği an.

| Dosya | İçeri | Dışarı |
|---|---|---|
| sivas.kcad | 46 + 41 ms; son değişim 98 ms | 97 + 118 ms; 225 ms |
| NCZ | 153 + 44 ms; 208 ms | 233 + 144 ms; 397 ms |
| DWG | 53 + 22 ms; 86 ms | 50 + 29 ms; 89 ms |
| DXF | 24 + 24 ms; 59 ms | 25 + 57 ms; 93 ms |

**“Tümünü göster”in kutusu:**

| Dosya | Açılışta sığdırılan kutu | “Tümünü göster”in kutusu | 0,0 içinde mi |
|---|---|---|---|
| sivas.kcad | 567 658 … 603 072 × 4 391 845 … 4 412 597 | aynı | hayır |
| NCZ | 567 605 … 603 484 × 4 391 592 … 4 412 617 | −4 104 135 … 4 702 431 × −374 748 … 4 785 838 | evet |
| DWG | −242 … 350 × −2 073 … −1 727 | −3 515 … 3 841 × −4 023 … 288 | evet |
| DXF | 416 … 428 × 4 446 … 4 453 | aynı | hayır |

**Gerçek pencerede** (Xvfb, lavapipe, hata ayıklama derlemesi; `.run/gui-evidence/render/rm01/ncz-pencere`) büyük NCZ'de
14 çentik 30 ms arayla:

- **İçeri:** ilk değişim ilk çentikten 1 074 ms sonra geldi, son değişim son çentikten 1 704 ms sonra. Arada yalnız 3
  ayrı resim gösterildi.
- **Dışarı:** ilk değişim 260 ms sonra, son değişim 2 574 ms sonra; 4 resim.
