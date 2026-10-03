# Web görevi: pafta kipi, tasarımcı ve şablon galerisi

Bağlayıcı belge: [design.md](design.md). Bağlama kuralları: [integration.md](integration.md). Bu
dosya, web ajanının işini sırasıyla ve kabul ölçütleriyle verir. Tasarımdan sapmak gerekirse
§Sapmalar'a gerekçesiyle yazılır.

## Ortak kurallar

- **Depo ve dal:** `/home/cihad/Projects/kentos-cad`, dal `feat/sheet-layouts`. Dal değiştirilmez.
  `git commit`, `push`, `stash`, `reset`, `checkout -- …`, `clean` yapılmaz. Commit'leri
  koordinatör atar.
- **Önce oku:**
  - `CLAUDE.md` (tamamı, bağlayıcı: katman sırası, TS kuralları, `h()` ve `signal`, React/Vue/Lit
    yasağı);
  - `DESIGN.md` (ölçüler, renkler, simgeler, şerit);
  - `docs/sheet/design.md`, `docs/sheet/integration.md`.
- **Örnek alınacak mevcut kod:**
  - Kabuk: `app/createApp.ts`, `app/ribbon.ts`, `app/commands.ts`, `app/keybindings.ts`,
    `ui/ribbon/`, `ui/statusbar/`.
  - Bileşenler: `ui/widgets/` (PropertyGrid, TreeView, Dialog, Dropdown, PopupMenu, Splitter,
    VirtualRows, tooltip…). `ui/icons.ts`'teki simge kuralı (20×20, 1,4 çizgi, `currentColor`).
  - **Stil yöneticisi:** sistem öğeleri ile kendi öğelerin ayrımı. Şablon galerisi aynı kalıbı ve
    dili izler.
  - Bulut: `ui/cloud/ShareDialog.ts`, `ui/cloud/ProjectsDialog.ts`, `app/cloud/`.
  - Görüntü düzeneği: `scripts/e2e/cdp.mjs`, `scripts/e2e/shots.mjs`.
- **Dokunulacak yerler:**
  - Serbest: `apps/web/src/{product,render,tools,ui,app}/sheet/` (yeni dizinler),
    `apps/web/scripts/e2e/sheet-shots.mjs`, `docs/sheet/` içindeki web bölümleri.
  - Paylaşılan dosyaya yalnız integration.md §3'teki bağlantı noktası olarak dokunulur: tek
    `installSheets` çağrısı, şerit sekmesi, sekme şeridinin yeri, komut ve kısayol kaydı. Her
    dokunuş o tabloya bir satır olarak yazılır.
  - Yeni ortak bileşen gerekiyorsa önce `ui/sheet/widgets/` altında yazılır; `ui/widgets/`
    değişmez.
- **Yeni npm bağımlılığı yok.**
- **`contracts/generated/sheet/` ve `product/sheet/pkg/` elle yazılmaz;** Rust ajanı üretir. Sunucu
  ve çekirdek tiplerinin elle yazılmış kopyası olmaz. Arayüzün kendi görünüş modelleri
  (`TemplateCard` gibi) serbesttir.
- **Ağır komutlar kilitle sırayla çalışır;** Rust ajanı da aynı kilidi kullanır:
  `flock /home/cihad/Projects/kentos-cad/.run/heavy.lock <komut>`. Ağır sayılanlar:
  `pnpm wasm`, `test`, `typecheck`, `build`, tarayıcı ve görüntü betikleri.
  - `pnpm dev` sunucusunu açık bırakacaksan port çakışmasına bak: 5199 başka bir projenin.
- **Gerçek kullanıcı verisine yazma:** tarayıcı profili geçici dizinde, uygulamanın yerel deposu
  temiz.
- **Arayüz metinleri Türkçe**, kod tanıtıcıları İngilizce. Kâğıt konumu “Sol / Üst / Genişlik /
  Yükseklik”tir (design §2).
- **Bitince** integration.md §7 “Web” bölümüne raporunu yaz. Konuşmanın sonunda da aynı raporu
  döndür. Raporda olmayan iş yapılmamış sayılır; yapılmayanı açıkça yaz.

## A bölümü: motordan bağımsız işler (hemen)

Çekirdek henüz hazır değil; Rust ajanı aynı anda yazıyor. Motorsuz yapılabilecek her şey:

1. **Kurulum kancası:** `app/sheet/install.ts` → `installSheets(ctx)`. Sekmeler, şerit sekmesi,
   komutlar, kısayollar ve paneller buradan bağlanır. `createApp`'e tek çağrı (bağlantı noktası).
