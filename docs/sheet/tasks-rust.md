# Rust görevi: pafta çekirdeği, WASM, sunucu ve masaüstü

Bağlayıcı belge: [design.md](design.md). Bağlama kuralları: [integration.md](integration.md). Bu
dosya, Rust ajanının işini sırasıyla ve kabul ölçütleriyle verir. Tasarımdan sapmak gerekirse
§Sapmalar'a gerekçesiyle yazılır.

## Ortak kurallar

- **Depo ve dal:** `/home/cihad/Projects/kentos-cad`, dal `feat/sheet-layouts`. Dal değiştirilmez.
  `git commit`, `push`, `stash`, `reset`, `checkout -- …`, `clean` yapılmaz. Commit'leri
  koordinatör atar.
- **Önce oku:** `CLAUDE.md` (tamamı, bağlayıcı), `DESIGN.md`, `docs/sheet/design.md`,
  `docs/sheet/integration.md`. `crates/shared/*` ve `crates/wasm/*` için ilgili ADR'ler: 0001, 0002,
  0008, 0010, 0013.
- **Dokunulacak yerler:**
  - Serbest: `crates/shared/sheet/`, `crates/wasm/sheet-wasm/`, `crates/sheet-ui/`,
    `fixtures/sheet/`, `scripts/fonts/sheet_metrics.py`,
    `apps/web/src/contracts/generated/sheet/` (yalnız üretici yazar).
  - 4. adımda: sunucunun yeni dosyaları.
  - Başka paylaşılan dosyaya yalnız integration.md §3'teki bağlantı noktası olarak dokunulur; her
    dokunuş o tabloya bir satır olarak yazılır.
- **Yeni dış bağımlılık yok.** Çalışma alanındakiler kullanılır: `serde`, `serde_json`, `ts-rs`,
  `schemars`, `kentos-expression`, `kentos-geometry-core`, `kentos-style-core`… Gerçekten
  gerekiyorsa durup gerekçesini rapora yaz.
- **Ağır komutlar kilitle sırayla çalışır;** web ajanı da aynı kilidi kullanır:
  `flock /home/cihad/Projects/kentos-cad/.run/heavy.lock <komut>`. Ağır sayılanlar: `cargo build`,
  `test`, `clippy`; `pnpm wasm`, `rust:*`; vitest; tarayıcı.
  - Bütün çalışma alanını derleyen `cargo test`'i gereksiz yere çalıştırma; `-p` ile hedefle.
  - `.cargo/config.toml` 4 iş sınırını koyar, değiştirme.
- **Gerçek kullanıcı ayarlarına ve profillerine yazma.** Testler ve görüntüler `.run/` ya da
  geçici dizin kullanır.
- **Kod:**
  - `unwrap`/`expect` yalnız testte ya da değişmezi yorumla kanıtlanmış yerde. Kullanıcı girdisinde
    panik yok; hatalar tiplidir (kod + Türkçe ileti).
  - Belirleyicilik: `HashMap` sırasına güvenme, sabit yuvarlama, sabit sayı biçimi.
  - Yorum dili ve biçimi çevredeki koda uyar.
- **Bitince** integration.md §7 “Rust” bölümüne raporunu yaz. Konuşmanın sonunda da aynı raporu
  döndür. Raporda olmayan iş yapılmamış sayılır; yapılmayanı açıkça yaz.

## Adım 1: `kentos-sheet` çekirdeği

`crates/shared/sheet` (`shared` grubu; makine + wasm32).

1. **Model** (design §2–§3): bütün tipler serde ile; ts-rs dışa aktarımı
   `apps/web/src/contracts/generated/sheet/` altına. Kalıp `crates/shared/contracts`'taki gibidir
   (özellik bayrağı, `TS_RS_EXPORT_DIR`).
2. **Doğrulama ve normalleştirme:** tekil adlar ve kimlikler, kopuk bağ, aralık dışı değerler, grup
   çerçeveleri. Bilinmeyen alan hatadır (`deny_unknown_fields` ya da eşdeğeri).
3. **İşlemler ve tersleri** (§4). Her işlem için test: `apply` sonra `inverse` → özdeş kitap.
4. **Yeniden yerleşim** (§3.2), **ana sayfa** (§3.3), **varlık üst verisi** (§3.4).
5. **Yapışma** (§5): `SnapSession` + `query`; eşit aralık, mesafe rozetleri, boyutlandırma ve dönüş
   yapışması; hit test, döndürülmüş kutu dahil.
6. **Metin:**
   - `scripts/fonts/sheet_metrics.py` (fontTools 4.61 bu makinede var) ile
     `crates/shared/sheet/data/font-metrics.json` üret.
   - Kaynak, çizimin yazı tipleridir (ADR 0055: `apps/web/src/app/appearance.ts` `DRAWING_FONTS`,
     `apps/web/src/assets/fonts`). `--check` kipini de yaz.
   - Çekirdek bu tabloyla satır kırar, hizalar ve `ShrinkToFit` uygular.
7. **İfadeler** (§7): `kentos-expression` ile bağlar ve `[% … %]` parçaları; hazır değişkenler;
   `⟨ad?⟩`.
8. **Çizim planı** (§8): her öğe türü.
   - Karelaj: Y/X, yazı bandı, zebra ve çentik.
   - Ölçek çubuğu: ADR 0110'un 1-2-5 kuralı ve görünüşü.
   - Kuzey oku: `KentosK` ADR 0110'daki gibi; grid, coğrafi ve manyetik kuzey.
   - Lejant yerleşimi (girdiler ev sahibinden); tablo; koordinat listesi; antet.
   - Meridyen yakınsaması: `kentos-geometry-core`'da izdüşüm hesabı varsa çekirdekte, yoksa girdiden.
9. **SVG yazıcısı:** sabit sayı biçimi. Harita içi girdiden gelir; gelmezse açık gri “harita” yer
   tutucusu çizilir.
10. **Şablonlar** (§12): biçim, `validate`, `migrate`, `instantiate` (kâğıt seçilirse yeniden
    yerleşim), `extract`; **10 sistem şablonu** `crates/shared/sheet/templates/*.json`.
    - Her biri `workspaces` ve `project_types` taşır (§11a, §12 tablosu).
    - `Border` öğesi (pafta çerçevesi; CAD'de bölge işaretli) model ve çizim planındadır.
    - **Kip profilleri** veri olarak `crates/shared/sheet/data/profiles.json`'da durur (§11a):
      araçlar, hazır biçimler, adlar, varsayılan şablon, galeri sırası; hibrit, CAD ve GIS'in
      birleşimi; profili olmayan kip ortak profili kullanır.
    - Çekirdek şunları dışa verir:
      - `profile_for(workspace, capabilities)`;
      - `tool_availability` (görünür, devre dışı + neden);
      - `rank_templates(templates, workspace, project_type)`.

      WASM'da da açılır.
    - Yetenek kaybı (karelaj ya da kuzey okunda koordinat sistemi yok gibi) ön denetim bulgusudur.
    - Şablonlar özenli tasarlanır: kenar boşlukları, antet hücreleri, yazı boyları, hizalar. Bir
      mühendisin teslim edeceği paftanın düzeyinde olmalıdır.
    - Her biri A4, A3, A1 ve A0'da, iki yönde, ön denetimden hatasız geçer (veri yer tutucuları
      hariç).
11. **Eşitleme planı** (§13 tablosu), **ön denetim** (§9), **atlas planı** (§7a), **standart ölçekler**
    ve **kâğıt boyları** (veri).
