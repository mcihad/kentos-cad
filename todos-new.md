# Yeni yapılacaklar — inceleme: düzeltilecekler ve eksikler

24 Kasım 2026, `main` `0d7ebec02`. Sahibin isteği: “Projeyi bir incele ve düzeltilmesi gereken ve yapılması gerekenler
varsa todos-new.md içerisine yaz”, “özellikle eksikleri çıkar”, “hem cad hem harita cad hem de mimari cad açısından, yol
zaten daha çok eksik”.

**Nasıl bakıldı.**

- Komut kataloğu (dondurulmuş envanter ve ekler: 1 340 komut, araç ve işlem), dört modülün şeridi ve bağlamsal sekmeler.
- CAD manifestosu (`docs/cad/manifest.md`) ve planlar: `plan/cad.md`, `plan/map-cad.md`, `plan/architecture-advanced.md`,
  `plan/bim-plan.md`, `plan/road.md`.
- Bütün üreteçlerin `--check`'i: 825 betik koşturuldu; farkları depodan ayrı bir kopyada ölçüldü.
- Kesin kuralların taraması: başka uygulama adı, arayüzde ADR atfı, dosya boyu; dallar, zulalar, kılavuz ve betikler.

**Öncelik:**

- **P0:** doğrulamayı ya da çalışma düzenini kıran.
- **P1:** kullanıcının günlük işini eksik bırakan.
- **P2:** bakım ve temizlik.

Bu incelemede düzeltilenler ayrıca yazılı (bölüm H); doğrulanmayanlar bölüm G'de.

---

## A. Düzeltilmesi gerekenler (doğrulandı)

### A1 — P0 — Üreteçlerin `--check`'i kırmızı

825 betikten 506'sı yeşil, **40'ı kırmızı**. 258'i bu makinede koşamıyor:

- 156'sı başında yazılı bir paketi bulamıyor: mpmath 91, pyproj 29, shapely 24, ifcopenshell 13, scipy 6, ezdxf 6,
  sympy 1 betikte (13 betikte birden çok paket eksik);
- 102'si de çalışırken import hatası veriyor.

21'i `--check`'i başka biçimde alıyor, koşturulmadı.

Kırmızıların sınıfları (fark, depodan ayrı bir kopyada betik yazdırılarak ölçüldü):

- **Python sürümüyle değişen son basamak.** Aynı hesap, farklı yuvarlama (3.14 → 3.15; örnek
  `0.41421356237309503` → `0.4142135623730951`):
  - `wall_cases`, `arch_cases`, `dimension_cases`, `leader_cases`, `quick_dimension_cases`, `topology_cases`;
  - `block_scale_cases`, `geometry_generator_cases`, `plan_road_cases`, `arch_bond_cases`, `wall_shift_cases`;
  - `road_sampling_cases`, `road_haul_fleet_cases`, `road_volume_uncertainty_cases`,
    `road_terrain_compare_section_cases`;
  - `raster_georef_cases` (468 satır), `v1_identity_reference` (yalnız sürüm dizesi).
- **Gerçekten eskimiş örnekler.** Üreteç bugün daha çok durum yazıyor; Rust sınamaları bu durumları hiç görmüyor:
  - `arch_bond_conflict_cases`: üç dosyada +11 000 satır (kolon katmanı, akslar…);
  - `wall_command_cases`: `cad.walls.joins.json` +1 210 satır (duvar stili bileşenleri);
  - `field_package_cases`: +1 359 satır;
  - öbürleri: `tag_cases`, `text_along_cases`, `text_command_cases`, `share_cases`, `temporal_command_cases`,
    `layer_service_command_cases`, `service_mvt_cases`, `dxf_hatch_cases`, `ribbon_cases`;
  - `address_define_command_cases`: nottaki yol `contracts` → `commands`.
- **Ortam:**
  - `raster_codec_cases`: GDAL'da LERC kodeki yok;
  - `distance_scene`: GDAL sürümüyle `.tif` farkı;
  - `pointcloud_scene`: laspy gerekli;
  - `program_share_cases`, `unit_schedule_cases`: shapely ve uv gerekli;
  - `bond_drag_scene`: `kcad` modülü bulunamıyor.
