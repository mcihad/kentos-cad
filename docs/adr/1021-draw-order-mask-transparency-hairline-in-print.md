# ADR 1021: Çizim sırası, maske, saydamlık ve saç teli baskıda ve iki görünüm penceresinde aynı

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11` maddesi “Çizim sırası, maskeler,
  saydamlık ve saç teli çizgilerinde model, viewport ve PDF eşdeğerliği aransın. Ekran temasını değiştirmek çizimin
  atanmış veya basılacak rengini değiştirmesin.” Yeni davranış yok; var olan kuralın kanıtı.
- **Bağlam belgesi:** ADR 0310 (alan maskesi, WIPEOUT), ADR 0264, ADR 1090 (`pdf_hairline`), `kentos_plot::maps`
  (kâğıt paleti).

## 1. Kaynak ve karar

AutoCAD'de WIPEOUT çizim sırasında kendinden önce çizileni örter, sonra çizileni örtmez; çizgi kalınlığı 0 cihazın en
ince çizgisidir; saydamlık baskıda korunur (PLOTTRANSPARENCYOVERRIDE). KentOS'ta bunlar zaten çizicinin kuralıydı;
bu ADR, kâğıdın renginin ekran temasından gelmediğini ve iki ölçekte aynı çıktığını kanıtla bağlar. Tema jetonu `fg`
kâğıtta sabit kâğıt mürekkebiyle (`#111111`, web'in `paperPalette`'i) basılır; ekranın teması hiçbir yoldan kâğıda
ulaşmaz.

## 2. Kanıt

- `kentos-plot` `line_look_order::order_mask_transparency_and_hairline_print_the_same_through_two_viewports`: kırmızı
  tireli A (0,5 mm), ortasında çerçevesiz maske, maskeden sonra çizilen 0 kalınlıklı B (`fg`), saydamlığı 0,5 mavi C.
  1:50 ve 1:20 görünüm pencerelerinde vektör PDF'in sırası A < maske < B < C; maske kâğıt beyazı ve kenarsız; B
  0 genişlikli ve `#111111`; C'nin mürekkebi yarı opak (`#0000ff80`); A'nın kâğıttaki tiresi iki ölçekte aynı; çizim
  değişmez.
- `line_look_order::the_cpu_picture_keeps_the_order_and_the_mask`: CPU baskı resminde maskenin içinde A yok (kâğıt),
  maskeden sonra çizilen B var.
- Masaüstü `sheet_book::tests::the_screen_theme_changes_neither_the_drawing_nor_the_printed_pdf`: aynı pafta takımı
  dört temada (koyu, gece, yüksek karşıtlık, açık) bayt bayt aynı PDF; çizim değişmez.
- Resim `.run/gui-evidence/cad-adv-11-sira-maske-genis-1.png` (PDF, 200 dpi): bakıldı.

## 3. Sınırlar

Ekranın (GPU) kendi çiziminde aynı sıra ve maske piksel ölçüsüyle bu testte karşılaştırılmadı (ekran ADR 0310'un
resimleriyle doğrulanmıştı); baskı profilinin saydamlığı ezmesi ayrı bir yoldur.