12. **`cloud` modülü:** sunucu API'sinin istek ve yanıt tipleri (§13). ts-rs ile dışa aktarılır.
13. **Fixture'lar:** `fixtures/sheet/v1/` (§14 tablosu) ve biçimini anlatan bir `README.md`.
    Rust testleri hepsini koşar.
14. **Gözle denetim:**
    - Her sistem şablonunun önerilen kâğıttaki SVG'si `fixtures/sheet/v1/svg/` altında altın dosya
      olarak durur.
    - Aynı SVG'lerin PNG'si `.run/shots/sheet/templates/<id>.png` olarak üretilir. Örnek:
      `google-chrome --headless --screenshot=… --window-size=…`; çıktı geçici dizinden okunur.
    - Bu görüntüler koordinatörün incelemesi içindir, depoya girmez.

**Kabul:**

- `cargo test -p kentos-sheet`;
- `cargo clippy -p kentos-sheet --all-targets -- -D warnings`;
- `cargo build -p kentos-sheet --target wasm32-unknown-unknown`;
- `pnpm arch:deps`;
- `python3 scripts/fonts/sheet_metrics.py --check`.

## Adım 2: `kentos-sheet-wasm`

`crates/wasm/sheet-wasm` (`wasm` grubu). Öbür `crates/wasm/*` paketlerinin kalıbında: JSON mu,
`serde-wasm-bindgen` mi, hata nasıl döner, onlara bak ve aynısını yap.

- **Dışa açılanlar** (JS adlarıyla):
  - `engineInfo`, `paperSizes`, `standardScales`;
  - `systemTemplates`, `validateTemplate`, `instantiateTemplate`, `extractTemplate`;
  - `applyOp`, `hitTest`;
  - `SnapSession` sınıfı (`new`, `query`, `free`);
  - `displayList`, `toSvg`, `preflight`, `atlasPlan`, `planSync`.
- **Bağlantı noktaları:** R-1, R-2, R-4, R-5 (integration.md §3). Paketin çıktısı
  `apps/web/src/product/sheet/pkg`.
- **Kabul:**
  - `flock … pnpm wasm` temiz.
  - Node'da küçük bir duman koşusu: paketi `initSync` ile yükle, bir fixture'ın `applyOp`,
    `displayList` ve `planSync` sonuçlarını Rust testinin beklediğiyle karşılaştır. Betik
    `crates/wasm/sheet-wasm/tests/` altında ya da raporda komutuyla.
