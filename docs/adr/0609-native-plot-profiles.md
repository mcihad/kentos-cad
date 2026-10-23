# ADR 0609 — Ayrı CTB/STB anahtarları ve yerel baskı profili

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-19; görünüşün uygulanması CAD-SHEET-20.

## Veri ve otorite

`SheetBook.plotProfiles` belgenin parçasıdır. `savePlotProfiles` mevcut `cad.sheets.edit` komutunun tipli, doğrulanan ve geri alınan işlemidir. Sayfa düzeni profili kimliğiyle seçer; eksik profil çıktı sırasında ret verir. Arayüz aynı kitap listesini gösterir. Profil düzenlemesi belge revizyonuna dahildir; çıktı başlangıcının sabitlenmesi CAD-SHEET-29'dadır. Uygulama `profileSnapshot` ile profilin kendisini de yakalar; sonradan kitaplık değişikliği hazırlanmış paftayı değiştirmez. Açık, eski bir kimlik başvurusu kitaplıktan çözülür; yeniden uygulama güncel profili yakalar. Şablon çıkarma profili kendi içine alır ve kaynak preset kimliğini temizler; başka projede ek kitaplık gerektirmez.

- `colorDependent` tablosu yalnız ACI 1–255 anahtarı taşır. 0/256 renk değil ByBlock/ByLayer kaynağıdır; tabloya stil diye yazılmaz.
- `named` tablosu yalnız ad taşır. Aynı görünen `aci:1` ve `named:1` farklıdır; karışık tablo reddedilir. `Normal` nesne görünüşüdür; kullanıcı tarafından yeniden tanımlanmaz. Atama kaldırılarak nesne görünüşüne dönülür.
- Atama modeli model katmanı veya kâğıt öğesini açık tablo anahtarına bağlar. Adlı profil ayrıca varsayılan bir stil seçebilir. Atanmayan renk bağımlı kaynak nesne görünüşünü korur; en yakın RGB'den ACI tahmini yapılmaz. Özellikle ACI 1 ve 10 gibi aynı RGB'yi veren indeksler birleştirilmez.
- Modelde renk kipinin nesne/siyah/gri/açık renk seçimi, screening, kâğıt çizgi kalınlığı, açık çizgi aralıkları, opaklık, cap ve join bulunur. `None` nesneden al demektir; boş çizgi aralıkları sürekli çizgidir. Uzunluk mikrometre, screening ve opaklık yüzde olarak açık birimlidir.

## Kayıt ve kopyalama bütünlüğü

Son pafta silinse veya ilk pafta henüz oluşturulmasa da profil ve sayfa düzeni kitaplıkları boş kitap sayılmaz; KCAD bunları saklar. `.kpafta` yanına ekleme/yerine koyma iki kitaplığı da taşır. Yanına eklemede ad çakışması açık `(2)` ekiyle çözülür; kimlikler kategori bazında yenilenir. Model katmanı/view kimlikleri ve kullanıcı metni değiştirilmez. Özellikle profil kimliği, kâğıt öğesi kimliği ve bir model katmanı aynı yazı olsa da farklı sahipliklerdir. Profilin kâğıt hedefleri yeni öğe kimliklerini izler. Çoğaltma ve şablondan üretim de bu hedefleri eşler; kaynak kitap değişmez.

## Alışveriş stratejisi

Yerel taşınabilir dosya **`.kplot` / `kentos.plot/1` JSON**'dur. `plot_profile::read/write`, Python/AI'ın okuyup yazabildiği aynı tipli modeli kullanır; kitap ve paket de aynı modeli taşır. Bilinmeyen alan kaybolmaz: ret verilir. Okuma 4 MiB, tablo 4096 stil/100000 atama ile sınırlıdır. Kimlik, ad, anahtar ve hedef tekliği, oranlar, renkler ve çizgi boyları doğrulanır.

**İkili `.ctb/.stb` okuyucusu/yazıcısı bu kararla var olmuş sayılmaz.** Bu imzalar `unsupported_plot_profile` ile açıkça reddedilir. JSON'a `.ctb` adı verilmez; mevcut dosya yeniden kaydedilerek ezilmez. Önce bu yerel biçimde onaylanmış eşleme kullanılır. İkili codec eklendiğinde:

1. Kaynağın SHA-256'sı ve biçimi tutulur; açılmış veri boyu ve sayım sınırlandırılır.
2. CTB'nin gerçek indeksleri, STB'nin gerçek adları kullanılır. RGB'den indeks ve renkten ad tahmin edilmez. DWG/DXF kaynak metadata'sı ancak hâlâ eşleştiği kanıtlanınca atamaya dönüştürülür; değişmiş renk verisi eski sınıflandırmayı izleyemez.
3. Her özellik yukarıdaki birimli modele açık eşlenir. Dither, fill style, adaptive line pattern ve sürücüye özgü alanlar için ya doğrulanmış karşılık ya adlandırılmış kayıp/ret gerekir; varsayılan sessiz kırpma yasaktır.
4. Kaynakta bulunan bilinmeyen alanlar ham kaynakla korunabilir; yeni yerel profilin onlara uyduğu iddia edilmez. Hedef ikili biçimde temsil edilemeyen profil yazılmaz.
5. Bağımsız gerçek CTB/STB korpusu ve AutoCAD çıktısı ile doğrulanmadan tam binary uyumluluk işareti konmaz. Çalışma zamanı Python/ezdxf bağımlılığı eklenmez.

Bu ayrım CAD-SHEET-19'un istediği yerel model ve alışveriş stratejisidir; bütün AutoCAD plotter biçimlerinin tamamlandığı iddiası değildir.

## Başvurular

[Autodesk plot styles](https://help.autodesk.com/cloudhelp/2020/ENG/AutoCAD-Core/files/GUID-929FE8EC-EFE3-43BB-A79F-4FF509A91D5A.htm), [renk bağımlı tablolar](https://help.autodesk.com/cloudhelp/2023/ENU/DWGTrueView/files/GUID-8C13707B-0130-4E48-96FB-E660BE46AC96.htm), [screening](https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/Set-up-plot-screening.html), [ezdxf CTB/STB codec başvurusu](https://ezdxf.readthedocs.io/en/stable/addons/acadctb.html). Son kaynak yalnız bağımsız biçim başvurusudur; ürüne Python bağımlılığı eklenmez.

## Doğrulama

20 bağımsız profil anahtarı ve aynı 20 profilin kaynak taşıma/kopyalama/şablon durumları çekirdekte geçti. 398 çekirdek testi ve 69 bütünleşim testi (önceden var olan dört altın resim denetimi ayrı), 12+26 native plot testi, 46+41+7 UI testi, 236 Python testi, MCP'nin 20 profil akışı, katalog, clippy, bağımlılık yönü ve ortak WASM seti geçti. Masaüstünde profil seçicisi fareyle açıldı; yukarı/Enter ile başka profil seçilip kayıtlı kopyası denetlendi. Açık/koyu/büyük yazılı üç ekran ve açılır liste görüntüsü açılarak incelendi.