- **Kaynak eşitliği:**
  - `scripts/vendor/geographiclib.py`: gömülü crate üç dosyada sürümden sapmış;
  - `scripts/gis/coverage.py`: `docs/gis/coverage.*` eski; betik web'in üretilmiş dosyasını okuyor (web iptal).

Önerilen sıra:

1. Üreteçlerin Python ortamını sabitle: sürüm ve paketler; `uv` betik başlıkları ya da bir gereksinim dosyası.
2. Hepsini koşan tek bir hedef ekle (`make fixtures-check`).
3. Son basamak farkını kanıtla, sonra yeniden yaz. CLAUDE.md §23.4: beklenen örnek hataya göre yenilenmez.
4. Eskimiş örnekleri yeniden yaz ve ilgili Rust sınamalarını koş.

### A2 — P0 — Ağır iş kilidi macOS'ta yok — [x] yapıldı

**Durum (24 Kasım 2026):** `heavy` `flock` yoksa macOS'un `lockf`'uyla aynı tanıtıcıyı kilitliyor; ikisi de yoksa uyarıyla
kilitsiz koşuyor. Komut tanıtıcıyı devralmıyor. İki işle denendi: ikincisi “başka bir ağır iş sürüyor” deyip bekledi.

`scripts/dev/svc.sh heavy`, `flock` yoksa komutu kilitsiz koşar (`exec "$@"`). Bu Mac'te `flock` yok; yani CLAUDE.md'nin
“ağır iş tek tek” kuralı fiilen işlemiyor, iki derleme aynı anda koşabiliyor. macOS'ta `/usr/bin/lockf` var; ya da
`mkdir` kilidi kullanılabilir.

### A3 — P1 — `pnpm arch:deps`, dolayısıyla `pnpm rust:test`, kırmızı — [x] yapıldı

**Durum (24 Kasım 2026):** `crates/vendor/` altındakiler crates.io sürümlerinin yerinde yamasıdır; artık grup aranmıyor,
dış bağımlılık gibi adına göre denetleniyor. `pnpm arch:deps`: “Bağımlılık yönü temiz: 85 crate, 108 crate × hedef”.

`crates/vendor/block` hiçbir bağımlılık grubuna ait değil (30 bulgu). ADR 9196 bunu “önceden vardı” diye kaydetmiş ama
kapatılmamış.

### A4 — P1 — `pnpm rust:test:desktop` paket ayrımından sonra eksik — [x] yapıldı

**Durum (24 Kasım 2026):** betiğe dört modül paketi, `kentos-cad` ve `RUST_MIN_STACK=67108864` eklendi; kılavuz satırı da.
Takımın tamamı uzun olduğu için koşturulmadı.

Betikte `kentos-desktop-arch`, `-cbs`, `-map`, `-road` ve `kentos-cad` (programlar arası sınamalar) yok; masaüstü takımı
diye koşan komut modüllerin sınamalarını atlıyor.

### A5 — P1 — Kılavuz taşınmış sınamaları eski paketle çağırıyor — [x] yapıldı

**Durum (24 Kasım 2026):** sekiz çağrı `-p kentos-desktop-arch`'a çevrildi; `material_library` da o pakette.

`docs/kilavuz/02-calistirma.md` şu sınamaları `-p kentos-desktop` ile çağırıyor; hepsi `-p kentos-desktop-arch`'ta:

- `design_option_scenes` (3 satır),
- `ifc_viewer_tests` (1),
- `materials_window` (2),
- `module_storey_scenes` (2).

### A6 — P1 — Arayüzde başka uygulama adı (10 Kasım kesin kuralı)

**Katalog metinleri (26 yer; araç ipuçlarında görünür):**

