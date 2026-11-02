# Blok yerleştirme noktasını kısıtlama

Yalnız yerleşim çözülür; dönüş açısı, ayna, X/Y ölçekleri, öznitelikler ve
tanım değişmez. İç nesne kısıtları bu adaptörün kapsamında değildir.
Gösterilen blok serbestlikleri yalnız iki yerleştirme koordinatına aittir.
[Karar ve sınırlar](../adr/0833-block-insertion-anchor-constraints.md).

```python
from kentos import cad

# reference_uid bir nokta nesnesi, block_uid bir blok yerleştirmesidir.
def anchor(uid):
    return dict(uid=uid, element=dict(kind="point", place=dict(kind="node", part=0)))

a, b = anchor(reference_uid), anchor(block_uid)
edit = cad.AddConstraintEdit.from_json(dict(kind="add", constraints=[
    dict(rule=dict(kind="fix", a=a)),
    dict(rule=dict(kind="horizontal", a=a, b=b)),
    dict(rule=dict(kind="distance", a=a, b=b, parameter="yerlesim"), value="6m"),
]))
cad.constraints.edit.plan(doc, edit=edit)
cad.constraints.edit(doc, edit=edit)
cad.parameters.edit(doc, edits=[cad.SetParameterEdit(name="yerlesim", expression="8m")])
state = cad.constraints.query(doc)
```

Referans zaten kilitli katmandaysa fazladan `fix` eklemeyin; katman kilidi
noktayı tutar. `6m` ve `8m` fiziksel uzunluklardır; mm projede 6000/8000 mm
olarak görünür. Görünür bir blok köşesini Node(1) diye kullanmayın;
Node(0), bloğun kendi yerleştirme noktasıdır. Başarısız işlem atomiktir.
Dosyaya kaydedilmiş mevcut blok tanımı ve yerleştirme bilgisi korunur.

MCP, aynı `edit` nesnesini açık çizimin `drawing` tutamacıyla
`cad.constraints.edit` aracına gönderir; salt okunur önizleme için
`op: "plan"` kullanılır. Değer güncellemesi `cad.parameters.edit`
aracına aynı `edits` listesiyle yapılır; sonuç `cad.constraints.query`
ile okunur. Genel blok dönmesi/iç parça kısıtı iddiası yoktur.
