# ADR 0860 — Kutupsal formül sonucu ve bütün yerleştirmelerde hesap yenileme

Tarih: 2026-11-03. CAD-ADV-06; ADR 0318, 0855 ve 0859'un devamı.

## Doğrulanan sorun

Masaüstünün gerçek Python konsolundan kutupsal `Formüllü uç` parametresine
`@Genişlik * 2` eklendiğinde komut “Tamamlandı” diyordu. Kayıtlı sonuç
`2.1` sayısıydı; kutupsal parametrenin sözleşmesi `{x, y}` noktasıdır.
Gerçek **Farklı kaydet** ve kurtarma kaydı `file_unwritable` ile başarısız
oldu. Tek Undo sağlıklı dosyaya bayt düzeyinde geri döndü. Bunun yanında
normal dinamik tanım düzenlemesi parametre haritası boş kök yerleştirmeyi
atlıyor, başka tanımların iç yerleştirmelerini yenilemiyordu.

[AutoCAD BPARAMETER](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-E4460179-F0FC-4AC6-85D7-35B05D018D16.htm)
ve [BricsCAD BPARAMETER](https://help.bricsys.com/en-us/document/command-reference/b/bparameter-command?version=V26)
kutupsal parametreyi iki nokta arasındaki uzaklık ve açıyla tanımlar.
[BricsCAD boyutsal kısıtları](https://help.bricsys.com/en-us/document/bricscad/drawing-accurately/using-dimensional-constraints?version=V27)
uzunluk ve açı ifadelerini ayırır. Bu kaynaklar KentOS'un JSON noktası
veya formül sözdizimini kullandığı iddiası değildir; aşağıdaki uzaklık
formülü kararı KentOS'un mevcut `Polar.rule` uzaklık sözleşmesinden gelir.

## Karar

- Kutupsal parametrenin sayısal `rule.expression` sonucu **uzaklıktır**.
  Nokta `from + normalize(to-from) * uzaklık` olarak saklanır. Tanımdaki
  başlangıç ve yön korunur. İfade bir X/Y koordinat çifti ya da yeni bir
  açı üretmiş sayılmaz. Ayrı açı kontrolü yeni bir sözleşme gerektirir.
- Negatif uzaklık, sıfır/sonlu olmayan tanım yönü ve sonlu olmayan sonuç
  reddedilir. Son nokta parametrenin normal değer/kural denetiminden de
  geçer. Doğrusal/açı formüllerinin sayı sonuçları korunur.
- Formüllü parametrenin tanımını veya formülün gerçekten okuduğu varsayılan
  girdiyi değiştiren `cad.blocks.edit(operation="dynamic")`, kitaplık
  güncellemesindeki tam grafik yenilemesini kullanır. Parametre haritası
  boş kökler ve değişmemiş üst tanımların iç yerleştirmeleri de hesaplanır.
- Eski hesap önbellekleri girdi sayılmaz. Mevcut literal girdiler korunur;
  yeni formül/kural bir örneğin girdisiyle hesaplanamıyorsa hiçbir şey
  yazılmadan ret verilir. Hesap hatasında `unwrap_or_default` ile genişlik,
  yön veya görünüşün silinmesi kabul edilmez. Yeni türetilmiş parametreye
  dönüşen eski literal override da sessizce atılmaz; açık ret alır.
- Kilitli katmanda kaynak tanım güncellemesi için ADR 0855'in politikası
  sürer: literal girdiye dokunmadan hesap önbelleği yenilenir. Bu,
  kilitli yerleştirmenin parametrelerinin kullanıcı tarafından doğrudan
  düzenlenmesine izin vermez. Boyutu yeniden yorumlayan birim düzeltmesinin
  ADR 0859'daki kilit politikası ayrıdır.
- Birincil tanım, değişen üst tanımlar ve kök hesap kayıtları aynı işlemde
  yazılır. `changed` üst tanımları da listeler. `sync.inserts` kökteki
  değişen kayıtları sayar; iç kullanım sayısı gibi sunulmaz.
- Yalnız sunum grubu değişimi yeniden hesaplatmaz. Formül parametresini/okuduğu varsayılanı değiştirmeyen sınır, etiket veya
  eylem düzenlemesinin ve tüm dinamik bölümü kaldırmanın mevcut uyarlama
  politikası bu dilimde genelleştirilmez. Yeni KCAD şeması
  veya komut sürümü yoktur. Web handler'ının eşitliği ayrıca ele alınır.

## Kabul

Gerçek masaüstünde aynı komut eski sürümde kayıt hatası üretti; düzeltilmiş
sürümde `Formüllü uç = 2.1, 0` noktasıyla Farklı kaydet başarılı oldu.
Panelde genişlik 1.05→1.10 m tek Enter ile değiştirildi; sonuç 2.2, 0 oldu.
Kaydet–aç–kaydet, formül eklemenin tek Undo/Redo'su ve ölçünün tek Undo'su
bayt düzeyinde doğrulandı. Örnekte sıfıra bölen yeni formül gerçek konsolda
reddedildi; önceki genişlik, formül ve dosyanın bütün baytları korundu.
GUI'nin ekleme ve ölçü dosyaları korumalı Python komutlarıyla tam aynı;
10 kanıt dosyası bağımsız okuyucu ve native yeniden kayıtla doğrulandı.
Ekran görüntüleri 246–250 ve kabul kayıtları depo dışındadır.

Native testler boş kök/override, açılı başlangıç vektörü, iç yerleştirme,
uyumsuz örnekte atomik ret, negatif/taşmış sonuç, formül kaldırma, kilitli
katmanda literal koruma, okunan varsayılan değişimi ve birim dönüşümünü
kapsar. Gerçek çekirdek varyantında hesaplanan noktanın Move eylemi doğru
geometriyi üretir ve hesaplanan parametreye tutamaç çıkmaz. Python ve MCP
aynı plan/doğrulama/yürütme yolunu, KCAD kaydını ve Undo'yu kullanır.

339 application + ayrı çekirdek çizim testi, 1612 masaüstü, 62 MCP ve 462
Python testi geçti. Önceki paylaşılan 14801 komut senaryosu uyumluluğu,
Clippy (Python dahil) ve masaüstü derlemesi geçti. Gerçek GUI ikilisinin
SHA-256 değeri dış kabul kaydında saklanır; kanıt görselleri Git'e eklenmez.
