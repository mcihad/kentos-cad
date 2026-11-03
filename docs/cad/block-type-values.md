# Ortak tip ve örnek değerleri

Native arayüz, Python ve MCP kabulü ADR0863'tedir; web yüzeyi ayrı kapsamdadır.

Bir çizim bloğunun **Tip değerleri…** penceresindeki Tip değeri aynı
blok tanımının bütün kullanımlarına aittir. Örnek değeri sadece seçilen
yerleştirmede düzenlenir. Eski ve yeni kitaplık yayınları ayrı çizim
bloklarıdır; yayın kimliğinin aynı olması onları birbirine bağlamaz.

Farklı genişlikler taşıyan kapıları ortak tipe geçirmek bütün boyutları
sessizce değiştirmez: işlem reddedilir. Önce değerleri eşitleyin veya
ayrı blok tanımları kullanın. Ortak tipten örneğe dönmek mevcut ortak
boyutu bütün kullanımlara aktararak görünüşü korur. Kilitli bir kullanım
etkilenecekse ortak değişim reddedilir.

GUI proje biriminde gösterir; Python/MCP aynı mevcut kanonik parametre
birimini kullanır. Fiziksel blokta doğrusal değer metre, açı radyandır;
birimsiz blokta tanımın ham koordinatıdır. Örneğin mm proje ayarında
penceredeki 1200 mm fiziksel bloğun API'sinde `1.2`'dir. Yerleştirmenin
ayrıca verilmiş ölçeği bu parametrenin birimi değildir.

```python
from kentos import cad

block = doc.blocks()[0]  # Düzenlenecek çizim tanımını açıkça seçin.
dynamic = block.to_json()["dynamic"]
# Örnek: bütün kullanımlarda zaten aynı 1.1 m genişlik var.
dynamic["typeParameters"] = ["Genişlik"]
dynamic["typeValues"] = {"Genişlik": 1.1}
args = dict(operation="dynamic", block=block.id,
            dynamic=cad.DynamicBlock.from_json(dynamic))
plan = cad.blocks.edit.answer(doc, **args, op="plan")
if plan.status != "completed":
    raise RuntimeError(plan.error.message)
# Etkiyi inceleyin. İncelenen plan başka bir değer için kullanılamaz.
cad.blocks.edit(doc, **args, expected=cad.Expected(plan=plan.plan_id))
```

Mevcut ortak kapsamı/değerleri düzenlerken haritayı ve listedeki diğer
adları koruyun; örnek kod yeni kapsam tanımlama örneğidir. Ortak formül
alanı da kapsamda ve tipli `typeValues` haritasında yer alır; motor onun
önbelleğini ortak girdilerden hesaplar. Tip formülü örnek girdisine
bağımlı olamaz. Lookup bütün satır hedefleri ortaksa ortak kapsamlıdır;
lookup adını `typeParameters` veya `typeValues` içine eklemeyin.
GUI ortak lookup için satır seçimi sunar. Python/MCP
aynı satırın `values` girdilerini ilgili `typeValues` anahtarlarına
yazar ve bütün tanımı korumalı planla uygular.

MCP'de aynı `cad.blocks.edit` aracı `dynamic.typeParameters/typeValues`,
`op=plan`, sonra `expected.plan` ile yürür. `cad.blocks.parameters get`
ortak ve örnek etkin değerlerini birlikte verir. Ortak adı örneğin `set`
veya adla `reset` işlemine vermek reddedilir; bütününü sıfırlamak ortak
kataloğu değiştirmez. Kapsamı örneğe döndürmek için adı ve ortak değerini
iki metadata alanından çıkarıp aynı korumalı tanım komutunu kullanın.

`doc.undo()` bütün kapsam/değer/grafik değişimini tek adımda geri alır.
Kaynak dosyayı kaydedip açınca metadata ve örnek girdileri korunur. DXF
statik görünüşü korur; parametrik düzenlenebilirlik kaybı ayrı raporlanır.

Ortak tip değerleri içeren dışa aktarılmış `kentos.blocks` kitaplığı sürüm
3'tür. Eski uygulamalar bu dosyayı boyutlarını yanlış göstererek açmak yerine
reddeder. Sürüm numarasını elle 1/2'ye indirmeyin; hem normal içe alma hem
yayın yükseltme bunu reddeder. Eski ortak değersiz kitaplıkların 1/2 sürümü
korunur. KCAD kaynak kaydı ayrı olarak şema 160 kullanır.