- Bitince `.run/sheet-engine-ready` dosyasını yaz (API'nin kısa özeti). **Sonra dur ve raporla.**
  3. ve 4. adımlara koordinatör onaylamadan başlama.

## Adım 3: sunucu ve `kentos-cloud` (koordinatör söyleyince)

design §13'e göre:

- migration;
- `crates/server/application/src/sheet_templates.rs`: satır güvenliği, komut günlüğü, idempotency,
  outbox olayı;
- `apps/api/src/http/sheet_templates.rs`: okuma uçları, komutların kaydı, kişi arama;
- negatif testler;
- `crates/native/cloud/src/sheet_templates.rs`: masaüstü istemcisi.

Mevcut `projects`, `invitations`, `sharing`, `people` kalıplarını izle.

**Kabul:** ilgili `cargo test -p …` (veritabanlı testler `KENTOS_TEST_DB` ile; veritabanı yoksa
nasıl çalıştırılacağını raporla), clippy, `pnpm arch:deps`.

## Adım 4: `kentos-sheet-ui` (koordinatör söyleyince)

`crates/sheet-ui` (yeni `sheet-ui` grubu; bağlantı noktası R-3). design §11'deki kip, `kentos-ui`
bileşenleriyle:

- sekmeler, kâğıt görünüşü (Iced canvas), cetveller, kılavuzlar ve akıllı kılavuz çizimi;
- araçlar, denetçi (kısıt düzenleyicisiyle), öğe ağacı, şablon galerisi, ön denetim, dışa aktarma;
- `MapPainter` trait'i: harita içini ev sahibi boyar. Testlerde sahte boyayıcı kullanılır.

Bağlantı: `apps/desktop`'ta kip, şerit sekmesi ve `MapPainter` (kentos-render-wgpu ile).

Görüntüler `cargo test -p kentos-sheet-ui screens -- --ignored --nocapture` ile alınır. Grafit ve
Pafta temasında, 1440 ve 1100 piksel. Çıktı `.run/shots/sheet-desktop/`.

**Kabul:** `pnpm rust:test:desktop` kalıbında testler ve clippy; `pnpm arch:deps`.

## Sapmalar

Rust ajanı, 1. ve 2. adım (2 Ekim). Her biri tasarımdan ayrıldığı yer ve nedeni.

**Model (§2–§3)**

1. **`Paper` düz bir sayımdır:** `a0` … `a5`, `b0` … `b4`, `custom`; `Iso(IsoSize)` yerine. JSON'da
   `"paper": "a3"`. Boylar yine veridir (`data/papers.json`).
2. **`Master` bir `page` taşır:** ana sayfanın tasarlandığı kâğıt. Bağlı paftanın kâğıdı başkaysa
   ana sayfanın öğeleri kısıtlarıyla o kâğıda yerleşir (`scene.rs`); yoksa A3 ana sayfası A1
   paftada köşede kalırdı.
3. **`Sheet.variables` eklendi:** pafta düzeyi değişkenler. Şablonun sorularının cevapları buraya
   yazılır (`instantiate`), “Şablon olarak kaydet” onları cevapsız soruya çevirir (`extract`).
4. **Tablo sütununun genişliği sayıdır:** `0` kendiliğinden, başka değer µm; `Auto | Fixed` yerine.
5. **Çizgi ve çokgen noktaları çerçeveye göredir:** milyonda bir (`0 … 1 000 000`), böylece çerçeve
   esneyince şekil de esner ve yeniden yerleşim noktalara dokunmaz.
6. **Birim benzeri varyantlar boş yapı taşır** (`Group(GroupItem {})`, `Ellipse(Empty {})`,
   `Auto(Empty {})` …). JSON yine `{"type": "group"}`'tur; serde birim varyantta
   `deny_unknown_fields` uygulamadığı için bilinmeyen alan ancak böyle hata olur.
7. **Genel bakış haritasının merkezi verilmemişse** gösterdiği haritanın merkezini izler.

**İşlemler (§4)**

8. **İşlemler sahibini `Owner { kind: sheet | master, id }` ile anar.** Tasarımın listesine ek:
   `ReplaceSheet`, `RemoveMaster`, `ReplaceMaster`, `InsertItems`, `SetFrames`, `SetOrder`,
   `SetItems`, `SetSnapGrid`, `SetAtlas`, `SetExport`, `AddAssets`, `RemoveAssets`. Çoğu bir başka
   işlemin tersi olarak gerekti (ör. `RemoveItems`'in tersi yerlerini bilen `InsertItems`).
9. **`Applied` bir de `label` taşır** (geri alma adımının Türkçe adı: “Taşı: Harita”); hata tipi
   `OpError` değil, her yerde aynı `SheetError { code, message, path }`.

**İfadeler ve çizim (§7–§9)**

10. **Ek hazır değişkenler:** `@kagit` (kâğıdın adı ya da “W × H mm”) ve `@olcek_payda` (ölçeğin
    paydası, sayı olarak; `@olcek` “1/1000” metnidir).
11. **Meridyen yakınsaması çekirdekte hesaplanır, `kentos-geometry-core`'da değil:** orada enine
    Merkatör hesabı yok. `geodesy.rs` TM tersini Krüger serisiyle (Karney 2011) `libm` ile yapar;
    PROJ'un (pyproj 3.7.2) değerleriyle 1e-8° içinde sınandı. Koordinat sistemi TM/UTM değilse ev
    sahibinin `MapInput.convergence` girdisi kullanılır, o da yoksa ok grid kuzeyini gösterir ve
    ön denetim `convergence_unknown` der.
12. **Başka koordinat sistemindeki karelaj ve derece-dakika-saniye yazıları bu sürümde çizilmez;**
    ön denetim uyarısıdır (`grid_crs_unsupported`, `grid_format_unsupported`).
13. **Satır kırmada kelime bölünmez:** sığmayan tek kelime kendi satırında taşar ve
    `text_overflow` bulgusu olur. Harf çifti aralığı (kerning) uygulanmaz (`no_kerning` bilgisi).
14. **Ön denetime `overlap` uyarısı eklendi:** içerik öğelerinin (metin, tablo, lejant, antet, ölçek,
    kuzey oku…) üst üste gelmesi. Kâğıt küçülünce kısıtların yetmediği yeri gösterir.
15. **Bir sistem şablonu önerdiği kâğıtlarda uyarısızdır** (sınandı); önermediği kâğıtta (A1 şablonu
    A4'te) çakışma ya da taşma uyarısı verebilir. Kabul ölçütü yalnız hatayı yasaklar; o da yoktur.
16. **`schemars` şeması üretilmedi.** Biçim serde tipleri, ts-rs tipleri ve fixture'larla tanımlıdır.

**Kip profilleri (§11a)**

17. **Bir kipin satırı bir aracın hazır biçimlerini söylemiyorsa ortak profilinkiler geçerlidir**
    (şekil, çizgi, ölçek çubuğu, antet her kipte aynı); söylüyorsa yalnız onunkiler (CAD'in bölge
    işaretli çerçevesi, revizyon tablosu). Tasarım bunu açıkça söylemiyordu; yoksa CAD'de “Şekil ▾”
    boş kalırdı.
18. **Rozet ve not “CBS” der** (“CBS şablonu”, “Bu öğe (…) CBS kipinin aracıdır …”): uygulama `gis`
    kipini “CBS” diye adlandırıyor (`apps/web/src/app/workspaces.ts`, ayar şeması); tasarım örnekte
    “GIS” yazmıştı. Ad profil verisindedir (`label`), değişirse yalnız orası değişir.

**WASM (2. adım)**

19. **Cevaplar zarftır, fırlatılmaz:** öbür `crates/wasm/*` paketleri okunamayan girdide `JsError`
    fırlatıyor; pafta paketinin her işlevi `{"ok":true,"value":…}` ya da
    `{"ok":false,"error":{"code","message","path"}}` döndürür. Hatalar tiplidir (kod + Türkçe
    ileti) ve şablon dosyası kullanıcı girdisidir. Yalnız `SnapSession` kurucusu fırlatır
    (`code: message`).
20. **Listede olmayan işlevler de açıldı:** `readBook` (yüklenen kitabı denetler ve normalleştirir),
    `bookDigest`, `applyOps` (tek geri alma adımı), `snapRotation`, `checkExpression` (ƒ alanı),
    `bindableProperties`, `newItem`, `itemNote`; görev düzeltmesiyle `profileFor`,
    `toolAvailability`, `rankTemplates`.
21. **`.gitignore`'a paketin çıktı satırı eklendi** (integration.md §3, R-6); bağlantı noktaları
    listesinde yoktu, öbür `pkg/` dizinleri de orada.

**Fixture'lar (§14)**

22. **Altın SVG'lerde harita çerçeveleri gri yer tutucudur:** haritanın resmi ev sahibinindir;
    altın dosya yalnız çekirdeğin ürettiğini tutar. PNG görüntülerinde yapay bir harita resmi vardır.
23. **Tasarımın listesine ek aileler:** `ops/`, `hit/`, `atlas/`, `profiles/`. WASM sınırının web
    vitest'i (§14) web ajanınındır; şimdilik `crates/wasm/sheet-wasm/tests/smoke.mjs` aynı
    fixture'ları Node'da paketten geçirir.

**Yerleşim düzenleri (1b, design §3.2a)**

24. **Temel düzen düzen geçerliyken `baseLayout`'ta saklanır; geçerli düzen `activeVariant`'tır.**
    Tasarım temel düzenin nerede durduğunu söylemiyordu: bir düzen geçerliyken `Item.frame` o
    düzenin çerçevesidir, temel düzen ancak böyle geri gelir. İkisi birlikte vardır ya da hiçbiri
    (doğrulama). Arayüz geçerli düzeni kitaptan okur; bir kâğıdın hangi düzeni seçeceğini
    `variantFor` söyler.
25. **Düzenin `reference` kâğıdı taşındığı kâğıdın kenar boşluklarıyla düşünülür:** düzen kenar
    boşluklarına uzaklıklarını korur. Kenar boşluğu düzene ayrıca yazılmadı.
26. **Düzeni olan bir pafta (ana sayfa) üzerindeki işlemin tersi, paftanın (ana sayfanın) önceki
    hâlidir** (`replaceSheet` / `replaceMaster`). Düzene yazmanın (kısıtların tersiyle geri
    taşıma) yuvarlaması ince taneli bir tersi kesin olmaktan çıkarabilirdi; bütün hâl her zaman
    kesindir. Düzeni olmayan paftada eski ince taneli tersler aynen kalır.
27. **Bir öğenin düzendeki kaydı gizliliğini de taşır** (`hidden`): küçük kâğıtta bir blok
    gizlenebilir. Düzenin andığı bir grubun anmadığı üyeleri grupla birlikte taşınır.
28. **`AddVariant` düzeni listenin başına koyar ve kâğıt onu seçiyorsa hemen geçerli yapar**
    (hiçbir şey kımıldamaz). `SetVariant` adı, koşulu ve sırayı değiştirir ama düzeni yeniden
    seçmez; seçim kâğıt değişince olur. Geçerli düzen silinince kâğıdın şimdi seçtiği düzen
    (ya da temel düzen) gelir.
29. **Sistem şablonları:** temel düzen şablonun kendi kâğıdının düzenidir; her şablon iki yönün
    de düzenini açıkça taşır (kendi yönününki temel düzenin kopyasıdır), A4 ya da A3 için boyla
    sınırlı düzenler önce gelir.
    - A4'te yer kalmayan bloklar o düzende gizlidir: CAD mimari'de çizim listesi (A4 yatayda
      notlar da), imar planında plan notları, tematik haritada A4 yatayda genel bakış haritası.
    - CAD tekniğin ve mimarinin anteti ile revizyon tablosu beyaz dolguludur: ISO 5457'nin
      ortalama işaretleri çerçevenin 5 mm içine girer, dikey A4'te tam genişlikteki antetin
      yazısını kesiyordu. İmar planının kuzey oku ve ölçeği de beyaz zeminli: A4 yatayda
      planın içinde duruyorlar.
    - ISO 7200 anteti dikey A4'te 170 mm'dir: çift çizgili çerçevenin bölge bandı 5 mm'dir ve
      çizim alanı 170 mm kalır; büyük dikey kâğıtlarda 180 mm'dir.

**Sunucu ve `kentos-cloud` (3. adım, design §13)**

30. **Olay WebSocket'le değil, kişinin olay listesinden ve uzun yoklamayla gider.** Var olan
    `outbox_event` ile `command_log` satırı bir projeye bağlıdır (`project_id not null`) ve
    `/v1/ws` bir projeye abone olur; şablon olayı ise projeye değil kişilere gider. Bu yüzden her
    alıcı için bir satır (`kentos.sheet_template_event`: sahip, paylaşılanlar ve paylaşımı az önce
    alınan kişi) yazılır, `GET /v1/me/sheet-templates/events?after=&limit=&wait=` ile imleçten sonra
    okunur. `wait` (en çok 25 sn) projelerin uzun yoklamasıyla aynıdır (ADR 0044): bu süreçteki
    bir şablon komutu bekleyeni hemen uyandırır (`hub`, R-12), başka sürecinki 5 sn içinde görülür.
    Olay şablonu ve revizyonu söyler; içerik ya da kişi taşımaz. WebSocket kanalı paylaşılan
    `ClientMessage`/`ServerMessage` sözleşmesini değiştirirdi; birleştirmede istenirse eklenir.
31. **Komut günlüğü ayrıdır:** `kentos.sheet_template_command` (kişi, idempotency anahtarı,
    isteğin özeti, saklanan yanıt). Aynı anahtarla yinelenen istek saklanan yanıtı `replayed: true`
    ile alır; anahtar başka bir istekte kullanılırsa 422. Anahtar 8–200 karakterdir.
32. **Denetim kaydı `kentos.audit_event`'e projesiz yazılır**, şablonun kişisel alanında, bir
    `security definer` işlevle (`kentos.sheet_template_audit`): yalnız şablonda rolü olan, yalnız
    kendi adına yazar. Bir editör öbür paylaşımları göremediği için olayın alıcılarını da veritabanı
    söyler (`kentos.sheet_template_audience`).
33. **Beş komut `kentos_contracts::catalog()` ve `commandCatalog.json`'da değildir:**
    `kentos-contracts`, `kentos-sheet`'e bağlanamaz (`kentos-sheet` ona bağlıdır, döngü olurdu).
    Girdileri `kentos_sheet::cloud::catalog_entries()` verir (`schema` özelliği; girdi ve çıktı
    şemaları schemars'tan); sunucunun tam olarak bunları çalıştırdığını
    `the_server_runs_exactly_the_template_commands_of_their_catalog` sınar. `SERVER_COMMANDS` de
    onları saymaz (`sheet_templates::COMMANDS` sayar). Birleştirmede katalog dosyasına girmeleri
    istenirse ya DTO'lar `kentos-contracts`'a taşınır ya da `catalog()` dışarıdan girdi alır.
34. **Komutlar yalnız kişisel alanın komut ucuna gelir** (`POST /v1/tenants/{kişisel alan}/commands`,
    zarfın `projectId`'si boş). Bir kurumun ucu 422 ile söyler: kurum kitaplığı 2. aşamadadır.
    Başkasının kişisel alanı, var olmayan bir kurum gibi 404'tür (var olan kural).
35. **Her şablon komutu 201 döner** (`tenant_command`'ın var olan davranışı), yinelenen ve hiçbir
    şey değiştirmeyen (`changed: false`, ör. aynı rolle yeniden paylaşım) istek de.
36. **`GET /v1/sheet-templates/{id}`'nin `ETag`'i tırnaklı revizyondur** (`"2"`); `If-None-Match`
    tutarsa 304, gövdesiz.
37. **Silme yumuşaktır:** şablon her listeden ve okumadan kalkar (404), revizyonlar denetim için
    kalır. `expectedRevision` verilirse daha yeni revizyon 409'dur (cihaz görmediği bir revizyonu
    silmez; design §13'ün “silinmiş | daha yeni” satırı).
38. **Sınırlar sunucuda da denetlenir:** içerik (varlıklar dahil, JSON olarak) ≤ 8 MB, ad ≤ 120
    karakter, kişi başına ≤ 500 şablon (silinmişler sayılmaz). İçerik cihazın okuduğu gibi okunur
    (`read_template`: bilinmeyen alan, `sys:` kimliği, bozuk varlık reddedilir); sunucu kimliği,
    revizyonu ve tarihleri kendisi yazar.

**Masaüstü (4. adım, design §11)**

39. **Haritanın içi kentos-render-wgpu'nun sahnesinden, Iced tuvaline çizilir; GPU dokusu olarak
    değil.** Bu yapıda Iced'in resim özelliği (ve onunla gelecek `image` crate'i) yok; tuvale ham
    resim konamaz. `MapPainter` bu yüzden haritanın içini tuvale boyar. Masaüstünün boyayıcısı
    (`apps/desktop/src/sheets.rs`) çizim alanının kendi sahnesini (`scene::build_fixed` ve
    `build_curves`, kâğıdın paletiyle) alır: katmanlar, renkler, eğriler ve noktalar çizim alanıyla
    aynıdır. Çizimin yazı nesneleri harita çerçevesinde henüz çizilmez (çizim alanında da sahnenin
    dışındadırlar). Boyanan harita, görünüşü, ekrandaki boyu ve çizimin sürümü değişmedikçe
    önbellekte kalır. *(4b: çizimin yazıları artık çiziliyor, 58.)*
40. **Resimler (logo, mühür) ekranda bir mozaik olarak görünür:** kendi piksellerinin renginde,
    birkaç ekran pikseli büyüklüğünde hücreler (bir kenarda en çok 160). Sebep 39'daki gibi. SVG
    dışa aktarımında resmin gerçek baytları gider; PNG'de aynı mozaik dışa aktarma çözünürlüğünde.
    *(4b: mozaik kalktı, resim piksel piksel çizilir, 64.)*
41. **Kırpma boyayıcıda yapılır:** Iced'in yazılımsal çizicisi tuval geometrisini kırpmaz (GPU
    çizicisi kırpar); iki çizici aynı sonucu versin diye yollar kırpma dikdörtgenine boyayıcıda
    kesilir. Döndürülmüş kırpma dikdörtgeni dönmemiş hâliyle uygulanır. Grubun saydamlığı
    ögelerinin renklerine çarpılır (grubun öğeleri üst üste gelirse kesişim biraz koyu görünür).
    *(4b: dönmüş kırpma dönüşüyle uygulanır, 65.)*
42. **PNG dışa aktarma Iced'in yazılımsal çizicisiyle, 1024 satırlık şeritlerle** yazılır; dpi
    `pHYs` ile dosyaya yazılır. Sınırlar web'inkidir: bir kenar en çok 16 384 piksel, en çok 120
    milyon piksel.
43. **Sürüklerken yapışmayı Ctrl kapatır; Alt boyutlandırmayı merkezden yapar; Shift oranı korur ve
    dönüşü 15° adımlar.** Tasarım yapışmayı kapatan tuşu söylemiyordu.
44. **Boşluk basılıyken geçici el aracı yok:** tuval, klavyenin bir yazı alanında olup olmadığını
    bilemez; denetçide yazılan boşluk eli açardı. El aracı H ile ya da orta tuşla sürüklenir.
    *(4b: var, 61.)*
45. **Denetçide her değer değişikliği bir geri alma adımıdır;** bir sayıyı sürükleyerek
    değiştirmek birkaç adım yazar (birleştirilmez). *(4b: sürükleme tek adım, 62.)*
46. **Galerinin “Kurumum” ve “Benimle paylaşılanlar” kaynakları masaüstünde boştur** ve bunu
    gelecek zamanla söyler; bulut rozetleri, Eşitle ve Paylaş bağlı değildir (masaüstünün bulut
    istemcisi 3. adımda yazıldı, pafta kipine bağlanmadı). “Paylaş…” nedenini söyleyerek kapalıdır.
    *(4b: bulut bağlandı, 52–57.)*
47. **Pafta öndeyken masaüstünde çizim alanı ile birlikte sağdaki paneller (Katmanlar, Özellikler)
    ve alt panel ile komut satırı da çekilir;** pafta kipinin kendi sol ve sağ panelleri vardır.
    “Model | Pafta 1 | +” sekmeleri model öndeyken çizim alanı ile alt panel arasındadır.
48. **Ön denetimin ev sahibi eylemleri** (`project.crs`, `map.placeFromView`) masaüstünde henüz
    açılmaz; ileti günlüğüne söylenir. `sheet.variables` pafta kipinin kendi Değişkenler
    penceresini açar. *(4b: ikisi de çalışır; bilinmeyen bir eylem yine söylenir.)*
49. **Bağlamsal Pafta sekmesinin sayı hapı paftaların sayısıdır:** KentOS UI'nin bağlamsal sekmesi
    her zaman bir sayı taşır.
50. **Kitaplar masaüstünde `~/.local/share/kentos-cad/pafta/` altındadır** (`kitaplar/`,
    `sablonlar/`, `resimler/`), web'in proje anahtarlarıyla (`bulut/…`, `proje/…`, `dosya/…`,
    `oturum/…`); dosya yanına yazılıp üstüne taşınır, okunamayan kitap silinmez, `.bozuk.json`
    olarak kenara alınır.
51. **`.kpafta` kodeki çekirdektedir** (koordinatörün isteği): `kentos_sheet::kpafta::{encode, decode}`,
    WASM'da `encodeKpafta`, `decodeKpafta`. Okuma resimleri kitabın bilgilerine tutar ve bozuk dosyayı
    yarım okumaz (`bad_asset`, `unknown_schema`, `newer_schema`, `bad_json`, kitabın kodları).
    “Yanına ekle”nin kimlik yenilemesi platformdadır: kimlikleri ev sahibi verir (design §12).

**Masaüstü 4b (3 Ekim): web'e denklik ve bulut**

52. **Kitaplığın eşitlenmesi `kentos-cloud`'dadır** (`sheet_library.rs`), masaüstünün kendisinde
    değil. `kentos-sheet-ui` `kentos-cloud`'a bağlanamaz (`sheet-ui` grubu). Sürücü cihazın
    deposunu `DeviceLibrary` trait'iyle okur ve yazar; böylece API testi onu gerçek sunucuyla
    sınar. Masaüstü onu Iced görevi olarak koşar; pafta kipiyle `Effect::Library` ve
    `LibraryEvent` üzerinden konuşur.
53. **Oturum yokken görünen hesap:** bu açılışta oturum kapatılmadıysa son hesabın (`cloud.account`
    ayarı) kopyaları görünür. Çevrimdışı açılışta kişinin şablonları kaybolmasın diye. Bu açılışta
    oturum kapatıldıysa hiçbiri görünmez.
54. **Koşunun ortasında 401 koşuyu bitirir**, kitaplık “giriş yapın” der. Kitaplığın
    “Çevrimdışı”sı bulutun bağlantı durumunu değiştirmez (`went_offline` çağrılmaz); bağlantıya
    bulutun kendi akışı karar verir.
55. **Yineleme anahtarı** web'in metniyle aynı kalıptadır; masaüstü onun SHA-256'sından bir UUID
    yapar. Sunucu 8–200 karakter ister; UUID her zaman uyar.
56. **Yükleyen cihaz kendi kopyasının oluşturma ve güncelleme tarihlerini korur** (web'inki de
    öyle). API testi cihazların kopyalarını karşılaştırırken tarihleri ayırır.
57. **Cihazın kaydı `kentos.sheets.deviceTemplate` v1 zarfıdır:** şablon, kayıt anı ve bulut
    durumu. Bulut durumu çekirdeğin `kentos_sheet::cloud::DeviceCloudState`'idir. Alan adları
    web'in `TemplateCloudState`'iyle aynı (camelCase). TS'e dışa aktarılmaz: web'in kendi tipi var.
    4. adımın düz şablon dosyaları da okunur.
58. **Harita çerçevesindeki yazılar çizim alanının etiketleridir.** Haritanın kamerasında en az 640
    piksellik bir kenarla yerleşir, sonra çerçeveye ölçeklenir. Çerçevenin kenarını kesen yazı
    hiç çizilmez: tuval yazıyı dönmüş bir kutuya kesemez. Web'de canvas'ın kırpması onu yarım yazar.
    Haritada kuzey oku ve ölçek çubuğu çizilmez; paftanın kendileri var.
59. **Lejantın simgeleri sadeleşmiştir:** stil kitaplığının simgesi en yakın alan yamasına, çizgiye
    ya da işarete çevrilir (`sheet_inputs::symbol_of`).
60. **Ondalık nokta kuralı `kentos-ui`'ye ayar olarak girdi:** `set_point_rule` ve alan ya da sayı
    kutusu başına `point()`. Varsayılan değişmedi; masaüstünün öbür pencereleri eskisi gibi yazar.
61. **Boşlukla el tuvaldedir ve yalnız işaretçi kâğıdın üstündeyken açılır:** yazı alanında yazılan
    boşluk eli açmaz (44'ün sorunu).
62. **Sürüklenen sayı tek adımdır:** `NumberInput::on_drag` ilk sürükleme adımında jest açar.
    Jest boyunca gelen değişiklikler geri alma yığınının üstündeki adıma birleşir (tersleri başa
    eklenir); `on_release` jesti kapatır. Yazılan değer kendi adımıdır.
63. **ƒ düzenleyicisi `kentos-sheet-ui`'nindir, masaüstünün İfade oluşturucusu kullanılmadı.**
    Oluşturucu (`apps/desktop/src/expression`) uygulama crate'inin içindedir; `kentos-sheet-ui`
    uygulamaya bağlanamaz. Ayrıca oluşturucu bir alanın ifadesini çizimin nesneleri ve alanlarıyla
    önizler; paftanın bağı ise paftanın ve projenin değişkenleriyle değerlenir. Pencere: ifade,
    hazır adlar, durum satırı, Kaydet / Bağı kaldır. Denetim çekirdeğin `expr::check`'idir,
    web'in `checkExpression`'ı.
64. **Resimler piksel piksel çizilir** (hücreler, en çok 400 000). Büyütülünce yumuşatılmaz.
    “Resim seç…” yalnız PNG ve JPEG alır, SVG'yi nedenini söyleyerek reddeder: çalışma alanında
    yeni paketsiz SVG çizici yok. Web'de eklenmiş bir SVG masaüstünde boş resim kutusu görünür.
    Bkz. integration.md §7, “Açık kararlar”.
65. **Kâğıttaki dönmüş kırpma:** çizgi ve dolgu dönmüş kutuya tam kesilir. Ayağı kutunun dışındaki
    yazı çizilmez, kalanı kutunun dik çevresine kesilir. İç içe iki dönmüş kırpmada yalnız en içteki
    uygulanır; çekirdek ikisini iç içe koymaz.
66. **Bu cihazda yapılan şablonun kimliği UUIDv7'dir** (web'inki gibi); bulut kimliği eşitlenince
    gelir, eski kimlik `formerIds`'te kalır.
67. **Kullan'ın paftası şablonun kendi pafta adını alır** (`name: None`), web'deki gibi; 4. adımda
    şablonun adını alıyordu.
68. **Sahne boyunu her çizimde bildirir** (`StageEvent::Resize`): Sığdır, Gerçek ve Seçim
    sahnenin o anki boyunu kullanır. Önceden boyu yalnız işaretçi olayları getiriyordu.
69. **Galeri pencereye sığar** (`Fitted`): genişlik en çok 1240, gövde en çok 580 piksel (12
    piksellik yazıya göre); en az 860 × 260. Liste dar kalınca kartlar ikişer sıralanır.
70. **Resmin sığdırma sözleri web'inkiler:** İçine sığdır, Kapla, Ger, Özgün boy; Çerçevede kırp.
    4. adımda Sığdır, Doldur, Esnet, Özgün boyut; Çerçeveye kırp idi.
71. **PDF'in PNG okuyucusu çekirdeğin kendisinindir** (`pdf/images.rs`, kilitteki `miniz_oxide` ile
    inflate). `png` paketi wasm32 için onaylı değil. Okuyucu her renk tipini, derinliği ve Adam7'yi
    okur; `png` paketine karşı sınanır. Kesik dosyayı reddeder: her parçanın CRC baytları ve `IEND`
    gerekir. CRC değeri denetlenmez; görüntü verisini zlib'in adler32'si korur.
72. **GeoPDF'in `GPTS`'i sonradan yazılır.** `pdf-writer` sayıları `f32` yazar; enlem-boylam için bu
    ~0,4 m eder. Dizi önce aynı boyda bir yer tutucudur, `finish()`'ten sonra on ondalıklı değerle
    değiştirilir. Belge kimliği yer tutucuyla değil, değerlerle hesaplanır.
73. **SVG resmi PDF'te boş resim kutusudur:** çekirdekte SVG çizici yok (8. adımın konusu masaüstü
    ekranıdır).
74. **Masaüstünün PDF haritası stilli katmanlardandır** (`map_vectors.rs`, `sheet_pdf.rs`). Stil
    motoru haritanın ölçeğinde, kâğıdın paletiyle kurar; yollar web'in `mapVectors.ts` ve
    `corePaths`'inin satır satır karşılığıdır. İlk sürümdeki düz sahne ve her çizgi 0,18 mm yolu
    bırakıldı (koordinatörün beş maddesi). Sonuç: 150 dpi'da haritanın pikselleri web'inkiyle aynı,
    tek pikselde bir seviye fark.
75. **İşaret şekillerinin çevresi kapalıdır.** Masaüstü dolu daireyi ve halkayı kapalı halka yazar;
    çift-tek kuralında başka halkanın içindeki halka onun deliğidir. Web'in `Outline`'ı tam daireyi
    kapatmaz (`closePath` yok), dolu daire simgesinin dolgusu web'in PDF'inde kaybolur; delikli
    simgede her halkayı ayrı doldurur. Demo çizimde böyle simge yok. Koordinatöre bildirildi.
76. **PDF'te çizimin yazıları masaüstünün etiket motorundandır** (`labels.rs`), web'in çapalarıyla:
    ortalı etiketin ortası, metnin taban çizgisi başı, ölçü değerinin taban çizgisi ortası.
    Yerleştirmeyi çekirdek kendi ölçüleriyle yapar. Boy kâğıdın CSS pikselidir (25,4/96 mm).
77. **Çapa kuralı çekirdektedir** (`pdf/maps.rs`): çapası içerik kutusunun dışındaki yazı yazılmaz.
    Altın PDF değişti, çerçeve dışında çapalı 10 yazı çıktı.
78. **Eksik karakterin kapsamı ölçü tablosudur.** Tablonun ölçmediği karakter (ör. Yunanca harf) bir
    yüzün TTF'inde olsa da hiçbir yüzde yok sayılır ve “?” yazılır. Ön denetim, satır kırma, PDF, SVG
    ve ekran tek kaynaktan okur. Kapsamı genişletmek `scripts/fonts/sheet_metrics.py`'deki `EXTRA`'ya
    karakter eklemektir. Ekranda bir harfi başka yüzden çizmek için masaüstü parçaları tek tek çizer
    (`paint.rs`).
79. **Eksik değer işareti ‹ad?›'dir** (U+2039/U+203A). ⟨ad?⟩ hiçbir çizim yüzünde yoktu.
80. **Masaüstünün yedek resmi düz sahnedir.** Vektör biçimi olmayan katmanlı (desen dolgulu) harita
    PDF'e resim olarak gider. Masaüstü o resmi düz sahneyle çizer, stilsiz: desen dolgusunu çizecek
    bir CPU çizicisi yok, çizim alanının GPU çizicisi ekran dışında kullanılamıyor. Web'in yedek resmi
    stillidir.
81. **Masaüstünün ekrandaki haritası da PDF'inkidir.** Çizgiler ekranda en az 0,6 piksel, PNG
    çıktısında en az 0,25 pikseldir. Stilli katmanlar çizimin durumu, kitaplık ve ölçek başına bir
    kez kurulur; en çok dört ölçek tutulur. Vektör biçimi olmayan katman gösteren harita düz sahneyi
    çizer.
82. **Kullan'ın soruları:** çok satırlı değer iced'in `text_editor`'üdür (Ctrl+Enter oluşturur).
    Okunamayan sayı iletiyle reddedilir; web boş bırakır. Masaüstü Değişkenler penceresinin kuralını
    izler.
83. **“Yeni sürüm var” masaüstünde üç yerde:** sekmede bilgi mavisi nokta (`Tab::note`),
    paftalar listesinde yazı, denetçinin Sayfa sekmesinde “Şablon” bölümü. Masaüstü sekmesinin ipucu
    yok; web'in ipucundaki not denetçinin ipucu satırıdır.
84. **Dışa aktarmanın dosya adı web'inki:** SVG ve PNG paftanın adı; PDF tek paftada çekirdeğin
    `export_name`'i, birden çok paftada “<çizim> paftaları”; `.kpafta` projenin adı. Önceden masaüstü
    `[% … %]` şablonunu yazılmadan kullanıyordu.
85. **Yazdır** PDF'i `<geçici>/kentos-pafta/<ad>.pdf`'e yazar ve sistemin açıcısıyla açar
    (`xdg-open`, `open`, `cmd /C start`). Dosya kalır, görüntüleyici açık tutabilir.
86. **PDF'in başlığı** paftanın `baslik` değişkeninden, yoksa ilk paftanın adından gelir.
87. **WMM'in hesabı NOAA'nın kendi `geomag` biçimidir:** Gauss normlu Legendre özyinelemesi,
    Schmidt çarpanları katsayılara uygulanır, jeodezik–küresel dönüşüm o kodun formülleriyle.
    Teknik rapordaki denklemlerle aynı sonucu verir. NOAA'nın 100 noktalık dosyasında bileşenler
    0,0007 nT içinde.
88. **Elle sapma `declinationHand: Option<bool>`'dur.** Alanı olmayan eski kitapta sıfırdan farklı
    sapma elle sayılır, 1. aşamada model yoktu. Masaüstü denetçisi alan yokken de anahtarı bu
    kuralla gösterir.
89. **Paftanın tarihi `@tarih`'tir:** paftanın, sonra projenin “tarih” değişkeni, yoksa ev
    sahibinin `project.date`'i (bugün). ISO olmayan tarih `magnetic_no_date` hatasıdır.
90. **Sapma elipsoid üstündeki yüksekliktedir (0 km).** Haritanın yüksekliği bilinmiyor; birkaç yüz
    metrede fark 0,01°'nin altındadır.
91. **Kuzey çizelgesinin açıları 12°'den darsa genişletilir.** Sırası korunur, eşit olanlar üst
    üste kalır. Uç adları GK, CK, MK'dir; yazı boyunun 0,6'sı, en az 1,8 mm. CK'nin yıldızı çizilmiş
    bir çokgendir: “★” çizim yüzlerinde yok. Kutup ızgara sapması (GV) yoktur.
92. **Şeridin yakınlaştırmaları kâğıdı yeniden çizer** (Sığdır, Gerçek, Seçim). Önceden kâğıdın
    katmanları eski yakınlıkta önbellekte kalıyordu; yalnız tekerlek ve kaydırma boşaltıyordu. Hata
    kuzey çizelgesinin yakın görüntüsü çekilirken bulundu. Test: `a_ribbon_zoom_draws_the_paper_again`.
93. **Açı işaretleri ASCII'dir (6b):** dakika `'`, saniye `"` (“6°19' D”, “−0°05'47"”), ′ ve ″
    değil. Barlow'un U+2032 glifi 0,60 em genişliğinde ve 0,46 em'i boş (`fontTools` ile ölçüldü:
    ilerleme 600, mürekkep 2–136); ′'den sonra hep çift boşluk görünüyordu ve bu yazı tipinin
    tasarımı, biçimlendiricinin değil. Barlow'un ′'sini “eksik” saymak PDF'te öbür yüzün glifini
    getirirdi, ama web ekranı satırı tek yüzle çizdiğinden orada boşluk kalırdı ve her açı yazısı
    `glyph_missing` verirdi. `'` ve `"` her çizim yüzünde dar bir çentik; klavyeden yazılır, PDF'te
    aranır, NetCAD ve Türkçe ölçü belgeleri de böyle yazar.
