# ADR 0611 — Karşılaştırmalı baskı paftaları ve PDF dolgu boyası

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-21.

`plot_comparison_cases.py` KentOS çalıştırmadan model ve kâğıt kontrol noktalarını kurar. Renkli, tek renk ve gri üç A3 pafta aynı 1:50 görünüşü taşır: tarama üstünde maskeli yazı, çizgilerin arasında wipeout ve daha önde mavi çizgi, taramanın üstünde dolgu, bağlı PNG, gerçek dosyadan eklenmiş PDF altlığı. Paftaların tamamı hem vektör yoluyla hem PNG'nin gerektirdiği CPU resim yoluyla üretilir. Kaynak resim pikselleri baskı profilinin çizgi rengine dönüştürülmez.

Bağımsız kaynak PDF testte standart PDF nesneleri ve xref tablosuyla yazılır; KentOS'un kendi PDF yazıcısının çıktısı referans değildir. Kaynakta kırmızı dolgu, üstünde beyaz dolgu ve siyah sınır bulunur. Poppler sonucu kontrol noktalarında ve tarama/wipeout iç bölgelerinin tamamında ölçer. Yalnız çizgilerin arasındaki boşluğu örnekleyip tarama sırası geçti denmez. Yazı maskesinin kontrolü tam arkadaki tarama çizgisine konur. PDF altlığının Yazdır kapalı hâlinde panelin tamamı beyaz olmalıdır.

## Bulunan ve düzeltilen eksik

PDF okuyucu dolu yolları alan geometri olarak saklıyordu; blok çizeri bunları normal CAD alan sınırı gibi, dolgusuz çiziyordu. `PieceStroke::pdf_fill` yalnız türetilmiş blok boyama bilgisidir: PDF altlığının alanları opak dolgu boyasıyla, ayrı stroke yolları ayrı çizgiyle çizilir. Model geometrisi ve PDF'nin kâğıt metresi değişmez. Beyaz dolgu artık ithalde atılmaz; önceki mürekkebi örten boyadır, tek renk seçeneğinde siyaha dönmez.

Altlık görünüşü özyinelemeli flatten sırasında uygulanır; daha sonra önbelleğe alınmış ebeveyne eklenen bir değişiklik değildir. Böylece PDF altlığını içeren başka bir blok da dolgu, soldurma ve tek renk görünüşünü taşır. Hem JSON hem tipli sözleşme → çekirdek köprüleri varsayılan PDF görünüşünü de tanır. 20 bağımsız iç içe blok/soldurma/tek renk/sıra durumu kaynak geometrisinin değişmediğini doğrular.

PDF okuyucusunun iç resim/gölgeleme gibi mevcut destek sınırları değişmedi; bunlar ekleme komutunun `pdf_left_out` uyarısıdır. Bu pafta gerçek PNG altlığını ve bütün öğeleri desteklenen vektör kaynak PDF'yi ayrı ayrı sınar; bütün PDF standardının desteklendiğini iddia etmez.

## Yeniden üretme

- `python3 scripts/fixtures/plot_comparison_cases.py --check`
- Python test ortamında `python -m unittest discover -s python/tests -p test_plot_comparison.py`.
- `python python/tests/test_plot_comparison.py --artifact`: `.run/shots/sheet21/comparison.pdf`; kaynak PNG ve kaynak PDF de yanındadır.
- Poppler ile 127 dpi (5 px/mm) kontrol; renk toleransı 3/255. A3 nokta nicemlemesinin son raster satırı yukarı yuvarlanabilir; fiziksel PDF sayfa ölçüsü ayrı ölçülür.

Kabul: 515 çekirdek geometri testi (5 önceden var olan ignore), 20 PDF boyama/soldurma/iç içe blok durumu, 4 xref bütünleşmesi ve ortak xref komut durumları; 239 tam Python testi, clippy ve WASM build/smoke geçti. Clippy'nin mevcut NURBS testindeki platforma özgü `hypot` kullanımını ortak `js_hypot` ile değiştirmek de bu denetimin küçük onarımıdır. Üç gerçek çıktı görüntüsü (`.run/shots/sheet21/view-{1,2,3}.png`) açılarak dolgu, beyaz boşluk, maske ve sıralama incelendi.
