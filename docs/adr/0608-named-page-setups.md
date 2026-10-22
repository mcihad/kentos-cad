# ADR 0608 — Adlı sayfa düzenleri ve fiziksel baskı dönüşümü

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-18.

## Karar

- `SheetBook.pageSetups` adlı kayıtların kâğıdını, yönünü, kenar boşluklarını ve baskı ayarlarını taşır. Önceki kitaplarda alan yoktur; boş liste yazılmaz.
- `PlotSettings`: pafta/tüm öğeler/pencere alanı, kesin kâğıt oranı/sığdır, yatay/düşey kayma, ortalama, profil kimliği ve PDF/adlı sistem cihazı. Kâğıt uzunlukları tam mikrometredir.
- `savePageSetups`, `applyPageSetup`, `setSheetPlot`, mevcut `cad.sheets.edit` işlemleridir. Python ve AI aynı işlemleri ve ters işlemleri kullanır. Uygulama, kâğıdı ortak relayout hesabıyla değiştirir; eski sayfanın tamamı ters işlemde korunur.
- Uygulanan ayarlar paftaya kopyalanır. Kayıt silinince veya değiştirilince daha önce hazırlanmış bir pafta kendiliğinden değişmez; yeniden uygulanır.
- Pafta alanı 1:1 basılır. Viewport oranı ile çıktı oranı ayrı kavramlardır. Pencere/tüm öğeler modları açık oran ya da yazılabilir alana sığdırma kullanır. Kayma sağa/aşağı pozitif; ortalama yazılabilir alanda yapılır.
- Ortak `print_setup::Plan`, hedef ve kaynak kırpmasını, oranı ve kaymayı verir. PDF aynı dönüşümü bütün yazı, vektör ve resimlere uygular. GeoPDF kayıt kutuları da aynı dönüşümden geçer; coğrafi kaynak köşeleri değişmez.
- Tüm öğeler alanı görünür, basılabilir öğelerin dönük çerçevelerini kapsar; ana sayfa bu kâğıda yerleştirilir. Grup kapsayıcısının kendi kutusu mürekkep sayılmaz.
- Ayarsız eski paftanın çıktısı değişmez. Profil kimliği çözülemezse PDF `unknown_plot_profile` ile durur; görünüş sessizce değiştirilmez. Profil modeli CAD-SHEET-19'dadır.
- Masaüstünde Sayfa → Sayfa düzenleri: kayıt yükle/sil, düzenle, bu paftaya uygula, kaydet ve uygula. Kâğıt/yön/kenar ayarları aynı Sayfa bölümündedir ve kayda alınır. Cihaz adı taşınabilir bir tercihtir; fiziksel yazdırmayı ev sahibi doğrular. PDF üretimi cihaz adına bağlı değildir.

## Bağımsız doğrulama

`page_setup_cases.py` KentOS yüklemeden 20 dönüşüm beklentisi üretir: alanlar, oranlar, sığdırma, ortalama, kayma ve cihaz. Çekirdek uygulama/ters işlem/kayıt, native PDF/Poppler, Python kayıt-yeniden açma ve MCP çıktısı; masaüstü mesajları ve açık/koyu/büyük yazılı GPU ekranları kabul kapsamıdır. Gerçek yazıcı kalibrasyonu bu testlerin yerine geçmiş sayılmaz. PDF vektör köşeleri bağımsız içerik okuyucusunda 0,002 mm toleransla denetlenir. 389 çekirdek testi, 66 bütünleşim testi (önceden var olan dört altın görüntü denetimi ayrı), 26 native çıktı testi, 234 Python testi, katalog, clippy, bağımlılık denetimi ve ortak WASM fixture seti geçti. Sayfa düzeni penceresinin üç GPU görüntüsü ve gerçek PDF açılarak incelendi.

## Başvurular

[Autodesk named page setups](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-2C5C9DE4-0740-4D7F-A781-745A04F588B0.htm), [Layout alanının 1:1 davranışı](https://help.autodesk.com/cloudhelp/2020/ENG/AutoCAD-Core/files/step_1_create_dwf_or_dwfx_files_using_publish_wrkflw.htm), [fiziksel layout ölçeği](https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/Plotting-a-layout-in-paperspace-results-in-the-the-drawing-not-plotting-to-scale-in-AutoCAD.html).
