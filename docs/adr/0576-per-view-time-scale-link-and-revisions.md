# ADR 0576: Bölünmüş görünümde görünüm başına zaman, ölçek bağı ve iki revizyonun yan yana karşılaştırılması

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-19'un kalanı (ADR 0523'ün “Kalan” listesinden ilk iki iş; üçüncüsü, ikiden çok
  görünüm, yapılmadı). Kapsamı koordinatörün sözleriyle ben belirledim.
- **Bağlam belgesi:**
  - ADR 0285 (bölünmüş görünüm);
  - ADR 0523 (bağlı görünümler, tema, kaydırmalı karşılaştırma; görünümün teması bir görünüm ayarıdır, çizim değişmez);
  - ADR 0210 (zaman sürgüsü, `Window`, `Store::set_time_window`);
  - ADR 0179 (`ops::compare`, veri karşılaştırma);
  - ADR 0087 ve 0119 (Geçmiş: revizyonun ve kontrol noktasının indirilmesi);
  - ADR 0253 (komut protokolü), ADR 0244 (manifest).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** ikiden çok görünüm; ikinci görünümde revizyonun *kendi* zamanı (revizyon görünümünün zamanı sürgüyü izler); revizyon
  görünümünde seçme (yalnız okunur olduğu için kasıtlı değil, bkz. §4); büyük çizimde fark hesabının süresi (ölçülmedi, §7).

## Bağlam

ADR 0523 her görünüme kendi temasını (projenin bir katman durumu) ve iki görünüme Bağla'yı verdi: Bağla merkezi, ölçeği ve dönüklüğü birlikte
taşır. Kalan üç eksik, kullanıcının “2015 solda, 2023 sağda” ve “eski revizyon solda, bugünkü sağda” istekleridir:

1. Zaman sürgüsü (ADR 0210) bütün çizimin tek penceresidir; iki görünüm aynı anda iki ayrı tarihi gösteremez.
2. Bağla ya hepsini ya hiçbirini bağlar. Ortofoto–kadastro karşılaştırmasında ölçekler ayrı kalsın, yalnız merkez izlensin istenir. Hangi
   görünümün hangi ölçekte olduğu da hiçbir yerde yazmaz.
3. İkinci görünüm hep açık çizimin kendisini gösterir; çizimin geri alma geçmişindeki bir önceki hâli, bir dosyayı ya da sunucudaki bir
   revizyonu yan yana göremezsiniz.

## Karar

### 1. Görünüm başına zaman

- Her görünümün zamanı ya **sürgüyü izler** (varsayılan, bugünkü davranış) ya da **kendi anı** vardır. Kendi zamanı yalnız zaman sürgüsü
  açıkken vardır; sürgü kapanınca bütün görünümler izlemeye döner (pencere yok, her nesne görünür).
- **Kendi anı:** Kendi zamanı'na basınca sürgünün anı alınır, sonra görünüm kendi yoluna gider; ◀ ve ▶ sürgünün adımıyla (`step`) ilerler;
  Sürgüyü izle yine bağlar. Görünümün penceresi, anın *sürgünün ızgarasındaki* en yakın konumudur (`tools::view_link::own_window`; yakınlıkta
  eşitlik önceki konuma, `time::nearest`) ve sürgünün Anlık ya da Aralık kipindedir. Bu yüzden iki görünüm, sürgüyle aynı ızgarada
  aynı kurallarla gösterir; ızgaradan çıkan yoktur.
- **Çizim değişmez.** Görünümün zamanı, ADR 0523'ün teması gibi bir görünüm ayarıdır: belge, revizyon ve geri alma adımları aynı kalır
  (test: `a_time_of_its_own_is_no_edit`). Bu yüzden ürün komutu değildir (§6).
