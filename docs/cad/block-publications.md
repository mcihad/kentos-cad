# 2B bileşenin kitaplık kimliği

Blok kartındaki isteğe bağlı bileşen bilgisi aileyi, tipi ve yayın numarasını
saklar. Tip adı okunabilir etikettir; ad değişse de `typeId` korunur. Aynı
ailedeki farklı tiplerin `familyId` değeri aynıdır, `typeId` değerleri ayrıdır.
Yeni yayında aynı aile/tip kimlikleri korunup `release` artırılır. Çizimdeki
blok kimliği bunlardan bağımsızdır. Var olan kartın `version` alanı yerel
geometri değişikliği sayacıdır; yayın numarası olarak kullanılmaz.

Bloklar panelinde sağ tuş **Kart…** açın. Bileşen bilgisi seçeneği açılınca
tip adı ve yayın numarası görünür; yeni kimlikler otomatik hazırlanır.
**Kimlik ayrıntıları** bölümü aynı aile/tipin kimliğini kopyalamak veya
kitaplıktaki kimliği yapıştırmak içindir. **Vazgeç** çizimi değiştirmez.
Kaydetmek buluta yayın yapmaz. Dosya açılışında kendiliğinden yükseltme yoktur.

## Python

Bağımsız Python betiğinde mevcut bir çizime kimlik vermek:

```python
from uuid import uuid4
from kentos import cad

doc = cad.Document.open('/mutlak/yol/cizim.kcad')
block = doc.blocks()[0]
publication = cad.BlockPublication(
    family_id=str(uuid4()),       # yeni aile; aynı ailenin diğer tiplerinde korunur
    type_id=str(uuid4()),         # yeni tip; tip adı/yayın değişince korunur
    type_name='Standart tek kanat',
    release=1,
)
args = dict(operation='card', block=block.id, publication=publication)
plan = cad.blocks.edit.answer(doc, **args, op='plan')
if plan.status == 'completed':
    result = cad.blocks.edit.answer(doc, **args, expected=cad.Expected(plan=plan.plan_id))
    if result.status == 'completed':
        doc.save('/mutlak/yol/kimlikli.kcad')
```

Yeni tip üretmediğinizde rastgele yeni kimlik üretmeyin; mevcut
`block.card.publication` değerindeki kimlikleri kullanın. `publication`
verilmeyen normal kategori/etiket düzenlemesi kimliği korur. Açıkça kaldırmak:

```python
cad.blocks.edit(doc, operation='card', block=block.id, clear_publication=True)
```

`publication` ile `clear_publication=True` aynı çağrıda verilmez. Geçersiz
kimlik, sıfır yayın, boş/uzun/kontrol karakterli tip adı `invalid_block_card`
ve `publication` yoluyla reddedilir; çizim kısmen değişmez. Hatalı UUID
metni tip çözümlemesinde de reddedilebilir. Plan salt okunurdur; tek Undo
kartın eski kimliğini ve diğer bilgilerini birlikte geri getirir.

Uygulama içindeki konsolda `doc = cad.current()` kullanılır; dosya kaydı
normal Kaydet/Farklı kaydet arayüzüyle yapılır.

## MCP ve kayıt

Mevcut `cad.blocks.edit` aracında aynı alanlar camelCase kullanır:
`publication: {familyId, typeId, typeName, release}`, `clearPublication`.
Önce `op: "plan"`, sonra dönen kimlikle `expected: {plan: ...}` kullanılır.
Araç yeni bir ayrı geometri yazıcısı kullanmaz. Kimlik `drawing` oturumundaki
bloklardan okunur; yerleştirmeleri bu işlem değiştirmez.

KCAD şema 158 ve KentOS blok kitaplığı sürüm 2 veya üstü kimliği korur.
Ortak tip değeri yoksa kimlikli kitaplık sürüm 2, kimliksiz kitaplık sürüm 1
olarak yazılır. Ortak tip değeri varsa geometrinin eski okuyucuda sessizce
bozulmasını önlemek için sürüm 3 gerekir; bağımlı çocuk tanımlar da hesaba
katılır. Eski sürüm 1/2 kitaplıklar okunmaya devam eder. Kimliği sürüm 1'e,
ortak değeri sürüm 1/2'ye eklemek reddedilir. Eski uygulama desteklemediği
kitaplık sürümünü metadata'yı kaybetmek yerine reddeder. Eski çizimlerde kimlik
kendiliğinden üretilmez; yeni alan yoksa önceki dosya baytları korunur. Bu
bilgi bir imza veya yayın içeriğinin değişmediği garantisi değildir.
Seçilmiş yayın yükseltmesi [kitaplık yükseltme rehberinde](block-publication-upgrade.md)
anlatılır; [tip–örnek parametreleri](block-type-values.md) aynı yayın
kimliğine sahip eski ve yeni çizim tanımlarını birbirine bağlamaz. [Karar ve kabul kapsamı](../adr/0856-block-component-publication-identity.md).
