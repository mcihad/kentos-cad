# ADR 0863 — Bloklarda tip ve örnek parametre kapsamı

Tarih: 2026-11-03. CAD-ADV-06. Bu dilimin native/Python/MCP kabulü
tamamlandı; CAD-ADV-06 bütünü birleşik kabul ve iç içe öznitelik kapsamı için açıktır.

## Dayanak ve mevcut model

[Autodesk Family Types](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-Customize/files/GUID-7C0C2F22-8E85-4576-BD84-D710AC6D4B6F.htm)
parametre grupları, değer ve hesaplanan formülü ayrı sunar; Apply/Cancel
ortak tip düzenleme kapsamını açık eder.
[BricsCAD Parameters Manager](https://help.bricsys.com/en-us/document/panels/parameters-manager-panel)
girdi ile hesaplanan salt okunur değeri ve tasarım tablosunu ayırır.
Bunlar kullanıcı deneyimi dayanağıdır; 3B aile modelini 2B uygulamaya
aynen taşımıyoruz.

Mevcut `DynamicBlock` başlangıç noktaları eylemlerin fark ölçtüğü
geometridir. Bunları ortak değer gibi değiştirince eylemin farkı kaybolur.
Yayın `familyId/typeId/release` ise dağıtım kimliğidir; aynı tipin eski ve
yeni yayınlarının çizimde bağımsız kalması gerekir. Tip kapsamı bir
çizim `BlockId`'sine aittir; yayın kimliğine göre küresel bağ kurulmaz.

## Sözleşme

- İsteğe bağlı `typeParameters: string[]` gerçek skaler parametre adlarını
  belirtir; `typeValues: map<name, ParamValue>` her biri için ortak girdiyi
  veya hesaplanan önbelleği taşır. Tip kapsamı yoksa alanlar yazılmaz.
  Mevcut parametre/eylem sırası ve `parameterGroups` korunur.
- Lookup kendi skaler değeri olarak saklanmaz. Bütün satır hedeflerinin
  kapsamını alır. Aynı tabloda tip ve örnek hedefi karıştırılamaz.
- Ortak değer adları kapsamla birebir eşleşir; türü, görünürlük seçeneği
  ve sayı sınırları mevcut ortak kurallardan geçer. Örnek haritasında tip
  adı saklanamaz. Panel ve grip bu alanı örnekten değiştiremez.
- Ortak formül yalnız ortak sayısal girdileri okuyabilir. Örnek formülü
  ortak girdiyi okuyabilir. Gerçek ifade lexer'ı ve motorun Türkçe ad
  karşılaştırması kullanılır; ADR0861 bağımlılık/döngü kuralları korunur.
  Ortak hesap sonucu tanımda, örnek hesap sonucu örnekte tutulur.
- Ortak girdi değişince boş kökler ve değişmeyen üst tanımların iç
  yerleştirmeleri de kendi literal girdileriyle yeniden hesaplanır.
  Yeni yerleştirme açık `parameters` verilmeden de formülleri hesaplar.
- Tip kapsamına geçiş, doğrudan ve tanım içi bütün eski etkin değerlerle
  karşılaştırılır. Tek bir uyuşmazlık bile varsa işlem veri yazmadan
  reddedilir. Kullanıcı değerleri eşitler veya ayrı çizim tipi oluşturur.
  Genişlikler sessizce tek değere indirgenmez.
- Örneğe dönüş, eski ortak literal değeri her kök ve tanım içi kayda
  aktarır. Yeni örnek formülü varsa kendi girdilerinden hesaplanır.
  Ortak alanı ve değerini birden kaldırıp veri kaybettirmek reddedilir;
  önce kapsamın örneğe geçirilmesi gerekir.
- Ortak kapsam/değer değişikliği kilitli bir kök veya kilitli üst blok
  yerleştirmesini etkiliyorsa bütün işlem reddedilir. ADR0855'in yalnız
  hesap önbelleği bakım politikası bundan ayrıdır. Saf grup düzenleme ve
  gerçek no-op kilide takılmaz.
- Sıradan blok değiştirme/yayın yükseltmesi: tip→tip hedefin ortak
  değerini benimser; tip→örnek eski ortak literal girdiyi korur;
  örneğe özel literal→farklı ortak hedef atomik reddedilir. Formül
  önbelleği kullanıcı girdisi gibi taşınmaz. Eski yayın tanımı korunur.

## Yüzeyler ve kayıt

Ayrı ikinci bir geometri komutu eklenmez. GUI, Python ve MCP aynı
`cad.blocks.edit(operation=dynamic)` validate/plan/execute yolunu ve
revizyon/plan korumasını kullanır. `cad.blocks.parameters` ortak değerleri
get ile gösterir; set/named-reset ortak adları reddeder; all-reset yalnız
örnek değerlerini sıfırlar. Komut çıktısı ve tek Undo bütün tanım/grafik
kaydını birlikte kapsar. Native arayüz kabulü web kabulü değildir;
web düzenleme yüzeyi TODOS0.2 kapsamında ayrıca doğrulanacaktır.

KCAD160 isteğe bağlı iki alanı kapılar; ortak parametresi olmayan dosyalar
önceki minimum şemada, önceki baytlarıyla kalır. Bağımsız Python okuyucu ve
yazıcı aynı kapıyı, değer kurallarını ve yeniden yazımı denetler.
FORMATS106 ve üretilen komut/SDK sözleşmesi aynı alanları bildirir.
Ortak tip değeri taşıyan kitaplık sürüm 3 olarak yazılır; eski okuyucu onu
reddeder. Ortak metadata'yı sürüm 1/2 diye getiren import ve yayın yükseltme
yazısız reddedilir. Ortak değer taşımayan kitaplıklar önceki 1/2 sürümünü
korur; bağımlı tanımlar da minimum sürüm hesabına girer.
Birim dönüşümü ortak sayısal/nokta değerlerine mevcut tipli adaptörü
uygular; açı, bayrak ve görünürlük uzunluk gibi ölçeklenmez. Statik DXF,
çizim, sınır/seçim ve iç içe açılım aynı çekirdek etkin değerini kullanır;
düzenlenebilir davranış kaybı raporu devam eder.

## Arayüz kararı

Bloklar sağ menüsünde ve panelin Değerler menüsünde tek **Tip değerleri…**
girişi vardır. Pencere her parametrenin Örnek/Tip kapsamını ve ortak değerini
birlikte sunar; lookup kapsamı hedeflerinden gelir; ortak katalog satırı seçimi bütün
hedeflerin ortak değerini birlikte atar, lookup adı saklanmaz. Hesaplanan değer
salt okunurdur. Proje uzunluk/açı birimi kullanılır; değiştirilmemiş alan
tekrar sayıya çevrilerek hassasiyet kaybettirilmez. Mevcut tip adı sırası
korunur. Doğrudan ve tanım içi kullanım sayıları ile global değişim etkisi
kaydetmeden önce açıklanır. Kaydet revizyonu yeniden denetler; Vazgeç/Esc
yazmaz; alt eylemler kaydırılabilir gövdeden ayrı kalır.

## Doğrulama ve kabul

Doğrudan/iç içe çakışma, kapsam geçişleri, kilitli üst kullanım, ortak ve
örnek formülleri, lookup kısıtı, boş örnekte gerçek geometri/grip, birim,
kitaplık değiştirme/yayın bağımsızlığı, KCAD ve statik DXF kabul edildi.
1164 contracts, 316 çekirdek, 234 KCAD, 356 application, 64 MCP, 1619
masaüstü ve 472 Python testi geçti; mevcut ignore'lar korunur. Tüm ilgili
hedeflerde Clippy, SDK/katalog üretim denetimi ve masaüstü derlemesi geçti.
Bağımsız üretici 1508 kayıt dosyasını yeniden üretme eşitliğiyle denetledi.

Gerçek GUI'de 1.1→1.2 m ortak değişim üç kapıyı birlikte değiştirdi;
çizim odağında tek Undo/Redo ve kaydet–aç dosyaları beklenenle tam aynı.
Örneğe dönüş her kök ve iç kayda 1.2 m yazdı. Bir örnek 1.15 m iken tekrar
ortak tipe geçiş o yerleştirmeyi söyleyerek atomik reddedildi. Dar pencere,
iptal, ortak salt okunur panel ve ortak grip bulunmaması görüldü.

Milimetre projesinde 1100 mm, katalog Dar→700 mm ve üç kapının gerçek
geometrisi kontrol edildi. Değişikliksiz Kaydet, Esc/Vazgeç ve gerçek
Undo başlangıcı bayt bayt korudu; kayıtlar korumalı Python komutuyla tam
aynı. Son ekran tekrarında 1300 mm reddi ve adım uyuşmazlığı sınandı.
Ekranda görülen ham kanonik hata sayısı ve geçersiz girdi sırasında yanlış
lookup kapsam etiketi de giderildi: sınır/adım proje biriminde ve parametre
adıyla açıklanır; ortak katalog satırı hatalı sayıyı da düzeltebilir.
Son derlemede fare/klavyeyle 1200 mm kayıt ayrıca Python ile tam eşleşti.

Kanıtlar depo dışındaki `/tmp/kentos-cad-adv04-2026-11-01` dizinindedir:
`block-type-final-results.json`, `block-type-validation-final-results.json`,
`block-type-gui-acceptance.json`, `block-type-sdk-gui-acceptance.json`,
`block-type-mm-gui-acceptance.json`, `block-type-final-mm-gui-acceptance.json`.
265–280 ekranları ve test derlemelerinin hash'leri aynı dizindedir;
ekran görüntüsü veya derleme repo içine eklenmez.

Birleşik kabul çizimi, mevcut öznitelik yeniden adlandırmanın yalnız
çizim köklerini eşitlediğini de doğruladı. İç tanımlardaki NO→ODA göçünü
sessizce yaymak yerine açık kapsam ve etkiyle ele almak gerekir. Bu
ADR tip parametresi dilimini tamamlar; o öznitelik kapsamını ve CAD-ADV-06
birleşik kabulünü tamamlanmış saymaz.

## Kitaplık sürümünün son kabulü

Gerçek eski GUI çıktısı ortak `typeValues` taşırken sürüm 2 yazıyordu.
Son GUI aynı tanım gövdesini sürüm 3 yazdı; çıktı Python `export` komutuyla
karakter karakter aynı. Gerçek menüden sürüm 3 alındı; gerçek eski sürüm 2
çıktısı uyarı gerekçesiyle reddedildi. Bu işlemler, minimum pencerede beş
parametre/formül kaydırması ve Esc sonrası CAD kaydı başlangıçla bayt
bayt aynı kaldı. 284–288 ekranları ve
`block-type-library-gui-acceptance.json` bu son derlemenin kanıtıdır.

Sürüm koruması sonrasında 14801 paylaşılan komut vakası dahil 356
application, 1619 masaüstü, 64 MCP testi ve tüm ilgili hedeflerde Clippy
tekrar geçti. Python derlemesinden sonra 53 blok ve 9 katalog testi
geçti; bunlar sürüm 3 normal alımı, yalnız çocuk tanımda ortak değer varsa
sürüm 3 seçimini, eski değersiz yayının sürüm 2 kalmasını ve iki import
yolunda sürüm düşürmenin validate/plan/execute atomik reddini içerir.
Önceki tam 472 Python paketi bu hedefli son tekrardan ayrı kayıttır.
Paylaşılan desteklenmeyen kitaplık sürümü vakası artık 4'ü sınar;
3'ün yeni desteği hata beklentisiyle çelişmemelidir. SDK/katalog üretimi,
son masaüstü derlemesi ve `git diff --check` geçti. Son sonuçlar
`block-type-library-final-results.json` içindedir.