2. **Pafta kipi durumu** (`product/sheet/`):
   - açık pafta sekmesi; model ile pafta arasında geçiş;
   - projeye göre pafta kitabının IndexedDB deposu (`kentos.sheets.v1`). Proje anahtarı ev
     sahibinden gelir; bulut projesinde proje kimliği, dosyada dosyanın kimliği.
3. **Sekme şeridi** (`ui/sheet/SheetTabs.ts`): “Model | Pafta 1 | … | +”. Model sekmesi hep ilk,
   kapanmaz. Sağ tık menüsü: ad ver, çoğalt, sil, taşı.
4. **Pafta görünüşü iskeleti** (`ui/sheet/SheetWorkspace.ts`, `render/sheet/`):
   - masa zemini, beyaz kâğıt ve gölgesi;
   - mm cetvelleri; yakınlaştırma (Ctrl+tekerlek, Ctrl+0 sığdır, Ctrl+1 gerçek boyut); kaydırma
     (Boşluk + sürükle, orta tuş);
   - yüksek DPI'da keskin çizim.
5. **Paneller:**
   - sol: “Paftalar” (küçük resim listesi), “Öğeler” (TreeView: görünürlük, kilit, sürükle-bırak
     sıra);
   - sağ: **Denetçi** çatısı: bölümler, çoklu seçim, “—” karışık değer, ƒ bağ düğmesi;
   - **Kısıt düzenleyicisi** bileşeni (`ui/sheet/widgets/ConstraintEditor.ts`): kare çizim, dört
     kenar ve iki merkez çizgisi, açılır seçim.
6. **Şerit:** bağlamsal **Pafta** sekmesi; design §11'deki gruplar. Motor gelene dek eylemler
   devre dışıdır ve nedeni ipucunda yazar. Gereken simgeler `ui/sheet/icons.ts`'te, aynı kuralla.
   - **Kipe göre (§11a):** hangi araçların görüneceği, adları ve hazır biçimleri kip profilinden
     gelir (motorun `profile_for` / `tool_availability` sonucu). Kodda kip başına `if` yazılmaz.
   - Motor gelene dek sabit bir “ortak” liste kullanılır ve B bölümünde profile bağlanır.
   - Proje kipi değişince (hibrit, CAD, GIS) şerit ve galeri yenilenir.
   - Var olan öğeler kip değişince kaybolmaz.
7. **Şablon galerisi** (`ui/sheet/TemplateGallery.ts`):
   - bölümler: Sistem · Benim · Kurumum · Benimle paylaşılanlar;
   - arama; kâğıt ve tür süzgeçleri;
   - kartlar ve rozetler; ayrıntı paneli ve eylemler.
   - Veri bir sağlayıcı arayüzünden gelir: sistem (motor), bu cihaz (IndexedDB), bulut (5. adım).
     Kurumum, 2. aşamaya kadar “yakında” açıklamasıyla boş görünür.
   - **Varsayılan süzgeç projenin kipi ve türüdür** (§11a); sıra: türe uyan, kipe uyan, ortak.
     “Bütün kiplerin şablonları” anahtarı öbürlerini kip rozetiyle gösterir. Başka kipin şablonunda
     eksik yetenek varsa kullanmadan önce söylenir.
8. **Şablon paylaşım penceresi iskeleti** (`ui/sheet/ShareTemplateDialog.ts`): ShareDialog'un
   kalıbında. Eşitlenmemiş şablonda “Paylaşmak için önce buluta eşitleyin” der.
9. **Durum çubuğu hücreleri** (kâğıt imleci mm, seçim boyu, yakınlaştırma, sayfa, ön denetim
   rozeti) ve **pafta kipinin kısayolları** (design §11).
10. **Görüntü betiği** `apps/web/scripts/e2e/sheet-shots.mjs`:
    - `cdp.mjs`'i kullanır; `shots.mjs`'e dokunmaz;
    - sahneleri: koyu ve açık tema, 1440×900 ve 1100×650;
    - çıktı `apps/web/scripts/e2e/out/shots/sheet/`.

A bölümü bitince `.run/sheet-engine-ready` dosyasına bak:

- **Varsa:** B bölümüne geç.
- **Yoksa:** A'nın testlerini ve görüntülerini tamamla, raporunu yaz ve dur. Koordinatör motor
  hazır olunca devam ettirecek.

