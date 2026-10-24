# 0626 — Birim ve çizim ölçeği sadeleşir; CAD'de yazılar gerçek boyunda

Tarih: 24 Ekim 2026. Kapsam: masaüstü (web ADR 0228 gereği bekler; web'de gerekenler TODOS.md §0.2'de).
ADR 0622 §2'nin sihirbaz ve durum çubuğu kurallarının, ADR 0624 §1'in “yeni yerel CAD mm / 1:1” varsayılanının ve
ADR 0205 §5'in varsayılan yazı boyunun yerine geçer.

## Sahibin bildirimi

Sahip (24 Ekim):

- Ölçü sistemi çok karmaşık. Yeni proje oluştururken ve proje açarken sürekli birim ve ayar soruluyor.
  Kullanıcı bunu böyle kullanamaz, kafası karışır.
- Durum çubuğunda iki ayrı ölçek var.
- Açıklama › Ölçü araçlarıyla ölçü alınca yazılar kaybolup geliyor. Sahneyle uyumsuz ve çok yavaş.

Sahibin kararları (24 Ekim, seçeneklerle soruldu):

- **Yeni CAD projesi: işe göre tek seçim.** Sihirbaz yalnız “Ne çiziyorsunuz?” diye sorar. Seçenekler Mimari proje
  (metre, 1:100), Harita / imar planı (metre, 1:1000, gerçek koordinat) ve Detay / imalat (milimetre, 1:1).
  Yazı boyları ve yazı tipi sihirbazdan çıkar, Proje ayarları'nda kalır.
- **CAD'de yazılar gerçek boyunda.** Yazılar çizimdeki boylarıyla çizgilerle birlikte büyür, küçülür.
  Okunamayacak kadar küçük olanlar ince bir çizgi olarak görünür. Hiçbiri kaybolmaz, sıçramaz.
  CBS'deki etiketler bugünkü gibi kalır.

## Bulunan

Sahibin dosyasında ölçüldü. Dosya `deneme_suşehri.dxf`: 71 820 nesne, 13 112 yazı, `$INSUNITS` 4, `$DIMSCALE` 1,
`$TEXTSIZE` 2,5. Ölçüm `label_probe` ile yapıldı, `perf::file` aşağıda.

- Durum çubuğunda CAD projesinde iki hücre vardı:
  - **“Yazı 1:1.000 m”:** çizim ölçeği. Yazı ve ölçü boylarını belirler.
  - **“Ekran 1:N”:** görünümün yakınlaşma ölçeği.

  İkincisi CAD'de bir şey anlatmıyordu; dar pencerede gizlendiği için yalnız geniş ekranda görünüyordu.
- Aynı ölçeğin dört adı vardı: sihirbazda “Yazı ve ölçü ölçeği”, durum çubuğunda “Yazı”, Proje ayarları'nda
  “Çizim ölçeği”, ayarların bölümünde “Ölçek ve yazılar”. Sihirbaz ayrıca birim kartlarını, kâğıttaki yazı ve ölçü
  boylarını ve yazı tipini soruyordu.
- Kaybolmasın kuralı yazıları gizliyordu:
  - 8 pikselden küçük her yazıyı 8 piksele büyütüyordu. Büyütülüp birbirinin üstüne binenleri gizliyordu.
    Ölçü değerleri de bu kurala giriyordu.
  - Görünüm kayarken ADR 0597 son yerleşimi tutar. Durulunca yerleşim baştan yapılır. Çalışma görünümünde
    1 096 yazının **488'i belirdi, 445'i kayboldu.**
  - Kayarken yazılar eski büyütmeleriyle çizildi. Görünüme yeni giren yazılar hiç çizilmedi.
  - “Sahneyle uyumsuz” ve “kaybolup gelen” bu kuraldır.
- Gerçek boy kuralı da yazıları gizliyordu:
  - 5 pikselden küçük ve 240 pikselden büyük yazıyı hiç çizmiyordu.
  - Aynı görünümde 13 112 yazıdan yalnız 7'si görünüyordu.
- Açılan DXF her zaman 1:1 açılıyordu (ADR 0624 §2).
  - Bu dosyada yeni yazılar dosyanınkilerle aynı boyda (2,5 birim) çıkıyor; burada sorun değil.
  - DIMSCALE 100 olan bir mimari çizimde ise yeni ölçüler dosyanın ölçülerinin yüzde biri boyunda çıkıyordu.

## 1. Yeni proje: “Ne çiziyorsunuz?”

