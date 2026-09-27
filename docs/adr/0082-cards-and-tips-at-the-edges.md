# ADR 0082: Bilgi kartı, değer alanı ve ipuçları kenarlarda

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** DESIGN.md §7.4.2, §7.8; ADR 0018 (değer alanı), 0054 (ipuçları), 0068 (üzerine gelme kartı)
- **Kaynak:** sahibin bildirimi: "objelerin üstüne gelince çıkan hint metinleri çok uzun olunca sağa taşma yapıyor … mpyy sembolojilerinin üstüne gelince açıklamaları uzun olduğundan farkediliyor". Web ajanının `5c1cf5a`'sı (`ui/widgets/placeBeside.ts`, `HoverCard.ts`, `CursorInput.ts`, `tooltip.ts`, `shell.css`, `controls.css`) ve aynı dalın `21be91b`'si (ADR 0081'in iki notu). Kuralı web ajanı koydu; masaüstü birebir aldı.

## Bağlam

- **Web:** MPYY vitrininde bir katmanın adı yaklaşık yüz karakter. Kart adı tek satırda tutuyordu (`white-space: nowrap`): ad kartın kenarını geçiyor, çizimin kenarında kesiliyordu. Kart hep imlecin sağ altında durduğu için çizimin sağ ya da alt kenarında kendisi de dışarı taşıyordu.
- **Masaüstü (`hover_card.rs`, ADR 0068):**
  - Kart ve imleç yanındaki değer alanı imlecin yanında sabit uzaklıktaydı; kenarda dışarı taşıyordu.
  - Kartın adları ve değerleri iki ayrı sütundaydı. Uzun bir değer iki satıra kayınca alttaki adlar değerlerinden kopuyordu.
- **İpuçları:** iki platformda da uzun başlık kaymıyor, sağ dokun satırının ipucu pencere kenarına sıkışıyordu.

## Karar

### Kural (iki platformda aynı; DESIGN.md §7.4.2, §7.8)

- **Kartın yeri** (web `besidePointer`, masaüstü `kentos_ui::widget::beside_pointer`):
  1. İmlecin 18 px sağında ve 20 px altında durur.
  2. Sağ kenarı geçecekse aynı uzaklıkta imlecin soluna geçer.
  3. Alt kenarı geçecekse imlecin 20 px üstüne çıkar.
  4. Sonra çizimin her kenarından 8 px içeride tutulur. Çizimden büyük kartın sol üst köşesi içeride kalır.
  - Önce yan değişir, sonra içeride tutulur. Alan, çizim alanıdır (kameranın boyu).
- **Kartın genişliği:** en az 160, en çok 280 px × yazı ölçeği; çizimden 16 px dar.
  - Masaüstünde yazı ölçeği, seçilen boyutun varsayılana (13) oranıdır: `typography::from_default`.
- **Kaydırma:** uzun ad ve metin bu genişlikte kayar, gerekirse kelime içinden (web `overflow-wrap: anywhere`, masaüstü `Wrapping::WordOrGlyph`). En çok yükseklik ve kısaltma yoktur; adın tamamı görünür.
- **Başlık:** tür solda, katman sağda; ikisi bir satıra sığmazsa katman türün altına iner ve orada kayar. Katman örneği ilk satırdadır.
- **Satırlar:** `auto 1fr`. Ad tek satırdır; değer kayar, sağa yaslıdır ve kartın sağ kenarında biter.
- **İmleç yanındaki değer alanı:**
  - Aynı kural, (18, −58) uzaklıkla. İmlecin sağ üstünde kalır, aracın ölçü etiketi sağ altta.
  - Sağ kenarda imlecin soluna geçer. Dikeyde yan değiştirmez; üst kenarda aşağı kayar.
- **İpuçları:**
  - Aynı genişlik ve kaydırma; ayrıntı satırı (kısayol) bölünmez.
  - İstenen yanda yer yoksa karşı yana geçer: sağ dokun satırı soluna, alttaki üstüne, üstteki altına.
  - Sonra pencerenin kenarından 8 px içeride tutulur (önceden 5 px).

### Masaüstünde nasıl

KentOS UI'da üç yeni parça var:

- **`widget::beside`:**
  - saf `beside_pointer` ve `EDGE_MARGIN`;
  - bulunduğu yeri alan sayan `beside(content, at, offset)`. İçeriği önce kendi boyunda yerleştirir, sonra kurala göre koyar.
  - Iced'in `pin`'i çocuğunu konumu kadar daraltır; kenara yakın kart sıkışıp satır satır kayardı. Bu yüzden `pin` kullanılmadı.
- **`widget::Pairs`:** ad–değer satırları, `auto 1fr`. Verilen genişliğe yayılınca değer sütunu sağ kenara uzanır.
- **`widget::InfoCard`:** başlık, etiket, çizgi ve satırlar. Genişlik, başlık tek satırdayken ya da satırların genişliğidir, sınırlar içinde. Etiket sığmazsa başlığın altına iner.

Masaüstündeki yerleri ve öbür değişiklikler:

- **Kullanıldığı yerler:** `hover_card.rs` kartı bunlarla kurar. `preview.rs` değer alanını `beside` ile koyar. `tip.rs`'in yerleşimi saf `place`'tir.
- **Yardımcı:** `typography::from_default(px)`, varsayılan yazı boyutunda verilmiş ölçünün o anki karşılığıdır (web'in `px × var(--ui-scale)`'i).

### Kart yalnız ayarı açıksa (sahibin isteği)

- Sahibin isteği: "Üzerine gelince ipucu çıkması ayarlarda açıksa olmalı, ayarlarda varsayılan açık olmalı".
- Kart `drafting.hoverInfo` ("Nesne bilgi kartı") açıkken görünür; varsayılan açıktır. Web bunu uygulama ayarlarında zaten gösteriyordu.
- Masaüstü ayarı okuyordu ama şemada yalnız web ev sahibiydi; ayarlar penceresinde yoktu, kapatılamıyordu.
- Artık masaüstü de ev sahibidir: Uygulama ayarları'nda Çizim yardımcıları'nda, Komut şeridi'nin altındadır.
- Düğmelerin ipuçlarının ayarı yoktur; her zaman görünürler (DESIGN.md §7.8).

### Web ajanının aynı daldaki öbür işi (`21be91b`, ADR 0081'in notları)

- Bekleme listesi hep doğrudur: katmanı çizimde olan kayıt listeden çıkar. Sunucunun artık tutmadığı kayıt da çıkar.
- "Sunucudakini al"da, katmanı çizimde olmayan kopya bekler. Önceden okunamadı diye reddediliyordu ve yeniden "benimki" diye gidiyordu.
- Masaüstü ikisini ADR 0081'de zaten böyle yapıyor.

## Doğrulama

- **KentOS UI:**
  - `beside` web'in yedi durumunu aynı sayılarla sınıyor.
  - `Pairs`: sütun, satır yüksekliği, verilen genişliğe yayılma.
  - `InfoCard`: tek satır, alta inen etiket, en az genişlik.
  - `tip`: karşı yana geçme ve içeride tutma.
- **Masaüstü görüntüleri** (`hover_card::screens`, `.run/shots/`):
  - dosyalar: `uzerine-gelme-uzun`, `-kose`, ikisinin `-buyuk`'u (yazı 16) ve `deger-alani`, `deger-alani-kose`;
  - 1440×900 ve 1100×650, koyu ve açık;
  - her resim bütün olarak incelendi.
- **Ayar:** kart testi ayarın varsayılanda açık olduğunu, ayar kapatılınca kartın çıkmadığını sınıyor. Ayarlar penceresinin görüntüsü (`pencere-ayarlar-*`) 1440×900'de ve 1100×650'de incelendi.
- **Web:** `pnpm e2e:layout` dört yeni görünümle geçiyor (web ajanı); her görünüm, kart ya da ipucu alanından çıkarsa düşer.
- **Geçenler:** `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm inventory:check`, `pnpm e2e:layout` (270 görünüm), `KENTOS_E2E_DB=scratch pnpm e2e:cloud`.
