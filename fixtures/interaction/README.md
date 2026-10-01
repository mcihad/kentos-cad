# Etkileşim izleri

TODOS.md §5 (`UX-01`, `UX-04`, `UX-06`, `UX-07`, `UX-09`) ve [ADR 0018](../../docs/adr/0018-tool-session-and-input.md).

Bir iz, kullanıcının çizim alanında yaptıklarını adım adım yazar: komut seçmek, tıklamak, yazmak, tuşa basmak. Her adımdan sonra görülmesi gerekeni de platformdan bağımsız olarak söyler.

- **Web** izleri bugün gerçek tarayıcıda oynatır: `pnpm e2e:interaction` (`make e2e-interaction`). Başsız Chrome'a gerçek fare ve klavye olayları gönderilir.
- **Masaüstü** aynı dosyaları değiştirmeden, pencere açmadan oynatır: `cargo test -p kentos-desktop traces` (`apps/desktop/src/traces/`, [ADR 0021](../../docs/adr/0021-native-tool-session.md)). Tuşlar ve fare, uygulamanın kendi aboneliğinden ve çizim alanının kendi hareket kodundan geçen Iced olaylarıdır. `kentos-cad snapshot çıktı.png --iz <iz> --adim <n>` bir izi görüntüye oynatır.

İz, iki uygulamanın kullanım davranışının ortak referansıdır.

**Kullanım senaryoları.** `usage-` ile başlayan izler, uygulamayı gerçek bir iş akışıyla baştan sona kullanır ve `shot` adımlarında resim ister. Test olarak öbür izler gibi oynatılırlar. Resim olarak da şöyle oynatılırlar:

```bash
./target/debug/kentos-cad kullan usage-parcel            # masaüstü: .run/shots/kullanim/masaustu-<iz>-<nn>-<ad>-<GxY>[-acik].png
pnpm -C apps/web e2e:use usage-parcel                    # web: aynı adlar web- ile
python3 scripts/usage/compare.py usage-parcel            # iki platform yan yana: karsilastir-…png
```

Her boyut (1440×900, 1100×650) ve tema (koyu, açık) senaryoyu baştan oynatır; yüksekliği 800 px ve üstü olan pencerenin resminde komut geçmişi açıktır ve son satırı görünür (küçük pencerede son ileti durum çubuğundadır). Beklentiler yol boyunca denetlenir ve söylenir.

