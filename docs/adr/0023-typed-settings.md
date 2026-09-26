# ADR 0023: Tipli ayarlar, katmanlı çözüm ve cihazın kenar yumuşatması

- **Durum:** kabul edildi (2026-09-26). Yön TODOS.md §7 (`SET-01..05`), §8.2 (`AA-01`, `AA-02`) ve §22.1'in 7. maddesindendir. Şemanın yeri ve biçimi, kapsam kuralları, hata kodları, göçler, masaüstünün ayar dosyası ve kenar yumuşatmanın iki platformdaki yolu bu dilimin kararıdır. Sahibe sorulanlar sonda.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md `SET-01..10`, `AA-01..06`, `REN-02`; CLAUDE.md §4.4, §9.3, §4.9, §9.5; ADR 0010 (platform sınırı), 0017 (masaüstü kabuğu), 0019 (wgpu çizim alanı), 0021 (araç oturumu)

## Bağlam

- **Web:** tercihler `localStorage kentos.prefs.v1`'de, `persistedSignals` ile türsüz bir nesne olarak duruyordu. Tür, aralık ve kapsam denetimi yoktu; bozuk bir değer olduğu gibi okunuyordu.
  - Çizim kalitesi tek bir değerdi (`renderQuality`: `high`, `balanced`, `fast`). Kenar yumuşatma değişince arka uç yeni tuval ve bağlamla baştan kuruluyordu.
  - Orto, kutupsal izleme, kenet, ızgara ve izleme oturum sinyalleriydi (`DraftingSettings`).
- **Masaüstü:** kalıcı ayar yoktu. Araç oturumu web'in varsayılanlarıyla çalışıyordu (`Draft::default()`, `cursor_input: true`). Tema her açılışta koyuydu.
  - Aynı yapılandırma klasöründe KentOS UI vitrininin kendi düz metin dosyası vardı: `~/.config/kentos-cad/ayarlar`.
- **Çizim alanı:** ADR 0019'a göre Iced'in karesi tek örneklidir ve MSAA yoktu. Kendi hedefi ve resolve'uyla MSAA `AA-01..03`'e bırakılmıştı.

## Karar

### Tek şema (`SET-01`)

- Şema ortak sözleşmelerdedir: `crates/shared/contracts/src/settings/`.
  - `schema.rs` ayarların listesidir, tek kaynaktır.
  - `rules.rs` doğrulama, katman kuralı, çözüm, saklanan belge ve hazır ayarlardır.
  - Tipler ts-rs ile TypeScript'e (`apps/web/src/contracts/generated/Setting*.ts`), schemars ile JSON Schema'ya çıkar.
- **Her ayarın taşıdıkları:**
  - sabit anahtar: `grup.ad` (`graphics.msaa`), camelCase ASCII; bir kez verilen anahtar başka anlama verilmez;
  - tür: `boolean`, `integer`, `number` ya da `enum`;
  - varsayılan; aralık (`min`, `max`) ya da seçenekler (Türkçe etiketleriyle); birim (`px`, `deg`, `sample`);
  - kapsam: `default`, `organization`, `user`, `device`, `project`, `session`;
  - onu kullanan uygulamalar (`hosts`: `web`, `desktop`);
  - sürüm; Türkçe başlık ve açıklama; hassas bayrağı; uygulanış biçimi (`live`, `recreate`, `restart`).
- **Şemada ayrıca:** gruplar, hazır ayarlar, her hata kodunun ve her farkın Türkçe iletisi. İki uygulama aynı durumda aynı cümleyi söyler.
- **Web şemayı okur, masaüstü çağırır:**
  - Web `settingsSchema.json`'u okur. Dosya `cargo test -p kentos-contracts` ile karşılaştırılır; bilinçli değişiklikten sonra `KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings` yeniden yazar.
  - Saklanan belgenin JSON Schema'sı `settingsFile.schema.json`'dadır; dış araçlar (ileride Python) dosyayı onunla denetleyebilir.
  - Masaüstü `settings_schema()`'yı çağırır.
- **Bu sürümde 41 ayar:**
  - Web'in bütün tercihleri (25 `user`).
  - Oturum yardımcıları (5 `session`): orto, kutupsal, kenet, ızgara, izleme.
  - Grafik (3 `device`): `graphics.msaa`, `graphics.hiDpi`, `graphics.backend`.
  - Projenin sekiz ayarı (`project`). Bunlar `ProjectSettings`'ten okunur; `.kcad` okuyucusu otorite olmaya devam eder, bu dilimde proje dosyası tipli doğrulamadan geçmez.
  - Masaüstünün teması (`appearance.theme`, yalnız masaüstü).
  - Masaüstünün kullandıkları 16 ayar: çizim yardımları, grafik, tema ve proje ayarları.
