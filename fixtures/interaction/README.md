# Etkileşim izleri

TODOS.md §5 (`UX-01`, `UX-04`, `UX-06`) ve [ADR 0018](../../docs/adr/0018-tool-session-and-input.md).

Bir iz, kullanıcının çizim alanında yaptıklarını adım adım yazar: komut seçmek, tıklamak, yazmak, tuşa basmak. Her adımdan sonra görülmesi gerekeni de platformdan bağımsız olarak söyler.

- **Web** izleri bugün gerçek tarayıcıda oynatır: `pnpm e2e:interaction` (`make e2e-interaction`). Başsız Chrome'a gerçek fare ve klavye olayları gönderilir.
- **Masaüstü** aynı dosyaları değiştirmeden, pencere açmadan oynatır: `cargo test -p kentos-desktop traces` (`apps/desktop/src/traces.rs`, [ADR 0021](../../docs/adr/0021-native-tool-session.md)). Tuşlar ve fare, uygulamanın kendi aboneliğinden ve çizim alanının kendi hareket kodundan geçen Iced olaylarıdır. `kentos-cad snapshot çıktı.png --iz <iz> --adim <n>` bir izi görüntüye oynatır.

İz, iki uygulamanın kullanım davranışının ortak referansıdır.

| Dosya | İçerik |
|---|---|
| `v1/polygon-accept.json` | §5 kabul izi: poligon başlat, tıkla, `12` yaz, Enter, sonraki nokta, kapat, geri al, yinele, kaydet ve aç |
| `v1/polygon-signs.json` | Değer yazmaya `-` ya da `+` ile başlamak (`UX-04`) |
| `v1/polygon-keys.json` | Esc, Geri (G), Ctrl+Z, sağ tık, çift tık, eksik nokta, Backspace, Tab, Boşluk, son komutu yinele, odak (`UX-06`) |
| `v1/polygon-close.json` | İlk köşeye dönmek alanı kapatır: tıklama, yakınına tıklama, yazma, üç köşeden az, yayla kapatma |
| `v1/empty.kcad` | İzlerin başladığı boş çizim (`.kcad` v1) |

## Biçim (`kentos.interaction-trace`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.interaction-trace"`, `1` |
| `id`, `title` | Kimlik (dosya adıyla aynı) ve Türkçe açıklama |
| `source`, `covers` | Kaynak ve kapsanan TODOS maddeleri |
| `document` | İlk adımdan önce açılan çizim. Açılınca geri alma geçmişi boştur, çizim kirli değildir |
| `view` | Çizim alanının merkezi `[doğu, kuzey]` ve ölçeği `metresPerPixel` |
| `draft` | Kenet (`snap`), ızgara, ortho, kutupsal izleme ve kenet izlemesi; açık ya da kapalı |
| `prefs` | Kullanıcı tercihleri; bugün yalnız `cursorInput` (imleç yanında değer girişi) |
| `clickTolerance` | Tıklanan noktaların karşılaştırılacağı mesafe, metre |
| `steps` | Adımlar |

Adımlardaki koordinatlar, `view.center`'a göre doğu ve kuzey farklarıdır, metre cinsinden.

**Eylemler.** Her adımda en çok bir eylem bulunur:

| Eylem | Anlamı |
|---|---|
| `run` | Komutu kimliğiyle çalıştırır; şeritten, menüden ya da komut satırından seçmekle aynıdır (`tool.polygon`) |
| `key` | Tek tuş: `Enter`, `Esc`, `Tab`, `Backspace`, `Space`, bir harf (`G`), `-`, `+` ya da `Ctrl+` akoru (`Ctrl+Z`) |
| `text` | Karakterler tek tek yazılır. Klavyenin ürettiği metin sayılır, fiziksel tuş konumu değil |
| `move` | İmleç çizimde bu noktaya gelir |
| `click`, `doubleClick` | Sol tuşla tıklama ya da çift tıklama |
| `rightClick` | Sağ tuşa kısa basıp bırakma; menüyü açan basılı tutmadan kısa |
| `focus` | Klavye odağı: `commandLine` (komut satırına tıklamak) |
| `saveAndReopen` | Uygulamanın kendi kaydetme komutuyla yeni bir dosyaya yazar ve o dosyayı yeniden açar |