- komut adı “Netcad NCZ…” (`file.import.ncz`);
- “AutoCAD’in tüm yay yöntemleri” (`tool.arc`), “Netcad’in Koordinat Yaz’ı” (`tool.coordinateLabel`);
- “ArcGIS'in Spline'ı”, “ArcGIS'teki gibi”, “(AutoCAD LWT)”, “QGIS ve PDAL'ın biçimi”;
- `tool.centerline`, `tool.edgeShift`, `tool.stationLabels`, `crs.points`, `file.import.ncn`, `file.export.dxf`
  açıklamaları.

**Kod** (`git grep` ile sayıldı; ADR, plan ve örnek dosyaları hariç):

- Rust (`crates/`, `apps/`, sınamalar dahil): yorumlarda 831 satır; kodda 27 dosyada 55 satır, 46'sı metin sabiti.
  Örnek yardım metni: `apps/desktop-road/src/volume_window.rs:1260` “NETPRO'nun adlarıyla”.
- Python paketi (`python/kentos`): 153 satır, 132'si `cad/types.py`'de. Belge dizgileri `help()`'te ve SDK belgesinde
  kullanıcıya görünür; örnek “AutoCAD's DIMZIN”, “AutoCAD's OVERKILL”, “A Netcad NCZ drawing's MPROJ block”.
- Üreteçler (`scripts/fixtures`): 449 satır, çoğu doğrulamanın kaynağını söyleyen not.

**Öneri:**

- Arayüz metinlerini ve Python paketinin belge dizgilerini hemen temizle; yorumlar kuralın dediği gibi dokunuldukça.
- Bekçi sınaması ekle: katalog, masaüstü metin sabitleri ve `python/kentos`.
- Biçim ve protokol adlarını ayrı tut (F4).

### A7 — P2 — Arayüz metninde ADR atfı

`tool.hatch` açıklaması “(docs/adr/0186)”, `tool.railing` açıklaması “(docs/adr/8045 §3)” diyor; ipuçlarında görünüyor.
Kural: ipucu ve arayüz metni ADR'ye atıf yapmaz.

### A8 — P1 — Pafta kipinin 52 katalog komutu “bekliyor”

Bekleyen komutlar: `sheet.align.*`, `sheet.distribute.*`, `sheet.group`/`ungroup`, `sheet.order.*`,
`sheet.export.png`/`svg`/`kpafta`, `sheet.pageSetup`, `sheet.variables`, `sheet.grid`, `sheet.atlas`, yakınlaştırma
komutları…

Pafta kipinin kendi şeridi (`crates/sheet-ui`) bunların çoğunu yapıyor: hizala, dağıt, grupla, sayfa ayarları,
değişkenler, karelaj, atlas, PNG, SVG, ön denetim. Ama Komut ara, komut satırı, hızlı erişim ve Python aynı işleri
“bekliyor / pafta açık değil” diye görüyor. Manifesto da `sheet.grid`'i bekleyen, `sheet.zoomIn`/`zoomOut`'u yarım sayıyor.
Katalog, pafta kipiyle eşlenmeli.

### A9 — P1 — Şeridin iki kaynağı ayrışmış — [x] yapıldı

**Durum (24 Kasım 2026):** tek kaynak `scripts/ui/ribbon.py`: JSON'a elle eklenen 54 fark betiğe yorumlarıyla taşındı
(Mimari aileleri, nokta bulutu, rol panelleri, Ağ sekmesi, tip panelleri…). JSON betikle yeniden yazıldı; yalnız alan sırası
değişti. `ribbon_cases.py` sekmeleri artık masaüstünün şeridinden okuyor. İkisinin de `--check`'i yeşil. Şerit, kısayol
harfi, katalog, modül, ayarlar, Mimari şeridi ve programlar arası sınamalar geçti.

- `scripts/ui/ribbon.py` 426 satır geride: çalıştırılırsa JSON'a elle eklenmiş komutları siler (TODOS.md'de yazılı).
- `fixtures/shell/v1/ribbon.json` da eski (`ribbon_cases`).
- Tek kaynak seçilmeli: JSON mı, betik mi?

### A10 — P2 — 2 000 satırı aşan 61 üretim dosyası — [x] bekçi kuruldu (bölme işi ayrı)

