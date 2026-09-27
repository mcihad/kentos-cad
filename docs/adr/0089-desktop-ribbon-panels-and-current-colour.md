# ADR 0089: Masaüstünde şeridin kendi panelleri, bağlamsal Seçim sekmesi ve yeni nesnelerin rengi

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.5, §4.7 (CMD-07); ADR 0017 (masaüstü kabuğu ve şerit), 0051 (pencereye sığan şerit).
- **Kaynak:** web'in `ui/ribbon/panels.ts` (`builtinPanel`), `ui/toolbar/fields.ts` (alanlar ve listeleri), `ui/ribbon/Ribbon.ts` (bağlamsal sekme, `updateContextual`), `tools/drawTools.ts` (`colour()`), `tools/createCommand.ts` (`writeObjects`), `styles/ribbon.css` (`.rsel__*`, `.ribbon__tab--context`, `.ribbon__count`). Referans görüntüler web ajanının `ribbon-*` görüntüleridir.

## Bağlam

Web şeridinin üç paneli komut düğmesi değildir; web onları kendisi çizer (envanterde `{ "builtin": … }`):

- **Katmanlar** (Giriş): etkin katmanın açılır alanı; Yeni katman, Yeni grup, Katmanları göster, Katman paneli.
- **Özellikler** (Giriş): yeni nesnelerin rengi, çizgi tipi ve kalınlığı; çizim ölçeği.
- **Seçim** (bağlamsal Seçim sekmesi): seçimin sayısı ve türleri; seçim komutları.

Masaüstü bu panelleri hiç çizmiyordu. Bağlamsal sekmeyi katalogdan çıkarıyordu. Çizim araçları ürün komutlarına hep `color: None` yazıyordu: web ise etkin rengi girdiye açıkça yazar (CMD-07).

## Karar

### Paneller: `apps/desktop/src/ribbon_panels.rs`

**Katmanlar:**

- Etkin katmanın alanı: katmanın renk örneği ve adı.
- Menüsü web'in `layerField`'i gibidir:
  - her grup, yolu (“Kadastro / Yapı”) ile bir başlıktır;
  - her katman renk örneği ve nesne sayısıyla bir seçenektir;
  - kilitli katman seçilemez.
- Seçmek, katman ağacında çift tıklamak gibi katmanı etkin yapar: dosyada saklanır ama düzenleme değildir.
- Altında iki sıra düğme: Yeni katman ve Yeni grup; Katmanları göster ve Katman paneli.
- Açıcı (↘): Katman stili….

**Özellikler:**

