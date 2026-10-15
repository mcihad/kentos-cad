# ADR 0297: Esnet'te yalnız penceredeki köşeler, yayın kiriş yüksekliği ve izleyenlerin önizlemesi

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-11`: “Stretch seçim penceresinin yalnız
  kapsadığı köşeleri düzenlesin; bağlı ölçü, açıklık, tarama ve dizi etkileri önizlemede görülsün.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0047 (Esnet aracı), 0160 §3 (topolojik düzenleme), 0175 §4 (bağlı yazı), 0186 §6 (ilişkili tarama), 0249
  ve 0263 (ilişkili ölçü), 0254 §2 (anlamsal `cad.entities.stretch`), 0296 (taşınanlar ve ilişki politikası).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

`cad.entities.stretch` (ADR 0254 §2) köşeleri çekirdeğin `stretch_entity`'siyle taşıyordu. Üç eksik vardı:

1. **Yay:** yalnız ortası pencerede olan yay da değişiyordu (orta nokta pencereye girince tümüyle taşınıyor, yay şişiyordu).
   Bir ucu taşınan yayın ortası yer değiştirmenin yarısı kadar kayıyordu; kiriş dönünce yayın yüksekliği değişiyordu. AutoCAD'in
   STRETCH'inde yayın uçları esner, kiriş yüksekliği sabit kalır.
2. **Başka nesneyi izleyen nesne kendisi de esniyordu:** pencerede köşesi olan ilişkili tarama, bağlı yazı ve ölçünün bağlı noktası
   kendi köşeleriyle taşınıyordu. Belgenin adım sonu izlemesi (ADR 0175, 0186, 0249) bu yüzden onları ya ilişkisinden koparıyor (nesnesi
   değişmeyen, kendisi değişen) ya da kendi yerleşimiyle yeniden yazıyordu. Sonuç kullanıcının penceresine değil, sıraya bağlıydı.
3. **Önizleme izleyenleri göstermiyordu:** plan yalnız esneyen nesneleri ve topoloji komşularını veriyordu. Bağlı ölçünün ve
   taramanın nereye gideceği ancak yazınca görülüyordu; Python ve ajan da planda göremiyordu.

AutoCAD'le karşılaştırma:

| AutoCAD STRETCH | Önce | Şimdi |
|---|---|---|
| Kesişen pencere; penceredeki köşeler taşınır | var | aynı |
| Yay: uçları esner, kiriş yüksekliği sabit | uç esner, orta yarı yarıya kayar | kiriş yüksekliği ve yanı sabit |
| Yalnız ortası ya da merkezi pencerede olan yay değişmez | ortası pencerede yay şişiyordu | değişmez |
| Daire, elips, yazı, blok: merkezi ya da yeri pencerede olan taşınır | var | aynı; pencere nesneye değmeli (kesişen seçim) |
| Çoklu çizginin yay kenarı bükümünü korur | var | aynı |
| İlişkili ölçü geometrisini izler | adımda izliyordu, önizlemede yok | izler; önizlemede ve planda |
| İlişkili tarama sınırını izler | tarama kendi köşesiyle de esnerdi | yalnız sınırını izler; önizlemede |
| İlişkili dizi, mimari açıklık | KentOS'ta yok | “Kalanlar” |

## Karar

### 1. Yayın köşeleri uçlarıdır

Çekirdeğin `ops::stretch::stretch_entity`'si (masaüstü, başsız sunucu ve web'in WASM'ı tek kaynaktan):

- Uçlarından hiçbiri pencerede değilse yay değişmez (`None`): ortası ya da merkezi pencerede olsa da.
- İki ucu da penceredeyse yay olduğu gibi taşınır.
- Bir ucu penceredeyse o uç taşınır; yayın kiriş yüksekliği (kirişin ortasından yayın ortasına uzaklık) ve kirişin hangi yanında
  olduğu korunur (`with_height`). Yay, yeni uçlardan ve yeni ortadan geçen yaydır.
- Yeni kirişin boyu sıfırsa (iki uç üst üste gelirse) yay değişmez.

Çoklu çizginin yay kenarı bükümünü korur (değişmedi). Daire, elips, yazı, blok, tablo ve resim yerleri penceredeyse bütünüyle taşınır;
pencere kesişen seçimdir: dairenin içinde kalıp çevresine değmeyen pencere onu seçmez (AutoCAD'deki gibi).

### 2. İzleyen nesne kendisi esnemez

Başka nesnelere bağlı olanlar bağlı oldukları nesneyle izler; kendi köşeleriyle taşınmaz:

| Nesne | İzlediği | Esnet'te |
|---|---|---|
| İlişkili tarama (`assoc`, kapalı nesnesi çizimde) | kapalı nesnesi, adaları, boşlukları | kendisi esnemez; nesneleri değişince adım sonunda bölgesi yeniden kesilir |
| Bağlı yazı (`labelOf`, nesnesi çizimde) | nesnesinin etiket yeri | kendisi esnemez; nesnesi değişince yeniden yerleşir |
| Ölçünün bağlı noktası (canlı başvuru, nesnesi çizimde) | nesnesindeki yeri | nokta yerinde kalır; nesnesi değişince yeri izler |
| Ölçünün serbest noktası | — | penceredeyse taşınır |

- Pencerede yalnız bağlı noktaları olan ölçü esnemez. Serbest noktası penceredeyse ölçü esner; bağlı noktaları eski yerlerinde kalır,
  sonra nesnelerini izler.
- Pencereden bulunan nesnelerde izleyenler sessizce dışarıda kalır. `uids`'te verilen izleyen nesne yeni `follows_object` uyarısıyla
  (“Nesne bağlı olduğu nesneyi izler; kendisi esnemedi.”, yolu `uids[i]`) atlanır. Hiçbir nesne kalmazsa ret yine
  `nothing_to_stretch`'tir.
- Kapalı nesnesi silinmiş ilişkili tarama ve nesnesi silinmiş bağlı yazı bağlı sayılmaz; kendi köşeleriyle esner. Kırık başvurulu ölçü
  noktası serbesttir.

### 3. Plan ve sonuç izleyenleri verir

- `EntitiesStretchPlan`'e `dependents` eklendi: adım sonunda izleyecek nesneler (ilişkili ölçüler, ilişkili taramalar, bağlı yazılar),
  olacakları hâlleriyle, çizimin sırasıyla. `EntitiesStretched`'e `dependents` eklendi: izleyenlerin kimlikleri. Boşsa alan yazılmaz;
  eski cevaplar ve ortak durumlar değişmedi.
- Esneyen ya da topoloji komşusu olan bir nesne de izliyorsa (serbest ve bağlı noktalı ölçü), listedeki hâli izledikten sonraki
  hâlidir; `dependents`'ta yinelenmez.
- Cevabı belgenin kendi kuralı verir: `Document::followers(changes)` değişiklikleri bir kopyaya uygular, adım sonundaki izlemeyi
  (`follow_into`: bağlı yazılar, ilişkili taramalar, ilişkili ölçüler) çalıştırır, izleyenleri okur ve kopyayı geri alır. Hiçbir şey
  yazılmaz. Çizimde bağ yoksa (`can_follow`) ya da değişen nesnelere hiçbir şey bağlı değilse kopya yapılmaz.
- Araç her harekette plan ister. Kopyayı `followers_with` ile araç tutar; çizim değişmedikçe (`generation`, `edits` aynı) aynı kopya
  kullanılır ve her plandan sonra eski hâline döner. `stretch::plan_with` bu kopyayı alır, `plan_in` her seferinde yenisini kurar.
- Yürütme izleyenlerin kimliklerini yazmadan önce aynı kuralla bulur, yalnız çizimde bağ varsa (`can_follow`).

### 4. Araç

Esnet aracı (`kentos_interaction::stretch`):

- Esneyen nesneler önceki gibi kesikli vurgu renginde; izleyenler planın `dependents`'ından, kenet renginde ince noktalı çizilir.
- Yazınca “n bağlı nesne (ölçü, tarama ya da yazı) nesnesini izledi.” der.
- Kopya araçta tutulur, yazınca bırakılır.

Yeni ikon yok.

### 5. Performans

- Bağ olmayan çizimde ek iş yoktur: `may_follow` yalnız dizinlere bakar.
- Bağ olan çizimde ilk önizleme kopyayı kurar; sonraki her hareket yalnız değişen nesneleri uygular, izler ve geri alır.
- Bütçe: 100 000 nesnelik çizimde tutulan kopyayla bir planın izleyenleri 4 ms içinde.

## Uygulama

- **Çekirdek:** `crates/shared/geometry-core/src/ops/stretch.rs` (`with_height`, yayın uçları).
- **Belge:** `crates/native/domain/src/followers.rs` (`Document::followers`, `followers_with`, `can_follow`); `edit::changed`
  crate içine açıldı.
- **Sözleşme:** `cad_stretch.rs`'in `dependents`'ları; `codes.rs`'te `FOLLOWS_OBJECT`; yetenekler ve katalog açıklaması.
- **Komut:** `crates/native/application/src/stretch.rs` (`Ties`, `Moved`, `carried`, `plan_with`).
- **Araç:** `crates/native/interaction/src/stretch.rs` (izleyenlerin önizlemesi, kopya).
- **Python:** `kentos.cad`'in türleri yeniden üretildi. MCP aracı katalogdan gelir.
- **Web'in kayıtlı çağrısı:** `fixtures/geometry/v1/calls-p5-entities.json`'ın “yayın bir ucunu esnetmek” beklentisi yeni kurala göre
  bağımsız başvurudan yeniden hesaplandı (web dururken koşmaz).
- **Resimler:** `apps/desktop/src/stretch_scenes.rs` (`esnet-izleyenler`, `esnet-yazildi`, `esnet-yay`).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/stretch_policy_cases.py` KentOS kodu olmadan yazıldı. Ortak durumlar
  `fixtures/commands/v1/cad.entities.stretch.policy.json`'da (26 durum):
  - yayın ortası, merkezi, bir ucu (iki yön), iki ucu, çeyrek yay, seçilen yayın dışarıda kalması;
  - çoklu çizginin yay kenarı;
  - daire, elips, yazı ve kesişen pencere;
  - bağlı ölçünün izlemesi, geri alınması ve `follows_object`;
  - ölçünün serbest ve bağlı noktaları;
  - ilişkili ve ilişkisiz tarama, sıfır yer değiştirme, seçimdeki uyarılar.

  Yaylar plan adımında 1e-9 içinde, öbürleri bire bir karşılaştırılır. Masaüstü (`fixtures.rs`) ve Python aynı dosyayı çalıştırır.
