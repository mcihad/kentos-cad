# ADR 0077: Masaüstünde şeridin “Komut ara” kutusu

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.5, §4.6; DESIGN.md §7.3; TODOS.md `UI-11`; ADR 0051 (pencereye sığan şerit), 0058 (komut arama, Alt+Q), 0064 (sekme satırı)
- **Kaynak:** web kodu, koddan okundu:
  - `ui/ribbon/search.ts` (`RibbonSearch`);
  - `ui/ribbon/Ribbon.ts` (`where`, `reveal`, `fitBar`'ın `data-tight` basamakları);
  - `core/commands.ts` (`search`);
  - `styles/ribbon.css` (`.rsearch`).

## Bağlam

- Web'in sekme satırında adın sağında bir “Komut ara…” kutusu var.
- Masaüstünde Alt+Q komut satırının listesini açıyordu (ADR 0058). TODOS.md bunu “Açık: şeritteki arama kutusu” diye tutuyordu.

## Karar

### Kutu (KentOS UI `widget::search_box`)

- **Görünüş:**
  - 206 × 24 px; içinde solda büyüteç, sağda “Alt+Q” ipucu.
  - Odakta vurgu kenarı alır ve ipucu gizlenir. Üzerine gelince kenar koyulaşır.
- **Yer:** sekme satırında, çizimin adından sonra, koordinat sistemi düğmesinden önce.
- **Liste:**
  - Yazdıkça kutunun altında açılır; kutunun sağ kenarına hizalı, pencereye sığacak kadar içeridedir.
  - 390 px genişliktedir; en çok dokuz satır gösterir.
  - Satırda komutun ikonu, adı, altında yeri, kısayolu ve “Şeritte göster” iğnesi vardır.
  - Satırın altındaki yazı komutun şeritteki yeridir, “Değiştir › Değiştir” gibi: önce Giriş dışındaki sekmeler, sonra Giriş aranır. Şeritte olmayan komutun kategorisi yazar. Masaüstünde çalışmayan komut soluk görünür ve “Web'de var; masaüstüne henüz taşınmadı” der.
  - Eşleşme yoksa: “‘x’ ile eşleşen komut yok. Komut satırındaki takma adlar da aranır (ör. L, CIZGI).”
- **Sıralama** (web'in `search`'ü): önce takma adı yazılana eşit olanlar, sonra takma adı onunla başlayanlar (kısa ad önce), sonra adı onunla başlayanlar, en son adında geçenler. Türkçe harfler katlanır: CIZGI Çizgi'yi bulur.
- **Tuşlar:**
  - ↑ ↓ satır seçer, uçlardan öbür uca döner.
  - Enter çalıştırır; Alt+Enter yerini gösterir.
  - Esc yazıyı siler; kutu boşsa odağı bırakır.
- **Fare:** üzerine gelinen satır seçilir, tıklanan çalışır, iğne yerini gösterir.
- **Kapanma:** dışarı tıklamak ya da odağın gitmesi listeyi kapatır; yazı kalır ve kutuya dönünce liste yeniden açılır.
- **Alt+Q** (`view.commandSearch`) kutuyu odaklar ve içindekini seçer.

### Çalıştırma ve yerini gösterme

- **Çalıştırmak:** kutu boşalır ve klavyeyi bırakır; araç başlarsa klavye çizimindir (web'in `view.focus()`'u).
- **Yerini göstermek:**
  - Komutun sekmesi açılır.
  - Düğmesi 1,6 saniye 2 px vurgu çerçevesiyle çizilir. Bölünmüş düğmedeki komutta bölünmüş düğme, seyrek araçlardakinde panelin ▾'i, tek düğmeye katlanmış paneldekinde o düğme çerçevelenir.
- **Dar pencere** (web'in `fitBar`'ı): önce ipucu, onunla birlikte koordinat sisteminin adı gider. Sonra kutu büyütece iner; büyütece basmak kutuyu açıp odaklar. Çizimin adı en son kesilir.

### Katman ara aynı kutuyla (web ajanının `c63cd77`'si)

- Katmanlar panelinin “Katman ara”sı da bu kutudur: panel genişliğinde, 28 px.
- **Eşleşen grup** bütün katmanlarıyla görünür. Eşleşmenin altındaki her şey listelenir; eşleşmelerin üstündeki gruplar eskisi gibi kalır. ADR 0075'te grubun adı tek başına çocuklarını tutmuyordu.
- **Kutunun tuşları:**
  - ↓ klavyeyi ağaca verir, listenin ilk satırında.
  - Esc yazıyı siler; boş kutuda klavyeyi çizime bırakır.

## Web'den ayrılanlar

- **Daraltılmış şerit:** masaüstünde sekme tıklamasıyla açılan geçici şerit yok. Yerini göstermek şeridi açar.
- **Katlanmış panel:** web katlanmış paneli açıp içindeki düğmeyi gösterir; masaüstü katlanmış panelin düğmesini çerçeveler.
- **Eşit puanlıların sırası:** web komutları kaydediliş sırasıyla, masaüstü envanterin sırasıyla dizer.

## Doğrulama

- `ribbon_search::tests`: sıralama, Türkçe harfler ve sınır; satırın yeri, yerini göstermek, çerçevenin süresi, çalıştırmak.
- `layer_tree::tests`: eşleşen grubun katmanları, kutudaki ↓.
- Görüntüler, koyu ve açık, 1440×900 ve 1100×650:
  - `cargo test -p kentos-desktop ribbon_search::screens -- --ignored --nocapture`: `.run/shots/komut-ara-*`, `komut-ara-yok-*`, `komut-ara-yeri-*`. 1100 px'te kutu büyütece inmiştir.
  - `layer_tree::screens`: `katman-ara-*`.
- Geçenler: `pnpm rust:test:desktop` (KentOS UI, vitrin ve masaüstü), `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e` (web ajanının “Katman ara” duman denetimi dahil), `pnpm inventory:check`.