CAD projesinin sihirbazı üç adımdır.

| Adım | Ne sorulur |
| --- | --- |
| Proje türü | CAD ya da CBS |
| Çizim | Mimari proje, Harita / imar planı, Detay / imalat (`kentos_project::wizard::WORKS`). Harita / imar planı seçilirse altında il ve koordinat sistemi gelir. |
| Proje adı | Yalnız ad ve kısa özet |

Seçenekler ve getirdikleri:

| Seçenek | Koordinatlar | Birim | Çizim ölçeği |
| --- | --- | --- | --- |
| Mimari proje | Yerel (0,0) | metre | 1:100 |
| Harita / imar planı | Gerçek (ilin dilimi) | metre | 1:1000 |
| Detay / imalat | Yerel (0,0) | milimetre | 1:1 |

- Seçilen işin birimi ve ölçeği `Draft::choose_work` ile gelir.
- Yerel çizimin varsayılan ölçeği birimine göredir (`cad_default_scale`):
  - mm, inç ve ayak 1:1;
  - cm 1:50;
  - metre 1:100.
- Gerçek koordinatlı çizim 1:1000'dir.
- CBS projesinde Konum adımında il ve koordinat sistemi seçilir, sonra ad verilir. Harita ölçeği sihirbazda
  sorulmaz; 1:1000 başlar.
- Yazı boyları (ADR 0205 §1) ve çizim yazı tipi sihirbazdan kalkar. Proje ayarları'nda değiştirilir.
- `newProjects.drawingUnit`'in varsayılanı metredir. Sihirbaz önce Mimari proje seçili açılır. Kullanıcının son
  yerel projesinin birimi bir sonraki sihirbazda seçili gelir: milimetre Detay / imalat'ı seçer.
- Başsız sunucu, Python ve MCP'nin yeni çizim isteğinde (ADR 0245) `drawingUnit` ve `plotScale` açıkça verilebilir.
  - Birim verilmezse sihirbazın ilk işi alınır: metre, 1:100. ADR 0624'ün mm / 1:1'i yerine geçer.
  - Ölçek verilmezse yukarıdaki kuraldan gelir.
- fixtures/project/v1/wizard.json masaüstünün modelinden yazılır (`KENTOS_WRITE_WIZARD=1`). Web'in sihirbazı
  TODOS.md §0.2'de bekler.

## 2. Tek ad: “Çizim ölçeği”

Yazı ve ölçü boylarını belirleyen ölçeğin her yerde tek adı vardır: **çizim ölçeği**. Bu ad sihirbazın özetinde,
durum çubuğunun ipucunda ve menüsünde, Proje ayarları'nda ve Öznitelikler'in çizim satırında kullanılır.

## 3. Durum çubuğunda tek ölçek

- **CAD:** yalnız çizim ölçeği görünür. Hücrede ölçeğin ikonu ve “1:100” yazar. Menüsü projenin birimine uyan
  ölçekleri ve “Ölçek yaz…”ı verir.
  - Birim imlecin koordinatının yanına geçer: “X 12.345  Y 67.890  m”.
  - Görünümün yakınlaşma ölçeği CAD'de gösterilmez.
- **CBS:** yalnız görünümün ölçeği görünür. QGIS'teki gibi “1:N” yazar; seçilir ya da yazılır (ADR 0165 §5).
- Durum çubuğunun genişlik hesabı bu hücreleri sayar.

## 4. Proje ayarları: “Birim ve ölçek”

“Ölçek ve yazılar” ile “Birimler ve hassasiyet” tek bölümdür: **Birim ve ölçek**. Bölümde sırasıyla şunlar yer alır:

- yerel projede çizim birimi;
- çizim ölçeği;
- uzunluk ve koordinat basamakları;
- alan birimi ve basamakları;
- açı birimi;
- önizleme;
- kâğıttaki yazı yükseklikleri.

Bölümün “Varsayılana döndür”ü ikisini birden döndürür.

## 5. Açılan çizimin ölçeği dosyadan

DWG, DXF ya da NCZ açılırken kullanıcıya bir şey sorulmaz. Ölçek sırayla şuradan gelir:

1. Dosyanın `$DIMSCALE`'i 1'den büyükse odur.
2. Değilse dosyadaki TEXT ve MTEXT yazılarının ortanca yüksekliği 2,5 kâğıt mm'ye bölünür. Sonuç birimin standart
   ölçeklerinden en yakınına oturtulur.
