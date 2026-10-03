# QGIS 4.2 yerleşim tasarımcısı incelemesi

- **Tarih:** 2026-10-02
- **Sürüm:** QGIS 4.2.3 “Belém do Pará” (`b041a18904a`), bu makinede kurulu.
- **Yöntem:**
  1. Yerleşim kaydı, ölçek çubuğu biçimleri, ızgara sabitleri, veriye bağlanabilen özellikler ve dışa
     aktarma ayarları PyQGIS ile okundu.
  2. Örnek bir A3 ifraz paftası kuruldu: 12 parsel, EPSG:5254, harita, karelaj, lejant, ölçek
     çubuğu, kuzey oku, öznitelik tablosu ve antet çerçevesi.
  3. Tasarımcı sanal ekranda açıldı ve ekran görüntüleri alındı.
- **Neden:** hedefimiz QGIS'ten iyi bir pafta düzeni ([tasarım](design.md)). Neyi geçeceğimizi
  ölçebilmek için önce QGIS'in neyi iyi yaptığını, nerede kullanıcıyı yalnız bıraktığını kayda
  geçiriyoruz. PiriCAD incelemesi ayrı belgededir ([piricad-review.md](piricad-review.md)).

## 1. Yetenekler (API'den okunan)

| Konu | QGIS 4.2 |
|---|---|
| Öğe türleri (18) | grup, sayfa, harita, resim, etiket, lejant, şekil, çokgen, çoklu çizgi, ölçek çubuğu, çerçeve (çok çerçeveli öğelerin kabı), HTML, öznitelik tablosu, metin tablosu, sabit tablo, işaret, yükseklik kesiti, grafik |
| Ölçek çubuğu biçimleri (8) | tek kutu, çift kutu, ortada/aşağıda/yukarıda çentikli çizgi, basamaklı çizgi, oyuk, sayısal |
| Izgara biçimi | çizgi, artı, işaret, yalnız çerçeve ve yazı |
| Izgara çerçevesi | yok, zebra, denizci zebrası, iç/dış/iç-dış çentik, çizgi kenar, denizci kenarı |
| Izgara yazısı | ondalık, ekli ondalık, derece-dakika(-saniye) (ekli/eksiz/sıfırlı), ifadeyle özel; çerçevenin içinde ya da dışında; yatay, düşey, kenar yönünde, çentiğin üstünde/altında/üstüne; her kenar ayrı |
| Atlasla ölçek | otomatik (kenar payı), sabit, önceden tanımlı ölçekler |
| Dayanak noktası | 9 nokta (köşeler, kenar ortaları, merkez) |
| Veriye bağlanabilen özellik | **90**: konum, boyut, dönüş, saydamlık, karışım, dışa aktarmadan çıkarma, sayfa boyu ve yönü, harita kapsamı, ölçeği, dönüşü, katmanları, teması, koordinat sistemi, ızgara aralığı ve yazıları, lejant başlığı ve sütunları, ölçek çubuğu parçaları ve renkleri, resim kaynağı… |
| Dışa aktarma | resim (dünya dosyası, içeriğe kırpma), PDF (GeoPDF, katmanlar ve temalar, ISO 32000 ya da OGC coğrafi bağlama, yazıyı metin ya da eğri olarak, rasterleştirme, sadeleştirme, üst veri), SVG (katmanları gruplar olarak), yazıcı |
| Çok sayfa | sayfa koleksiyonu; çok çerçeveli öğeler (tablo, HTML) sayfadan sayfaya akar |
| Rapor | bölümlü rapor düzenleyicisi (başlık, gövde, alt bilgi, alan gruplu bölümler) |

## 2. Tasarımcının görünüşü

Ekran görüntüsünde (1880×1050) görülenler:

- **Kabuk:** üstte menü çubuğu (Layout, Edit, View, Items, Add Item, Atlas, Settings). Altında iki
  sıra araç çubuğu, toplam ~30 simge, hiçbirinde yazı yok.
- **Araç kutusu:** solda 20 simgelik dikey kutu: kaydır, yakınlaştır, seç, içeriği taşı, düğüm
  düzenle; harita, resim, etiket, lejant, ölçek, kuzey oku, şekiller, işaret, ok, düğümlü öğeler,
  HTML, tablo, kesit, grafik eklemek.
- **Çalışma alanı:** mm cetvelleri; gri zeminde kâğıt.
- **Sağ panel:** iki sekme kümesi.
  - Üstte **Items** (görünürlük ve kilit kutulu öğe listesi) ve **Undo History**.
  - Altta **Layout**, **Item Properties** ve **Guides**.
- **Öğe özellikleri:** uzun, kaydırılan, katlanır gruplar. Harita için Main Properties, Layers,
  Extents, Elevation range, Temporal range, Controlled by Atlas, Grids, Overviews, Position and
  Size, Rotation, Frame, Background, Item ID, Variables, Rendering. **Her alanın yanında “veriye
  bağla” düğmesi** var.
- **Durum çubuğu:** yakınlaştırma kutusu ve kaydırıcı.

