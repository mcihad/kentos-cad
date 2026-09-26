# Etkileşim izleri

TODOS.md §5 (`UX-01`, `UX-04`, `UX-06`, `UX-07`, `UX-09`) ve [ADR 0018](../../docs/adr/0018-tool-session-and-input.md).

Bir iz, kullanıcının çizim alanında yaptıklarını adım adım yazar: komut seçmek, tıklamak, yazmak, tuşa basmak. Her adımdan sonra görülmesi gerekeni de platformdan bağımsız olarak söyler.

- **Web** izleri bugün gerçek tarayıcıda oynatır: `pnpm e2e:interaction` (`make e2e-interaction`). Başsız Chrome'a gerçek fare ve klavye olayları gönderilir.
- **Masaüstü** aynı dosyaları değiştirmeden, pencere açmadan oynatır: `cargo test -p kentos-desktop traces` (`apps/desktop/src/traces/`, [ADR 0021](../../docs/adr/0021-native-tool-session.md)). Tuşlar ve fare, uygulamanın kendi aboneliğinden ve çizim alanının kendi hareket kodundan geçen Iced olaylarıdır. `kentos-cad snapshot çıktı.png --iz <iz> --adim <n>` bir izi görüntüye oynatır.

İz, iki uygulamanın kullanım davranışının ortak referansıdır.

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
| `v1/empty.kcad` | İzlerin başladığı boş çizim (`.kcad` v1) |
| `v1/objects.kcad` | Seçim ve kenet izlerinin çizimi: çizgiler (1–3; 2 ile 3 (9,6; 8,8)'de kesişir), kapalı alan (4), nokta (5), kilitli katmanda çizgi (6), gizli katmanda çizgi (7) |
| `v1/edits.kcad` | Değiştirme izlerinin çizimi (ADR 0047): (−20, 12)'de kesişen 1 ve 2, x = −4'te sınır 3, köşesi (0, 4)'te L biçimli çoklu çizgi 4, (14, 4)'te birleşen 5 ve 6, kırılacak 7, (8, −4)'te uç uca gelen 8 ve 9, kapalı alan 10, 10 m'lik 11, 12, 9'un ucundan devam eden, kilitli katmandaki 13 |
| `v1/tools.kcad` | Çizim araçlarının izleri (ADR 0057): bölünecek 20 m'lik çizgi 1, dik referansı çizgi 2, (16, −8)'de 4 m'lik daire 3, kilitli katmanda çoklu çizgi 4; kot noktalarının `kot` katmanı |

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
| `key` | Tek tuş: `Enter`, `Esc`, `Tab`, `Backspace`, `Space`, `Delete`, `F3` (kenet), `F8` (orto), bir harf (`G`), `-`, `+` ya da `Ctrl+`, `Shift+`, `Ctrl+Shift+` akoru (`Ctrl+Z`, `Shift+H`, `Ctrl+Shift+V`). Shift'le basılan harf, klavyenin yaptığı gibi büyük gelir |
| `text` | Karakterler tek tek yazılır. Klavyenin ürettiği metin sayılır, fiziksel tuş konumu değil |
| `move` | İmleç çizimde bu noktaya gelir |
| `click`, `doubleClick` | Sol tuşla tıklama ya da çift tıklama |
| `drag` | `[[doğu, kuzey], [doğu, kuzey]]`: sol tuş ilk noktada basılır, imleç ortadan ikinci noktaya gider, orada bırakılır (seçim kutusu) |
| `rightClick` | Sağ tuşa kısa basıp bırakma; menüyü açan basılı tutmadan kısa |
| `focus` | Klavye odağı: `commandLine` (komut satırına tıklamak) |
| `saveAndReopen` | Uygulamanın kendi kaydetme komutuyla yeni bir dosyaya yazar ve o dosyayı yeniden açar |

`shift: true`, `click` ya da `drag` adımında tuşa basılıyken Shift'in basılı olduğunu söyler (seçime ekleme ve çıkarma).

**Beklentiler.** `expect` isteğe bağlıdır ve eylemden sonra denetlenir. Yalnız yazılan alanlar karşılaştırılır:

| Alan | Anlamı |
|---|---|
| `tool` | Etkin araç; komut yokken `select` |
| `points` | Çalışan komutun aldığı nokta sayısı |
| `options` | İstemdeki seçenek tuşları, sırasıyla (`["Y", "U", "G", "Enter"]`) |
| `dynamicInput` | İmleç yanındaki değer alanının metni; kapalıysa `null` |
| `commandLine` | Komut satırının metni |
| `entities` | Çizimdeki nesne sayısı |
| `newest` | En son oluşturulan nesne: `kind`, köşeler `points` (çizginin iki ucu: başlangıç, bitiş; noktanın yeri; yayın saat yönünün tersine başlangıcı ve bitişi), ardışık köşe farkları `edges`, yaylı kenar sayısı `arcs`, dairenin ya da yayın merkezi `center` ve yarıçapı `radius` (ikisi de `clickTolerance` içinde). Eğrinin köşeleri geçtiği noktalardır; elipsin `points`'i eksen uçlarıdır (büyük, küçük, saat yönünün tersine; eliptik yayda başlangıç ve bitiş), `center`'ı merkezidir; yardımcı çizginin ve ışının `points`'i geçtiği nokta ve doğrultusunda bir metre ötesidir (ADR 0057) |
| `canUndo`, `canRedo`, `dirty` | Geri al, yinele ve kaydedilmemiş değişiklik |
| `log` | Son iletinin düzeyi: `success`, `info`, `warn`, `error` |
| `metresPerPixel` | Görünümün ölçeği |
| `viewCenter` | Görünümün merkezi, `view.center`'a göre doğu ve kuzey farkı: Kaydır'ın ve yakınlaştırmaların bıraktığı yer (ADR 0056) |
| `selected` | Seçili nesnelerin kimlikleri, seçildikleri sırayla (`[1, 4]`) |
| `hover` | İmlecin altında vurgulanan nesnenin kimliği; yoksa `null` |
| `snap` | Kenet işaretinin türü (`endpoint`, `midpoint`, `center`, `node`, `quadrant`, `intersection`, `perpendicular`, `tangent`, `nearest`); yoksa `null` |
| `ids` | Çizimdeki nesnelerin kimlikleri, belge sırasıyla: geri alınan silmenin nesneleri yerlerine döner |
| `objects` | Kimliğiyle verilen nesneler, her biri `newest` gibi (`id`, `kind`, `points` …): yerinde taşınan, döndürülen ya da aynalanan nesne (ADR 0037) |

`note`, adımın neyi gösterdiğini okura anlatır; denetlenmez.

## Karşılaştırma kuralları

- **Tıklanan nokta** ekran pikselinden gelir. `clickTolerance` içinde karşılaştırılır.
- **Yazılan değer** kesindir. `edges` her köşeden sonrakine olan farktır ve tam eşit olmalıdır. Tıklanan ilk noktadan aynı piksel satırında `12` yazmak, tam `[12, 0]` verir.
- **`metresPerPixel`** göreli `1e-9` ile karşılaştırılır.
- **`viewCenter`** tıklamalardan gelir; `clickTolerance` içinde karşılaştırılır.
- **Noktalar çizim alanında kalır.** Merkezden doğuya ve batıya en çok 240, kuzeye ve güneye en çok 160 piksel uzakta olurlar. `0,125` m/piksel ölçekte bu ±30 × ±20 m eder. En küçük desteklenen pencere (1100×600) bu kutuyu çizim alanında gösterir. Oynatıcı alanın dışına düşen noktayı sessizce kaçırmaz; izi hatayla durdurur.
- **Görünüm değişince** noktalar o anki görünüme göre alanda kalır: Kaydır'dan sonra kutu görünümle birlikte kayar. Bir kutuya yakınlaştırmanın (Seçime yakınlaştır, Pencere yakınlaştır) ölçeği alanın boyutuna bağlıdır ve iki uygulamada farklıdır; o ölçek karşılaştırılmaz, sonraki noktalar sığdırılan kutunun içinden seçilir, çünkü kutu iki uygulamada da görünür.

## Kurallar

- İz, web'in bugünkü davranışını yazar; web geçmeden iz eklenmez.
- Davranış değişecekse sıra şudur:
  1. karar (ADR 0018);
  2. iki uygulamada değişiklik;
  3. izin güncellenmesi.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
- Yeni bir iz ya da alan eklenince bu belge ve iki oynatıcı birlikte güncellenir: web (`apps/web/scripts/e2e/interaction.mjs`) ve masaüstü (`apps/desktop/src/traces/`). Masaüstü oynatıcısı bilmediği alanda durur.
- İz, araçların oturum boyunca hatırladıklarını (web'in statik alanları: son daire yarıçapı, dikdörtgenin dönmesi ve köşeleri, düzgün çokgenin kenar sayısı ve çemberi, öteleme mesafesi ve Noktadan geç, Kırp, uzat-kısalt kipi ve değerleri, dizinin son satır, sütun ve aralığı, kutupsal dizinin adedi, açısı ve dönmesi, Hizala'nın Ölçekle'si, yardımcı çizginin açısı, paralel çizginin mesafeleri, ekseni ve alanı, halkanın çapları, bulutun biçimi ve yay boyu, Böl'ün kipi, parça sayısı ve aralığı) başladığı gibi bırakır. Geri kurulamayanlar (son köşe yarıçapı ve pah mesafeleri) izin ilk yazdığıyla kurulur; ondan önceki beklentiler onlara bağlı değildir. Pano ve son komut da oturum boyunca kalır; iz onların başlangıcına dayanmaz: yapıştırmadan önce kendisi kopyalar ya da keser, Enter'la yinelemeden önce kendisi bir komut başlatır (ADR 0056). Web oynatıcısı sayfayı izler ve varyantlar arasında yeniden açmaz; masaüstü her izi yeni bir uygulamada oynatır (ADR 0032).
- Yazılan değerin dilbilgisi ayrı bir dosyadadır: `fixtures/point-input/v1/cases.json`. Web'in ve masaüstünün okuyucusu onu okur.

## Varyantlar

Web oynatıcısı her izi üç varyantta oynatır. Masaüstü de aynısını yapar.

| Varyant | Anlamı |
|---|---|
| `us` | US klavye, 1× ekran |
| `tr-q` | Türkçe Q klavye. `+` Shift+4'le, `-` `*`'ın sağındaki tuşla, `@` AltGr+Q ile yazılır; AltGr Windows'taki gibi Ctrl+Alt olarak gelir |
| `hidpi` | US klavye, 2× ekran (HiDPI) |

Bir varyantı seçmek için: `pnpm e2e:interaction -- --variant=tr-q`.

**Odak başka bir metin alanındayken** yazma durumu `polygon-keys`'te. **Çizim alanından komut satırına** yazma ve öneri listesi `command-name`'de. Masaüstü oynatıcısı komut satırını bileşenin bir modeliyle izler: odak işlemleri (bir harf komut satırını odaklar, komut satırından başlayan araç odağı çizime geri verir) ve öneri listesi (liste açıkken Enter ve Boşluk vurgulanan öneriyi çalıştırır, Tab adını yazar). Bir test modeli gerçek bileşene tuş tuş ve işlem işlem bağlar ([ADR 0027](../../docs/adr/0027-line-and-polyline-commands.md)).

§5 kabul izinin istediği şu varyantlar henüz yok:
- Türkçe F klavye;
- IME açıkken yazma.
