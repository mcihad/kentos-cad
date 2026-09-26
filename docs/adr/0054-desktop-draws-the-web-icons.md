# ADR 0054: Masaüstü web'in ikonlarını çizer; ipucu §7.8'e uyar

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** DESIGN.md §7.3.1, §7.8; ADR 0016 (KentOS UI), 0017 (masaüstü kabuğu), 0051 (şeridin sığması)
- **Sahibin istekleri (26 Eylül):** "Görsellik en önemli şeylerden biri"; şeridin "daha rafine ve kalite algısı yüksek" görünmesi.

## Bağlam

- Web'in 186 ikonu vardır (`apps/web/src/ui/icons.ts`). Hepsi 20×20'lik, elle çizilmiş çizgi ikonlarıdır; dolu kareler CAD tutamaçlarıdır, aracın tıklamalarının nereye gittiğini gösterir.
- KentOS UI'nin kodla çizilmiş 93 ikonu vardır. Masaüstü, web'in ikon adlarını bunlardan anlamca en yakınına eşliyordu (`apps/desktop/src/icons.rs`):
  - Ölçekle, Esnet, Döndür, Aynala, Patlat ve ölçü araçları aynı cetveli gösteriyordu;
  - Dizi, Kutupsal dizi, Hizala, döndürülmüş dikdörtgen ve düzgün çokgen aynı dikdörtgeni;
  - Buda, Uzat, Kır ve öbür çizgi araçları aynı çizgiyi.
  Şerit bu yüzden tekdüze görünüyor, web'den ayrılıyordu.
- İpucu iced'in `tooltip`'iydi. Hemen açılıyordu ve tıklamada kapanmıyordu. Bölünmüş düğmenin menüsü açılınca ipucu menünün üstünde kalıyordu (Dizi ▾).
- DESIGN.md §7.8 başka ister: ipucu 450 ms gecikmeyle açılır; bir ipucu kapandıktan sonraki 600 ms içinde komşu öğelerinki hemen açılır; tıklamada kapanır.

## Karar

### Web'in ikonları envanterden gelir

- Web `ICONS`'u dışa açar. Envanter toplayıcısı (`apps/web/scripts/inventory/collect.mjs`) seti `web.json`'un `icons` bölümüne yazar: ad → SVG metni (tutamaçlar içinde). `pnpm inventory:check` onu da denetler.
- Masaüstü kataloğu seti envanterden okur. Komutun, aracın ve panelin ikonu web'in çizimidir (`Icon::Svg`). Envanterde olmayan ad eski eşlemeyle KentOS UI ikonuna düşer. Uygulamanın kendi çerçevesi (paneller, durum çubuğu, sekmeler) KentOS UI ikonlarında kalır.
- KentOS UI, setin kullandığı SVG alt kümesini okur ve çizer (`crates/ui/src/icon/svg.rs`):
  - öğeler: `path` (M, L, H, V, C, S, Q, T, A, Z; göreli ve mutlak), köşesi yuvarlatılabilen `rect`, `circle`, `rotate()` ile döndürülen `ellipse`;
  - boyama: `fill="currentColor"` ve `fill-opacity`, `stroke="none"`, `stroke-width`, `stroke-dasharray`, `stroke-linecap`, `fill-rule`;
  - SVG yayı, SVG 1.1 F.6.5'in merkez biçimiyle Bézier'lere çevrilir; küçük yarıçap büyütülür (F.6.6);
  - çizgi web'inki gibidir: yazı rengi, 1,4 birim, yuvarlak uç ve birleşim. Şeridin 28 piksellik ikonları sabit 1,55 px çizgiyle çizilir (web: `vector-effect: non-scaling-stroke`);
  - çözülemeyen öğe atlanır; hiçbir girdi panik yaptırmaz.

### İpucu (KentOS UI `tip`)

- İpucu KentOS UI'nin kendi bileşenidir; iced'in `tooltip`'inin yerini aldı. Bütün `tip` çağrıları onu kullanır.
- DESIGN.md §7.8'in kuralları:
  - imleç öğenin üzerinde 450 ms durunca açılır;
  - bir ipucu kapandıktan sonraki 600 ms içinde komşu öğeninki hemen açılır;
  - tıklamada kapanır ve imleç öğeden çıkana dek yeniden açılmaz. Tıklama sıcak süreyi başlatmaz.
- Öğenin kendi açılır menüsü açıkken ipucu çizilmez: menünün üstüne binmez.

## Bu dilimde olmayanlar

- Klavyeyle odaklanan öğenin ipucu.
- Hazır olmayan komutun ipucundaki amber not (§7.8); masaüstü bunu ipucu gövdesinde düz metinle söylüyor.
- KentOS UI'nin kendi ikonlarının web setine göre yeniden çizilmesi.

## Doğrulama (26 Eylül 2026, Linux)

- `crates/ui/src/icon/svg.rs` testleri:
  - sayılar SVG'nin yazdığı gibi okunur (`1.2.5`, `-1-2`, bitişik yay bayrakları);
  - göreli ve örtük komutlar;
  - yayın bitiş noktası ve yönü; küçük yarıçap;
  - öğeler ve boyama;
  - bozuk metinler panik vermez.
- `tip` testleri: bekleme, sıcak süre, tıklamada kapanma ve imleç çıkınca yeniden bekleme.
- Masaüstü `icons::tests`: envanterdeki 186 ikonun her öğesi çizilir; envanterin adını verdiği her komut ve araç ikonu sette vardır.
- Görüntüler (`.run/shots`):
  - `serit-*`: şerit, web ikonlarıyla, koyu ve açık, 1440 ve 1100 px;
  - `serit-ipucu-*`: üzerine gelince hemen ipucu yok, 450 ms sonra var; Dizi ▾ menüsü ipucusuz açılır (`ribbon_tests::tip_screens`).
- `pnpm rust:test:desktop` (450), `pnpm typecheck`, `pnpm test` (1500), `pnpm inventory:check` geçti.
