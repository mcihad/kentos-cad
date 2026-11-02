# Blok birimi ve boyut düzeltmesi

Bloklar → Bu çizim → blok sağ menüsü → **Birim**.

- Birimi doğrudan seçmek fiziksel boyu korur. Örneğin 0.9 m genişlikteki
  kapıyı **Milimetre** olarak etiketlemek kapıyı büyütmez veya küçültmez.
- **Sayıları koru: …** yanlış yorumlanmış boyları düzeltir. Metre kabul
  edilmiş 900 birimlik kapıyı milimetre kabul etmek, 900 m yerine 0.9 m
  yapar. Geometriyle birlikte dinamik ölçüler ve yerleştirmeye özel
  genişlikler de aynı oranda değişir.

Kanonik uzunluklar metredir; özellik paneli proje birimini gösterir.
Bloğun birim etiketi, projenin gösterim birimini değiştirmez.
Örneğin metre projesinde 900 mm kapı genişliği **0.9 m** görünür.

Düzeltme kilitli katmandaki doğrudan veya iç içe yerleştirmeyi etkiliyorsa
önce kilit açılmalıdır; işlem kısmen uygulanmaz. **Geri al**, tanımı ve
bütün etkilenen kayıtları birlikte eski haline getirir. Birimsizliğe
geçiş veya birimsizlikten çıkış mevcut yerleştirmeler varken reddedilir.

## Python ve AI/MCP

Arayüz ayrı bir geometri işlemi yapmaz; aynı komut kullanılır:

```python
from kentos import cad

args = dict(operation="units", block=block_id,
            units=cad.BlockUnits.MM, rescale=True)
plan = cad.blocks.edit.answer(doc, **args, op="plan")
if plan.status == "completed":
    cad.blocks.edit(doc, **args, expected=cad.Expected(plan=plan.plan_id))
```

Yalnız etiketi değiştirmek için `rescale=False` kullanılır. MCP'de
`cad.blocks.edit` aynı `operation`, `block`, `units`, `rescale`, `op` ve
`expected.plan` alanlarını alır; çizim `drawing` ile belirtilir. Plan ve
validate yazmaz. Ret durumunda çizim değişmez. Bu düzeltme yeni komut
veya dosya şeması eklemez; teknik karar [ADR 0859](../adr/0859-scoped-dynamic-block-unit-correction.md).
