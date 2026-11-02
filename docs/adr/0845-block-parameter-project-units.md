# ADR 0845 — Blok parametreleri özellik panelinde proje birimiyle okunur

Tarih: 2026-11-02. Kapsam: CAD-ADV-06'nın masaüstü parametre paneli.

## Kanıtlanan sorun

`properties/rows/params.rs` doğrusal parametreyi üç basamaklı ham değerle,
her açıyı dereceyle gösteriyordu. `properties::commit` uzunluğu ham sayı,
açıyı her zaman derece kabul ediyordu. Milimetre projesinde aynı kapı üstte
900 mm boyutundayken Genişlik satırı birimsiz 0.900 gösteriyordu. Bu değer
900 olarak düzenlenince komuta 900 metre gidiyordu; varsa sınır kuralı
reddediyor, serbest parametrede çok büyük bir geometri oluşabiliyordu.

## Araştırma ve karar

[BricsCAD mekanik blok belgesi](https://help.bricsys.com/en-us/document/bricscad/blocks-attributes-external-references-and-mechanical-blocks/mechanical-blocks-and-mechanical-block-inserts?version=V27)
parametreleri özellik panelinden düzenlemeyi, fiziksel parametre biriminin
çizim biriminden çevrilmesini ve birimsiz parametrenin ayrı tutulmasını anlatır.
KentOS'taki mevcut sözleşme birebir aynı veri modeli değildir: doğrusal ve
nokta parametreleri tanımın yerel koordinatlarında, açılar radyandır.
[AutoCAD dinamik blok rehberi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-DidYouKnow/files/GUID-4005A6BB-388A-4EC7-BF17-555F1E6C137C.htm)
özellikten değişiklik, test yerleştirmesi ve sınırlı genişliklerle denemeyi
birlikte tarif eder. Buradaki değişiklik yalnız birim arayüzünü kapatır;
editörde ayrı test ortamı tamamlanmış sayılmaz.

- Fiziksel tanımın uzunluğu metre olarak kalır. Panel proje birimini gösterir;
  çıplak giriş o birimdedir, `105 cm` gibi açık birim de kabul edilir.
- `blocks_insert::unit_factor` tek kaynak olarak kullanılır. Birimsiz blok
  tanımındaki 0.9, mm projede 0.9 mm'dir; yanlışlıkla 900 mm'ye dönüştürülmez.
- Örneğin ilave ölçeği parametreye katılmaz. Üç kat ölçeklenmiş örneğin 900 mm
  tanım genişliği yine 900 mm'dir; gerçek görünen boyut ayrı boyut satırındadır.
- Açı proje ayarındaki derece/grad ile okunur, komuta radyan gider. Blok yerel
  noktalarına dünya koordinatı/UCS ötelemesi veya CBS semt yönü uygulanmaz.
- Yerel nokta/polar iki uzunluk olarak okunur. Ortak uzunluk ayrıştırıcısı,
  açık birimler, nokta için virgül/noktalı virgül/boşluk ayracı kullanılır.
  Tam sayısal bilimsel gösterim korunur; bilinmeyen birim son eki kesilip atılmaz.
- Formül sonucu salt okunur, lookup seçenekleri mevcut kanonik değerleriyle
  çalışır. Sınırlar hâlâ ortak komut tarafından uygulanır; başarısız giriş atomiktir.
- Python/MCP `cad.blocks.parameters` sözleşmesi, varsayılanlar, kayıt biçimi ve
  Undo değişmez; API'ye panelde görünen mm sayıları ham olarak gönderilmez.

## Doğrulama

Gerçek eski sürüm ekranı: yerel `159-block-parameter-mm-old-raw-value.png`.
Otomatik kabul; mm çıplak/açık birim girişi, sınır/bozuk girişte atomiklik,
tek Undo, mm/cm/m/in/ft fiziksel ve birimsiz tanımlar, bağımsız örnek ölçeği,
grad açı, yerel nokta, salt okunur türetilen parametre ve lookup içerir.
Son temiz aday: 1588 masaüstü ve 50 MCP testi, derleme ve Clippy geçti.
Gerçek GUI 900→1050 mm düzenlemesi Python sonucuyla bayt bayt aynı; tek
GUI Undo dosyası başlangıçla tam aynı. 2880×1730 ve 2204×1452 pencerelerde
birim ve giriş alanı görüldü (`160`–`162` numaralı yerel ekran kanıtları).
API validate/plan belgeyi değiştirmedi. Ekran görüntüleri Git
dışındadır: `/tmp/kentos-cad-adv04-2026-11-01/`.

Tür/sürüm, tip–örnek ayrımı, kullanıcı parametre grupları, iç içe güncelleme
etki önizlemesi, editörde test yerleştirmesi ve web eşitliği açıktır.
