# ADR 0594: Açık CAD görünüm penceresi kaynağı, oranı ve kilidi

Tarih: 22 Ekim 2026 · CAD-SHEET-09

Görünüm penceresi mevcut harita öğesidir; modelin nesnelerini çoğaltmaz. Kâğıt çerçevesi, gerçek içerik çerçevesi (iç boşluk/yazı bandı hariç), model merkezi, model kapsamı ve görünüm dönüşü ayrı anlam taşır.

- `MapItem.source`: `drawing` mevcut çizimi; `viewSnapshot` aynı çizimin adlı görünümünün açık kopyasını belirtir. Kopya, mevcut `NamedView` sözleşmesinin kimlik/ad, merkez, dünya genişlik/yükseklik ve twist değerlerini saklar. Eski kimliksiz görünüm mevcut `view_key` kuralıyla kimlik alır. Bu bir anlık kopyadır; canlı bağ, görünüm değişikliğini takip etme ve callout etkisi CAD-SHEET-14'tür.
- `FixedView.scale` eski tamsayı 1:N değeri olarak okunmaya devam eder. İsteğe bağlı `ratio: {paper, model}` iki uzunluğu **aynı fiziksel birimde** verir; varsa gerçek payda `model/paper` olur. 1:37,5, 2:1, 3:2 gibi değerler çizim/PDF/raster hattına `f64` ile, tamsayıya kesilmeden gider. Geçerli payda aralığı 10⁻⁶–10⁸; iki sayı da sonlu ve pozitif olmalıdır. Standart/özel oran kitaplığının tamamı CAD-SHEET-11'de kalır.
- `cad.sheets.edit` içindeki tipli `setViewport` işlemi; kaynak, merkez **veya** kapsam, oran **veya** eski 1:N ölçek, dönüş ve kilidi tek adımda düzenler. Çelişkili istekler `ambiguous_source` ile reddedilir. Adlı görünüm kopyası mevcut çerçeveye, iki yöndeki boyun büyüğüyle gerçek oranında sığar. Kapsamdan sığdırma dönüşü hesaba katar; geometriyi değiştirmez.
- `cad.sheets.set` zaten dört model köşesini ve bbox'ı verir; artık ölçek çıktısı gerçek kesirli paydadır. Ayrıntılı kitap kaynak kopyası, merkez, oran ve kilidi de verir. `SheetMapInfo.scale` ve `SetFrame.scale` ondalık sayıya genişler; eski tamsayı cevaplar sayısal olarak aynı kalır. Çizim planının eski tamsayı ölçek JSON yazımı korunur.
- `viewLocked` kameranın merkezi/ölçeği/dönüşü/kaynağını korur; kâğıt çerçevesinin kilidinden ayrıdır. `setItemProps` ile aynı değişiklik de denetlenir. Kilidi açma ve kamera değişikliği aynı açık `setViewport` işleminde yapılabilir. Çerçeveyi kâğıtta taşımak ve katman görünüşünü değiştirmek görünüm kilidince engellenmez. Bu bir düzenleme davranışıdır, kullanıcı yetkilendirmesinin yerine geçmez.
- `clip: frame` içerik çerçevesinde dikdörtgen kırpmayı açıkça belirtir. Ekran, SVG, vektör PDF ve resim PDF aynı pencerenin kapsamını kullanır. Serbest dikdörtgen/çokgen kırpma ve çerçevenin plot/no-plot seçenekleri CAD-SHEET-12'dir.
- Var olan katman durumları, seçili katman listesi ve öğeye ait freeze/plot/renk/çizgi tipi/kalınlık ezmeleri korunur. Bunlar kamera kilidinden bağımsızdır; ileri annotative temsil seçimi CAD-SHEET-15'tir.
- Arayüz Kaynak seçicisini, oran alanlarını, merkezi ve dönüşü gösterir. Çizim ölçeğini al düğmesi projenin gerçek ondalık paydasını alır; 37,5 değeri 38’e yuvarlanmaz. Görünüm kilidi üstteki anahtardadır; kilitliyken kamera alanları salt okunur ve sığdırma düğmeleri kapalıdır. Çoklu seçimde yerleştirilmemiş merkez/olmayan oran için yarım nesne yaması üretilmez. Python ve MCP aynı `setViewport` şemasını kullanır.
- Tam sayı ölçeğe dönmek eski oranı açıkça kaldırır. Nesneye sığdırma, varsa önceki özel oranı kaldırıp hesapladığı ölçeği uygular; tek noktaya merkezlenirken mevcut gerçek oran korunur. Bütün ters işlemler kopya kaynağını, oranı ve kilidi geri getirir. Önceden bulunmayan varsayılan alan için ters yamadaki `null`, varsayılan değere döner.
- Pafta çizimlerinin annotative/stil önbellekleri gerçek ölçeğin `f64` bitleriyle anahtarlanır; örneğin 37,1 ile 37,9 aynı tamsayı önbelleği paylaşmaz.

