# 3180 — Teslim paketinde projenin kullandığı ızgaralar

Tarih: 2026-11-09. plan/map-cad.md MAP-17, beşinci tur. Durum: taslak. Planın satırı: “Teslim paketi çizim + bağlı
kaynaklar + kullanılan profil/ızgara bilgisi + kayıp/kalite raporu taşısın”; ADR 2780: “Izgara (CRS dönüşüm ızgarası)
bilgisi bu ADR'de yoktur.”

İlgili:
- [ADR 0506](0506-relative-sources-and-package.md), [0558](0558-institution-profile-import-migration-mapping.md),
  [2780](2780-delivery-package-open-files.md): Paketle, profiller, devir dosyaları;
- [ADR 0168](0168-custom-crs-and-datum-transforms.md) §3: datum seçimi ve NTv2 ızgarası (`GridChoice`);
- [ADR 0519](0519-vertical-crs-and-epoch.md), [ADR 0631](0631-geoid-velocity-and-vertical-statements.md): yüksekliklerin geoit modeli, cihazın
  ızgara kütüphanesi (geoit ve hız ızgarası).

## Dayanak

- **PROJ, Resource files** ([belge](https://proj.org/en/stable/resource_files.html), 9 Kasım 2026'da okundu):
  - dönüşüm ızgaraları kaynak dosyalardır; açık yol, arama yolları, kullanıcının yazılabilir klasörü
    (`${XDG_DATA_HOME}/proj`), `PROJ_DATA`, kurulum klasörü ve çalışma klasöründe aranır;
  - ağ açıksa ızgara CDN'den okunur;
  - `only_best_default` açıksa en iyi dönüşümün ızgarası yoksa hata verir; kapalıysa eksik ızgara sessizce kabul edilir.

  Izgara projenin dışında, makinenin kendi klasörlerindedir. Proje başka makineye gidince ızgara da gitmezse aynı
  dönüşüm yeniden üretilemez ya da sessizce başka yoldan yapılır.
- **AutoCAD eTransmit, MicroStation Create Package** (ADR 1111 ve 2780'in okumaları): destek dosyaları pakete girer.
- **KentOS bugün:**
  - proje ızgaraları SHA-256'larıyla adlar: datum seçiminin NTv2 ızgarası (`datumTransforms[i].grid.id`), yüksekliklerin
    geoit modeli (`heights.geoid.sha256`), ölçme kampanyalarının hız ızgarası (`mapSurvey.campaigns[i].velocity`);
  - dosyaların kendisi cihazın ızgara kütüphanesindedir (`KENTOS_GRID_DIR`, yoksa `$XDG_DATA_HOME/kentos-cad/izgara/`:
    `<SHA-256>.gsb`, `.gtx`, `.tif` ve not `<SHA-256>.json`);
  - Paketle profilleri taşıyordu, ızgaraları taşımıyordu.

## Karar

1. **Paketle** (`file.package`) projenin adladığı ızgaraları cihazın kütüphanesinden pakete koyar:
   - `izgaralar/<SHA-256>.<gsb|gtx|tif>` (kütüphanenin adıyla), notu varsa `izgaralar/<SHA-256>.json`;
   - bayt bayt, paketin kopya kuralıyla (SHA-256 geri okunur);
   - aynı ızgarayı iki ayar adlıyorsa bir kez kopyalanır; bağ sayısı (`links`) adlandığı yer sayısıdır, notunki 0;
   - sıra ayarların sırası: datum seçimleri, geoit, kampanyalar.
2. **Dosya türü** `PackagedKind::Grid` (`grid`; sözleşmeye ek tür, komut sürümü değişmez). `paket.json` ve cevabın
   `files`'ı ızgaraları bu türle listeler.
3. **Bırakılanlar:** SHA-256'sı verilmemiş ya da kütüphanede olmayan ızgara pakete girmez:
   - `left`'te `grid`, nedeni `missing`, yolu ızgaranın adı, yeri onu adlayan ayar (`heights.geoid`,
     `mapSurvey.campaigns[0].velocity`, `datumTransforms[0].grid`);
   - var olan `package_left` uyarısı onu söyler (“Hız ızgarası (yerinde yok)”).
4. **Projenin bağları değişmez:** ızgara SHA-256'sıyla adlanır, yolla değil. Alan makine `izgaralar/`'daki dosyaları
   Izgaralar penceresiyle kütüphanesine ekler; adları aynı kalır.
5. **Masaüstü** (Paketle): ileti taşınan ızgara sayısını söyler (“1 ızgara”), notları saymaz.
6. **Kapsam işareti:** Paketle gibi **CAD + CBS** (her modül).

## Kanıt

- **Rust birim testi** (`kentos-headless`, `package::tests::the_grids_the_project_names_come_from_the_library`):
  - geçici kütüphanede TG geoidi (notuyla) ve NTv2 ızgarası;
  - geoit ve NTv2 SHA-256'larıyla kopyalanır; iki kez adlanan geoit iki bağla bir kez; notu bağsız;
  - kütüphanede olmayan hız ızgarası kampanyanın ayarıyla bırakılır;
  - SHA-256'sız geoit `heights.geoid` ile bırakılır, kütüphaneye bakılmaz.
- **Paket testleri** (`tests/headless` `package`, 9 test): ızgara adlamayan eski durumlar aynen geçer.
- **Bağımsız** (`python/tests/test_package_grids.py`, hashlib; KentOS kodu yok): kütüphane `KENTOS_GRID_DIR`'deki
  geçici klasör. `paket.json`'da üç `grid` dosyası doğru bağ sayılarıyla; her birinin boyu ve SHA-256'sı klasördeki
  dosyanınki, adı SHA-256'sı; hız ızgarası `missing` ile bırakılır, uyarı onu söyler. 2 test geçer.
- **MCP** (`crates/native/mcp/tests/package_grids.rs`): örnek çizim `.kcad` olarak açılır ve paketlenir. İki ızgara
  `izgaralar/`'da SHA-256'larıyla, cevap ve `paket.json` aynı; eksik hız ızgarası bırakılır.
- **Masaüstü** (`exchange/package_tests.rs`):
  - ileti iki ızgarayı sayar, notu saymaz;
  - resim `.run/gui-evidence/paket-izgara-3180/`: geoit modeli adlanmış örnek çizimin paketi, “1 ızgara”; yanında
    `paket.json`.

  Paketle testleri ve çapraz denetimler geçer.

## Sonra

- ~~Paketten açılan projede ızgaraların kendiliğinden bulunması: komutlar `izgaralar/`'a da baksın ya da açılışta
  kütüphaneye eklensin.~~ [ADR 3580](3580-package-grids-found-on-opening.md): açılışta yerinde okunur, kitaplık yazılmaz.
- Proje içinde adı olmayan ızgaralar (bir komutun bir kez kullandığı NTv2): kullanım kaydı yoksa paket onları bilemez.
- DXF'te TABLE yazımı; tanımı olmayan ek özniteliklerin ATTRIB'i (MAP-17'nin öbür maddeleri).
- Web (TODOS §0.2).