94. **Çekirdeğin yazdığı küçük yazıların en küçük boyu 1,5 mm'dir** (`text::LEGIBLE_MIN`, ~4 pt;
    `text::layout_down_to`). Kullanıcının “Sığdır”ı eskisi gibi yarı boya iner; çekirdeğin kendi
    yazısı (çizelgenin açıları, okun notu) 1,5 mm'de durur ve sığmazsa `text_overflow` verir
    (ayrıntı `written\u{1f}<ne>`; iletinin düzeltme cümlesi “Sığdır”ı önermez).
95. **Kuzey çizelgesinin açıları geniş çerçevede çizelgenin yanına yazılır** (en > boy × 1,2), öbür
    türlü altına; biri sığmazsa öbürü denenir. Yelpazenin genişliği çizgilerin boyunu sınırlar ve
    yelpaze kapladığı yere göre ortalanır (önceden dip nokta çerçevenin ortasındaydı). Satır önce
    kaynağından önce kırılır, sonra boşluklardan.
96. **Dört sistem şablonunda kuzey okunun çerçevesi genişledi** (aplikasyon krokisi 10 → 26,
    genel A4 11 → 27, GIS atlası 9 → 28, rapor sayfası 11 → 27 mm; boy aynı, sağ kenar yerinde;
    şablonun iki yön değişkesinde de). 10 mm genişliğe 1,5 mm'de bile çizelge sığmıyor; boy
    büyütülseydi K okunun kendisi büyürdü. Görünen değişiklik: okun beyaz kutusu genişledi ve K oku
    kutunun ortasına, 8–9,5 mm sola kaydı. Sistem şablonlarının revizyonu 1'de kaldı (dal
    yayımlanmadı; artırmak her paftada “Yeni sürüm var” derdi).