3. Yazı yoksa birimin kuralı uygulanır. NCZ 1:1000 kalır.

- Okuyucu stillerin kâğıt boylarını projenin ölçeğiyle hesaplar (ADR 0183 §7). Ölçek 1'den başka çıkarsa dosya bu
  ölçekle bir kez daha okunur.
- Var olan nesneler dosyadaki boylarında kalır. Yeni yazı ve ölçüler dosyanınkilerle aynı boyda çıkar.

## 6. Yazıların boyu: “Türe göre”

`graphics.annotationSize`'a **Türe göre** değeri gelir ve varsayılan budur:

- CAD projesinde Gerçek boy;
- CBS projesinde Kaybolmasın (bugünkü gibi).

Kullanıcı Görünüm sekmesinden açıkça başka bir kural seçebilir.

Gerçek boy kuralı (ADR 0205 §5'in Gerçek boy'u bununla değişir):

- Her yazı çizimdeki boyunda çizilir. Büyütme ve birbirinden ayırma yoktur. Bir yazı başka bir yazı yüzünden gizlenmez.
- Ekranda 3 pikselden küçük kalan yazı harfleriyle çizilmez. Kutusu boyunca yazının renginde soluk, ince bir şerit
  olarak görünür. Uzaktan bakınca yazıların yeri görünür, yakınlaşınca harfler gelir.
- Büyük yazı da çizilir. Resmi olamayacak kadar büyük dönük yazı ana hatlarıyla çizilir.
- CAD'de yazıların halesi yoktur (AutoCAD gibi). Yazının zemini (ADR 0145) halenin yerine geçmez; zemin kendi kuralıyla
  çizilir. Hale yalnız Kaybolmasın ve Ekranda sabit kurallarında kalır.

## 7. Hareket ederken

- Gerçek boy kuralında yazının yeri ve boyu görünümden bağımsızdır. ADR 0597'nin tuttuğu son yerleşim hareket
  boyunca doğrudur.
- Yerleşim artık görünümden büyük bir kutu için istenir: görünüm ve her yanda bir görünüm genişliği.
  - Görünüm bu kutunun içinde kaldıkça kayarken ve yakınlaşırken yazılar aynıdır.
  - Görünüm kutudan çıkınca yerleşim hareket sürerken de yeniden istenir. Ekrana yeni giren yazı eksik kalmaz.
- Kaybolmasın'da ADR 0597'nin kuralı değişmez.

## Ölçüm

Sahibin `deneme_suşehri.dxf` dosyası üzerinde ölçüldü: 71 820 nesne, 13 112 yazı. Pencere 1440 × 900, sürüm derlemesi,
wgpu, Intel Iris Xe. Makinenin NVIDIA sürücüsü 7.0.0-38 çekirdeğinde yüklü değildi.

| | Kaybolmasın | Gerçek boy |
| --- | --- | --- |
| Çalışma görünümünde çizilen yazı | 1 096 (büyütülmüş, ayıklanmış) | 7 980 (hepsi; çoğu şerit) |
| Kaydırma durulunca belirip kaybolan | 488 / 445 | 0 / 0 |
| Ölçü yazılınca belirip kaybolan | 0 / 0 | 1 (ölçünün kendisi) / 0 |
| Kaydırırken kare: işlemci + GPU | 4,7 + 30,0 ms | 14,3 + 25,4 ms |
| Tekerlekle kare: işlemci + GPU | 3,9 + 23,6 ms | 10,0 + 19,2 ms |

- Deponun cevabı her iki kuralda yaklaşık 10 ms'dir.
- Gerçek boyda işlemcinin payı yazı katmanının her karede yeniden kurulmasıdır: 7 980 yazı ve şeritleri. Hareket sürerken
  son çizilenin kaydırılıp ölçeklenmesi ayrı bir adımdır.
- GPU'nun payı Intel Iris Xe'dedir. Makinedeki RTX 3050 sürücüsü yüklenince yeniden ölçülecek.

## Doğrulama

- `kentos_project::wizard`'ın testleri ve fixtures/project/v1/wizard.json.
- Masaüstünde sihirbazın testleri ve resimleri `sihirbaz-*`.
- Durum çubuğunun ve ayarların resimleri `duzen-olcek-*`, `proje-*`.
- Gerçek boyun çekirdek testleri `store::legible`.
- `label_probe` ve `perf::file` sahibin dosyasında:
  - kayıp ve beliren yazı sayısı;
  - kare süreleri.