## B bölümü: motorla (`.run/sheet-engine-ready` varken)

1. **`product/sheet/engine.ts`:** WASM paketini yükler; üretilmiş tiplerle ince, tipli sarmalayıcı.
   Hata Türkçe iletisiyle yukarı çıkar.
2. **İşlemler ve geri alma** (`product/sheet/history.ts`): her eylem bir `Op`; tersi saklanır; adlı
   girdiler; Ctrl+Z ve Ctrl+Y pafta kipinde bu yığını kullanır.
3. **Boyayıcı** (`render/sheet/painter.ts`): çizim planını Canvas2D ile çizer.
   - Yazı satırları, yollar, resimler (varlık deposundan).
   - **Harita çerçeveleri:** projenin çizimi, mevcut çizim hattıyla ekran dışı bir yüzeye, haritanın
     görünüşüyle (merkez, ölçek, dönüş) çizilir ve önbelleğe alınır. Yalnız görünüş, boy ya da
     katmanlar değişince yeniden çizilir.
   - Seçim, tutamaçlar, akıllı kılavuz çizgileri, mesafe rozetleri, eşit aralık işaretleri.
4. **Araçlar** (`tools/sheet/`): seç, taşı, boyutlandır, döndür, alan seçimi; her öğe türünü
   ekleme; cetvelden kılavuz sürükleme; klavyeyle mm adımları. Yapışma `SnapSession`'dan gelir.
5. **Denetçi:** her öğe türünün özellikleri.
   - Harita: ölçek standart listeden ya da elle; merkez “Görünümden al”; katmanlar ve tema;
     karelaj.
   - Lejant, ölçek çubuğu, kuzey oku, metin, tablo, koordinat listesi, antet hücreleri.
   - Kısıtlar; ƒ bağları; çoklu seçim.
6. **Paftalar ve şablonlar:**
   - yeni pafta, şablondan pafta (kâğıt seçimiyle), sayfa ayarları, ana sayfa;
   - “Şablon olarak kaydet” → bu cihaz;
   - galeride canlı küçük resimler (çizim planından küçük tuval).
7. **Ön denetim paneli** ve **dışa aktarma:** SVG (çekirdekten; harita içi PNG olarak gömülür),
   PNG (dpi), `.kpafta` dışa ve içe aktarma.
8. **Testler:**
   - vitest: `fixtures/sheet/v1/` örnekleri WASM üzerinden; geri alma; galeri sağlayıcıları;
     IndexedDB deposu (sahte depo ile).
   - `pnpm typecheck`, `pnpm test`.
9. **Görüntüler** (iki tema, iki boy):
   1. demo çizimde sistem şablonundan ifraz paftası;
   2. öğe sürüklenirken akıllı kılavuzlar ve rozetler;
   3. çoklu seçimde denetçi ve kısıt düzenleyicisi;
   4. şablon galerisi, Sistem ve Benim;
   5. ön denetim bulguları;
   6. dışa aktarma.

## C bölümü: bulut eşitlemesi ve paylaşım (koordinatör söyleyince)

Sunucu uçları hazır olunca:

- `app/sheet/cloud.ts` (mevcut bulut istemcisinin istek yardımcılarıyla);
- eşitleme planı motordan (`planSync`); rozetler; çevrimdışı davranış;
- paylaşım penceresinin canlı hâli; `sheet_template.changed` olayıyla yenileme.

## Sapmalar

Web ajanı, A bölümü (2 Ekim):

1. **Önizleme verisi.** Motor yokken bir paftanın içeriği gösterilemez. Görüntüler ve testler için
   arayüzün kendi görünüş modelleriyle elle yazılmış bir kitap ve şablon kartları
   `product/sheet/preview.ts`'tedir. Uygulama bu dosyayı içe aktarmaz; ondan alınan görüntülerin adı
   `onizleme-` ile başlar. B bölümünde motorun kitabı, şablonları ve çizim planı yerini alır.
2. **Öğeler çerçeveleriyle çizilir.** Kâğıt, kenar boşlukları, seçim, tutamaçlar, cetveller gerçek;
   öğelerin içi motorun çizim planıyla gelene dek adlı çerçevedir (harita çerçevesi açık tonlu).
3. **Seçim için vuruş testi TS'te** (`tools/sheet/pick.ts`, döndürülmüş çerçeve dahil). B bölümünde
   motorun `hitTest`'i yerini alır (yazının satırları, çizginin kendisi).
