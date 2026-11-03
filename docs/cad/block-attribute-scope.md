# İç içe blok özniteliklerinin etiketini değiştirme

Bloklar panelindeki **Öznitelikler…** penceresinde aynı satırın etiketini
değiştirin. Tanım başka bloklarda kullanılıyorsa **Etiket taşıma kapsamı**
alanından **İç içe tanımlar da dahil** seçin. Alt açıklama etkilenecek üst
tanımları gösterir. Kaydet değerleri ve özel yazı konumlarını yeni etikete
taşır; Vazgeç hiçbir kayıt yazmaz.

Varsayılan dar kapsam yalnız çizimdeki doğrudan yerleştirmeleri eşitler.
Bu kapsamda iç yerleştirme eski etiketli veriyi saklar; yeni etiketin
varsayılanını gösterebilir. Kapsamı genişletmek bilinçli bir seçimdir.
Kilitli etkilenen kullanım veya farklı hedef değeri varsa geniş kapsamlı
işlem bütünüyle reddedilir. Önce bildirilen kaydı inceleyin.

```python
from kentos import cad

block = doc.blocks()[0]  # İlgili tanımı açıkça seçin.
attributes = block.to_json()["attributes"]
for attribute in attributes:
    if attribute["tag"] == "NO":
        attribute["tag"] = "ODA"
args = dict(
    operation="attributes", block=block.id,
    attributes=[cad.AttributeDefinition.from_json(a) for a in attributes],
    renames={"NO": "ODA"}, rename_nested=True,
)
plan = cad.blocks.edit.answer(doc, **args, op="plan")
if plan.status != "completed":
    raise RuntimeError(plan.error.message)
# plan.output.changed içindeki üst tanımları da inceleyin.
cad.blocks.edit(doc, **args, expected=cad.Expected(plan=plan.plan_id))
```

MCP'de aynı `cad.blocks.edit` aracına `renameNested: true`, `renames`,
`op: "plan"` verin; aynı girdiyi `expected: {"plan": planId}` ile yürütün.
Python'da `rename_nested`, JSON/MCP'de `renameNested` kullanılır.

Daha önce dar kapsamda yapılmış değişim için tanımı yeniden yazmak yerine
`operation="sync", renames={"NO":"ODA"}, rename_nested=True` kullanın.
Hedef `ODA` tanımda mevcut ve sabit olmayan bir öznitelik olmalıdır.
İç tanımlarda yalnız bu etiket haritası uygulanır; varsayılanlar ve ilgisiz
alan önbellekleri genel olarak eşitlenmez. `sync.inserts` doğrudan çizim
kayıtlarının sayısını vermeye devam eder. `changed` üst tanımları da içerir.

`doc.undo()` tek adımda geri alır. İşlem parametre, görünürlük, boyut ve
yayın kimliklerini değiştirmez. Yerel tanım değişikliği kitaplıkta yeni bir
yayını kendiliğinden oluşturmaz. Ayrıntılar ADR0864'tedir.
