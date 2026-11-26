# ADR 9251: Aracın önizlemesi görünüm hareket ederken de her karede çizilir

- **Durum:** kabul edildi (2026-11-26).
- **Plan:** [plan/render.md](../../plan/render.md) RM-05'in ve RM-14'ün kalanı (F1, F4).
- **İstek:** sahibin sözleri (26 Kasım 2026):
  - “bu çok ciddi bir sorun, bunu düzeltelim”;
  - “zoom bitince geri geliyor onda sorun yok ama zoom esnasında gidiyor, zoom esnasında görmeden ben çalışamam”.
- **Yerine geçtiği:** [ADR 9205](9205-live-camera-in-the-canvas.md) §3'ün “ekran boyutlu işaretler görünüm durana kadar
  gizlenir” kuralı. Kural işaretler, tutamaçlar ve aracın önizlemesi için kalktı; kartlar ve yerinde düzenleyiciler
  için duruyor.
- **Sözleşmeler:** çizim alanının iletisi `Event::ViewChanged`'e `at` eklendi: orta tuşla kaydırmanın imleci bıraktığı
  yer. Komut, ayar, Python, MCP ve `.kcad` değişmedi.

## §1 Sorun

Masaüstünde CAD'de Çizgi, Çoklu çizgi ya da Duvar ilk noktayı aldıktan sonra görünüm oynatılınca (tekerlek, dokunmatik
yüzeyde iki parmak, orta tuşla kaydırma) ekrandan şunlar kalkıyordu:

- lastik bant;
- çoklu çizginin ve duvarın o ana kadar verilen kenarları (komut onaylanana kadar önizlemedirler);
- ölçü etiketi, kenet işareti, tutamaçlar.

Görünüm 150 ms durunca hepsi geri geliyordu. Çizgi aracının her tıkta yazdığı parçalar sahnede olduğu için kalıyordu.

**Kök neden: bilinçli bir gizleme.** RM-05 (ADR 9205) jest boyunca kamerayı çizim alanının bileşenine taşıdı. O
karelerde uygulamanın görünümü kurulmuyor; uygulamanın kamerasıyla kurulmuş üst katmanlar `Follow` ile canlı kameraya
uyduruluyor. Önizleme katmanı (`preview::layer`: işaretler, taslak, ölçü etiketi, değer alanı) `Follows::Still` ile
sarılıydı.

- Gerekçesi şuydu: ekranda bir boyda çizilen işaretler resim gibi ölçeklenirse hızlı bir tekerlekte yedi kat büyür.
- Ölçeklemek yerine görünüm durana kadar gizlemek seçilmişti. Hareket boyunca çizen kişi ne çizdiğini göremiyordu.

**Aynı akışta iki kusur daha** (gerileme sınamasında çıktı):

1. **Orta tuşla kaydırma.** Sürükleme sürerken imlecin hareketi uygulamaya ileti olarak gitmiyor, kaydırma oluyor.
   Görünüm durunca uygulama imlecin dünya noktasını sürüklemenin başladığı ekran yerinden okuyordu. Komut görünüm
   değişince imleci yeniden görür ([ADR 0283](0283-transparent-commands-keep-the-tool.md)); bandın ucu imleçten sürükleme kadar
   öteye atlıyordu. Fare kıpırdayınca düzeliyordu.
2. **Yakınlaştır ve Uzaklaştır** (şerit, kısayol) ortadan yakınlaştırır. İmlecin dünya noktasını ortanınki yapıyordu.
   Komut sürerken bant ekranın ortasına sıçrıyordu; şeritteki düğmeye basılınca da, imleç çizim alanında değilken.

## §2 Araştırma

- **CAD programları:** kamera değişince sahne yeniden çizilir, yeniden üretilmez (ADR 9205 §2).
  - Komut nokta beklerken tekerlekle yakınlaştırma ve kaydırma komutu bozmayan görünüm işlemleridir. Kaynaklar:
    Autodesk forumunda geliştirici tartışması (“Transparent commands during GetPoint”, tekerlek düzenleyici girdi
    beklerken her zaman çalışır) ve AutoCAD LT yardımının saydam `'PAN` sayfası.
  - Bulunan resmî belgeler lastik bandın jest sırasındaki görünüşünü ayrıca yazmıyor. Burada değer uydurulmadı; ölçüt
    sahibin sözü: hareket boyunca görünmeli.
