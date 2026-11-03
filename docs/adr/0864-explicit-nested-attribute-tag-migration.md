# ADR 0864 — İç içe bloklarda açık kapsamlı etiket taşıma

Tarih: 2026-11-03. Kapsam: native komut, masaüstü, Python ve MCP.
CAD-ADV-06'nın iç içe öznitelik politikası; bütün maddenin kabulü ayrı izlenir.

## Gerekçe ve karar

Bir kapının `NO` özniteliğini `ODA` yapınca doğrudan yerleştirmeler kendi
değerlerini taşıyordu. Başka bir tanımın içindeki kapı `NO=İÇ-1` kaydını
koruyor, fakat yeni `ODA` için varsayılanı gösteriyordu. Mevcut çizim kapsamını
sessizce genişletmek eski komutların anlamını değiştirir.

`cad.blocks.edit` girdisine isteğe bağlı `renameNested` eklenir. Yok veya
`false`: yayımlanmış davranış. `true`: `attributes`, `update` veya `sync`
işlemindeki açık `renames` haritası, hedef bloğu kullanan diğer tanımlardaki
yerleştirmelere de uygulanır. `sync=false` ve başka işlemlerle birlikte
istenirse `invalid_parameter_value`, yol `renameNested` döner.

Bu işlem etiket **kimliği** taşır. İç tanımlarda ilgisiz öznitelikleri
temizlemez, yeni varsayılanlar yazmaz ve alan şablonu önbelleklerini genel
olarak yeniden hesaplamaz. Değer ve özel yazı konumu bütün kaynaklar okunup
sonra hedeflere yazılarak taşınır; eşzamanlı takas/zincir veri kaybetmez.
Parametre, görünürlük, geometri, yerleştirme kimliği ve yayın sürümü korunur.
Değişen üst tanımın yerel `card.version` sayacı bir artar.

İlgisiz hedefte farklı bir değer veya özel konum varsa işlem reddedilir;
tanım adı ve iç yerleştirme numarası bildirilir. Kilitli bir doğrudan kullanım,
değişen iç kayıt veya üst seviyelerdeki kilitli kullanım geniş kapsamı atomik
olarak reddeder. Dış başvuruya ait üst tanım değiştirilmez. İlgisiz kilitler
işlemi engellemez. Son tanım grafiği ve taşınmış doğrudan kayıtlar birlikte
doğrulanır; ara haldeki eski etiketler üzerinden ret üretilmez.

## Arayüz ve API

Blok öznitelikleri penceresi, tanım içi kullanımlar varsa **Etiket taşıma
kapsamı** seçicisini gösterir. Varsayılan **Yalnız çizimdeki yerleştirmeler**;
ikinci seçenek **İç içe tanımlar da dahil**. Tablo altında güncel komut planının
gerçekte değiştireceği iç kayıt sayısı ve üst tanım adları gösterilir.
Dar kapsam, eski iç etiketlerin taşınmayacağını ve varsayılan görünme
ihtimalini bildirir. Önizleme yazı yazarken giriş tablosunu yerinden oynatmaz.
Gövde kayar; başlık ve Kaydet/Vazgeç görünür kalır.

Plan ve kaydet aynı girdi ile açılış revizyonunu kullanır. Başka çizim
oturumu veya değişmiş revizyon eski pencereden kaydedilemez. Vazgeç, plan ve
doğrulama çizimi değiştirmez. Kaydet tek geri alma adımıdır.

`BlocksEditPlan.changed` ve yürütme çıktısındaki `changed` ilgili üst
tanımları da içerir. Eski `AttributeSync.inserts` alanının anlamı değişmez:
yalnız çizimdeki doğrudan kayıtların sayısıdır. Tanım içi adet bu alana
eklenmez. Daha önce dar kapsamda değiştirilmiş etiketi sonradan taşımak için
aynı harita ile açık `operation="sync", rename_nested=True` kullanılır.