97. **Not satırları altta K okuna en az üç harf boyu bırakır** (önceden dört): geniş çerçevede
    değişmez; GIS atlasının 16 mm'lik okunda notla birlikte sığmak için.
98. **`northInfo` dört girdi alır** (`book, sheetId, itemId, inputs`): haritanın merkezi, CRS'nin
    TM parametresi ve `project.date` girdilerdedir. Tarihin kaynağı `Scope::lookup`'ın sırasıyla
    bulunur: paftanın “tarih”i, projeninki, `project.date`.
99. **Disk doldu (3 Ekim, 6b sırasında):** `target/debug/incremental` 72 GB'tı ve diskte yer
    kalmamıştı; ağır kilit altında altı saatten eski 518 artımlı derleme önbelleği dizini silindi
    (49 GB). Yalnız yeniden üretilebilir derleme önbelleği; son altı saatin önbelleği yerinde.
100. **“Sapmayı elle gir” modelin değeriyle başlar (6b):** `magnetic_out_of_model`'in düzeltmesi
    ve masaüstü denetçisinin anahtarı, açılırken `declination`'a modelin değerini (mdeg) yazar,
    web'in anahtarı gibi (`sheet-north.mjs` 6317 bekler). `magnetic_no_place`'te model değeri
    yoktur; yalnız anahtar açılır, değer korunur.