- **Eski durumlar:** `cad.entities.stretch.json`'ın 20 durumu değişmeden geçer. Yetenek testi `follows_object`'i yeni durumlarda görür.
- **Kalıcı kimlikli testler:** `crates/native/application/tests/all/stretch_ties.rs`.
  - Bağlı yazı nesnesini izler; plan yazının olacak hâlini verir, yazılan plandır, tek geri alma.
  - Seçilen bağlı yazı ve tarama için `follows_object`.
  - İlişkili tarama alanını izler ve bağlı kalır.
  - Tutulan kopya taze kopyayla aynı cevabı verir, çizim değişince yenilenir.
  - Bağsız çizimde kopya yapılmaz.
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
- **Süreler:** release, 100 000 çizgilik çizim; komutu `stretch_ties::timing`:

  | İş | Süre |
  |---|---|
  | Planın izleyenleri, her planda yeni kopya | 7,07 ms |
  | Planın izleyenleri, tutulan kopya | 0,01 ms (bütçe 4 ms) |
  | Bağsız nesnelerin planı | 0,00 ms (kopya yok) |

## Yolda düzelenler

- ADR 0292 (aynada okunur yazı) DXF yazıcısının aynalı yerleştirmedeki öznitelik yazısını değiştirmişti, `blocks.dxf` örneği ve
  bağımsız `dxf_write_reference.py` eski kuralda kalmıştı (`rust:test`'te `the_blocks_fixture_is_written_to_its_committed_bytes`
  düşüyordu). Başvuru artık yazının dönüşünü ve hizasını `affine_reference.text_frame` ile (ADR 0292'nin kuralı, KentOS kodu olmadan)
  bulur; `blocks.dxf` yeniden yazıldı ve başvurudan geçer.
- `break_join_ties.rs`'te gereksiz `clone` (clippy).
- `break_join_policy_cases.py --check` bu makinenin Python'unda yayların bükümünde son basamakta ayrışıyor (`tan(π/8)`); dosyalar
  değiştirilmedi, durumlar 1e-9 içinde karşılaştırıldığı için masaüstü testleri geçer.

## Kalanlar

- **İlişkili dizi** (`CAD-EDIT-14`) ve **mimari açıklık** (plan §14) KentOS'ta henüz yok. Bugünkü diziler bağımsız kopyalardır; her kopya
  kendi köşeleriyle esner. Bu nesneler geldiğinde izlemeleri belgenin adım sonu kuralına eklenir ve `followers` ile `dependents` onları
  kendiliğinden gösterir.
- Önizleme izleyenlerin çizgilerini gösterir, ölçünün yazısını göstermez.
- Elips yayının uçları esnemez; elips yalnız merkeziyle taşınır.