Python ve MCP aynı yerel komut yolunu kullanır; ayrı bir geometri yazıcısı
yoktur. Plan kimliği girdiye/kapsama bağlıdır; dar kapsamlı plan geniş kapsam
için yetki oluşturmaz. KCAD alanı değişmez; dosya şeması artırılmaz.
Web yürütücüsüne bu yeni davranış eklenmiş sayılmaz; görev TODOS'da ayrıdır.

## Karşılaştırma kaynakları

- [AutoCAD ATTSYNC](https://help.autodesk.com/cloudhelp/2021/ENU/AutoCAD-Core/files/GUID-56B14079-250B-4C99-AB3D-F95BA1C32AB7.htm):
  mevcut öznitelik değerlerini korur; biçim/özellik değişiklikleri ve ek veriler
  hakkında ayrıca kayıp uyarıları vardır. Buradaki dar etiket taşıma özel
  konumları ve ilgisiz kayıtları korur; tam ATTSYNC uyumluluğu iddia edilmez.
- [BricsCAD ATTSYNC](https://help.bricsys.com/en-us/document/command-reference/a/attsync-command?version=V27):
  seçilen tanımı eşitleme ve atlama seçenekleri kapsamın kullanıcıya açık
  olmasını destekler. Yeni bir onay zinciri eklemek yerine mevcut pencerede
  seçim ve etki önizlemesi kullanılır.

## Doğrulama

364 application testi (8 yeni kapsam testi ve paylaşılan komut durumları),
1622 masaüstü, 65 MCP ve tüm 475 Python testi, SDK üretim kontrolü,
tüm ilgili hedeflerde Clippy ve uygulama derlemesi geçti. Son kompakt
pencere değişiminde 6 pencere testi yeniden geçti. Kapsam testleri farklı
doğrudan/iç değerleri, özel konumları, takas/zinciri, hedef çakışmasını,
kilitli doğrudan ve üst kullanımları, dar kapsamın korunmasını, sonradan
sync'i, revizyon korumasını ve tek geri alma/kaydet–aç sonucunu sınar.

Gerçek GUI'de iki görünürlük, lookup, ortak 1.1 m genişlik, 2.2 m hesap
alanı, farklı yönler, iç kapı ve bağlı 1100 mm ölçü aynı dosyadaydı.
`NO→ODA` üç farklı değeri K-1/K-2/İÇ-1 olarak korudu. Son sürümde O ve DA
ayrı ayrı yazılırken tablo ve düğmeler yerinden oynamadı. En küçük
2200×1364 test penceresinde seçim klavyeyle yapıldı; boş etikette Kaydet
pasifti; düzeltme/önizleme ve Vazgeç çalıştı. İptal sonrası dosya, önceki
dosyayla tam aynı kaldı. Ekran görüntüleri depoya eklenmedi.

Etiket göçünden sonra gerçek Kitaplık panelinden yalnız ilk kapı yayın 3'e
yükseltildi. Hedef 1.2 m/2.4 m oldu ve bağlı ölçü 1200 mm'yi gösterdi.
İkinci kapının Sade/Yön girdileri, eski ortak 1.1 m ve iç İÇ-1 korundu.
Her iki işlemin GUI dosyaları korumalı Python yürütmesiyle bayt düzeyinde
aynı; gerçek tek Undo/Redo kayıtları da tam aynı. Kaydet–aç yapıldı.
Gerçek statik DXF yazıldı ve uygulamada yeniden açıldı: ölçü, kodlar,
görünüş ve iç boyutlar korundu; dinamik düzenlenebilirlik kaybı API/sonuç
raporunda açıkça bildirildi. Aynı DXF'in GUI/Python baytları tam aynı.
Bağımsız KCAD okuyucu birleşik kaynak ve GUI kayıtlarını kabul etti.

Birleşik testte yazım **öncesi** DXF penceresinde parametrik kayıp
uyarısının eksik olduğu ayrıca görüldü. Bu ADR bunun giderildiğini veya
CAD-ADV-06'nın tamamen kapandığını iddia etmez; kalan kabul plan dosyasında
izlenir.
