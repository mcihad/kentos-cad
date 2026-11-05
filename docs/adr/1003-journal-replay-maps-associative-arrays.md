# ADR 1003: Komut günlüğünün yeniden oynatılmasında ilişkili dizi kimliği

- **Durum:** kabul edildi (2026-11-05). Kapsam: plan/cad-advanced.md `CAD-ADV-20`'nin açık işi “Komut günlüğünün
  yeniden oynatılmasında `AssocArray` kimliği → yeni dizi kimliği eşlemesi” (ADR 0873'ün CAD-ADV-20'ye bıraktığı iş).
- **Bağlam belgesi:** ADR 0253 §3 (günlük ve yeniden oynatma), ADR 0873 (var olan yerleştirmeleri diziye
  benimsetme), `native/application/src/replay.rs`.

## 1. Karar

Komut satırı (`CallRecord`) ve komut özeti (`CommandSummary`) nesne kimliklerinden ve otomatik ölçü gruplarından ayrı
olarak `arrays: NodeChanges` taşır: komutun eklediği, değiştirdiği ve sildiği ilişkili dizi kimlikleri (adımın
`ArrayAdd/ArrayUpdate/ArrayRemove` işlemlerinden). Alan boşsa yazılmaz; eski günlükler aynı okunur.

Yeniden oynatma bir satırın `arrays.added` listesini, o satırın yeniden çalıştırılmasının özetindeki `arrays.added`
ile sırayla eşler (`mapped`); sonraki satırın girdisindeki eski dizi kimliği (`cad.arrays.edit`'in `array`'i gibi)
yeni çalıştırmanın kimliğiyle değiştirilir. Yapılan/değişen/silinen dizi sayısı farklıysa satır `differs` olur.
Komutun kendi girdisi ve plan parmak izi değişmez; doğrudan Python/MCP çağrısı ve istek tekrarı bu eşlemeyle
karışmaz (ADR 0873).

## 2. Doğrulama

`review_take::a_replayed_journal_maps_an_associative_arrays_id`: `cad.entities.array` ile 2×3 ilişkili dizi, ardından
`cad.arrays.edit` ile 3×4; günlük JSON'a yazılıp okunarak başka bir çizimde oynatılır. Kurulum satırı dizinin
kimliğini kaydeder, oynatma onu kendi yaptığı diziye eşler, düzenleme o diziye uygulanır (12 öğe), başarısız ya da
farklı satır yoktur. Eşleme olmasaydı düzenleme satırı olmayan diziyi adlandırıp `failed` olurdu.

## 3. Sınırlar

Paylaşılan geçmişte birden çok kullanıcının günlüklerini birleştirme ve dizinin kalıcı öğe kimliklerinin ayrı
eşlenmesi (öğeler zaten nesne kimliği olarak eşlenir) bu ADR'de yoktur.
