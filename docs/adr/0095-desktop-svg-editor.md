# ADR 0095: Masaüstünde SVG çizim düzenleyicisi

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** docs/STYLE.md §7; ADR 0008 (SVG çekirdeği), 0090 (stilli çizim), 0092 (Stil yöneticisi), 0094 (Sembol tasarımcısı)
- **Sahibin yönü (27 Eylül):**
  - web'in stil pencereleri masaüstüne en az bire bir gelir, olabildiğince iyileşerek;
  - iyileşmeler web'e de gelir;
  - görsellik en önemli işlerdendir;
  - SVG düzenleyicisi ve alt pencereleri stil pencerelerinin son dilimidir.

## Bağlam

- **Web:** SVG çizim düzenleyicisi (`ui/svgedit/`) işaret ve desen çizimlerini (piktogramlar) çizer ve Kitaplığım'a SVG varlığı olarak kaydeder:
  - dokuz araç, şekil listesi, cetveller ve kılavuzlar, kenet, düğüm aracı, ölçü;
  - Özellikler, Hizala, Dönüştür ve Dizi sekmeleri; Yol, Nesne ve Seç menüleri;
  - dosyalar: aç, içe al, pano, bırakma, kitaplıktan aç, farklı kaydet, dışa aktar, belge özellikleri, XML kaynağı, izleme altlığı, bitmap izleme;
  - kendi geri alması.
  - Hesabın hepsi SVG çekirdeğindedir (`crates/shared/svg-core`, web'de WASM).
- **Masaüstü:** Stil yöneticisinde SVG çiziminin Düzenle'si ve Yeni sembol → SVG çizimi, Sembol tasarımcısında Yeni çizim… ve Düzenle… soluktu (ADR 0092, 0094); `style.svgEditor` komutu yoktu.
- **Web penceresinin kusurları:**
  - Ölç'ün açısı sağdan saat yönünün tersine 0–360° sayıyordu. Yanındaki ΔY aşağı doğru büyüyor, cetveller ve şeklin Döndürme'si saat yönünde sayıyordu: bir okuma öbürünü yalanlıyordu;
  - taşıma, boyutlandırma ya da döndürme sürerken Esc sürüklemeyi bırakmıyordu;
  - geri alma yalnız klavyedeydi;
  - Belge özellikleri çizimi kaydırıp ölçeklerken kılavuzları yerinde bırakıyordu;
  - Stil yöneticisinde açık kategori ne olursa olsun yeni çizim Çizimlerim'e kaydoluyordu;
  - tuvalin etiketleri (kenetin adı, ölçü, parça boyları, köşe boyu) koyu haleyle yazılıyordu: beyaz şeklin üstünde küçük kalın harfler bulanıklaşıyordu. Ölçünün ikinci satırı sabit 15 piksel aşağıdaydı, büyük yazıda birincisine biniyordu;
  - yazının metni ya da boyu değişince Kutu alanları eski değerde kalıyordu;
  - 1100 piksellik pencerede uzun durum yazısı alt çubuğu büyütüyordu;
  - Ölç'le sürüklerken etiketlerin yazısı seçiliyordu.

## Karar

### Kurallar: iki platformda aynı, `fixtures/style/v1/svgedit.json` ile

- **Web:** `ui/svgedit/svgEditModel.ts` (DOM bilmez). Tuval, araçlar ve paneller onu çağırır. **Masaüstü:** aynı işlevler `style/svgedit/` altında (`draw_tool.rs`, `pointer.rs`, `measure.rs`, `rulers.rs`, `node_tool.rs`, `files/mod.rs`, `state.rs`).
- Model şunları tutar:
  - sürüklemenin çizdiği şekil (dikdörtgen, elips, çokgen ya da yıldız, yazı; Shift kare ya da daire, Alt merkezden) ve yeni şeklin çizgi kalınlığı;
  - tutamaçla ölçeklenen kutu, döndürme düğmesinin açısı (Shift 15°);
  - ölçünün okunuşu, cetvelin adımı, köşe boyunun yuvarlanması;
  - dosya adı, başlangıç ızgarası, panellerin başlangıç uzaklığı.
- Masaüstü V8'in `Math.hypot`'unu ve libm'in `atan2`'sini kullanır (`jsmath`): açılar bit bit aynıdır.
- Durum dosyasını web yazar (`apps/web/scripts/fixtures/record-svgedit.test.ts`, `GOLDEN_WRITE=1`). İki platform geçer:
  - web: `ui/svgedit/svgEditFixture.test.ts`;
  - masaüstü: `style/svgedit/fixture.rs`.
- Geometrinin kalanı zaten tek kaynaktır: yol ve düğüm işlemleri, kenet dizini, hizalama, dönüşüm, diziler, içe alma, dışa aktarma ve izleme SVG çekirdeğindedir. Şekiller onun JSON nesneleri olarak alan alan tutulur; bir işlemin masaüstünde yaptığı, çizimin metnine kadar web'dekinin kendisidir.

### Pencere: `apps/desktop/src/style/svgedit/`

- Web'in penceresi, aynı sözlerle ve boyda (1320 × 860, en çok pencerenin %92'si). Açtığı pencerenin üstünde durur, kapanınca ona döner. Alt pencereleri (içe alma, dışa aktarma, belge özellikleri, izleme, kitaplık, farklı kaydet) onun üstündedir.
- **Solda:** dokuz araç, harfleriyle (Seç V, Düğüm A, Dikdörtgen R, Elips E, Çokgen P, Kırık çizgi L, Kalem B, Yazı T, Ölç M), altında şekil listesi (göz, kilit, çift tıkla ad, sürükleyerek sıra).
- **Üstte çubuk:** Dosya ▾, Altlık…, Bitmap izle…, Dışa aktar…, Kaynak; Yol ▾, Nesne ▾, Seç ▾; Geri al, Yinele; Kenet ve türleri ▾, Cetvel; −, ölçek, +, sığdır. Dar ortada düğme düğme alt satıra geçer; sığmayan menü kayar.
- **Ortada tuval** (Iced'in canvas'ı):
  - kâğıt, ızgara, izleme altlığı, şekiller; döşeme önizlemesi ve dizinin soluk kopyaları;
  - üstünde, ekran uzayında: kılavuzlar, taslak, seçim kutusu ve tutamaçlar ya da düğümler, ölçü, panelin noktası, kenet işareti, cetveller.
  - Şekiller çizim ya da görünüm değişince bir kez çizilir (canvas önbelleği); kalanı imleci izler.
  - Yazılar çizimin yazı tipleriyle (ADR 0055) harf çizgileri olarak, dönüşleriyle ve yığındaki yerlerinde çizilir.
  - Görüntüleri (altlık, izlenecek resim) düzenleyicinin dokulu dörtgeni çizer (`raster.rs`, Iced'in shader widget'ı). Iced'in canvas'ı `image` özelliği olmadan görüntü çizmez; tek dörtgen için yeni paket gerekmedi.