**Beklentiler.** `expect` isteğe bağlıdır ve eylemden sonra denetlenir. Yalnız yazılan alanlar karşılaştırılır:

| Alan | Anlamı |
|---|---|
| `tool` | Etkin araç; komut yokken `select` |
| `points` | Çalışan komutun aldığı nokta sayısı |
| `options` | İstemdeki seçenek tuşları, sırasıyla (`["Y", "U", "G", "Enter"]`) |
| `dynamicInput` | İmleç yanındaki değer alanının metni; kapalıysa `null` |
| `commandLine` | Komut satırının metni |
| `entities` | Çizimdeki nesne sayısı |
| `newest` | En son oluşturulan nesne: `kind`, köşeler `points`, ardışık köşe farkları `edges`, yaylı kenar sayısı `arcs` |
| `canUndo`, `canRedo`, `dirty` | Geri al, yinele ve kaydedilmemiş değişiklik |
| `log` | Son iletinin düzeyi: `success`, `info`, `warn`, `error` |
| `metresPerPixel` | Görünümün ölçeği |

`note`, adımın neyi gösterdiğini okura anlatır; denetlenmez.

## Karşılaştırma kuralları

- **Tıklanan nokta** ekran pikselinden gelir. `clickTolerance` içinde karşılaştırılır.
- **Yazılan değer** kesindir. `edges` her köşeden sonrakine olan farktır ve tam eşit olmalıdır. Tıklanan ilk noktadan aynı piksel satırında `12` yazmak, tam `[12, 0]` verir.
- **`metresPerPixel`** göreli `1e-9` ile karşılaştırılır.
- **Noktalar çizim alanında kalır.** Merkezden doğuya ve batıya en çok 240, kuzeye ve güneye en çok 160 piksel uzakta olurlar. `0,125` m/piksel ölçekte bu ±30 × ±20 m eder. En küçük desteklenen pencere (1100×600) bu kutuyu çizim alanında gösterir. Oynatıcı alanın dışına düşen noktayı sessizce kaçırmaz; izi hatayla durdurur.

## Kurallar

- İz, web'in bugünkü davranışını yazar; web geçmeden iz eklenmez.
- Davranış değişecekse sıra şudur:
  1. karar (ADR 0018);
  2. iki uygulamada değişiklik;
  3. izin güncellenmesi.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
- Yeni bir iz ya da alan eklenince bu belge ve iki oynatıcı birlikte güncellenir: web (`apps/web/scripts/e2e/interaction.mjs`) ve masaüstü (`apps/desktop/src/traces.rs`). Masaüstü oynatıcısı bilmediği alanda durur.
- Yazılan değerin dilbilgisi ayrı bir dosyadadır: `fixtures/point-input/v1/cases.json`. Web'in ve masaüstünün okuyucusu onu okur.

## Varyantlar

Web oynatıcısı her izi üç varyantta oynatır. Masaüstü de aynısını yapar.

| Varyant | Anlamı |
|---|---|
| `us` | US klavye, 1× ekran |
| `tr-q` | Türkçe Q klavye. `+` Shift+4'le, `-` `*`'ın sağındaki tuşla, `@` AltGr+Q ile yazılır; AltGr Windows'taki gibi Ctrl+Alt olarak gelir |
| `hidpi` | US klavye, 2× ekran (HiDPI) |

Bir varyantı seçmek için: `pnpm e2e:interaction -- --variant=tr-q`.

**Odak başka bir metin alanındayken** yazma durumu `polygon-keys`'te.

§5 kabul izinin istediği şu varyantlar henüz yok:
- Türkçe F klavye;
- IME açıkken yazma.
