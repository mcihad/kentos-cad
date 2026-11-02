# Blok özniteliğini kullanıcı değerlerini koruyarak yeniden adlandırma

Masaüstü Blok öznitelikleri penceresinde mevcut satırın etiketini değiştirin.
Aynı satırın eski etiketi hatırlanır; Kaydet, tanım ve doğrudan çizim
örneklerindeki değer/özel yazı konumunu aynı Undo adımında taşır. Yeni
varsayılan, kendi değeri olan örneği ezmez. Yeni satır açıp eskisini silmek
aynı işlem değildir: bu açık bir silme/eklemedir.

Tanım düzenlemesinde hedef etikette başka bir kullanıcı değeri veya özel
konum varsa kayıt `duplicate_tag` ile reddedilir. Çizim korunur; farklı
etiket seçin veya hangi verinin kalacağını önce düzenleyin. Kilitli örnekler
uyarıyla atlanır. Mevcut eşitleme yalnız doğrudan çizim yerleştirmelerinedir;
iç içe tanımların değerlerini otomatik değiştirmez.

Kabul çizimi: `fixtures/interaction/v1/block-attribute-identity.kcad`.
R-1/R-2/R-3 ve Beton/Çelik/Polimer birbirinden farklı kullanıcı değerleridir.

```python
from kentos import cad

doc = cad.Document.open("fixtures/interaction/v1/block-attribute-identity.kcad")
block = next(b for b in doc.blocks() if b.name == "Rögar")
attributes = [a.to_json() for a in block.attributes]
attributes[0]["tag"] = "KOD"
before = doc.to_bytes()

cad.blocks.edit(
    doc, operation="attributes", block=block.id,
    attributes=[cad.AttributeDefinition.from_json(a) for a in attributes],
    renames={"NO": "KOD"},
)
assert [r.entity.to_json()["attrs"]["KOD"] for r in doc.entities()] == ["R-1", "R-2", "R-3"]
doc.undo()
assert doc.to_bytes() == before
```

MCP aynı `cad.blocks.edit` girdisini kullanır: `attributes` yeni tam liste,
`renames` açık eski→yeni eşlemesi. `op="plan"` / `op="validate"` yazmaz;
`drawing.save` kaynak dosyanın farklı bir kopyasına yapılabilir.

Eşlemelerde A→B, B→A gibi değiş tokuşlar bütün kaynaklar okunarak eşzamanlı
uygulanır. Bununla birlikte eski açık `operation="sync"` göçü, hedefteki
verinin yerine kaynaktakini koyma sözleşmesini korur. Bu işlem normal tanım
satırını değiştirmekle aynı politika değildir; otomasyon bunu ancak bu
sonucu istediğinde kullanmalıdır. Aynı işlemde tek Undo değerleri getirir.

Bu dilim masaüstü/native/Python/MCP kabulüdür. Web öznitelik penceresinin
eşitleme kapsamı, iç içe güncelleme önizlemesi ve bileşen sürüm yönetimi
CAD-ADV-06'da henüz açıktır. [ADR 0844](../adr/0844-block-attribute-rename-identity.md).
