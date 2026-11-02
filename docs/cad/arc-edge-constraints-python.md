# Yaylı çoklu çizgi kenarı — Python ve MCP

CAD-ADV-05 / ADR 0837. Dairesel bir path kenarı `arcEdge` ile tanımlanır:
`part`, `ring`, `index` sıfırdan başlar; `ring=0` dış sınırdır. Düz kenar
`segment`, bağımsız daire/yay `circle` olarak kalır. `arcEdge`, kenarın
**altındaki daireyi** temsil eder; sınırlı yay üzerinde serbest bir nokta
veya elips değildir. Uçları `point/vertex` başvurularıyla ayrı bağlanır.

```python
from kentos import cad

doc = cad.Document.from_bytes(open("cizim.kcad", "rb").read())
uid = next(r.uid for r in doc.entities() if r.entity.to_json()["kind"] == "polyline")
arc = cad.ArcEdgeCadElement(part=0, ring=0, index=0)
edit = cad.AddConstraintEdit.from_json({
    "kind": "add",
    "constraints": [{
        "rule": {"kind": "radius", "a": {"uid": uid, "element": arc.to_json()},
                 "parameter": "yaricap"},
        "value": "3m",
    }],
})
# Önce yazmadan etkiyi/teşhisi inceleyebilirsiniz.
plan = cad.constraints.edit.plan(doc, edit=edit)
cad.constraints.edit(doc, edit=edit)
cad.parameters.edit(doc, edits=[cad.SetParameterEdit(name="yaricap", expression="4m")])
assert all(r["holds"] for r in cad.constraints.query(doc).to_json()["constraints"])
```

Uçları sabit tutmak için aynı UID üzerinde aşağıdaki iki noktaya `fix`
bağı eklenir. Kapalı kenarın ikinci ucu son köşeden sıfırıncıya döner;
`index+1` koşulsuz kullanılmaz.

```json
{"kind":"point","place":{"kind":"vertex","part":0,"ring":0,"index":0}}
```

MCP'de aynı `cad.constraints.edit` girdisi `drawing` tutamacıyla gönderilir;
`op: "plan"` yazmadan önizler. `cad.parameters.edit`, `cad.constraints.query`,
`drawing.save`, `drawing.undo` aynı ürün komutlarını kullanır. Ek bir
çözücü veya AI'ye özel yaklaşık geometri yolu yoktur. İşlem ağırsa mevcut
komut işi/durdurma düzenini kullanır.

- Yarıçap/çap, eş yarıçap, eş merkezlilik ve mevcut daire kurallarının
  desteklediği bağlar kullanılabilir. Bir doğruyla ortak uçtaki teğet,
  köşeye Çakışık bağı ile kurulduğunda o uçta hesaplanır.
- Uçlar ayrı tutulmadıysa yarıçap değişikliği uçları da oynatabilir;
  `fix arcEdge`, alt dairenin merkezi ve yarıçapını tutar, iki köşenin
  tamamını dondurmaz. Bütün nesneyi geçici tutmak farklıdır.
- Yayın işareti ve küçük/büyük yay dalı korunur. Kirişe göre imkânsız
  yarıçapta işlem atomik reddedilir. Yarım dairede daha büyük yarıçap
  istenince küçük yay dalı kullanılır; mevcut büyük dal korunur.
- Geometri metre cinsindedir; değer ifadeleri proje birimini veya açık
  `mm/cm/m/in/ft` birimini kullanır. Bulge, boyutsuz `tan(sweep/4)`dür.
  Plan, execute ve query aynı teşhisi verir; başarısızlık belgeyi değiştirmez.
- Kalıcı başvuru KCAD şema 157 ister. Yeni bağ içermeyen dosya, mevcut
  en eski uygun şemada yazılmaya devam eder. Önceki okuyucuya özellik
  kaybettiren bir dosya yazılmaz; desteklemeyen okuyucu açıkça reddeder.
