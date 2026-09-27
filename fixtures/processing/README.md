# İşlem araçlarının ortak durumları

İşlem araçlarının (İşlemler, [docs/PROCESSING.md](../../docs/PROCESSING.md)) iki platformda aynı sonucu vermesi için ortak durumlar. Her durum bir çizimde yerleşik bir aracı ya da modeli çalıştırır ve çalıştırmanın ne yaptığını platformdan bağımsız olarak söyler: yeni katmanlar, eklenen, değişen ve silinen nesneler, seçim, özet, iletiler, geri alma adımı ya da ret iletileri.

- **Web**: `apps/web/src/processing/cases.test.ts` (Vitest). Her durum `ProcessingRunner` ile sayfada, sonra işçinin yolundan (`handleJob`) bir kez daha çalışır; ikisi de beklenenle karşılaştırılır.
- **Masaüstü**: `crates/native/processing` (`kentos-processing`) aynı dosyayı oynatır.

Çalıştırmalar ürün komutu değildir: katalogda kaydı ve `CommandResult`'ı yoktur, belgenin işlemiyle tek geri alma adımında yazar. Python ve yapay zekâ için ileride `cad.processing.run` onları saracak (CLAUDE.md §18).

| Dosya | İçerik |
|---|---|
| `v1/parcels.kcad` | Durumların çizimi (`.kcad` v1): Parsel katmanında yan yana üç parsel (1: 0…20, 2: 20…45, 3: 45…60 doğu; 0…30 kuzey; 1 ile 2 x = 20'yi, 2 ile 3 x = 45'i paylaşır; Ada, Parsel, Nitelik öznitelikleri, etiketleri parsel numarası), kilitli katmanda parsel 4, gizli katmanda parsel 5, Çizim'de çoklu çizgi 6 (30 m ve 15 m) ve 20 m'lik çizgi 7, Mevcut noktalar katmanında parsel 1'in üç köşesinde P00001–P00003 (8–10). Koordinatlar (487000, 4420000)'e göre verilmiştir |
| `v1/cases.json` | Durumlar |

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
