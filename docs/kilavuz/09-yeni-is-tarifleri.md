# KentOS CAD kılavuzu — §9 Yeni iş ekleme tarifleri

> [CLAUDE.md](../../CLAUDE.md)'nin 9. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §9…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 9. Yeni iş ekleme tarifleri

### 9.1 Komut

Registry/handler, enable/watch, gerekiyorsa alias/kısayol ve menü kaydı ekleyin;
aynı iş için UI ve otomasyonda ayrı iş kuralı yazmayın. Belgeyi değiştiren ürün komutu §9.2'deki bütünlükle gelir: katalog
ve yetenekler, ortak durumlar ve bağımsız başvuru, modüllerin kararı, Python SDK, MCP ve yapay zekâ ajanı, eklenti,
manifest.

### 9.2 Araç

Mevcut araç ailesini seçin; `tools/catalog.ts`, `steps`, preview/input/iptal,
kilitli katman ve tek undo testlerini tamamlayın. Hesabı ortak Rust'a ekleyin.

**Araç = komut (sahibin kararı, 13 Ekim 2026).** Çizimi ya da projeyi değiştiren her yeni araç işini kendi anlamsal ürün
komutuyla yapar; aynı işi Python ve yapay zekâ ajanı, geometriyi kendileri hesaplamadan, aynı komutla yapar. Bunlardan biri
eksikse araç bitmiş sayılmaz:

- **Anlamsal komut:** girdisi kullanıcının verdiğidir (nesnelerin kalıcı kimlikleri, yerleri, tıklanan noktalar, sayılar,
  seçenekler), sonucu komut hesaplar; `validate`, `plan`, `execute`, `CommandResult`, protokolün kaydı ve tekrar oynatması
  (ADR 0250, 0253, 0254, 0255). Hazır geometri alan `cad.entities.create` ve `.edit` işlemleri yalnız komutun iç yazma yoludur.
- **Araç komutu kullanır:** önizleme komutun planından ya da onun hesabından (`plan_in`, `measured_in` gibi), yazma komutun
  `execute`'undan; hesap araçta ikinci kez yazılmaz. Aracın görünümü, seçimi ve seçenekleri girdinin açık alanlarıdır (CMD-07).