**Durum (24 Kasım 2026):** `scripts/dev/source_size.py` sınırı `crates/` ve `apps/`'in bütün Rust dosyalarına uyguluyor
(`crates/vendor/` dış kod). Bugün aşan 71 dosya (sınamalar dahil) tabanda ve büyüyemez; yeni dosya aşamaz; taban yalnız
küçülür. `make arch` ve `pnpm rust:test` koşuyor. Dosyaların bölünmesi, dokunuldukça yapılacak.

ADR 8054'ün bekçisi yalnız `apps/desktop/src`'yi sınıyor; dosyalar `crates/` altında. En büyükleri:

- `commands/catalog/mod.rs` 4 690, `ui/widget/docking.rs` 4 333, `maplibre/lib.rs` 4 186, `dxf/emit.rs` 4 096;
- `gis/sources/mod.rs` 3 892, `contracts/arch/arch.rs` 3 631.

Bekçi genişletilmeli ya da sınır yazılı kuralla netleşmeli (CLAUDE.md §8: 400 satır üstünde ayrıştırmayı değerlendir).

### A11 — P2 — Otomatik denetim yok

`.github` yok; bütün denetimler elle koşuyor, `main`'e kırmızıyla gönderilebiliyor. A1 ve A3 bunun sonucu. Karar için F5.

### A12 — P2 — Depo temizliği — [x] yapıldı

**Durum (24 Kasım 2026):** salt okur denetim `.run/audit-stash-branch.md`'de. 30 zulanın 29'u aynı eski işin (pay ve resmî
alanlar; ADR 8031 ile yeniden yazıldı) tekrar kayıtları, biri eski paralel çizgi denemesi: hepsi geçersiz. `adv-03b`, `adv-05d`,
`adv-06f` ve iki `yedek/*` dalı yeniden yazılmış commit'lerle `main`'de; `origin/wip/formats-dxf` 3786 commit geride. ADR
0580 dalının işi `main`'e başka yoldan girmiş (ADR 0644); `origin/plan/kek-extension-architecture` yalnız belge. Sahibin
onayıyla (24 Kasım 2026) 30 zula, 6 yerel ve 2 uzak dal silindi; karmaları `.run/dropped-stashes-2026-11-24.txt` ve
`.run/deleted-branches-2026-11-24.txt`'de (`git gc`'ye kadar geri alınabilir). Birleşmemiş tek dal `origin/render-motoru` (A13).

- **30 zula** (`stash@{0…29}`), çoğu Kasım başının “clean test preservation” zulaları.
- **`main`'e girmemiş dallar:**
  - `adv-06f-turetilmis-cizim`: 3 commit, 23 bin satır;
  - `adv-05d-cok-katli-modul`: 13 bin satır;
  - `adv-03b-ozel-kapilar`: 20 bin satır;
  - `worktree-agent-a804e8d9b65d2d11d`: ADR 0580, CBS-31 yarım; bitirilmeden birleştirilmemeli;
  - `origin/wip/formats-dxf`: DXF ek verisi, yarım;
  - `origin/plan/kek-extension-architecture`, `yedek/*`.
- ADR 0777 ve 0796 `main`'de; bu dalların içeriği büyük olasılıkla başka commit'lerle girmiş. Tek tek doğrulanıp ya
  silinmeli ya birleştirilmeli.

### A13 — P1 — `origin/render-motoru` birleştirilmeyi bekliyor — ⏸ dalın işi sürüyor

**Durum (24 Kasım 2026):** sahip: “render motoru dalında halen çalışma var bitince birleştireceğiz”. Dala dokunulmaz;
iş bitince birleştirilir.

Dal 45 commit önde; ADR 9200–9219'u kullanıyor. Bugün `main`'deki üç ADR aynı numaraları almıştı; 9300–9302'ye
taşındı (`63f67c86d`). ADR numarası seçmeden önce birleşmemiş dallar da denetlenmeli.

### A14 — P1 — Web iptalinden sonra kalanlar

Hepsi kullanılmıyor ya da yanlış yönlendiriyor:

