# ADR 0877 — Büyük koordinatta kısa örüntü aralığını toplam açıklıkla doğrula

Tarih: 3 Kasım 2026. CAD-ADV-07 tanımasının sayısal regresyon düzeltmesi.

## Ölçülen sorun

Native ve bağımsız tipli Python çağrısıyla aynı hata üretildi: X=4 000 000 000 m,
3 mm aralıklı sekiz mevcut blokta iç kaynak seçilince `cad.arrays.find` boş
sonuç veriyordu. Aynı kökler ve 3 mm aralık `cad.arrays.adopt.plan` ile
geçerliydi. 30 mm doğrusal sıra ve iki ekseni büyük koordinatlı 30 mm × 4 mm
grid de benzer yanlış olumsuz sonuç üretti. Kaynak nesneler değişmiyordu.

[Goldberg'in kayan nokta aritmetiği makalesi](https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html)
temsil yuvarlaması ve küçük farkların hata hesabı için temel dayanaktır.
Buradaki hata makaleden varsayılmamış, KentOS komutlarıyla yeniden üretilmiştir.
Yakın dünya koordinatlarının farkı tam hesaplanabilse bile, koordinatların
önceden yuvarlanmış olması kısa aralığın ölçüsünü etkileyebilir. Tek kısa
aralıktaki bu fark bütün yerlere çoğaltıldığında son konum doğrulaması
başarısız oluyordu. Ayrıca başarısız önerinin üyelik anahtarı erkenden
tekrar önbelleğine alınarak sonraki geçerli öneriyi bastırabiliyordu.

## Karar

Gözlenen komşu aralıklar önceki gibi tam sayılı yer üyeliğini kurar; gizli
ara yerler varsayılmaz. Sonra aynı üyelik için iki yerleşim önerisi sırayla
denetlenir:

1. Doğrusal sırada ilk/son dolu kökün gerçek dünya farkı yer aralığı sayısına
   bölünür. Gridde dahil olan köklerin X/Y açıklıkları sütun/satır aralığı
   sayısına bölünür; kaynak köşenin yön işaretleri korunur.
2. Toplam açıklıktan türetilen doğrusal aralık, aynı iş bütçesi içinde üyeliği
   yeniden taramak için de kullanılır. İlk kısa aralık tahmininde dışarıda kalan
   geçerli uç kök böylece geri bulunabilir. Yeni boş yer/yarım aralık varsayılmaz.
3. Toplam açıklık önerisi geçmezse önceki gözlenen kısa aralık denenir.

Her öneri değişmemiş `arrays_adopt::inspect` ve mevcut gerçek yerleşim
motorundan geçmek zorundadır. Tolerans büyütülmez, koordinat/birim/ölçek veya
nesne değiştirilmez. Fark raporu aynı mutlak model toleransındadır. Bir
üyelik ancak kabul edilmiş öneriden sonra tekrar önbelleğine alınır; ret
sonraki geçerli denemeyi susturmaz. Ek denemeler ortak iş bütçesine ve
iptal denetimine dahildir.

Bu yöntem keyfi gürültüde eksiksiz en iyi uyum çözücüsü veya ideal ondalık
aralığı geri çıkarma iddiası taşımaz. Ölçümlerden türetilen aralık açıkça
incelenir. Hiçbir öneri bütün gerçek konumları mevcut toleransta doğrulamazsa
aday uygulanamaz; geometriyi toleransa uydurmak için taşıma yapılmaz.

## Kabul kapısı

Önce iki native regresyon testi ve iki bağımsız ortak durum eski kodda
başarısız olarak kaydedildi. Doğrusal test 3/30 mm; grid testi 30×4 ve
30×300 mm durumlarını, geçerli bilinen yerleşimle kıyaslar. Düzeltme sonrası
11 öğeli uç üyelik regresyonuyla 18 native örüntü testi başarılıdır. 33 ortak durum native ve Python üzerinden, 13 Python dizi testi ve dört
ilgili MCP testi başarılıdır. Native/desktop/MCP/Python ilgili tüm hedeflerde
Clippy başarılıdır; yeni MCP testi için denetim ayrıca yinelenmiştir.
Gerçek fareyle içteki altıncı kaynak seçildi, 11 dolu/0 boş/3 mm sonuç
incelendi ve bağlandı. Bağımsız KCAD okuyucusunda dizi metaverisi dışındaki
bütün alanlar aynıdır; Undo dosyası başlangıçla, Redo dosyası bağlanmış
durumla byte düzeyinde eşittir. Sonuç manifesti
`recognition-precision-gui-acceptance.json` dosyasındadır.

Aynı uç koordinatlı GUI denemesinde 0,6×0,5 mm semboller yatay çizgi
gibi göründü. Bu ayrı render sorunu açık kalır; dizi tanıma kabulü render
kabulü olarak sunulmaz. 405–408 numaralı ekranlar bulguyu korur.
Ham kayıtlar `/tmp/kentos-cad-adv04-2026-11-01/recognition-precision-*`
altındadır; ekranlar repoya eklenmez.


180 geçerli bilinen yerleşimle yapılan ek sayısal taramada, mutlak toleransı
koordinatın bir ULP'sine eşit/üstünde olan örneklerde eksik tam örüntü kalmadı.
Bu genel doğruluk kanıtı değildir: 52 örnekte tolerans koordinatın temsil
aralığının altındaydı ve tam örüntü hâlâ bulunamadı. Bu sınır bu düzeltmeyle
kapatıldı diye sunulmaz; tol gizlice büyütülmez. Yerel probe ve ayrıntılı
sonuçlar repo dışında korunur.
