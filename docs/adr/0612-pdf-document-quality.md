# ADR 0612 — PDF yazısı, eğri doğruluğu ve pafta çözünürlüğü

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-22.

## Bağımsız belge kontrolü

`plot_document_cases.py` KentOS çalıştırmadan 22 font yüzünün beklentisini kaynak TTF dosyasının SFNT PostScript adından okur. Her A3 sayfada Türkçe/ligatür denemesi, analitik kâğıt dairesi, elips, ikinci derece Bézier ve rasyonel çeyrek daire; modelde 5 m çizgi, daire, tarama ve yazı bulunur. Kaynak geometri çıktıdan sonra aynıdır. PDF okunarak şu özellikler sınanır:

- Poppler `pdffonts`: kullanılan 22 yüzün tamamı gömülü alt küme ve ToUnicode haritalı; beklenen gerçek font adları vardır.
- `pdftotext`: her sayfanın Türkçe metni ve model yazısı aynen bulunur. Bunlar resim veya harflerin çizgiye dönüştürülmüş hâli değildir.
- Bağımsız içerik okuyucusu: sayfa ölçüsü ve kâğıt dairesi/elips/Bézier/rasyonel eğri 0,002 mm toleransla kontrol edilir. Kübik eğrilerin iç noktaları da örneklenir; yalnız uçların doğru olması yeterli değildir.
- Model dairesinin kiriş orta noktası 0,01 mm içinde, uçları 0,002 mm içindedir; tarama çizgileri bölgenin içindedir. Vektör belge içinde resim XObject'i yoktur.
- Ayrı 20 raster sayfasında 72–600 dpi hedeflerinin piksel boyutları bağımsız hesaplanır ve PDF'den okunur. Bir pafta diğerinin en yüksek dpi değerine zorlanmaz.

## Yakalanan iki eksik

Model dairesi sabit 72 kenarla gidiyordu; 20 mm kâğıt yarıçapında kiriş hatası kontrolü geçmedi. `plot_outline` çember/yay/elips, fit-point spline/NURBS ve ince bulgeli çizgi için kâğıtta 0,005 mm kiriş toleransını model metresine çevirir. Konikte `max(|p''|) h²/8`, spline'da Bézier ikinci türevinin kontrol sınırı kullanılır; NURBS mevcut bounded tessellation yolundadır. En çok 65536 üretilmiş nokta; sınır aşılınca daha kaba çizme ya da kontrol poligonunu basma yoktur, çıktı reddedilir. Kaynak ölçü geometrisi değişmez. Çıktı alanının ayrıca seçilen kâğıt oranı bütün grafiğe uygulandığı gibi bu kâğıt toleransını da çarpar.

Bu seçim açık `View.plot_precision` bayrağıdır. `screen=false` tek başına baskı anlamına gelmez; stil kitaplığı ve mevcut ekran davranışı değiştirilmez. Donmuş stil durumları aynen geçer. Native vektör ve CPU resim toplayıcısı baskı bayrağını kullanır; masaüstü dışa aktarma resimleri de aynı CPU yoluna gider. Ekranın normal sahnesi kendi mevcut ayrıntı hesabını korur.

Native raster toplayıcısı daha önce bütün kitabın en yüksek çözünürlüğünü kullanıyordu; artık sahibi olan paftanın dpi değerini kullanır. Ortak ana sayfanın her sayfadaki sahibi de korunur.

## Hata ile destek sınırı farklıdır

Stilin vektör karşılığı olmaması resim yolunu seçebilir; geometri hesabının başarısız olması bunu yapamaz. `VectorIssue` bu ikisini ayırır. Native stil toplayıcısı hatayı boş katmana çevirmeden geçirir; genişletilmiş görünüm kapsamıyla kutusu kesişmeyen nesneler sınanmaz, geometri görünmese de görünür yazıları olan katman kaybolmaz. Masaüstü `MapPainter::export_content` aynı ayrımı taşır. PDF/SVG için resim gerçekten boyanamıyorsa gri yer tutucuyla başarılı dosya üretilmez. Masaüstünde gerçek aşırı eğri denemesi de çıktıyı reddeder.

## Kabul kanıtı

- 22 gerçek PDF sayfası, 20 dpi durumu, görünür/görünmeyen aşırı eğri; native plot 12 + 33 test.
- 20 konik/ölçek referansı ve bütçe reddi; stil çekirdeği 34 + 9 test, donmuş durumlar dahil.
- UI 46 + 44 + 7; iki gerçek hata yolu ve mevcut dışa aktarma kontrolleri.
- 240 tam Python testi; KCAD yeniden açma, font/Unicode ve geometri Python'dan da doğrulandı. MCP gerçek PDF ve masaüstü gerçek hata denemesi geçti.
- Clippy, geometri WASM kontrolü, pafta WASM build/smoke; SDK ve bağımlılık denetimi.
- `.run/shots/sheet22-document.pdf` ilk ve son sayfası Poppler ile açılıp incelendi: `sheet22-document-{first,last}.png`.
