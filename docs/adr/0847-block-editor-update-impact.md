# ADR 0847 — Blok düzenleyicisinde kayıt öncesi etki incelemesi

Tarih: 2026-11-02. CAD-ADV-06'nın bir dilimi; bütün madde tamamlanmadı.

## Dayanak

[AutoCAD: Modifying Block Definitions](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-BBD754F5-D51F-4692-8194-49E5B130CE68.htm)
tanım kaydedilince çizimdeki başvuruların birlikte güncellendiğini, dış
kitaplıktaki dosyanın değişmesinin ise kendi başına çizimi güncellemediğini
açıklar. [BricsCAD'in blok düzenleme rehberi](https://helpcenter.bricsys.com/en-us/document/bricscad/blocks-attributes-external-references-and-mechanical-blocks/editing-blocks-and-external-references?version=V26)
geçici yerel koordinatlı düzenleme oturumunu ve iç içe başvuruları ayrı ele alır.
KentOS mevcut taslak/normal Kaydet davranışını korur; değişiklik uygulanmadan
önce hangi kullanımların bu tanımı izlediğini görünür kılar.

## Karar ve kapsam

- Blok düzenleyicisinin **Etkiyi gör** düğmesi güncel, kaydedilmemiş nesnelerle
  mevcut `cad.blocks.edit/update` komutunun `CallOp::Plan` yolunu çalıştırır.
  Ortak protokol planı gerçek komutu bağımsız kopyada yürütür; bağlı kayıtların
  izlenmesi, retler ve uyarılar aynı hesaplamadan gelir. UI ikinci bir yazma
  veya ölçü/tarama/tablo güncelleme kuralı tanımlamaz.
- Pencere parça sayısını, doğrudan yerleştirmeleri, iç içe kullanımdan etkilenen
  üst seviye yerleştirmeleri, kilitli katmandakileri ve tanımı içeren blok
  adlarını gösterir. Her üst seviye yerleştirme bir kez sayılır; bir tanımın
  içindeki iki kapı iki üst seviye yerleştirme diye sunulmaz.
- Başvuru erişimi ile gerçek kayıt farkı ayrıdır. Bir Insert kaydı değişmeden
  tanımının yeni geometrisini çizer; **0 değişen kayıt**, **0 etkilenen görünüm**
  demek değildir. Kilitli katman da paylaşılan tanım değişikliğini yalıtmaz.
- Doğrudan bağlı ölçü, tarama, etiket, bağlı yazı ve kaynak tablolar için mevcut
  `copy_links::links_to` kullanılır. Blok çizelgelerinin okuduğu tanım adları
  ayrıca denetlenir. Bunlar potansiyel okuyucu sayılarıdır; gerçek değişen ve
  bunlar içindeki bağlı kayıt sayısı ortak `CommandSummary`'den gelir.
  Transitif bütün veri grafiği veya her iç parça için ayrı ölçüm raporu iddiası yoktur.
- Hesap UI iş parçacığında çalışmaz. Kapat/Esc incelemeyi bırakır ve sonucu
  iptal eder; taslak ve asıl çizim değişmez. İşçi ortak plan çağrısından önce/
  sonra ve erişim taraması sırasında iptal işaretini kontrol eder. Ortak
  planın iç hesabı zorla kesilmez; pencereyi kapatmak CPU'nun anında duracağı
  anlamına gelmez. Kaynak kopya işçi bitince serbest kalır.
- Her isteğin ayrı kimliği vardır. Kapalı pencereye, eski isteğe veya değişmiş
  taslağa gelen sonuç yeni rapora karışmaz. Taslak değiştiyse yeniden inceleme
  gerektiği yazılır; pencere sonsuza kadar yükleniyor durumunda bırakılmaz.
- Gövde gerektiğinde kayar, başlık/Kapat görünür kalır. Yalnız incelemedir;
  uygulama normal **Kaydet**, kaynak çizime dönme **Vazgeç** ile yapılır.
  İç içe öznitelik eşitleme kapsamı sessizce genişletilmez.

## Python, AI ve sürüm sınırı

Yeni kalıcı alan veya ürün komutu eklenmedi. Aynı yazma ve gerçek veri farkı
Python `cad.blocks.edit.answer(..., op="plan")` ve MCP `cad.blocks.edit`
`op: plan` ile erişilir. `summary` ve `planId` alınır; yazma `expected.plan`
ile aynı planı zorunlu tutabilir. UI'deki potansiyel başvuru sayıları masaüstü
sunumudur; ortak komut özetinin gerçek veri değişikliği sayılarıyla aynı adla
sunulmaz. [Kullanım ve API örneği](../cad/block-update-review.md).

Kitaplık tür/sürüm eşleme ve seçilmiş örnekleri yükseltme bu dilim değildir.
Web ara kararı sürer; gereken pencere TODOS.md §0.2'de tutulur.

## Kabul

1595 masaüstü testi geçti (mevcut 256 ignored test çalıştırılmış sayılmaz).
Yeni dört test: salt okunur gerçek plan/execute farkı; değişmeyen tanım ve
iptal/öz başvuru reddi; çok yollu iç içe başvuru, kilitli katman ve bağlı
ölçü/çizelge; kapanmış/yeni istek/geçersizleşen taslak sonucu.
51 MCP testi ve Python plan–korumalı yazma–tek Undo kabulü geçti;
masaüstü derlemesi ve desktop/MCP tüm hedeflerde Clippy geçti.

Gerçek fare/klavyeyle 60,450→300,450 mm kol çizildi. Rapor 4→5 parça,
3 doğrudan/2 iç içe/2 kilitli yerleştirme, 1 ölçü/1 tablo ve ortak hesapta
2 değişen bağlı kayıt gösterdi. Değişmemiş rapor, tek Esc, Kapat, dar pencere
ve normal Kaydet denendi. GUI kayıt dosyası Python'un aynı planla yazdığıyla
tam aynı; GUI tek Undo dosyası başlangıcın native kaydıyla tam aynı.
Görüntüler ve makinece kabul kaydı `/tmp/kentos-cad-adv04-2026-11-01/`
altında (169–172, `block-review-gui-acceptance.json`); Git'e görüntü eklenmedi.