- **Katalog:** tanım, girdi, çıktı ve plan şemaları, örnekler, `capabilities` (hedef türler, ret ve uyarı kodları; anlamları
  `codes.rs`'te); `DESKTOP_COMMANDS`, `registry.rs`; başsız sunucu kayıttan çalıştırır.
- **Ortak durumlar:** `fixtures/commands/v1/<komut>.json`, en az 20 durum, KentOS kodu olmadan yazılmış bağımsız başvurudan
  (`scripts/fixtures/<konu>_cases.py --check`); yetenek testi (`capabilities.rs`: her kod bir durumda ya da gerekçesiyle, tür
  denemeleri) geçer.
- **Modüller** (CLAUDE.md'nin modül ve otomasyon kuralı, 24 Kasım 2026): komut her modülün profilinde ve komut kümesinde
  ayrı ayrı var ya da yoktur (`apps/desktop-*/src/profile.rs`, `catalog/v1/modules.json`:
  `KENTOS_WRITE_MODULES=1 cargo test -p kentos-desktop --lib module_catalog`); matris sınaması `cargo test -p kentos-cad`
  geçer. Komutun modülü kayıttaki bölümüdür (`kentos_native_application::registry::discipline_of`).
- **Python:** `python3 scripts/python/sdk.py` ile `kentos.cad` yeniden üretilir ve `pnpm py:test` geçer (ortak durumlar
  tipli katmandan da).
- **MCP ve yapay zekâ ajanı:** ikisi aynı araç katmanından çalışır (`kentos_headless::agent_tools`, ADR 8056 §2, §4); araç
  katalogdan gelir, komut modülünün kapısından çalışır: `cad.run` her modülün çekirdeği; `arch.run`, `map.run`,
  `gis.run`, `road.run`. Var olanı değiştiren komut `agent_tools.rs`'in `DESTRUCTIVE` listesine yazılır; yazan komut
  önce `plan`, sonra aynı girdiyle `execute` edilir.
- **Eklenti:** eklentinin kodu komutu `run` ya da `call` ile çalıştırır; okuyan komut `document.read`, yazan
  `document.write` izni ister (ADR 8057; `apps/desktop/src/shell/extensions/runner.rs`). Başsız konakta yalnız
  `headless-python` bildiren eklenti çalışır. Geri al, yinele ve dosya işleri eklentiye kapalıdır (`not_for_extensions`).
  `kentos.ext` değişince eklenti SDK'sı yeniden üretilir: `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack`.
- **Manifest:** aracın düzeyi `catalog/v1/operations.json`'da `semantic`, `via`'sı komut; `python3 scripts/cad/manifest.py`
  (ADR 0244). Yeni araç `geometry` ya da `none` düzeyinde gelmez.
- **Kanıt:** izler değişmeden geçer; araç komutla çalışırken resimlenir (`kentos-cad kullan …`); aracın yazması kayıtta komutun
  adıyla görünür.
- **Karar tablosu:** işin ADR'si iki tablo taşır: modüller (Mimari, Harita, Yol, CBS: var ya da yok) ve kanallar (Python,
  MCP, yapay zekâ ajanı, eklenti: var ya da yok). Var satırı nasıl ulaşıldığını yazar (`kentos.cad.<ad>`, `<kapı>.run`,
  `run` ve izin); yok satırı gerekçesini. Var olan bir işe dokunan iş, o işin tablosunu da tamamlar.

Var olan `geometry` ve `none` düzeyindeki araçlar borçtur: `docs/cad/manifest.md` onları sayar; dokunulan araç bu kurala getirilir.

### 9.3 Ayar

Önce kapsamını belirleyin (§4.4); tip, varsayılan, validasyon, migration,
kalıcılık ve live-apply davranışını birlikte ekleyin. Ayar
`crates/shared/contracts/src/settings/schema.rs`'e eklenir, `KENTOS_WRITE_SETTINGS=1 cargo test
-p kentos-contracts settings` ile üretilir; yeni kural `fixtures/settings/v1` ve iki
çalıştırıcıyla birlikte değişir (ADR 0023).

### 9.4 Test

- Web davranışında `pnpm typecheck`, ilgili Vitest testleri ve kapsamına göre
  `pnpm test/build`; UI değişiminde tarayıcı e2e ve açık/koyu/Büyük yazı kontrolü. Web dururken (ADR 0228) bunlar koşmaz.
- Rust/hesap değişiminde ilgili native testler, clippy, WASM fixture/bağımsız
  referanslar; sözleşme değişiminde Rust → TS üretimi ve drift denetimi.
  Sözleşme tipi ürün komutu kataloğunun şemasını da değiştirir:
  `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands catalog` (ADR 0013).
- DB/cloud değişiminde gerçek PostGIS entegrasyonu ve `KENTOS_TEST_DB=required`;
  offline/retry/conflict/tenant/permission senaryoları. Atlananları açıklayın.
- Golden/visual fixture güncellemesi otomatik onay değildir; farkı okuyun,
  algoritmanın kendi çıktısını tek doğruluk kanıtı saymayın. Toleransı büyütmeyin.
- Yalnız doküman değişiminde bağlantı, komut, durum ve tutarlılık kontrolü
  yeterlidir; kod testleri çalışmadıysa çalışmış gibi raporlamayın.

### 9.5 Render backend

WebGPU/WGSL ve WebGL2/GLSL yollarını aynı sahneyle doğrulayın; renk/alpha,
dash, piksel ölçeği, AA ve device-loss davranışını koruyun. Headless SwiftShader
sonuçları gerçek GPU benchmark'ı değildir; mevcut `apps/web/scripts/e2e/cdp.mjs` ayarlarını kullanın.

### 9.6 Processing aracı

`processing/builtin/` içinde bildirim ve katalog kaydı; pure hesap testi ve
runner üzerinden belge testi ekleyin. Ayrıntı `docs/PROCESSING.md`'dedir.

### 9.7 Dosya biçimi

Reader/writer `crates/shared/formats`, şema `contracts`, dar binding
`crates/wasm/formats-wasm`, web akışı `io/` ve `app/fileExchange.ts` içindedir.
DXF (`crates/wasm/dxf-wasm`) ve Netcad NCZ (`crates/shared/ncz`, `crates/wasm/ncz-wasm`)
kendi modüllerindedir; biçim işçisi onları ilk gerektiğinde yükler, sonuçları tipli sütunlarla
geçer ve büyük içe aktarma iki platformda kare kare, tek geri alma adımında yazılır (ADR 0138).
NCZ okuyucusu GPL-2.0-or-later bir kaynaktan türetildi: yazarın izni gelene dek onu içeren
derleme dağıtılmaz (TODOS.md `NCZ-01`).
Bozuk/kötücül girdide panic yok; boyut, nesting/karmaşıklık ve iptal sınırı vardır.
CRS sorulur, kayıp raporlanır, import tek undo olur; Rust ve WASM aynı fixture'ı
okur. Ayrıntı ADR 0009. GeoJSON (okuma/yazma) ve Shapefile (okuma) aynı yoldadır (ADR 0046):
kurallar ADR'de, bağımsız okuyucu `tools/formats/gis.py`, fixture'lar `fixtures/formats/v1/gis`;
dosyanın dediği koordinat sistemi gösterilir, projeninkinden başkaysa içe aktarma kapalıdır,
dönüşüm yoktur. Proje dosyası değişim biçimi değildir: kodek `crates/shared/kcad`,
spesifikasyon `docs/specs/kcad-v2.md`, karar ADR 0025; değişiklik spesifikasyon,
`fixtures/kcad/v2` (bağımsız Python yazıcısıyla), Rust kodeği ve `tools/kcad/kcad.py`
ile birlikte yapılır.

### 9.8 Ayrı işletim sistemi penceresi

Uygulamadaki her pencere, işletim sisteminin ayrı penceresine alınabilir. Kullanıcı onu başka bir ekrana taşır ve
çalışırken çizimin değiştiğini görür. Destek uygulama genelidir
([ADR 8060](../adr/8060-secondary-os-windows.md) §10, `apps/desktop/src/shell/windows/`). Ortak işler bir kez yazılmıştır:

- pencereyi ayırıp geri getirmek, seçimi hatırlamak (`ayri-pencereler.json`);
- ana pencerede başka bir pencere açıkken geri getirmeyi nedeniyle reddetmek;
- başlıktaki düğme ve ipuçları;
- yerleşim: pencereyi doldurmak ya da çizimin üstünde durmak;
- Esc ve Tab;
- izin yanıtları ve tuşların ana pencereye sızmaması.

Bir pencereyi katmak:

1. **Sözleşme:** pencerenin kendi modülünde bir birim yapı (`pub(crate) struct …Window;`) için
   `crate::windows::OwnWindow` yazılır:
   - `kept_as`: seçimin anahtarı; bir kez yayımlandıktan sonra değişmez.
   - `sizes`: ilk ve en küçük boy, 12 px yazıda.
   - `what`: iletideki adı, belirtme hâlinde (“3B görünümü”).
   - `apart_tip` ve `return_tip`: iki düğmenin açıklaması.
   - `dialog`: çizimin üstündeki `Dialog` türü.
   - `is_open`, `title`, `view` ve `close`.
   - Gerekirse: `opens_apart` (kendiliğinden ayrı açılması), `has_dialog`, `work_in_main` (çizimden seçerken klavye
     ana pencereye geçer), `on_key`, `trace_title` ve `trace_control`.
2. **Ad:** `windows::Surface`'a bir değişken eklenir ve `Surface::spec` onu sözleşmeye bağlar. `windows/app.rs`
   değişmez.
3. **Açılış ve kapanış:** pencerenin durumu kurulunca `self.show_surface(Surface::…)` çağrılır. Kapanınca
   `self.surface_away(Surface::…)` çağrılır. `self.dialog` elle yazılmaz.
4. **Çizim:** pencerenin `Dialog` kutusu `crate::windows::placed(kutu, Surface::…, ayrı_mı, kapat_iletisi)` ile
   verilir. Böylece başlıktaki düğme ve iki yerleşim birlikte gelir. Başlığı olmayan bir pencere
   `windows::return_button` kullanır.
5. **Durum:** pencere ayrıyken ana pencere serbesttir. Pencerenin gösterdiği şey çizimle birlikte değişiyorsa her
   mesajdan sonra izlenir (örnek: `App::follow_scene3d`).
6. **Sınama:** pencerenin ayrılması ve geri gelmesi sınanır. Başka bir pencere açıkken geri gelmenin reddi, ayrı
   penceredeki Esc ve kapatma düğmesi, izin yanıtı ve seçimin hatırlanması da sınanır. Örnekler
   `gis/processing/apart_tests.rs` ve `tests/scene3d_window_tests.rs`'tedir. Sözleşmenin kendi sınaması
   (`windows::tests::every_surface_fulfils_the_contract_with_a_name_of_its_own`) yeni pencereyi kendiliğinden kapsar.

### 9.9 Yeni modül ve yeni istemci

CLAUDE.md §25'in sekizinci kuralının tarifidir. Kararın gerekçesi [ADR 9195](../adr/9195-module-separation.md) ve
[ADR 9196](../adr/9196-storage-by-project-kind.md), işin sırası [plan/ayrim.md](../../plan/ayrim.md)'dedir.
Kayıt `apps/desktop/src/shell/module_profile.rs`'tedir (`ModuleProfile`, `profile`); kendi paketindeki modül profilini
ve kabuğun ona dokunduğu her yeri modül arayüzüyle verir (`apps/desktop/src/shell/module_parts.rs`, `ModuleParts`, ADR
9199). Bağımlılık denetimi `scripts/dev/module_deps.py --check`, ortak katalog `catalog/v1/modules.json`'dur; ikisini de
masaüstü sınamaları koşar (`module_catalog_tests`).

**Yeni yüzey** (panel, alt sekme, durum hücresi, ayar bölümü, katman menüsü öğesi):

1. Türün enum'una değer eklenir; program dört modülün profilinde ve `NoProject`'te yerleşmeden derlenmez.
2. Katman menüsünün öğesi ayrıca katman türleriyle yerleşir (`LayerEntry::kinds`, `apps/desktop/src/shell/layer_entries.rs`)
   ve matris sınamasının tablosuna yazılır (`layer_menu_tests`).
3. Şeritte karşılığı olan öğe modülün komut kümesini izler; sınama bunu denetler.

**Yeni modül** (ör. altyapı, kentsel dönüşüm):

1. **Modül tanımı:** `crates/native/launch/src/module.rs`'e modül, proje türleri, CAD işi, program adı, saklama türü
   (`Module::storage`) ve katman türleri (`Module::layer_kinds`).
   - Saklama türü ya **CBS** (`Storage::Database`: veri veritabanında, proje dosyası yapı, stil, ayar ve açıklama
     tutar) ya da **CAD** (`Storage::File`: projenin tamamı proje dosyasında) olur; üçüncüsü ADR ister.
   - Sunucu proje türünün saklama kuralını buradan alır.
   - Ortak katalog yeniden yazılır: `KENTOS_WRITE_MODULES=1 cargo test -p kentos-desktop --lib module_catalog`.
2. **Profil:** modülün kendi klasöründe (`apps/desktop/src/<modül>/profile.rs`), `ModuleProfile`'ı uygulayan tek bir
   tür; kayda (`module_profile.rs`'in `profile`'ı) bir satır.
   - Paneller, alt sekmeler, durum hücreleri, proje ve uygulama ayarları, katman menüsü, Kaynaklar ve Öznitelikler
     satırları joker kolsuz eşleşmedir: her değer için var ya da yok.
   - Biçimler, pafta ve bulut türleri, araç seçenekleri, tematik çiziciler ve kitaplık dalları da profildedir.
   - Kabuğa modülün adı yazılmaz.
3. **Sihirbaz:** modülün yeni projesi neyi sorar, neyle açılır; modülün programı yalnız kendi türünü sunar.
4. **Komut kümesi:** şeridin verisinde (`scripts/ui/ribbon.py`) komutların `works`'ü; modüle özgü komutların
   çekirdekteki disiplini (`kentos_native_application::registry::discipline_of`).
5. **Bulut:** proje türleri ve saklama biçimi. CBS türü veritabanı, CAD türü dosya revizyonudur.
6. **Matris sınaması:** `apps/kentos-cad/tests/programs/module_surface_tests.rs`'in matrisi yeni modülün her yüzeyini
   sınar. Hepsi bir arada programda o türün yüzeyleri modülün programındakiyle aynıdır.
7. **Bağımlılık:** modül yalnız ortak çekirdeğe ve sözleşmeye bağlanır. `scripts/dev/module_deps.py`'nin `FOLDERS` ve
   `TRACKED` listelerine modülün klasörü girer; `--check` yeşil kalır, taban yalnız azalır.
   - **Kendi paketi** (ADR 9199, örneği `apps/desktop-map`): `apps/desktop-<modül>` (`kentos-desktop-<modül>`) yalnız
     çekirdeğe bağlanır; `ModuleParts`'ı uygular, `register()`'ı programların `main`'inde çağrılır, sınamalarında
     `ctor` ile çağrılır. Durumu `App.modules`'te, iletisi `Message::Module`, penceresi `Dialog::Module`'dür;
     `impl App`'leri uzantı trait'idir (`ext.rs`). `module_deps.py`'nin `PACKAGES`'ına girer. Programlar arası
     sınamalar `apps/kentos-cad/tests/programs`'tadır. Sınamaları çekirdeğin sınama kitini kullanır
     (`kentos-desktop`'un `testing` özelliği: `files_testing`, `tools_scenes`, `tools_screens::take` …); modülün
     resimleri kendi `screens` toplayıcısındadır (örneği `apps/desktop-arch/src/tests/arch_screens.rs`).
8. **Resim:** modülün penceresi, her katman türünün menüsü ve sihirbazı.

**Yeni istemci** (ör. mobil görüntüleyici ya da düzenleyici; karar [ADR 9197](../adr/9197-mobile-clients.md)):

1. Modülleri, proje türlerini, saklama türünü, katman türlerini ve komut kümesini ortak katalogdan
   (`catalog/v1/modules.json`) okur; modül mantığını kendi içinde yeniden yazmaz.
   - İstemci ortak tanıma girer (`kentos_launch::Client`): neyi yazdığını saklama türüne göre söyler (`writes`).
   - Hangi katmanı yazdığı katman türünden çıkar (`LayerKind::written_by`); istemci bunu yeniden yazmaz.
   - Katalog yeniden üretilir; masaüstünün `module_catalog_tests`'i istemcileri ortak tanımla karşılaştırır.
2. Projeyi sözleşmeyle açar: bulut API'si ya da `.kcad`. CBS verisini veri katmanının kaynağından, rolün okuduğu
   biçimde alır (sunucuda `form_guard`, istemcide `kentos_forms::reach`).
3. **Görüntüleyici** yalnız okur. Rolün okumadığı alanı göstermez, adını söyler.
4. **Düzenleyici** her değişikliği komutla yazar: CBS verisini düzenleme oturumuyla ya da sunucu komutuyla (formun
   kuralları ve kapsamı), CAD projesini proje dosyasının revizyonuyla. Kuralı atlayan doğrudan yazma yoktur.
5. Ekranı açık projenin modülü belirler; istemci bir modülün yüzeyini başka modülün projesinde göstermez.
