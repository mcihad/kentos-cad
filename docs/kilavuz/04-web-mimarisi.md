# KentOS CAD kılavuzu — §4 Web mimarisi

> [CLAUDE.md](../../CLAUDE.md)'nin 4. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §4…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 4. Web mimarisi

### 4.1 Katmanlar

Temel sıra: `core → geo → model → style/processing/product → render → viewport → tools → ui → app`.
Bu bir yerleşim sırasıdır: sonraki katman öncekini kullanır; ters yönde runtime
bağımlılığı kurulmaz. `AppContext`, viewport/tool arayüzleri için mevcut
`import type` istisnalarını koruyun; somut servisleri yukarıya bağımlı kılmayın.
`model`, `style`, `processing` DOM/UI bilmez; `render` UI/tools bilmez.
`io/` model/contracts düzeyinde, `wasm/` hesap bağlayıcısıdır. `product/` ürün
komutlarıdır (ADR 0013, 0022): belgeyi alır, DOM/UI bilmez; araçlar çağırır.
`contracts/generated/` Rust'tan üretilir, elle düzenlenmez.

### 4.2 Kompozisyon kökü

`app/createApp.ts` servisleri kurar. Tema/font ölçeği palette okumasından önce
uygulanır; yeni global singleton veya ikinci belge sahibi oluşturmayın.

### 4.3 AppContext

Özellikler servisleri `app/context.ts` arayüzünden alır. Somut kurulum ve
lifecycle kompozisyon kökündedir; reusable widget'ları `AppContext`'e bağlamayın
(mevcut komuta bağlı `CommandButton` istisnası korunur).

### 4.4 Durum kapsamları

| Kapsam | Sahip / kalıcılık |
|---|---|
| Proje verisi/ayarı | `CadDocument`, `doc.settings`; proje dosyası/cloud revision |
| Kullanıcı tercihi | `ctx.prefs` (tipli ayarların cephesi, `ctx.settingsStore`); web `kentos.settings.v1`, masaüstü `ayarlar.json` |
| Cihaz | grafik (`graphics.*`: arka uç, MSAA, HiDPI); başka cihaza taşınmaz. Kalite ayarları yalnız çizim alanını düşürür; arayüz, kaplamalar, önizlemeler ve etiketler her zaman tam kalitededir (sahibin kararı, ADR 0023 eki) |
| Yerleşim | `ctx.ui`; kullanıcıya özgü panel/ribbon düzeni |
| Oturum | seçim, etkin araç, drafting durumu, pano; proje verisi değil |

Çözüm sırası: varsayılan → kullanıcı → cihaz → oturum; proje ayarı yalnız projeden gelir;
kurum politikası üst sınırdır (ADR 0023).
Proje varsayılanını değiştirmek açık projenin değerini değiştirmez.
Proje değişikliği dirty/revision üretir. Ayar pencereleri taslakla çalışır;
Kaydet'te uygular. Aynı ayarı hem proje hem uygulama penceresinde çoğaltmayın.

### 4.5 Komutlar

`core/commands.ts` registry, `app/commands.ts` ve ilgili app modülleri handler
kaydeder. Menü, ribbon, kısayol ve komut satırı aynı command ID'yi çağırır.
`isEnabled/isChecked` bağımlılıklarını `watch` ile bildirin. Henüz olmayan
özellik `pending(...)` ile açıkça belirtilir; sessiz no-op düğme koymayın.
`app/menus.ts` ve `tools/catalog.ts` tek kaynaklardır; ribbon'a ayrı araç listesi
yazmayın. Mevcut UI registry, hedefteki tam async/headless command bus değildir.
Ürün komutları ayrı düzeydir (ADR 0013, 0022): katalog `contracts/generated/commandCatalog.json`,
işleyiciler `product/` (web) ve `crates/native/application` (masaüstü); iki kayıt katalogla,
davranış `fixtures/commands/v1` ile eşit tutulur. Girdi örtük arayüz durumunu okumaz (CMD-07);
sonuç `CommandResult`'tır.

### 4.6 Klavye ve odak

`e.key` ile Türkçe Q/F klavyeyi koruyun. Metin alanlarında yalnız izinli
kısayollar çalışır; dialog tuşları uygulamaya sızmaz. Aktif araç seçenekleri
ilgili harflerde önceliklidir; Enter/Boşluk odaklı kontrolün yerel anlamını korur.
Koordinat yazmaya başlamak command/dinamik girişi açar; desktop aynı davranışı
hedefler. Kısayol yardımı kayıt üzerinden üretilir.

