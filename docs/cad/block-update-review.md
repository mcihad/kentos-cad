# Blok değişikliğini uygulamadan incelemek

1. Bloğu **Düzenle** ile açın ve geometriyi düzenleyin.
2. Alt çubuktaki **Etkiyi gör** ile güncel taslağın etkisini okuyun.
3. Doğrudan/iç içe yerleştirmeler görünüşü değişebilecek kullanımlardır.
   Kilitli katman paylaşılan tanımın değişmesini yalıtmaz. Bağlı ölçü/tablo
   sayısı ile gerçekten değişecek kayıt sayısı ayrı gösterilir.
4. **Kapat** veya Esc taslağa döner. **Kaydet** asıl çizimi tek adımda
   günceller; **Vazgeç** asıl çizimi korur.

Python ve MCP'de mevcut komut planı gerçek kayıt farkını verir. Örnek
fiziksel kapıya 240 mm kol ekler; API uzunlukları bu tanımda metredir:

```python
from kentos import cad

doc = cad.Document.open("fixtures/kcad/v2/dynamic-blocks.kcad")
before = doc.to_bytes()
block = doc.blocks()[0]
pieces = list(block.entities)
pieces.append(cad.Entity.from_json({
    "kind": "line", "id": 5, "layerId": "", "attrs": {},
    "a": {"x": .06, "y": .45}, "b": {"x": .3, "y": .45},
}))
args = dict(operation="update", block=block.id, entities=pieces)
plan = cad.blocks.edit.answer(doc, **args, op="plan")
assert plan.status == "completed" and plan.plan_id
assert doc.to_bytes() == before
print(plan.summary.blocks, plan.summary.changed, plan.summary.dependents)

# Kullanıcının/ajanın incelemesinden sonra, aynı planı zorunlu tutarak yazılır.
done = cad.blocks.edit.answer(doc, **args,
                             expected=cad.Expected(plan=plan.plan_id))
assert done.status == "completed"
doc.undo()
assert doc.to_bytes() == before
```

MCP girdisi aynı `operation`, `block`, `entities` ile `drawing` ve
`op: "plan"` taşır. Cevabın `summary` ve `planId` alanları okunur; uygulamada
`op: "execute"`, `expected: {"plan": "<planId>"}` verilir. Planın `changed`
çıktısı tanımları, execute'un `changed` çıktısı kimliklerini taşır.

Tanım değişikliği çoğu Insert kaydını yeniden yazmaz; bu örnekte üç kapı
yeni kolu çizse de `summary.changed` boş olabilir. `cad.blocks.query`
`count` gerçek iç parça oluşumlarını sayabilir; UI'deki bir kez sayılan üst
seviye yerleştirme sayısıyla karıştırılmamalıdır. İnceleme otomatik kitaplık
güncellemesi veya seçilmiş örnekleri başka sürüme taşıma işlemi değildir.
