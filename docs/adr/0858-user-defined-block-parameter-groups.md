# 0858 — Kullanıcının tanımladığı blok parametre grupları

Tarih: 2026-11-03. Kapsam: CAD-ADV-06'nın kalıcı sunum grupları.
Bu karar tip–örnek parametre kapsamının tamamlandığı anlamına gelmez.

## Gerekçe ve kaynaklar

ADR 0848 dört otomatik başlık sağlar; bir üretici veya ofis kendi
“Kanat”, “Kasa” ve “Ürün bilgileri” başlıklarını kaydedemiyordu.
[Revit parametre yerleşimi](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-Customize/files/GUID-1E6281BF-3194-40DB-8590-A60767E03FF1.htm)
parametrenin tip/örnek kapsamını ve özellikler penceresindeki başlığını
ayrı kararlar olarak ele alır. KentOS'ta da gruplandırma yalnız sunumdur;
bir gruba “Tip” adı vermek parametreyi bütün örnekler için ortak yapmaz.
[AutoCAD dinamik blok tutamakları](https://help.autodesk.com/cloudhelp/2027/ENG/AutoCAD-Core/files/GUID-31E1AD7C-387A-46B5-BCFD-6DD2173747C3.htm)
özellik panelindeki düzenlemeyi sahne tutamaklarından ayrı sunar.
Bu dilim tutamakların yerini veya görünürlüğünü değiştirmez.

## Sözleşme

`DynamicBlock.parameterGroups?` sıralı `{name, parameters}` dizisidir.
Üyeler mevcut parametrelerin tam adlarıdır. Grup ve üye sırası panelde
korunur; değerlendirme için `parameters/actions` sırası değiştirilmez.
Atanmayan parametreler mevcut dört otomatik başlık altında kalır.

- Ad 1–64 Unicode karakterdir; başında/sonunda boşluk veya denetim
  karakteri olamaz. Türkçe ad karşılaştırmasıyla iki grup aynı adı alamaz.
- Grup boş olamaz; olmayan parametreye başvuramaz. Aynı parametre bir
  grupta iki kez veya farklı gruplarda bulunamaz. Sayı sınırı mevcut
  `MAX_DYNAMIC_ITEMS` sınırıdır.
- Boş metadata yazılmaz. Eski çizimlerin baytları değişmez.
- KCAD şema 159 yalnız grup varsa seçilir; 158 veya daha eski şemada bu
  alan `unknown_field` ile reddedilir. Boş/bozuk değerler `bad_value`
  olur. Bağımsız Python okuyucu/yazıcı aynı kuralları uygular.
- Kitaplık içindeki dinamik tanım aynı metadata'yı taşır; eski okuyucu
  bilmediği alanı reddeder. Desteklenmeyen sürüme sessiz veri kaybı yoktur.
- Ortak `cad.blocks.edit(operation="dynamic")` kullanılır. Yalnız grup
  değişimi örnek değerlerini yeniden uydurmaz ve geometrinin yerel
  `card.version` sayacını artırmaz. Gerçek parametre/eylem değişimi mevcut
  doğrulama, sayaç ve örnek uyarlama davranışını korur.

## Masaüstü kullanımı

Bloklar → Bu çizim → dinamik blok sağ menüsü → **Parametre grupları…**.
Her parametrede grup yazılabilir veya mevcut ad listeden seçilebilir.
**Otomatik grup** atamayı kaldırır; diğer atamaları değiştirmez.
Grup sırası yukarı/aşağı kontrolleriyle düzenlenir. Üye sırası API'de
açıktır; pencere mevcut üye sırasını korur, yeni üyeleri tanım sırasıyla
sona ekler. Bu dilimde ayrı üye sıralama arayüzü yoktur.

Başlıklar bağımsız kapanır. Kimlik grup adı ve blok kimliğinden üretilen
sahipli metindir; çalışma zamanı metinleri statik belleğe sızdırılmaz.
Mevcut sabit başlık kimlikleri ödünç `Cow` olarak kalır.
Ad aynıysa farklı tanımların başlıkları birbirini kapatmaz.
Özel gruplarla birlikte kalan otomatik başlıklar **· Otomatik** eki alır;
kullanıcı kendi grubuna aynı adı verdiğinde iki başlık birbirine karışmaz.

Kaydet ortak komutu açılıştaki beklenen belge revizyonuyla yürütür.
Başka çizim oturumu veya arada değişmiş belge eski formu uygulayamaz.
Vazgeç/Esc yazmaz; aynı grupları yeniden kaydetmek boş tarihçe üretmez.
Kaydet düğmesi ve hatalar kaydırılan içerikten ayrı görünür kalır.
Birden çok örneğin panel düzeni değişir, geometri/override'ları değişmez.

## Python ve AI

[Kullanım örneği](../cad/block-parameter-groups.md) oluşturulan tipli
Python bağlamasını kullanır. MCP aynı `cad.blocks.edit` girişini,
salt okunur `plan` ve `expected.plan` korumasıyla kullanır. Grup için
ayrı geometri komutu veya yalnız arayüzde kalan veri deposu yoktur.
Web arayüzü ADR 0228 uyarınca ertelidir; ortak tip üretimi web kabulü
değildir.

## Doğrulama

Temiz ana dal adayında: 1612 masaüstü (256 görsel/elle çalıştırılan test
erteli), 319 application (37 erteli), 60 MCP, 457 Python, 234 KCAD,
dört katalog ve sekiz ilgili TS bağlama testi geçti. Bağımsız okuyucu/yazıcı
1501 dosyayı doğruladı. Native/MCP/codec/desktop Clippy ve son masaüstü
derlemesi başarılıdır. Son arayüz düzeltmelerinden sonra masaüstü testleri,
Clippy ve derleme yeniden çalıştırıldı.

Gerçek Mac arayüzünde yeni grup yazma, mevcut adı listeden seçme,
**Otomatik grup** ile atamayı kaldırma, grup sırası, hatalı adla pasif Kaydet,
Esc iptali ve minimum 2200×1364 pencerede sabit düğmeler görüldü.
Ürün kapalıyken Kanat açık kaldı; mevcut 1 m/Evet örnek değerleri korundu.
Kaydedilen dosyada yalnız metadata değişti; bütün geometri, yerleştirme
değerleri ve kart/yerel sayaç tam aynıydı. Gerçek tek Undo başlangıç
baytlarını, Redo yeni baytları tam geri verdi. Son derlemede dosya yeniden
açıldı; ad/üye/grup sırası, ret, açılır seçim, yeniden sıralama, dar pencere
ve tek Undo tekrar doğrulandı. Gruplanmış genişlik gerçek klavyede tek
Enter ile 1→1.05 m değiştirildi; bu işlemin tek Undo'su grup düzeni dahil
önceki dosyayı tam geri getirdi.

GUI ve korumalı Python komutları grup oluşturma, yeniden sıralama ve
ölçü düzenleme için tam aynı dosya baytlarını verdi. Grup oluşturma
sonucu SHA-256: `81f27a84e63142783569837883d8ff99ee21173c42f404cbecc137666a145589`.
Son GUI ikilisi SHA-256: `bcc9a12ed2d81cb058fe07bb8fe58214a3f6455418f3aa973b5cc85c68f47c53`.
Kabul kayıtları ve 234–239 numaralı gerçek görüntüler
`/tmp/kentos-cad-adv04-2026-11-01/parameter-groups-*` altında, repo dışında
tutuldu. Bu dilimde koyu tema ve web arayüzü için yeni kabul iddia edilmez.
Kalıcı tip–örnek kapsamı ve birleşik CAD-ADV-06 kabulü halen açıktır.
