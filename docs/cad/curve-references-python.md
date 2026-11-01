# NURBS üzerinde kalıcı ölçü başvuruları

`cad.dimensions.create` ve `cad.dimensions.reassociate`, eğrinin kalıcı
kimliğine bağlanır. Başlangıç/bitiş, kontrol noktası değildir: eğrinin
knot aralığının gerçek uçlarında değerlendirilir. `AlongSubPlace.t`,
0–1'e normalize edilmiş **parametre aralığıdır**; eğri uzunluğunun aynı
yüzdesi veya ekran kirişinin üzerindeki bir nokta anlamına gelmez.

```python
from kentos import cad

cad.dimensions.reassociate(doc, uid=dimension_uid, points=[
    cad.PointBinding(point="a", to=cad.PlaceOn(
        uid=curve_uid, place=cad.StartSubPlace())),
    cad.PointBinding(point="b", to=cad.PlaceOn(
        uid=curve_uid, place=cad.AlongSubPlace(
            part=0, ring=0, index=0, t=0.5))),
])

# Geometri değişikliği ve bağlı ölçünün güncellenmesi aynı Undo adımıdır.
cad.curves.edit(doc, uid=curve_uid,
    edit=cad.MoveControlCurveEdit(index=0, at=(2, 0)))

# Şekli koruyan düğüm eklemesi mevcut parametre başvurusunu korur.
# 'at' normalize kesir değil, NURBS'in kendi knot aralığındadır.
cad.curves.edit(doc, uid=curve_uid,
    edit=cad.InsertKnotCurveEdit(at=3))
```

Düğüm aralığı örneğin `[2,6]` ise `t=.5`, `u=4`'teki noktadır.
Başka eğrinin `[0,1]` aralığında olduğu varsayılmamalıdır. Sahnede uç
ve eğri üzeri yakalama aynı gerçek eğriyi değerlendirir. Denetim ağına
veya tessellation kirişine yanlışlıkla başvuru verilmez.

Kır işleminde başvuru, noktanın bulunduğu parçaya ve onun kimliğine
aktarılır. Buda'da kesilen uç yeni ucu izler; silinen iç noktadaki bağ
kopuk olarak bildirilir, ölçü son koordinatını tutar. Açıkça seçilen
NURBS→çoklu çizgi dönüşümü tür değiştirdiği için çözülemeyen başvurular
`kind_changed` ile işaretlenir. `entity.to_json()["refs"]` içindeki
`broken` alanı bu durumu gösterir; kopuk başvuru yeni bir canlı bağ diye
sunulmamalıdır. Tek Undo geometriyi ve başvuruları beraber geri getirir.

MCP aynı komut adlarını ve `place` yapılarını kullanır; ayrı geometri
veya ölçek sistemi yoktur. Native kapsam ve sayısal sınırlar
[ADR 0826](../adr/0826-nurbs-measurement-references.md)'dadır.