101. **Kurum şablonları yeni göçtedir: `0014_org_sheet_templates.sql` (7).** 0013 izlenmeyen bir
    dosya ve bildiğim her veritabanı geçici bir kaptı. Ama kullanıcının geliştirme veritabanına
    uygulanıp uygulanmadığını dokunmadan doğrulayamam; sqlx değişmiş bir göçü reddeder (595ecf1'in
    dersi). 0014 rol işlevinin iki girdili sürümünü kaldırır, politikaları yeniden kurar.
102. **Kurum şablonunda roller `owner`, `admin`, `viewer`'dır.** `TemplateRole::Admin` yenidir:
    istemci “Sil”i ve “Düzenle”yi yöneticiye de açmalı, `Editor` olarak gösterilseydi silemeyeceği
    sanılırdı. Yayımlayan `owner`'dır, ama yalnız kurumda yayımlayabildikçe. Yetkisi giden
    (proje yöneticisinden düzenleyiciye inen) yalnız kullanır. Tasarımın “yayım yetkisi olmayan üye
    düzenleyemez” kuralı budur.
103. **Kurum şablonu tek tek paylaşılmaz.** `share`/`unshare`, paylaşım listesi ve aday araması
    kurum şablonunda 422'dir. Paylaşım politikası yalnız kişisel alanın şablonlarına izin verir.
    Kurumu bütün etkin, koltuklu üyeler zaten görür. Tek kişiye düzenleme yetkisi tasarımda yok.
