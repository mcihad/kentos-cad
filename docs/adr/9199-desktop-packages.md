# ADR 9199: Masaüstünü paketlere bölme: çekirdek ve modül paketleri

- **Durum:** kabul edildi (2026-11-23); plan [plan/ayrim.md](../../plan/ayrim.md) AY-07. [ADR 9195](9195-module-separation.md)
  §6'nın kalanını (kabuğun modüllere bağı, taban) uygular.
- **İstek:** sahibin planı (23 Kasım 2026): modüller ayrı programlara bölünebilecek kadar gevşek bağlı olsun; hepsi bir
  arada program bütün paketleri, modül programı yalnız kendininkini taşısın.
- **Kural:** CLAUDE.md §25 kural 3 (kabuk modülü tanımaz, modül modüle bağlanmaz), kural 8 (yeni modül tarifle).

## §1 Bağlam ve ölçüm (23 Kasım 2026, AY-08'den sonra, `9f6f34078`)

**Paket:** masaüstü tek crate'tir (`kentos-desktop`): 1 562 dosya, 793 kök modül (`lib.rs` ve `test_suites.rs`'te
`#[path]` ile). Klasörlere göre satır: kabuk 193 bin, CBS 155 bin, CAD 128 bin, sınamalar 121 bin, Yol 32 bin, Mimari
28 bin, Harita 19 bin.

**Düğüm:** üretim kodunun 379 kök modülünden yalnız 15'i (560 bin satırın 4,4 bini) `App`'e hiç ulaşmaz (kapanışıyla,
`crate::app`). Geri kalan her şey `App` ve `Message` çevresinde tek bir düğümdür:

- `impl App` blokları: kabukta 201, CBS'de 161, CAD'de 111, Yol'da 44, Mimari'de 43, Harita'da 32 dosyada;
- CAD'in 88 kök modülünden 84'ü kabuğun `app` modülüne bağlıdır; klasör olduğu gibi pakete alınamaz;
- modüllerin durumu zaten toplu (`App.road`, `map`, `gis`, `arch`; ADR 8054 §6), iletisi de (`Message::Road`, `Map`,
  `Gis`, `Arch`) ve pencereleri (`Dialog::Road` …).

**Derleme süresi** (sınama ikilisi, `cargo test -p kentos-desktop --lib --no-run -j 10`, bu Mac):

| Durum | Süre |
|---|---|
| Masaüstü crate'i sıfırdan (bağımlılıklar hazır) | 72 s (112 s işlemci: 1,6 çekirdek) |
| Harita dosyasında tek satır | 17,4 s |
| `App`'e yeni yöntem | 24,5 s |

Tek crate'in ön yüzü büyük ölçüde tek iş parçacığıdır: masaüstü sıfırdan derlenirken on çekirdeğin 1,6'sı çalışır.

## §2 Araştırma (23 Kasım 2026'da okundu)

- **Rust başvurusu, *Implementations*:** bir türün kendi (inherent) `impl`'i yalnız türün tanımlandığı crate'te
  olabilir; başka crate'te E0116. Başka crate yalnız kendi trait'ini o türe uygular (yetim kuralı); yabancı trait'i
  (`From<Yerel>`) yabancı türe, yerel tür parametresiyle uygulayabilir.
  <https://doc.rust-lang.org/reference/items/implementations.html>,
  <https://doc.rust-lang.org/nightly/error_codes/E0116.html>
- **Cargo kitabı, *Reporting build timings*:** birçok crate tek bir crate'i bekliyorsa o crate darboğazdır; onu
  hızlandırmak (bölmek) paralelliği artırır. <https://doc.rust-lang.org/cargo/reference/timings.html>
- **iced belgesi** (`Element::map`, `Task::map`, `Subscription::map`): alt parçanın kendi ileti türü olur, üst parça onu
  kendi iletisine çevirir. <https://docs.rs/iced/latest/iced/>

## §3 Karar

1. **Paketler ve yön.**
   - **Çekirdek** `kentos-desktop` (`apps/desktop`): kabuk ve CAD. Modüller ayrıldıkça modülsüz kalır; hiçbir modül
     paketine bağlanmaz.
   - **Modül paketleri** `kentos-desktop-map`, `-road`, `-arch`, `-gis` (`apps/desktop-<modül>`): yalnız çekirdeğe
     bağlanır; birbirine bağlanmaz.
   - **Programlar** ayrı bir pakette: `kentos-cad` bütün modül paketlerini, modül programı yalnız kendininkini
     başlangıçta kaydeder.
   - Sıra en az bağlıdan: Harita, Yol, Mimari, CBS. Çekirdek, modüllerin hepsi çıkınca “CAD çekirdeği” paketidir.
   - **Planın sırasından sapma:** plan önce CAD çekirdeğini ayrı paket yapar. Bağımlılık tabanının katmanları da öyledir:
     kabuk en altta, CAD çekirdeği üstünde (kabuk → cad izlenir), CBS CAD'e bağlanmaz (gis → cad izlenir). Ama CAD'i
     kabuktan ayırmak 111 dosyalık `impl App`'i ve kabuk → cad (77 dosya), gis → cad (54 dosya) bağlarını çevirmeyi
     ister; modül paketlerinin düzeneği (§3.3–3.5) bunun da önkoşuludur. Düzenek önce en az bağlı modülde (Harita)
     kanıtlanır, CAD çekirdeği aynı düzenekle modüllerden sonra ayrılır.
