# ADR 9327: Ölçek her yerde tek biçimde yazılır; Tümünü göster ve Genel bakış veri katmanlarının kapsamını kaynaktan alır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  7 ve 8, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:**
  - [ADR 0181](0181-overview-and-magnifier.md) (Genel bakış ve Büyüteç);
  - [ADR 0165](0165-project-types-cad-gis.md) §5 ve [ADR 0626](0626-units-scale-and-true-size-texts.md) §3 (ölçek seçici, tek ölçek hücresi);
  - [ADR 0637](0637-up-to-four-views.md) (bölünmüş görünüm ve rozetleri);
  - [ADR 0141](0141-netcad-hand-navigation-query-selection.md) (gezinme);
  - [ADR 9101](9101-data-layers.md) §5 (veri katmanının kapsamı), [ADR 9203](9203-show-everything-without-far-objects.md) (Tümünü göster ve açılıştaki görünüm).

## Bağlam

- **Madde 7:** yalnız GeoPackage veri katmanı olan bir haritada Genel bakış “Çizimde gösterilecek nesne yok” dedi. Ana
  harita iki noktayı çiziyor, tablo iki satırı okuyordu.
- **Madde 8:** üç bağlı görünümde tekerlekle en yakına inildi.
  - Rozet `2.646:1`, durum çubuğu `1:0`, ölçek çubuğu `0.0010 cm` yazdı.
  - Harita › Tümünü göster beklenen kadraja dönmedi; kayıtlı görünümle dönüldü.
- **Kod:**
  - Genel bakış resmini ve kapsamını yalnız çizimin uzamsal deposundan alıyordu. Veri katmanının satırları çizimde
    değildir; okuyan ayrı bir iş parçacığıdır.
  - Veri katmanı, tablosunun eklendiği andaki kutusunu tutuyordu (`DataLayer.extent`). Boş yapılan katmanın kutusu
    yoktu; düzenleme oturumunun kaydı onu güncellemiyordu. Tümünü göster ve Katmana yakınlaştır bu kutuya bakıyordu:
    yeni katmana yazılan noktalar hiç gösterilmedi.
  - Durum çubuğu ölçeği `1:` ve yuvarlanan paydayla yazıyordu; rozetin kendi biçimleyicisi vardı (UR-098). Coğrafi
    projede rozet dereceyle, durum çubuğu metreyle hesaplıyordu.
  - Kamera boyu olmayan kutuyu (tek nokta) en derin ölçeğe, 2 646:1'e sığdırıyordu.
  - Ölçek çubuğu santimetrenin altını da santimetreyle yazıyordu.

## Araştırma