104. **Komutun yolu şablonun alanıdır.** Kurum şablonunun komutları o kurumun yoluna, kişisel
    alanınkiler (kendi ya da paylaşılan) kişisel alanın yoluna gider; yanlış yol 422. Görünmeyen
    şablon önce 404 alır, yani yol kuralı varlığı sızdırmaz.
105. **`publish` kendi kişisel şablonunu ve onun bulutta son revizyonunu kopyalar.** Paylaşılanı ya
    da bir kurumunkini kopyalamaz (403; önce “Şablonlarıma kopyala”). Aynı kaynaktan ikinci yayım
    ikinci bir kurum şablonu açar; sunucu tekilleştirmez. İstemci bunun yerine “Kurumdakini
    güncelle”yi önerir: kopyanın içeriği değişir ve eşitleme yeni revizyonu `expectedRevision` ile
    yükler. Yeni sunucu kuralı gerekmedi.
106. **Kurum şablonunun çakışma kopyası kişisel alana gider** (“… (bu cihazdaki kopya)”). Kurumun
    kitaplığı kendiliğinden şablon almaz; değişiklikler kaybolmaz.
107. **Yayımlayanın adı ayrı bir politikayla görünür:** `sheet_template_publisher_seen(kişi)`, bir
    `security definer` sorgu. Kişinin, bakanın etkin ve koltuklu üyesi olduğu bir kurumda silinmemiş
    şablonu var mı, ona bakar. Her şablonun rol işlevini satır satır soran düz politika her
    `app_user` satırında pahalı olurdu.
