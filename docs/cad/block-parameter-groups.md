# Blok parametre grupları ve sıfırlama

Dinamik blok seçildiğinde tanım/sürüm ve örneğin değişmiş değer sayısı
**Blok parametreleri** bölümünde görünür. **Boyut ve konum**, **Görünüş**,
**Katalog** ve **Hesaplanan** grupları ayrı kapatılabilir; boş gruplar yoktur.
Hesaplanan alanlar formülüyle okunur, doğrudan yazılmaz.

**Değerler** menüsünde **Genişlik · Varsayılana döndür** yalnız genişliği;
**Tip · Varsayılana döndür** o kataloğun kontrol ettiği değerleri sıfırlar.
Diğer ayarlar korunur. **Hepsini varsayılana döndür** bütün örnek değerlerini
sıfırlar. Her işlem tek geri alma adımıdır. Tanımın kendisi değişmez.

```python
from kentos import cad

doc = cad.Document.open("fixtures/kcad/v2/dynamic-blocks.kcad")
uid = next(doc.entities()).uid
cad.blocks.parameters(doc, operation="set", uids=[uid],
                      values={"Genişlik": 1.05, "Görünüş": "Sade"})
before = doc.to_bytes()
args = dict(operation="reset", uids=[uid], names=["Genişlik"])
plan = cad.blocks.parameters.answer(doc, **args, op="plan")
assert plan.status == "completed" and doc.to_bytes() == before
cad.blocks.parameters(doc, **args, expected=cad.Expected(plan=plan.plan_id))
values = cad.blocks.parameters(doc, operation="get", uids=[uid]).inserts[0].values
assert values["Genişlik"] == .9 and values["Görünüş"] == "Sade"
doc.undo()
assert doc.to_bytes() == before
```

MCP: aynı `cad.blocks.parameters`, ek olarak `drawing`. Salt okunur
inceleme `op: "plan"`, korumalı uygulama `expected: {"plan": "<planId>"}`.
Katalog sıfırlaması için `names: ["Tip"]`; hepsi için `names` alanı verilmez.
API fiziksel blokta metre/radyan, birimsiz blokta yerel tanım sayıları
kullanır; panel proje birimini gösterir.
