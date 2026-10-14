# ADR 0283: Şeffaf komutlardan dönünce araç korunur

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-19`: “Şeffaf pan/zoom/ölçme ve nokta
  hesaplayıcıdan dönünce aktif aracın parametreleri ve preview'ı korunsun.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0018 (araç oturumunun durumları), 0056 (Kaydır ve Pencere yakınlaştır), 0067 (Mesafe ölç), 0083 (nokta
  hesaplayıcı, `Session::nest`), 0140 (Açı ölç, Koordinat oku), 0141 (görünüm geçmişi), 0166 (kilitler, referans, Yapım kipi).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Tekerlek, orta tuşla kaydırma ve Tümünü göster çalışan aracı bitirmiyordu ama aracın önizlemesi imleç kımıldayana kadar eski görünümdeki
yerinde kalıyordu. Kaydır, Pencere yakınlaştır, Mesafe ölç, Açı ölç ve Koordinat oku şeritten, kısayoldan ya da komut satırından
başlatılınca `Session::start` çalışan komutu düşürüyordu: iki köşesi konmuş Çoklu çizgi, taban noktası verilmiş Taşı, Referans'ı
başlamış Döndür kayboluyordu. Nokta hesaplayıcı komutun üstünde çalışıyordu (ADR 0083), ama döndüğünde komutun önizlemesi de eski
imleçteydi. AutoCAD'de `'pan`, `'zoom`, `'dist` ve `'cal` komutu bitirmeden çalışır, bitince komut kaldığı yerden sürer.

## Karar

1. **Şeffaf araçlar:** Kaydır (`pan`), Pencere yakınlaştır (`zoomWindow`), Mesafe ölç (`measure`), Açı ölç (`measureAngle`) ve Koordinat
   oku (`crsQuery`); `session::TRANSPARENT`. Ortak yanları görünümü değiştirmeleri ya da çizimi yalnız okumalarıdır. Çizimi değiştiren
   araç hiçbir zaman şeffaf değildir: Alan hesapla bile (Alan olarak çiz yazar) değildir; böyle bir araç başlayınca çalışan komut eskisi
   gibi biter.
2. **Komutun üstünde:** komut çalışırken (ya da tutamaç sıcakken) şeffaf araç başlatılınca `Session::start_over` onu komutun üstüne
   koyar: komut askıda bekler, noktaları, seçenekleri (Kopya, Stil …), belleği ve istemi değişmez. Komutun oturumdaki durumu da onunla
   bekler: kilitleri, kilit için beklenen kenar, Referans noktası ve Yapım kipi (`Kept`); şeffaf araç kilitsiz başlar. Şeffaf araç
   bitince (Esc, nokta beklemeyen şeffaf araçta Enter, işi bitmiş araçta Enter, Pencere yakınlaştır'ın kutusu) komut ve durumu geri
   gelir; şeffaf araç komuta nokta vermez. Kendi başına çalışan Kaydır'ın üstüne başka bir şeffaf araç konmaz, onun yerine başlar;
   komutun üstündeki şeffaf aracın yerine yenisi geçer (iç içe tek şeffaf araç). Şeffaf araç son komut sayılmaz, araç çalışması da
   sayılmaz: nesne şablonunun çalışması sürer (ADR 0176 §3).
3. **Askıdakiler yığını:** `Session`'ın askıdakileri yığındır (`parents`); her biri hesaplayıcının mı şeffaf aracın mı altında
   olduğunu bilir. Nokta hesaplayıcı şeffaf Mesafe ölç'ün üstünde de çalışır, noktası ölçmeye gider; hesaplayıcının üstünde şeffaf araç
   da çalışır; hesaplayıcı kendi üstünde çalışmaz. `tool_id` en alttaki komuttur (izler `tool`'u öyle okur), `top_id` imleci alan araç.
4. **Önizleme:** şeffaf araç çalışırken komutun taslağı altta olduğu gibi çizilir (çizgileri, alanları, hayaletleri; etiketi ve
   işaretleri değil). Komut geri gelince ya da görünüm onun altında değişince (tekerlek, Tümünü göster, Önceki görünüm, Pencere
   yakınlaştır) masaüstü imleci olduğu yerde yeniden verir (`Session::take_resumed`, `App::follow_view`, `repoint`): önizleme yeni
   görünümde imlecin altındadır. Görünüm değişirken imleç çizim alanındaki yerinde kalır (Tümünü göster de). Pencere yakınlaştır'ı
   bitiren basışın bırakılışı komuta gitmez.
5. **Kısayol penceresi:** Çizerken grubunda “Komutu bitirmeden kaydır, yakınlaştır, ölç; Esc ya da Enter komuta döner” satırı.

## Sonuçlar

- Yeni komut ya da `.kcad` alanı yok; şeffaf araçlar çizimi değiştirmez, Python ve MCP'nin karşılığı gerekmez (görünüm ve ölçme
  başsız sunucuda `measure` sorgusudur).
- İzlerde şeffaf araç çalışırken `points` ve `prompt` şeffaf aracındır, `tool` askıdaki komutun.

## Doğrulama

- `cargo test -p kentos-interaction --test all transparent` (her şeffaf araçtan sonra komutun istemi, noktaları, seçenekleri ve
  önizlemesi aynı; kilitler; Döndür'ün Kopya'sı ve Referans'ı; sıcak tutamaç; hesaplayıcı şeffaf ölçmenin üstünde; Enter; çizen araç
  şeffaf değil).
- `cargo test -p kentos-desktop transparent_tests` (şeritten Kaydır, Pencere yakınlaştır ve Mesafe ölç; tekerlek, Tümünü göster ve
  Önceki görünümden sonra önizleme imlecin altında).
- İz `fixtures/interaction/v1/transparent.json`: `KENTOS_TRACES_ONLY=transparent cargo test -p kentos-desktop every_trace_passes`;
  bütün izler değişmeden geçer.
- Resimler: `KENTOS_SHOTS_ONLY=seffaf-pencere,seffaf-donus cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
