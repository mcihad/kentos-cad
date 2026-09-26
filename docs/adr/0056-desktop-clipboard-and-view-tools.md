# ADR 0056: Masaüstünde pano ve görünüm araçları

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.4 (oturum durumu: pano), §4.5, §4.7; TODOS.md `UX-01`, `UX-06`, `UX-09`; ADR 0014 (kalıcı kimlik), 0017 (masaüstü kabuğu), 0018 (araç oturumu ve izler), 0021 (native araç oturumu), 0029 (seçim ve kenet), 0037 (değiştirme araçları)

## Bağlam

- Masaüstü şu web komutlarını "web'de var; masaüstüne henüz taşınmadı" diye geri çeviriyordu: Kes (`edit.cut`, Ctrl+X), Panoya kopyala (`edit.copy`, Ctrl+C), Yapıştır (`edit.paste`, Ctrl+V), Özgün koordinatlara yapıştır (`edit.pasteOriginal`, Ctrl+Shift+V), Kaydır (`tool.pan`, Shift+H), Pencere yakınlaştır (`tool.zoomWindow`, Z), Seçime yakınlaştır (`view.zoomSelection`, Ctrl+Shift+F), Son komutu yinele (`tool.repeat`).
- Web'in davranışı koddan okundu (26 Eylül):
  - Pano uygulamanın kendi belleğidir (`app/clipboard.ts`); sistem panosu kullanılmaz. Nesnelerin `id`'siz ve `uid`'siz kopyalarını seçim sırasıyla ve kutularının sol alt köşesini (temel nokta) tutar.
  - Kes ve yapıştır ürün komutundan değil, belgeden yazar: `doc.transact('Kes', () => doc.remove(…))`, `doc.addMany(…, 'Yapıştır')` (`app/commands.ts`, `tools/editTools.ts` `pasteEntities`).
  - Yapıştırma aracı (`PasteTool`) katalogda değildir: `tools.run(…, 'Yapıştır')` ile başlar, son komut sayılmaz.
  - Kaydır ve Pencere yakınlaştır kamerayı kendileri değiştirir (`camera.panBy`, `camera.fit(kutu, 0)`); `confirm`'leri yoktur. `tool.confirm` (Enter, Boşluk) onayı olmayan araçta son komutu yineler; Kaydır son komut sayılmaz (`ToolManager.lastRepeatable`).
- Masaüstünde araçlar `kentos-interaction`'dadır ve görünümü yalnız okur (`View`); kamera uygulamanındır.

## Karar

### Pano (`kentos_interaction::clipboard`, `apps/desktop/src/clipboard.rs`)

