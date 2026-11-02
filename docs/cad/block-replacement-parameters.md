# Blok değişiminde örneğe özel değerler

Yerleştirmeyi seçin, **Öznitelikler › Geometri › Blok** alanından hedef tanımı
seçin. Yalnız o yerleştirme değişir. Uyumlu ad/türdeki örnek değerleri korunur;
hedef formülleri yeniden hesaplanır. Uyumsuz girdi varsa işlem reddedilir,
değerleri sessizce sıfırlamaz. **Değerler** menüsünden ilgili girdiyi bilinçli
sıfırlamak veya uyumlu bir hedef seçmek gerekir. Tek Undo değişimi geri alır.

Python:

```python
from kentos import cad

args = dict(from_=source_block_id, to=target_block_id, uids=[insert_uid])
plan = cad.blocks.replace.answer(document, **args, op="plan")
if plan.status == "completed":
    result = cad.blocks.replace.answer(
        document, **args, expected=cad.Expected(plan=plan.plan_id)
    )
else:
    print(plan.error.code, plan.error.path, plan.error.message)
```

MCP `cad.blocks.replace`, aynı alanları JSON'da `from`, `to`, `uids` olarak
alır. Önce `op: plan`, sonra `expected: {plan: plan_id}` ile execute kullanılır.
`invalid_parameter_value` hatasında `uids[n].parameters.<ad>` düzeltilecek
girdiyi belirtir. Plan/validate yazmaz; hatalı toplu işlem kısmen uygulanmaz.

Eski tanım ve diğer yerleştirmeler korunur. İç içe tanımların yerleştirmeleri
bu kapsamda değildir. Fiziksel tanımlarda sayılar kanonik metredir; panelde
proje birimi görünür. Birimsiz–fiziksel uzunluk değerleri otomatik eşlenmez.
Bu adım sürüm ailesi veya otomatik kitaplık güncellemesi eklemez.
