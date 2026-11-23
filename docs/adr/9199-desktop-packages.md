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

## §5 Kalan

- Yol, Mimari ve CBS paketleri (en az bağlıdan); her biri aynı düzenle: ortak parçalar çekirdeğe, `ModuleParts`,
  silinen türler, uzantı trait'leri, paket, sınamaların yeri.
- CAD çekirdeğinin kabuktan ayrılması (§3.1): kabuk → cad 77, CBS → cad 54 dosya.
- Modül programlarının ayrı ikilileri ve paketlemesi: bir modülün programı yalnız kendi paketini kaydeder.
