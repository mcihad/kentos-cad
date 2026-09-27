# ADR 0080: Masaüstünde ızgara (F7) ve durum çubuğunun çizim yardımcıları

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.4, §4.9; DESIGN.md §7.7; TODOS.md `UI-11`; ADR 0019 (wgpu çizim hattı), 0023 (tipli ayarlar), 0058 (durum çubuğu)
- **Kaynak:** web kodu, koddan okundu:
  - `render/grid.ts` (`gridSpacing`, `gridExtent`, `buildGrid`);
  - `ui/statusbar/StatusBar.ts` (`status__toggles`, dar pencerede `STEPS`);
  - `styles/shell.css` (`.status__toggle`, `data-fit-*`);
  - `styles/tokens.css` (`--canvas-grid-minor`, `--canvas-grid-major`).

## Bağlam

- Web'in çizim alanında ızgara var (Izgara, F7).
- Web'in durum çubuğunda altı çizim yardımcısı var: Kenet, Izgara, Orto, Kutupsal, İzleme, Kalınlık. Her biri bir lamba, açıkken yanar.
- Masaüstünde ızgara ve bu anahtarlar yoktu.
- Masaüstünün durum çubuğu dar pencerede, bulut projesi açıkken taşıyordu: 1100 px'te hücreleri kesiliyordu.

## Karar

### Izgara (`draft.grid`, F7)

- **Ayar:** oturum ayarıdır, varsayılanı açık (`drafting.grid`). Tipli ayarların şemasında artık masaüstü de ev sahibidir.
- **Aralık** (web'in `gridSpacing`'i):
  - İnce çizgiler ekranda en az 14 px arayla durur; aralık bu şartı sağlayan en küçük 1-2-5 adımıdır.
  - Kalın çizgi beş incede birdir; 5'lik adımda dörtte bir. Böylece kalın çizgiler yuvarlak değerlere düşer.
- **Konum:** çizgiler dünya koordinatlarının yuvarlak değerlerindedir, TM değerlerine oturur.
- **Renk:** web'in jetonları: koyu zeminlerde beyazın %4'ü ve %9'u, kâğıt zeminde siyahınkiler.
- **Çizim hattı:** ızgara wgpu sahnesinin en alttaki parçasıdır; çizimin altında çizilir.
  - Görünümün üç katı genişlikte bir kutu için kurulur.
  - Görünüm kutudan çıkınca, aralık, köken ya da zemin değişince yeniden kurulur. Kutunun içinde kaydırmak ve yakınlaştırmak GPU'ya yeni bir şey yüklemez (web'in `gridExtent`'i).
- Çizimi etkilemez: kenet ve seçim ızgarayı görmez (web'de de).

### Durum çubuğunun çizim yardımcıları

- **Yeri:** koordinatlardan ve seçim sayısından sonra altı anahtar gelir: Kenet (F3), Izgara (F7), Orto (F8), Kutupsal (F10), İzleme (Shift+F3), Kalınlık.
- **Lamba** (web'in `.status__toggle`'ı): adın önünde 6 px kare; kapalıyken çerçeveli, açıkken vurgu renginde. İpucu durumu, kısayolu ve açıklamayı söyler.
- **Taşınmamışlar:** masaüstünde çalışmayan İzleme ve Kalınlık kapalı ve sönüktür; ipuçları “Web'de var; masaüstüne henüz taşınmadı” der.
- **KentOS UI:** `status_bar::Toggle` ikonsuz verilince lambayı çizer. `description` ve `compact` eklendi.

### Dar pencerede durum çubuğu (web'in `STEPS`'i)

Hücreler sırayla çekilir, en az gerekenden başlayarak. Her hücre ipucunu korur.

1. Çizim motorunun adı: ikonu kalır.
2. Koordinat sistemi: sekme satırında da var.
3. Ekran ve pafta ölçeği.
4. Bulut hücrelerinin sözleri: noktaları ve ikonları kalır, sözleri ipuçlarındadır.
5. Çalışma modunun adı: ikonu kalır.
6. Çizim yardımcılarının iç boşluğu.

- **Genişlik tahmini:** metinlerden, arayüzün yazı boyunda yapılır (sekme satırı gibi): harf başına boyun 0,52'si, koordinat rakamları için 0,6. Tahmin görüntülerde ölçülerek ayarlandı.
- **Sonuç:** 1440 × 900'de bulut yokken bütün hücreler durur. 1100 × 650'de bulut projesiyle bile hiçbir hücre kesilmez.

## Web'den ayrılanlar

- **İleti hücresi:** web'in durum çubuğunda son iletinin gösterildiği hücre (`status__flash`) masaüstünde yok. İletiler komut satırının geçmişindedir.
- **Pafta ölçeği ve nesne sayısı:** masaüstünde bu hücreler de var. Pafta ölçeği ekran ölçeğiyle birlikte çekilir.

## Doğrulama

- `kentos-render-wgpu`, `tests/scene.rs`: ızgaranın aralığı (web'in değerleri) ve çizgileri; ince ve kalın katmanlar, yuvarlak değerler.
- Masaüstü, `selecting::tests::f7_turns_the_grid_on_and_off`: F7, komutun işareti, ayar.
- Görüntüler, koyu ve açık, 1440×900 ve 1100×650; hepsinde ızgara ve durum çubuğu görünür:
  - `katman-ara-*`, `katman-klavye-*`: bulut yok;
  - `bulut-agac-kilitli-*`: bulut projesi açık.
- Geçenler: `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm inventory:check`.