108. **Kurum şablonları kişi başına 500 sınırına sayılır.** Kuruma ayrı bir sınır yok.
109. **`SyncReport.organizations` bir `Option`'dır:** liste okunamadıysa `None`. Masaüstü son
    okunanı tutar; çevrimdışıyken “Kuruma yayımla” bulutun gerekçesini söyler.
110. **Canlı masaüstü testinde uzun yoklama görüntülerden sonra başlar.** Bir bekleme 25 sn sürer;
    görüntü çekimi daha uzun sürdüğünde bekleme olaysız dönüyordu (testin ilk koşusunda bulundu).
111. **Resimler doku olarak çizilir (8):** resmin kendi boyu ve tekrar tekrar yarılanmış hâlleri.
    Yarılamada her piksel kapladığı dört pikselin ortalamasıdır; renkler opaklıklarıyla
    ağırlıklanır, saydam kenar karartmaz. Ekranda gösterilen boyun en az iki katı olan en küçük
    yarılama çizilir (ekranda noktaya iki piksel düşebilir). Hücrelerle çizim ve 400 000 hücre
    sınırı kalktı.
112. **Büyütülen resim, burada tam katlı büyütülmüş dokudan çizilir** (çift doğrusal, en çok 4096 px
    bir kenar; resim başına en çok dört kat tutulur).
    - Neden: Iced'in yazılım çizicisi (tiny-skia: PNG çıktısı ve ekran görüntüleri) büyütülmüş
      dokuyu `(bounds.x / ölçek) as i32` ile yerleştirir.
    - Sonucu: 2×1'lik bir resim 30 px'e büyütülünce 15 px'e kadar kayıyordu. Döndürmenin merkezi
      doğru kaldığı için dönmüş resim yanlış yere düşüyordu (`a_turned_picture_turns_as_the_shapes_do`
      testinde bulundu).
    - wgpu'nun yerleştirmesi zaten doğru. Aynı dokuyla iki çizici aynı sonucu verir: ekran,
      ekran görüntüsü ve PNG çıktısı.
113. **Her resim kendi katmanındadır:** harita gibi iki kâğıt katmanının arasında. Bir çizici bir
    tuvalin resimlerini şekillerinin üstüne çizer; resimden sonra gelen şekiller sonraki katmanda
    kalır, sıra korunur. Şablon galerisinin küçük resimleri tek tuvaldir; orada resim şekillerin
    üstündedir.
114. **SVG resimleri masaüstünde de eklenir ve çizilir** (resvg; “Resim seç…” süzgeci ve uyarı
    sözcükleri “PNG, JPEG ya da SVG”). Boyu dosyanın söylediğidir (usvg, inçe 96 piksel). PDF'te
    SVG resmi eskisi gibi boş resim kutusudur: çekirdekte SVG çizici yok (design §9a), iki
    platformda aynı.
115. **`image` kod çözücüsüzdür; resvg'nin `raster-images` özelliği kapatılamadı.** Iced resvg'yi
    varsayılan özellikleriyle alır. Bu yüzden `gif`, `image-webp`, ikinci `png` (0.17) ve
    `zune-jpeg` (0.4) kilitte. Yalnız SVG'nin içine gömülü resimleri çizerler.
116. **SVG resim PDF'e ev sahibinin PNG'siyle girer (9):** `PdfAsset.raster`. Çekirdeğe SVG çizici
    eklenmedi (koordinatörün kararı: yeni çekirdek paketi yok).
    - `pdf::svg_sizes` her SVG resmin en büyük çerçevesinin dışa aktarma dpi'sindeki boyunu verir,
      uzun kenarı en çok 8192. Böylece iki ev sahibi aynı boyda çizer.
    - `pdf::findings` ayrı bir işlevdir: `toPdf`'in dönüşü (baytlar) değişmedi, web'in çağrısı bozulmasın.
    - Masaüstü PNG'yi resvg ile çizer; SVG'nin yazıları sistemin yazı tipleriyle, Iced'in SVG
      desteğinin okuduğu gibi (bir kez okunur). Bulguları PDF yazılınca günlüğe yazar (Kaydet ve Yazdır).
117. **Masaüstü denetçisinde kuzey okunun sapma bölümü web'in sırasıyla:** Harita, Kuzey, Biçim;
    sapma bloğu; “Elle sapma” ve “Sapmanın yılı” (yalnız elle girilirken) ve ipucu; Yakınsama notu,
    Renk.
    - Genel denetçi üç parçaya bölündü, aynı alan kimlikleriyle.
    - Alan açıklamaları sayfa denetçisinde görünmez (yardım bölümü kapalı); ipuçları alanların
      altında soluk satırlardır.
    - `north_info` her karede değil, seçim ya da kitap değişince hesaplanır.
    - 6b'deki genel alan kısayolu (`declinationHand`) kaldırıldı: anahtar kendi iletisidir
      (`NorthMessage`).
    - “Renk” alanı eklendi (web'de vardı, masaüstünde yoktu).
118. **“Sapmayı elle gir” modelin değerini dakikaya yuvarlar ve yalnız değer yazılmamışsa yazar:**
    web'in anahtarı böyle yapar. Çekirdeğin `magnetic_out_of_model` düzeltmesi ve masaüstünün
    anahtarı buna hizalandı (6b'de yuvarlamasız ve her zaman yazıyordu, sapma 100'ün düzeltmesi).
119. **Galeri kartının resmi plandaki sırayla katmanlıdır (9):** her plan katmanı ve aradaki her
    harita ya da resim kendi tuvalinde, her birinin önbelleği ayrı. Tek tuvalde resim şekillerin
    üstüne çıkıyordu; sınamada aynı liste tek tuvalde çizilip bunun böyle olduğu da gösterilir.
120. **Vektör biçimi olmayan harita, stilli gölgelendiricilerin CPU ikiziyle çizilir (9):**
    render-wgpu `styled::cpu`, `stroke`/`fill`/`marker`/`shapes.wgsl`'in satır satır karşılığıdır.
    - Kullandıkları: `style_block`, `in_scale`/`in_view`/`legible`, atlasın 8…512 basamakları,
      `raster::image` (atlas da artık bunu kullanır).
    - Farklar: karışım sRGB'dedir (GPU doğrusal ışıkta karıştırır, yalnız yumuşak kenarlarda
      ayrışır). Dolgu üçgenlerinin kenarı yumuşatılır (GPU pikselin ortasını örnekler).
    - Katmanlar arası sıra katman, sonra toplu sırasıdır: PDF'in vektörlerininki. Çizim alanının
      sembol düzeyi sırası (`view.order`) kullanılmaz.
    - Ekran da böyle bir haritayı ikizle gösterir. Önceden sade sahne çizilirdi, renksiz desensiz.
      Ekranda en çok 4096, dışa aktarmada 8192 piksel bir kenar.
    - Eski yedek yol ulaşılabilirdi (desen dolgulu Bina katmanı sınaması), bu yüzden kaldırılmadı;
      stillendirildi. Sınama eski yolla 0 mavi nokta, yenisiyle desen buluyor.
121. **Harita resimleri saydam zeminlidir (9):** PDF'in yedek resmi ve SVG çıktısının harita
    resimleri.
    - Yazılım çizicisinin görüntüsü temanın zeminiyle (açık gri) doluyordu. 5. adımdan beri PDF'te
      haritanın altı griydi. 9. adımın PDF kesitinde bulundu.
    - Şimdi zemin saydam, pikseller düz alfaya çevrilir. PDF resmi yumuşak maskeyle kâğıdın ya da
      çerçevenin dolgusunun üstüne oturur.
