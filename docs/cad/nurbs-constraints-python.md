# NURBS uçlarını ölçüyle bağlama

Yerel masaüstü, Python ve MCP mevcut `cad.constraints.edit`,
`cad.parameters.edit`, `cad.constraints.query` yolunu kullanır.
Destek: açık, periyodik olmayan, iki ucu clamped NURBS'in gerçek Start/End
noktaları. Yalnız iki uç değişir; iç kontrol noktaları, derece, düğümler
ve ağırlıklar korunur. Serbestlik hesabı bu uç modeline aittir.
[Kesin sınır ve kanıt](../adr/0832-bounded-nurbs-endpoint-constraints.md).

```python
from kentos import cad

a = dict(uid=source, element=dict(kind="point", place=dict(kind="start")))
b = dict(uid=source, element=dict(kind="point", place=dict(kind="end")))
edit = cad.AddConstraintEdit.from_json(dict(kind="add", constraints=[
    dict(rule=dict(kind="fix", a=a)),
    dict(rule=dict(kind="horizontal", a=a, b=b)),
    dict(rule=dict(kind="distance", a=a, b=b, parameter="aciklik"), value="6m"),
]))
cad.constraints.edit.plan(doc, edit=edit)  # Salt okunur
cad.constraints.edit(doc, edit=edit)
cad.parameters.edit(doc, edits=[cad.SetParameterEdit(name="aciklik", expression="8m")])
state = cad.constraints.query(doc)
```

`6m`/`8m` fiziksel uzunluktur; proje mm ise sonuçlar 6000/8000 mm'dir.
Bu ölçü eğri uzunluğu değil iki uç arasındaki mesafedir. Eğriyle çakışık
bağ kurulacaksa öteki nokta için aynı Point başvurusu kullanılır.
Kapalı, periyodik veya uç yapısı uygun olmayan eğriye bu bağı ekleme
isteği açıklamayla reddedilir; başarısız işlem belgeyi değiştirmez.
Gerçek geometri değişiminde eski fit metadata'sı temizlenir; Undo geri
getirir. Genel NURBS teğeti/yarıçapı veya iç kontrol noktası bu kapsamda yoktur.

MCP için `cad.constraints.edit` girdisinde aynı `edit` nesnesi, açık
çizimin `drawing` tutamacı ve salt okunur deneme için `op: "plan"`
gönderilir. `cad.parameters.edit` aynı `edits` listesini alır.