### 4.7 Araçlar

`tools/catalog.ts` yeni aracın kimlik, grup, yöntem, kısayol ve `steps` kaynağıdır.
Araç DOM'a dokunmaz; mevcut `Tool`, araç aileleri ve `ctx.view` arayüzünü kullanır.
Akış/istem/önizleme TS'te, hesap Rust'tadır. Enter/onay, Esc/iptal ve nested
şeffaf araç davranışı korunur; iptal taslağı kalıcı geometriye dönüştürmez.
Kenet pointerdown/up'ta yeniden hesaplanır; eski pointermove sonucuna güvenilmez.
Masaüstünün karşılığı `kentos_interaction`'dır (ADR 0021): iki taraf `fixtures/interaction/v1`
izlerini ve yazılan değerin `fixtures/point-input/v1` dilbilgisini geçer; davranış
değişikliği ADR 0018'in sırasıyla yapılır. Kenet ve seçim iki platformda aynı Rust
deposundan gelir; masaüstü depoyu belgenin günlüğüyle (`changes_since`) izler, olay
başına yeniden kurmaz (ADR 0029). Araçların oturumlar arası hatırladıkları (web'de statik alanlar)
masaüstünde `Memory`'dir; iz onu başladığı gibi bırakır (ADR 0032).
Masaüstünde araç kamerayı kendisi değiştirmez; `Context.view_changes`'a `ViewChange` ekler, kabuk
çağrıdan sonra uygular. Pano oturumun durumudur, sistem panosu kullanılmaz; kesme web'deki gibi
`cad.entities.delete` ile “Kes” adımında siler, yapıştırma `cad.entities.create` ve `cad.entities.set` ile
“Yapıştır” adımında yazar (ADR 0056, 0074).

### 4.8 Belge ve kayıt

`CadDocument.add/update/remove`, toplu karşılıkları ve `transact` tek mutation
yoludur. Transaction hata verirse rollback; nested işlem savepoint; çoklu edit
tek mantıksal undo adımıdır. Katman ve grup eklemek ve silmek de geri alınabilir adımdır; içe
aktarmanın açtığı katmanlar onun adımındadır (ADR 0072, 0076). Async grup işlemlerinde mevcut
group/cancel yolunu kullanın.
`markSaved(revision)` yalnız gerçekten kaydedilen güncel sürümü temizler.
Kaydetme sürerken yapılan yeni değişikliği dirty=false yapmayın.
`.kcad` KCAD v2'dir (`DocumentSnapshotV2`, ADR 0025): web biçim işçisinde, masaüstü yerel
olarak aynı Rust kodeğiyle yazar; baytlar geri okunup doğrulanmadan dosyaya yazılmaz.
v1 JSON okunur ama üzerine yazılmaz: Kaydet v2'nin yerini sorar. Tür içerikten anlaşılır;
okuyucu sürüm/alan/SRID/kimlik doğrular.
`replaceWith` öncesi aday belge doğrulansın; başarısız açılış mevcut işi kaybettirmesin.
Büyük çizimde (ADR 0030) nesneler işçiye tipli sütunlarla geçer (`io/columns.ts` ↔
`crates/shared/kcad/src/columns.rs`; düzen iki tarafta ve `FORMATS_VERSION` ile birlikte değişir);
işçi baytları gönderilen sütunlarla ve başla karşılaştırır. Açılış aşamalı ve durdurulabilirdir:
belge yalnız bütün dosya okunup denetlenince tek adımda değişir; durdurulan, geride kalan ya da
sürerken çizimi değişen açılış hiçbir şeyi değiştirmez; açılış sürerken komut çalışmaz.
Kaydedilmemiş yerel çizimin kurtarma kopyası hiçbir `.kcad` dosyasının içinde ya da yanında değildir;
kayıt ya da bilerek bırakma siler, geri yüklenen kopya dosyasız ve kaydedilmemiş açılır.
Masaüstünün karşılığı `kentos_domain::Document`'tir; iki belge `fixtures/document-ops/v1`'i
geçer, davranış değişikliği fixture'la birlikte yapılır (ADR 0020).
Her nesnenin kalıcı `uid`'i vardır (ADR 0014): yeni nesne yeni `uid` alır, düzenleme ve
geri alma korur, `replace` yuvayı ve kimliği tutar. v2 her nesnenin `uid`'ini yazar ve korur;
v1 yazmaz, açılışta içerikten türetilir, v2 kaydı göç kaynağını ve proje kimliğini yazar. Bulutta nesnenin kimliği `uid`'idir (ADR 0026): açılış sunucunun
kimliğini `uid` yapar, `applyExternal` gelen kimliği alır ve aynı kimliğe dokunan geri
alma adımlarını düşürür; ayrı eşleme kurmayın.

### 4.8.1 Hesaplama çekirdeği

`model/geom`, `model/ops` ve diğer hesap cepheleri Rust çağrı adapter'larıdır.
Geometri algoritmasını yeniden TS'e yazmayın. `singleSource.test.ts`, native/WASM
fixture'ları ve bağımsız referanslar korunur. Yeni hesap için §14 ve ADR 0008'i izleyin.

### 4.9 Çizim hattı

- `RenderBackend` sözleşmesini ve WebGL2/WebGPU davranış uyumunu koruyun.
- CPU kaynak koordinatı f64; GPU'ya yerel orijine göre float32 fark gider. Stilli çizimde yerel orijin, konumun çapaya ortalanmış 65,536 km'lik karosunun merkezidir; kamera kaba ve ince iki parça gider (ADR 0157).
  Mutlak büyük dünya koordinatını GPU'ya yüklemeyin.
- Kirli katmanlar güncellenir; seçim/hover/ızgara ayrı cache/katmanlardır.
  Masaüstünde seçim ve üzerine gelme ayrı wgpu sahne parçasıdır; kenet işareti ve
  seçim kutusu Iced canvas'ıdır (ADR 0029). Iced her olayda bütün pencereyi çizer;
  masaüstünün çizim alanı resmini tutar ve yalnız gösterdiği değişince sahneyi yeniden
  çizer, üzerine gelme vurgusu üstüne çizilir. Yazıların sorgusu ve çok nesneli
  Öznitelikler de değişmeyen karede yeniden bulunmaz (ADR 0120). Çizimin yazıları da, web'deki gibi sahnenin
  üstünde Iced canvas'ıdır; neyin nerede çizileceğini ortak depo söyler (ADR 0055).
  `requestRender/requestOverlay` istekleri birleşir; her olayda tam belge rebuild yok.
- Canvas resize sonrası siyah kareyi önleyen mevcut senkron redraw istisnasını koruyun.
- Pick/snap/geometri kararları `PickIndex` → Rust store üzerinden gelir.
  Renderer kaynak geometriyi yuvarlamaz veya kalıcı belgeyi değiştirmez.

### 4.10 Arayüz

`Component` ve `DisposableStore` ile abonelik/dinleyici yaşam döngüsünü kapatın.
Uzun listelerde mevcut `TreeView`/`VirtualRows`'u kullanın; editte bütün ağacı
yeniden kurup odak/yeniden adlandırmayı kaybettirmeyin. Renk/font/ölçek DESIGN.md'dendir.
Uygulama içi sorular `ui/widgets/confirm.ts` (`confirmDialog`, `askUnsaved`,
`askRemove`) üzerinden sorulur; `alert/confirm` veya durum satırında soru yok.
Esc/×/arka plan vazgeçtir. Değişmemiş yeni düzenleyici için kaydet sorusu çıkarmayın.

### 4.11 Processing

`defineTool` metadata'sı UI/komut/parametrelerin kaynağıdır. Hesap salt okunur
girdiyle çalışır, `ChangeSet` üretir; runner sonucu doğrular ve tek undo adımı
uygular. Web Worker server worker değildir. İfade dilini genel JS/Python `eval`
ile değiştirmeyin. Ayrıntı: [docs/PROCESSING.md](../../docs/PROCESSING.md).

### 4.12 Proje türleri

Proje ya CAD ya CBS türündedir (ADR 0165; `app/workspaces.ts`, masaüstünde `modes.rs`);
Hibrit yoktur. Tür proje ayarıdır; sahneyi, eksen ve açı düzenini ve şeridi belirler, veri
modelini ve hesapları değiştirmez. Türü sorulmamış proje (eski Hibrit dahil) CBS gibi
gösterilir ve kullanıcının açtığında bir kez sorulur; başsız sunucu, Python ve MCP sormaz.
Şeritte olmayan komutun kısayoldan erişilebilir olması yetki verildiği anlamına gelmez;
tür güvenlik sınırı değildir.
