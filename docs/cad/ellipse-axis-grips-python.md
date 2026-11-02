# Tam elips eksen tutamacı — Python ve MCP

`cad.entities.edit`, `operation="grip"` ile kaynak elipsin bir eksenini
seçerek düzenler. `index`: 0:+major, 1:+minor, 2:−major, 3:−minor.
`at` dünya metre koordinatıdır; çizimin mm/m gösteriminden bağımsızdır.
Eksen sırası değişirse indeksi komut eşler; sonraki çağrı yeniden mevcut
elipsin eksenlerini kullanır. Bu indeks kalıcı bir bağ kimliği değildir.

```python
from kentos import cad

change = cad.EllipseAxisEntityEdit(
    uid=ellipse_uid, index=1, at=cad.Vec2(x=487000, y=4420003)
)
preview = cad.entities.edit.plan(doc, operation="grip", changes=[change])
result = cad.entities.edit(doc, operation="grip", changes=[change])
diagnosis = cad.constraints.query(doc)
doc.undo()
```

MCP: `cad.entities.edit` girdisi:

```json
{
  "drawing": "<açık çizim tanıtıcısı>",
  "operation": "grip",
  "changes": [{
    "kind": "ellipseAxis", "uid": "<elips UUID>", "index": 1,
    "at": {"x": 487000, "y": 4420003}
  }]
}
```

Yazmadan planlamak için `op: "plan"` eklenir. Bu girdi tam elips içindir;
kısmi elipsin Start/End düzenlemesi mevcut geometri `update` yolunu kullanır.
Hedef diğer kalıcı bağlarla mümkün değilse `constraint_conflict` döner;
hiçbir kısmi geometri yazılmaz. Katman kilidi ve beklenen sürüm denetimi
normal `cad.entities.edit` ile aynıdır. Geçici sürükleme hedefi kaydedilen
kısıt sayısına/serbestliğine eklenmez.

[Uygulama kararı ve kabul](../adr/0842-explicit-full-ellipse-axis-grip-targets.md).
