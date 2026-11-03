# ADR 0873 — Mevcut blokları kimliklerini değiştirmeden ilişkili diziye bağlama

Tarih: 3 Kasım 2026. CAD-ADV-07'nin açık yerleşim ve öğe eşlemesi altyapısıdır.
Otomatik örüntü bulma, görsel aday inceleme akışı ve kaynak türü genişletme
bu kararla tamamlanmış sayılmaz.

## Dayanak

[BricsCAD V27 ARRAYDETECT](https://help.bricsys.com/en-us/document/command-reference/a/arraydetect-command?version=V27)
bulunan örüntülerin kullanıcı tarafından seçilip dönüştürülmesini sağlar;
önceden bloklaştırma aday kalabalığını azaltabilir.
[AutoCAD ARRAYEDIT](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-MAC/files/GUID-1D0F6016-F4DC-44BA-A517-B116D8CFC141.htm)
dizi özelliklerini, öğe değiştirmeyi ve silinmiş öğe/istisnaları açık Reset
ile geri getirmeyi ayırır. KentOS'ta bunun mevcut karşılığı `AssocArray`,
`ArrayItem` ve `cad.arrays.edit`tir (ADR0300/0301/0872).

Tekrar dönüştürme komutu (ADR0871) mevcut köklerden blok yerleştirmeleri
oluşturur. Bunları tekrar silip yeni bir ilişkili dizi oluşturmak UUID'leri
ve iç parçalara bağlanan ölçüleri gereksiz yere değiştirirdi. Yeni işlem
bu yerleştirmeleri olduğu yerde mevcut dizi kaydına bağlar.

## Sözleşme

`cad.arrays.adopt` v1; native uygulama, üretilmiş Python SDK, MCP ve
masaüstünün Python konsolu aynı komutu kullanır. `validate` ve `plan`
yazmaz; `execute` tek adımla yalnız bir dizi kaydı ekler.

- `source`: sıfır numaralı yeri tutan, korunmayan mevcut Insert UUID'si.
  Dizinin bloğu, katmanı ve ilk yerleşimi bu gerçek nesneden alınır.
- `layout`: mevcut grid/doğru/kutupsal/yol sözleşmesi; bütün uzunluklar
  metre, yerleştirme açıları radyan, kutupsal `fill` mevcut API gereği derece.
  Grid eksenleri mevcut modelde dünya eksenleridir; dönük bir grid varmış
  gibi bütün öğeler `kept` yapılarak saklanmaz.
- `items`: her yer için bir `ArrayItem`. Sıra mevcut yerleşim sırasıdır;
  grid satır satırdır. Boş kayıt eksik yeri, `kept: true` açıkça kabul edilmiş
  istisnayı gösterir. UUID iki yerde bulunamaz; bir Insert iki dizinin öğesi
  olamaz. En az iki normal öğe gerekir.
- `tolerance`: mutlak konum toleransı, varsayılan 0,000001 m = 0,001 mm.
  Yönler tam tur modunda 1e-10 rad, ölçekler 1e-12 bağıl toleransla ayrı
  karşılaştırılır; ayna birebir eşittir. Konum toleransı açı veya ölçek
  uyuşmazlığını gizlemez. Tolerans içindeki geometri bile taşınmaz. Boş veya
  korunan yer dahil her önerilen poz sonlu olmalıdır; sonraki Reset'te
  ortaya çıkacak bir sayı taşması incelemede gizlenemez.
- `expectedRevision` ve protokolün plan/stamp beklentileri geçerlidir.
  Eski inceleme yazmadan reddedilir. İstemcinin verdiği matris kullanılmaz;
  her normal öğenin gerçek pozu `domain::arrays::placements` ile karşılaştırılır.
  Yalnız metadata yazıldığı için kaynak/korunan öğe/yol ve iç içe blok
  tanımlarının okuma damgaları ayrıca kaydedilir. Plan kimliği, yeni dizi
  kimliği hariç gerçek önerilen metadata'yı da kapsar. Bütün yerleştirmeleri
  birlikte taşımak veya yalnız korunan öğeyi değiştirmek eski plana yazma
  izni vermez. Kaynak blok tanımı değişince de yeniden inceleme gerekir.

Plan; nil kimlikli önerilen `AssocArray`, en büyük normal konum sapması,
eksik/korunan sayıları ve belge sürümünü döndürür. Gerçek dizi kimliği
yalnız yazmada ayrılır. Çıktı mevcut UUID'leri yerleşim sırasında verir;
hiçbir kök ya da blok tanımı eklenmez, silinmez veya yeniden sıralanmaz.
Proje birimi, pafta ölçeği ve ölçü referansları değişmez. Tek Undo yalnız
dizi kaydını kaldırır; Redo aynı kayıt ve öğe kimliklerini getirir.
Sonraki açık sil/reset işlemleri mevcut modelin kurallarına tabidir:
yerinde kalan üyeler aynı UUID'yi taşır; Reset'in doldurduğu boş yer yeni
UUID alır. Silinmiş üyenin kimliği yeniden verilmez; silmeyi Undo ile
geri almak, Reset ile yeni üye oluşturmakla eşdeğer değildir.

## İlk kapsam ve gerekçeli ret

Yerleştirmeler ortak katman/görünüşlü, verisiz, tekdüze ölçekli statik
bloklardır. Yerinde korunmuş öğe başka statik blok kullanabilir; Reset
sonrası dizinin kaynak bloğuna döneceği mevcut modelin davranışıdır.
Kırpma, mimari kayıt, öznitelik/sembol/etiket/ek dosya, özel Y ölçeği,
parametre/öznitelik yeri, dinamik/dış referanslı/öznitelik tanımlı kaynak
veya farklı katman/görünüş açık ret alır. Böylece ileride oluşacak öğelerin
şablonu mevcut veri kısıtlarını çoğaltarak bozamaz. Tanımın iç geometrisi
yeniden yazılmaz; tanımlı bir statik iç blok var diye patlatılmaz.

Eksik yerlerin sayısı ya da yanlış aralık kendiliğinden tahmin edilmez.
Örneğin yalnız her ikinci yer doluysa gerçek aralık belirsizdir; bu API'ye
incelemeden kabul edilmiş açık yerleşim verilmelidir. Geometri uymuyorsa
`array_adoption_refused`, katman kilitliyse `layer_locked` döner. Girdi,
yol ve revizyon hataları mevcut ortak kodlarını korur. Ret atomiktir.

## Arka plan ve eşzamanlılık

Komut durdurulabilir native Job olarak çalışır. Üyelik denetimi mevcut
dizileri bir kez tarar, yalnız seçilmiş kimlikleri indeksler ve iptali
öğe başına kontrol eder; her adayda bütün dizileri yeniden taramaz.

Yazma `ArrayAdd` içerdiğinden mevcut `Document::apply_patch/rebase` kuralı,
belge revizyonu **veya generation** değişmişse eski işi birleştirmeyi
reddeder. Bu, geometri yazmayan bir işin eski pozu okuduktan sonra yanlış
metadata eklemesini de önler. Araya ilgisiz bir düzenleme girse bile yeni
inceleme gerekir; yalnız kaynaklara göre gevşek birleştirme iddiası yoktur.
Durdurulan/stale iş metadata, Undo ve mevcut nesneleri değiştirmez.

Komut özeti yalnız dizi kaydı değiştiğinde de gerçek Undo etiketini
raporlar; aynı düzeltme mevcut dizi düzenlemesinin metadata-only Reset'ini
kapsar. Diğer komutların plan parmak izleri değiştirilmez. Genel komut
günlüğünün dizi kimliklerini yeni çalıştırmaya eşlemesi CAD-ADV-20'de açık
iştir; doğrudan Python/MCP komutları ve istek tekrarıyla karıştırılmaz.

## Kabul kapsamı

- Native: planın yazısızlığı; yalnız metadata değişmesi; iki eksik yer,
  açıkça korunmuş öğe, iç parça ölçü referansı, sayı/aralık değiştirme,
  silme/reset, tek Undo/Redo ve dosya turu. Negatif aralıklı grid, iki
  kutupsal yön ve kapalı yolun açılmasını aynı UUID'lerle izleme.
- Bağımsız ortak komut durumları: kaynak/üyelik/görünüş/veri hataları,
  NaN/sonsuz aralık, kilit, revizyon, plan/execute ve Undo/Redo.
- Python: üretilmiş tipler, eski plan, istek tekrarının tek yazması,
  job iptali/çatışması, dosya eşitliği ve bağlı ölçünün izlemesi.
- MCP: gerçek plan beklentisi, dosya eşitliği, iki boş yer, ölçü referansı
  ve tek Undo/Redo; yerleşim değişince ölçünün gerçek koordinatları.
- Gerçek uygulama: dışarıda tutulan kabul çizimi ve görüntü kayıtlarıyla
  konsoldan native komut, ardından mevcut **Diziyi düzenle** aracı.
  Bu kontrol ayrı bir görsel örüntü bulma panelinin teslimi sayılmaz.
