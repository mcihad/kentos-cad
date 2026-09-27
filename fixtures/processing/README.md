# İşlem araçlarının ortak durumları

İşlem araçlarının (İşlemler, [docs/PROCESSING.md](../../docs/PROCESSING.md)) iki platformda aynı sonucu vermesi için ortak durumlar. Her durum bir çizimde yerleşik bir aracı ya da modeli çalıştırır ve çalıştırmanın ne yaptığını platformdan bağımsız olarak söyler: yeni katmanlar, eklenen, değişen ve silinen nesneler, seçim, özet, iletiler, geri alma adımı ya da ret iletileri.

- **Web**: `apps/web/src/processing/cases.test.ts` (Vitest). Her durum `ProcessingRunner` ile sayfada, sonra işçinin yolundan (`handleJob`) bir kez daha çalışır; ikisi de beklenenle karşılaştırılır.
- **Masaüstü**: `crates/native/processing` (`kentos-processing`) aynı dosyayı oynatır.

Çalıştırmalar ürün komutu değildir: katalogda kaydı ve `CommandResult`'ı yoktur, belgenin işlemiyle tek geri alma adımında yazar. Python ve yapay zekâ için ileride `cad.processing.run` onları saracak (CLAUDE.md §18).

| Dosya | İçerik |
|---|---|
| `v1/parcels.kcad` | Durumların çizimi (`.kcad` v1): Parsel katmanında yan yana üç parsel (1: 0…20, 2: 20…45, 3: 45…60 doğu; 0…30 kuzey; 1 ile 2 x = 20'yi, 2 ile 3 x = 45'i paylaşır; Ada, Parsel, Nitelik öznitelikleri, etiketleri parsel numarası), kilitli katmanda parsel 4, gizli katmanda parsel 5, Çizim'de çoklu çizgi 6 (30 m ve 15 m) ve 20 m'lik çizgi 7, Mevcut noktalar katmanında parsel 1'in üç köşesinde P00001–P00003 (8–10). Koordinatlar (487000, 4420000)'e göre verilmiştir |
| `v1/cases.json` | Durumlar |
| `v1/dialog.json` | İşlem penceresinin davranışı: formlar, oturumlar ve saf kuralların tabloları ([aşağıda](#pencere-kentosprocessing-dialog-sürüm-1)) |

## Biçim (`kentos.processing-cases`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.processing-cases"`, `1` |
| `tolerance` | Koordinatların karşılaştırılacağı mesafe, metre (`1e-9`): iki platform aynı Rust çekirdeğini çağırır |
| `documents` | Çizim başına `defaults`: araçların çizimden aldığı varsayılanlar (`DefaultsContext`: uzunluk ve alan ondalığı, açı birimi, çizim ölçeği, çizim yazı tipi, etkin katman) ve `tools`: her aracın ve modelin o çizimdeki varsayılan değerleri, pencerenin açtığı gibi |
| `cases` | Durumlar |

Bir durum:

| Alan | Anlamı |
|---|---|
| `id`, `title` | Kimlik ve Türkçe açıklama |
| `document` | Çalıştırmadan önce açılan çizim. Açılınca geri alma geçmişi boştur |
| `selection` | Çalıştırmadan önceki seçim (nesne kimlikleri); `selection` kapsamı ve İfadeyle seç'in biçimleri bunu okur |
| `view` | İsteğe bağlı: `visible` kapsamının kutusu, `[minX, minY, maxX, maxY]` mutlak metre; yoksa görünüm yoktur |
| `run` | `{ "tool": "<kimlik>" }` ya da `{ "model": "<kimlik>" }` |
| `values` | Aracın o çizimdeki varsayılanlarının üstüne yazılan değerler, pencerenin verdiği biçimde (`{ "scope": "selection" }`, `{ "layerId": "mevcut" }`, `{ "newName": "Numaralar" }` …) |
| `expect` | Beklenen sonuç |

Beklentiler:

| Alan | Anlamı |
|---|---|
| `status` | `ok`, `invalid` (çalışmadı, değerlerde sorun var) ya da `error` |
| `issues` | `invalid`'de sorunlar, parametre sırasıyla: `param` (parametrenin adı; aracın kendi kuralında yok) ve `message`, tam metin |
| `message` | `error`'da ileti |
| `summary` | `ok`'ta çalıştırmanın özeti, tam metin (geçmişte ve pencerenin alt çubuğunda görünen satır) |
| `log` | Aracın ve çalıştırıcının iletileri, sırasıyla: `{ level, text }` (`info`, `warn`); yazılmazsa hiç ileti yoktur |
| `undo` | Çalıştırmanın tek geri alma adımının adı (aracın adı, modelde modelin adı); `null`: çizim değişmedi, geri alınacak adım yok |
| `layers` | Oluşturulan katmanlar: `id`, `name`, `style` (aracın yeni katman stili, yeni katmanın varsayılanlarının üstüne) |
| `added` | Eklenen nesneler, kimlik sırasıyla; her biri kimlikleri dışındaki bütün alanlarıyla (`kind`, `layerId`, geometri, `label`, `text`, `height`, `rotation`, `attrs` …) |
| `updated` | Değişen nesneler: `id`, bütün `attrs` ve varsa `label` |
| `removed` | Silinen nesnelerin kimlikleri |
| `selection` | Çalıştırmadan sonraki seçim |
| `outputs` | Aracın çıktılarından yazılanlar: sayılar ve kimlik listeleri tam |

## Karşılaştırma kuralları

- Metinler (özet, iletiler, nesnelerin yazıları ve öznitelikleri) tam eşit olmalıdır.
- Eklenen bir nesnenin alan kümesi beklenenle aynı olmalıdır; fazla ya da eksik alan farktır. Yalnız geometri alanları (`p`, `a`, `b`, `c`, `pts`, `holes`) `tolerance` içinde karşılaştırılır, öbür sayılar tam.
- `ok` ve `undo`'su olan her durumda, durum yazmasa da: geri alma adımın adını verir, çizim (nesneler ve katman ağacı) çalıştırmadan önceki hâline döner, başka adım kalmaz; yineleme sonucu geri getirir.
- `undo: null` olan durumda geri alınacak adım yoktur.

## Beklenenlerin kaynağı

Sayılar, adlar, özetler ve ret iletileri araçların kuralından okunur ([docs/PROCESSING.md](../../docs/PROCESSING.md) §4, §11): köşe noktaları parsellerin köşelerindedir, adlar biçimden gelir (`P` + `00001`), ortak köşe ve kenar bir kez sayılır. Köşe ve kenar yazılarının yeri ve dönüşü çekirdeğin cevabıdır (`crates/shared/geometry-core/src/processing`: `corner_text_at`, `edge_lengths`); doğrulukları çekirdeğin kendi testlerindedir, burada iki platformun aynı yeri vermesi sabitlenir. Yerler denetlenmiştir: kenar yazısı kenarın ortasında, dışa (açık çizgide sola) 0,8 m, dikey kenarda 90°; köşe yazısı köşenin dış açıortayında.

## Kurallar

- Durumlar web'in bugünkü davranışını yazar; web geçmeden durum eklenmez.
- Bir araç değişince durumu, iki çalıştırıcı ve bu belge birlikte değişir. Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
- Yeni bir yerleşik araç ya da model en az bir başarılı durum ve bir ret durumuyla gelir.

## Pencere (`kentos.processing-dialog`, sürüm 1)

`v1/dialog.json`, işlem penceresinin (web: `ui/processing/ToolDialog.ts` ve `paramFields.ts`) ne gösterdiğini ve kullanıcının her işinde nasıl değiştiğini tutar. Kurallar sayfasızdır: `apps/web/src/ui/processing/dialogPlan.ts` (form, bölümler, sorunlar, alt satır, yerler, durum), `fieldPlan.ts` (alanlar) ve `dialogTexts.ts` (sözler); pencere onlardan çizer.

- **Web**: `apps/web/src/ui/processing/dialogFixture.test.ts` oynatır (oynatıcı `dialogFixture.ts`). Kaydedici `apps/web/scripts/fixtures/record-processing-dialog.test.ts`, yalnız bilerek: `GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-processing-dialog.test.ts`. Kaydedilen cevaplar okunmuştur; yeniden yazmak farkı okunacak bilinçli bir değişikliktir.
- **Masaüstü**: araç penceresi (ADR 0084) aynı dosyayı oynatır.

| Alan | Anlamı |
|---|---|
| `document` | Oturumların çizimi (`parcels.kcad`) |
| `texts` | Pencerenin metinleri; bir değerden yapılan metin `{ sample, text }` (ör. `fixFields`: 2 → "Çalıştırmadan önce 2 alanı düzeltin.") |
| `scopes` | Kapsam düğmelerinin adları (Seçili, Görünen, Tümü, Katman) |
| `targetLabels`, `targetShort` | Web'in yer adları ("Bu tarayıcıda", "arka planda"). Masaüstü yerleri kendi sözüyle adlandırır (ADR 0084: "Bu bilgisayarda", "Arka planda"); seçeneklerin `value`, `note`, `disabled` ve `checked`'i ortaktır |
| `forms` | Her yerleşik aracın ve modelin formu, pencerenin açıldığı kimlikle (`model:` önekli): `title`; `rows` (her parametre: `name`, `label`, `description`, `optional` "isteğe bağlı", `stacked` denetim etiketin altında mı, `control`); `side` (kategori yolu ve simgesi, aracın simgesi, açıklaması, yardım paragrafları, modelde sırasıyla adımlar ve düzenleme düğmesi, Önizleme bloğu var mı, takma adlar) |
| `sessions` | Oturumlar |
| `status` | Alt satır ve düğmeler: çalıştırmanın durumu, Çalıştır'a basıldı mı, sorunlar → `status`, `footer` |
| `targets` | Nerede çalışır: aracın bildirdiği yerler, bu evde olanlar, Otomatik'in şimdi seçeceği, model mi, saklanan seçim → seçenekler, ipucu, çalıştırmanın alacağı seçim |
| `restore` | Saklanan değerlerin (son çalıştırma) varsayılanların üstüne geri yüklenmesi: değerler, Gelişmiş'in açık başlaması ve doğrulamanın iletileri |
| `kinds` | Tür çipleri: önce, tıklanan türle yeni değer, sonra |
| `tokens` | Alan adının ifadede yazılışı: çıplak (`Ada`) ya da köşeli parantezde (`[Tapu alanı]`, ayrılmış sözcükler Türkçe katlanarak: `[ve]`, `[Değil]`) |
| `inserts` | İmleçte ekleme (alan çipi, değişken, işlev): metin, seçim, eklenen → yeni metin ve imleç. Önündeki metin boşluk, "(" ya da "," ile bitmiyorsa araya boşluk girer |
| `numbers` | Sayı alanının metni → değer: nokta ya da virgül ondalık; okunamayan `null` (web'de NaN; doğrulama "için geçersiz değer." der) |
| `icons` | İfade satırının simgesi: satır " yok." ya da " boş." içeriyorsa `info`, değilse `check` |

Formdaki denetimler (`control.type` parametrenin türüdür): `features` kapsam düğmeleriyle; `number` birimiyle; `string` (`short`: en çok 2 karakter); `boolean` anahtar; `enum` en çok üç seçenek ve her etiket en çok 22 karakterse düğmeler (`segmented`), yoksa açıklamalı liste (`dropdown`); `layer`; `point`; `field` yeni ad yazılabiliyorsa yazı ve liste (`combo`, "Alan adı"), yoksa yalnız liste (`dropdown`, "Alan seçin"); `expression`.

### Oturum

| Alan | Anlamı |
|---|---|
| `id`, `title` | Kimlik ve Türkçe açıklama |
| `open` | `{ "tool": … }` ya da `{ "model": … }` |
| `selection` | Pencere açılmadan önceki seçim |
| `view` | İsteğe bağlı: görünen alan `[minX, minY, maxX, maxY]`, mutlak metre |
| `available` | Bu evin çalıştırabildiği yerler: masaüstü bugün `["client"]`, web `["client", "worker"]` |
| `given` | Pencereye verilen değerler (geçmişin "Yeniden aç"ı); varsa son değerlerin yerine geçer |
| `last` | Aracın son çalıştırmasının değerleri |
| `choice` | Saklanan yer seçimi; yoksa Otomatik |
| `opened` | Açılınca görünen (bütün görünüş) |
| `steps` | Adımlar: `do` (kullanıcının işi) ve `expect` (bir önceki görünüşten değişenler); `pickObjects` adımında ayrıca `picking` ve `after` (aşağıda) |

Kullanıcının işleri (`do`):

| İş | Anlamı |
|---|---|
| `choose` `{ name, value }` | Seçmek: düğme, anahtar, listeden satır. Seçili olanı yeniden seçmek hiçbir şeyi değiştirmez |
| `type` `{ name, text }` | Yazmak: sayı alanı metni sayı okur (`numbers`), hedef katman alanı yeni katmanın adı olarak (`{ newName }`), öbürleri metin olarak. Değer aynı kalsa da alana dokunulmuş sayılır |
| `scope` `{ name, scope }` | Kapsam düğmesi; Katman etkin katmanla başlar; tür süzgeci kalır; seçili kapsama basmak değiştirmez |
| `scopeLayer` `{ name, layerId }` | Katman kapsamının listesinden bir katman |
| `kind` `{ name, kind }` | Tür çipi: tür çıkar ya da girer; kapsamdaki bütün türler yeniden alınınca süzgeç kalkar |
| `toggleAdvanced` | Gelişmiş ayarlar'ı açıp kapamak |
| `target` | Nerede çalışır'da bir seçenek |
| `reset` | Varsayılanlar |
| `run` | Çalıştır; sorun yoksa çalıştırma bitene kadar sürer |
| `undo` | Çalıştırmadan sonra alt satırdaki "Geri al": çizimin son adımı geri alınır |
| `pick` `{ name, point }` | Nokta alanının Sahneden seç'i: gösterilen nokta ya da `null` (vazgeçti). Pencere olduğu gibi döner; nokta bir seçim gibi yazılır |
| `pickChoice` `{ name, point }` | Bir seçeneği çizimdeki nokta olan seçimin (`picks`, ADR 0088) yanındaki Sahneden seç: nokta, nokta parametresine yazılır ve seçim o seçeneğe geçer; ikisine de dokunulmuş sayılır. `null` (Esc) hiçbir şeyi değiştirmez |
| `pickObjects` `{ name, tolerance, actions, end }` | Girdi nesnelerinin Sahneden seç'i (ADR 0088), tek adımda. Pencere kenara çekilir; seçim saklanır ve boşalır. `actions` sırayla: `click` (nokta; `tolerance`, dünya biriminde seçme açıklığı: ekrandaki açıklık bölü görünümün ölçeği) ya da `box` (`from`, `to`; sağdan sola çizilen kesişim, öbürü pencere). `end`: `done` (Enter, Boşluk ya da hızlı sağ tık) ya da `cancel` (Esc). Alınan türler değerin `kinds`'i, yoksa parametreninki. Tıklama: imlecin altındaki en belirli nesne alınan türdense o, değilse alınan türlerden kenarı en yakın olan (erimde yoksa hiçbiri); seçimdeyse çıkar. Kutu: içindekilerden alınan türler eklenir. `done` ve en az bir nesne: alan `{ scope: 'selection' }` olur (tür süzgeci kalır), dokunulmuş sayılır, günlüğe "n nesne seçildi." yazılır. Yoksa (`cancel` ya da hiç nesne): pencere olduğu gibi, önceki seçim geri gelir |

`pickObjects` adımının çizim tarafı:

| Alan | Anlamı |
|---|---|
| `picking` | Seçim (kimlikler, küçükten büyüğe) ve komut satırı (`prompt`: "Alanlar: nesneleri tıklayın ya da pencereyle seçin (n seçili) [Bitti (Enter) / Vazgeç (Esc)]"): seçim başlarken, sonra her tıklama ya da kutudan sonra |
| `after` | Pencere döndüğünde seçim; nesneler alındıysa günlüğün satırı (`said`) |

Görünüş:

| Alan | Anlamı |
|---|---|
| `values` | Değerler, çalıştırmanın alacağı gibi |
| `sections` | `groups`: Girdi (`features`), Ayarlar (öbürleri), Çıktı (`layer`); satırı olanlar, bu sırayla, satırlar parametre adları. `advanced`: Gelişmiş ayarlar'ın satırları ve açık mı (kullanıcı açtıysa ya da satırlarından birinde sorun görünüyorsa açık); görünen gelişmiş parametre yoksa `null` |
| `fields` | Görünen alanların (Gelişmiş'inkiler açıkken) değişen parçaları; sayı, metin, anahtar ve seçim alanları yalnız değerlerini gösterir. Kapsam alanı: seçili düğme (`scope`; önceki adımın çıktısı ilk kapsam görünür), Katman kapsamında katman listesi ve yazısı (olmayan katman "—"), ne okunduğu (`count.text`, boşsa `count.empty` ve uyarı simgesi), tür çipleri (`chips`: tür, ad, sayı, basılı mı, ipucu; kapsamda iki ya da daha çok tür varsa ya da süzgeç varken) ya da aracın uygun türleri notu. Hedef katman: yazısı (seçili katman; yeni adda "(mevcut)" var olan katmanın adıyla, ya da "(yeni)"), yeni katman adı alanı (`name`; mevcut katman seçiliyken `null`), liste (başlıklar, "Yeni: …", katmanlar yollarıyla; kilitliler kapalı ve "kilitli"). Nokta: `text`, `button`, `shown`. Öznitelik alanı: `text`, not (`note`: "n nesnede var; değeri değişir.", "Yeni alan: nesnelere eklenir.", "Bu nesnelerde böyle bir alan yok.", boş ad için boş), liste (alanlar en çok bulunandan; alan yoksa kapalı tek satır). İfade: alan çipleri (ilk 6: ad, yazılışı, ipucu), kalanlar `more` ("+n"), satır (`preview`: simge ve metin; ifade boşken ya da hatalıyken `null`) |
| `issues` | Alanların altındaki sorunlar: dokunulmuş alanınki hemen, Çalıştır'dan sonra bütün alanlarınki (başarılı çalıştırmaya ya da Varsayılanlar'a kadar). Çalıştırıcının çalıştırmadan önce bulduğu (seçim boş, girdinin hepsi kilitli) bir değer değişene ya da yeni çalıştırmaya kadar alanında durur |
| `preview` | Yan paneldeki Önizleme: aracın önizlemesi, bir alanda sorun varken (görünmese de) "Önizleme için alanları düzeltin."; önizlemesi olmayan araçta `null`. `muted`: gösterecek önizleme yok |
| `status` | Alt satır: `kind` (`idle`, `running`, `ok`, `warn`, `error`), `icon`, `text`; çalışırken `progress` (0–100); başarıda `actions`: `zoom` "Seçime yakınlaştır" (araç seçti), `select` "Sonuçları seç" (`pick`: eklenenler, yoksa değişenler ya da seçilenler), `undo` "Geri al" (çizim değişti). Uyarı: Çalıştır'dan sonra aracın kendi kuralı, yoksa "Çalıştırmadan önce n alanı düzeltin.", yoksa çalıştırıcının iletisi |
| `footer` | Çalıştır (çalışırken "Çalışıyor…" ve kapalı), Kapat (çalışırken "Durdur": çalıştırmayı durdurur), Varsayılanlar (çalışırken kapalı) |
| `targets` | `options` (`value`, `label`, `note`, `disabled`, `checked`), `hint` (Otomatik seçiliyken ipucu), `choice` (çalıştırmanın alacağı seçim). Birden çok yer varsa önce Otomatik ("şimdi: …"; modelde "adım adım"); sonra aracın bildirdiği her yer: bu evde varsa seçenek (tek yerse "bu çalıştırmada" notuyla, işaretli), yoksa "yakında" ve kapalı. Modelde bildirilen yerler adımlarının bu evdeki yerleridir. Saklanan seçim bu evde yoksa Otomatik (tek yerde o yer) |

`expect` yalnız değişenleri yazar: bir parça değiştiyse bütünü; `values` ve `fields` ad ad, artık görünmeyen alan `null`. Beklenen görünüş, bir öncekinin üstüne bunlar konarak bulunur.

Pencerenin öbür kuralları (görünüşte yok):

- **Açılış:** verilen değerler, yoksa son değerler, varsayılanların üstüne; uymayan değer varsayılana döner (`restore`). Gelişmiş ayarlar, görünen bir gelişmiş değer varsayılanından farklıysa açık başlar.
- **Klavye:** metin alanında Enter, her yerde Ctrl+Enter çalıştırır. Açılışta ilk metin ya da sayı alanı (yoksa seçili düğme) odaklanır; başarısız denemeden sonra ilk sorunlu alan, çalıştırmadan sonra Çalıştır.
- **Sonuçları seç** seçimi `pick` yapar, pencereyi kapatır ve seçime yakınlaştırır; **Seçime yakınlaştır** seçime dokunmadan aynısını yapar.
- **Durdur** çalıştırmayı durdurur; iptal edilen çalıştırmanın satırı `error`'dur ("İşlem iptal edildi; çizim değişmedi.").
- Oturumların çalıştırmaları `cases.json`'daki gibi gerçek çalıştırmalardır; özetleri ve seçtikleri iki platformda aynı çalıştırıcıdan gelir.
- İfade satırlarının metinleri ve ifade hataları dilin çekirdeğindendir (`model/expression`); dil değişince bu dosya kaydediciyle yeniden yazılır ve farkı okunur.