| Dosya | İçerik |
|---|---|
| `v1/polygon-accept.json` | §5 kabul izi: poligon başlat, tıkla, `12` yaz, Enter, sonraki nokta, kapat, geri al, yinele, kaydet ve aç |
| `v1/polygon-signs.json` | Değer yazmaya `-` ya da `+` ile başlamak (`UX-04`) |
| `v1/polygon-keys.json` | Esc, Geri (G), Ctrl+Z, sağ tık, çift tık, eksik nokta, Backspace, Tab, Boşluk, son komutu yinele, odak (`UX-06`) |
| `v1/polygon-close.json` | İlk köşeye dönmek alanı kapatır: tıklama, yakınına tıklama, yazma, üç köşeden az, yayla kapatma |
| `v1/line-chain.json` | Çizgi aracı ([ADR 0027](../../docs/adr/0027-line-and-polyline-commands.md)): tıkla, `12` yaz, Enter, Geri (G), Ctrl+Z, Kapat (K), her parçanın ayrı geri alınması, sağ tık, tek noktayla onay |
| `v1/polyline-arc.json` | Çoklu çizgi aracı (ADR 0027): tıkla, `12` yaz, Enter, yay parçası, Geri (G), düz parça, Ctrl+Z, sağ tıkla bitirme, tek adımda geri alma, Uzunluk (U) |
| `v1/command-name.json` | Çizim alanından komut adı yazmak (ADR 0018, 6. adım): kısayolu olmayan harf komut satırını açar, Esc yazılanı siler, Enter önerilen komutu (`ka` → Kapalı alan) başlatır, klavye çizime döner |
| `v1/snap-polygon.json` | Kenet ([ADR 0029](../../docs/adr/0029-desktop-selection-and-snap.md)): kapalı alanın köşeleri var olan çizimin uç, orta ve kesişim noktalarına tam oturur; orto (F8) kenetlenen noktayı kaydırmaz; F3 keneti kapatır; gizli katman kenetlenmez, kilitli katman kenetlenir; nokta nesnesi; seçim aracı kenetlenmez |
| `v1/select-delete.json` | Seçim (ADR 0029): üzerine gelme, tıklama, Shift ile ekleme ve çıkarma, soldan sağa pencere ve sağdan sola kesişim, Esc, gizli ve kilitli katmanlar; Delete seçimi `cad.entities.delete` ile siler, kilitli nesne kalır, Ctrl+Z aynı nesneleri yerlerine getirir; seçimsiz Delete tıklananı siler |
| `v1/point-series.json` | Nokta aracı ([ADR 0032](../../docs/adr/0032-desktop-drawing-tools.md)): kenetlenen, tıklanan ve yazılan noktalar; her nokta ayrı nesne ve adım; Ctrl+Z en yenisini geri alır; Enter araçtan çıkar |
| `v1/circle-methods.json` | Daire aracı (ADR 0032): merkez ve yarıçap (tıklanan, yazılan, Çap ile), 2N, 3N, iki nesneye teğet ve yarıçaplı (TTY; yazılan, sonra Enter ile son yarıçap), üç nesneye teğet (TTT) |
| `v1/arc-variants.json` | Yay aracı (ADR 0032): nesne yokken Devam uyarısı; üç nokta; başlangıç–merkez ve açı; başlangıç–bitiş ve yarıçap; önce merkez; çizgiye teğet Devam |
| `v1/rect-options.json` | Dikdörtgen aracı (ADR 0032): iki köşe, köşe yuvarlama, döndürme ve boyutlar, pah; döndürülmüş dikdörtgen; düzgün çokgen |
| `v1/move-copy.json` | Taşı ve Kopyala ([ADR 0037](../../docs/adr/0037-desktop-modify-tools.md)): seçim yokken araç önce seçer (tıklama, pencere, sağ tıkla devam); kenetlenen temel nokta; yazılan `@dY,dX` ile taşıma, tek adımda geri alma; her tık bir kopya, kilitli katmandaki nesne kopyalanmaz; Bitir (Enter) |
| `v1/rotate-scale.json` | Döndür ve Ölçekle (ADR 0037): kenetlenen merkez, Kopya (K) ile yazılan 90°; referans uzunluğuyla (R) ve yazılan faktörle ölçekleme |
| `v1/mirror.json` | Aynala (ADR 0037): kenetlenen eksenle simetrik kopya; Kaynağı sil (S) ile yazılan eksene göre yerinde çevirme, tek adımda geri alma |
| `v1/edge-tools.json` | Buda, Uzat ve Ötele ([ADR 0047](../../docs/adr/0047-desktop-edit-tools.md)): görünen kenarlarla budama, Shift ile öbür işlem, Sınır seç (S) ve Tüm kenarlar (T), kilitli çizgi düzenlenmez, tek adımda geri alma; yazılan mesafe ve Noktadan geç (N) |
| `v1/corner-tools.json` | Köşe yuvarla ve Pah (ADR 0047): imleç altındaki köşe, yazılan yarıçap, Enter ile son yarıçap, sırayla seçilen iki çizgi, iki mesafeli pah, Kırp (K) kapalıyken yalnız pah çizgisi |
| `v1/path-edit-tools.json` | Kır, Köşe ekle/sil ve Uzat-kısalt (ADR 0047): kenetlenen kırılma noktası, Enter ile tek noktadan bölme; köşe ekleme ve silme; ucu fareyle izleme, yazılan toplam uzunluk, Toplam (T) kipi |
| `v1/object-tools.json` | Birleştir ve Patlat (ADR 0047): seçimden önce seçme, kesişim penceresi, kilitli çizgi atlanır; seçimle hemen çalışma, parçalar seçilir; temel nesne patlatılmaz |
| `v1/stretch-align.json` | Esnet ve Hizala (ADR 0047, 2. kısım): tıklanan ve sürüklenen pencere, yazılan fark, kilitli çizgi atlanıp pencerenin yeniden istenmesi, seçim varken yalnız seçili nesne; iki çiftle döndürme, Ölçekle (Ö), sağ tıkla yalnız taşıma, çakışan ikinci hedef |
| `v1/arrays.json` | Dizi ve Kutupsal dizi (ADR 0047, 2. kısım): Enter ile son sayılar, iki noktayla aralık, yazılan sayılar ve aralık, reddedilen sıfır satır aralığı, kilitli çizginin kopyası yapılmaz; yazılan merkez, Adet (N), Açı (A), Nesneleri döndür (D), dönmeden yarım tur; her dizi tek adımda geri alınır |
| `v1/clipboard.json` | Pano ([ADR 0056](../../docs/adr/0056-desktop-clipboard-and-view-tools.md)): Ctrl+C, Ctrl+V ile yapıştırma aracı (tıklanan yer, yazılan `@dY,dX` temel noktadan), her yapıştırmada yeni nesne ve tek adımda geri alma; Ctrl+X kilitli nesneyi kesmez; Ctrl+Shift+V özgün koordinatlara yapıştırır; Esc ve Enter yapıştırmadan çıkar |
| `v1/navigation.json` | Görünüm araçları (ADR 0056): Kaydır (Shift+H) sürükleyerek taşır, onay almaz, son komut sayılmaz; Seçime yakınlaştır (Ctrl+Shift+F); Pencere yakınlaştır (Z) iki tıkla ve sürükleyerek, Enter onu yeniden başlatır; Son komutu yinele |
| `v1/ellipse-spline.json` | Elips ve Eğri ([ADR 0057](../../docs/adr/0057-desktop-drawing-tools-3.md)): eksenin iki ucu ve yazılan yarı eksen; Yay (Y) ve Merkez (M) ile döndürme açısından (D) eliptik yay, reddedilen açı, başlangıç ve bitiş açısı; eğrinin noktaları, Geri (G), Enter ile açık, Kapat (K) ile kapalı eğri, Ctrl+Z, tek noktayla Enter |
| `v1/construction-lines.json` | Yardımcı çizgi ve Işın (ADR 0057): bir noktadan geçen çizgiler, Enter; Yatay (Y), yazılan Açı (A), Açıortay (B); başlangıçtan ışınlar, Ctrl+Z en yenisini geri alır |
| `v1/parallel-line.json` | Paralel çizgi (ADR 0057): köşelerde birleşen iki yan ve eksen, Geri (G), sağ tıkla bitirme, tek adımda geri alma; yazılan ve iki noktayla gösterilen mesafe, Enter eski değeri korur; Eksen (E) ve Alan olarak (U) ile koridor alanı, Kapat (K) |
| `v1/perpendiculars.json` | Dik in ve Dik çık (ADR 0057): referans hattın başlangıcı tıklamaya yakın uç, hatta ve uzantısına dik inme, hattın üstündeki nokta, daireye merkezden dik, Başka hat (H); yazılan ve gösterilen dik ayak ve dik boy; kilitli katmandaki kenar referans olur |
| `v1/donut-cloud.json` | Halka ve Revizyon bulutu (ADR 0057): her tıkta dolu halka, Dış çap (D) ve İç çap (İ), reddedilen dış çap, iç çap 0 ile dolu daire, Ctrl+Z; Yay boyu (U) ile dikdörtgen bulut, Çokgen (Ç), üç köşesiz bulut uyarısı |
| `v1/spot-divide.json` | Kot noktası ve Böl (ADR 0057): yazılan kotla kot katmanına nokta, kot beklenirken tık bekler; yazılan parça sayısıyla bölme, tek adımda geri alma, Aralık (A) ile uçtan ölçme, kilitli katmandaki nesne de bölünür, geçersiz parça sayısı, Esc |
| `v1/dimensions.json` | Ölçülendirme ([ADR 0061](../../docs/adr/0061-desktop-dimension-tool.md)): iki nokta ve yeriyle hizalı ölçü, çizimin geri alması; Düşey (X) kilidiyle yazılan mesafeli doğrusal ölçü, Yön (O), noktalar varken Enter; iki kenar arasında açı (boş yer ve paralel kenar uyarısı, kilitli katmandaki kenar da); dairenin yarıçapı, Ctrl+Z önce seçilen daireyi bırakır; Açı'nın Daireden yolu (dairedeki nokta, ikinci nokta) ve yerleştirilirken açılan Zemin (Z), değer zeminli; Doğrusal'ın Açı (A) ile yazılan ölçme doğrultusu (50 grad, [ADR 0147](../../docs/adr/0147-new-dimension-kinds.md) §7), sonraki ölçüde de durur, Yön (O) imlece geri verir; Enter araçtan çıkar |
| `v1/dimension-kinds.json` | Ölçülendirme'nin Koordinat ve Yay uzunluğu yöntemleri ([ADR 0147](../../docs/adr/0147-new-dimension-kinds.md) §7), `dimensions.kcad` üzerinde: parsel köşesinin Y'si imleçten, X'i kilitle (kilit sonraki ölçüde de durur), Eksen (O) imlece geri verir, çizginin boyu yazılır; yol kenarının yay uzunluğu yazılan uzaklıkla, Kısmi (K) ile yayın üstünde iki nokta arası; Ctrl+Z ölçüyü kaldırır; Kısmi ve Hizalı'ya dönülerek araçların belleği başladığı gibi bırakılır |
| `v1/dimension-more-kinds.json` | Ölçülendirme'nin Kırıklı yarıçap, Semt ve Eğim yöntemleri ([ADR 0147](../../docs/adr/0147-new-dimension-kinds.md) §7), `dimension-ground.kcad` üzerinde, kenet açık: yolun dış kenarında kırıklı yarıçap (gösterilen merkez 20 m geride ve 3 m yanda, yaydaki nokta, yazılan kırık uzaklığı); parselin batı kenarının semti iki noktayla, güney kenarınınki Kenardan (K) ve yazılan uzaklıkla; kotlu iki nokta arasında eğim, kotlar kenetlenen noktalardan; kotsuz köşelerde kotlar sorulur ve yazılır; Ctrl+Z ölçüyü kaldırır; yolun dış kenarının kendi açısı (Açı, Yaydan), yazılan yarıçapla; Hizalı'ya dönülür |
| `v1/quick-dimension.json` | Hızlı ölçü ([ADR 0147](../../docs/adr/0147-new-dimension-kinds.md) §7), `quick-dimension.kcad` üzerinde: seçim yokken önce iki parsel ve yaylı yol kenarı tıklanarak seçilir, Enter; imleç parsellerin 4 m üstündeyken tık dokuz ölçüyü tek adımda yazar (parsellerde dışarıda, ortak kenar bir kez, yolda imlecin tarafında, yaya yay uzunluğu; araç çıkar); Ctrl+Z tek adımı geri alır; seçim dururken araç hemen yeri sorar, Zemin (Z) ve yazılan uzaklık (3 m); bellek başladığı gibi bırakılır, Esc çıkar |
| `v1/quick-dimension.kcad` | `quick-dimension.json`'un belgesi; yerler (487000, 4420000)'e göre: yan yana iki parsel (0, 0)–(20, 30) ve (20, 0)–(40, 30), ortak kenarları x = 20; (0, −10)'dan (10, −10)'a düz, sonra (20, −20)'ye saat yönünde çeyrek yay olan yol kenarı; (50, 50)'de nokta; etkin katman Ölçü |
| `v1/dimension-ground.kcad` | `dimension-more-kinds.json`'un belgesi; yerler (487000, 4420000)'e göre: parsel (0, 0), (40, 0), (44, 32), (2, 30); (20, −60) merkezli 50 ve 42 m yarıçaplı yol kenarları (50°–130°); parselin doğusunda kotu 102.40 olan (50, 0) ve 101.15 olan (54, 32) noktaları |
| `v1/topology.json` | Topolojik temizlik ([ADR 0148](../../docs/adr/0148-topology-cleanup.md) §9), `topology.kcad` üzerinde, seçim yokken bütün çizim: hatırlanan 0,01 m toleransla bir uç birleşir; yazılan 0,05 m'de dört uç birleşir (ikisi kotlu noktaya), kısa kalan uç kilitli yola uzar, taşan uç budanır; Uzat (Z) kapalıyken uç yola taşınır; Köşeler (K) komşu alanın iki köşesini birleştirir; her yeni bulgu iletiyle söylenir; Enter tek adımda yazar (yerler ve alanın köşeleri), Ctrl+Z geri alır; tolerans ve işler hatırlanır; T toleransı sorar; bellek başladığı gibi bırakılır, Esc çıkar |
| `v1/topology.kcad` | `topology.json`'un belgesi; yerler (487000, 4420000)'e göre: (−20, 0)–(20, 15) parsel bloğunun çizgileri; batıdan (0, 0)'a gelen çizgi, 6 mm doğusundan başlayan çizgi ve 3 cm yukarıda biten bölücü; kuzey kenarı (19,97, 15)'te, doğu kenarı (20, 14,98)'de biten ve kotu 102,5 olan (20, 15) noktası; x = −10'da kuzey kenarını 2 cm geçen, x = 10'da (kilitli) yola 3 cm varmayan çizgi; kilitli Yol katmanında (−25, −10)–(25, −10); (4, 4)–(9, 10) alanı ve köşeleri 4 ve 3 mm ötede başlayan komşusu |
| `v1/polygonize.json` | Toplu alan ([ADR 0151](../../docs/adr/0151-polygonize.md) §8), `polygonize.kcad` üzerinde, seçim yokken bütün çizim: altı parsellik adanın bölgeleri ve yapı; altı alan yazılacak (ikisi etiketsiz, biri iki etiketli), bir etiket sınırda, var olan alan yinelenmez, iki uç boşta, ayrıntıları iletiyle; Adalar (A) açılıp kapanır; Ö (O) öznitelik adını imlecin yanındaki yazı kutusunda sorar, “Parsel” yazılır; Enter etkin katmana (Parsel) tek adımda yazar, Ctrl+Z geri alır; bellek başladığı gibi bırakılır, Esc çıkar |
| `v1/polygonize.kcad` | `polygonize.json`'un belgesi; yerler (487000, 4420000)'e göre: (0, 0)–(60, 30) çerçeve, x = 20 ve x = 40'ta düşey, y = 15'te yatay çizgi (altı parsel); sol üst parselde (5, 19)–(13, 26) yapı; çerçevenin dışında (62, 10)–(70, 5) çizgisi; parsellerde 2 m yüksekliğinde numaralar (orta üstte numara ve alan değeri, sağ üstte yok), (40, 8)'de düşey çizginin üstünde etiketli S1 noktası; sağ alt parsel Parsel katmanında zaten alan; etkin katman Parsel |
| `v1/survey-points.json` | Ölçü noktası ([ADR 0152](../../docs/adr/0152-survey-points.md) §2–§4), `survey-points.kcad` üzerinde: Nokta'nın Ad (A) ve Kod (K) yazı kutusunda, Kot (Z) yazılarak; ad her noktada artar (101, 102), nokta adı, `Kod` özniteliği ve kotuyla yazılır; S1'in yerinde soru, Düzelt S1'i 103 yapar (tek adım “Nokta düzelt”), S2'nin yerinde Ekle, Ctrl+Z en yeni noktayı geri alır ve adını geri verir, Atla; Çizgi'de `#S2` ve `#103` adlı noktaların yerlerini verir, `#S1` artık yoktur; boş Enter'lar belleği başladığı gibi bırakır |
| `v1/vertex-points.json` | Köşelere nokta (ADR 0152 §5), `survey-points.kcad` üzerinde: araç önce seçer (iki parsel); ortak köşe bir kez, S1'in durduğu köşe atlanır ve sayılır; Ad (A) 201, Kod (K) PK; Enter beş noktayı adları, kodları ve köşelerin kotlarıyla tek adımda yazar, Nokta'nın adı 206'ya geçer; Ctrl+Z geri alır |
| `v1/survey-points.kcad` | `survey-points.json`'un ve `vertex-points.json`'un belgesi; yerler (487000, 4420000)'e göre: Parsel katmanında (0, 0)–(20, 14) 12 numaralı parsel (köşe kotları 100,12, 100,48, kotsuz, 101,05) ve (20, 0)–(36, 14) 13 numaralı parsel; etkin Nokta katmanında (20, 14)'te S1 (Kod ST) ve (−2, 7)'de S2 (Kod ST, kot 99,8) |
| `v1/hatches.json` | Tarama ([ADR 0062](../../docs/adr/0062-desktop-hatch-tool.md)): parselin içine tıklamak yapıyı ada bırakır, çizimin geri alması; Adalar (A) kapalıyken bütün parsel; kapalı nesne olmayan yer uyarır; Sınır: çizgiler (B) ile çizgilerin kapattığı yüz, Sınır katmanı (K) nesneyle seçilir, Esc seçmeyi bırakır; Dolu desen; Enter araçtan çıkar |
| `v1/areas.json` | Alan işlemleri ([ADR 0065](../../docs/adr/0065-desktop-area-tools.md)): iki parseli tıklayarak seçip birleştirme, tek adımda geri alma; alanı tıklanan noktalarla bölme, oluşacak parçaların önizlemesi, geri alınan bölmenin ilk parçası parselin kendisidir; iki seçimle alan çıkarma; çizgilerin kapattığı karenin içine tıklayarak alan (geri alınan yazmaların kimlikleri yeniden verilmez); delikli parseli çoklu çizgilere çevirme |
| `v1/measure-parcel.json` | Mesafe ölç, Alan hesapla ve Parsel oluştur ([ADR 0067](../../docs/adr/0067-desktop-measure-and-parcel.md)): yolun noktaları, Uzunluk (U) beklerken Ctrl+Z, Geri (G) gibi, noktayı alır ve beklemeyi bitirir, Enter ile toplam uzunluk (hiçbir şey yazılmaz); dört köşe ve ilk köşeye tıklayarak kapanan alanın alanı ve çevresi; parsel katmanına numarasıyla yazılan, seçilen ve tek adımda geri alınan parsel (`areas.kcad` üstünde) |
| `v1/grips.json` | Tutamaçlar ([ADR 0068](../../docs/adr/0068-desktop-grips-and-hover-card.md)): seçili çizginin ucunu sürükleyerek taşıma ve tek adımda geri alma; sürüklenmeden tıklanan tutamaç sıcak kalır, sonraki tık yerleştirir; Esc tutamacı bırakır (seçim kalır), Enter imlecin olduğu yere koyar; kapalı alanın kenar ortası yeni köşe olur; kilitli katmandaki nesnenin tutamacı alınmaz, basmak seçim kutusudur (`objects.kcad` üstünde) |
| `v1/point-calc.json` | Nokta hesapla, çalışan Çizgi'nin içinde (`UX-07`): nokta beklenirken komut satırına yazılan takma ad (`YAN`, `KKES`, `DKES`, `HAT`, `AM`, `ORTA`) hesaplayıcıyı açar, Çizgi askıda kalır; referanslar kenetlenir (kilitli katmandaki çizgi de); hesaplanan nokta Çizgi'ye tıklanmış gibi gider ve iletisi projenin ondalıklarıyla yazılır; iki çözümden tıklanan, paralel doğruların uyarısı, `1/4` oranı, grad açı; okuyamadığı yazıyı hesaplayıcı her adımda kendi sözüyle reddeder (referans çizimde gösterilir; değer türünün biçimiyle, açı projenin biriminde; iki çözümden biri tıklanır), adım kalır; Esc askıdaki komuta hiçbir şey eklemeden döner; Ctrl+Z hesaplayıcının son referansını geri alır, çizimi geri almaz (`objects.kcad` üstünde) |
| `v1/object-tracking.json` | Nesne izleme (`UX-07`), Nokta aracında: kenette durarak (`rest`) izleme noktası alma, noktanın yatay hizasına kilitlenme ve tıklananın hizaya oturması, hizadayken yazılan mesafe (noktadan hiza boyunca), iki noktanın hizalarının kesişimi, alınmış noktada yeniden durarak bırakma, Shift+F3 izlemeyi kapatır ama noktaları tutar, yeniden açınca kalan nokta hiza verir, en çok üç nokta (dördüncüsü en eskisini düşürür), araçtan çıkınca noktalar gider (`objects.kcad` üstünde) |
| `v1/usage-parcel.json` | Kullanım senaryosu: boş çizimde koordinatları yazarak (`Y,X`, `@dY,dX`) parsel oluşturma (parsel katmanı yoksa yeni projedeki gibi açılır, parselle aynı adımda), güney kenarını ölçülendirme, içini tarama, seçip özniteliklerine ve kartına bakma, her işi kendi adımıyla geri alma; kot noktası (kot katmanı yoksa açılır) |
| `v1/block-insert.json` | Blok ekle ([ADR 0144](../../docs/adr/0144-blocks.md), 4. adım): çizimin ilk bloğu yazılan noktaya; Blok (B) sıradaki bloğa geçer; Ölçek (O, Ö'yü seçer; sıfır reddedilir), Dönüş (D) ve Aynala (A) ile iki katı, 90° dönmüş, aynalı direk; araç açıkken Ctrl+Z son yerleştirmeyi geri alır; bellek başladığı gibi bırakılır; Enter bitirir (`blocks.kcad` üstünde) |
| `v1/block-define.json` | Blok oluştur ([ADR 0144](../../docs/adr/0144-blocks.md), 4. adım): yazılan koordinatlarla çember ve çizgiden vana simgesi; soldan sağa pencereyle seçip sağ tıkla onaylama; yazılan taban noktası adı soran pencereyi açar; alınmış ad (“Rögar”) Oluştur'u kapatır; “Vana” adıyla ve açıklamasıyla tanımlanır, nesneler yerleştirmeyle değiştirilir, yerleştirme seçilir; tanım tek adımda geri alınır ve yinelenir; Blok ekle yeni bloğu sunar (`blocks.kcad` üstünde) |
| `v1/block-explode.json` | Patlat ve bloklar ([ADR 0144](../../docs/adr/0144-blocks.md), 3. adım): direk tanımının nesnelerine bir düzey açılır (temel ve kol direğin katmanında, lamba iç içe yerleştirme olarak döner), lamba da patlatılır, her patlatma tek adımda geri alınır; taramalı ve yazılı trafo (`blocks.kcad` üstünde) |
| `v1/block-attribute-insert.json` | Blok ekle ve öznitelik değerleri ([ADR 0144](../../docs/adr/0144-blocks.md) §7, 6b-3 adımı): öznitelikli blokta nokta verilince Öznitelik değerleri sorulur; yazılan değerlerle ve varsayılanla yerleştirilir, Vazgeç noktayı bırakır; patlatılan yerleştirmeler değerleri yazar; geri alınır (`block-attributes.kcad` üstünde) |
| `v1/block-attribute-define.json` | Blok öznitelikleri penceresi ([ADR 0144](../../docs/adr/0144-blocks.md) §7, 6b-2 adımı): seçili rögarın bloğuna NO özniteliği eklenir (Öznitelik ekle, hücreler, yerini seç ile yerleştirmede gösterilen yer, Kaydet); bütün rögarlar kendi numaralarını yazar, yazıya gelmek ve tıklamak rögarı vurgular ve seçer, Patlat numarayı yazı yapar; iki adım geri alınır (`blocks.kcad` üstünde) |
| `v1/block-attributes.json` | Blok öznitelikleri ve Patlat ([ADR 0144](../../docs/adr/0144-blocks.md) §7, 6a adımı): öznitelik yazısına tıklamak yerleştirmeyi seçer; öznitelik yerleştirmenin değeriyle, değeri yoksa varsayılanıyla yazıya açılır; boş değer ve varsayılansız öznitelik yazı olmaz; dönük ve ölçekli yerleştirmenin yazısı onunla döner; her patlatma tek adımda geri alınır (`block-attributes.kcad` üstünde) |
| `v1/usage-blocks.json` | Kullanım senaryosu ([ADR 0144](../../docs/adr/0144-blocks.md), 4. adım): Bloklar paneli açılır; yazılan koordinatlarla vana simgesi çizilir, Blok oluştur ile “Vana” olur ve yerine konur (pencere adımıyla), Blok ekle ile iki yere daha yerleştirilir (hayaletiyle); batıdaki patlatılır ve geri alınır; Blokları temizle kullanılmayan blok bulmaz; kaydedip yeniden açınca bloklar kalır (`blocks.kcad` üstünde) |
| `v1/usage-block-attribute-insert.json` | Kullanım senaryosu ([ADR 0144](../../docs/adr/0144-blocks.md) §7, 6b-3 adımı): Blok ekle öznitelikli rögarı yerleştirirken numarasını ve kotunu sorar; yeni rögar değerleriyle yazılır (`block-attributes.kcad` üstünde) |
| `v1/usage-block-attribute-define.json` | Kullanım senaryosu ([ADR 0144](../../docs/adr/0144-blocks.md) §7, 6b-2 adımı): rögar bloğuna numara özniteliği tanımlanır: Blok öznitelikleri penceresinde satır eklenir ve doldurulur, yeri seçili rögarda gösterilir, kaydedilince bütün rögarlar kendi numaralarını yazar (`blocks.kcad` üstünde) |
| `v1/usage-block-attributes.json` | Kullanım senaryosu ([ADR 0144](../../docs/adr/0144-blocks.md) §7, 6a adımı): rögarlar numaralarını ve kotlarını kendi değerleriyle ya da varsayılanla yazar; kot yazısına gelmek ve tıklamak rögarı vurgular ve seçer, Öznitelikler'de Blok öznitelikleri; varsayılanla yazılan rögar patlatılır ve geri alınır (`block-attributes.kcad` üstünde) |
| `v1/usage-blocks-view.json` | Kullanım senaryosu ([ADR 0144](../../docs/adr/0144-blocks.md), 2. adım): bloklu yol kesitinde yerleştirmeler tanımlarının parçalarıyla çizilir (iç içe blok, bloğun yazısı ve taraması), üzerine gelince ve seçilince bütünüyle vurgulanır; rögarların merkezine kenetlenen kanal hattı; direği kopyalama ve yolun ekseninden karşı kaldırıma aynalama; ağacı taşıma ve ölçekleme; trafoyu döndürme; kesişim penceresiyle ağaçları seçme; geri alma ve yineleme; kaydedip yeniden açma (`blocks.kcad` üstünde) |
| `v1/text-options.json` | Yazı'nın seçenekleri ([ADR 0145](../../docs/adr/0145-text-extras.md) §6, 4. adım): Hiza (H) adıyla, bitişik ve Türkçe imsiz yazılır (`sagust`), hiza adı olmayan söz söylenir; Genişlik (G) 0'ı reddeder, 0,8 alır; Zemin (Z) ve Artır (R) açılır; yazı hizası, çarpanı ve zeminiyle yazılır; Artır'ın önerdiği numara Enter ile yazılır, üstüne yazılabilir, sonu sayı olmayan aynen önerilir; varsayılanlarda hiçbir ek yazılmaz; Ctrl+Z son yazıyı geri alır, Esc çıkar |
| `v1/readable.json` | Okunur yap ([ADR 0145](../../docs/adr/0145-text-extras.md) §6, 4b adımı): seçim yokken önce seçtirir; pencereyle seçilen dört yazıdan ters okunan ikisi kutuları yerinde kalarak yarım döner (ortalı hizalarla, genişlikten bağımsız), okunan kalır, kilitli katmandaki atlanır ve söylenir; tek adımda geri alınır; seçimle hemen çalışır, ters okunan yoksa bunu söyler; yalnız kilitli yazıyla uyarır (`texts.kcad` üstünde) |
| `v1/find-replace.json` | Bul ve değiştir ([ADR 0145](../../docs/adr/0145-text-extras.md) §6, 4c adımı): seçim yokken bütün çizimde aranır; jokerli `Ada *` → `Parsel *` bütün yazıya uyar ve * yakaladığını taşır, hiza kalır; jokersiz, büyük küçük harf eşleşerek `l` → `L` her geçtiği yerde değişir, kilitli katmandaki yazı listelenir ama değişmez ve söylenir; pencere açık kalır; her değiştirme tek adımda geri alınır; seçim varken yalnız seçimde aranır (`texts.kcad` üstünde) |
| `v1/text-file.json` | Metin dosyası yerleştir ([ADR 0145](../../docs/adr/0145-text-extras.md) §6, 4d adımı): araç başlarken açma penceresi sorulur, iz `openFile` ile `satirlar.txt`'yi verir; satırlar Yazı'nın seçenekleriyle alt alta, 1,5 yükseklik aralıkla yazılır, boş satır yerini tutar; tek adımda geri alınır |
| `v1/leader.json` | Kılavuz ([ADR 0146](../../docs/adr/0146-leader.md) §7, 4. adım): okun ucu ve köşeler; tek köşeyle Enter söylenir, araç bekler; Enter kolun ucunda not kutusunu açar, not yazılır, kılavuz tek adımda; Ok (O) adıyla, Türkçe imsiz de (`acik`) yazılır, ok adı olmayan söz söylenir; Zemin (Z) açılır; boş kutuda Enter notsuz kılavuz yazar (notsuzun zemini yazılmaz); Geri (G) son köşeyi siler; seçenekler varsayılanlarına döndürülür (uygulama boyunca kalırlar); Ctrl+Z son kılavuzu geri alır, Esc araçtan çıkar (`empty.kcad` üstünde) |
| `v1/satirlar.txt` | Metin dosyası yerleştir izinin dosyası: üç satır ve arada bir boş satır |
| `v1/empty.kcad` | İzlerin başladığı boş çizim (`.kcad` v1) |
| `v1/objects.kcad` | Seçim, kenet ve nokta hesabı izlerinin çizimi: çizgiler (1–3; 2 ile 3 (9,6; 8,8)'de kesişir), kapalı alan (4), nokta (5), kilitli katmanda çizgi (6), gizli katmanda çizgi (7) |
| `v1/edits.kcad` | Değiştirme izlerinin çizimi (ADR 0047): (−20, 12)'de kesişen 1 ve 2, x = −4'te sınır 3, köşesi (0, 4)'te L biçimli çoklu çizgi 4, (14, 4)'te birleşen 5 ve 6, kırılacak 7, (8, −4)'te uç uca gelen 8 ve 9, kapalı alan 10, 10 m'lik 11, 12, 9'un ucundan devam eden, kilitli katmandaki 13 |
| `v1/hatch.kcad` | Tarama izinin çizimi (ADR 0062): (−28…−8, −6…10)'da 20 × 16 m parsel 1, içinde 8 × 6 m yapı 2 (Yapılar); x = 2, 14, 26 ve y = −6, 10'da iki 12 × 16 m yüz kapatan çizgiler 3–7; soldaki yüzde (6…10, 0…4) Yapılar'da kapalı çoklu çizgi 8 |
| `v1/areas.kcad` | Alan işlemleri izinin çizimi (ADR 0065): Parseller'de (0…10, 0…10)'da parsel 1 ve (6…16, 4…14)'te parsel 2 (24 m² örtüşür); (−20, 8)'de 3 m'lik daire 3; (−28…−18, −15…−5)'te bir kare kapatan çizgiler 4–7; (18…28, −15…−5)'te (21…25, −12…−8) delikli parsel 8; kilitli katmanda (−10…−4, −18…−12) kare 9; y = 2'de çizgi 10 |
| `v1/tools.kcad` | Çizim araçlarının izleri (ADR 0057): bölünecek 20 m'lik çizgi 1, dik referansı çizgi 2, (16, −8)'de 4 m'lik daire 3, kilitli katmanda çoklu çizgi 4; kot noktalarının `kot` katmanı |
| `v1/blocks.kcad` | Blok senaryosunun çizimi (ADR 0144): y = ±4'te yol kenarları, y = 0'da eksen; (−10, 0, 10; −1,5)'te Rögar yerleştirmeleri 4–6 (iki çember ve “R” yazısı); (−6, 4,8)'de ve (8, 4,8)'de kolu güneye dönük Aydınlatma direği 7–8 (temel, kol ve kolun ucunda iç içe Lamba bloğu); (−9, 7,5), (−2, 7,5)'te ve 0,8 ölçekle (4,5, 7,5)'te Ağaç 9–11 (yaylı taç ve gövde); (−4, −7)'de taramalı ve yazılı Trafo 12 |
| `v1/texts.kcad` | Okunur yap izinin çizimi (ADR 0145): (−10, 5)'te 180° dönük orta hizalı “Ada 101” (1), (10, 5)'te 200° dönük orta taban hizalı “Yol 12” (2), (0, −5)'te 30° dönük “Park” (3), kilitli katmanda (0, −12)'de 180° dönük “Kilitli” (4); yükseklikler 2 m |
| `v1/text-extras.kcad` | Yazı ekleri sahnesi ([ADR 0145](../../docs/adr/0145-text-extras.md), 2. adım; iz değil, iki platformun resimleri için): Yazı katmanında on iki hizanın her biri kendi işaretli noktasında (x = 12, 46, 80; y = 30, 20, 10, 0; işaretler İşaret katmanında), (118, 25)'te 30° dönük tam orta; (4; −14, −19, −24)'te 0,6, 1 ve 1,5 genişlik çarpanı; (70…140, −30…−12)'de taramanın ve y = −21'deki çizginin üstünde zeminsiz (88, −16) ve zeminli (122, −24) yazı. Masaüstü `labels::text_extras_screens`, web `shots.mjs texts` |
| `v1/leaders.kcad` | Kılavuz sahnesi ([ADR 0146](../../docs/adr/0146-leader.md); iz değil, iki platformun resimleri için): `fixtures/kcad/v2/leaders.json`'un nesneleri, notları birbirini örtmesin diye aralanmış; yerler (500000, 4400000)'e göre. Kılavuz katmanında (0, 0)'dan (6, 5)'e dolu oklu “Mevcut bina”; (44, −12)'den kırılarak (36, −7)'ye açık oklu, zeminli, kolu sola uzanan “Ø150 PVC”; (48, 0)'dan (52, 6)'ya nokta oklu, 30° dönük, kırmızı ve 0,35 mm “Ada 101 Parsel 5”; (0, −12)'den (9, −8)'e oksuz, notsuz kılavuz; (64, −10)'da üç kat büyük, içinde “V” notlu kılavuz olan Vana bloğu. Masaüstü `labels::leader_screens`, web `shots.mjs leaders` |
| `v1/dimensions.kcad` | Yeni ölçü türleri sahnesi ([ADR 0147](../../docs/adr/0147-new-dimension-kinds.md); iz değil, iki platformun resimleri için); yerler (500000, 4400000)'e göre. Taranmış parsel (0, 0), (40, 0), (44, 32), (2, 30); (20, −60) merkezli 50 ve 42 m yarıçaplı yol kenarları; doğu köşelerde kotlu noktalar (102,40 ve 101,15). Ölçü katmanında: kuzeybatı köşenin Y'si (yukarı, kırık) ve X'i (sola, kırık); yol kenarının 70°–110° yay uzunluğu, 3 m dışarıda; aynı kenarın 125°'deki kırıklı yarıçapı, gösterilen merkez 22 m geride ve 3 m yanda, kırık 10 m'de; batı kenarın semti, 4 m dışarıda; doğu kenarın iki kot arasındaki eğimi, 4 m dışarıda; kuzey kenarın zeminli hizalı ölçüsü, taramanın içinde; (62, 8)'de 90° dönük, içinde 16 m'lik eğim (%2) olan Rampa bloğu. Yay uzunluğu 9 numaralı nesnedir. Masaüstü `labels::dimension_screens`, web `shots.mjs dimensions` |

## Biçim (`kentos.interaction-trace`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.interaction-trace"`, `1` |
| `id`, `title` | Kimlik (dosya adıyla aynı) ve Türkçe açıklama |
| `source`, `covers` | Kaynak ve kapsanan TODOS maddeleri |
| `document` | İlk adımdan önce açılan çizim. Açılınca geri alma geçmişi boştur, çizim kirli değildir |
| `view` | Çizim alanının merkezi `[doğu, kuzey]` ve ölçeği `metresPerPixel` |
| `draft` | Kenet (`snap`), ızgara, ortho, kutupsal izleme ve kenet izlemesi; açık ya da kapalı. Kenet türleri, kenet ve seçim yarıçapı ve kutupsal açı adımı ayarların varsayılanlarıdır (en yakın dışında bütün türler; 11 ve 5 piksel; 45°). Masaüstü ızgara ve nesne izlemesi açık bir izi oynatmaz, açık bir iletiyle durur |
| `prefs` | Kullanıcı tercihleri; bugün yalnız `cursorInput` (imleç yanında değer girişi) |
| `clickTolerance` | Tıklanan noktaların karşılaştırılacağı mesafe, metre |
| `steps` | Adımlar |

Adımlardaki koordinatlar, `view.center`'a göre doğu ve kuzey farklarıdır, metre cinsinden.

**Eylemler.** Her adımda en çok bir eylem bulunur:

| Eylem | Anlamı |
|---|---|
| `run` | Komutu kimliğiyle çalıştırır; şeritten, menüden ya da komut satırından seçmekle aynıdır (`tool.polygon`) |
| `key` | Tek tuş: `Enter`, `Esc`, `Tab`, `Backspace`, `Space`, `Delete`, `F3` (kenet), `F8` (orto), bir harf (`G`), `-`, `+` ya da `Ctrl+`, `Shift+`, `Ctrl+Shift+` akoru (`Ctrl+Z`, `Shift+H`, `Ctrl+Shift+V`, `Shift+F3` nesne izleme). Shift'le basılan harf, klavyenin yaptığı gibi büyük gelir |
| `text` | Karakterler tek tek yazılır. Klavyenin ürettiği metin sayılır, fiziksel tuş konumu değil |
| `move` | İmleç çizimde bu noktaya gelir |
| `rest` | İmleç bu noktaya gelir ve nesne izlemesinin noktayı alacağı (ya da bırakacağı) kadar durur: web'de 350 ms'lik bekleme süresinden uzun (500 ms). Masaüstü oynatıcısı kendi saatini bu süreden öteye ilerletir |
| `click`, `doubleClick` | Sol tuşla tıklama ya da çift tıklama |
| `drag` | `[[doğu, kuzey], [doğu, kuzey]]`: sol tuş ilk noktada basılır, imleç ortadan ikinci noktaya gider, orada bırakılır (seçim kutusu) |
| `rightClick` | Sağ tuşa kısa basıp bırakma; menüyü açan basılı tutmadan kısa |
| `focus` | Klavye odağı: `commandLine` (komut satırına tıklamak) |
| `saveAndReopen` | Uygulamanın kendi kaydetme komutuyla yeni bir dosyaya yazar ve o dosyayı yeniden açar |
| `shot` | Adı verilen resim (`"shot": "parsel"`): resim oynatıcıları (`kentos-cad kullan`, `e2e:use`) uygulamanın o anki hâlini çeker; test oynatıcıları adımı geçer |
| `dialog` | Başlığıyla açık pencereyi yanıtlar (`"dialog": "Blok oluştur"`). Web oynatıcısı pencerenin açılmasını bekler (pencereler ilk açılışta yüklenir), masaüstü açık değilse durur. Sonra, bu sırayla: `fill` alanları etiketleriyle doldurur (alana tıklanır, içindeki seçilir, metin yazılır: `{"Ad": "Vana"}`), `check` onay kutularını yazılarıyla açar ya da kapatır (`{"Seçilenleri blokla değiştir": false}`), `press` yazısıyla verilen düğmeye basar (`"Oluştur"`). Kapalı düğmeye basmak bir şey yapmaz. Web'de fare ve klavyeyle, masaüstünde denetimin gönderdiği iletiyle yapılır (`apps/desktop/src/traces/answers.rs`); izden yanıtlanamayan pencere açık bir iletiyle durur |

`shift: true`, `click` ya da `drag` adımında tuşa basılıyken Shift'in basılı olduğunu söyler (seçime ekleme ve çıkarma).

**Beklentiler.** `expect` isteğe bağlıdır ve eylemden sonra denetlenir. Yalnız yazılan alanlar karşılaştırılır:

| Alan | Anlamı |
|---|---|
| `tool` | Etkin araç; komut yokken `select`. Nokta hesabı çalışırken askıdaki komut (`line`) |
| `points` | Çalışan komutun aldığı nokta sayısı; nokta hesabı çalışırken onun aldığı referans noktaları |
| `options` | İstemdeki seçenek tuşları, sırasıyla (`["Y", "U", "G", "Enter"]`) |
| `prompt` | İstemin tam metni: komutun adı, adımı ve seçenekleri (`Çizgi: sonraki noktayı belirtin [Geri (G) / Bitir (Enter)]`); nokta hesabı çalışırken onun istemi |
| `dynamicInput` | İmleç yanındaki değer alanının metni; kapalıysa `null` |
| `commandLine` | Komut satırının metni |
| `entities` | Çizimdeki nesne sayısı |
| `newest` | En son oluşturulan nesne: `kind`, köşeler `points` (çizginin iki ucu: başlangıç, bitiş; noktanın yeri; yayın saat yönünün tersine başlangıcı ve bitişi), ardışık köşe farkları `edges`, yaylı kenar sayısı `arcs`, dairenin ya da yayın merkezi `center` ve yarıçapı `radius` (ikisi de `clickTolerance` içinde). Eğrinin köşeleri geçtiği noktalardır; elipsin `points`'i eksen uçlarıdır (büyük, küçük, saat yönünün tersine; eliptik yayda başlangıç ve bitiş), `center`'ı merkezidir; yardımcı çizginin ve ışının `points`'i geçtiği nokta ve doğrultusunda bir metre ötesidir (ADR 0057); bir yerleştirmenin `points`'i yerleştirme noktası, sonra tanımın temel noktasından bir metre doğudaki ve bir metre kuzeydeki noktaların yerleştirilmiş yerleridir: dönüş, ölçek ve aynalama birlikte görünür (ADR 0144); bir yazının `points`'i yeri, `text`'i metnidir, metin tam karşılaştırılır (patlatılan blok özniteliğinin değeri, ADR 0144 §7); yazının hizası `align`, genişlik çarpanı `widthFactor` ve dönüşü `rotation` (ADR 0145), kılavuzun oku `arrow` (ADR 0146), yazının, kılavuzun ve ölçünün zemini `mask` tam karşılaştırılır; ölçünün doğrultusu `angle` (derece: doğrusal ölçünün ölçtüğü, koordinat ölçüsünün ekseni) 1e-9 içinde karşılaştırılır, yazılan grad açısı radyandan geçer (ADR 0147); nesnenin etiketi `label` (ölçü noktasının adı; `null` etiketsiz), öznitelikleri `attrs` (hepsi) ve noktanın kotu `z` (`null` kotsuz) tam karşılaştırılır (ADR 0152) |
| `canUndo`, `canRedo`, `dirty` | Geri al, yinele ve kaydedilmemiş değişiklik |
| `log` | Son iletinin düzeyi: `success`, `info`, `warn`, `error` |
| `logged` | Adımın yazdığı iletiler arasında, bu sırayla ve tam metinleriyle bulunması gerekenler (her düzeyde; araya başka iletiler girebilir): sayıların projenin ondalıklarıyla yazılışı, ret iletileri |
| `metresPerPixel` | Görünümün ölçeği |
| `viewCenter` | Görünümün merkezi, `view.center`'a göre doğu ve kuzey farkı: Kaydır'ın ve yakınlaştırmaların bıraktığı yer (ADR 0056) |
| `selected` | Seçili nesnelerin kimlikleri, seçildikleri sırayla (`[1, 4]`) |
| `hover` | İmlecin altında vurgulanan nesnenin kimliği; yoksa `null` |
| `snap` | Kenet işaretinin türü (`endpoint`, `midpoint`, `center`, `node`, `quadrant`, `intersection`, `perpendicular`, `tangent`, `nearest`); yoksa `null` |
| `trackPoints` | Nesne izlemesinin aldığı noktalar, alınma sırasıyla, `view.center`'a göre `[doğu, kuzey]` (`clickTolerance` içinde) |
| `track` | İmlecin nesne izlemesiyle kilitlendiği hiza: `null` ya da `{ point, lines: [{ origin, angle }] }`. `point` kilitlenilen yer (`clickTolerance` içinde), `lines` bir hiza ya da kesişen iki hiza, çekirdeğin sırasıyla; `origin` hizanın alınmış noktası, `angle` doğudan saat yönünün tersine derece (tam). Kenet varken izleme yoktur |
| `ids` | Çizimdeki nesnelerin kimlikleri, belge sırasıyla: geri alınan silmenin nesneleri yerlerine döner |
| `objects` | Kimliğiyle verilen nesneler, her biri `newest` gibi (`id`, `kind`, `points` …): yerinde taşınan, döndürülen ya da aynalanan nesne (ADR 0037). |
| `dialog` | Açık pencerenin başlığı; yoksa `null`. Web oynatıcısı karşılaştırmadan önce pencerenin açılmasını ya da kapanmasını bekler |

`note`, adımın neyi gösterdiğini okura anlatır; denetlenmez.

## Karşılaştırma kuralları

- **Tıklanan nokta** ekran pikselinden gelir. `clickTolerance` içinde karşılaştırılır.
- **Yazılan değer** kesindir. `edges` her köşeden sonrakine olan farktır ve tam eşit olmalıdır. Tıklanan ilk noktadan aynı piksel satırında `12` yazmak, tam `[12, 0]` verir.
- **`metresPerPixel`** göreli `1e-9` ile karşılaştırılır.
- **`viewCenter`** tıklamalardan gelir; `clickTolerance` içinde karşılaştırılır.
- **Noktalar çizim alanında kalır.** Merkezden doğuya ve batıya en çok 240, kuzeye ve güneye en çok 160 piksel uzakta olurlar. `0,125` m/piksel ölçekte bu ±30 × ±20 m eder. En küçük desteklenen pencere (1100×600) bu kutuyu çizim alanında gösterir. Oynatıcı alanın dışına düşen noktayı sessizce kaçırmaz; izi hatayla durdurur. Kullanım senaryosunun resimlerinde web'in 1100×650 penceresinde kayan araç kutusu alanın batısını örter: orada batıya en çok 180 piksel kalır (`usage-blocks-view` 0,0625 m/piksel ölçekte 10,5 m'ye, 168 piksele dek gider).
- **Görünüm değişince** noktalar o anki görünüme göre alanda kalır: Kaydır'dan sonra kutu görünümle birlikte kayar. Bir kutuya yakınlaştırmanın (Seçime yakınlaştır, Pencere yakınlaştır) ölçeği alanın boyutuna bağlıdır ve iki uygulamada farklıdır; o ölçek karşılaştırılmaz, sonraki noktalar sığdırılan kutunun içinden seçilir, çünkü kutu iki uygulamada da görünür.

## Kurallar

- İz, web'in bugünkü davranışını yazar; web geçmeden iz eklenmez.
- Davranış değişecekse sıra şudur:
  1. karar (ADR 0018);
  2. iki uygulamada değişiklik;
  3. izin güncellenmesi.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
- Yeni bir iz ya da alan eklenince bu belge ve iki oynatıcı birlikte güncellenir: web (`apps/web/scripts/e2e/interaction.mjs`) ve masaüstü (`apps/desktop/src/traces/`). Masaüstü oynatıcısı bilmediği alanda durur.
- İz, araçların oturum boyunca hatırladıklarını (web'in statik alanları: son daire yarıçapı, dikdörtgenin dönmesi ve köşeleri, düzgün çokgenin kenar sayısı ve çemberi, öteleme mesafesi ve Noktadan geç, Kırp, uzat-kısalt kipi ve değerleri, dizinin son satır, sütun ve aralığı, kutupsal dizinin adedi, açısı ve dönmesi, Hizala'nın Ölçekle'si, yardımcı çizginin açısı, paralel çizginin mesafeleri, ekseni ve alanı, halkanın çapları, bulutun biçimi ve yay boyu, Böl'ün kipi, parça sayısı ve aralığı, Ölçülendirme'nin biçimi, doğrusal kilidi ve Köşeden'i, Tarama'nın deseni, sınır yolu ve adaları, Alan kesiştir'in Kaynakları sil'i, Alan çıkar'ın Çıkarılanları sil'i, İçine tıklayarak alan'ın Adalar'ı) başladığı gibi bırakır. Geri kurulamayanlar (son köşe yarıçapı ve pah mesafeleri) izin ilk yazdığıyla kurulur; ondan önceki beklentiler onlara bağlı değildir. Pano ve son komut da oturum boyunca kalır; iz onların başlangıcına dayanmaz: yapıştırmadan önce kendisi kopyalar ya da keser, Enter'la yinelemeden önce kendisi bir komut başlatır (ADR 0056). Web oynatıcısı sayfayı izler ve varyantlar arasında yeniden açmaz; masaüstü her izi yeni bir uygulamada oynatır (ADR 0032).
- Yazılan değerin dilbilgisi ayrı bir dosyadadır: `fixtures/point-input/v1/cases.json`. Web'in ve masaüstünün okuyucusu onu okur.

## Varyantlar

Web oynatıcısı her izi üç varyantta oynatır. Masaüstü de aynısını yapar.

| Varyant | Anlamı |
|---|---|
| `us` | US klavye, 1× ekran |
| `tr-q` | Türkçe Q klavye. `+` Shift+4'le, `-` `*`'ın sağındaki tuşla, `/` Shift+7'yle, `@` AltGr+Q ile yazılır; AltGr Windows'taki gibi Ctrl+Alt olarak gelir |
| `hidpi` | US klavye, 2× ekran (HiDPI) |

Bir varyantı seçmek için: `pnpm e2e:interaction -- --variant=tr-q`.

**Odak başka bir metin alanındayken** yazma durumu `polygon-keys`'te. **Çizim alanından komut satırına** yazma ve öneri listesi `command-name`'de. Masaüstü oynatıcısı komut satırını bileşenin bir modeliyle izler: odak işlemleri (bir harf komut satırını odaklar, komut satırından başlayan araç odağı çizime geri verir) ve öneri listesi (liste açıkken Enter ve Boşluk vurgulanan öneriyi çalıştırır, Tab adını yazar). Bir test modeli gerçek bileşene tuş tuş ve işlem işlem bağlar ([ADR 0027](../../docs/adr/0027-line-and-polyline-commands.md)).

§5 kabul izinin istediği şu varyantlar henüz yok:
- Türkçe F klavye;
- IME açıkken yazma.
