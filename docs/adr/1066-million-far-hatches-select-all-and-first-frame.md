# ADR 1066: Bir milyon nesnede uzak tarama, Hepsini seç ve açılışın ilk karesi

- **Durum:** kabul edildi, kısmen (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-19`, TODOS.md `PERF-13`.
  10⁶ karma çizimde uzak görünüm, Hepsini seç ve açılışın ilk karesi **60 Hz'de değil** (§6); madde `[~]` kalır.
- **Bağlam belgesi:** ADR 1062 (ölçüm düzeni), 1063–1065 (uzak görünüm kısayolları), 0029 (seçimin vurgusu), 0123
  (büyük seçimin vurgusu çekirdeklerde), 0186 (tarama gölgelendiricisi), CLAUDE.md §6, §23.
- **Web:** ertelendi (ADR 0228). Ortak çekirdekte (`kentos-geometry-core`) yalnız yerele özgü
  `Store::put_many_parallel` eklendi (`wasm32`'de yok); web'in kodu değişmez.
- **Şema:** `.kcad` değişmedi (182). Komut, ayar, pencere değişmedi.

## Bağlam

ADR 1065'ten sonra 10⁶ karma çizimde uzak kaydırma GPU p50 ~120 ms, uzak tekerlek ~70 ms, açılışın ilk karesi MİB'de
~2,2 s, Hepsini seç MİB'de ~2,1 s idi. Önce her biri parçalarına ayrıldı (§1), sonra en büyük ölçülen nedenler
kesildi; yakın görünüm ve kenet/seçim aynı kaldı (§5).

### Araştırma

- **AutoCAD, büyük seçim:** [PROPOBJLIMIT](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-LT/files/GUID-6170C0EC-0910-46A6-81DF-DE0A888573A0.htm)
  (ilk değer 25 000) Özellikler paletinin bir kerede değiştirebileceği nesne sayısını sınırlar; sınır aşılınca alanlar
  griye döner. [GRIPOBJLIMIT](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Core/files/GUID-705F3A42-4A2F-4B5C-A2A6-0CF8949B8ED5.htm)
  (100) seçim bundan büyükken tutamaçları göstermez;
  [SELECTIONPREVIEWLIMIT](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-LT/files/GUID-9CDDA4F9-5B88-428F-B0C8-345E9A9DC4F1.htm)
  (20 000) pencere/kesişme seçiminin ön vurgusunu sınırlar; belge, büyük seçimde vurgunun programı yavaşlatabileceğini
  yazar. KentOS'ta tutamaçlar zaten tek nesnede gösterilir; bu dilim sınır koymadı (özellikler panosu bir milyon
  nesneyi de değiştirir), aynı işi daha az yaparak hızlandırdı (§3). Bir sınır sahibinin kararıdır (§7).
- **QGIS, çok çekirdekte çizim:** 2.4'ten beri katmanlar ayrı iş parçacıklarında çizilir; çizim sürerken görünüm
  kaydırılabilir ve önizleme 250 ms'de bir yenilenir ([değişiklik günlüğü](https://changelog.qgis.org/en/entry/142),
  [geliştirici listesi](https://lists.osgeo.org/pipermail/qgis-developer/2013-December/029719.html)). KentOS'ta açılışın
  nesne kayıtları ve parçaları, büyük seçimin parçaları çekirdeklere dağıtıldı (§3, §4); sonuç tek iş parçacığındakiyle
  aynıdır (test).
- **Tarama:** GPU'nun tarama gölgelendiricisi çizgiler üç cihaz pikselinden sıklaşınca ortalama kaplamayı çizer
  (`hatchFs`, ADR 0186); o durumda sonuç desenin açısına, ötelemesine ve tiresine bağlı değildir (§2).

## Karar

### 1. Önce ölçüldü

Yeni tanı testi `perf::million` (10⁶ çizimi uygulamanın kendi açılışıyla açar, bir adımı `KENTOS_PROFILE_ROUNDS` kez
yineler) macOS `sample` ile örneklendi (sembollü ikili:
`--config 'profile.release.package.kentos-desktop.strip="none"'`); GPU katman katman `perf::large::layers` ile.

**Uzak kaydırma, GPU p50, katman katman (10⁶):** çizgi 51,5, tarama 24,8, blok 15,2, daire 14,6, ölçü 11,3, yazı 2,5,
hepsi 118,4 ms. Tarama katmanının 50 000 nesnesi 49 516 ayrı batch'tir (her taramanın açısı, ötelemesi ya da deseni
kendine), her biri bir çizim çağrısı.

**Hepsini seç (10⁶, MİB ~2,1 s):** görünüm ~1,95 s'nin yarısı seçimin vurgusu (`highlights`: yerleşimlerin 3,1 milyon
parçası tek çekirdekte nesneye çevriliyordu, `objects_of`), yarısı Öznitelikler panosu (`work_out_panel`: her yazının
genişlik çarpanı ve paragraf değerleri, her çizginin çizgi tipi, ölçeği ve deseni karşılaştırmak için metne
yazılıyordu; nesne başına türün adı karşılaştırılıyordu). Uygulama 135 ms (seçim kümesi), GPU ~600 ms (vurgunun
yüklenmesi).

**Açılışın ilk karesi (10⁶, MİB ~2,2 s):** uygulamanın işi ~1,4 s: geometri deposunun kurulumu (`Spatial::reload`
~0,9 s: nesnelerin kayıtları ve yerleşimlerin parçaları tek çekirdekte), bağlı dosya denetimi (`assets_check`
~0,2 s: bir milyon nesnenin her birinin UID'si metne yazılıyordu, yalnız resim/raster/tablo/ek taşıyanlar için gerekir);
görünüm ~0,77 s (stilli parçaların kurulumu ~0,4 s, zaten çekirdeklerde; yazıların ilk yerleşimi ~0,2 s).

### 2. Uzak taramalar tek çağrıda (GPU)

Tarama gölgelendiricisi (`hatchFs`) desen çizgileri üç cihaz pikselinden sıklaşınca çizgileri değil ortalama
kaplamayı çizer: sonuç yalnız rengin, çizgi kalınlığının ve aralığının, birimin, karonun ve tutulan kâğıt kalınlığının
sonucudur; açı, öteleme, tire ve kaydırma (stagger) girmez. Çizim sırasında art arda gelen, tamponda yan yana duran ve
bu değerlerde uyuşan tarama batch'leri o karede (ana görünüm; büyüteç her birini ayrı çizer) ilkinin stiliyle tek
çağrıda çizilir (`StyledGpu::draw_through`). Aynı üçgenler aynı sırada aynı değeri karıştırır: pikseller aynıdır (§5'te
iki ikilinin resimleri karşılaştırıldı). Üç pikselden seyrek desende her batch eskisi gibi kendi çağrısıdır.

### 3. Hepsini seç

- **Vurgu:** yerleşimlerin parçaları çekirdeklerde nesneye çevrilir (`objects_of_parallel`); depo bu iş parçacığında
  okunur (`Spatial::pieces_source`: parçalar ve hangisinin göründüğü), her çekirdeğin yaptığı nesneler yerinde kalır
  (bir milyonluk dizi bir kez daha taşınmaz). Aynı nesneler aynı sırada (test
  `a_large_selection_is_made_into_the_same_objects_on_several_cores`). Paftanın görünümü (sheet view) eskisi gibi.
- **Öznitelikler panosu:** yazıların ortak değerleri, genişlik çarpanı ve paragrafı ilkininkiyle aynı olanlar için
  yeniden yazılmaz (değerler bunlardan yazılır; ±0 dışında eşit değer aynı metni verir, genişlik ve aralık eksi sıfır
  almaz); çizgi tipi ve desen değerle, çizgi tipi ölçeği bitleriyle (eşit değilse eskisi gibi yazılı biçimiyle)
  karşılaştırılır; türler son türü elde tutarak sayılır. Gösterilen ve düzenlenen satırlar aynıdır.
- **Sınır konmadı:** AutoCAD'in PROPOBJLIMIT/GRIPOBJLIMIT/SELECTIONPREVIEWLIMIT'i gibi bir eşik (ör. 25 000'den büyük
  seçimde panonun değerlerini hesaplamamak) bir kullanım kararıdır; sahibine bırakıldı (§7).

### 4. Açılışın ilk karesi

- **Geometri deposu:** nesnelerin kayıtları (`viewport_scope::record_at`) ve parçaları (`Store::expansion_of`:
  yerleşimler, duvarlar, mekânlar) çekirdeklerde çıkarılır, depo nesneleri yine sırayla koyar
  (`Store::put_many_parallel`, yalnız yerel). Aynı depo: her nesnenin biçimi, kutusu, parçaları, katmanı ve sırası,
  tıklama sonuçları (test `many_objects_put_on_several_cores_make_the_same_store`, 6 000 nesne, 8 çekirdek).
- **Bağlı dosya denetimi:** UID yalnız bir dosya ya da adres bağlayabilecek nesne için yazılır (`links_anything`);
  rapor aynıdır.

### 5. Ölçüm ve eşitlik (Apple M5, release)

Ortam: Apple M5 (10 çekirdek), 32 GB, macOS 26.6.2, GPU Apple M5 (Metal), rustc 1.96.0, `--release` (lto thin, sembollü
ikili: sembol tablosu hızı değiştirmez), 1440×900, 4× örnekleme. “Önce” `origin/main` `8563e0e2e` ikilisi, “sonra” bu
dalın; aynı sabit iz arka arkaya, ortak ağır iş kilidi tutularak. Bütün tablolar:
[docs/perf/large-desktop-adr1066-2026-11-06.md](../perf/large-desktop-adr1066-2026-11-06.md).

| (ms; ısınmış koşu, önce → sonra) | 10 000 | 100 000 | 1 000 000 |
|---|---:|---:|---:|
| Açılış: ilk kare MİB | 28,3 → 44,6 | 238,0 → 174,4 | 2 255 → 1 478 |
| Açılış: iş parçacığı / açılıştan ilk kareye | 16,6 / 59,9 → 24,7 / 87,8 | 155,7 / 433 → 157,7 / 374 | 1 659 / 4 194 → 1 640 / 3 373 |
| Hepsini seç MİB / GPU | 14,5 / 6,7 → 11,1 / 11,4 | 171,5 / 26,6 → 109,2 / 38,5 | 2 156 / 322 → 1 152 / 203 |
| Uzak kaydırma GPU p50 | 7,4 → 7,3 | 12,1 → 10,7 | 118,5 → 108,1 |
| Uzak tekerlek GPU p50 | 7,1 → 7,3 | 17,2 → 15,6 | 71,0 → 65,0 |
| Yakın 1:200 kaydırma GPU p50 | 7,3 → 7,4 | 7,4 → 7,6 | 10,1 → 11,3 |
| Bir yazıyı taşı MİB | – | – | 21,9 → 22,0 |
| Süreç tepe ayak izi (MB) | 1 021 → 1 014 | 2 131 → 2 183 | 15 360 → 14 336 |

10⁴ satırındaki açılış farkı (28 → 45 ms) bu sayının koşudan koşuya oynamasının içindedir: ADR 1065'in aynı kodlu iki
koşusu da 26 ve 47 ms verdi; 10⁴'te açılışın ilk karesi ölçümün ilk kareleriyle (ısınmamış önbellekler) karışır. Katman katman 10⁶ uzak kaydırma: tarama 24,8 → 13,2 ms,
hepsi 118,4 → 107,8 ms (`perf::large::layers`, aynı ölçü).

**Eşitlik:**

- **Resimler:** iki ikilinin uygulama resimleri (10⁴ uzak, 1:200, 1:20; 10⁵ uzak, 1:200; iki tema) çizim alanında
  **piksel piksel aynı** (her birinde 0 farklı piksel; pencerenin geri kalanında yalnız komut geçmişinin saati).
- **`perf::stability`:** 30 satırın hepsi ve kenetler ADR 1065'teki koşuyla sayı sayı aynı (kaydırmalar, ayrıntı
  geçişleri, kısayolların açık/kapalı satırları; kenet üç görünümde bite kadar aynı).
- **Testler:** depo çok çekirdekte aynı (`many_objects_put_on_several_cores_make_the_same_store`); vurgunun nesneleri
  aynı ve aynı sırada (`a_large_selection_is_made_into_the_same_objects_on_several_cores`).

### 6. Hâlâ 60 Hz'de olmayanlar (10⁶)

- **Uzak kaydırma 108 ms, tekerlek 65 ms (GPU):** çizgi katmanı 51 ms (400 000 ayrı düz çizgi, her biri 1–2 px; tek
  nesne sadeleşmez), tarama 13 ms (49 516 batch'in hazırlığı ve aynı stilde olmayanların çağrıları), blok 15 ms (vekil
  açık), daire 15 ms, ölçü 11 ms. Sonraki aday: kaydırmada tutulan resmin kaydırılıp yalnız açılan şeridin çizilmesi
  (ekran karoları), ya da aynı renkteki uzak batch'lerin sıra korunarak birleştirilmesi; ikisi de ayrı iş.
- **Hepsini seç 1,15 s:** vurgunun kurulumu ~0,6 s (3,1 milyon parçanın çekirdeklerde çizgiye çevrilmesi ve parçaların
  birleştirilmesi), Öznitelikler ~0,3 s (düzenleyicilerin her birine bir milyon kimliğin kopyası), seçim kümesi 0,14 s;
  GPU'da vurgunun yüklenmesi 0,2 s. Vurguyu arka planda kurup hazır olunca göstermek ya da bir PROPOBJLIMIT benzeri
  eşik sahibinin kararıdır.
- **Açılışın ilk karesi 1,5 s:** depo kurulumunun sırayla kalan kısmı (dizine koyma), stilli parçalar ~0,4 s, yazıların
  ilk yerleşimi ~0,2 s. Açılışın iş parçacığı (1,6 s) ayrıca sürer; pencere o sürede akıcıdır (açılış sürerken kareler
  MİB p95 0,4 ms).
- **Tek düzenleme 22 ms (10⁶):** değişen bir parçanın yeniden kurulması 21 ms (1 470 parçadan biri); bu dilimde
  ölçüldü, değiştirilmedi.

### 7. Sahibinin kararları

- Büyük seçimde AutoCAD gibi eşikler (PROPOBJLIMIT 25 000, SELECTIONPREVIEWLIMIT 20 000): panonun değerlerini ve
  vurguyu belli bir sayının üstünde hesaplamamak ya da arka plana almak.
- 10⁶ uzak görünüm için ekran karoları (tutulan resmin kaydırmada yeniden kullanılması).

## Sonuçlar

- 10⁶'da açılışın ilk karesi 2,26 → 1,48 s, Hepsini seç 2,16 → 1,15 s, uzak kaydırma 118 → 108 ms; 10⁵'te Hepsini seç
  172 → 109 ms, ilk kare 238 → 174 ms, uzak kaydırma 12,1 → 10,7 ms.
- Çizilen hiçbir piksel değişmedi; kenet ve seçim aynı.
- CAD-ADV-19 `[~]`: 10⁶'da 60 Hz yok (§6).