- CLAUDE.md §3 (TypeScript kuralları) ve §20 (web'in yükleme kuralları);
- kılavuz §2'nin web komutları, `package.json` betiklerinin çoğu;
- TODOS.md §0.2'nin eski satırları;
- ortak modül tanımındaki web istemcisi (`kentos_launch::Client::Web`).

Karar için F3.

### A15 — P2 — Gerçek pencere kabulü bu Mac'te depodaki düzenekle yapılamıyor

`scripts/usage/real` Xvfb ve XTEST'e dayanıyor (Linux). Bu Mac'te Swift CGEvent yolu kullanılıyor ama depoda değil.
ADV-01…07'nin hepsinde “gerçek pencerede kabul” kalan; düzenek macOS'a taşınmalı.

---

## B. Eksikler — genel CAD (Mimari, Harita ve Yol'un ortak çekirdeği)

### B1 — P1 — Yazdırma

- Dosya ▸ Yazdır ve pafta çıktısı (`file.print`) ve PDF pafta (`file.export.pdf`) bekliyor.
- Pafta kipindeki Yazdır, PDF hazırlayıp sistemin görüntüleyicisinde açıyor.
- Yok olanlar: yazıcı ya da plotter seçimi, kopya sayısı, kâğıt kaynağı, model alanından pencereyle hızlı çıktı.

### B2 — P1 — Nesne grubu

Çizim nesnelerini adlı bir grupta birleştiren ve çözen komut ya da veri yok. Var olanlar yalnız katman grubu, blok ve
pafta öğesi grubu.

### B3 — P1 — Python ve MCP'nin arayüz gibi yapamadıkları

Manifesto (`docs/cad/manifest.md`): CBS'ye özgü olmayan 1 018 satırdan 41'i.

- **Ürün komutu yok (20):**
  - dosya alışverişi: `file.export.geojson`, `.gpx`, `.kml`, `.ncn`, `.pdf`; `file.import.gnss`, `.ncn`, `.ncz`, `.photos`;
    `file.saveSelection`, `file.takeFrom`, `file.print`;
  - Harita: `map.parcelReport`, `map.profile`, `map.sheet`, `analysis.volume`;
  - öbürleri: `data.compare`, `field.send`, `pointcloud.vpcSave`, `topology.rules`.
- **Sonucu çağıran hesaplıyor (7):** `cogo.update`, `crs.points`, `tool.placeTextFile`, `tool.subdivide`,
  `tool.topology`, `transform.edgematch`, `transform.fit`.
- **Yalnız İşlemler'den (3):** `analysis.slope`, `map.contours`, `map.edgeLengths`.
- **Okuma, Python'da yok (11):** `calc.stakeout`, `cogo.check`, `crs.query`, `data.search`, `data.statistics`,
  `service.info`, `sheet.preflight`, `tool.measure`, `tool.measureAngle`, `tool.stakeout`, `tool.stationOffset`.

Özellikle betikle NCZ, NCN, GeoJSON ya da KML alışverişi yapılamıyor.

### B4 — P1 — Kısayol özelleştirme

Kullanıcı kısayol ve takma ad tanımlayamıyor. Ayarlarda böyle bir bölüm yok; yalnız Klavye kısayolları listesi ve klasik
CAD takma ad profili var.

### B5 — P2 — Eylem kaydedici ya da makro yok

`plan/cad.md`'de CAD-CLI-07 ve CAD-CLI-08 açık.

### B6 — P2 — Görünümü resim ya da SVG olarak kaydetme yok

PNG yalnız lejanttan alınıyor; paftanın PNG ve SVG'si de katalogda bekliyor (A8).

### B7 — P2 — Çoklu çizgi stili (çizgi aralıkları, uç kapakları, dolgu) yok

Paralel çizgi aracı ve Mimari'nin duvarı kısmen karşılıyor.

### B8 — P2 — Dış başvuruyu yerinde düzenleme yok

Blok düzenleyici var; dış başvuru için karşılığı yok.

### B9 — P2 — DGN ve DWF alışverişi yok

DGN yalnız “Çizimden boykesit”in notunda geçiyor.

### B10 — P1 — `plan/cad.md` 23 Ekim'den beri güncellenmemiş — [x] yapıldı

**Durum (24 Kasım 2026, `e8dabcfaf`):** açık maddeler kodla karşılaştırıldı: `[ ]` 161 → 20 gerçek açık madde, 92 `[~]`
(kalanı satırında), 40 `[x]` (kanıtıyla). Kalan en değerlileri: yazdırma/PDF/pafta gibi ürün komutu olmayan ~31 araç;
DWG gidiş-dönüşünün kanıtı ve derlem; Windows'ta Python/MCP bağlantısı; kopyada kimlik eşlemesi; ajana görünüm resmi.

161 açık madde: Python CAD-PY-01…43, yapay zekâ CAD-AI-01…40, CLI, yaşam döngüsü, performans, kalite. Bir kısmı
yapılmış olabilir (Asistan, ADR 8056; Python SDK); kodla karşılaştırılıp yeniden sıralanmalı.

---

## C. Eksikler — Harita CAD

### C1 — P1 — Bekleyen Harita komutları

Katalogda bekliyorlar, şeritte yerleri yok:

- **İfraz** (`tool.subdivide`). Sahibin kararı: hedef alanla kesim Alan böl'ün Hedef alan'ıdır; ifraz hukukidir.
- **Aplikasyon aracı** (`tool.stakeout`). Hesap penceresi var (`calc.stakeout`).
- **Parsel alan çizelgesi** (`map.parcelReport`).
- **Boy kesit al** (`map.profile`).
- **Pafta bölümlemesi** (`map.sheet`). Atlas kapsam katmanından sayfa yapıyor; standart pafta adı ve indeksi yok.
- **Hacim hesabı** (`analysis.volume`). Arazi'de Stok hacmi var.

### C2 — P1 — Kadastro ve resmî işlem çıktıları kodda yok

- kamulaştırma (0 dosya);
- tescil bildirimi, değişiklik beyannamesi, fen klasörü (0);
- ölçü ve aplikasyon krokisi (yalnız bir dosyada söz);
- röleve (0);
- yola terk, ihdas, cins değişikliği (0).

Sahibin 7 Ekim kararı: GIS-06 ve GIS-07 (ifraz, tevhit, hisse, sayı kuralı, kurum onayı) yol haritasının sonunda, sahiple
ayrı çalışma. Parsel alan çizelgesi ve kroki gibi teknik çıktıların bu karardan ayrılıp ayrılmayacağı için F6.

### C3 — P1 — NCZ'ye yazma yok

Yalnız okuma var; NCZ-01 lisans ve dağıtım kapısı da açık. Belediyelerle alışverişte önemli.

### C4 — P2 — Kurum servisleri bağlı değil

TKGM, MEGSİS ve TAKBİS servisleriyle (parsel sorgusu, ada ve parsel getirme) bağ yok; TKGM ADR'lerde yalnız araştırma
olarak geçiyor.

### C5 — P1 — MAP-01…20'nin hiçbiri bitmiş değil

Her birinin planda yazılı, sahibini beklemeyen bir sıradaki işi var (`plan/map-cad.md`, 12 Kasım durum özeti):

| Madde | Sıradaki iş |
|---|---|
| MAP-01 | sınırın projede saklanması; aplikasyon yanılma sınırı (m. 11) |
| MAP-02 | kurulumdan kırılma kestirimi; karşılıklı-eşzamanlı ve atlamalı ölçünün birleşimi |
| MAP-03 | işleme istatistiğiyle ağırlıklı bağımsız baz seçimi; “en az iki bağımsız baz” denetimi |
| MAP-04 | kurumun alan ve kod eşlemesiyle değerlerin çevrilmesi |
| MAP-05 | ağırlıklı benzerlik; bölümlü dengeleme; kabulde dayanıklı kestirim |
| MAP-06 | dönen ve yaylı referansın bağı; yüzeye bağlı kot; çok parçalı nesnede Paralel kaydır |
| MAP-07 | tutamaçla birden çok nesne; gidiş oku |
| MAP-08 | semt ve yarıçap ölçüleri, ölçülerden alan ve kapanma; kenar ölçülerinin pencerede girilmesi |
| MAP-09 | taslakta geometri ve plan notu değişikliği; EK-2 standartları denetimde |
| MAP-10 | süreyle erişim; bakının haritası; eskiyen tabloyu yerinde yazma |
| MAP-11 | yaylı parçada yay boyunca yazı; değer esasında yeni parselin değeri |
| MAP-12 | ağda radyometrik fark ve kenar algılama; alfa bandından ve maskeden ayak izi |
| MAP-13 | kontrol noktalarının ağa ölçü olarak girmesi; sanal bulutta Dönüşüm uygula |
| MAP-14 | kesitin çizimde gösterilmesi ve projede saklanması |
| MAP-15 | kesit sayfaları ve UVDF §6.4 dışa aktarımı; ek kesitler; Brükner diyagramı |
| MAP-16 | yol kaplamasının profilde gösterimi; uç çizgisi |
| MAP-17 | kodların ve katman adlarının kurum biçimi (TUCBS GKM §9.5.2.5, VTS) |
| MAP-18 | paftada kalite tablosu; önizlemede görünüş |
| MAP-19 | kayıttaki eşiklerin forma alınması; incelemenin raporu ve paftaya yazılması |
| MAP-20 | bulunan katmana tarifin bileşimlerini ve formlarını ekleme seçeneği |

### C6 — P2 — Canlı GNSS (HYB-24) ertelendi

Alıcı yok (sahibin 5 Ekim kararı).

### C7 — P2 — Gerçek veriyle ölçüm bekleyenler

MAP-07 büyük veri, MAP-12 DSM/DTM, MAP-13 LiDAR süre ve bellek, MAP-14 karolu hesap, MAP-20 bütçe.

---

## D. Eksikler — Mimari CAD

### D1 — P1 — Gerçek pencerede kabul

ADV-01…07'nin hepsinde, B6'da (binanın 3B görünümü) ve B12b'de (IFC görüntüleyici) kalan. A15'e bağlı.

### D2 — P1 — Hız hedefi tutmadı

BIM planı Z13: 2+1 daire gerçek pencerede 82 etkileşim ve 121,3 s; hedef en çok 80 etkileşim ve 100 s.

### D3 — P1 — Planların kalanları

`architecture-advanced.md` §3 ve §5, `bim-plan.md` §3.2 Z14:

- örnek proje ve malzeme bağı (M8);
- EK-4 profili ve blok katmanı (M9);
- 1:20 kasa profili (M11), perdenin 1:20 taraması (M14);
- kot etiketi ve çakışmasız yerleşim (M27), çokgen oda;
- yüzer ya da dok mimari pencereler (M7), mimari kenet süzgeci (M20), çift tıkla oda ve hedef farkı (M22);
- etiket motorunun artımlı yerleşimi (ADV-07);
- Aynı tiptekileri seç'in bağımsız bölüm kapsamı, dört kanatta eşit olmayan kanat (ADV-03);
- modül seçenekleri arasında eşleme ve birleştirme (ADV-05);
- kitaplık kaynağının izlenmesi (ADV-02).

### D4 — P1 — BIM planının açık kalanları (`bim-plan.md` §6)

| Madde | Kalan |
|---|---|
| B15 | mevzuat denetimlerinin tamamlanması (BIM-13) |
| B16 | ortak veri ortamının durumları; e-PYS pilotu |
| B17 ⏸ | büro ve belediye pilotu (sahibin işi) |
| B18 | gösterilemeyen nesnenin işareti; dosyanın bulutta saklanması |
| B19 | çakışmanın BCF'si; sorunların notlarda saklanması; masaüstü BCF görüntüleyicisinde elle açılış |
| B20 | imar denetimi; e-PYS aktarımı; erişim kaydı |
| B21 | giydirme cephe tipi; eski sürüm denetimi; kitaplık |
| B22 | BYKHY 50/2, 50/6, 51; eğitim, toplanma, endüstri, depolama kullanımları (yasal en geç 1.9.2033) |
| B23 | ertelendi (TKGM 3B kat irtifakı; yapılırsa Harita'nın işi) |

### D5 — P2 — Erişilebilirlik denetimi (TS 9111) yok

Yalnız rampa eğimi denetleniyor. TSE metni ücretli, bekleniyor (`bim-plan.md` §8).

### D6 — P2 — Blok kitaplığı küçük

Temel kitaplıkta (`catalog/v1/blocks/kentos-temel.kblok`) 40 blok var: kapı, pencere, ev mobilyası ve tek bir
çalışma masası, ıslak hacim, mutfak, asansör, otopark yeri, kot ve kuzey işaretleri. Öbür kitaplık (`kentos-ofis.kblok`)
4 bloklu bir büro kitaplığı örneği. Yok olanlar: ofis, okul ve hastane donatısı, engelli WC, peyzaj (ağaç, araç),
üretici katalogları.

### D7 — P2 — Sunum çıktısı yok

3B görünüm var; perspektif ya da render görüntüsü, gölge ve malzemeli sunum yok.

### D8 — P2 — Disiplin çizimi yok (bilinçli kapsam)

Statik ve tesisat yalnız IFC altlığı olarak giriyor; bunları çizen araç yok.

---

## E. Yol (kısa)

`plan/road.md`: 95 açık, 111 kısmi, 73 bitmiş madde; 7 Kasım'dan beri güncellenmemiş. Sahibin sözüyle en eksik modül;
ayrıntılı dökümü bu incelemenin kapsamında değil.

---

## F. Sahibin kararını bekleyenler

| # | Karar |
|---|---|
| F1 | Python ve MCP'de yeni CBS projesi de boş mu açılsın (ADR 9302 §4)? Python'da yaklaşık 50 `Document.new` çağrısı etkilenir. |
| F2 | Kurum şablonu “Şablondan yeni proje” komutu olarak dönsün mü (ADR 9302 §2.5)? |
| F3 | Web iptali: web'in kodu, komutları, kılavuz bölümleri ve `Client::Web` silinsin mi, arşivlensin mi (A14)? |
| F4 | “ArcGIS REST” (servis türü) ve “AutoCAD 2007 DXF” (DXF sürüm adı) gibi adlar başka uygulama adı kuralına girer mi (A6)? |
| F5 | Otomatik denetim: yerel bir kanca mı, sunucuda CI mı (A11)? |
| F6 | Kadastronun teknik çıktıları (parsel alan çizelgesi, kroki) GIS-06/07 kararından ayrılsın mı (C2)? |
| F7 | Mobil: AY-08'in kalanları (mobil profiller, çevrimdışı düzenleme, mobil teknoloji). |

---

## G. Bu incelemede doğrulanmadı

- Tam Rust takımları (`pnpm rust:test`, `pnpm rust:test:desktop`) ve Python takımı koşturulmadı. Yalnız bugünkü işlerin
  dokunduğu sınamalar koştu: şerit, düzenleme oturumu, sihirbaz, katalog, modül, programlar arası.
- Kılavuzdaki 46 sınama süzgecinin adı hiçbir modül adına uymuyor; işlev düzeyinde süzgeç olabilirler, tek tek
  bakılmadı.
- `--check`'i başka biçimde alan 21 üreteç koşturulmadı.
- Pafta komutlarının (A8) Komut ara'dan çalıştırılınca ne dediği gerçek pencerede denenmedi.

---

## H. Bu incelemede düzeltilenler

- `make desktop`, `build-desktop` ve `desktop-snapshot` artık programı (`kentos-cad`) derliyor. Eskiden yalnız
  kütüphaneyi derleyip eski programı açıyorlardı. `make desktop MODUL=cbs` ve `make desktop-release` eklendi
  (`75dd7138a`).
- `svc.sh start` macOS'ta `setsid` olmadığı için hiçbir servisi başlatmıyordu; Python'un `os.setsid`'i yedek oldu
  (`75dd7138a`).
- Bugünkü üç ADR'nin render dalıyla numara çakışması giderildi: 9200–9202 → 9300–9302 (`63f67c86d`).
