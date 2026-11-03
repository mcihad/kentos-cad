# Kitaplıktaki yeni bileşen yayınını uygula

Bloklar → Kitaplık'ta bir KCAD dosyasını açın. Çizimde aynı aile ve tipin
daha eski yayını varsa, hedef blok satırına sağ tıklayın ve **Yayını
karşılaştır…** seçin. Birden fazla eski tanım varsa alt menüden seçilir.

Pencere mevcut ve hedef yayını gösterir. Sahnedeki seçili yerleştirmeler
varsa başlangıç kapsamı yalnız onlardır; kapsamı açıkça bütün doğrudan
yerleştirmelere genişletebilirsiniz. İç içe eski kullanımlar korunur.
Eklenecek tanımlar, değişecek kayıtlar, kilitli katman uyarısı ve düşecek
öznitelikler onaydan önce görünür. **Vazgeç** ve Esc yazmaz. **Değiştir**
incelenen planı uygular; tek geri alma hem yeni tanımları hem yerleştirmeleri
eski hale döndürür.

Yeni yayın eski tanımı ezmez. Ortak tip değeri hedef tanımdan gelir;
eski tanımın ortak değeri korunur. Uyumlu örnek girdileri korunur; hedefin
hesaplanan değerleri yeniden üretilir. Daha eski/aynı yayın, başka aile/tip,
birimsiz–fiziksel geçiş ve uyumsuz parametre reddedilir. Aynı yayın hedefi
yerel olarak değiştirilmişse mevcut tanım sessizce ezilmez. Tip adı yalnız
etikettir; kimlikler karşılaştırılır. Kaynak dosyanın kitaplığa okunmuş hali
incelenir; dosyayı açmak otomatik güncelleme yapmaz.

## Python ve MCP

```python
from kentos import cad

doc = cad.Document.open('/mutlak/yol/proje.kcad')
source = cad.Document.open('/mutlak/yol/yayin3.kcad')
library = cad.blocks.edit(source, operation='export').library

# from_ çizimdeki eski tanımın; to kaynak dosyadaki yeni tanımın kimliğidir.
inserts = list(doc.entities(kinds=['insert']))
args = dict(from_=doc.blocks()[0].id, to=source.blocks()[0].id,
            library=library, uids=[inserts[1].uid])
plan = cad.blocks.replace.answer(doc, **args, op='plan')
if plan.status == 'completed':
    result = cad.blocks.replace(doc, **args, expected=cad.Expected(plan=plan.plan_id))
    print(result.target, result.replaced)
    doc.save('/mutlak/yol/yukseltilmis.kcad')
```

Uygulama konsolunda `doc = cad.current()` kullanılır ve normal Kaydet
arayüzüyle kaydedilir. Girdiler açık kimlik ve kapsam taşımalıdır; yukarıdaki
`blocks()[0]` yalnız örnek içindir, gerçek betikte doğru aile/tip seçilir.

MCP'de aynı `cad.blocks.replace` aracı `drawing`, `from`, `to`, `library`,
`uids` alır. Önce `op:"plan"`, sonra aynı girdiyle `expected:{plan:...}`
gönderilir. Önizlemeden sonra kaynak metnini değiştirip eski planı uygulamak
reddedilir. Kaynak kimliğiyle hedef çizim UUID'sinin aynı olmasını beklemeyin;
sonucun `target` alanını kullanın. Eski tanımları koruyan bu kipte `purge`
verilmez. [Karar ve kabul kapsamı](../adr/0857-selected-block-publication-upgrade.md).

Kitaplık metni yayın kimliği için sürüm 2, ortak tip değerleri taşıyorsa
sürüm 3 kullanır. Masaüstü Kitaplık yüzeyi ve `cad.blocks.edit export`
aynı minimum sürüm hesabını yapar; numarası elle düşürülmüş ortak değerli
kaynak, hiçbir tanım eklenmeden reddedilir. [Ortak değerler](block-type-values.md).
