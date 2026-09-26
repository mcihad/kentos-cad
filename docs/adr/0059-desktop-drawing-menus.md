# ADR 0059: Masaüstünde çizim alanının sağ tık menüleri ve tek seferlik kenet

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.6, §4.7; TODOS.md `UX-06`, `UX-07`, `UI-11`; ADR 0018 (tuş ve fare anlamları), 0029 (seçim ve kenet), 0056 (onay almayan araçlar)
- **Sahibin yönü (26 Eylül):** web'deki araçlar, düzenleyiciler, sağ tık menüleri ve davranışlar masaüstüne birebir taşınır.

## Bağlam

- Web'de çizim alanının sağ tuşu üç iş görür (`ui/shell/viewportMenus.ts`, `ViewportController.onRightDown`/`onRightUp`):
  - **Kısa sağ tık:** onay alan bir komut çalışıyorsa Enter'dır. Komut yoksa ya da onay almıyorsa (Kaydır, Pencere yakınlaştır) boşta menüsünü açar.
  - **300 ms basılı tutmak:** komut çalışırken komut menüsünü, komut yokken boşta menüsünü açar.
  - **Shift ve sağ tuş:** tek seferlik kenet menüsünü hemen açar.
- **Tek seferlik kenet:** sonraki tıkta yalnız seçilen türe kenetlenir; sürekli kenet (F3) kapalıyken de çalışır. Sol basışta ya da başka bir komutta düşer. Komut satırında "Sonraki tık: …" ve × ile görünür.
- Masaüstünde kısa sağ tık yalnız Enter'dı. Komut yokken ya da onay almayan araçta hiçbir şey yapmıyordu (ADR 0056'da not edilmişti). Basılı tutmanın ve Shift'in anlamı yoktu.

## Karar

### Sağ tuş (`apps/desktop/src/drawing_menus.rs`, `viewport.rs`)

- Basılı tutmayı çizim alanı bileşeni kendisi ölçer; ayrı zamanlayıcı yoktur.
  - Basışta Iced'den 300 ms sonrası için bir kare ister (`request_redraw_at`). O karede tuş hâlâ basılıysa `RightHeld` bir kez söylenir.
  - Kare gelmeden tuş geç bırakılırsa menü bırakışta açılır.
  - Basılı tutmadan sonraki bırakış Enter değildir.
- Etkileşim izleri kısa sağ tıkı eskisi gibi oynatır: basılı tutma olmadığından izler değişmedi. İlk yazımda zamanlayıcı ayrı bir iş parçacığındaydı. İz oynatıcısı görevleri hemen bitirdiği için zamanlayıcı bırakıştan önce doluyor, `arrays` izinin sağ tıkla bitirmesi bozuluyordu.
- Menü, uygulamanın açıp kapattığı bir KentOS UI bağlam menüsüdür (`ContextMenu::controlled`). Uygulama yerini verir. Menü komut seçilince, dışarı tıklanınca ya da Esc'le kapanır ve bunu bildirir.

### Menüler (web'in öğeleri, sırasıyla)

- **Boşta menüsü:**
  - "Yinele: {son komut}" (Enter);
  - Tümünü göster, Seçime yakınlaştır, Kaydır;
  - Tümünü seç, Seçimi kaldır;
  - Taşı, Kopyala, Panoya kopyala, Yapıştır, Sil;
  - Koordinat listesi.
  - Komutlar katalogdan gelir: başlık, ikon, kısayol. Çalışamayanlar soluktur, örneğin seçim yokken Seçime yakınlaştır.
- **Komut menüsü:**
  - başlıkta komutun adı; Onayla / bitir (Enter), İptal (Esc);
  - istemin seçenekleri, değerleriyle (Enter ve Esc dışında);
  - Tek seferlik kenet ▸;
  - Kenetleme, Orto, Kutupsal izleme, Nesne izleme (işaretleriyle);
  - Tümünü göster.
- **Kenet menüsü:**
  - "Tek seferlik kenet (sonraki tık)" başlığı;
  - Uç nokta, Orta nokta, Kesişim, Merkez, Dik, Teğet, Çeyrek, Nokta, En yakın (web'in kenet işaretli ikonlarıyla);
  - Kenet ayarları….
- KentOS UI menüsünde işaretlenebilir öğe, işaretsizken kendi ikonunu gösterir; önceden boş kalıyordu. İşaretliyken ✓ gösterir.

### Tek seferlik kenet

- Seçilen tür, seçildiği komut sürdükçe geçerlidir. Sonraki sol basışta ya da komut değişince düşer.
- Kenet hesabında taslak ayarının kopyası kullanılır: kenet açık, türler yalnız seçilen.
- Komut satırında istemin yanında "Sonraki tık: {tür}" çipi görünür; × ile düşer.

## Bu dilimde olmayanlar

- Web'in tutamaç öğeleri: imlecin altındaki köşeyi silmek, kenarın ortasına köşe eklemek, kenarı yaya ya da düze çevirmek. Masaüstünde tutamaçlar henüz yok.
- Komut menüsünde Nokta hesapla ▸: masaüstünde nokta hesaplayıcı yok (`UX-07`).
- Çizimin üstündeki komut şeridinde (`drafting.commandBar`, varsayılan kapalı) tek seferlik kenet çipi.

## Doğrulama

- `drawing_menus::tests`:
  - kısa sağ tık, komut yokken boşta menüsünü açar, Çizgi'de Enter'dır;
  - basılı tutmak boşta ve komut menüsünü açar, Enter değildir; Shift ile kenet menüsü açılır;
  - tek seferlik kenet sol basışta ve başka bir komutta düşer.
- `viewport::tests`: basılı tutma bir kez söylenir; ardından bırakış tık değildir; kare gelmeden geç bırakış menüyü açar.
- `cargo test -p kentos-desktop` (izler dahil), `pnpm rust:test:desktop` (clippy temiz).
- Görüntüler (`drawing_menus::screens`, `.run/shots/sag-tik-*`), koyu ve açık, 1440×900 ve 1100×650: boşta menüsü, Çizgi'de komut menüsü, kenet menüsü, komut satırında "Sonraki tık: Kesişim" çipi.