2. **Ortak parça çekirdeğe iner.** Bir modülün klasöründe duran ama başka modüllerin de kullandığı parça, paket
   bölünmeden önce çekirdeğe (ya da sahibi olan modüle) taşınır. Harita'da: kuzey oku ve ölçek çubuğu (`map_marks`),
   Nokta hesapla (`point_calc`), Harita sistemi (`map_crs`, CBS ve Yol'da da açık), Emsal ve TAKS rozeti
   (`zoning_badge`, Mimari'nin).
3. **Modül arayüzü** (`ModuleParts`, çekirdekte; profilin yanında): kabuğun modüle dokunduğu her yer bir yöntemdir.
   İletisi, pencereleri, komut kimlikleri, çizim alanındaki işaretleri, panelleri, durum hücresi, belge değişince ve
   işaretler temizlenince yaptıkları, kullanım izinin pencere başlıkları ve denetimleri.
4. **Silinen türler.** Modülün durumu, iletisi ve pencere türü çekirdekte adıyla durmaz:
   - durum `App.modules`'tedir (modüle göre, `Any`); modül kendi erişimcisini yazar;
   - ileti `Message::Module(ModuleEvent)`: modül ve paylaşılan bir değer; modül `From<Event> for Message` ile kurar ve
     kendi türüne geri çevirir;
   - pencere `Dialog::Module(ModuleDialog)`: modül ve pencerenin sayısı.
5. **Modülün `impl App`'i uzantı trait'idir.** Modül paketinde her `impl App` bloğu kendi trait'iyle yazılır
   (`impl ZoningWindow for App`); modülün kendi ön yüklemesi (`prelude`) hepsini kapsama alır. Gövdeler değişmez.
6. **Görünürlük:** modül paketinin kullandığı çekirdek öğeleri `pub` olur; çekirdeğin sınama yardımcıları `testing`
   özelliğiyle açılır.
7. **Ölçüm her adımda:** bağımlılık denetimi (`scripts/dev/module_deps.py --check`), `App`'e ulaşmayan modüllerin sayısı
   ve §1'in derleme süreleri.

## §4 Uygulama

Her adım kendi alt bölümünde, ölçümüyle yazılır.

### §4.1 Adım 1: Harita modül arayüzünün arkasında (aynı crate'te)

- **Ortak parçalar yerine indi** (davranış aynı):
  - kuzey oku ve ölçek çubuğu `map_marks` → `cad/`; Harita sistemi `map_crs` → `cad/`, kendi iletisi
    (`Message::MapCrs`, `MapCrsClose`) ve penceresiyle (`Dialog::MapCrs`);
  - Nokta hesapla `point_calc` → `shell/`, kendi iletisiyle (`Message::PointCalc`); Emsal ve TAKS rozeti `zoning_badge`
    → `shell/` (Mimari'nin; Mimari paketi ayrılırken onun kancalarına geçer);
  - pencere iskeleti `road_dialog` ve görüntüleyici `picture` → `shell/` (Harita, Mimari, CBS ve CAD pencereleri de
    kullanır; DESIGN.md §7.19);
  - GNSS çözümünün ve durumunun sözcükleri Öznitelikler'e (`properties::survey_kind_words`, `survey_state_words`);
  - sistemlerin datuma göre listesi `kentos_project::crs::others_by_datum` (önce CBS'nin `second_crs::choices`'iydi;
    Harita sistemi, ikinci sistem, proje ayarları ve katman sistemi kullanır).
  - Yuvalar bağımlılık tabanını büyütmeyecek yere seçildi: kabuk en alttır, her klasörün kabuğa bağı serbesttir.
- **Modül arayüzü** `shell/module_parts.rs`: `ModuleParts` (ileti, durum, komut kimliği ve açıklığı, pencere görünümü,
  iz başlığı ve denetimi, kapanan pencere, belge değişimi, işaret temizleme, işaretler, alan katmanları, panel),
  `ModuleEvent`, `ModuleDialog`, `ModuleStates`, `ModuleMarks`; kayıt `all()` (bu adımda içeride: Harita).
- **Harita** `map/parts.rs`: 21 komut kimliği, 19 pencerenin başlığı ve iz denetimi, dört işaret (imar, baz, boru,
  sınır grafiği), Harita işinin onaylı biçimleri, iki panel. `Message::Map`, `Dialog::Map` ve `App.map` kalktı:
  ileti `From<map::Event> for Message`, pencere `map::dialog(kind)`, durum `App.modules` ve `MapState`
  (`modules.map()`, `map_mut()`; 694 erişim betikle çevrildi, bir ödünç çakışması elle çözüldü).
- **Bağımlılık tabanı** (yalnız azaldı): kabuk → Harita 13 → 2 (iki kayıt dosyası), cad → Harita 7 → 0, Harita → CBS
  4 → 3, kabuk → Yol 12 → 6, cad/CBS/Mimari/Harita → Yol 5/4/2/22 → 0.
- **Sınama:** masaüstü 3061 geçti; `kentos-project`'in `others_by_datum` sınaması.

### §4.2 Adım 2: Harita kendi paketinde

- **Uzantı trait'leri:** Harita'nın 30 `impl App` bloğu `impl <Dosya>App for App` oldu (betikle; gövdeler aynı); hepsi
  `ext.rs`'te adlanır, dosyalar `use crate::map_ext::*;` ile alır. Çekirdeğin `road_dialog::Col`'una eklenen yardımcı
  yerel işleve döndü.
- **Paketler:**
  - `apps/desktop-map` (`kentos-desktop-map`): Harita'nın 37 modülü ve 51 sınama dosyası. Kökü çekirdeğin modüllerini
    kendi adlarıyla alır (`use kentos_desktop::*;`), dosyalar çekirdekte yazıldıkları gibi kalır.
  - `apps/kentos-cad` (`kentos-cad`): program; `main` önce `kentos_desktop_map::register()`'ı çağırır. Programı derleyen
    betikler (`module_links.py`, `pgo.sh`, `rebuild-times.sh`, `ubuntu.sh`, gerçek pencere kılavuzu) `-p kentos-cad`.
  - Çekirdek modül paketini adıyla anmaz: kayıt dinamiktir (`module_parts::register`, ilk sorulunca modüllerin
    sırasıyla sabitlenir); profil de parçalardan gelir (`ModuleParts::profile`).
- **Görünürlük:** Harita'nın ve sınamalarının kullandığı çekirdek modüller `pub mod`, öğeler `pub` oldu; derleyicinin
  görünürlük hatalarından betikle (tanımın yeri notta; alanda yapı çekirdekte bulunur), yalnız gerekenler.
- **Sınamalar üç yerde:**
  - çekirdek kendi sınamalarını koşar (2837); Harita'yı tanımaz;
  - Harita'nın paketi kendi pencerelerini ve Harita projesindeki davranışı sınar (191): kayıt `ctor`'un bildirimsel
    biçimiyle sınamalardan önce (geliştirme bağımlılığı, çalışma alanının kilit dosyasında zaten var; Apache-2.0/MIT;
    macOS, Linux, Windows); çekirdeğin sınama yardımcıları `testing` özelliğiyle açılır (`files_testing`, `Log::last`,
    `said`, `Keeper::open`/`kept`, `listed_formats`, `Service::settle`/`members`, `document::write`,
    `sheet_pdf::open_with_viewer`; anahtar zinciri, çevrimiçi adres, 3B bina, sihirbaz ve gösterge dalları);
  - programlar arası sınamalar `apps/kentos-cad/tests/programs`'ta, her modül kayıtlı (34): yüzey matrisi, katman
    menüsü, her komutun işleyicisi, her kullanım izi, Yeni proje'de iki modül.
  - Rust'ta çekirdeğin birim sınaması modül paketine bağlanamaz (geliştirme bağımlılığı çevriminde crate'in iki kopyası
    olur), bu yüzden modülü gereken sınama ya modülün ya programların paketindedir. Mimari menüsünün sınaması ikiye
    bölündü: Mimari'nin yarısı çekirdekte, Harita'nın yarısı Harita'nın paketinde.
- **Bağımlılık tabanı:** kabuk → Harita 2 → 0. Harita → CBS 3 dosya kalır (CBS çekirdekteyken derlenir; CBS ayrılmadan
  sıfırlanmalı). `module_deps.py` modül paketlerini de okur (`PACKAGES`).
- **Derleme süresi** (§1'in yöntemi; sınama ikilisi `--no-run -j 10`):

| Durum | Önce | Sonra |
|---|---|---|
| Harita dosyasında tek satır | 17,4 s (bütün masaüstü) | **3,7 s** (Harita'nın sınamaları); program 3,5 s |
| Çekirdek sıfırdan | 72 s | 69 s |
| `App`'e yeni yöntem | 24,5 s | 23,6 s (çekirdeğin sınamaları) |

  Kazanç modülün kendi işindedir; çekirdek değişikliği çekirdeği yeniden derletir, modül paketleri onun üstüne gelir.
- **Sınama:** çekirdek 2837, Harita 191, programlar 34 geçti (önce 3061; Mimari sınaması ikiye bölündü); yok sayılanlar
  (resimler) 664 + 92 + 5 = 761, önceki gibi.

### §4.3 Adım 3: Harita hiçbir modüle bağlı değil

- Harita → CBS'nin üç dosyası CBS'nin dört yardımcısını kullanıyordu; hepsi çekirdeğe indi, eski yerleri onları
  yeniden dışa verir (CBS'nin yolları aynı):
  - sayı okuma ve yazma `shell/numbers.rs`: `read_number` (web'in `readNumber`), `small_angle_text`, `mm_text`;
  - bir komutun reddi ve tek geri alma adımı `shell/document.rs`: `refusal`, `in_step`.
- Bağımlılık tabanı: Harita → CBS 3 → 0. Harita'nın paketi yalnız çekirdeğin kabuğuna ve CAD'ine bağlıdır.
- Sınama: çekirdek 2837, Harita 191, programlar 34.

### §4.4 Adım 4: Harita ile Yol'un ortak arazi işi çekirdekte

- 15 modül (7,5 bin satır) `road/`'dan `cad/`'a taşındı: Arazi yüzeyleri ve düzenlemeleri, yüzey katmanı, pafta
  eğrileri, yığın, buluttan arazi, yüzey analizi, Eğimle kot ver (başvuru ve yüzeyiyle), platform, drenaj, proje ağacı,
  iş grafiği ve kesit kontrolü. Son ikisi yalnız ortak pencerelerin düğmesiyle açılır; onlar da ortaktır.
- Kendi merkezleri `cad/terrain_work.rs`: `TerrainWorkState` (`App::terrain_work`), `terrain_work::Event`
  (`Message::TerrainWork`), `TerrainDialogKind` (`Dialog::TerrainWork`). Yol'un merkezinden 11 alan, 11 ileti ve 10
  pencere çıktı; başvurular betikle çevrildi (204 ileti, 59 pencere, 381 durum erişimi).
- Proje ağacının başlığı ve boş metni modül adına göre dallanıyordu (`self.program == Some(Module::Map)`, programa
  bakarak); artık açık projenin modülünün profilinden gelir (`ModuleProfile::project_tree`, her profilde açıkça):
  hepsi bir arada programda Harita projesi de “Proje ağacı”nı gösterir.
- Bağımlılık tabanı: kabuk → Yol 6 → 4; çekirdekte modül adına göre dal artmadı.
- Sınama: çekirdek 2837, Harita 191, programlar 34.

### §4.5 Adım 5: Yol modül arayüzünün arkasında (aynı crate'te)

- `road/parts.rs`: 37 komut kimliği (editörler, LandXML, IFC, glTF), 22 pencerenin başlığı ve iz denetimi (izlerin
  kodundan betikle üretildi), eksen katmanı (`alignment_layer`). Profil parçalardan gelir.
- Dosya seçicilerin cevapları (`LandxmlExport`, `LandxmlMachine`, `LandxmlAlignments`, `IfcExport`, `GltfExport`)
  çekirdeğin iletisi değil, Yol'un iletisidir. 3B penceresinin glTF düğmesi `gltf.export` komutunu çalıştırır; komut
  Yol'un parçalarına gider, yeni kanca gerekmedi.
- `Message::Road`, `Dialog::Road`, `App.road` kalktı: `From<road::Event> for Message`, `road::dialog(kind)`,
  `RoadState` (`modules.road()`, `road_mut()`; 470 erişim betikle).
- Kayıt: paketine ayrılmamış modülün parçaları çekirdeğin kayıt dosyasındadır (`module_profile::IN_CORE`); `all()`
  onları programın kaydettikleriyle birleştirir.
- Bağımlılık tabanı: kabuk → Yol 4 → 1 (kayıt dosyası). Sınama: çekirdek 2837, Harita 191, programlar 34.

### §4.6 Adım 6: Yol kendi paketinde

- `apps/desktop-road` (`kentos-desktop-road`): Yol'un 33 modülü (iç içe dört dosyasıyla) ve 32 sınama dosyası; program
  `main`'de, programlar arası sınamalar `ctor`'la Yol'u da kaydeder. Paketine ayrılmamış modül kalmayınca
  `module_profile::IN_CORE` boşaldı (Mimari ve CBS ayrılana kadar onlar profil eşleşmesiyle çekirdekte).
- Taşınmayan iki sınama dosyası Yol'a bölündü: `typing_tests`'in Hakediş sınaması (`road_typing_tests`), Yol
  pencerelerinin resimlerini sınayan `picture_tests` (komut kimliğiyle açtığı için yol taramasında görünmüyordu).
- Bağımlılık tabanı: kabuk → Yol 1 → 0; Yol'un paketi yalnız çekirdeğe bağlı.
- Derleme: Yol dosyasında tek satır Yol'un sınamalarını 4,0 s'de derler (önce bütün masaüstü 17,4 s).
- Sınama: çekirdek 2710, Harita 191, Yol 127, programlar 34 (toplam 3062, yok sayılan 761: önceki gibi).

### §4.7 Adım 7: Mimari klasöründe olup çekirdeğin kullandığı parçalar

- Mimari'ye bağlı olmayan ve çekirdeğin kullandığı beş modül çekirdeğe indi: programın kimliği (`program`), etkin kat
  (`active_level`), IFC olguları (`ifc_facts`), kesit rozeti (`section_badge`) → `shell/`; görünüş tarifi
  (`views_recipe`) → `cad/`. Yuvalar yine tabanı büyütmeyen yerler.
- `program.rs`'in modül adına göre 42 dalı kabuğa geçemezdi (ADR 9195 §6'nın kalanı): modülün sözcükleri
  `kentos_launch::Module`'e (`open_in`, `in_program`, `in_project`, `of_program`), disiplinin modülü
  `Discipline::module`'a, CBS programında açılan CAD çiziminin sorulduğu programlar saklama türünden (dosya saklayan
  modüller) geldi; satır içi sınamaları `program_tests.rs`'e ayrıldı. Kabuktaki dal sayısı aynı kaldı (36).
- Bağımlılık tabanı: kabuk → Mimari 28 → 15, CAD → Mimari 7 → 3.
- Sınama: çekirdek 2711, Harita 191, Yol 127, programlar 34, `kentos-launch` 16.

### §4.8 Adım 8: Yol'un tasarım denetimi Yol'a, şablonlar ve etkin kat çekirdeğe

- Tasarım denetimi (`alignment.check`, plan/road.md Y21) Mimari klasöründe Mimari'nin merkezindeydi; Yol'undur: durumu,
  iletisi ve penceresi Yol'un merkezinde, komutu, başlığı ve iz denetimi Yol'un parçalarında, sınamalarıyla Yol
  paketinde. Yorumundaki başka uygulama atıfları temizlendi.
- Şablonlar her modülün (AY-03: CBS'nin öğe şablonu): panel, şablonla çizme, editör ve üye yazımı `cad/`'a indi,
  kendi merkezleri `cad/template_work.rs` (`App::template_work`, `Message::TemplateWork`, `Dialog::TemplateWork`:
  `RemoveTemplate`, `TemplateEditor`).
- Etkin katın durumu `App::active_level` (Mimari'nin merkezinden çıktı).
- `shell_boundary_tests`'in “disiplin modülleri bulundu” eşiği 100'den 50'ye indi: modüller paketlerine çıktıkça
  çekirdekte kalan sayı düşer.
- Bağımlılık tabanı: kabuk → Mimari 15 → 14, CAD → Mimari 3 → 2. Sınama: çekirdek 2709, Harita 191, Yol 129,
  programlar 34.

### §4.9 Adım 9: Mimari modül arayüzünün arkasında (aynı crate'te)

- Mimari klasöründe olup Mimari'nin olmayan dört parça çekirdeğe indi:
  - NTv2 ızgara kitaplığı (`grids`, proje ayarlarının datum seçimi) → `shell/grids.rs`: `App::grids`,
    `Message::GridsLooked`.
  - Araç ayarları paneli (`architecture_settings` → `shell/tool_settings.rs`): her modülün araçlarının ayarları;
    Katman anahtarı satırı ve `Message::ToolSetting` onunla.
  - Dar durum çubuğunun Yardımcılar menüsü (`compact_aids_menu`) → `view/status.rs`.
  - Altlık modelinin okunuşu (dosya, özet, gövdeler, plan çizgileri) → `shell/underlay_model.rs`; 3B görünüm onu
    kullanır, ne zaman okunacağı Mimari'nin.
  - Ruhsat yayınının kapısı (`permit_gate`) çizimin `cad` olgularını kendisi ölçer.
- `arch/parts.rs`: 31 komut kimliği, 21 pencerenin başlığı ve iz denetimi, Esc ve Tamam'la kapanışları (kendi kaydı
  olan dört düzenleyici önce sorar).
- `ModuleParts`'a yeni kancalar:
  - `started`: Mimari programının ilk duvar stili.
  - `device`: Malzemeler kitaplığı, programın geçmiş klasörü yanında.
  - `follow`: altlıkların okunuşu ve planı.
  - `escape_closes`, `dialog_confirmed`: Mahalleri birleştir.
  - `checked`: Tadilat görünümü.
  - `picked`: altlıkta Planda seç.
  - `preview`: Denetim'in seçili düzeltmeleri.
  - `bottom_tab_view`: Denetim sekmesi.
  - `properties_ask`: Öznitelikler'in Tipe uygula önizlemesi, Tip satırı ve Çizelge eylemleri. Satırlar CAD
    çekirdeğinde kalır; soruyu kabuğun `PropertiesAsk` türü taşır, kabuk → CAD bağı büyümez.
  - `building3d` (`Building3dParts`): 3B görünüm ve IFC görüntüleyicide çizimin yanında gösterilen modeller, işaretli
    nesneler, seçilen altlık nesnesi.
- `Message::Arch`, `Dialog::Arch`, `App.arch`, `Message::IfcBuildingExport(Cut)` kalktı. Yerlerine:
  - `From<arch::Event> for Message` (213 yer) ve `arch::dialog(kind)` (42 yer);
  - `ArchState`: `modules.arch()`, `arch_mut()` (800 erişim betikle).
  - Dosya seçicinin cevabı Mimari'nin iletisidir. Mimari şablonu penceresinin başlığı `TITLE`'dır.
- Bağımlılık tabanı: kabuk → Mimari 14 → 1 (kayıt dosyası), CAD → Mimari 2 → 0.
- Sınama: çekirdek 2709, Harita 191, Yol 129, programlar 34.

### §4.10 Adım 10: Mimari'nin `App` yöntemleri uzantı trait'i

- Mimari'nin `impl App` bloklu 32 dosyası 32 trait oldu (betikle, gövdeler değişmeden); adlar `arch/ext.rs`'te,
  dosyalar ve sınamalar `use crate::arch_ext::*` ile alır. İç içe modüllerin (`arch_audit`'in dördü, Bina programının
  görünüşü) adları klasörden gelir (`ArchAuditPreviewApp`), modülleri trait'e ulaşılsın diye `pub(crate)`.
- Gövdesiz bildirimde `mut` desen olamaz (iki yöntem); trait'in açtığı üç tür (`Table`, `ContextAction`, `Input`)
  `pub(crate)` oldu.
- Sınama: çekirdek 2709, programlar 34.

### §4.11 Adım 11: Mimari kendi paketinde

- `apps/desktop-arch` (`kentos-desktop-arch`): Mimari'nin 30 modülü (iç içe `arch_audit`'in dördü, `quantities`, Bina
  programının görünüşü) ve 80 sınama dosyası; program `main`'de, programlar arası sınamalar `ctor`'la Mimari'yi de
  kaydeder. `module_profile::IN_CORE` yine boş; CBS profil eşleşmesiyle çekirdekte.
- Sınamaların ağı: Mimari'nin sınamaları çekirdeğin sahne kitine dayanıyordu. Kit `kentos-desktop`'un `testing`
  özelliğiyle açıldı (`files_testing` gibi):
  - `tools_scenes`, `tools_screens` (resim koşucusu `take`; çekirdeğin listesi ve `screens`'i `cfg(test)`);
  - duvar, program, mahal, mobilya, açıklık, gezinti ve mimari sahneleri; ruhsat ve kabul yardımcıları (14 modül);
  - kitte yalnız çekirdeğin sınamalarının kullandığı yardımcıların uyarısı kapalı; kit iki bağımlılığı isteğe bağlı
    alır (`miniz_oxide`, `parking_lot`); yazı ölçüsü kilidi `appearance::TYPOGRAPHY`'dir.
- Çekirdekte kalan üç sahne dosyası bölündü: Mimari pencerelerine dokunan sahneler pakete (`wall_style_scenes`,
  `program_window_scenes`, `quantities_scenes`); duvar, ev ve mahal çizen yardımcılar çekirdekte. Mimari'nin resimleri
  kendi toplayıcısındadır (`arch_screens`).
- Mimari'nin kaydına dayanan beş çekirdek sınaması taşındı:
  - Mimari programının başlangıcı (ölçü katmanı, Duvara hizala) ile CBS'yi Mimari'yle karşılaştıran IFC sınaması pakete
    (CBS henüz çekirdekte);
  - modülleri karşılaştıran iki sınama (her programın yerleşimi, Mahal ayırıcı) programlara.
  - 2+1 dairesinin izi (`traces::flat`) çekirdeğin kitinde; ruhsat paketi sınaması onu kullanır.
- Bağımlılık tabanı: kabuk → Mimari 1 → 0; Mimari'nin paketi yalnız çekirdeğe bağlı. Kalan izlenen bağlar kabuk → CAD
  77, kabuk → CBS 44, CAD → CBS 30, CBS → CAD 54.
- Derleme: Mimari dosyasında tek satır Mimari'nin sınamalarını 3,7 s'de, çekirdekte tek satır çekirdeğinkileri 14,7
  s'de derler (önce bütün masaüstü 17,4 s).
- Sınama: çekirdek 2503, Mimari 204, Harita 191, Yol 129, programlar 36. Toplam 3063, öncekiyle aynı; yok sayılan 762,
  yeni `arch_screens` ile bir fazla.

### §4.12 Adım 12: `gis/` çekirdeğin ortak CBS altyapısı; CBS modülü `cbs/`

**Çözümleme** (24 Kasım 2026, `6e3ef13e6`'dan sonra). `gis/` 84 modül, 155 bin satır; çekirdeğin 74 dosyası (kabuk 44,
CAD 30) ona bağlı, 59 dosyası 294 yöntemini çağırıyor. Ortak katalogdaki (`catalog/v1/modules.json`) komut kümeleri
gösteriyor ki bu klasör CBS modülü değil, bütün modüllerin ortak CBS ve veri altyapısıdır:

- Çağrılan komutların çoğunu Harita da, çoğunu Yol ve Mimari de sunar: veri katmanları ve kaynaklar, işlem kutusu
  (`processing.*`'ın 62'si Harita'da), raster ve nokta bulutu, katman süzgeci, çevirisi ve birleştirmesi, etiketler,
  topoloji, adres, Hesap, noktalar, formlar, öznitelik tablosu.
- Yalnız CBS'nin olanlar az: ağlar ve alt ağlar (`network.*`, `tool.net*`), zaman (`time.*`, `tool.time*`), senaryonun
  dördü, grafikler (`chart.*`), `mesh.add`, `raster.stac`, işlem kutusunun 65 aracı (araç kayıtlıdır; komut kümesi yalnız
  sunulup sunulmadığını söyler).

**Araştırma.** QGIS'in Python API belgeleri (`qgis.org/pyqgis`, 24 Kasım 2026'da okundu) kütüphaneleri ayırır: `core`
veri sağlayıcılarını, ifade motorunu, işlem çatısını, raster, nokta bulutu, mesh ve vektör katmanlarını, koordinat
sistemlerini, etiketlemeyi, coğrafi kodlamayı ve STAC'ı tutar; `gui` öznitelik tablosu ve form düzenleyicilerini, harita
araçlarını; `analysis` ağ ve yüzey çözümlemelerini; uygulama ve eklentiler bunların üstündedir.

**Karar.**

- `gis/` çekirdeğin klasörü olur (kabuk ve CAD çekirdeği gibi), adı ve modül adları değişmez; `module_deps.py`'de
  `CORE = (shell, cad, gis)`, modüller `cbs, arch, map, road`. Çekirdeğin klasörleri arasındaki bağlar da izlenir ve
  yalnız azalır (kabuk → gis 43, CAD → gis 30, gis → CAD 54); modüllere bağ izlenir (0 olmalı).
- CBS modülünün kendisi `cbs/`'dir: profili (`gis_profile`), Ağlar ve Alt ağlar. Kendi merkezi `cbs/state.rs`
  (`CbsUiState`, `cbs::Event`, `CbsDialogKind`; `CbsState` ile `modules.cbs()`), parçaları `cbs/parts.rs`.
- Ortak merkez (`gis/state.rs`: `App::gis`, `Message::Gis`, `Dialog::Gis`) çekirdekte kalır; ağ ve alt ağ kısmı CBS'nin
  merkezine geçti.
- Zaman ve grafikler komutlarının çoğu CBS'nin olsa da modülleri Harita'yla paylaşılır (senaryonun ikisi, bir grafik
  komutu); modül düzeyinde ortaktırlar, çekirdekte kalırlar. İkiye ayrılmaları sonraki iştir.

**Uygulama.**

- Taşınan: `networks/`, `subnetworks_window.rs`, `profile.rs` → `cbs/`.
- Çözülen bağlar:
  - İfade düzenleyicisinin `Target::Network`'ü genel `Target::Back`'e döndü (pencere metni kendi iletisiyle geri alır);
    ağ alanının düzenleyicisini açmak ağ penceresinin işi.
  - `command_tasks` (araçların istediği komutlar) çekirdeğe indi.
  - Ağ denetiminin işareti CBS'nin `marks()`'ıdır (topoloji bulgusu gibi çizilir, `marks::problem`).
  - Araçların ağ soruları `follow` kancasıyla yanıtlanır.
  - Başka çizim ekrana gelince ağlar unutulur: yeni `drawing_shown` kancası.
- `module_profile::profile` artık yalnız parçalara sorar; CBS'nin parçaları paketine ayrılana dek `IN_CORE`'da.
- Taban yeniden yazıldı (sınıflama değişti): kabuk → cbs 1 (kayıt dosyası), gis → cbs 0.
- Sınama: çekirdek 2503, Mimari 204, Harita 191, Yol 129, programlar 36.

### §4.13 Adım 13: CBS modülünün `App` yöntemleri uzantı trait'i

- `cbs/`'nin `impl App` bloklu dört dosyası (merkez, ağlar, Ağlar penceresi, Alt ağlar) dört trait oldu (betikle,
  gövdeler değişmeden); adlar `cbs/ext.rs`'te, dosyalar `use crate::cbs_ext::*` ile alır. Çekirdeğin hiçbir dosyası
  CBS'nin yöntemini çağırmıyor: önsöze başka yerde gerek olmadı.
- Sınama: çekirdek 2503.

### §4.14 Adım 14: CBS kendi paketinde

- `apps/desktop-cbs` (`kentos-desktop-cbs`): CBS modülünün altı modülü (merkez, parçalar, önsöz, profil, ağlar, Alt
  ağlar) ve ağların iç sınamaları; program `main`'de, programlar arası sınamalar `ctor`'la CBS'yi de kaydeder.
  `module_profile::IN_CORE` boş: dört modülün dördü de paketinde.
- CBS'nin kaydına dayanan çekirdek sınamaları taşındı: ortak CBS çekirdeğinin işlerini CBS projesinin alt sekmeleri ve
  panelleri (Adres, Grafik, İstatistik, Tablo, Topoloji, Arama, köşe tablosu) üzerinden sınayan 43 sınama pakete
  (`*_tab_tests.rs`, işleri çekirdeğin); Harita'nın GNSS arşivini dört modülde karşılaştıran sınama programlara.
- Çekirdeğin sınama kiti büyüdü: `data_layers_tests`, `chart_scenes`; tablonun okumaları, Öznitelik tablosunun
  sıfırlanması, işlem penceresinin araç adları `testing`'de de açık.
- Bağımlılık tabanı: kabuk → cbs 1 → 0; CBS'nin paketi yalnız çekirdeğe bağlı.
- Derleme: CBS dosyasında tek satır CBS'nin sınamalarını 3,3 s'de derler.
- Sınama: çekirdek 2446, CBS 57, Mimari 204, Harita 190, Yol 129, programlar 37. Toplam 3063, öncekiyle aynı; yok
  sayılan 762.

### §4.15 Adım 15: Modül programı yalnız kendi paketini kaydeder

- `kentos-cad`'in `main`'i başladığı adı (`kentos-mimari`, `kentos-harita`, `kentos-gis`, `kentos-yol`; ADR 0802'nin
  tek dosyaya bağlantıları) ya da `--modul`'ü okur: bir modülün programı yalnız o modülün paketini, `kentos-cad` dördünü
  de kaydeder. Eşleşme joker kolsuz: yeni modül paketi buraya girmeden derlenmez. `snapshot` ve `kullan` hepsini alır.
- Pencere açmadan doğrulandı: `kentos-mimari --modul harita` ve bilinmeyen modül nedenini söyleyerek 2 ile reddedilir.
  Programlar arası sınamalar dört modülü kaydeder (`ctor`).
- AY-07'nin teslimi böylece tamam: dört modül kendi paketinde, çekirdekte modül yok (§3.1: çekirdek artık “CAD çekirdeği”
  paketidir), hepsi bir arada program hepsini, modül programı yalnız kendininkini taşır; her adımda taban ve derleme
  süresi ölçüldü.

## §5 Kalan

**Yol paketinin çözümlemesi** (23 Kasım 2026, `a9f546a0e`'den sonra):

- Yol 46 modül, 31,5 bin satır, 46 dosyada `impl App`; başka modüle bağı yok. Çekirdekten ona dokunan: kabuğun `app`'i
  (ileti, pencere, durum), kayıt, izler, görünüm; gizli çağrılar: ~50 komut kimliği, dosya seçici sonuçları
  (`Message::LandxmlExport`, `LandxmlMachine`, `LandxmlAlignments`, `IfcExport`, `GltfExport`), görünüm alanında seçme
  (`terrain_picked`, `grade_picked`), işaretler (`grade_marks`: `cad/sub_object.rs` ve alan), panel ve başlığı, 3B
  penceresinin glTF dışa aktarımı, iz başlıkları ve denetimleri.
- **Ortak olan** (ortak katalogda Harita'nın komut kümesinde de): Arazi (`terrain.surfaces`, `terrain.new`,
  `terrain.sheetContours`, `terrain.stockpile`, `terrain.cloud`, `terrain.analysis`), `tool.grade`, `platform.editor`,
  `drainage.editor` ve proje ağacı (`Panel::RoadProject`, Harita'da “Proje ağacı”; `panel_title`'da modül adına göre
  dal var). Bunlar §3.2 gereği Yol ayrılmadan önce çekirdeğe (CAD) iner.
- **Yol'a özgü:** eksen, boykesit, enkesit ve şablon, dever, koridor ve şev planı, hacim, taşıma ve plan, aplikasyon,
  yol paftaları ve görünüşleri, çizgiden eksen ve boykesit, korkuluk, menfez, istinat, süpürme, görüş, platform suyu,
  hakediş, güzergâh, kavşak, LandXML/IFC/glTF.
- **Düğüm:** ortak pencerelerin kapanışı Yol'un neredeyse tamamını çeker (31,5 bin satırın 30,8 bini): hepsi Yol'un
  merkezine (`road`: `RoadUiState`, `road::Event`, `RoadDialogKind`) bağlı, merkez de her Yol penceresini sayar. Önce
  ortak pencerelere çekirdekte kendi merkezleri verilir (durum, ileti, pencere türü; Harita'daki düzenin çekirdek içi
  benzeri), Yol'un merkezine bağları kalkar; `road_project_panel`'in `project_graph_window` bağı da çözülür.

- Zaman ve grafiklerin yalnız CBS'nin olan yarısı (`time.*`, `chart.*`) modülleri ikiye ayrıldıkça CBS paketine geçer.
- **Çekirdeğin katmanları** (kabuk, CAD çekirdeği ve ortak CBS çekirdeği ayrı crate'ler; 24 Kasım 2026'da ölçüldü):
  kabuk 105 modül (198 bin satır), CAD 112 (146 bin), ortak CBS 81 (149 bin). Kabuk CAD'in 112 modülünden 91'ini
  adlandırır (kabuk → cad 77 dosya; `style` 12, `crs` 9, `labels`, `layering`, `marks`, `properties` 5'er modülden), ortak
  CBS'ye 43 dosyayla bağlıdır; CAD ile ortak CBS birbirine iki yönde bağlıdır (CAD → gis 30, gis → CAD 54). Crate'lere
  bölmek için `App`'in CAD ve CBS durumu, iletileri ve pencereleri modüllerinki gibi silinmiş türlere döner ve CAD ile
  ortak CBS arasındaki döngü kırılır (ya da ikisi bir crate kalır). Bu, modül paketlerinin toplamından büyük ayrı bir
  iştir. **Karar (24 Kasım 2026, sahip): şimdilik beklenir.** Gerekçe:
  - Hiçbir program bir katmanı bırakamaz: CBS'nin açıklamaları CAD çekirdeğinin, Harita ortak CBS altyapısını kullanır.
  - Derleme kazancı işin küçük bir payına düşer: son 400 masaüstü commit'inin 314'ü çekirdeğe, 206'sı kabuğa dokunur
    (kabuk en altta kalacağından kabuktaki değişiklik yine her şeyi derletir); yalnız CAD'e ya da ortak CBS'ye
    dokunan 108'dir.
  - Beklemek borcu büyütmez: `module_deps.py` bu bağları izler, yalnız azalırlar.
  - Yeniden ele alınma ölçütleri: çekirdekteki derleme süresi (tek satırda 14,7 s) işi yavaşlatırsa ya da bir katmanı
    taşımayan bir program istenirse. Derleme süresi hedef olursa önce kabuğun sık değişen bölümleri ölçülür.
- Modül programlarının ayrı ikilileri ve paketlemesi (isteğe bağlı): bugün tek dosya ve bağlantılar (ADR 0802), her
  program kendi paketini kaydeder (§4.15); ayrı ikili, dağıtım boyutu ölçülünce karar verilir.