- **Oturumun durumudur** (CLAUDE.md §4.4): `App.clipboard`. Açık çizim değişince kalır; nesneler bir çizimden öbürüne geçer (Özgün koordinatlara yapıştır'ın web'deki amacı budur). Uygulama kapanınca gider.
- **Sistem panosu kullanılmaz.** Web kullanmıyor; karar web'inkidir. Başka programlarla (başka bir KentOS penceresi dahil) alışveriş ayrı bir karardır.
- **Tuttuğu:** seçili nesnelerin kopyaları, seçildikleri sırayla, yuvasız; temel nokta, kutularının sol alt köşesi (geometri deposunun `extent`'i; kutu yoksa başlangıç noktası).
- **Panoya kopyala:** seçim yokken kapalıdır ve bir şey demez. Kilitli katmandaki nesne de kopyalanır (web). "N nesne panoya kopyalandı."
- **Kes:** kilitli katmandaki nesne kesilmez ve seçili kalır: "N nesne kilitli katmanda olduğu için kesilmedi." Hepsi kilitliyse pano değişmez. Kesilenler tek geri alma adımıyla ("Kes") çizimden çıkar: "N nesne panoya kesildi."
- **Yapıştır:** yapıştırma aracı (`paste.rs`) panonun kopyasıyla başlar; çalışan komutun yerine geçer; komut satırında komutun adıyla anılır (YAPISTIR), öbür masaüstü araçları gibi.
  - Nesneler temel noktalarıyla imleci izler: en çok 400 nesnenin dış çizgisi, kesik çizgiyle (web'in hayaletleri, kendi geometri deposundan).
  - Tık ya da yazılan nokta yerleştirir: koordinat, temel noktadan `@dY,dX`, imlece doğru mesafe. Nesne kenetine oturur; orto uygulanmaz (web).
  - Tek geri alma adımı ("Yapıştır"); yapıştırılanlar seçilir, araç çıkar. Enter ve Esc yapıştırmadan çıkar; Ctrl+Z çizimi geri alır.
  - Son komut sayılmaz (`Session::run`).
- **Özgün koordinatlara yapıştır:** yerine, tek adım; yapıştırılanlar seçilir.
- **Yapıştırılan nesne yenidir:** yeni yuva, yeni kalıcı kimlik (ADR 0014); aynı pano kaç kez yapıştırılırsa yapıştırılsın.
- **Katman:** nesne kendi katmanına gider, çizimde varsa ve kilitli değilse; yoksa etkin katmana. Etkin katman kilitliyse ve bir nesne oraya gidecekse hiçbiri yapıştırılmaz: "“Ad” katmanı kilitli; yapıştırılamadı." Araç yine çıkar.
- **Geometri** ortak çekirdeğin öteleme matrisi ve `transform_shape`'idir. Web aynısını kendi deposundaki kopyalarla yapar.
- **Ürün komutu yok.** Web kesme ve yapıştırmayı belgeden yazıyor; masaüstü de öyle yapar. Bunlar için ürün komutu (Python ve AI için) web'le birlikte ayrı bir karardır.

### Görünüm araçları (`kentos_interaction::navigate`)

- **Araç görünüm değişikliğini veri olarak ister:** `Context.view_changes`'a `ViewChange::Pan { dx, dy }` ya da `ViewChange::Fit { bounds, padding }` ekler. Masaüstü çağrıdan sonra kamerasına sırayla uygular; imleç ekranda yerinde kaldığından altındaki dünya noktası görünümle birlikte güncellenir.
- **Kaydır (Shift+H):** sol tuşla sürükleme görünümü imleçle taşır (orta tuş her araçta taşır). Açık el imleci (web'in `grab`'ı). Esc'e dek kalır, kenetlenmez.
- **Pencere yakınlaştır (Z):** iki tık ya da 4 pikselden uzun sürükleme bir kutu verir. Görünüm kutuyu kenar payı olmadan gösterir, araç çıkar. Genişliği ya da yüksekliği olmayan kutu görünümü değiştirmez. Kutu, ilk köşeden imlece vurgu renginde kesik çizgiyle (5/4 piksel) çizilir.
- **İkisi de onay almaz** (`Tool::confirms`): Enter ve Boşluk son komutu yineler; kısa sağ tık masaüstünde bir şey yapmaz (web boştaki menüyü açar; o menü masaüstünde henüz yok).
  - Kaydır son komut sayılmaz: Enter ondan önceki komutu başlatır.
  - Pencere yakınlaştır sayılır: Enter onu yeniden başlatır, ilk köşe bırakılır.
- **Seçime yakınlaştır (Ctrl+Shift+F):** seçimin kutusu kenarlardan 96 piksel içeride sığar (web'in `zoomToSelection`'ı). Seçim yokken kapalıdır.
- **Son komutu yinele:** son başlatılan aracı yeniden başlatır; yoksa bir şey yapmaz. Boş komut satırında Enter ve Boşluk zaten aynısını yapıyordu (`tool.confirm`).

### Kısayollar ve etkinlik

- Ctrl+C, Ctrl+X, Ctrl+V, Ctrl+Shift+V, Ctrl+Shift+F, Shift+H ve Z envanterdeki web kısayollarıdır. Tek tuşlu kısayol yalnız masaüstüne taşınmış komuta ulaşır (ADR 0017). Metin alanı klavyedeyken bu akorlar alanın kendisinindir, çizime ulaşmaz (web'de de öyle).
- Web'in `isEnabled`'ı kapalı komutu çalıştırmaz. Masaüstünde de kapalı olan bu komutlar bir şey yapmaz ve şeritte soluk durur (`App::available`).

### İz biçimi

- `viewCenter` beklentisi: görünümün merkezi, `view.center`'a göre doğu ve kuzey farkı; `clickTolerance` içinde karşılaştırılır.
- `key` eylemi `Shift+` ve `Ctrl+Shift+` akorlarını alır; Shift'le basılan harf büyük gelir. Web oynatıcısı Shift'i önceden yok sayıyordu: `Shift+H` düz H (tarama aracı), `Ctrl+Shift+V` Ctrl+V olarak gidiyordu.
- Yakınlaştırmanın ölçeği alanın boyutuna bağlıdır, iki uygulamada farklıdır; karşılaştırılmaz. Sonraki noktalar sığdırılan kutunun içinden seçilir.
- Pano ve son komut oturumda kalır; iz onların başlangıcına dayanmaz.
- Değişiklik iki oynatıcıda ve `fixtures/interaction/README.md`'de birlikte yapıldı.

## Web'den bilerek ayrılanlar

- **Pencere yakınlaştır'ın kutusu dünya koordinatlarıyla çizilir.** Araç sürerken tekerlekle yakınlaştırılırsa kutu ilk köşeye bağlı kalır. Web ilk köşenin ekrandaki eski yerini kullanır. Görünüm değişmezken ikisi aynıdır.
- **Komut satırı aracı komutun adıyla anar** (YAPISTIR, PAN, Z), web etiketiyle ("Yapıştır"). Bu, öbür masaüstü araçlarının biçimidir.

## Etkileşim izleri

- `fixtures/interaction/v1/clipboard.json`:
  - Ctrl+C, kopyalama çizimi değiştirmez;
  - yapıştırma aracı ve tıklanan yer (kutusu yüksekliği olan çapraz çizgi: temel noktanın iki koordinatı da sayılır);
  - tek adımda geri alma; yazılan `@-10,-20` temel noktadandır, kenarlar kesin;
  - her yapıştırmada yeni kimlik;
  - Ctrl+X kilitli çizgiyi kesmez, seçili bırakır;
  - Ctrl+Shift+V; iki geri alma; Esc ve Enter yapıştırmadan çıkar.
- `fixtures/interaction/v1/navigation.json`:
  - Shift+H ve sürükleme; Enter çizgiyi başlatır;
  - Seçime yakınlaştır;
  - Pencere yakınlaştır iki tıkla ve sürüklemeyle; Enter onu yeniden başlatır (sonraki tık yeni ilk köşedir);
  - `tool.repeat`.
- İki iz web'de ve masaüstünde üç varyantta geçer.

## Ters deneme

Her bozma çalıştırıldıktan sonra geri alındı (`.run/breaks-0056.log`):

1. Masaüstü Kaydır'ı son komut sayarsa navigation izi 5. adımda düşer ("tool: pan, beklenen line"); `navigate` testi de düşer.
2. Masaüstünde Enter onayı olmayan araca onay verirse navigation 5. ve 18. adımda düşer.
3. Masaüstünde Kes kilitli katmanı yok sayarsa clipboard 12. adımda düşer; birim testi de düşer.
4. Temel nokta kutunun sol üst köşesi olursa clipboard 5. adımda düşer. Deneme hazırlanırken izin ilk hâlinin bu bozmayı kaçıracağı görüldü (yatay çizginin kutusunun yüksekliği yok); iz çapraz çizgiyi kopyalayacak biçimde değiştirildi.
5. Pencere yakınlaştır 96 piksel pay bırakırsa izler geçer, çünkü ölçek karşılaştırılmaz ve merkez aynı kalır. Masaüstü uygulama testi (ölçek 30,4, beklenen 40) ve iki `navigate` testi düşer.
6. Web Kaydır'ı son komut sayarsa, 7. web'de Kes kilitli katmanı yok sayarsa web izleri masaüstüyle aynı adımlarda düşer.
8. Web'de Kaydır görünümü ters yöne taşırsa navigation 4. adımda `viewCenter` ile düşer.
9. Web oynatıcısı Shift'i yok sayarsa `Shift+H` tarama aracını başlatır, `Ctrl+Shift+V` yapıştırma aracını: iki iz de düşer.

## Bu dilimde olmayanlar

- Sistem panosu ve başka programlarla alışveriş.
- Boştaki ve komut sağ tık menüleri.
- Nokta hesaplayıcı (`acceptPoint`) ve tutamaçlar.
- Kes ve Yapıştır için ürün komutu.
- Yapıştırma aracında orto ve kutupsal izleme (web'de de yok).

## Doğrulama (26 Eylül 2026, Linux; main `f627a27` birleştirilmiş, dal `worktree-agent-afc824c9621543135`)

- `pnpm rust:test`: 876 geçti, 5 atlandı; clippy `-D warnings` ve bağımlılık yönü temiz. Yeni `kentos-interaction` testleri: `clipboard` (10), `navigate` (5).
- `pnpm rust:test:desktop`: 461 geçti, 63 atlandı; clippy temiz. Yeni masaüstü testleri (`clipboard::tests`, 4): kısayollar ve etkinlik, panonun çizim değişince kalması, Kaydır ve Pencere yakınlaştır'ın kamerası ve Enter'ı, Seçime yakınlaştır'ın 96 piksel payı.
- İzler: 24 iz × 3 varyant masaüstünde (`cargo test -p kentos-desktop traces`) ve web'de (`pnpm e2e:interaction`) geçti.
- `pnpm typecheck`, `pnpm test` (1502 geçti, 14 atlandı), `pnpm inventory:check` geçti. Masaüstü 86 / 167 komutu çalıştırıyor.
- Çalıştırılmayan: `pnpm build` ve `pnpm e2e`. Bu dilim web uygulamasının kaynağını değiştirmedi; yalnız iz oynatıcısı değişti, o da `pnpm e2e:interaction`'da koştu.
- Görüntüler (`.run/shots`, `cargo test -p kentos-desktop clipboard::screens -- --ignored --nocapture`); koyu ve açık, 1440×900 ve 1100×650:
  - `pano-hayalet-*`: yapıştırılacak çizginin hayaleti;
  - `pano-yapistirildi-*`: yapıştırılan çizgi seçili;
  - `pano-deger-*`: yazılan `@-10,-20`;
  - `pano-kes-*`: kilitli çizgi seçili kalır, uyarı ve sonuç iletisi;
  - `gorunum-kaydir-*`: Kaydır etkin, görünüm kaymış;
  - `gorunum-secime-*`: seçime yakınlaştırılmış görünüm;
  - `gorunum-pencere-*`: sürüklenen kutu, düğme basılı.
