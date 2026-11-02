# ADR 0852 — Kitaplık değiştirmede sahne yerleştirmelerini de doğrula

Tarih: 2026-11-02. CAD-ADV-06.

## Sorun ve dayanak

[AutoCAD blok tanımını değiştirme](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-MAC/files/GUID-FA75897D-71EA-4929-A926-F0E1D7524986.htm)
aynı tanımın bütün referanslarını etkiler. KentOS'un `cad.blocks.edit`
`operation: import, replace: true` kitaplık yenilemesi tanımların son hâlini
ve iç içe referansları denetliyor, sahnedeki doğrudan referansları denetlemiyordu.

Hata mevcut `dynamic-blocks.kcad` ile eski native Python bağında üretildi:
kitaplıkta dinamik davranış kaldırılınca import tamamlanıyor; sahnenin ikinci
yerleştirmesinde kalan parametreler nedeniyle sonraki KCAD kaydı
`file_unwritable` oluyordu. Aynı hata, bir sınırın mevcut örnek değerinden
daha daraltılması ve özel konumu olan özniteliğin artık taşınamaz yapılmasıyla
da üretildi. Dört yeni Python kabul testinin üçü eski bağda başarısızdı;
uyumlu güncelleme kontrolü zaten başarılıydı. `file.import.block`
`mode: blocks, replace: true` de eski bağda aynı uyumsuz kaynağı validate
aşamasında kabul ediyordu; kaynak dosya bu kez geçerli düz tanımlı KCAD idi.

## Karar

- İçe alınan ve mevcut tanımların birleşik son hâli ortak
  `kentos_commands::blocks::check_of` ile doğrulanır. Sahnedeki doğrudan
  referanslar, parametreleri ve özel öznitelik konumları da bu kapsamdadır.
- Mevcut tanım denetiminin sırası ve hata kodu/ileti eşlemesi korunur;
  yalnız bu çağrıda sahne listesi eklenir. Diğer tanım düzenleme yollarına
  yeni bir kendiliğinden eşitleme politikası uygulanmaz.
- `file.import.block` birleştirilmiş son tanımları da aynı kuralla denetler;
  kendi mevcut katalog sözleşmesi gereği `block_refused`, yol `path` verir.
  Dosya okuma, kategori ve mevcut sürüm kontrolü sırası korunur.
- `validate`, `plan` ve `execute` aynı denetimi yapar. Uyumsuz parametre
  `invalid_parameter_value`, artık taşınamayan özel öznitelik konumu
  `attribute_not_movable`, yol `library` ile reddedilir.
- Retten önce hiçbir tanım, yerleştirme veya Undo adımı yazılmaz. Özel
  değer/konum sessizce silinmez veya yeni varsayılana çekilmez. Kaynak
  çizim kaydedilebilir kalır.
- Uyumlu yeni geometri, mevcut yerleştirme verilerini koruyan normal
  kitaplık işlemiyle uygulanır; bütün değişim tek Undo'dur. Plan kimliğiyle
  koruma mevcut ortak protokoldür, yeni komut veya format yoktur.

Bu koruma `cad.blocks.edit` kitaplık import ve `file.import.block` dosyadan
yeniden tanımlama yolundadır. Yayın sürümü tanıyan gelişmiş yükseltme bu
dilimle tamamlanmış sayılmaz. Değişen formül, birimsiz/fiziksel
semantik geçişi ve tip–örnek politikası ayrıca ele alınmalıdır. Tanımın
geçerli olması tek başına bu ileri davranışların kanıtı değildir.

## Doğrulama

317 application (37 mevcut ignored), 1603 masaüstü (256 ignored), 55 MCP
testi, Clippy ve masaüstü derlemesi geçti. Dosya yolunun eklenmesinden
sonra 317 application testi ve iki yolu içeren hedef MCP testi ayrıca
geçti. Beş yeni test dahil 436 Python testi geçti. Gerçek GUI Python
konsolundan kitaplık plan/execute reddi ve dosyadan yeniden tanımlama
reddi görüldü; arayüzden kaydedilen ret sonrası KCAD başlangıçla bayt
düzeyinde aynı. Uyumlu güncelleme üç kapıda görüldü, GUI kaydı bağımsız
Python sonucuyla tam eşleşti; gerçek klavye ile tek Undo sonrası kayıt
başlangıca tam döndü. Güncel kayıt yeniden kodlandığında da aynı.
Ekranlar 196–199 ve `block-library-safety-gui-acceptance.json` depo dışı
`/tmp/kentos-cad-adv04-2026-11-01/` dizinindedir. Yeni Python testleri üç
uyumsuz durumda validate/plan/execute reddi ve bayt düzeyinde kaynak
korumasını; uyumlu durumda salt okunur plan, plan kimliğiyle execute,
örnek verileri, kaydet–aç ve tek Undo'yu sınar. Beşinci Python testi gerçek geçerli KCAD kaynağından dosyayla yeniden
tanımlamanın üç aşamada atomik reddini denetler. MCP testi iki import yolunu
açık çizim/dosya üzerinden ve uyumlu güncelleme/tek Undo ile sınar.
