# ADR 1000: İnceleme notları ve seçilen revizyon farklarını alma

- **Durum:** kabul edildi (2026-11-05). Kapsam: plan/cad-advanced.md `CAD-ADV-15`'in açık iki işi — “Revizyon
  bulutu/notu ve inceleme kaydı nesne/alt nesne, görünüş ve kaynak revizyona bağlanabilsin. Açık/yanıtlandı/çözüldü
  durumu, açıklama ve değişen kaynak nedeniyle eskime görülsün” ve “Seçilmiş farkları alma, aynı nesnenin iki tarafta
  değişmesi ve yetkisiz kapsam için mevcut revizyon/çatışma kuralları kullanılsın”. Sahibin sözü (5 Kasım 2026):
  “plan/cad-advanced.md maddelerini tamamlamanı istiyorum”. Madde tek parçada biter.
- **Bağlam belgesi:** [revizyon karşılaştırması](../cad/revision-comparison.md) (`cad.drawing.compare`, UUID eşleme,
  `blockDefinition`/`layerStyle`/`dimensionPresentation`/`hatchAssociation`, `projectChanges`, aday eşleme);
  ADR 0576 §3 (iki revizyon yan yana), ADR 0637 (izlenen işaretler), ADR 0721 §4 (`cad.revisions.mark`),
  ADR 0253 (komut kaydı, nesne damgaları), ADR 0047 (kilitli katman: hepsi ya da hiçbiri).

## 1. Araştırma

