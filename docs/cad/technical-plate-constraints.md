# Birleşik teknik plakayla parametrik 2B çalışma

CAD-ADV-05'in tekrar kullanılabilir çizimi:
`fixtures/interaction/v1/constraint-technical-plate.kcad`. Dokuz nesne,
24 bağ, yedi parametre ve dört serbestlik vardır. Plaka, eşit delikler ve
üst teğet çizgi tam kısıtlıdır; elips/izleyen çizginin dört serbestliği
kalır. Birimler bu örnekte metredir. Ayrıntı: [ADR 0843](../adr/0843-combined-parametric-plate-and-readable-bars.md).

## Arayüz

1. Çizimi açın; Parametreler komutunu çalıştırın. `delik_yaricapi` için
   `1.5` yazıp Enter'a basın. Eşit iki delik ve teğet çizgi birlikte değişir.
2. `taban=7`, `genislik=taban*2` nedeniyle genişliği 14 m yapar.
   İfadesiz `kosegen` yalnız ölçer; geometriye değer dayatmaz.
3. Pencereyi kapatıp elipsi seçin; üst kısa eksen tutamacını seçip
   `487019,4420007` mutlak hedefini yazın. Diğer eksen ölçüye uyum sağlar.
4. Teğet simgesinin üzerine gelin: ilgili çizgi/daire vurgulanır. Kısıt
   çubuklarını Kısıtları göster eylemiyle gizleyip tekrar gösterebilirsiniz.
5. Çizim katmanı kilitliyken ölçü değişikliği açıklanarak reddedilir.
   Açmak için katmanın kilidini açıkça kaldırın. Kilit eylemi mevcut
   katman sözleşmesinde Undo adımı değildir.
6. Sabit sol alt köşeyi taşımak atomik reddedilir. Yalnız gerçekten
   bağları kaldırmak istediğinizde Gevşeterek düzenle'yi seçin; bir
   sonraki düzenlemenin bozduğu bağlar kaldırılır ve uyarı görünür.
   Bu geometri/bağ değişikliği tek Undo ile geri alınır.

## Python

```python
from kentos import cad

doc = cad.Document.open("fixtures/interaction/v1/constraint-technical-plate.kcad")
before = doc.to_bytes()
cad.parameters.edit(doc, edits=[
    cad.SetParameterEdit(name="delik_yaricapi", expression="1.5")
])
q = cad.constraints.query(doc)
assert len(q.constraints) == 24 and all(c.holds for c in q.constraints)
assert q.diagnosis.dof == 4
doc.undo()
assert doc.to_bytes() == before
```

Elips için [açık eksen hedefi rehberi](ellipse-axis-grips-python.md) aynı
komutu kullanır. API hedefi dünya metresidir; görüntü birimiyle çevrilmez.
`doc.relaxed()` açık gevşetme bağlamıdır; kalıcı katman kilidini aşmaz.

MCP'de `drawing.open` tutamacıyla `cad.parameters.edit`,
`cad.entities.edit` ve `cad.constraints.query` kullanılır. `op="plan"`
yazmaz; `relaxConstraints=true` yalnız açık gevşetme niyetidir. Başarısız
çağrıda `layer_locked` / `constraint_conflict` hata kodunu ve açıklamayı
okuyun; aynı çağrıyı körlemesine tekrarlamayın. Ortak uçtan uca kabul
`crates/native/mcp/tests/mcp.rs` içindeki `technical_plate_…` testindedir.

Çizim içindeki KCAD bilgisi bağımsız olarak okunabilir:

```sh
python3 tools/kcad/kcad.py dump fixtures/interaction/v1/constraint-technical-plate.kcad
```

Destek sınırları: [parametrik destek matrisi](parametric-support.md).
