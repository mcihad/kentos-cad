# ADR 0848 — Okunur blok parametre grupları ve tek değeri sıfırlama

Tarih: 2026-11-02. CAD-ADV-06'nın parametre paneli dilimi.

## Dayanak ve mevcut sınır

[BricsCAD Parameters Manager](https://helpcenter.bricsys.com/en-us/document/panels/parameters-manager-panel?version=V26)
parametreleri açılır/kapanır kategorilerde gösterir; hesaplanan değer ile
katalogdan seçilen değeri ayırır. [AutoCAD dinamik özellik rehberi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-2AD97498-3E30-46E7-A4B3-3AB469C1371A.htm)
tanım parametrelerinin yerleştirme özellikleri olarak gösterilmesini açıklar.
KentOS'ta bütün parametreler tek bölümdeydi; hepsini sıfırlama mevcutken
tek parametreyi sıfırlayan ortak komut panelden kullanılamıyordu.

## Davranış

- Dinamik yerleştirmede **Blok parametreleri** tanım adını, varsa mevcut
  `BlockCard.version` değerini ve örnek–tanım kapsamını açıklar. Kartı olmayan
  eski bloğa hayali sürüm numarası atanmaz.
- **Boyut ve konum** doğrusal/kutupsal/açı/nokta, **Görünüş** çevirme ve
  görünürlük, **Katalog** lookup, **Hesaplanan** formüllü parametrelerdir.
  Boş grup gösterilmez; her grup bağımsız daraltılır. Bu sunum sırasıdır:
  tanımdaki parametre veya eylem sırası değiştirilmez, dosyaya yazılmaz.
- Birimli değer, formül, kaynak notu ve min/max/adım aynı kalır. Notlar her
  yeni grupta tam satıra yayılır. Hesaplanan değer düzenlenemez.
- **Değerler** menüsü hem hepsini hem tek değişmiş parametreyi varsayılana
  döndürür. Örneğin yalnız genişlik sıfırlanınca Sade görünüşü korunur.
  Lookup sıfırlaması, kontrol ettiği adları ortak komutun kendi kuralıyla
  sıfırlar; UI bu kuralı yeniden hesaplamaz.
- Değişiklik sayacı kullanıcı girdilerini sayar; türetilmiş formül sonucu ve
  aynı girdilerle eşleşen katalog satırı ayrı override diye sayılmaz.
  Kilitli örnekte düzenleme/sıfırlama menüsü açılmaz; eski panel menüsünden
  gelen olay mevcut belge/seçim anahtarıyla reddedilir.

Bu gruplar mevcut parametre türlerinden türetilir. Kullanıcı tanımlı grup,
kalıcı tip parametresi veya seçilmiş örnekleri kitaplık sürümüne yükseltme
sözleşmesi eklenmedi. Tanım/örnek açıklaması bunların tamamlandığı iddiası değildir.

## Komut ve AI erişimi

Tek sıfırlama mevcut `cad.blocks.parameters` `operation: reset`,
`names: ["Genişlik"]` yolunu kullanır; lookup için `names: ["Tip"]` aynı
ortak kuralla çalışır. `names` verilmezse tümü. Python ve MCP aynı plan/
validate/execute ve tek Undo yoluna sahiptir. Şema/katalog/API değişmedi.
[Kullanım/API rehberi](../cad/block-parameter-groups.md).

## Kabul

1598 masaüstü testi geçti; mevcut 256 ignored test çalıştırılmış sayılmaz.
52 MCP testi, Python adlı/katalog sıfırlama testi, masaüstü derlemesi ve
desktop/MCP tüm hedeflerde Clippy geçti. Ek masaüstü kapsamı: grup ve tanım
sırası, salt okunur hesaplanan alan, bağımsız daraltma, tek/katalog sıfırlama,
diğer override'ların korunması, hatalı adın atomik reddi, kilitli/eski menü.

Gerçek GUI'de 1050 mm + Sade ayarlandı; yalnız genişlik 900 mm'ye sıfırlandı,
Sade korundu, formül 1800 mm'ye döndü. GUI önce/sonra kayıtları Python'un
aynı iki set ve adlı reset işlemleriyle tam aynı; tek GUI Undo önceki
dosyayla tam aynı. Dar pencere, bağımsız grup kapatma, açık/koyu tema ve
Büyük yazıda metin/menü kontrol edildi. Görüntüler 173–178 ve makinece kabul
`block-groups-gui-acceptance.json`, `/tmp/kentos-cad-adv04-2026-11-01/`
altında, repo dışında tutulur.