| Kaynak | Bilgi | KentOS kararı |
| --- | --- | --- |
| [AutoCAD LT 2024 — Markup Set Manager](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-LT/files/GUID-EA117B76-44DE-4D83-A75C-EEA07765901C.htm) | İşaretlemenin durumu `<None>`, Question, For Review, Done; durum ve yorum işaretleme dosyasına kaydedilince kalıcıdır. | Durum tasarım nesnesinden ayrı bir kayıttır ve çizimle birlikte kaydedilir. |
| [AutoCAD LT — About Responding to Markups](https://help.autodesk.com/cloudhelp/2018/ENU/AutoCAD-LT/files/GUID-E18C89E4-877B-477D-9828-BC24BD15EB74.htm) | Yanıt, işaretlemenin geçmişine eklenir; durum ayrıca değiştirilir. | Yanıt durumu kendiliğinden değiştirmez; ikisi açık ayrı girdidir. |
| [Bluebeam Revu — özel durumlar](https://support.bluebeam.com/articles/revu-save-and-share-custom-statuses) | Review modeli: Accepted, Rejected, Cancelled, Completed; durum PDF'de ya da profilde tutulur. | Plan metnindeki üç durum (Açık, Yanıtlandı, Çözüldü) yeterlidir; özel durum modeli bu maddenin dışında. |
| [AutoCAD 2027 — Trace][R09], [DWG Compare Limitations][R10] | Yorum ana çizimden ayrı katmanda durur; karşılaştırma bütün türleri aynı düzeyde desteklemez. | Not bir çizim nesnesi değildir: çizilmez, basılmaz, seçim ve ölçü hesabına girmez. |

[R09]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Platform/files/GUID-5D5E0BA1-10E7-4662-8118-E046EC579B08.htm
[R10]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-3D918CC7-4FB7-4951-8F04-C2434DFD04A2.htm

Bu ürünlerin kapalı motorlarının iç işleyişi bilinmiyor; yalnız belgelerdeki kullanıcı davranışı alındı.

## 2. İnceleme notu modeli (`ReviewNote`, `.kcad` şema 180)

Notlar projenin ayarlarında sıralı bir listedir (`settings.reviewNotes`). Bir not:

| Alan | Kural |
| --- | --- |
| `id` | Küçük harfli, tireli UUID; çizimde tek. Verilmezse komut üretir. |
| `number` | 1'den başlar; yeni not çizimdeki notların en büyük numarasının bir fazlasını alır. Aradaki silinen notun numarası boş kalır; en büyük numaralı not silinirse o numara yeniden verilebilir (numara kimlik değildir, kimlik `id`'dir). |
| `text` | Kenarı boşluksuz, boş değil, en çok 4000 karakter; `\n` ve `\t` dışında denetim karakteri yok. |
| `status` | `open` (Açık), `answered` (Yanıtlandı), `resolved` (Çözüldü). Yeni not `open`. |
| `origin` | `user`, `pdf`, `ai`. Notun kaynağıdır; PDF ya da AI notu da yalnız nottur, çizimi değiştirmez. Varsayılan `user`. |
| `author`, `createdAt` | İsteğe bağlı, en çok 120 / 64 karakter; komut saati kendisi koymaz (ortak durumlar belirlenimli kalır). Masaüstü kullanıcının adını ve yerel ISO zamanını verir. |
| `revision` | İsteğe bağlı kaynak revizyon etiketi (“B”, “2026-11-05”), en çok 64 karakter. |
| `at` | İsteğe bağlı model noktası (dünya koordinatı, sonlu). |
| `sheet`, `view` | İsteğe bağlı pafta kimliği ve adlı görünüş kimliği; verildiyse çizimde bulunmalı. |
| `anchors` | En çok 1000 nesne bağı: `uid`, bağlandığı andaki nesne damgası `stamp` (ADR 0253 §2), blok yerleştirmesinde tanımın ve iç içe tanımların birleşik damgası `blockStamp`. Aynı nesne iki kez bağlanamaz. |
| `replies` | En çok 200 yanıt: `text` (aynı metin kuralı), `author`, `createdAt`. |

Bir çizim en çok 10 000 not taşır. Dosya okuyucusu bu kuralların yapısal olanlarını (kimlik biçimi ve tekliği, metin
sınırları, sayı sınırları, durum) denetler; bozuk ya da yinelenen kimlikli liste reddedilir, boş liste yazılmaz.

**Kilitli katman:** not nesneyi değiştirmez; kilitli ya da gizli katmandaki nesneye bağlanabilir. Bu, ADR 0047'nin
“kilitliye yazma yok” kuralının ihlali değildir.

**Eskime:** bir bağın durumu `current` (damga aynı), `changed` (nesnenin damgası farklı), `blockChanged` (nesne aynı,
blok tanımının birleşik damgası farklı) ya da `removed` (nesne yok). Bağlardan biri `current` değilse, ya da verilen
pafta/görünüş artık yoksa (`sheetMissing`, `viewMissing`) not eskimiştir. Eskime saklanmaz; her sorguda hesaplanır.
Undo nesneyi eski haline getirince not kendiliğinden yeniden güncel görünür.

## 3. `cad.review.notes.edit` v1

Tek işlem; bir Undo adımı “İnceleme notu”. Girdi `{ change: … }`:

- `add`: `id?`, `text`, `anchors?` (uid listesi), `at?`, `sheet?`, `view?`, `revision?`, `origin?`, `author?`,
  `createdAt?`. Damgalar komutta çizimin o anki halinden alınır.
- `update`: `id`, `text?`, `status?`, `revision?` — en az biri verilmeli.
- `reply`: `id`, `text`, `author?`, `createdAt?`, `status?` (verilirse yanıtla birlikte değişir).
- `rebind`: `id`, `anchors?` — verilen bağlar (verilmezse bütün bağlar) çizimin şimdiki damgalarıyla yenilenir:
  “değişikliği gördüm, not yeni geometriye bağlı kalsın”. `anchors` verilince notun bağ listesi tam olarak bu liste
  olur (eklenen ve çıkarılan bağlar açıktır). Yok olan nesne yeniden bağlanamaz.
- `remove`: `id`.

Denetim sırası (ilk başarısız olan cevaptır): metin (`invalid_review_note`, yol `change.text`), kimlik biçimi
(`invalid_uid`, `change.id`), not var mı (`review_note_not_found`) / yinelenen kimlik (`duplicate_review_note`), güncellemede
boş değişiklik (`invalid_review_note`, `change`), yazar/tarih/revizyon (`invalid_review_note`, `change.author` …), nokta sonlu
(`not_finite`, `change.at`), bağ sayısı ve tekliği (`invalid_review_anchors`, `change.anchors`), bağın UUID biçimi (`invalid_uid`,
`change.anchors[i]`), bağlı nesne var mı (`entity_not_found`, `change.anchors[i]`), pafta (`unknown_sheet`, `change.sheet`), görünüş
(`unknown_view`, `change.view`), yanıt sayısı (`too_many_review_replies`), not sayısı (`too_many_review_notes`), en son
beklenen revizyon (`revision_conflict`). Doğrulama ve plan hiçbir şey yazmaz. Çıktı: `id`, `number`, `notes` (toplam), `revision`.

## 4. `cad.review.notes.list` v1 (sorgu)

Girdi: `status?` (durum listesi), `staleOnly?`. Çıktı: her not alanlarıyla, `stale`, `anchorStates`
(`uid`, `state`), `sheetMissing`, `viewMissing`; sayaçlar `open`, `answered`, `resolved`, `stale` (süzgeçten önce,
bütün notlar); `revision`. Notlar numara sırasındadır. Python `kentos.cad.review.notes_list`, MCP aynı komut.

## 5. `cad.drawing.take` v1 — seçilen farkları al

Karşılaştırılan sürümden (`against`, `cad.drawing.compare`'ınki: geçmiş adım ya da dosya) seçilen nesneleri açık
çizime alır. Her `uid` için:

| Eski sürümde | Açık çizimde | Sonuç |
| --- | --- | --- |
| var | yok | Aynı kalıcı kimlikle yeniden eklenir (`added`). |
| yok | var | Silinir (`removed`). |
| var, farklı | var | Eski haliyle değiştirilir; yuva ve kimlik korunur (`replaced`). |
| var, aynı | var | Dokunulmaz (`unchanged`). |
| yok | yok | `entity_not_found` ile bütün işlem reddedilir. |

“Aynı”, nesnenin damga verisinin (`entity_value`) eşitliğidir; tolerans yoktur, çünkü alma işlemi veriyi bayt
anlamında taşır. Kurallar, sırasıyla:

1. `uids` 1–10 000 öğe (`invalid_input`, `uids`), her biri UUID (`invalid_uid`, `uids[i]`), yinelenmesiz (`invalid_input`, `uids[i]`).
2. Sürüm okunamazsa `cad.drawing.compare`'ın retleri (`no_such_step`, `invalid_file_path`, `file_not_found`,
   `import_unreadable`, `against.*`).
3. Eklenen/değiştirilen nesnenin katmanı açık çizimde yoksa `layer_not_found`; eklenen bloğun tanımı yoksa
   `unknown_block` (tanım alma bu komutun işi değildir; blok kitaplığı ya da tanım düzenleme ile alınır).
4. Değişen ya da silinen nesnenin açık çizimdeki katmanı veya yazılacak katman kilitliyse `layer_locked`:
   hepsi reddedilir (ADR 0047).
5. `base` verilmişse üç yollu denetim: nesne hem açık çizimde hem alınacak sürümde ortak tabandan farklılaşmış ve iki
   taraf birbirinden farklıysa `take_conflict` (`uids[i]`); `overwrite: true` açıkça verilmedikçe hiçbiri yazılmaz.
   Açık çizim tabanla aynıysa ya da alınan sürüm tabanla aynıysa çatışma yoktur.
6. Yazım tek Undo adımı “Farkları al”; bağlı ölçü/tarama/yazı izleyicileri işlem sonunda bir kez çalışır ve çizim
   bütünlüğü (`call::recorded`) denetlenir.

Uyarı: blok yerleştirmesi alındığında iki sürümdeki tanımın birleşik damgası farklıysa `block_definition_differs`
(tanım açık çizimdeki olarak kalır; yerleştirmenin görüntüsü eski sürümdekiyle aynı olmayabilir). Katman görünüşü
(`layerStyle`) bir nesne alanı olmadığından alınmaz; böyle bir fark varsa `layer_style_differs` uyarısı verilir.

Çıktı: `added`, `replaced`, `removed`, `unchanged` (uid listeleri, girdi sırasında), `revision`. Plan aynı listeleri
okuma kopyasında üretir ve beklenecek revizyonu verir.

## 6. Masaüstü

- **İnceleme notları** penceresi (Açıklama sekmesi › İşaretleme grubu, ikon `reviewNotes`): kitin veri tablosunda No,
  Durum, Metin, Bağ, Eskime sütunları; Açık/Yanıtlandı/Çözüldü/Eskimiş süzgeci; “Seçimden not ekle”, “Yanıtla”,
  durum seçimi, “Yeniden bağla”, “Nesneleri seç”, “Sil”. Eskimiş not uyarı renginde ve nedeniyle gösterilir.
  Pencere ve tablo `RoadDialog` iskeletinin genel CAD karşılığı olan kit `Dialog`'udur.
- **Bölünmüş revizyon görünümü:** “Farkları al…” düğmesi farkları (eklendi/silindi/değişti) işaretlenebilir bir tabloda
  açar; “Seçilenleri al” `cad.drawing.take`'i çalıştırır. Alınan durum görünüşte gösterilen sürümün kendisidir:
  uygulamanın geçici klasörüne `.kcad` olarak yazılır ve yoluyla verilir. Böylece açık çizimde sonra yapılan
  düzenlemeler (geçmişin “adım geri” sayısını kaydırır) alınan şeyi değiştirmez; komut kaydında bu geçici yol görünür.
  Sonuç tek Undo ile geri alınır. Not değişikliği domain'de ayrı bir geri alma işlemidir (`Op::ReviewNotes`):
  proje ayarlarının geri kalanı Undo dışında kalır, notlar kalmaz.

## 7. Bağımsız doğrulama

`scripts/fixtures/review_cases.py` KentOS kodu kullanmadan her komutun ortak durumlarını yazar (her biri ≥20):
metin/kimlik/sayı retleri, kilitli katmana not, damga ve eskime (taşıma → `changed`, silme → `removed`, Undo →
`current`), yanıt/durum, yeniden bağlama, silme; alma için eklenen/silinen/değişen/aynı, kilit, eksik katman/blok,
üç yollu çatışma ve `overwrite`, tek Undo. Damga değeri Python'da ADR 0253 §2'nin tarifiyle (kanonik JSON'un
SHA-256'sının ilk 16 onaltılık hanesi) hesaplanmaz; ortak durumlar damganın kendisini değil, eşitliğini ve sonuçlarını
denetler.

## 8. Sınırlar

- Not, alt nesne (path köşesi, ölçü referansı) düzeyinde bağlanmaz; nesne düzeyindedir. Alt nesne bağı sonraki iştir.
- Özel durum modelleri (Bluebeam tarzı), atama/kişi listesi ve bildirim yok.
- `cad.drawing.take` blok tanımı, katman görünüşü, pafta/görünüş (`projectChanges`) almaz; bunları uyarıyla bildirir.
- PDF içindeki yorumları içe alma bu ADR'de yok; `origin: pdf` el ile ya da otomasyonla girilen not içindir.
- Web dururken (ADR 0228) web karşılığı yazılmaz; TODOS.md §0.2'ye bir satır eklenir.
