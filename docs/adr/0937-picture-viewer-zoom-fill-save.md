# 0937 — Çizim görüntüleyici: yakınlaştırma, kalan yeri doldurma ve resim olarak kaydetme

Tarih: 2026-11-04. Durum: kabul. Yol pencerelerinin çizimleri (Y26'nın düzeni). Sahibin istekleri:

- "Enkesit, dever ve diğer görsellerin olduğu yer zoom yapılabilir olmalı; pencere büyütüldüğü zaman kalan tüm boşluğu bu
  görseller doldurmalı; zoom, unzoom ve diğer tool butonları estetik şekilde buttongroup ikonlu toolbar olmalı; bu
  kesit veya görsel kısım resim olarak kaydedilebilmeli, burası çok çok önemli".
- "Mouse tekerleği ile de zoomlama olmalı; büyüttüğümüz zaman sağa sola kaydırma olmalı; üretilen şeyler dinamik olmalı,
  zumladıkça yeniden üretilebilir; resmini zoomladığınızda pikseller bozuluyor".
- "Bu pencere bileşen olursa ileride geliştirmemiz kolay olur; yolla ilgili bütün pencereleri bununla değiştir".

## Dayanak

- **OpenRoads Designer** kesit ve boykesiti MicroStation görünümünde açar. Görünümün araçları Zoom In, Zoom Out, Window
  Area, Fit View ve Pan View'dır (Bentley yardım, şerit başvurusu).
- **Civil 3D** (Project Explorer'ın kesit ve boykesit görünüşleri): tekerlek yakınlaştırır, orta tuşla sürükleme
  kaydırır, sağ tık menüsünde Zoom to Extents vardır. Kesit düzenleyicide istasyon değişince öntanımlı kip "kapsamına
  sığdır"dır; ötekiler bir öteleme–kotta ya da alt montajda kilitler.
- **MicroStation Save Image:** kaynak görünüm seçilir, en-boy oranı korunur ve çözünürlük piksel olarak verilir.
  Ekrandan ayrıntılı resim Save Image ile alınır.
- **Kullanılabilirlik kaynakları:** tekerlekte imlecin altındaki nokta yerinde kalmalıdır. Düğmeler sabit oranla
  çalışır, yüzde gösterilir, bir "sığdır" düğmesi bulunur (Google Maps ve Figma'nın denetimleri, shadcn'in
  yakınlaştırma düğme grubu). Plotly'nin araç çubuğu imleç çizimin üstündeyken belirir.
- **NETPRO:** çevrimiçi belgesi bulunamadı.

## Karar

### §1 Bileşen: `kentos_ui::widget::picture::Picture`

**Yakınlaştırma yeniden üretmedir, büyütme değil.**

- İçerik (bir tuval) görünen alanın `z` katı büyüklüğe yerleşir ve kendini o boyda yeniden çizer. Görünen alan onun
  üstünde kayar.
- Tuval eksenlerini, aralıklarını ve yazılarını kendi boyuna göre kurar. Bu yüzden yakınlaşınca çizgiler incecik,
  yazılar keskin kalır: CAD'in yakınlaştırması gibi.
- Bir resmi büyütmek pikselleri iriltirdi. Bir dönüşümle büyütmek de çizgileri ve yazıları kalınlaştırırdı.
- Yol pencerelerinin 27 çizimi kendi kodu değişmeden yakınlaşır.

**Yakınlık ve kaydırma:**

- Yakınlık 1 (sığdırılmış) ile 40 arasındadır.
- Tekerlek satır başına 1,2 kat yakınlaştırır ve imlecin altındaki noktayı yerinde tutar. Düğmeler ortaya göre 1,5 kat
  yakınlaştırır.
- Sol ya da orta tuşla sürükleme her yöne kaydırır. Olay önce çizime verilir: km çizgisini sürükleyen bölünmüş görünüş
  sol basışı alır, orada kaydırma orta tuşla yapılır.
- Shift+tekerlek, eğik tekerlek ve dokunmatik yüzeyin yatay kaydırması sağa sola kaydırır (iced'in kaydırma alanının
  okuduğu gibi).
- Yakınken altta ve sağda ince kaydırma çubukları belirir. Tutulup sürüklenirler; yoluna basılınca oraya atlanır.
- Çift tık sığdırır.

**Araç çubuğu:**

- Sağ üstte birleşik bir düğme grubudur: `− %250 + | ⤢ | ⤓`. Simgeler kitin `Minus`, `Plus`, `ZoomExtents` ve `Save`
  simgeleridir.
- Düğmenin adı üzerine gelince altında yazar. Yakınlık etiketinin ipucu fare ile ne yapılacağını söyler.
- İmleç çizimin dışındayken çubuk yalnız yakınlık hapına (`%100`) kapanır. Ölçüm: açık çubuk kendi boyundaki Enkesit'te
  kesitin sağ şevini ve eksen yazısını, 260 px'lik hat planının %70'ini kapatıyordu.
- 360 px'ten dar çizimde açık çubuk yakınlık etiketini almaz.

**Resim olarak kaydet:**

- Görüneni, yakınlık ve kaydırmasıyla birlikte, ekransız çizicide (tiny-skia) iki kat piksel yoğunluğunda, aydınlık
  temada ve düz zeminde yeniden çizer. Rapora konacak bir resimdir, ekranın kopyası değildir.
- Araç çubuğu ve kaydırma çubukları resme girmez. Enkesit görünümü 2616×1212 piksel çıkar.
- PNG baytları `on_save`'in mesajıyla uygulamaya gider. Dosyayı uygulama sorar (çizimin klasöründe, çizimin adıyla) ve
  yazar; günlüğe "Resim kaydedildi (2616×1212 piksel): …" düşer.

**Durum:**

- Yakınlık ve kaydırma bileşenin ağacındadır. Veri değişse de korunur: sonraki kesite geçince aynı ayrıntıya bakılır
  (Civil 3D'nin "öteleme–kotta kilitli" kipi gibi).
- `View` bir `Operation`'ın `custom`'ıyla okunur ve değiştirilir; testler böyle okur.

**Ölçülen hata.** Diyalog gövdesini iki kez yerleştirir: önce en az boyla, sonra kendi boyuyla (ADR 0746).

- İlk sürüm kaydırmayı `layout`'ta boya kıstırıyordu. Tam ekranda pencerenin altına yakın bir yakınlaştırma ilk
  geçişin 247 px'ine kıstırıldı: kaydırma 1600 yerine 815 oldu, görünüm hendekten 1,3 m yukarıda açıldı.
- `layout` artık durumu değiştirmez. Kaydırma kullanılırken son boya göre kıstırılır.
- Birim testi bunu yakalamamıştı, çünkü orada iki geçiş yoktu. Bekçi
  `the_point_under_the_wheel_stays_in_a_full_screen_window`.

### §2 Kalan yeri doldurma: `kentos_ui::widget::grow`

- iced'in kaydırma alanı içeriğine sınırsız yükseklik verir; içindeki çizim kutunun büyüdüğünü bilemez.
- `Dialog::scroll` gövdeyi `Reach` ile sarar. Kutu büyütülünce ya da tam ekran yapılınca gövde kesin bir yükseklik
  alır; `Reach` bunu içindeki `Grows` sütunlarına söyler. Büyüyen çocuk öbürlerinin bıraktığı yeri alır, en az boyunun
  altına inmez; birden çok büyüyen çocuk yeri en az boylarıyla orantılı paylaşır.
- Kendi boyundaki kutu kesin yükseklik vermez: pencere bugünkü gibi açılır.
- Öbür modüllerin diyalogları etkilenmez. `Reach` geçirgendir; `Grows` kullanmayan gövdede hiçbir şey değişmez.

**Ölçüm (1920×1080 tam ekran):**

| Pencere | Kendi boyunda | Tam ekranda |
|---|---|---|
| Dever | 226 px | ~670 px |
| Enkesit | 247 px | 606 px; yanındaki hat planı da aynı boyda |
| Boykesit | 356 px | 542 px (altında PVI tablosu duruyor) |
| Kübaj, Brükner | — | kalan yerin hepsi |

### §3 Grafikler yakınlaştıkça yeniden üretilir

- `picture::visible()` içeriği çizerken görünen bölgeyi söyler. Masaüstünde `picture::Shown` onu okur.
- Beş ana grafik (Enkesit, Boykesit, Dever, Kübaj, Brükner):
  - Ölçü adımını piksel sıklığından seçer: 1, 2, 2,5, 5 × 10ⁿ; kotlar en az 40–45 px, mesafeler 70 px, km'ler 90 px
    aralıklı. Yakınlaştıkça araya yeni çizgiler ve yazılar girer. Ölçüm: Enkesit %430'da kotlar 0,5 m yerine 0,1 m,
    mesafeler 5 m yerine 1 m; Boykesit %299'da km'ler 50 m yerine 20 m.
  - Eksen yazılarını görünen bölgenin kenarına sabitler, yerinden kalkanı zemin kutusuna koyar. Boykesit'in bantları
    (Km, Mesafe, Siyah ve Kırmızı kot, Fark, Eğim) görünen bölgenin altına sabitlenir. Hepsi kaydırma çubuğunun üstünde
    durur.
  - Sığdırılmışken hiçbir şey yerinden oynamaz; görünüm eskisiyle aynıdır.
- **Tuzak:** iced'in tuvali bir çizimdeki bütün yazıları bütün dolguların üstüne çizer; zemin kutusu yazıyı örtmez.
  Sabitlenmiş bantlara ya da ad sütununa düşecek yazı (kurp etiketi, PVI adı, kenara taşan km değerleri) bu yüzden
  yazılmaz.

### §4 Pencereler

- `RoadDialog::picture` çizimi kendiliğinden görüntüleyiciye sarar ve büyüyen çocuk yapar. Resmin adı pencerenin
  başlığıdır; `picture_name` ile verilir.
- `RoadDialog` dışındaki pencereler `crate::picture::picture(çizim, ad)` kullanır. Yanında liste olan çizim enine de
  büyür; liste sabit genişlikte kalır, boyu satırla birlikte uzar.
- Dönüştürülen 27 çizim:
  - Dever, Genişletme, Düşey kurp boyu, Kübaj, Taşıma planı, Platform suyu, Kırmızı kot önerisi, Görüş mesafesi,
    Tasarım denetimi;
  - Boykesit, Enkesit (kesit, hat planı, tip kesit), Brükner, Koridor;
  - Menfez, Otokorkuluk, İstinat duvarı, Kavşak, Araç izi, Saha tesviyesi, Drenaj, Arazi çözümlemesi, Aplikasyon,
    Ön güzergâh;
  - Pafta, aks ve boykesit uydurma, bölünmüş görünüşlerin üç görünüşü.
- Dışarıda bırakılanlar: 3B sahne (kendi yörünge kamerası vardır) ve arazi çözümlemesindeki 10×10 renk örneği.

## Doğrulama

- **kentos-ui (12 test):**
  - imlecin altındaki noktanın kalması;
  - tekerleğin yakınlaştırması, Shift+tekerlek, eğik tekerlek ve dokunmatik yüzeyin kaydırması;
  - sürükleme, çift tık, düğmeler;
  - kaydırma çubuklarının yeri, sürüklenmesi ve yoluna basılması;
  - kaydedilen resmin boyu ve içeriği: yakınlaşınca işaret 2,25² ≈ 5,06 kat büyük çıkar;
  - dar çubuk;
  - `Grows`'un tam ekranda, kendi boyunda, alçak gövdede ve `Reach` dışında yerleşimi.
- **Masaüstü (3 test):**
  - Dever, Enkesit, Boykesit ve Kübaj tam ekranda çizime yer verir;
  - Enkesit yakınlaşır, kayar ve kaydedilir: PNG baytları, dosya adı, günlük;
  - tam ekranda imlecin altındaki nokta kalır.
- Pencere boyları sürecin ortak deposunda durduğu için bu testler sırayla koşar. Paralel koşumda bir testin tam ekran
  Enkesit'i öbürününkini yarıda tam ekran yaptı.
- Resimler `.run/gui-evidence/goruntuleyici/`.

## Açıklar

- Öbür modüllerin (CAD, Harita, Mimari) çizim pencereleri. Bileşen hazır, `Grows` ile `Picture`'ı kullanmaları yeter.
- Panoya resim kopyalama (iced'in panosu yalnız metin taşır), pencereyle yakınlaştırma (Window Area), SVG ya da PDF
  olarak kaydetme.
- Plan çizimlerinin (Koridor, Aplikasyon …) ölçek çubuğu; eksenleri yoktur, çizgileri yeniden üretilir.