- **Kapsam kararları:**
  - Web'in bugünkü kapsamı korundu. `kentos.prefs.v1`'dekiler `user`, durum çubuğu anahtarları `session`, proje ayarları `project`.
  - Orto ve kutupsal izleme her oturumda kapalı başlar. Açı adımı, kenet yarıçapı ve imleç yanında değer girişi kullanıcının tercihidir.
  - Tek değişiklik: **grafik ayarları (`graphics.msaa`, `graphics.hiDpi`, `graphics.backend`) `user`'dan `device`'a geçti.** Neden TODOS.md §7'dir: GPU arka ucu ve MSAA yalnız o cihazındır; başka cihazdan dosya ya da hesapla gelmemelidir. Web'de iki katman da aynı `localStorage`'dadır; ayrım dışa aktarılan dosyada ve çözümde görünür.
  - Web'in teması yerleşim deposunda (`kentos.ui.v1`) kaldı; görünüm ayarlarının göçü `SET-06/09`'la yapılır.

### Katmanlar ve çözüm (`SET-02`, `SET-03`)

- **Sıra:** varsayılan → kullanıcı → cihaz → oturum. En özel katmandaki geçerli değer **istenen** değerdir (`requested`); kaynağı da söylenir (`source`).
- **Hangi katman neyi tutar:** ayar kendi katmanında ya da daha özel bir katmanda durabilir.

  | Ayarın kapsamı | Tutabilen katman |
  |---|---|
  | `user` | `user`, `device`, `session` |
  | `device` | `device`, `session` (`?renderer=webgpu` bir oturum geçersiz kılmasıdır) |
  | `session` | yalnız `session`; hiçbir dosyaya yazılmaz |
  | `project` | yalnız proje |
  | `default`, `organization` | hiçbir katman |

- **Proje ayarını tercih ezemez.** Tercihte bir proje ayarı `wrong_scope` tanısıyla atlanır. Projede bir cihaz ayarı da öyle atlanır: başka cihazın MSAA'sı projeyle gelmez; proje başka cihazda açılınca CRS ve çizim yazı tipi projeninkidir.
- **Kurum politikası** bir üst sınırdır:
  - kilit (`value`); alt ve üst sınır (`min`, `max`); izinli değerler (`allowed`);
  - sınır, seçenekli sayıda en yakın alttaki seçeneğe, tam sayıda sınıra doğru tam sayıya iner.
  - Politikayı gönderen bir sunucu henüz yok. Açık bir giriştir:
    - web'de `settingsStore.setPolicy`;
    - masaüstünde `KENTOS_SETTINGS_POLICY=<dosya.json>`, “yerel deneme” diye söylenir.
  - Geçersiz kural (`organization` katmanında tanı) yok sayılır.
- **Cihaz kısıtı** en sondadır: cihazın kullanabildiği değerler, nedeni (`device_unsupported` ya da `device_failed`) ve Türkçe ayrıntı. İstenen değer izinli değilse en yakın alttaki kullanılır (sayılarda), ya da ilk izinli değer.
- **Etkin değer** (`effective`) kullanılan değerdir; istenenden farklıysa nedeni ve ayrıntısı vardır. **İstenen değer kaybolmaz:** aygıt 4×'e kadarsa 8× istenir, 4× çizilir, pencere “İstenen 8×, kullanılan 4×” der. Başka aygıtta yeniden 8× denenir.
- Geçersiz değer, yanlış katmandaki değer ve bilinmeyen anahtar tanı olarak raporlanır ve atlanır; alttaki katman karar verir.

### Doğrulama ve ortak durumlar

- **Hata kodları:**

  | Kod | Anlamı |
  |---|---|
  | `unknown_key` | böyle bir ayar yok |
  | `wrong_type` | tür yanlış |
  | `not_integer` | tam sayı değil |
  | `out_of_range` | aralık dışı |
  | `not_allowed` | seçeneklerden biri değil |
  | `wrong_scope` | bu katmanda durmaz |
  | `sensitive` | gizli değer dosyada |
  | `not_json` | JSON değil |
  | `not_settings` | ayar belgesi değil |
  | `unsupported_version` | okunamayan sürüm |

