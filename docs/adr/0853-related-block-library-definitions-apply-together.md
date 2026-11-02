# ADR 0853 — Bağlı blok tanımlarını birlikte uygula

Tarih: 2026-11-02. CAD-ADV-06.

## Sorun ve dayanak

[AutoCAD 2026 Blocks Palette](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-E3413A03-8EEF-44D5-B5C7-8B345A9257FE.htm)
kitaplık çiziminden blok tanımlarını da alır. Bu kaynak KentOS'un iç algoritmasını
belirlemez; bizim kitaplık güncellemesinin birbirine bağlı tanımları eksiksiz
alması gereğini uygulama kodu ve aşağıdaki hata tekrarı ortaya koyar.

`blocks_library::write` bütün değiştirmeleri yeni tanımlardan önce tek tek
uyguluyordu. Var olan ana blok yeni bir çocuk tanımını yerleştirdiğinde ortak
son-durum doğrulaması geçiyor, yazma ve bu yazmayı kopyada deneyen planlama
`block_refused` ile kalıyordu. Kitaplıktaki sırayı ters çevirmek bunu çözmüyordu.
Eski native Python bağı ve gerçek GUI konsolunda bu durum üretildi.

Bir diğer tekrar: iç içe çocuk parametresinin adı ve ana tanımdaki ona ait
örnek değeri birlikte değiştirilince son grafik geçerlidir. Yalnız çocuğu
önce değiştirmek eski ana tanımı; yalnız ana tanımı önce değiştirmek eski
çocuğu geçersiz yapar. Tanımları topolojik sıralamak yeterli değildir.
Yeni Python testlerinin bu üç alt durumu eski bağda başarısızdı.

## Karar

- `Document::merge_blocks` gelen tanım kimlik/adlarını ve mevcutlarla birleşik
  **son** grafiği mevcut ortak blok kurallarıyla doğrular. Aynı kimlik birden
  çok verilirse sessiz son-yazan-kazan uygulanmaz.
- Bütün denetimler geçmeden hiçbir geçmiş işlemi kaydedilmez. Bilinmeyen
  referans, döngü, ad/kimlik çakışması, bozuk parametre veya öznitelik konumu
  atomik reddedilir. Yeni bir kısmi-geçerli belge modu yoktur.
- Önceki tanımlar yerinde kalır, yeni tanımlar girdi sırasıyla sona eklenir.
  Tüm BlockUpdate/BlockAdd geçmiş işlemleri tek kayıtta uygulanır; türetilen
  bağımlı içerik son grafik üzerinden izlenir. Dış komutun işlem adı ve
  tek Undo/Redo davranışı korunur. Boş veya aynı tanım kümesi geçmiş yazmaz.
- `cad.blocks.edit` kitaplık yazımı ve `exchange_write::put_blocks` aynı
  yöntemi kullanır. İkincisi dosyadan blok alma/yeniden tanımlama ve mevcut
  çizimler arası alışveriş yollarının ortak yazıcısıdır. Önceki bekleme
  döngüsünün yazamadığı tanımları sessizce bırakması da kaldırılır.
- Yeni komut, dosya sürümü veya ayrı Python/MCP algoritması yoktur. ADR 0852'nin
  sahne değerlerini doğrulaması ve veri kaybı reddi korunur. Yeni eklenen
  çocuğa bağlı ana tanım artık salt ekleme sırası nedeniyle reddedilmez.

Bu, aile/yayın sürümü tanıyan yükseltme veya kalıcı tip–örnek parametre politikası
getirmez. Kullanıcının örnek değerleri kendiliğinden yeniden adlandırılmaz;
kitaplıktaki açık ve geçerli birleşik tanımlar uygulanır.

## Doğrulama

134 domain testi (1 ignored), 317 application (37 ignored), 56 MCP,
1603 masaüstü (256 ignored), 440 Python testi, Clippy ve masaüstü derlemesi
geçti. İki domain testi yeni çocuk/ana tanım sırasını, tek Undo/Redo,
geçersiz kümelerin kaynak/revision/Undo/Redo korumasını ve no-op'un bekleyen
Redo'yu bozmamasını sınar. Dört yeni Python testi kitaplık sırasını, eşzamanlı
iç içe parametre adını ve her iki dosyadan güncelleme durumunu kaydet–aç/tek
Undo/Redo ile sınar. MCP aynı kitaplık durumlarında salt okunur plan,
plan kimliğiyle execute, kök örnek verileri, yeniden açma ve tek Undo'yu sınar.

Gerçek eski GUI konsolunda plan hatası, sağ tuş **Başka çizimden yeniden
tanımla** yolunda geçerli kaynakla parametre hatası üretildi. Yeni derlemede
aynı menü/aynı kaynak geçti; iç içe örnek 1.0 değeri yeni Açıklık adıyla kaldı.
GUI kayıtları bağımsız Python sonucuyla tam aynı; gerçek klavye ile tek Undo
kayıtları başlangıçla tam aynı. Yeni çocuk ekleyen kitaplık plan/execute'si
GUI konsolunda ayrıca geçti ve kayıt/tek Undo eşitliği temiz tekrarda doğrulandı.
Araya farklı parametre işlemleri giren ilk ekleme kaydı kabul kanıtı dışındadır;
sonuç buna göre esnetilmedi. Menünün başarılı işleminde geçmiş kartının
“Çizimde değişiklik yok” demesi ayrıca izlendi; tanım değişimleri doğru kaydedildiği
hâlde bu özet yanlıştır, bu dilimde düzeltilmiş sayılmaz.

Ekranlar 200–203/206, `block-library-batch-gui-menu-acceptance.json`,
`block-library-batch-gui-library-acceptance.json` ve koşu günlükleri depo dışı
`/tmp/kentos-cad-adv04-2026-11-01/` dizinindedir. Bu kabul CAD-ADV-06'nın
kalıcı tip–örnek/aile/yayın kapsamını tamamlamaz.