- Renk, Tip ve Kalınlık alanları: önce “Katmana göre”, sonra web'in seçenekleri (8 renk, 4 çizgi tipi, 6 kalınlık).
- Ölçek alanı: 1:500, 1:1000, 1:2000, 1:5000, 1:25000. Projenin ayarıdır: değiştirmek çizimi kaydedilmemiş yapar, geri alma adımı değildir (web'deki gibi).
- Açıcı (↘): Proje ayarları.

**Seçim:**

- Sayı vurgu renginde ve büyük; altında “nesne seçili”.
- Bir çizginin ardında türler: çoğu olan önce, Türkçe küçük harfle (“ışın”); üçten fazlaysa ikisi ve “n tür daha”.
- Seçime yakınlaştır, Seçimi kaldır, Seçimi ters çevir.

**Daralma, web'in adımlarıyla:**

- Katmanlar: alan 204 px, 2. seviyede 150 px.
- Özellikler: alanlar 188, 162 ve 132 px; ölçek alanı 134 ve 112 px.
- Seçim: türler 2. seviyede gizlenir.
- 2. seviyede düğmeler yalnız ikondur.
- 3. seviyede panel tek düğmeye katlanır.
  - **Web'den fark:** web katlanan panelin denetimlerini açılır bir kutuda gösterir. Masaüstünde her katlanan panel gibi bir menü açılır (ADR 0051):
    - Katmanlar: “Etkin katman: …” alt menüsü ve dört komut;
    - Özellikler: “Yeni nesnelerin özellikleri” başlığı, Renk, Çizgi tipi, Çizgi kalınlığı ve Çizim ölçeği alt menüleri, her biri o anki değeriyle;
    - Seçim: sayı başlığı ve üç komut.

### Bağlamsal Seçim sekmesi

Seçim varken sekmelerin sonunda “Seçim” görünür:

- adı vurgu renginde, yanında sayı rozeti, üstünde ince vurgu çizgisi;
- tıklanınca açılır, seçili sekme yine hatırlanır; başka sekme seçilince kapanır;
- seçim boşalınca sekme kaybolur, şerit önceki sekmeye döner;
- yeni seçim onu kendiliğinden açmaz;
- Komut ara'nın “Şeritte göster”i komutun kendi sekmesini açar, bağlamsal sekmeyi değil.

Web'in `select` ve `updateContextual` kurallarıdır: `App::choose_tab`, `App::shown_tab` ve `App::update`'teki sıfırlama.

### Yeni nesnelerin rengi

Renk `kentos_interaction::Draft::color`'dadır; ayar değişince korunur, çünkü oturumun değeridir, ayar değildir. Web'deki gibi her çizim aracı rengi ürün komutunun girdisine açıkça yazar:

- çizgi, yay ve daire;
- nokta ve kot noktası;
- kapalı alan, çoklu çizgi, parsel, dikdörtgen, düzgün çokgen, revizyon bulutu;
- `cad.entities.create` ile yazan bütün araçlar: elips, eğri, yardımcı çizgi, ışın, halka, yazı, ölçü, tarama, paralel, dikler, böl, içine tıklayarak alan.

“Katmana göre” renk yazmaz. Hesap pencerelerinin noktaları renk almaz (web'in `addPoints`'i gibi).

Çizgi tipi ve kalınlık oturumda tutulur: web'de de hiçbir araç onları okumaz.

### KentOS UI eklemeleri

- `Group::stepped`: seviyeyle daralan kendi görünüşü ve katlanınca menüsü. `Group::custom` onun tek genişlikli hâlidir.
- `ribbon::Choice`: şeritte açılır alan. Önde soluk adı, renk örneği, değeri ve oku vardır; sığmayan değer “…” ile kısalır.
- `Button::icon_only`, `Button::measure`: yalnız ikonlu biçim ve genişliği. `Button::menu_entry` artık dışarı açık.
- `Ribbon::contextual_tab` ve `style::container::count`: bağlamsal sekme ve sayı rozeti.
- `typography::elide`: genişliğe sığdırmak için “…” ile kısaltma.

### Tek kaynak

Web'in `DRAW_COLORS`, `LINE_TYPE_LABEL` ve `LINE_WEIGHTS`'i masaüstünde üç yerde kopyalanmıştı: katman menüsü, Öznitelikler ve şerit. Artık tek yerde, `ribbon_panels.rs`'te durur.

## Doğrulama

- **`kentos-interaction` `tests/colour.rs`:** çizgi, kapalı alan, daire, dikdörtgen, nokta ve elips etkin rengi yazar; “Katmana göre” iken renk yazılmaz.
- **`ribbon_panels::tests`:**
  - Katmanlar, Özellikler ve Seçim adım adım daralır.
  - Katman menüsü örnek çizimin ağacını verir: başlık, sayılar, etkin ve kilitli katman. Seçilen katman etkin olur.
  - Seçilen renk sonraki çizgiye gider; ayar değişikliği onu silmez; “Katmana göre” renk yazmaz.
  - Tip, kalınlık ve ölçek web'in sözleriyle görünür; ölçek çizimi kaydedilmemiş yapar.
  - Seçim sekmesi seçimle gelir; seçim boşalınca önceki sekmeye dönülür; yeni seçim onu açmaz.
  - Türler Türkçe küçük harfle yazılır.
- **`ribbon_tests`:** her sekme yeni panellerle de 1100 piksele sığar.
- **Görüntüler:** `ribbon_panels::tests::screens`, koyu ve açık tema, 1440 ve 1100 piksel (`.run/shots/serit-panel-*`):
  - Giriş;
  - seçilmiş değerler;
  - Seçim sekmesi;
  - seçim varken Giriş: bekleyen bağlamsal sekme.