- **iced 0.14, kaynak koddan:**
  - Her karede bütün arayüz yeniden çizilir (`UserInterface::draw`), uygulamanın `view`'ı kurulmasa da.
  - Önbelleksiz bir kanvas programının `draw`'ı her karede yeni geometri üretir. Taslak ve işaretler önbelleksiz.
- **iced_winit 0.14.1:** kıstırma (pinch) olayını çevirmiyor. Dokunmatik yüzeyde iki parmakla kaydırma
  `WheelScrolled { Pixels }` olarak geliyor ve tekerlekle aynı yoldan geçiyor. Kıstırmayla yakınlaştırma bugün hiç yok;
  ayrı iş.

## §3 Karar

1. **Önizleme katmanı canlı kamerayı çizerken okur.**
   - `Marks` ve taslak (`Draft`) çizim alanının `LiveView`'ını taşır. `draw` her karede `live.camera()` ile dünya
     noktalarını ekrana çevirir.
   - Bu, sahnenin aynı karede çizildiği kameradır: shader programının `draw`'ı da onu okur. Jest yokken uygulamanın
     kamerasına eşittir (`live_for_view`).
   - Çizgiler bir piksel, işaretler kendi boylarında kalır. Ölçekleme yok, gizleme yok.
2. **Önizleme katmanı `Follow` ile sarılmaz** (`Follows::Still` kalktı).
3. **Ölçü etiketi, kilit etiketi ve değer alanı bağlı oldukları dünya noktasına iğnelidir** (`Follows::Pinned`).
   - Noktayla birlikte tam piksel kayarlar, ölçeklenmezler (`follow::pinned`).
   - Tekerlekte imlecin altındaki nokta kımıldamaz, etiket yerinde kalır. Kaydırmada imleçle gider.
4. **Kartlar ve yerinde düzenleyiciler** (gezinme ve Bilgi kartı, seçim fişi, yazı alanı, paragraf düzenleyicisi)
   ADR 9205'teki gibi görünüm durana kadar gizlidir. Bu işin dışında.
5. **Kaydırmanın imleci.**
   - `LiveView::pan` sürüklemenin imleci bıraktığı yeri tutar; `ViewChanged { at }` onu uygulamaya taşır.
   - Görünüm durunca imleç oradadır; alanın dışındaysa imleç yok.
   - Dünya noktası sürüklemenin imlecin altında tuttuğu noktadır: bant imleçte biter.
6. **Ortadan yakınlaştırma** (`App::zoom`): imleç ekranda yerinde kalır, dünya noktası altına gelen yeni noktadır
   (`Viewport::repoint_cursor`). İmleç alanın dışındaysa yok; bant son noktasında kalır.

**Bedel:** jest karelerinde işaretler ve taslak da çiziliyor. Komut ve seçim yokken katman boş (`Marks::is_empty`):
ölçüm dosyalarının jest karelerinde iş değişmedi. Tutamaçlı bir seçim (en çok `GRIP_LIMIT` nesne) varken jest karesine
onların çizimi eklenir; durmuş karede zaten çiziliyorlardı.

## §4 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | Önizleme katmanı ve canlı kamera kabukta; çizim alanı her modülün. Duvar sınandı. |
| Harita | var | Aynı katman; Çizgi ve Çoklu çizgi sınandı. |
| Yol (CAD) | var | Aynı katman; Yol'un araçlarının önizlemesi de bu katmanda. |
| CBS | var | Aynı katman: çizim araçları ve veri katmanı düzenleme araçları. |
| MCP, Python, eklenti, yapay zekâ ajanı | yok | Görünümün hareketi ve önizlemenin ekrana çizimi arayüzün işi. Kanallar komut çağırır; komutların doğrulaması, önizlemesi ve sonucu değişmedi. |

## §5 Kanıt

**Gerileme sınaması** (`render_preview_tests`, GPU, `KENTOS_GPU_TESTS=1`). Dokuz durum:

- Çizgi, Çoklu çizgi ve Duvar (Dış duvar 25) gerçek pencere olaylarıyla iki nokta alır, imleç bekler.
- Görünüm oynatılır: tekerlek içeri ve dışarı, dokunmatik yüzey, orta tuşla kaydırma, Yakınlaştır.
- Jest sürerken çizilen kare iki kez denetlenir:
  - bant ve verilen kenar o karenin kamerasının koyduğu yerde mi;
  - kare, görünüm durup o kamera için kurulunca çizilen kareyle aynı mı (kanal başına 40, bir piksel komşuluk).

