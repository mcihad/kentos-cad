# ADR 0610 — Baskı mürekkebi, fiziksel kalem ve kaynak görünüşü

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-20.

## Tek anlam, ayrı kaynak

Baskı profili kaynak belgeyi veya katman stilini yeniden yazmaz. `plot_profile::apply` yalnız çıktı görünüşünü üretir; `Resolved` profil atamalarını sayfa başında indeksler. Kâğıt öğesi ve modelin çizilen katmanı ayrı hedeflerdir. Kâğıt haritasının çerçevesini değiştirmek model içeriğini yeniden sınıflandırmaz. ACI ataması ve adlı atama ADR 0609'daki açık anahtarlarla çözülür; RGB'den ACI tahmini yoktur.

- Renk: özgün, siyah, gri veya açık RGBA renk. Gri için 8 bit sRGB kanallarında `0.2126 R + 0.7152 G + 0.0722 B` kullanılır. Screening beyaz kâğıda doğru karışımdır; saydamlıkla karıştırılmaz.
- Opaklık: özgün bırakılabilir veya yüzdeyle değiştirilebilir. Yüzdeden RGBA baytına dönüşüm tamsayıyla en yakına yapılır; `%50 → 128`, kayan noktalı `50×2.55 → 127` hatası yoktur. Açık opaklık renk değerinin alpha'sından önceliklidir.
- Kâğıtta öğenin kendi opaklığı değiştirilir; üst grubun opaklığı korunur. Türetilmiş renge ikinci kez opaklık eklenmez. Harita ve grup kapsayıcıları genel adlı stilin opaklığını tekrar almaz; doğrudan kapsayıcı ataması ayrıca yapılabilir. Modelde renk/mürekkep alpha'sı çözülmüş nesne görünüşünün üstüne uygulanır.
- Beyaz, knockout/maske mürekkebidir; siyaha çevrilmez. Halo ve resim pikselleri özgün renklerini korur. Resim nesnesinin opaklığı değişebilir. Prosedürel desenin mürekkebi shader düzeyinde renklendirilir; resim dokusunun pikselleri renk tablosundan yeniden sınıflandırılmaz.
- Kalınlık tam mikrometre; model haritasında milimetreye çevrilir. `0` cihazın en ince çizgisidir. Çizgi aralıkları aynı fiziksel kâğıt birimindedir; boş liste sürekli çizgidir. Tek sayıda aralık PDF/SVG gibi iki kez yinelenir. `0 1` ve yuvarlak uç noktalı çizgidir; negatif ve tümü sıfır aralık reddedilir.
- Uç düz/yuvarlak/kare, birleşim köşeli/yuvarlak/pahlı olabilir. Köşeli birleşim sınırı PDF ve CPU çıktısında 10'dur. Kâğıt çıktı alanının ayrıca seçilen oranı bütün kâğıt mürekkebini de ölçekler; pafta alanı ADR 0608 gereği 1:1'dir.

## Çıktı yolları

Kâğıt profili ortak çekirdeğin Export görünüşünde uygulanır. PDF model vektörleri aynı renk/kalem fonksiyonlarını akış sırasında kullanır; bütün model noktaları ikinci bir harita kopyası yapılmaz. Profilin büyüttüğü çizgi/köşe için kaynak toplama payı ve PDF görünürlük testi de büyür.

Resim gerektiren bir haritada desteklenen vektör mürekkebi tiny-skia ile aynı kalınlık/çizgi/uç/birleşimden geçirilir; resim/desen batch'leri ortak CPU shader'ında işlenir. Batch sırası korunur. Başka bir sayfada kullanılan aynı master haritanın kâğıt sahibi istekte taşınır; masaüstü raster toplaması aktif sayfanın profilini diğer sayfaya taşımaz. Native kitap ve atlas da açık sayfa kimliğini kullanır.

Efektli bir CPU çizgisinde yuvarlak dışındaki birleşimi yeniden üretme desteklenmez; `plot_profile_effect_join` nedeni ile çıktı reddedilir. Profilin birleşimini Özgün yapmak veya efekti kaldırmak çözümüdür; yanlış birleşim sessizce basılmaz. Genel efektsiz CAD çizgileri, rastere düşen paftanın içinde de bütün uç/birleşimleri kullanır. Piksel/efekt boyamaları geometriye ait olmayan linetype/cap/join özelliği kazanmış sayılmaz.

## Arayüz ve otomasyon

Sayfa → Baskı profilleri: adlı/renk profili oluştur, stil ekle/sil, açık anahtar, varsayılan adlı stil, renk seçici, yoğunluk, kalınlık, aralıklar, opaklık, uç/birleşim; katmana veya seçili kâğıt öğelerine atama ve atamayı kaldırma. Kitaplığa kaydet veya kaydet ve paftaya uygula tek geri alma adımıdır. Aynı `savePlotProfiles` ve `setSheetPlot` işlemleri Python/AI'da tipli olarak vardır. Uygulama ayarlara profili yakalar; eski pafta kitaplık değişince kendiliğinden değişmez.

## Bağımsız kontroller

`plot_ink_cases.py` KentOS yüklemeden 20 renk/alpha/kalem beklentisi kurar; sıfır ve tek sayıda aralık, bütün uç/birleşimler ve büyük koordinatlar dahil. PDF'nin içerik operatörleri ayrı okuyucuyla ölçülür; renk/alpha ayrıca Poppler ile beyaz kâğıtta kontrol edilir. CPU kalem kontrolü 508 dpi'da 0.05 mm adımlarını tam piksele düşürür; 0.04 piksel toleransı kullanır. Daha düşük dpi'da anti-alias kenarı piksel çözünürlüğüyle sınırlıdır; bu, vektör PDF'nin kalem ölçüsünün yerine geçmez. Kâğıt/model geometri toleransı 0.002 mm, kalem/dash içerik toleransı 0.0002 mm'dir. Karşılaştırmalı gerçek paftalar CAD-SHEET-21'dedir.

Kabul: 398 çekirdek + 71 bütünleşme (önceden var olan 4 ignore), native plot 12 + 30, UI 46 + 42 + 7, 237 tam Python testi; MCP gerçek PDF, katalog, SDK drift, clippy, bağımlılık ve WASM smoke geçti. GPU ekranında gerçek fareyle kaydet ve büyük yazıda kaydırma çalıştırıldı. Açılıp incelenen görüntüler: `.run/shots/sheet20-ink-{top,bottom}-{light,dark}-13.png`, büyük yazı `dark-18`; altı sayfalı `.run/shots/sheet20-ink.pdf` üçüncü sayfası da Poppler ile açıldı.
