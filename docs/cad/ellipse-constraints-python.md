# Elips merkezi ve eliptik yay uçlarını kısıtlama

2026-11-02. Yerel masaüstü, Python ve MCP aynı `cad.constraints.edit`,
`cad.parameters.edit` ve `cad.constraints.query` komutlarını kullanır.
Elips eğrisi poliline çevrilmez. [Destek sınırı ve kanıt](../adr/0829-native-ellipse-point-constraints.md).

Bir elipsin `source` kalıcı kimliği biliniyorsa, merkezini tutup mevcut
başlangıç noktasına uzaklığı parametreyle yönetmek için:

```python
from kentos import cad

center = dict(uid=source, element=dict(kind="point", place=dict(kind="center")))
start = dict(uid=source, element=dict(kind="point", place=dict(kind="start")))
edit = cad.AddConstraintEdit.from_json(dict(kind="add", constraints=[
    dict(rule=dict(kind="fix", a=center)),
    dict(rule=dict(kind="distance", a=center, b=start, parameter="eksen"), value="4m"),
]))
# plan çizimi değiştirmez. Var olan bağlarla çelişiyorsa sonucu inceleyin.
cad.constraints.edit.plan(doc, edit=edit)
cad.constraints.edit(doc, edit=edit)
cad.parameters.edit(doc, edits=[cad.SetParameterEdit(name="eksen", expression="6m")])
state = cad.constraints.query(doc)
```

Başlangıç noktası tam elipste kaydedilmiş `t0` dikişidir. Büyük eksen
ucunu kastediyorsanız `t0 = 0` olmalıdır; aksi halde bu noktaya merkezden
olan gerçek uzaklık ölçülür. Eliptik yayda Start/End gerçek yay uçlarıdır.
Bu destek ayrı bir eksen noktası veya genel elips teğeti API'si değildir.

MCP aynı öğeleri ve ifadeyi alır:

```json
{
  "drawing": "<açık çizim tutamacı>",
  "edits": [{"kind": "set", "name": "eksen", "expression": "6m"}],
  "op": "plan"
}
```

Bu `cad.parameters.edit` girdisidir; `op: execute` yazmayı gerçekleştirir.
Çelişki veya kilitli geometride ret halinde geçerli çizim korunur.
Genel parametre ad değiştirme, bağımlılık ve sürücü/başvuru incelemesi
[destek belgesinde](parametric-support.md) açıklanır.
