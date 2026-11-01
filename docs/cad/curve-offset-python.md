# Python ve MCP ile toleranslı eğri ötelemesi

Çizgi ve daire/yay için mevcut `cad.entities.offset` çağrısı korunur.
Elips, fit eğrisi ve NURBS için çoklu çizgi yaklaşımını açıkça seçin.
Bütün uzaklıklar metre; çizim birimi veya ekran yakınlığı çağrıyı değiştirmez.

```python
from kentos import cad

doc = cad.Document.open("cizim.kcad")
uid = next(doc.entities(kinds="nurbs")).uid
by = cad.DistanceOffsetBy(distance=0.20, side=(12.0, 8.0))
approximation = cad.OffsetApproximation(tolerance=0.0001)  # 0,1 mm

proposal = cad.entities.offset.answer(
    doc, op="plan", uids=[uid], by=by, approximation=approximation,
)
made = cad.entities.offset(
    doc, uids=[uid], by=by, approximation=approximation,
    expected=cad.Expected(plan=proposal.plan_id),
)
for report in made.approximations:
    print(report.source, made.created[report.copy_index],
          report.error_bound, report.crossings)
```

`side` eğrinin istenen yanındaki çizim koordinatıdır; ötelemenin yeni
merkezi değildir. Tek bir istek için `.plan(...)` da salt okunur geometri
ve aynı `approximations` raporunu verir. MCP aynı girdiyi alır:

```json
{
  "drawing": "acik-cizimin-kimligi",
  "uids": ["kaynak-nesnenin-uuid-kimligi"],
  "by": {"kind": "distance", "distance": 0.2, "side": {"x": 12, "y": 8}},
  "approximation": {"tolerance": 0.0001},
  "op": "plan"
}
```

Kaynak UID ve `copyIndex` sonuç dizinini eşler. Bu rapor kaynak izidir;
sonraki kaynak değişikliklerini otomatik izleyen bir kısıt değildir.
Kaynak varsayılan korunur. `delete_source=True` yalnız açık seçimle
siler, kopyalarla aynı Undo adımındadır. `both_sides=True` iki kopyanın
her birini aynı kaynak ve ayrı `copy_index` ile raporlar.

`error_bound` örneklenmiş orta nokta hatası değildir: paralelin türev
üst sınırı ve f64 güvenlik payını içerir. Tam IEEE aralık ispatı olarak
sunulmaz. Tekil normal, içe katlanma, teğetsiz birleşim, aşırı ağırlık
oranı, hassasiyet veya hesap bütçesi aşımı bütün yaklaşık işlemi reddeder.
Başarısızlıkta kaynak silinmez; tolerans kendiliğinden büyütülmez.
Yaklaşım seçimi eksikse `invalid_tolerance` döner.

Kod, matematik ve doğrulama sınırları:
[ADR 0822](../adr/0822-user-toleranced-curve-offsets.md).