4. **Şablon galerisinin sırası TS'te** (`product/sheet/templates.ts` `arrangeTemplates`, §11a'nın
   kuralı). Motor `RankedTemplate` veriyor; B bölümünde galeri motorun sırasını kullanır, TS'teki
   kural kaldırılır ya da yalnız motorun cevabını süzer. Kip rozeti şimdilik uygulamanın kendi
   adıyla “CBS şablonu” (çalışma modunun arayüzdeki adı CBS), tasarımdaki örnek “GIS şablonu”.
5. **Proje türü yok.** Açık bulut projesinin türü (`CloudProject`) istemciye gelmiyor, yerel çizimde
   tür yok; galeri şimdilik yalnız çalışma moduna göre sıralar (`traits().projectType = null`).
6. **Proje anahtarı:** bulut projesi `bulut/<kurum>/<proje>`, çizimin kalıcı kimliği
   `proje/<uuid>`, kimliksiz dosya `dosya/<ad>`, hiçbiri yoksa `oturum/<uuid>`. Kaydedilmemiş yeni
   çizimin kitabı ilk kayıtta dosyanın anahtarına taşınır; kalıcı bir adın kitabı başka bir kalıcı
   ada (Farklı kaydet, buluta yükleme) kopyalanır, eskisi kendi kitabını tutar; başka bir çizim
   açılınca kendi kitabı okunur.
7. **“+” bir menü açar** (Yeni pafta, Şablondan pafta…, .kpafta dosyasından…): şablon yolu bir
   tıklamada ve motor yokken de düğme ölü kalmaz.
8. **Pafta öndeyken** sağ dok gizlenir, denetçi onun yerini alır; alt panel ve komut satırı kalır.
   Çizim alanının kısayolları (çizim araçlarının harfleri, geri alma, F3, F7 …) paftada tutulur ve
   durum çubuğunda neden söylenir; şeritten ya da komut satırından çalıştırılan bir çizim komutu
   önce Model sekmesini öne getirir. Hızlı erişimin Geri al/Yinele'si henüz modelinkidir (B: pafta
   yığınına bağlanacak).