- **Ölçeğin yazımı:** QGIS'in ölçek kutusu 1'in altındaki ölçeği `N:1` yazar (N paydanın tersi, yuvarlanır), üstünü
  `1:N`; sayı yerelin basamak ayracıyla yazılır
  ([QgsScaleComboBox::toString](https://api.qgis.org/api/3.44/qgsscalecombobox_8cpp_source.html)).
- **Ölçek kutusuna yazma:** ArcGIS Pro'da ölçek virgüllü ya da virgülsüz yazılır; gösterim biçimi Scale Properties'in
  Scale Format sekmesindedir
  ([Map scales and scale properties](https://pro.arcgis.com/en/pro-app/latest/help/mapping/navigation/map-scales-and-scale-properties.htm)).
- **Boyu olmayan kutu:** QGIS'in harita tuvali boş (boyu olmayan) kutuda ölçeğini korur, yalnız merkezi taşır
  ([QgsMapCanvas::setExtent](https://api.qgis.org/api/3.20/qgsmapcanvas_8cpp_source.html)).
- **Genel bakışın kapsamı:** QGIS'in genel bakışı, ona giren katmanların kapsamlarının birleşimini gösterir
  ([#1788](https://issues.qgis.org/issues/1788)).
- **Düzenlemeden sonra kapsam:** QGIS'te kapsamın düzenlemeden sonra güncellenmemesi bilinen bir sorundur
  ([#14406](https://issues.qgis.org/issues/14406)). Her düzenlemede yeniden hesaplamak pahalı bulunmuş, GeoPackage'ta
  silinen satır kapsamı küçültmez.
- **KentOS kararı:**
  - Ölçeğin tek bir yazımı olur; büyütme `N:1` yazılır, `1:0` çıkmaz.
  - Boyu olmayan kutu ölçeği korur.
  - Veri katmanının kutusu kaynağın söylediğidir. Kaynak kutuyu kendisi tutar: GeoPackage'ın `gpkg_contents` kutusu,
    yazmalarla güncellenir; PostgreSQL'in tahmini, yoksa ölçülen. Kutu, çizim kaynağa yazdıkça yeniden okunur.

## Karar

### 1. Ölçeğin tek yazımı

- `screen_scale::scale_words` ölçeği yazar:
  - N 1 ve üstündeyse `1:N`, örneğin `1:3.428`;
  - 1'in altındaysa büyütme `N:1`, N = 1/ölçek yuvarlanmış, örneğin `2:1` ya da `2.646:1`;
  - okunamayan ölçek `—`.
- Sayı her zaman en az 1'dir; `1:0` ya da `0:1` çıkmaz. 1:1 sınırında iki yön buluşur: 0,99 ve 1,4 `1:1`, 0,5 `2:1`.
- **Kullananlar:** durum çubuğunun ölçek hücresi ve genişlik hesabı, bölünmüş görünümlerin rozetleri, Ölçek yaz…
  alanı.
- **Rozetin hesabı:** rozet de görünümün kendi kamerasıyla, metre üzerinden hesaplanır (`App::scale_of`). Coğrafi
  projede rozet ile durum çubuğu aynı ölçeği söyler.
- **Ölçek yaz…:**
  - Alan ölçeği hücrede okunduğu gibi açar: `1:1.000` ya da `2.646:1`. Önceden önünde sabit bir `1:` vardı, büyütmede
    alan `0` ile açılıyordu.
  - `1:N`, `N` ve büyütme `N:1` yazılır. Ölçek olmayan yazı nedeni ve örneğiyle reddedilir.

### 2. Ölçek çubuğu

- Santimetrenin altında milimetre, milimetrenin altında mikrometre yazılır: `5.0 mm`, `200 µm`.
- En derin yakınlaştırmada çubuk `10 µm` der.

### 3. Boyu olmayan kutu

- Kamera iki yönde de boyu olmayan kutuyu sığdırırken ölçeğini korur ve yeri ortaya alır.
- Tek yönde boyu olan kutu (yatay bir çizgi) uzunluğuna sığdırılır.
- Kural kameradadır: Tümünü göster, Seçime yakınlaştır ve Katmana yakınlaştır bu kurala uyar.

### 4. Veri katmanının kapsamı kaynaktan

- **Çekirdek:** `data_extent::data_extent` kaynağın kutusunu okur, tablonun ve projenin sisteminde. Tablosu satırsız ya
  da geometrisiz katmanın kutusu yoktur; okunamayan kaynak nedenini söyler.
- **Masaüstü** (`data_extents.rs`):
  - Her katmanın kutusu arayüz iş parçacığının dışında okunur. Çizimin oturumu ve kaynaklara yazmaları değişmedikçe bir
    kez okunur; bir kayıt yeniden okutur.
  - Okunurken katmanın tuttuğu kutu yerine geçer.
- **Tümünü göster:**
  - Elindekiyle hemen sığdırır. Okunan kutular gelince görünüm hâlâ ilk sığdırmanın bıraktığı yerdeyse yeniden
    sığdırır; araya giren kaydırma ya da yakınlaştırma korunur.
  - Hiç kutu yokken ““yeni” katmanının kapsamı kaynağından okunuyor; okununca görünüm sığdırılır.” der. Kutu gelince
    “Görünüm, katmanların kaynaktan okunan kapsamına sığdırıldı.” der.
  - Kutusu okunamayan katman nedeniyle söylenir.
  - Gösterecek bir şey yoksa: “Görünen katmanlarda gösterilecek nesne yok; gizli ve dondurulmuş katmanlar sayılmaz.”
- **Katmana yakınlaştır** aynı kutuları kullanır.
- **Değişmeyen:** katmanın kendi kutusu (`DataLayer.extent`) değişmez; kaynağa yazmak proje dosyasını kirletmez. Proje
  açılırken görünüm katmanın tuttuğu kutuyla kurulur (ADR 9203).

### 5. Genel bakış

- **Kapsam:** çizimin nesneleri ile görünen veri katmanlarının kaynaktan okunan kutularının birleşimi.
- **Veri katmanının satırları:** `data_extent::data_outline` okur:
  - yalnız geometri, alan okunmaz;
  - çizgi ve alan kartın bir pikseline sadeleştirilir;
  - en çok 100 000 satır okunur; daha çoksa ilk satırlar ve tablonun kutusu alan olarak çizilir.
- **Çizim:** satırlar çizimin nesnelerinin altında, katmanın renginde çizilir.
  - Çekirdeğin `overview_picture_of`'u birden çok depoyu verilen kapsamda çizer.
  - Veri satırının noktası 3 piksellik bir noktadır, haritadaki simgesi gibi. Çizimin kuralı (1 piksel, ortak durumlar)
    değişmez.
- **Resim yokken nedeni:**
  - “Veri katmanları kaynaklarından okunuyor…”;
  - “Veri katmanının kaynağı okunamadı.”;
  - “Görünen katmanlarda nesne yok; gizli katmanlar gösterilmez.”;
  - “Çizimde gösterilecek nesne yok.”
- **Yenilenme:** bir okuma geldikçe resim yeniden çizilir. Basış görünümü oraya taşır; sürükleme, tekerlek ve çift tık
  önceki gibidir.

### 6. Büyüteç ve Koordinat listesi

- Büyüteç veri katmanının satırlarını ana haritadaki gibi çizer.
- Koordinat listesi tabloda seçilen kaynak satırını ADR 9326'dan beri listeler.

### 7. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Ölçeğin tek yazımı | var: bölünmüş görünümün rozeti | var: aynı | var: aynı | var: rozet, durum çubuğunun ölçek hücresi, Ölçek yaz… |
| Ölçek çubuğunun küçük birimleri | yok: CAD projesinde ölçek çubuğu yoktur, eksen simgesi vardır | yok: aynı gerekçe | yok: aynı gerekçe | var |
| Boyu olmayan kutuda ölçeğin korunması | var | var | var | var |
| Veri katmanının kaynaktan kapsamı (Tümünü göster, Katmana yakınlaştır, Genel bakış) | yok: veri katmanı CBS'nindir | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 8. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Görünüm, gezinme ve kartlar masaüstünün sunumudur. Veri katmanının satırları `cad.layers.rows` ile okunur. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Süzgeç:** süzgeçli veri katmanının kutusu tablonun tamamınındır; süzgeç kutuya uygulanmaz.
- **Dış yazma:** başka bir programın yazması çizimin kaynaklara yazmalarına sayılmaz. Kutu, yeni oturumda ya da bu
  çizimin bir kaydından sonra yeniden okunur.
- **PostgreSQL:** tahmini kapsam (`ANALYZE`) eski olabilir; tahmin yoksa ölçülür. Genel bakış ve Tümünü göster bu dilimde
  gerçek PostGIS ile sınanmadı.
- **Büyük tablo:** 100 000 satırdan büyük tabloda Genel bakış ilk satırları ve tablonun kutusunu çizer.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Çekirdek:**
  - `cargo test -p kentos-geometry-core --test all -- navigation`: 5 geçti. Yeni sınama: tek depo kendi kapsamında
    kendi resmiyle bit bit aynı; veri deposunun noktası üç piksel, çizimin çizgisi onun üstünde. Ortak durumların
    resimleri değişmedi.
  - `cargo test -p kentos-render-wgpu --lib -- camera`: 7 geçti. Yeni sınama: boyu olmayan kutu ölçeği korur, yeri
    ortaya alır; yatay çizgi uzunluğuna sığar.
  - `cargo test -p kentos-native-application --test all -- data_extent`: 3 geçti.
    - Eklendikten sonra yazılan satır katmanın tuttuğu kutunun dışında, kaynağın kutusunda.
    - Satır sınırını aşan özet ilk satırları ve kutuyu tutuyor.
    - Boş tablonun kutusu yok; olmayan tablo adıyla söyleniyor.
- **Masaüstü:**
  - `cargo test -p kentos-desktop --lib -- screen_scale map_marks split_view navigation selecting layering gpkg_tests
    grids status data_extents …`: hepsi geçti; tek kırmızı işten önce de olan bağımlılık tabanı (`shell/approved_layer.rs`,
    `shell/gpu_frame.rs`, `gis/data_selection.rs`, ADR 9214'ün dosyaları).
  - `screen_scale`: 1:1 sınırının iki yönü; okunan ölçek yazılıp geri okununca aynı.
  - `map_marks`: `5.0 mm`, `1.0 mm`, `200 µm`, en derinde `10 µm`.
  - `data_extents_tests`:
    - boş yapılan katmana oturumla yazılan nokta Tümünü göster'le ortaya geliyor, ölçek değişmiyor;
    - ikinci noktayla ikisi birden görünüyor; Katmana yakınlaştır aynı kutuyu çerçeveliyor;
    - Genel bakış önce “okunuyor” diyor, sonra iki satırı katmanın renginde çiziyor; basış ana görünümü taşıyor;
    - katman gizlenince nedenini söylüyor.
  - `cargo test -p kentos-desktop-cbs`: 58 geçti.
  - `scripts/dev/module_deps.py`: yeni bağ yok, sayılar işten önceki gibi.
- **Resim** (`.run/gui-evidence/kullanim-9327/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-en-yakin-olcek`: üç bağlı görünüm; rozetler ve durum çubuğu `2.646:1`, ölçek çubukları `10 µm`.
  - `02-genel-bakis-ve-buyutec`: yalnız veri katmanlı proje; Genel bakış altı satırı çiziyor, Büyüteç satırı artının
    yanında gösteriyor.
  - `03-gizli-katman`: Genel bakış “Görünen katmanlarda nesne yok; gizli katmanlar gösterilmez.” diyor.
- **Resim yardımcısı:** GPU yazı katmanı ilk kez istenen harfi bir sonraki karede çizer; tek karelik resimde etkin
  görünümün çubuğu “10 µ” okunuyordu. `usage_shots` kaydetmeden önce bir kare çizer. `usage_shots_in` resmi alınacak
  temayı önce kurar; Genel bakış o temanın renkleriyle çizilir.