## CAD alışverişi

DXF/DWG köprüsü kesirli görünüm paydasını ve görünüm kilidini taşır. DXF VIEWPORT grup 90'ın `0x4000` kilit bayrağı okunur/yazılır; yazıcı artık görünümü kapatan `0x20000` yerine açık görünüm için `0x8000` yazar. Tanım [Autodesk DXF VIEWPORT başvurusuyla](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-2602B0FB-02E4-4B9A-B03C-B1D904753D34.htm) doğrulandı.

Dışa aktarım gerçek içerik çerçevesinin kâğıt boyunu/merkezini kullanır. Kâğıt çerçevesinin çeyrek dönüşlerinde genişlik/boy değiştirilir ve içerik twist'i birleştirilir. Eğik kâğıt çerçevesinin dikdörtgen olmayan sınırı ile viewport katman ezmelerinin foreign CAD karşılığı bu köprüde tamamlanmış değildir; açık kayıp raporu verilir. KCAD kaynağı ve PDF görünüşü korunur. Rastgele **içerik** dönüşü desteklidir; eğik kâğıt sınırının çokgen klibi CAD-SHEET-12 kapsamındadır.

## Doğrulama

- `scripts/fixtures/viewport_cases.py`, KentOS çağırmadan 32 oran/dönüş/kapsam/adlı görünüm/kilit ve ret durumu üretir. Aynı dosya shared core, native komut, gerçek tasarımcı ve Python'da yürür; model nesneleri, ters işlemler ve KCAD yeniden açma karşılaştırılır.
- Ham özellik yaması kilidi aşamaz; kilitli kamera korunurken kâğıt çerçevesi taşınabilir.
- Python + bağımsız ezdxf okuyucusu: 1:37,5, 2:1 ve 3:2; gerçek 380×220 mm içerik, 30° twist, kilit bayrağı, kapalı olmama, tekrar içe aktarma ve değişmeyen model.
- Gerçek MCP changeset'i: kesirli görünüm oluşturma/kilitleme, doğrudan kamera yamasının reddi, tek undo ile eski kitaba dönüş.
- `sheet09_viewport_screens`: 15° döndürülmüş 1:37,5 oda, adlı kaynak ve kilit; açık/koyu 1440×900 ile 18 px yazılı 1100×650 GPU ekranları. Aynı kitaptan PDF ve Poppler görüntüsü açılarak incelenir. Büyük yazı kontrolünde kilit anahtarı üst sıraya alınıp açıklamalar kısaltıldı.
- Web arayüzü ADR 0228 gereği erteli; kaynak/oran/kilit, yeni işlem ve ondalık ölçek yolları web'in kendi uygulamasına dönüşte bağlanmalıdır.

Doğrulama: 218 Python testi ve üç oran için gerçek DXF + DWG yeniden açma; 32 ortak görünüm senaryosu; shared/native/headless/MCP testleri ve değişmeyen SVG/PDF golden dosyaları geçti. GPU kâğıt bölgesinde açık/koyu 13 px ekranlarda 1110, dar 18 px ekranda 545 koyu geometri pikseli bulundu; kilit anahtarı her iki boyutta görünür.

Son kabul: 714 katalog/sözleşme testi, native/plot/headless/MCP takımları, 379 sheet birim + 54 entegrasyon, 44 UI birim + 33 tasarımcı + 7 kitaplık; clippy, optimize sheet WASM ve 47 crate / 61 hedef bağımlılık denetimi geçti. Standart PDF/SVG golden çıktıları değişmedi.
