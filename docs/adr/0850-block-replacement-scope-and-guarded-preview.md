# ADR 0850 — Blok değiştirmede kapsam ve korunan önizleme

Tarih: 2026-11-02. CAD-ADV-06.

## Dayanak

[BricsCAD BLOCKREPLACE](https://helpcenter.bricsys.com/en-us/document/command-reference/b/blockreplace-command?version=V26)
belirli yerleştirmeleri seçmeyi, benzerlerin tümüne uygulamaktan ayırır.
[AutoCAD blok değiştirme](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-5E78781D-AD09-4C6F-A3C0-05BD99CB410F.htm)
seçilmiş referanslarla çalışır. KentOS Bloklar menüsü ise hedef seçilir seçilmez
tanımın bütün doğrudan yerleştirmelerine yazıyordu; seçili tek kapı ile öbür
kapıların kapsamı görünmüyordu. ADR 0849'un koruma kuralının önüne açık kapsam
ve gerçek komut planı konur.

## Davranış ve sınır

- Bloklar › Bu çizim › tanımın sağ menüsü › **Başka blokla değiştir** hedef
  seçildikten sonra pencere açar; henüz çizime yazmaz. Kaynak/hedef adı ve
  varsa kartın mevcut sürüm sayacı görünür. Kart yoksa hayali sürüm üretilmez.
- Bu kaynak tanımın seçili yerleştirmeleri varsa **Yalnız seçili
  yerleştirmeler** varsayılan açıktır. Kapalıysa çizimdeki bütün doğrudan
  yerleştirmeler alınır. Hiçbiri seçili değilse bütün doğrudanlar seçilir ve
  boş seçim seçeneği pasiftir. Her iki UID listesi pencere açılışında dondurulur.
- Kapsam sayısı, tanımların içindeki değişmeyecek yerleştirme sayısı ve eski
  tanımın tutulacağı görünür. Bu pencerede silme veya iç içe kapsamı sessizce
  genişletme yoktur.
- `cad.blocks.replace` ortak `plan` çağrısı belgenin çalışma kopyasında ayrı
  iş parçacığında yürür. Gerçek değişen/bağlı kayıt sayıları, komut uyarıları,
  düşecek öznitelikler/özel konumlar ve hedef alan hesaplama sayısı gösterilir.
  Bağlı kayıt sayısı, komutun gerçekten değiştireceği kayıttır; etkilenmeden
  aynı kalan bir ölçünün sadece mevcut olması bu sayıya eklenmez.
- **Değiştir** yalnız tamamlanmış, değişiklik üreten bir planla açılır;
  `expected.plan` ile aynı plan execute edilir. Belge oturumu ve generation
  ayrıca kontrol edilir: başka belgeye veya değişmiş modele eski sonuç yazılmaz.
  Eski istek token'ı ve kapanmış pencerenin sonucu atılır.
- Vazgeç/Esc yazmaz. Kapatma veya kapsam değiştirme bekleyen sonucu iptal
  eder; çalışan ortak planın içi zorla kesilmez, işin önünde/sonunda iptal
  kontrol edilir. Bütün gövde kayar, uzun açıklamalar satıra yayılır.

Bu dilim **çizime zaten alınmış tanımlar arasında** kontrollü geçiştir.
Kaynak dosya/aile kimliği, harici kitaplıktan yan yana sürüm alma, kalıcı
tip–örnek parametre politikası ve özel grup metaverisi eklenmedi. Mevcut
kartın yerel değişiklik sayacı bir kitaplık yayın sürümü gibi yorumlanmaz.

## API ve kabul

Yeni protokol yoktur: Python/MCP `cad.blocks.replace` `uids` ile seçilmiş
kapsamı, validate/plan/execute, plan koruması ve tek Undo'yu zaten taşır.
GUI iz denetimi de kapsam onay/iptal düğmelerini adlarıyla denetler.
[Kullanım/API rehberi](../cad/block-replacement-parameters.md).

1603 masaüstü testi (256 mevcut ignored), tüm hedeflerde masaüstü Clippy ve
uygulama derlemesi geçti. Yeni dört test seçili/tüm kapsam, salt okunur plan,
iptal, uyumsuz hedef, eski sonuç, değişmiş belge ve başka belge oturumunu sınar.
Yeni iş kuralı eklenmediği için ADR 0849'un 53 MCP ve 50 ortak durum kabulü
geçerlidir. Bütün Python paketi 429 testle geçti; ilk geniş koşuda bulunan
69 eski etiket kurulum hatası ADR 0793 ekinde düzeltildi.

Gerçek GUI'de seçili 1 ve tüm 3 yerleştirme ayrı ayrı uygulandı. İki kayıt
da aynı planı uygulayan Python dosyalarıyla tam aynı; her iki işlemin tek GUI
Undo'su başlangıçla tam aynı. Esc/iptal ve pasif onaya tıklanan uyumsuz hedef
başlangıcı korudu. İç içe yerleştirme eski tanımda kaldı; dar pencere ve Büyük
yazı görüntüleri 184–187, kabul `block-replace-preview-gui-acceptance.json`;
hepsi `/tmp/kentos-cad-adv04-2026-11-01/` altında, repo dışındadır.
