# ADR 0844 — Blok özniteliğini yeniden adlandırırken örnek değerlerini koru

2026-11-02. CAD-ADV-06'nın ilk diliminin kabulü tamamlandı; madde bütünü açık.

## Araştırma ve gerçek hata

[AutoCAD ATTSYNC](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FCIV3D%2F2014%2FENU%2FfilesACD%2FGUID-56B14079-250B-4C99-AB3D-F95BA1C32AB7-htm.html)
öznitelik tanımı eşitlenirken mevcut örnek değerlerini korumayı anlatır.
[Test Block](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FACDLT%2F2014%2FENU%2Ffiles%2FGUID-B2732F5A-121D-4C33-8429-A53342003A53-htm.html)
kaydetmeden önce blok davranışını denemeyi önerir.
[BricsCAD design tables](https://helpcenter.bricsys.com/en-us/document/bricscad/parametric-modeling/working-with-design-tables?version=V26)
katalogdaki yapılandırma ile örneğin parametre değerini ayırır. Bu dilim
öznitelik kimliğindeki veri kaybını düzeltir; yeni bir design-table sistemi değildir.

Masaüstündeki `block_attributes::Window` tanımı yeniden yazıyordu ama mevcut
`cad.blocks.edit.renames` alanını doldurmuyordu. `NO` satırını `KOD` yapınca
`blocks_sync::sync`, NO'yu kaldırılmış etiket sayıyor ve R-1/R-2/R-3'ü
siliyordu. Gerçek eski GUI'de üç örnek birden VARSAYILAN gösterdi; kaydedilen
karşılaştırma dosyasından NO değerlerinin de silindiği bağımsız okundu.

Ayrıca açık API'de A→B, B→A işlemi sırayla remove/insert yapıyordu; ilk
hedefe yazılan değer sonraki adımın kaynağını eziyordu. Bu, kullanıcı
arayüzünden bağımsız Python/MCP veri kaybıydı.

## Uygulama

- Her düzenleyici satırı, açıldığı tanımdaki kaynak etiketi saklar. Yeni
  satırın kaynak etiketi yoktur. Kimlik satır numarasından tahmin edilmez;
  ekle/sil, konum seçme ve bir etiketi birden çok kez yazma sırasında korunur.
- Geçerli yeni etiket farklı ve sabit olmayan bir özniteliğe aitse eski→yeni
  eşlemesi mevcut komuta hem validate hem execute sırasında verilir.
  Yeni varsayılan eski örnek değerini ezmez; Undo tanım ve değerleri birlikte alır.
- Ortak native `blocks_sync` bütün kaynak değer/konumlarını yazmadan önce
  okur. Yeniden adlandırmalar eşzamanlı uygulanır; döngü ve zincirler
  BTreeMap sırasından etkilenmez. Değişmeyen etiketler ve görünüş verisi korunur.
- Tanım düzenlemesinde yeni hedefte eşlemenin kaynaklarından olmayan başka bir değer/özel konum
  varsa `duplicate_tag` ve yerleştirme numarasıyla atomik reddedilir. Veri
  üzerinde örtülü seçim yapılmaz. Aynı değerin tekrar taşınması veri kaybı
  oluşturmaz. Önceden yapılmış eşlemeler de ret halinde yazılmaz.
- Yerleştirme UID'si ve blok kimliği korunur. Katman kilidi, field, sabit
  öznitelik ve mevcut senkronizasyon kuralları yeniden kurulmaz.
- Arayüz doğrudan çizim yerleştirmelerinin eşitlendiğini, iç içe tanımların
  ve kilitli örneklerin aynı kapsamda olmadığını açıklar. Sessiz özyinelemeli
  güncelleme eklenmez. Bu kapsamı genişletme ve etki önizlemesi CAD-ADV-06'da açıktır.
- Mevcut açık `operation=sync` göçü hedefteki değerin yerine kaynaktakini
  koyabiliyor; 0317'nin yayımlanmış sözleşmesi korunur. Yeni çakışma koruması
  tanımın öznitelik listesini değiştiren `attributes/update` içindir. Tam
  paylaşılan fixture kabulünde bu ayrım ayrıca doğrulandı.
- Yeni kalıcı alan, KCAD sürümü, komut veya dış bağımlılık eklenmez.

## Kanıt ve sınırlar

`fixtures/interaction/v1/block-attribute-identity.kcad` üç kullanıcı kodu ve
üç farklı tip değeri olan 2.3 KB kabul çizimidir; ekran görüntüsü değildir.
Python kabulü normal rename, çakışmanın validate/plan/execute atomik reddi,
ikili değer/özel yazı konumu değiş tokuşu, kaydet/aç ve tam tek Undo içerir.
MCP aynı GUI çizimiyle planın salt okunurluğu, üç kod/tip/UID, yeniden açma
ve tam Undo denetler. Masaüstü testinde tekrar yazılan etiket, eklenen ve
silinen yeni satır ile aynı kayıtlı komut yolu sınanır.

Temiz adayda 1584 masaüstü testi (mevcut 256 ignore), 317 application
bütünleşme testi (37 ignore; 14772 paylaşılan komut vakası dahil), 50 MCP
testi ve dört Python rename testi geçti. Native/interaction/desktop/MCP
tüm hedef Clippy, masaüstü/Python derlemesi ve bağımsız KCAD doğrulaması geçti.
İlk tam paketin ortaya çıkardığı eski açık sync hedef değiştirme davranışı
korundu; paylaşılan vakalar yeni beklentiye çevrilerek geçilmiş sayılmadı.

Gerçek Mac GUI'de eski sürümde NO→KOD değerleri sildi; son temiz derlemede
aynı fare adımları R-1/R-2/R-3 ve Beton/Çelik/Polimer değerlerini korudu.
GUI sonucu Python sonucuyla bayt bayt aynı. Tek Undo başlangıç baytlarını
tam getirdi. Yeniden açılan dosyada KOD satırı ve üç kullanıcı kodu kaldı;
dar/geniş pencerede kapsam açıklaması ve eylemler okunur görüldü.

Başka KOD değeri bulunan örnekte NO→KOD yazmak hangi yerleştirmede çakışma
olduğunu gösterdi ve Kaydet'i kapattı. KOD_YENI seçmek hatayı kaldırdı;
Vazgeç ve farklı kaydet sonrası dosya başlangıçla bayt bayt aynı kaldı.
Ekranlar repo dışında `154-…158-*.png`; sonuçlar
`/tmp/kentos-cad-adv04-2026-11-01/block-rename-gui-acceptance.json`,
`block-rename-final-results.json`, `block-rename-clean-application-fixed.log`
dosyalarındadır. [Python/MCP rehberi](../cad/block-attribute-renames.md).

Bu dilim CAD-ADV-06'yı kapatmaz. Bileşen tür/sürüm yükseltme kapsamı,
parametre grupları, tip–örnek alanları, önizleme, editörde test yerleştirmesi
ve iç içe güncelleme kabulü açıktır. Eski web `blocksEdit` ve öznitelik
penceresi, native ATTSYNC/rename kapsamını bütünüyle uygulamıyor; bu teslim
webde aynı yeteneklerin var olduğu iddiası değildir ve bu eksik ayrıca
CAD-ADV-06 kapsamındadır.
