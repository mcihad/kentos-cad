# ADR 0596: CAD standart ölçekleri ve kayıpsız özel oran girişi

Tarih: 22 Ekim 2026 · CAD-SHEET-11

Mevcut gerçek `ScaleRatio {paper, model}` ve ortak fiziksel hesap korunur. Kullanıcının seçmediği yuvarlama artık arayüz işleminin parçası değildir.

- CAD görünüm penceresinin standart ölçekleri `viewport::standard_ratios()` tablosundadır: büyütmeler 10:1/5:1/2:1, 1:1 ve küçültmeler 1:2 … 1:5000; mimari 1:20/1:50/1:100 dahildir. GIS atlasının mevcut sığdırma ölçek tablosu değiştirilmez.
- Kaynak/kilit bölümündeki standart ölçek seçicisi doğrudan `setViewport` oranı yazar. Standart olmayan mevcut oran “Özel oran” olarak görünür; seçici açılırken veya başka alan düzenlenirken standart bir değere çekilmez.
- Önceki “1:N kullan” işlemi 37,5'i 38'e, büyütmeyi 1'e çevirebiliyordu. Yerine “Oranı 1:N yaz” açıkça `{paper:1, model:denominator}` yazar; tamsayıya yuvarlama yapmaz. Legacy `scale` dosya/komut alanı uyumluluk için kalır; arayüzün yeni 1:N alanı gerçek oran yazar.
- Pay/payda ve gerçek 1:N alanları kayıpsız sayısal metindir. `f64`'ün round-trip metni kullanılır; diğer sayı alanlarının altı basamaklı gösterim hassasiyeti bu alanlara uygulanmaz. Hem nokta hem virgüllü sayı kabul edilir; sonlu olmayan/bozuk metin görünür hatadır, kaynak değişmez. Pozitiflik ve payda sınırı ortak çekirdek tarafından denetlenir.
- Ham oran çifti, kullanıcının açık 1:N normalizasyonu dışında korunur. Örneğin 3:2 saklanır; başka özellik düzenlenirken 1:0,666… biçimine dönüştürülmez. Dosya, şablon, Python/MCP, undo/redo aynı modeli kullanır.
- `cad.sheets.set(details=true)` artık `standardRatios` tablosunu tipli olarak döner. Python SDK bunu üretir; otomasyon arayüzdeki seçenekleri aynı kaynaktan keşfeder. Yeni, ikinci bir ölçek değiştirme komutu yoktur.

Bağımsız generator 24 ek standart/özel/küçük-büyük sınır ve 12 basamaktan uzun oranı mevcut kamera kabulüne ekler; toplam 56 senaryo core/native/tasarımcı/Python ve mevcut WASM arayüzünde yürür. Kaynak çifti, kamera, dosyaya yeniden okuma ve ters işlem korunur. Arayüzün gerçek alan mesajları da paydanın tam yazımını, 24 senaryoda tek undo'yu ve başka alan değiştirirken eski çiftin korunmasını sınar; 18 hazır oran tek ortak işlemden geçirilir.

`sheet11_exact_scale_screens` gerçek GPU'da açık/koyu ve büyük yazıyla 1:37.123456789123'ü gösterir; küçük pencerede aşağı kaydırılmış alan görüntüsü de alınır. Görüntüler açılıp incelenir; yeni seçici ve tam sayı metni kontrol edilir. Web uygulaması ADR 0228 gereği erteli; web dönüşünde tam oran alanları/ortak standart tablo ve aynı 56 kabul uygulanmalıdır.

Kabul kanıtı: 379 sheet birim + 56 entegrasyon; 45 UI birim + 34 tasarımcı + 7 kitaplık; 714 sözleşme, native/headless/MCP takımları; 12 pafta Python testi, 56 WASM viewport + 40 fiziksel projeksiyon; clippy temiz. Gerçek fare kilit/undo kontrolü oranı değiştirmedi. Uzun ölçek açıklaması dar panelde ek metinle taşmasın diye ayrı kısa satıra alındı; giriş kutuları normal yatay metin kaydırmasını korur.
