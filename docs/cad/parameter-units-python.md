# Parametre ifadelerinde uzunluk, açı ve sayı

CAD-ADV-05 / [ADR 0840](../adr/0840-parameter-dimensions-without-legacy-reinterpretation.md).
Uzunluk ifadeleri proje uzunluk biriminde, açı ifadeleri proje açı
birimindedir. `cad.*` geometri koordinatları ise metre olarak kalır.

| İfade | Sonuç türü | Kullanım |
|---|---|---|
| `250` | Hedefin proje birimi | mm projede 250 mm |
| `250mm` | Uzunluk | Her projede 0.25 m fiziksel uzunluk |
| `boy + 25` | Uzunluk | Proje biriminde 25 ekler |
| `boy / 2` | Uzunluk | Yarısı |
| `boy / en` | Sayı | Birimsiz oran |
| `sqrt(boy^2 + en^2)` | Uzunluk | Köşegen |
| `boy * cos(donus)` | Uzunluk | İzdüşüm |
| `pi / 2` | Açı | 90° / 100 gon |
| `pi / 1rad` | Sayı | Matematiksel π |
| `adet * 1mm` | Uzunluk | Number parametresine açık birim verir |

Uzunlukla açı toplanamaz. `boy*en` bir ara alan olabilir, doğrudan Length
parametresine atanmaz. `sqrt(boy)` uzunluk değildir. Birimsiz oran da
doğrudan Length sayılmaz. Sayı üs değişirse onu okuyan formüllerin
boyutları yeniden kontrol edilir.

```python
from kentos import cad

doc = cad.Document.new(name="Ölçüler", workspace="cad", drawing_unit="mm")

def parameter(name, kind, expression):
    return cad.ParameterEdit.from_json({
        "kind": "add",
        "parameter": {"name": name, "kind": kind, "expression": expression},
    })

edits = [
    parameter("boy", "length", "4m"),
    parameter("en", "length", "3m"),
    parameter("kosegen", "length", "sqrt(boy^2 + en^2)"),
    parameter("oran", "number", "boy / en"),
]
preview = cad.parameters.edit.plan(doc, edits=edits)  # yazmaz
result = cad.parameters.edit(doc, edits=edits)        # tek Undo adımı
assert next(v.value for v in result.values if v.name == "kosegen") == 5000

bad = cad.ParametersEdit(edits=[cad.SetParameterEdit(name="boy", expression="45deg")])
answer = cad.parameters.edit.run(doc, bad, op="plan")
assert answer.error.code == "parameter_kind"
print(answer.error.message, answer.error.path)
```

MCP aynı `cad.parameters.edit` aracına `drawing` ve aynı girdiyi alır;
`op: "plan"` yazmadan denetler. Yeni ölçünün `cad.constraints.edit`
girdisindeki `value` da aynı kurala tabidir. Hata belgeyi veya Undo
geçmişini değiştirmez; geometriye özel ikinci bir AI yolu yoktur.

Eski dosyalarda `pi` Number parametresinde kullanılmış olabilir; mevcut
anlamı 180° veya 200 gon'un **sayısal değeri** idi. Açınca otomatik π'ye
dönüştürülmez. `cad.constraints.query.run(doc, cad.ConstraintsQuery())`
sonucunun `warnings` alanı `parameter_units_legacy` kodunu ve ilgili
parametre yolunu verir. Normal `.query(doc)` çağrısı Python warning üretir.
Parametre penceresi de aynı uyumsuzluğu gösterir. İfadeyi `pi / 1rad`
yapmak açık bir anlam değişikliğidir; önce etkisini planlayın, sonra
uygulayın. Rename eski sayısal anlamı korur; onarım tek Undo ile geri alınır.