- **Kapsam, çizim ve seçme.** `Store::with_window(pencere, f)` bir görünümün çizimi ve etiketleri sürerken mağazanın pencere kuralını
  `f` boyunca o görünümünkiyle değiştirir (`time_window()` ve `time_shown` onu okur; iç içe çağrılar dış pencereyi geri koyar). Pencere
  `Cell` değil atomiktir: stilli sahne iş parçacıklarında kurulur ve mağaza `Sync` kalmalıdır. Kalıcı mağaza penceresi **etkin
  görünümün** etkin penceresidir (`follow_split` her iletiden sonra eşitler): seçme, kenet, tıklama ve Öznitelikler etkin görünümün zamanında
  gösterilen nesneleri bulur. Öbür görünümün zamanı yalnız onun çizimini ve etiketlerini değiştirir. Etkin görünümün kendi zamanı varsa
  sürgünün penceresi seçmenin değil, görünümünkidir.
- **Arayüz:** her görünümün şeridinde bir zaman çipi: `Zaman: sürgü · Kendi zamanı` ya da `◀ 01.01.2015 ▶ · Sürgüyü izle`.

### 2. Ölçek bağı ve ölçek çipi

- **Üç kip:** `Bağ: yok` (her görünüm kendi yolunda), `Bağ: merkez` (öbür görünüm etkinin merkezini izler, kendi ölçeğini ve dönüklüğünü
  korur), `Bağ: merkez ve ölçek` (merkez, ölçek ve dönüklük; ADR 0523'ün Bağla'sı). Kuralı `tools::view_link::follow(Link, etkin, öbür)`.
  Görünümleri bağla (`view.splitLink`) açma kapama olarak kalır (kapalıdan `merkez ve ölçek`'e); kip, görünümün seçicisinden değişir.
- **Ölçek çipi:** her görünüm kendi ekran ölçeğini 1:N yazar. N = 1 / (ölçek × 0,00026458) (96 dpi, durum çubuğundaki “Ekran 1:N” ile aynı
  kural: `view_link::screen_scale`, `METRES_PER_PX`). Kip `merkez` iken iki çip farklı N'dir (resimde 1:545 ve 1:218), `merkez ve ölçek`
  iken aynı.
- Ölçek bağı da bir görünüm ayarıdır, oturumun durumudur; kayda ve komuta yazılmaz.

### 3. İkinci görünümde başka bir revizyon

İkinci görünümün **kaynağı**: Açık çizim (varsayılan), Geçmişten `n` adım geri, Dosyadan…, ya da Geçmiş sekmesinden “İkinci görünümde”.

- **Geri alma geçmişi:** `Document::past(n)` açık çizimin bir kopyasını `n` adım geri alır (kopya; açık çizim ve geri alma yığını
  değişmez) ve adımın adını verir (`undo_labels`, en yenisi önce). Ad: “2 adım geri, “Taşı” öncesi”. Seçicide en çok 12 adım.
- **Çizim dosyası:** `.kcad` (v2 ya da v1 JSON), Dosyadan… ile seçilir.
- **Sunucudaki revizyon (bulut dosya projesi):** *erişilir*. Geçmiş sekmesi ADR 0087 ve 0119'dan beri revizyonu ve kontrol noktasını
  `catalog_download` ile indirir; yeni bir indirme türü, `Download.side`, indirilen dosyayı uygulamanın kendi geçici yerine yazar,
  ikinci görünümde açar ve siler (`revision_from_download`). Geçmiş sekmesinin revizyon ve kontrol noktası satırlarında “İkinci görünümde”
  düğmesi vardır. Açık çizim o projenin değilse söylenir (ortak nesne az olabilir, fark kalıcı kimliklerle bulunur).
- **Salt okunur:** revizyon kendi belgesi ve kendi geometri mağazasıyla (`Spatial::reload`) çizilir ve hiçbir yere yazılmaz. Üzerine basmak
  görünümü etkin yapmaz ve araca, kenete, işaretçiye gitmez; düzenleme hedefi her zaman açık çizimdir. Kaynak seçicisinde bu söylenir.
- **Fark:** açık çizim ile revizyon **kalıcı kimlikle** (`uid`) eşlenir ve her nesne `ops::compare`'in kuralıyla sınıflanır
  (ADR 0179: geometri, bir tolerans içinde aynı mı; öznitelikler ve katman eşit mi; tolerans varsayılan 0,001 m).
  - **eklenmiş:** yalnız açık çizimde var;
  - **silinmiş:** yalnız revizyonda var;
  - **değişmiş:** ikisinde var, geometri ya da öznitelik ya da katman (bir öznitelik gibi sayılır) ayrı: `geometry`, `attributes` ya da `both`.

  Revizyon görünümünde **silinenler ve değişenler** uyarı renginde işaretlenir (açık çizimde olmayan eklenenler orada çizilemez); şeritte
  “1 eklendi · 1 silindi · 1 değişti” çipi sayıları söyler. İşaretler açık çizimin her düzenlemesinde yeniden bulunur (üretim sayısıyla,
  değişmeyen karede yeniden hesap yok).
- **Neden kimlikle:** konumla eşleme (ADR 0179'un Konum kipi) taşınan bir nesneyi “silindi” ve “eklendi” diye gösterirdi; amaç “neyin değiştiği”dir.
  v2 dosyalar nesnenin kalıcı kimliğini taşır; v1 dosyada kimlikler içerikten türetilir (ADR 0014) ve birbirinden ayrı dosyalarda ortak
  olmayabilir, bu yüzden bağımsız başvurunun dosyaları v2'dir.

### 4. Salt okunur revizyonda seçme

Revizyonda seçme ve tıklama bilerek yoktur: seçim, düzenleme hedefi ve kenet açık çizimindir. Revizyondan bir nesneyi almak isteyen, o dosyayı
ya da kontrol noktasını açık çizime alır (Başka çizimden al, ADR 0193). Bu bir sınırdır, eksik sayılmaz.

### 5. Yeni `.kcad` alanı yok

Zaman, bağ ve kaynak oturum durumudur; çizime yazılmaz. Şema ve `FORMATS_VERSION` değişmez (şema 98 ayrılmıştı, kullanılmadı).

### 6. Hangisi komut, hangisi oturum durumu

CLAUDE.md §9.2'nin ölçütü “çizimi ya da projeyi değiştirir mi” ve “aynı işi Python ya da bir yapay zekâ ajanı da yapmalı mı”dır.

- **Zaman, bağ, ölçek çipi, kaynak seçimi ve ikinci görünüm:** pencerenin ve oturumun göstermesidir, çizim ve proje değişmez; ADR 0523'ün
  teması gibi komut değildir. Bunların kuralı yine de çekirdektedir ve bağımsız başvuruyla sınanır (`tools::view_link`).
- **Fark:** ajanın da sorduğu bir sorudur (“bu çizim geçen adımdan ne değişti?”): **salt okunur ürün komutu `cad.drawing.compare` v1**
  (Query). Girdisi `{against: {kind: "history", steps} | {kind: "file", path}, tolerance?, limit?}`; çıktısı `same`, `changed`, `added`, `removed`,
  `addedUids`, `removedUids`, `changedObjects[{uid, status, fields, distance?}]`, `truncated`, `revision`. Kodlar: `invalid_tolerance`,
  `invalid_limit`, `no_such_step`, `invalid_file_path`, `file_not_found`, `import_unreadable`. Masaüstü işaretleri aynı hesaptan
  (`drawing_compare::diff`) alır: fark bir kez yazılır. Python (`kentos.cad.drawing.compare`) ve MCP (katalogdan; salt okunur olduğu için
  yıkıcı listesinde değil) aynı komuttur.
- Geri alma geçmişi oturumun durumudur, kayıt değildir: başsız sunucuda ve Python'da da bellekte tutulur; `history` karşılaştırması yalnız
  o oturumda anlamlıdır. Dosya mutlak yoldur (göreli yol reddedilir).

### 7. Sınırlar

- Geçmişten revizyon, çizimin bir kopyasıdır (`Document::clone` ve `n` geri alma); büyük çizimde bir görünümü değiştirmek bir kopya ve bir
  eşleme maliyeti taşır. Süre **ölçülmedi**; açık çizim her düzenlendiğinde (üretim sayısı değişince) fark yeniden hesaplanır. 100 000
  nesnede ölçüm sonraya kaldı.
- Tek revizyon görünümü vardır (ikinci görünüm). Kendi zamanı revizyonda yoktur.
- Revizyondaki eklenenler çizilmez (o nesneler revizyonda yok); sayıları çipte ve komutun `addedUids`'indedir.

## Sonuç

- İki görünüm ayrı iki tarihi gösterir (2015 solda, 2024 sağda); seçme ve kenet etkin görünümün tarihindeki nesnelerle yapılır.
- Bağ, ölçeği korumadan merkezi izleyebilir; her görünümün ölçeği yazar.
- Açık çizimin bir önceki hâli, bir dosya ya da sunucudaki bir revizyon yan yana görülür; değişenler ve silinenler işaretlidir; aynı soru
  Python ve MCP'de `cad.drawing.compare` ile sorulur.

## Doğrulama

| Denetim | Nasıl | Sonuç |
|---|---|---|
| Ortak durumlar (bağ, ölçek, pencere, ◀ ▶) | `python3 scripts/fixtures/view_link_cases.py --check` (bağımsız başvuru: Python, KentOS kodu yok), `cargo test -p kentos-geometry-core --test all view_links` | `fixtures/views/v1/links.json` güncel; 4 test geçti |
| Komut durumları | `python3 scripts/fixtures/drawing_compare_cases.py --check` (KCAD v2'yi bağımsız yazıcıyla yazar; eşleme ve sınıflama başvuruda yazılı), `cargo test -p kentos-native-application --test all every_command_case`, `.run/py/bin/python -m unittest python.tests.test_command_cases` | 28 durum (history ve dosya; aynı, taşınmış, öznitelik ve katman, beşinci köşe, tolerans, sınır, retler); masaüstü ve Python'da aynı sonuç; dosyalar `fixtures/compare/v1/revisions` |
| Python | `.run/py/bin/python -m unittest python.tests.test_drawing_compare` | geçmişe karşı ve dosyaya karşı; çizim, geri alma ve kirlilik karşılaştırmadan sonra aynı; retler; 2 test geçti |
| Katalog, yetenekler, MCP | `cargo test -p kentos-commands`, `cargo test -p kentos-mcp`, `python3 scripts/python/sdk.py --check`, `python3 scripts/cad/manifest.py --check` | geçti |
| Masaüstü | `cargo test -p kentos-desktop split_` | görünüm başına zaman (2015 öbür görünümde, etkin 2024, seçme etkin görünümün zamanında, sürgü kapanınca izleme), çizim değişmez, bağ üç kip, revizyon geçmişten (sayılar, işaretler, salt okunur, basmak etkin yapmaz), dosyadan ve indirilen dosyadan; 5 yeni ve 3 eski test geçti |
| Resimler | `KENTOS_SHOTS_ONLY=bolunmus-zaman,bolunmus-bag-merkez,bolunmus-bag-olcek,bolunmus-revizyon-gecmis,bolunmus-revizyon-dosya cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` | `.run/shots/arac-bolunmus-*` (iki tema, iki boyut) |
| Lint | `cargo clippy -p kentos-geometry-core -p kentos-commands -p kentos-domain -p kentos-native-application -p kentos-desktop --all-targets -- -D warnings` | temiz |

Çalıştırılmayanlar: sunucudaki revizyonun gerçek `kentosd` ile indirilip ikinci görünümde açılması (`apps/desktop/scripts/cloud-live.sh`; yerel
dosya yolu aynı `revision_from_download`'dan geçer ve test edilir, indirmenin kendisi ADR 0087/0119'un yoludur); 100 000 nesnede fark süresi.
