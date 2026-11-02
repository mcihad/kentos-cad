# ADR 0861 — Blok formüllerinin bağımlılık sırası ve döngü reddi

Tarih: 2026-11-03. CAD-ADV-06; ADR 0318 ve 0860'ın devamı.

## Sorun ve dayanak

Gerçek masaüstü konsolunda, `Nihai = 1 / @Ara`,
`Ara = @Genişlik * 2` zincirinde genişlik 1.10 m iken doğru sonuç
0.454545… olmasına rağmen eski yol `Ara` parametresinin 0.10 m geometrik
başlangıcını okuyup 10 sonucunu üst sınırdan reddetti. Aynı tanımda
`Nihai = @Ara + 1`, `Ara = @Nihai + 1` döngüsü plan oluşturabildi.
Eski algoritma formül sayısı kadar bütün listeyi yeniden dolaşıyordu;
ara sonuçları kontrol etmek ve sabit tur sayısı döngüyü çözmek değildir.

[BricsCAD'in resmî açıklaması](https://help.bricsys.com/en-us/document/bricscad/parametric-modeling/parametric-modeling-overview)
bağımlı parametrelerin girdisi değişince yenilenmesini ve parametre
döngüsünün reddini tanımlar.
[Revit FamilyManager.SetFormula](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-API-MainReference/files/html/cdc3156c-0334-0bba-70af-1df78fb18b50.htm)
de döngü oluşturan formülün ret gerekçesi olduğunu belirtir. Bunlar
KentOS ifade sözdizimini kullandıkları iddiası değildir; aşağıdaki
algoritma KentOS'un mevcut ifade motoruna uygulanır.

## Karar

- Bağımlılık adları gerçek ifade lexer'ının `At` tokenlarından çıkarılır.
  Tırnak içindeki `@ad` metni kenar değildir. Sayı girdileri mevcut motorla
  aynı doğrusal/açı parametreleridir; kutupsal/nokta bir sayı sayılmaz.
- Ad karşılaştırması `Schema::variable` ile aynı Türkçe harf ve büyük/küçük
  harf eşlemesini ve ilk eşleşme politikasını kullanır. Varsayılan girdi
  değişimi denetimi ve proje/blok birim dönüşümünün formül uyarlaması da
  bu eşleşmeyle çalışır. Açı girdisi uzunluk gibi dönüştürülmez.
- Kararlı topolojik sıra oluşturulur; her formül, bağımlılıklarının son
  değeriyle bir kez hesaplanır. Sınır/adım/liste denetimi bu sonuç üzerindedir.
  Parametre sunum sırası, eylem sırası ve grup metaverisi yeniden yazılmaz.
- Tekrarlanan aynı girdi bir bağımlılıktır. Eski hesap önbelleği veya başka
  formülün geometrik varsayılanı çözülmüş bağımlılık yerine geçmez.
- Döngü ve kendine başvuru yazmadan reddedilir. İleti gerçek döngüyü
  `“Nihai” → “Ara” → “Nihai”` olarak gösterir; yalnız o döngüyü okuyan
  aşağı akıştaki parametreyi suçlamaz. Bu bir yinelemeli denklem çözücüsü
  değildir; koşullu ifadenin metinsel bağımlılıkları da döngü denetimine dahildir.
- Ortak `derive` yolu tanım denetimi, parametre düzenleme/sıfırlama,
  kitaplık ve iç içe kullanım hesaplarında geçerlidir. GUI/Python/MCP
  aynı komut ve ret kodlarını kullanır. Yeni KCAD alanı veya komut yoktur.
- Eski döngülü tanım dosya okuyucusuyla açılabilir; bu dilim okuyucuda gizli
  hesap veya dosya dönüşümü yapmaz. Formül doğrulayan bir düzenleme döngüyü
  reddeder. Böyle bir çizimde formül düzeltilmeden yeni sonuç kabul edilmez.

## Kabul

İleri başvuru ve ters sunum sırası, son değer sınırları, Unicode/harf
varyantıyla kendine başvuru, gerçek döngü yolu, tekrarlı başvuru, tırnak
metni, eski önbellek, boş kök/iç kullanım ve 256 parametre sınırına yakın
ters zincir native testlerle sınanır. Korumalı plan, yazısız ret,
kaydet–aç ve tek Undo Python/MCP'de de doğrulanır. Gerçek masaüstünde eski
iki hata ve düzeltilmiş ret/başarı, ölçü düzenleme ve kayıt tekrarından
geçirilir. Kanıt görselleri depoya eklenmez.

Gerçek GUI'de eski yanlış ret ve yanlış döngü kabulü üretildi (251–252).
Yeni ikilide korumalı planla aynı ileri zincir eklendi, kaydedildi ve
1.10→1.15 m panel girdisi tek Enter ile bütün hesapları yeniledi.
Gerçek döngü iletisi `“Nihai” → “Ara” → “Nihai”`; reddin ardından dosya
baytları ve işlem geçmişi değişmedi. Çizim odağında tek Undo/Redo,
kaydet–aç–kaydet ve korumalı Python karşılaştırması tam aynı dosyaları verdi.
Ayrıca `@genislik` girdili zincirde gerçek sağ tuş menüsünden mm→cm
**Sayıları koru** uygulandı; formüller/geometri birlikte dönüştü ve tek Undo
başlangıç dosyasını verdi. 16 dosya bağımsız okuyucu ve native tekrar
kayıtla doğrulandı; ekranlar 253–257 depo dışındadır.

Kabul sırasında Python konsolu odaktayken Aç kısayolunun metin eklediği
ayrı arayüz sorunu kaydedildi; bu dilim onu çözmüş sayılmaz. Konsolun
kendi metin Undo'su, çizimin Undo kabulü yerine kullanılmadı.

348 application (paylaşılan komut korpusu dahil), 1612 masaüstü, 63 MCP ve
464 Python testi geçti. Panik varsayımlarını güvenli retlere çeviren son
kaynakta application 348, hedefli MCP zinciri ve yeniden derlenmiş Python
modülüyle 5 formül testi tekrar geçti; Python/masaüstü/MCP Clippy tüm
hedeflerde temiz. Son GUI ikilisinde döngü reddi, 1.15→1.20 m tek Enter
ve çizim odağında tek Undo dosyaları Python ile tam aynı (259–260).