## 3. Örnek paftada karşılaşılanlar

Bu bulguların hepsi aynı oturumda, varsayılan ayarlarla kurulan örnek paftada görüldü:

| # | Gözlem | Neden önemli |
|---|---|---|
| Q1 | Harita “kapsama sığdır” sonrası **1:855,172** ölçeğinde. Elle yuvarlanması gerekiyor | Kadastro ve imar paftası 1/500, 1/1000, 1/2000, 1/5000 ister. Ölçek önce standart listeden seçilmeli |
| Q2 | Sağ kenarın karelaj yazıları **kesiliyor** (“44…”): yazılar çerçevenin dışına, antet alanına taşıyor; uyarı yok | Yazı bandı haritanın çerçevesine dahil edilmeli ve ön denetim çakışmayı söylemeli |
| Q3 | Karelaj yazıları **“485200.000”** biçiminde, üç ondalıkla; Y/X adlandırması yok | Türkiye ölçmeciliğinin dili: Y sağa, X yukarı; tam metre; isteğe bağlı binlik ayırıcı |
| Q4 | Öznitelik tablosu varsayılan olarak `fid` dahil bütün alanları küçük yazıyla basıyor | Sütun seçimi, başlık adları ve biçim baştan anlamlı gelmeli |
| Q5 | Kuzey oku bir SVG resmi. Grid ve coğrafi kuzey seçilebiliyor; manyetik kuzey ve “K” yok | KentOS'un kuzey okunu (ADR 0110) ve yakınsama notunu kullanmalıyız |
| Q6 | Antet, çizilmiş bir dikdörtgen ve üstüne gelişigüzel bırakılmış öğeler. Çerçeve taşınınca öğeler yerinde kalıyor (grup yapılmadıysa) | Hücreli, alanlı antet tek öğe olmalı |
| Q7 | Özellik paneli tek öğeyi gösteriyor | Çoklu seçimde ortak özellikler birlikte düzenlenmeli |
| Q8 | Kâğıt A3'ten A1'e alınınca öğeler A3 konumlarında kalıyor; tek otomasyon “Resize layout to content” | **Kısıtlar:** bir şablon her kâğıtta kullanılabilmeli |
| Q9 | Şablonlar `.qpt` dosyaları. Kullanıcı ve uygulama klasörlerinde duruyor; eşitleme yok, paylaşım dosya göndererek ya da herkese açık QGIS Hub'a yükleyerek | Sistem, kullanıcı, bulut ve paylaşılan şablonlar tek kitaplıkta olmalı (§5) |
| Q10 | Hiçbir araç simgesinin yazısı yok; ~50 simgelik iki çubuk ve bir kutu | KentOS'un şeridi (yazılı, gruplu) ve klavye önceliği |

## 4. QGIS'in iyi yaptığı ve aynen almamız gerekenler

1. Her özelliği veriye bağlamak ve ifade oluşturucu.
2. Atlas: kapsam katmanı, süzgeç, sıralama, sayfa adı ifadesi, otomatik, sabit ya da önceden tanımlı ölçek.
3. Harita başına birden çok ızgara ve genel bakış çerçevesi; zengin ızgara çerçeve ve yazı biçimleri.
4. Ölçek çubuğu biçimlerinin genişliği; sabit parça genişliği ya da sığdırma.
5. Sayfadan sayfaya akan çok çerçeveli tablolar.
6. Öğe ağacında görünürlük ve kilit; geri alma geçmişi; cetvel ve kılavuzlar.
7. Dışa aktarma ayarlarının yerleşimle saklanması; GeoPDF.

## 5. QGIS'i geçeceğimiz yerler

- **Kısıtlar:** öğeler kâğıda göre konumlanır; bir şablon A4'ten A0'a her kâğıtta (Q8).
- **Ana sayfa:** antet ve çerçeve bir kez tanımlanır.
- **Akıllı kılavuzlar:** kenar, merkez, eşit aralık, mesafe rozetleri; cetvelden kılavuz; klavyeyle mm adımları.
- **Çoklu seçim denetçisi** (Q7).
- **Türk ölçmecilik dili:** standart ölçek listesi (Q1); Y/X karelajı ve yazı bandının çerçeveye
  dahil olması (Q2, Q3); “K” kuzey oku ve kuzey türleri (Q5); hücreli antet (Q6); koordinat
  listesi; anlamlı sütunlu tablolar (Q4).
- **Ön denetim:** dışa aktarmadan önce, sayfa dışı, kesilen yazı, kopuk bağ, düşük çözünürlük gibi
  bulgular (Q2).
- **İki platformda aynı sonuç:** bütün hesap tek Rust çekirdeğinde; web ve masaüstü yalnız boyar.
- **Şablon kitaplığı:** sistem, bu cihaz, bulut, kurum, benimle paylaşılanlar; kaynak ve sürüm
  rozetleri; güncelleme bildirimi (Q9).
- **Yazılı, gruplu şerit ve klavye kısayolları** (Q10).