- **Sağda:** Özellikler | Hizala | Dönüştür | Dizi.
  - Özellikler: seçimin boyası, kalınlık, saydamlık, çizgi biçimi, kutu, türe göre geometri, Yol, düzen ve seç grupları; düğüm aracının kutusu; seçim yokken tuvalin ayarları.
  - Sayı alanları yazılırken uygulanır, ↑ ↓ adım adım (Shift on adım) değiştirir.
- **Altta:** Ad, Kategori, son söz (dar pencerede kendi satırında), Vazgeç, Farklı kaydet…, Kaydet.
- **Klavye:** web'in bütün tuşları:
  - Esc sırası (yarım iş, düğüm düzenleme, seçim, kapatma sorusu);
  - Ctrl+Z, Ctrl+Y ve Ctrl+Shift+Z; yol işlemleri ve paneller (Inkscape'in tuşları);
  - düğüm aracının tuşları araç harflerinden önce; Delete, Enter, sıra (Home, End, Page Up, Page Down);
  - oklar ızgara adımıyla (Shift beş kat); yakınlaştırma, çevirmeler, araç harfleri;
  - Ctrl+O, Ctrl+Shift+S, Ctrl+Shift+E, Ctrl+Shift+X.
  - Alan klavyedeyken yalnız ↑ ↓ çalışır.
- **Dosyalar** (`files/`):
  - Yeni çizim; SVG dosyası aç (sistemin dosya penceresi) → içe alma penceresi;
  - Ctrl+V panodaki SVG'yi doğrudan ekler, “Panodan içe al” pencereyle alır;
  - pencereye bırakılan SVG içe alma penceresini açar. Bırakılan PNG ya da JPEG için web'in sorusu gelir: altlık mı, izleme mi;
  - Kitaplıktan aç, Farklı kaydet (Kitaplığım ya da Proje);
  - Dışa aktar: sembol SVG, düz renkli SVG, PNG (piksel genişlik ya da DPI; saydam ya da kâğıt zemin; yalnız seçilenler); dosyaya ya da SVG'yi panoya;
  - Belge özellikleri, XML kaynağı, izleme altlığı ve şeridi, Bitmap izle.
- **Okuma ve çizme:**
  - SVG metni kitaplığın temizliğinden geçer ve roxmltree ile okunur; HTML varlıkları ve bildirimsiz ön ekler onarılır. SVG çekirdeğinin içe alıcısına öğe listesi olarak verilir.
  - PNG'yi stil motorunun raster çizicisi çizimin kendi SVG'sinden çizer (`kentos-render-wgpu` `styled::raster::picture_pixels`), web'in tuvalinin çizdiği gibi; DPI pHYs parçasına yazılır.
  - XML kaynağının düzenleyicisi Iced'in `text_editor`'üdür; etiketler, öznitelikler ve değerler renklidir, satır numaraları düzenleyiciyle birlikte kayar.
  - Bitmap izleme pencerenin iş parçacığında çalışmaz; son ayar kazanır.
- **Açılış:**
  - `style.svgEditor` (Araçlar › Stil);
  - Stil yöneticisinde SVG çiziminin Düzenle'si ya da kartta çift tık, Yeni sembol → SVG çizimi;
  - Sembol tasarımcısında Yeni çizim… ve Düzenle….
- **Kayıt:**
  - Çizim Kitaplığım'a SVG varlığı olarak yazılır. Sistem çiziminde kullanıcının kopyası olur. Sembol boyu, zemin ve saklanan altlık dosyayla gider.
  - Stil yöneticisi kaydı yerinde gösterir; Sembol tasarımcısının alanı çizimi alır.
- **Kapatma:** değişiklik varsa web'in sorusu sorulur (Kaydetmeden kapat, Vazgeç, Kaydet ve kapat).

### İyileşmeler

- **İki platformda:**
  - Ölç'ün açısı sağdan saat yönünde, −180°…180° sayar: ΔY, cetveller ve Döndürme gibi. Araç seçilince durum satırı söyler;
  - taşıma, boyutlandırma ya da döndürme sürerken Esc şekilleri yerlerine koyar;
  - çubukta Geri al ve Yinele düğmeleri, geri alınacak bir şey yokken sönük;
  - Belge özellikleri kılavuzları da çizimle birlikte kaydırır ve ölçekler;
  - Stil yöneticisinde yeni çizim, arama yokken açık Kitaplığım kategorisine kaydedilir;
  - tuvalin etiketleri panelin renginde bir çipin üstündedir. Ölçünün ikinci satırı birincinin altına asılır, yazı boyu ne olursa olsun;
  - sürüklemenin şekli, tutamaç, döndürme, ölçü, cetvel, köşe boyu, dosya adı ve başlangıç değerleri tek modelde, `svgedit.json` ile.
- **Masaüstünde:**
  - Kutu'nun oran kilidi yeniden çizimde unutulmaz;
  - hiçbir şeyi değiştirmeyen düzenleme geri alma adımı bırakmaz;
  - ince çizgi üç piksel içinde bulunur (web tam çizgiyi istiyordu);
  - Yol menüsünde her işlemin ikonu;
  - Kitaplıktan aç'ın kendi penceresi: kartlar resimleriyle, ad ya da kategoriyle arama. Düzenleyici Stil yöneticisinin üstünde de durabildiği için onu seçme kipinde yeniden açmaz;
  - parçalı seçimler dar sütunda büyük yazıyla da tek satırdadır: parçaların payı adlarına göre ayrılır, gerekirse yanları daralır (web'in `flex: 1` ile `nowrap`'ı gibi).
- **Web'de:**
  - uzun durum yazısı alt çubukta iki satırda kesilir, tamamı imlecin ipucundadır;
  - tuvalde sürüklemek yazı seçmez;
  - yazının metni ya da boyu değişince Kutu alanları yeni değerleri gösterir. Masaüstünde panel her çizimde durumdan kurulur.

## Sonuçlar

- **Denetimler:**
  - `svgedit.json` iki platformda geçer;
  - pencere kullanıcının sürdüğü gibi `style/svgedit/tests.rs`'te sınanır: ızgaraya kenetlenen dikdörtgen ve tek geri alma adımı; seçme, taşıma, boyutlandırma ve döndürme; oklar ve kilitli şekil; kırık çizgi ve Enter; yol işlemleri ve düğüm aracı; cetvelden kılavuz ve cetvele geri; ölçü; Kitaplığım'a kayıt ve sistem çiziminin kopyası; kapatma sorusu; içe alma (yeni ya da ekle, renklerin seçimi); bırakılan SVG ve resim; XML kaynağının düzenlenip tek adımda okunması; belge özellikleri ve kılavuzlar; bitmap izleme; Stil yöneticisi ve Sembol tasarımcısından açılış ve çizimin geri verilmesi.
- **Resimler:** `cargo test -p kentos-desktop style::svgedit::screens -- --ignored --nocapture` çeker (`.run/shots/svge-*.png`; altlık ve izleme sahneleri `KENTOS_SNAPSHOT_BACKEND=wgpu` ister):
  - web'in resimlerinin durumları: yeni çizim, dikdörtgen, hepsi seçili, düğüm, kırık çizgi, ölçü, yazı, üç sekme, Dosya, Yol ve Nesne menüleri, XML kaynağı, belge özellikleri, dışa aktarma, içe alma, altlık, izleme, kitaplık çizimi, kitaplık seçicisi, soru;
  - iki boy, iki tema ve büyük yazıyla bir boy daha.
  - Web'in aynı resimleri `node apps/web/scripts/e2e/shots.mjs svgedit` ile.
- **Bilinen farklar ve kalanlar:**
  - **XML kaynağında seçili öğeler:** vurgu renginde ve kalın yazılır, satırın arkası boyanmaz. Iced'in düzenleyicisi yazının arkasına boya çizmez.
  - **PNG panoya kopyalanamaz:** masaüstünün panosu yalnız metin tutar. SVG panoya kopyalanır, PNG dosyaya yazılır.
  - **Bozuk XML:** katı XML olarak okunamayan ve onarılamayan dosya, ayrıştırıcının nedeniyle (satır, sütun) reddedilir. Web'in son çaresi tarayıcının hoşgörülü HTML ayrıştırıcısıdır; masaüstünde karşılığı yok.
  - **Kitaplıktan aç:** web Stil yöneticisini çizim seçme kipinde açar, masaüstünün kendi seçicisi var (yukarıda).
