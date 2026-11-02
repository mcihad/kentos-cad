# Blok parametrelerinde formül

Bir formül `@Genişlik` gibi adlarla mevcut **sayı** parametrelerini okur.
Doğrusal değer uzunluk, açı değeri radyandır. Komutların fiziksel uzunluk
sözleşmesi metredir; birimsiz blokta değerler tanımın ham koordinatlarıdır.
Panel proje birimiyle gösterir. Hesaplanan
parametre panelde salt okunur görünür; sonucunu doğrudan değiştirmek
`derived_parameter` reddi verir. Sonucu değiştirmek için girdisini düzenleyin.

Kutupsal parametrenin `rule.expression` değeri uzaklığı hesaplar. Örneğin
başlangıç `(0, 0)`, tanım ucu `(1.8, 0)` ve formül `@Genişlik * 2` ise,
1.05 m genişlikte sonuç **`(2.1, 0)`** olur. Başlangıç ve tanım yönü
korunur. Formülün sonucunu açı veya X koordinatı saymayın; negatif uzaklık
kabul edilmez. Kutupsal/nokta parametresi bir sayı değişkeni gibi `@ad`
ile okunmaz; mevcut ifade girdileri doğrusal ve açı parametreleridir.

Parametre eklemek tek başına geometriyi taşımaz: hangi parçaların nasıl
hareket edeceği blok düzenleyicisinin parametre/eylem ilişkisiyle belirlenir.
Görünürlük, lookup, örneğe özel girdiler ve sunum grupları bu ilişkiden ayrıdır.

## Python ve MCP

Mevcut bir `Genişlik` sayı parametresi olan blokta:

```python
from kentos import cad

block = doc.blocks()[0]
dynamic = block.to_json()["dynamic"]
dynamic["parameters"].append({
    "kind": "polar", "name": "Formüllü uç",
    "from": {"x": 0.0, "y": 0.0},
    "to": {"x": 1.8, "y": 0.0},
    "rule": {"expression": "@Genişlik * 2"},
})
args = dict(operation="dynamic", block=block.id,
            dynamic=cad.DynamicBlock.from_json(dynamic))
plan = cad.blocks.edit.answer(doc, **args, op="plan")
if plan.status == "completed":
    cad.blocks.edit(doc, **args, expected=cad.Expected(plan=plan.plan_id))
```

Aynı `cad.blocks.edit` MCP aracında çizim `drawing`, korumalı çalıştırma
`expected.plan` ile belirtilir. Plan bütün kök/iç kullanımları doğrular;
boş parametre haritalı yerleştirmeler de hesaplanır. Bir mevcut örnekte
hesap tanımsızsa veya girdi yeni kurala uymuyorsa formül bütünüyle
reddedilir. Genişliği varsayılana çekip hatayı gizlemez.

Yeni türetilmiş parametreyle aynı adda eski bir literal override varsa,
bu değer kendiliğinden silinmez; önce ilgili örnek girdisini gözden geçirin.
Kilitli yerleştirmede kaynak formülü değiştiğinde yalnız hesap önbelleği
yenilenebilir; kullanıcıya ait literal girdiler korunur. Doğrudan parametre
düzenlemesi katman kilidine tabidir. Tek Undo tanımı, üst tanımlardaki
hesapları ve kök sonuçları birlikte geri getirir.

Bu davranış [ADR 0860](../adr/0860-polar-formulas-and-complete-reference-refresh.md)
ile tanımlıdır. Yalnız grup adını/sırasını değiştirmek hesapları yenilemez.