- Denetim sırası sabittir: anahtar, katman, tür, tam sayı, aralık, seçenekler. İki yönden yanlış bir değer her yerde aynı kodu alır (örneğin 3,5 px kenet yarıçapı `not_integer`'dır, `out_of_range` değil).
- Sayılar normalleşir: `45.0` ile `45` aynıdır. Tam sayı kesirsiz yazılır, çünkü JavaScript ikisini ayırmaz.
- **Ortak dosya:** `fixtures/settings/v1/cases.json`. Değerler 33, yazma 16, çözüm 26, belge 17, hazır ayar 7 durum. Web (`core/settings/settings.test.ts`) ve Rust (`contracts/tests/settings.rs`) aynı dosyayı çalıştırır. Tanılar sırasız karşılaştırılır: bir katmanın anahtar sırası uygulamasınındır.

### Saklanan belge ve göçler (`SET-04`)

- **Belge** `kentos.settings` sürüm 1'dir: `user`, `device` ve `migrations` (nereden, ne zaman, kaç değer, kalanlar ve nedenleri).
  - Oturum değeri yazılmaz, proje değeri projededir, hassas değer asla yazılmaz ve okunmaz.
  - Bilinmeyen anahtar (yeni bir sürümden) olduğu gibi saklanır ve raporlanır: kaydetmek onu kaybettirmez.
  - Başka sürüm reddedilir, tahmin edilmez.
  - Web'in ve masaüstünün yazdığı metin aynı düzendedir: sıralı anahtarlar, iki boşluk girinti.
- **Web** (`app/settings/`):
  - Anahtar `localStorage kentos.settings.v1`'dir.
  - Belge yoksa `kentos.prefs.v1` **bir kez** taşınır:
    - her alan kendi ayarının katmanına gider (grafik cihaza);
    - `renderQuality` iki ayara bölünür (`high` → 4×, tam çözünürlük; `balanced` → kapalı, tam çözünürlük; `fast` → kapalı, düşük çözünürlük);
    - kuralların reddettiği değer taşınmaz ve kayıtta adıyla durur.
  - **Eski anahtara dokunulmaz;** yedek odur ve uygulama ona bir daha yazmaz.
  - Okunamayan belge, ya da reddedilen değer içeren belge, üstüne yazılmadan önce bütün metniyle `kentos.settings.v1.backup`'a kopyalanır. Okunamayan belgede ayarlar eski kayıttan (yoksa varsayılanlardan) yeniden kurulur. Yedek yazılamazsa hiçbir şey yazılmaz, ayarlar bellekte kalır.
  - Tarayıcı depolamaya izin vermezse ayarlar bellektedir; uygulama bunu ve nasıl düzeltileceğini söyler.
  - Yazma 250 ms sonra olur; sayfa kapanırken (`pagehide`) hemen.
  - `ctx.prefs` bir cephedir: her tercih için bir sinyal, **etkin** değeriyle. Sinyali değiştirmek tercihi yazar. Kuralların reddettiği değer alınmaz, sinyal kullanılan değere döner.
- **Masaüstü** (`apps/desktop/src/settings.rs`):
  - Dosya `$XDG_CONFIG_HOME/kentos-cad/ayarlar.json`'dır (yoksa `~/.config/kentos-cad/ayarlar.json`). Aynı `kentos.settings` belgesidir: web'den dışa aktarılan dosya masaüstünde açılır, tersi de.
  - **Vitrinin biçimi yeniden kullanılmadı.** Nedenleri:
    - iki uygulamanın aynı dosyayı yazması, birinin öbürünün anahtarlarını silmesi demektir (vitrin dosyayı bildiği anahtarlarla baştan yazar);
    - düz metin sürümsüz ve türsüzdür;
    - web'le ortak belge ve ortak doğrulama tek biçimle olur.
  - Vitrinin `ayarlar`'ı yalnız okunur, hiç yazılmaz. Masaüstünün dosyası yokken iki uygulamanın ortak tek ayarı, tema (`tema = koyu | aydinlik`), **bir kez** alınır. `gece` ve `karsitlik` masaüstünde yoktur; kayıtta adıyla kalır.
  - Okunamayan ya da reddedilen değer içeren dosya `ayarlar-yedek-<zaman>.json` olarak kopyalanır. Yazma geçici dosya ve tek yeniden adlandırmayla yapılır; çökme yarım dosya bırakmaz.
  - `App::boot` bellektedir: testler, görüntüler ve iz oynatıcısı kullanıcının dosyalarına dokunmaz. Gerçek dosyayı yalnız `main` açar.
- **Sıfırlama ve dışa/içe aktarma** iki uygulamanın penceresindedir. İçe aktarılan dosyanın değerleri taslağa gelir; Kaydet saklananların yerine koyar. Reddedilen dosya hiçbir şeyi değiştirmez; alınmayan değerler adıyla söylenir.

### Hazır ayarlar (`SET-05`)

- **Hızlı:** kapalı, düşük çözünürlük. **Dengeli:** kapalı, tam çözünürlük. **Kaliteli:** 4×, tam çözünürlük.
- Hazır ayar hiçbir yerde saklanmaz, yalnız iki değeri doldurur; her biri ayrıca değiştirilir. Değerler hiçbirine uymuyorsa pencere “Özel” der.
- Grafik dışında bir şeyi değiştirmez. “Hızlı” daha az hassas bir kayıt üretmez (TODOS.md §8.2 kabulü).

### Kenar yumuşatma (`AA-01`, `AA-02`)

- **Web WebGL2:**
  - Sayılar bağlamın kendi listesidir: `getInternalformatParameter(RENDERBUFFER, RGBA8, SAMPLES)`, `MAX_SAMPLES` içinde.
  - `setSamples` çizim hedefini sonraki karede yeni sayıyla kurar. Tuval, bağlam ve yüklenmiş katmanlar aynı kalır.
  - GL eski hedefi, kuyruktaki komutlar onu okumayı bitirince bırakır.
  - Kurulamayan sayı (framebuffer eksik ya da bellek yetmedi) son çalışan sayıya döner ve `onSamplesFailed`'le bildirilir.
- **Web WebGPU:**
  - Sayılar cihaza sorulur: her aday 1 × 1 doku olarak bir doğrulama kapsamında kurulur. Tarayıcı WebGPU'da 1 ve 4'ü doğrular.
  - Boru hatları sayı başına kurulur ve saklanır; stil çizicisi de öyle.
  - Değişen çok örnekli hedef, gönderilmiş iş bitince (`onSubmittedWorkDone`) yok edilir.
  - Kurulum hataları hata kapsamında toplanır; hata varsa son çalışan sayıya dönülür.
- **Web'in görünümü:**
  - Arka uç başlayınca sayılarını ayarlara cihaz kısıtı olarak verir. WebGPU'suz tarayıcıda, ya da WebGPU başlayamazsa, arka uç kısıtı olur.
  - Etkin değer değişince `setSamples` çağrılır, yalnız hedefler kurulur. HiDPI yeniden boyutlandırmadır.
  - Tuvali yeniden boyutlandırınca siyah kareyi önleyen eş zamanlı çizim korunur (CLAUDE.md §4.9).
- **Masaüstü** (`crates/render/wgpu/src/targets.rs`, `renderer.rs`):
  - Sayılar Iced'in aygıtına aynı yolla sorulur. Iced aygıtı `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` istemediği için WebGPU'nun güvencesi geçerlidir: **1 ve 4**. Aynı Iris Xe bu özellikle 1, 2, 4, 8 ve 16 örnek alır (wgpu hata iletisi).
  - Tek örnekli, tam çözünürlükte alan eskisi gibi Iced'in geçişine çizer.
  - Çok örnekliyken, ya da yoğun ekranda HiDPI kapalıyken, alan kendi hedefine çizer:
    - çok örnekli renk, aynı boyda bir resme resolve edilir;
    - resim tam ekran bir üçgenle Iced'in karesine, alanın viewport'u ve scissor'ı içinde basılır (`Primitive::render`). GPU'dan GPU'ya, geri okuma yoktur; `REN-02` korunur. ADR 0019'un “MSAA yok” maddesinin yerine budur.
    - HiDPI kapalıyken resim mantıksal boydadır ve yumuşak örnekleyiciyle büyütülür.
  - Boru hatları sayı başına kurulur ve saklanır.
  - Hedefler boyut ya da sayı değişince yeniden kurulur. Eskisi bırakılır; wgpu onu kullanan iş bitene dek yaşatır.
  - Hedef ya da boru hattı kurulamazsa görünüm son çalışan sayıyla çizer. Hata saklanır; uygulama ayarlara `device_failed` kısıtı verir ve bir kez uyarır.
  - Çizgi ve işaretlerin gölgelendiricide yumuşatılması sürer. MSAA bugün çerçevesiz dolguların kenarını yumuşatır.
  - Örnek çizimde 1× ile 4× piksel piksel aynı çıktı: her dolgunun kenarını kendi çerçevesi örtüyor. GPU testi çerçevesiz, döndürülmüş karede fark ölçtü: 0'a karşı 120 kenar pikseli.
  - Hedef belleği pencerede gösterilir (`FrameStats::target_bytes`).

### Arayüz

- **Web — Uygulama ayarları → Çizim motoru:**
  - arka uç; hazır ayar (Hızlı, Dengeli, Kaliteli, Özel); MSAA; HiDPI;
  - her birinde istenen ve kullanılan değer, fark varsa nedeni; yaklaşık hedef belleği.
- **Web — yeni “Ayar dosyası” bölümü:** dışa aktar, içe aktar, varsayılanlara döndür, kaydın yeri, göç kaydı, kurtarma notu.
  - Kendi değeri olmayan bölümde “Bu bölümü varsayılana döndür” devre dışıdır; sessiz düğme yoktur.
  - Pencere taslakla çalışır, Kaydet'te uygular (CLAUDE.md §4.4).
- **Masaüstü — `tools.options` (Ctrl+,):**
  - KentOS UI bileşenleriyle iki sütun: çizim yardımcıları ve tema; grafik ve ayar dosyası.
  - Taslakla çalışır. Vazgeç, Esc ve × hiçbir şeyi değiştirmez; Kaydet araç oturumuna (`Draft`, değer alanı) ve çizim alanına hemen ulaşır.
  - Kurumun kilitlediği anahtar kullanılan değeri gösterir ve çevrilemez.
- **Masaüstüne taşınan komutlar:**
  - `draft.ortho` (F8) ve `draft.polar` (F10). Web'in ifadesiyle söylenir: “Orto açık”.
  - Tema komutları temayı kalıcı tercih olarak yazar.
- **Masaüstü görüntü seçenekleri:** `kentos-cad snapshot` `--ayar anahtar=değer` alır (bellekte, dosyaya yazmaz). Görüntü alınmadan önce bir kare çizilir ki pencere cihazın sayılarını göstersin.

## Sonuçlar

- **Bağımlılık:** yeni paket ya da crate yok. Masaüstü zaten bağımlı olduğu `serde_json`'u kullanır. Tarih yazımı için crate eklenmedi: UTC takvim hesabı birkaç satırdır.
- **Başlangıç yükü:** web şemayı başlangıçta okur (`settingsSchema.json`, 26 KB ham, `?raw`). Tercihler açılışta gereklidir.
- **`render/quality.ts` kalktı.** Kenar yumuşatma artık arka ucu yeniden kurmaz.
- **Görsel test** (`e2e:visual`) tercihleri eski `kentos.prefs.v1` anahtarıyla verir; her açılış göçten geçer. Bu da göçün sürekli bir sınamasıdır.
- **Envanter:**
  - tercihlerin kapsamı ve varsayılanı şemadan gelir; tercih deposu `kentos.settings.v1`'dir;
  - depolar listesinde tipli depo, yedeği ve eski anahtar vardır;
  - masaüstü sütunu şemanın `hosts`'unu okur.
- **Masaüstü çizim alanı** varsayılan olarak 4× çizer (web'le aynı varsayılan). Alan kadar ek bellek tutar (1440 × 900 pencerede yaklaşık 14 MB) ve ek bir birleştirme geçişi yapar.

## Ertelenenler

- `SET-06..10`: bütün CAD ve performans ayarları, arama, miras ve kilit işaretleri, Python/AI erişimi, ayar envanteri.
- Web'in görünüm ayarlarının (tema ve yerleşim) tipli depoya taşınması; hesap düzeyinde eşitleme; kurum politikasının sunucudan gelmesi.
- Masaüstünde 2×, 8× ve 16×. Iced'in aygıtı `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` istemeli; Iced 0.14 bunu dışarıdan açtırmıyor (sahibe soru).
- `AA-03`: analitik çizgi yumuşatma, MSAA ve alfa birleşiminin sözleşmesi. `AA-04`: görüntü seti. `AA-05`: hedef belleğinin önceden hesaplanıp sınırlanması; bugün yalnız gösterilir ve hata son çalışan ayara döndürür. `AA-06`.
- Proje ayarlarının tipli doğrulamadan geçmesi: `.kcad` okuyucusu otoritedir.

## Doğrulama (26 Eylül 2026, Intel i5-11300H, Iris Xe, Linux; `worktree-agent-ad8a6b6e703bf1403`)

- `cargo fmt --all --check` temiz.
- `pnpm rust:test`: 460 test geçti; clippy temiz; bağımlılık yönü temiz (17 crate, 22 crate × hedef). Sözleşmelerde 10 ayar testi var, 99 ortak durum. Veritabanı testleri `KENTOS_TEST_DB=required` ile yerel PostGIS'te koştu (50 test).
- `pnpm rust:test:desktop`: 293 test geçti, clippy temiz. Masaüstü 40 test: ayar dosyası 8, pencere 5, izler 3.
- `KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu` (Iris Xe, Vulkan, Mesa 26.0.8):
  - sayılar [1, 4];
  - aynı çizicide 1× ↔ 4× geçişi yeni yükleme yapmadan (yalnız 64 baytlık uniform);
  - 4×'te çerçevesiz dolgunun kenarında 120 karışık piksel, 1×'te 0;
  - zorla 64× istenince 4×'e dönüş ve saklanan neden;
  - HiDPI kapalıyken dörtte bir piksel;
  - REN-07 ölçüsü değişmedi: 0,002 px.
- `pnpm typecheck` temiz. `pnpm test`: 1009 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm e2e` (ekransız Chrome):
  - WebGL2 ANGLE/Vulkan ile gerçek GPU'da, sayıları [1, 2, 4, 8, 16]: 8× istendi, 8× çizdi.
  - WebGPU SwiftShader'da, sayıları [1, 4]: 8× istendi, 4× çizdi; pencere nedeni söyledi.
  - İkisinde de aynı tuval kaldı. Zorlanan hata son çalışan sayıya döndü. SwiftShader sonuçları GPU ölçümü değildir.
- `pnpm e2e:interaction`: 4 iz × 3 varyant geçti. `cargo test -p kentos-desktop traces`: geçti.
- `pnpm inventory:check` güncel. `node scripts/wgsl/browser-check.mjs` geçti.
- **Ters deneme:**
  - web göçü kenet yarıçapını atınca 4 test düştü (beklenen 14, gelen 11);
  - masaüstü göçü temayı atınca 1 test düştü.
  - İkisi de geri alındı.
- **Görüntüler:** `kentos-cad snapshot --ayar graphics.msaa=8 --komut tools.options`, koyu ve açık temada; web pencereleri `pnpm e2e` ile.

## Ek (26 Eylül 2026): kalite ayarı yalnız çizimi etkiler

- **Sahibin isteği:** "Performans için kaliteyi düşürdüğümde arayüz de kalitesiz görünüyor; arayüz yüksek kalite olmalı."
- **Neden:** web'de `graphics.hiDpi`'nin piksel oranı çizim alanının üst katmanına da (`viewport__overlay`) uygulanıyordu. Bu katmanda araç önizlemesi, ölçü kutuları, kenet işaretleri ve adları, tutamaçlar, artı imleç, ölçek çubuğu ve kuzey oku vardır. "Hızlı" kalitede 2× ekranda hepsi yarı çözünürlükte çiziliyordu.
- **Karar:**
  - Üst katman her zaman ekranın piksel oranındadır (`ViewportController.overlayDpr`). Birkaç çizgi ve yazıdır; tam çözünürlüğün maliyeti yok denecek kadar azdır.
  - `graphics.hiDpi` ve `graphics.msaa` yalnız çizimi etkiler: GPU sahnesini ve çizimin kendi yazılarını (nesne etiketleri, ölçü değerleri). Etiketler çizimin içeriğidir; çok etiketli büyük çizimde en pahalı kısım olduğu için ayarı izler ve HiDPI kapalıyken üst katmana ölçeklenerek konur.
  - Ayar açıklamaları bunu söyler ("Arayüz ... her zaman tam çözünürlüktedir").
- **Masaüstünde değişiklik gerekmedi:** arayüz, önizleme, değer alanı ve kenet işaretleri Iced'in kendi katmanında, kenar yumuşatması hep açık çizilir. Ayar yalnız çizim alanının kendi hedeflerini değiştirir.
- **Doğrulama:** 2× ekranda "Hızlı" kaliteyle çizgi aracının önizlemesi.
  - Önce: üst katman tuvali 1288 × 759 px.
  - Sonra: 2576 × 1518 px; sahne tuvali 1288 × 759'da kalır.
  - Ölçü kutusu, "Uç nokta" etiketi ve artı imleç keskin; çizim etiketi ("P.108") çizimle aynı kalitede.