9. **Ctrl+] / Ctrl+[** tuşun yeriyle (`KeyboardEvent.code`) okunur: uygulamanın tuş haritası `]`'yi
   yazamaz ve Türkçe Q bu karakterleri AltGr ile yazar; Türkçe Q'da aynı yerdeki tuşlar Ctrl+Ü /
   Ctrl+Ğ'dir. Ek olarak Home sayfayı sığdırır (modeldeki “Tümünü göster” gibi), + ve − yakınlaştırır.
10. **Karışık değer “—”** (tasarımın H4'ü); modelin Öznitelikler paneli aynı durumda “Çeşitli” der.
11. **Yan sütunların genişliği** yalnız oturumda tutulur (`kentos.ui.v1` paylaşılan yerleşimdir);
    dar pencerede sütunlar kendiliğinden daralır.
12. **Komut satırından** yazılan ve devre dışı olan bir pafta komutu (mevcut davranış) sessiz
    kalır; şerit ve menüler nedenini gösterir.

Web ajanı, B bölümü (2–3 Ekim):

13. **A'nın 1–4. sapmaları ve 8'in son cümlesi kalktı.**
    - Önizleme verisi (`preview.ts`), TS'teki vuruş testi (`pick.ts`), galerinin TS sırası ve ortak
      araç listesi silindi.
    - Öğeler motorun çizim planıyla boyanır.
    - Hızlı erişimin Geri al / Yinele'si paftada paftanın yığınını kullanır (integration §3 W-10).
    - 5–7 ve 9–12 sürüyor.
14. **Harita resminin boyu sınırlı.** Ekranda en uzun kenar 4096 pikseldir; daha çok yakınlaşınca
    resim büyütülür. Küçük haritalar en az 640 piksel çizilip küçültülür, yoksa ince çizgiler açık
    zeminde kararır. Dışa aktarmada resim dpi'ye göre 2048 piksellik karolarla çizilir. Tarayıcı
    PNG'yi en çok 16 384 piksel kenar ve 120 milyon piksel yazar; büyüğü için pencere daha düşük
    dpi ya da SVG önerir.
15. **Harita, görünüm penceresinin geometri deposunu okur** (W-11): çizim ikinci kez kurulmaz.
    Bu yüzden haritanın katman listesi çizimde görünen katmanları süzer: modelde gizlenmiş bir
    katman listede olsa da çizilmez.
16. **Katman temaları uygulamada yok.** Şablondan tema ile gelen harita görünen bütün katmanları
    çizer; denetçi bunu söyler.
17. **Lejant simgeleri resimdir.** `legendOf` ve `drawSymbolPreview` ile 300 dpi'de çizilip motora
    resim olarak verilir; SVG'de PNG olarak gömülür.
18. **Yazılar tarayıcının yazı tipiyle çizilir.** Satır, motorun ölçtüğü genişliğe yatay olarak
    uydurulur (`fontKerning: none`); yazı tipi motorun ölçü tablosundakinden ayrı düşerse
    harfler biraz daralır ya da genişler, satırın genişliği doğru kalır.
19. **Grup saydamlığı yaklaşık.** `globalAlpha` çarpılır; bir grubun üst üste binen çocukları
    ayrı ayrı saydamlaşır, grup tek katman gibi saydamlaşmaz.
20. **“Karelaj ekle”** kipin şablonlarındaki ilk karelajdan başlar (aralık 0: ölçekten). Motorda
    yeni karelaj üreten bir işlev yok.
21. **Elipsoit tablosu** (GRS80, WGS84, Uluslararası 1924) motorun CRS girdisi için
    `app/sheet/inputs.ts`'tedir. Yeri `geo/crs.ts`; birleştirmeden sonra taşınabilir.
22. **Atlas düzenleyicisi yok.** `sheet.atlas` devre dışıdır ve nedenini söyler. Denetçi atlas
    haritasında “Atlas haritası: yeri ve ölçeği her atlas sayfasının nesnesinden gelir.” der.
23. **Ana sayfa:** seçme, Yok, seçili öğelerden ana sayfa yapma ve ayırma var. Ana sayfanın
    öğeleri yerinde düzenlenmez; Öğeler'de tek, kilitli bir satırdır.
24. **Öğeler'de sürükle-bırak** yalnız kardeşler arasında sırayı değiştirir. Gruba sokmak ya da
    gruptan çıkarmak Grupla / Grubu çöz iledir.
25. **`@kullanici`** oturum açılmamışsa boştur; ön denetim bunu söyler.
26. **IndexedDB `kentos.sheets.v1` sürüm 2:** paftalardaki resimler için `assets` deposu (W-9).
27. **Tek geçmiş:** paftanın geri alma yığını kitabın tamamınındır (bütün paftalar ve ana
    sayfalar), sekme başına değil.
28. **Görünüm paneline “Seçime yakınlaş”** eklendi (tasarımın §11'inde yok). Seçili öğeleri
    sığdırır; masanın ve öğelerin sağ tık menüsünde de vardır.

`.kpafta` zarfı masaüstününkiyle aynıydı. C bölümünde web, çekirdeğin tek kodeğine geçti
(`encodeKpafta` / `decodeKpafta`); TypeScript'teki biçim kopyası silindi.

Web ajanı, C bölümü (3 Ekim):

29. **Bir şablon kendiliğinden buluta gitmez.** “Bu cihazda” şablon, kullanıcı “Buluta eşitle”
    deyince yüklenir (galeriden ya da paylaşım penceresinden). Tasarımın “Bu cihaz: eşitlenmemiş
    şablonlar” kaynağı böyle kalır. Eşitlenmiş olanlar bundan sonra kendiliğinden eşitlenir.
30. **Eşitleme motorla başlar.** Plan motorundur (`planSync`), bu yüzden eşitleme ve olay dinleme
    pafta kipine ilk girişte ya da galerinin ilk açılışında başlar, uygulamanın açılışında değil
    (paket hiç açılışta yüklenmez). Ondan önce olay dinlenmez.
31. **Olay yoklaması web'de 8 sn bekler** (sunucunun sınırı 25). Geliştirme sunucusunun vekili isteği
    10 sn'de keser; kesilen istek sunucu yokmuş gibi görünürdü. Üretimde de ters vekilin zaman
    aşımına takılmasın diye kısadır. Değişiklik yine hemen gelir: sunucu bekleyen isteği uyandırır.
32. **Bulut kopyaları hesabındır.** Aynı tarayıcıda başka bir hesap açılırsa bir öncekinin
    kopyalarını görmez ve eşitlemez. Oturum kapatılınca gizlenir. Sunucudan cevap gelmezken
    (çevrimdışı açılış) son hesabın kopyaları gösterilir ki kullanılabilsin. Kopyalar `templates`
    deposundadır, her kayıtta hesabı ve bulut durumu (W-9).
33. **Bulutun verdiği kimlik:** yüklenen şablon sunucunun kimliğini alır; eski kimlik kayıtta
    kalır (`formerIds`). Eski kimlikle yapılmış paftalar “Yeni sürüm var”ı yine duyar. Bulutun
    revizyonu 1'den başladığı için eski kimliğin revizyonu çevrilir (`formerRevision`).
34. **Komutların anahtarı içerikten türer** (eşitleme komutlarında): cevabı kaybolan istek aynı
    anahtarla yeniden gider, sunucu yineler, çift şablon olmaz. Paylaşma ve kaldırma her seferinde
    yeni anahtar alır.
35. **Düzenleme izni giden şablon:** düzenleyici görüntüleyiciye düşerse bekleyen değişikliği
    “… (bu cihazdaki kopya)” adıyla bu cihazda ayrı bir şablon olur. Bulutunki indirilir;
    kullanıcıya söylenir.
36. **“Çakışma” rozeti**, çakışmanın ayırdığı kopyada durur. Kopya silinene ya da yeniden
    kaydedilene dek kalır; ayrıca “gördüm” düğmesi yok.
37. **Şablonun soruları** `Template.variables`'ın hepsidir: tipte “kullanırken sor” işareti yok,
    tasarım §12 hepsini soru sayar. “Şablon olarak kaydet” değerleri soruya çevirirken
    varsayılanlarını boşaltır (çekirdeğin davranışı). Soruların penceresi Kullan'da ve Yeni
    pafta'da açılır, şablonu düzenlemede açılmaz.
38. **Durum çubuğu** yalnız eşitleme yolunda değilken bir hücre gösterir: “Şablonlar: çevrimdışı”,
    “eşitleniyor”, “eşitlenemedi”. Eşitlenmişken gizlidir.
39. **“Şablonlarıma kopyala”:** stil yöneticisi aynı işi “Kopyala → Kitaplığıma / Projeye” ile
    yapar; galeride tek hedef olduğu için düğme hedefi adıyla söyler, aynı iyelik biçimiyle.
40. **Proje türü yok** (A'nın 5'i sürüyor), **kurum şablonları, e-postayla davet, sahiplik devri ve
    yeni sürümü paftaya uygulamak** 2. aşamadadır (tasarım §12, §13).

Web ajanı, D bölümü (3 Ekim):

41. **Dosya adı motorun yazısıyla, geçici bir metin öğesinden okunur.** Motorda `[% … %]` şablonunu
    tek başına değerlendiren işlev yok. Web, paftanın `export.fileName`'ini kitabın bir kopyasına
    eklenen geçici bir metin öğesine yazar ve çizim planındaki yazıyı okur (`exportName`). Kitap
    değişmez; sonuç ekrandaki yazıyla aynıdır. Çekirdeğe bir `exportName` gelirse bu yol onunla
    değişir.
42. **Harita çerçevelerinde piksel, kâğıdın CSS pikselidir** (1 px = 25,4/96 mm). Bu yazılar, nokta
    simgeleri ve piksel birimli çizgi kalınlıkları için geçerlidir. Böylece pafta %100'de ekranda
    nasıl görünüyorsa öyle basılır; her dpi'da ve PDF'in vektörlerinde aynı boydadır. B'de bunlar
    paftanın yakınlaştırmasında ekran pikseliydi; dışa aktarmada aygıt pikseli olduklarından dpi
    büyüdükçe küçülüyordu. Masaüstünün PDF'i aynı birimi kullanır (`sheet_pdf.rs` `PX_MM`).
43. **Vektör haritada yazılar bütün çizginin üstündedir.** Harita iki geçişle verilir: önce her
    katmanın çizgi ve alanları alttan üste, sonra her katmanın yazı maskeleri ve yazıları. Ekrandaki
    harita çerçevesi de böyle çizer. Bir katman iki geçişte de aynı kimlikle gider; çekirdek
    grupları kimlikle tuttuğu için PDF'te yine tek katmandır, sırası çizimin sırasıdır.
44. **Çekirdeğin harita tiplerinde olmayanlar atılır:** çizgi deseninin başlangıç kayması
    (`dashOffset`), yazının en/boy oranı (`widthFactor`) ve hale kalınlığı. Halenin yalnız rengi
    gider; kalınlığını çekirdek seçer (ekrandaki hale 3 px'tir).
45. **Yedek resim haritanın tamamı içindir.** Bir katman vektörle yazılamıyorsa harita bütünüyle
    seçilen dpi'da PNG olur: çekirdeğin `MapContent`'i harita başına ya vektör ya resimdir. Pencere
    hangi katmanın neden yazılamadığını söyler. Vektörle yazılamayanlar şunlardır:
    - desen dolgusu, resimli dolgu;
    - resimli simge, yazı simgesi;
    - yumuşak kenarlı çizgi, düz noktalar.
46. **GeoPDF yalnız TM ve UTM'de** yazılır. Çekirdek köşeleri bu sistemlerde kendisi hesaplar;
    başkasında ev sahibi verebilir (`PdfMap.corners`). Kayıttaki öbür sistemler şunlardır:
    - coğrafi TUREF ve WGS 84 (birimi derece, paftanın ölçeği orada anlamsız);
    - Web Mercator (altlık karolar için).

    Bunlarda GeoPDF kapalıdır; pencere nedenini söyler.
47. **Yazdır:** PDF gizli bir çerçevede açılır ve tarayıcının yazdırma penceresi çağrılır. Tarayıcı
    reddederse PDF yeni sekmede açılır, oradan yazdırılır. “Yeni sekmede aç” sekmeyi tıklamanın
    içinde açar; tarayıcılar sonradan açılanı engeller. Yazılamazsa boş sekme kapanır.
48. **Ctrl+P pafta öndeyken `sheet.print`'tir.** Genel `file.print` (Ctrl+P, “Yazdır ve pafta”)
    `main`'de hâlâ “henüz kullanılamıyor” der. Pafta öndeyken paftanın kısayolu ondan önce gelir.
    Birleştirmede `file.print` pafta öndeyken `sheet.print`'e, değilken pafta galerisine
    gidebilir.
49. **Birden çok paftanın dosya adı** “<çizimin adı> paftaları.pdf”tir. Kitabın bir dışa aktarma
    varsayılanı yok; tek paftada paftanınki kullanılır.
50. **PDF'te dpi alanı** yedek resimlerin çözünürlüğüdür (“Yedek resimlerin çözünürlüğü”). Ön
    denetimin düşük çözünürlük denetimi de bu değerle yapılır. Varsayılanı paftanın dışa aktarma
    dpi'sıdır.
51. **Ön denetim pencerenin başındadır**, biçimin hemen altında; PDF'in seçenekleri ondan sonra
    gelir. Birden çok pafta seçildiyse bulgunun başında paftanın adı yazar.

Web ajanı, D+ (3 Ekim):

52. **Harita yazısının dayanağı.** Kural iki platform için kararlaştırıldı: dayanağı harita
    çerçevesinin içindeki yazı yazılır ve çerçevede kesilir; dışındaki yazılmaz, harfleri içeri
    uzansa da. Dayanak, yazının yazıldığı noktadır:
    - metin nesnesinde kendi noktası (`p`), hizası ne olursa olsun;
    - çizginin adında yazıldığı parçanın ortası;
    - öbürlerinde (alanın ya da noktanın adı, ölçünün değeri, işaret çizgisinin notu, bloğun
      yazısı) kaydının noktası.

    Kural tek yerdedir (`app/sheet/frameLabels.ts`). Ekrandaki harita çerçeveleri, galerinin
    küçük resimleri, PNG ve SVG resimleri ve PDF'in vektör yazıları aynı kayıtları alır.
53. **Yazdır ve pafta'nın “Geliştirme aşamasında” notu öndekine bağlıdır.** Pafta öndeyken menüde
    ve aramada hazır görünür ve paftayı yazdırır; Model öndeyken eskisi gibidir (W-17).
54. **Bulut görüntülerinin adı boyu taşır:** `<n>-<sahne>-<light|dark>-<1440|1100>.png`. C'deki
    boysuz adlı resimler silindi.
55. **Bileşik denetimler tek Tab durağıdır.** İçlerinde oklarla gezilir:
    - galerinin kaynakları: ↑ ↓, Home, End; gelinen kaynak gösterilir;
    - galerinin kartları: oklar; seçili kart Tab'ı alır, yoksa ilki;
    - denetçinin sekmeleri: ← →, Home, End.

    Adlar:
    - kaynak “Sistem, 8 şablon”; notu (neden boş, ne zaman gelecek) açıklamasıdır;
    - Ön denetim sekmesi “Ön denetim, 6 bulgu”;
    - paftalar listesinin satırı bir kez: “İfraz paftası, A3 yatay · 420 × 297 mm”;
    - denetçinin katlama düğmesi “Denetçi”.

    Galerinin sekme listesi artık içte bir `div.tgal__tabs`'tır; sıra notu listenin dışında durur.

Web ajanı, E bölümü (3 Ekim):

56. **Denetçideki modelin değeri çekirdeğin kendi yazısından okunur.** Web hiçbir şeyi yeniden
    projekte etmez (CLAUDE.md §5); TM'den enlem-boylama çeviren bir işlevi de yok, motor da bunu
    vermiyor. Bu yüzden `magneticField`'ın haritanın merkezinin enlem-boylamı gerekir ama web
    bunu bilemez: sarmalayıcı yazıldı ve NOAA'nın sınama değerleriyle sınandı, denetçi onu
    çağırmıyor.
    - Denetçi, kitabın bir kopyasında okun manyetik kuzeyi notuyla gösterdiği çizim planını okur:
      “6°19′ D (WMM2025, 2026-10)”. Kitap değişmez; kâğıtla denetçi aynı şeyi söyler.
    - Motora bir `northValues` gelirse bu yol onunla değişir: yakınsama, sapma, kaynak, tarih,
      yer. `exportName` Sapmalar 41'i böyle kapattı.
57. **`magnetic_out_of_model` uyarısının düzeltmelerini web ekler.** Çekirdek bu uyarıya düzeltme
    vermiyor; cümlesi ise “tarihi denetleyin ya da sapmayı elle girin” diyor. Web iki düğme ekler:
    - “Değişkenleri aç” (`sheet.variables`);
    - “Sapmayı elle gir” (çekirdeğin `magnetic_no_place` için verdiği işlemin aynısı).

    Çekirdek bunları verirse web'inkiler kalkar (`fixesOf`).
58. **“Sapmayı elle gir” açılınca değer modelden başlar.** Daha önce bir değer girilmemişse,
    modelin değeri kâğıttaki gibi dakika duyarlığıyla alınır (6°19′ → 6317 milliderece).
59. **B'deki “Manyetik sapma” alanının birimi yanlıştı.** Alan, millidereceyi derece gibi
    gösteriyordu. Şimdi “Elle sapma” derecedir, kitaba milliderece yazılır; yalnız elle girilirken
    görünür.
60. **Dosya adı artık motorun `exportName`'inden gelir.** Sapmalar 41'deki geçici metin yolu
    silindi.
61. **Bulut sözleşmesinde `SheetTemplateList.organizations` alanı belirdi.** F bölümünün işi, Rust
    tarafında sürüyor. Sahte bulut (`templateCloudTesting.ts`) bu alan için boş liste verir; derleme
    yeşil kaldı.

Web ajanı, F bölümü (3 Ekim):

62. **Eksik değer işareti web'de bir kez yazılı; motordan okunmuyor.** Not “motordan okuyun”
    diyordu, ama motor işareti vermiyor (`EngineInfo`'da alanı yok). Çizim planından okumak için bir
    kitap kurup çizdirmek gerekirdi.
    - İşaret `product/sheet/marks.ts`'te (`missingMark`); üç arayüz metni oradan alır.
    - `marks.test.ts` çekirdeğin kâğıda yazdığıyla aynı olduğunu sınar: işaret değişirse test düşer.
    - Motora bir alan gelirse metinler onu okur.
63. **Dış çizgi düzeltmesi yalnız PDF'in vektörlerinde.** Not ekranı, küçük resimleri, PNG'yi ve
    SVG'yi de saydı. Bunlar haritayı çizim hattının resminden alır (`mapFrames.ts`); orada daire
    kapalı, delik boştu. Hata yalnız `corePaths`'e giden dış çizgilerdeydi. `sheet-pdf.mjs` ikisini
    de denetler: haritanın resminde ve PDF'te delikli simgenin ortası beyaz, dolu dairenin ortası
    dolu.
64. **Sapmalar 56, 57 ve 61 kapandı.**
    - 56: denetçi değerleri `northInfo`'dan okur. Açıyı sayıdan kendisi yazar (`degreesText`),
      çekirdeğin biçimiyle: dakikaya yuvarlı, `'` ile. `northInfo` yazı vermiyor; `sheet-north.mjs`
      denetçide de kâğıtta da aynı açıyı bekler (“6°19' D”).
    - 57: `magnetic_out_of_model`'in düğmeleri çekirdeğin; web'inkiler (`fixesOf`) silindi.
    - 61: sahte bulut kurumları sunucunun kurallarıyla taşır.