Düzeltmeden önce kırmızı, sonra yeşil:

| Durum | Önce, hareket karesinde | Önce, eşi olmayan piksel | Sonra |
|---|---|---|---|
| Çizgi, tekerlek içeri | bant yok | 1 556 | 0 |
| Çizgi, tekerlek dışarı | bant yok | 1 309 | 0 |
| Çizgi, dokunmatik yüzey | bant yok | 1 528 | 0 |
| Çizgi, orta tuşla kaydırma | bant yok; durunca ucu imleçten sürükleme kadar kaçık | 1 453 | 0 |
| Çizgi, Yakınlaştır | bant ekranın ortasına gidiyor (kare durulanla aynı) | 0 | 0, bant imleçte |
| Çoklu çizgi, tekerlek | bant ve verilen kenar yok | 1 811 | 0 |
| Çoklu çizgi, kaydırma | bant ve verilen kenar yok | 1 714 | 0 |
| Duvar, tekerlek | bant ve duvarın gövdesi yok | 11 220 | 0 |
| Duvar, kaydırma | bant ve gövde yok; durunca ucu kaçık | 8 516 | 0 |

“Sonra” sütununda her durumda bant ve verilen kenar hareket karesinde, o karenin kamerasının koyduğu yerde. Tekerlekle
dışarı ve kaydırma durumlarında aynı olmayan ama eşi bulunan pikseller (216 439 ve ~5 000) ızgaranın çizgileri: ızgara
görünüm durunca yeni aralıkla kuruluyor, soluk rengi eşiğin altında.

Resimler `.run/gui-evidence/render/onizleme/`'de (Git'e girmez): her durumun önce, hareket ve durunca kareleri, farkı,
önizlemenin kutusu üç kat büyütülmüş.

**Birim sınamaları:**

- `follow::a_pinned_card_goes_with_its_world_point_by_whole_pixels`;
- `viewport::a_settled_pan_puts_the_pointer_where_the_drag_left_it`;
- `viewport::a_zoom_about_the_middle_leaves_the_pointer_where_it_is_on_the_screen`;
- `live::a_gesture_moves_the_live_camera_and_hands_the_app_where_it_started` (sürüklemenin bıraktığı yer).

**Öbür sınamalar:**

- Masaüstünde görünüm, önizleme, işaretler, kilitler, çizim yardımcıları ve gezinme süzgeçlerinin 173 sınaması geçti.
- Render uygunluğu (ikiz uygulama, ADR 9224) geçti.
- Altın görüntüler (ADR 9225) bu işten önce de kırmızıydı. Farkları yalnız yazıların harflerinde: aynı gün main'e
  giren yazı yüzü işleri (ADR 9311, 9313) altın görüntüleri yeniden yazmamış. Bu resimlerde araç ve imleç yok; bu iş
  onlara dokunmuyor.

**Gerçek pencere:** bu oturumda yapılmadı; sahip Mac'in başındaydı ve gerçek fareyle girdi onun işine karışırdı.
Ekran dışı resimler aynı bileşenlerle, gerçek olaylarla çekildi. Gerçek pencerede deneme sahibe kaldı.

## Uygulama

- `apps/desktop/src/shell/preview.rs`: `layer(camera, live, …)`; taslak canlı kamerayla çizer; etiket ve alan iğneli.
- `apps/desktop/src/cad/marks.rs`: `Marks::live`, `Marks::shown`.
- `apps/desktop/src/shell/viewport/follow.rs`: `Follows::Pinned`, `pinned`.
- `apps/desktop/src/shell/viewport/live.rs`: `LiveView::pan(by, at, now)`; `take` sürüklemenin yerini verir.
- `apps/desktop/src/shell/viewport/mod.rs`, `state.rs`: `ViewChanged { at }`, `Viewport::repoint_cursor`.
- `apps/desktop/src/shell/input.rs`: `App::zoom` imleci yerinde tutar.
- `apps/desktop/src/shell/view/area.rs`: önizleme katmanı `Still` ile sarılmaz.
- `apps/desktop/src/tests/render_preview_tests.rs`: gerileme sınaması.
