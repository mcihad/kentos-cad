# Blok parametre grupları ve sıfırlama

Dinamik blok seçildiğinde tanım/sürüm ve örneğin değişmiş değer sayısı
**Blok parametreleri** bölümünde görünür. **Boyut ve konum**, **Görünüş**,
**Katalog** ve **Hesaplanan** otomatik grupları ayrı kapatılabilir; boş gruplar yoktur.
Kullanıcının bir başlığa atadığı parametreler aşağıda anlatılan özel gruplarda gösterilir.
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

## Kullanıcı tanımlı başlıklar

Bloklar panelinde **Bu çizim** sekmesini açın. Dinamik bloğun sağ
menüsünden **Parametre grupları…** seçin. Aynı başlık altında gösterilecek
parametrelere aynı grup adını verin; ok menüsünden mevcut adı da
seçebilirsiniz. Boş alan veya **Otomatik grup** eski otomatik başlığı
korur. Aşağıdaki liste grup sırasını gösterir; oklarla değiştirilir.

Kaydet bütün yerleştirmelerin özellik paneli düzenini günceller.
Boyut, görünüş, katalog seçimi ve kullanıcıya özel ölçüler değişmez.
Bir kez Geri Al eski başlıkları geri getirir. Bu işlem parametreleri
ortak tip parametresine dönüştürmez.

### Python

```python
from kentos import cad

doc = cad.Document.open("proje.kcad")
block = next(b for b in doc.blocks() if b.name == "Kapı (dinamik)")
# Mevcut parametreleri ve eylemleri koruyarak yalnız sunum metadata'sını değiştir.
dynamic = block.to_json()["dynamic"]
dynamic["parameterGroups"] = [
    {"name": "Kanat", "parameters": ["Yön", "Genişlik"]},
    {"name": "Ürün bilgileri", "parameters": ["Tip"]},
]
args = dict(operation="dynamic", block=block.id,
            dynamic=cad.DynamicBlock.from_json(dynamic))
review = cad.blocks.edit.answer(doc, **args, op="plan")
assert review.status == "completed", review
cad.blocks.edit(doc, **args, expected=cad.Expected(plan=review.plan_id))
doc.save("duzenlenmis.kcad")
```

Listede olmayan parametreler otomatik gruplarda gösterilir. Grup adları
benzersizdir; üye adları tanımdaki adlarla tam eşleşir. API grup ve üye
sırasını korur. Otomatik düzene dönmek için mevcut dinamik sözlükten
`parameterGroups` anahtarını kaldırıp aynı komutu uygulayın; dinamik
tanımın kendisini `None` yapmayın.

### MCP

`cad.blocks.edit` aracında `operation: "dynamic"`, `block` ve bütün
mevcut `dynamic` verisi gönderilir. Yukarıdaki `parameterGroups` alanı
aynı JSON sözleşmesidir. Önce `op: "plan"`, sonra aynı girdinin
`op: "execute", expected: {plan: planId}` biçimi kullanılabilir.
Önizlemeden sonra grup bilgisi veya çizim değişirse eski onay yürütülmez.
Tanımı önce ortak sorgudan okuyun; geometri/eylem alanlarını uydurmayın.
