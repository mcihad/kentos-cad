# 1097 — Coğrafi projede Mesafe ölç, Alan hesapla ve ölçek çubuğu elipsoit üstünden metre verir

Tarih: 2026-11-20. gis-usage-report.md GIS-040. Durum: kabul edildi.

İlgili: ADR 0171 (düzlem, elipsoit, zemin; `crs::ground`), ADR 0167 (ikinci sistem), CLAUDE.md §5 (birim), §23.3
(alan/uzunluk açık yöntemle).

## Sorun

EPSG:4326 projede çizimin koordinatları boylam/enlem derecesidir. Mesafe ölç aynı enlemde 0,103° aralıklı iki noktaya
“0,103 m” yazdı (elipsoitte ≈9,29 km), Alan hesapla 0,00 m² ve 0,206 m çevre verdi, ölçek çubuğu 0,2° genişliğinde
bir görünümde 5,0 cm gösterdi: derece metre sayılıyordu.

## Dayanak

- PROJ `geod` ve GeographicLib (Karney 2013): elipsoit üstünde jeodezik uzunluk ve poligon alanı. KentOS'ta
  `crs::ground::ground_measures` zaten bunu yapıyor (ADR 0171), coğrafi sistemde düzlem değerleri yok.
- QGIS ölçme araçları coğrafi sistemde ölçümü elipsoit üstünden yapar ve metre/m² yazar
  ([belgesi](https://docs.qgis.org/3.44/en/docs/user_manual/introduction/general_tools.html)).

## Karar

1. **Mesafe ölç ve Alan hesapla.** Projenin kendi sistemi coğrafiyse (`ground::geographic_system`) toplam uzunluk,
   alan ve çevre, Sabit ilk nokta'nın ışınları, İçine tıkla'nın bölgesi ve imlecin yanındaki canlı etiket elipsoit
   üstünden hesaplanır ve “Elipsoit üstünde: …” diye metre ve projenin alan birimiyle yazılır. Ardından gelen zemin
   satırı (yükseklik varsa) aynı kalır; elipsoit satırı yinelenmez. Sistemin ulaşmadığı nokta için var olan “değer
   yazılmadı” iletisi. Projeksiyonlu projelerde hiçbir şey değişmez.
2. **Ölçek çubuğu.** Bir çizim biriminin metre karşılığı görünümün orta enleminde, çubuğun erişimi kadar (120 px)
   doğu-batı jeodeziğinin uzunluğu bölü kapladığı derece olarak alınır (`map_marks::metres_per_unit`); çubuk piksel
   başına metreyle çizilir. 36° K'de bir boylam derecesi ≈90,2 km, 60° K'de ≈55,8 km.
3. **Görünüm ölçeği.** Durum çubuğunun 1:N'i ve ölçek listesinden seçim aynı çarpanla metreye göre (`view_scale`).

## Sınır

- Kameranın en derin zumu birim başına 10⁴ pikseldir; coğrafi projede bu ≈1:42 000'dir. Daha yakın zum ayrı iştir.
- Ölçeğe bağlı görünürlük (katman/etiket ölçek aralıkları) kameranın ham ölçeğini kullanmaya devam eder.
- Tampon, en yakın ve mesafe ile seçim gibi işlemler GIS-064–066'nın konusudur.

## Kanıt

- `crates/native/interaction/tests/all/measure.rs` `a_geographic_project_measures_on_the_ellipsoid`: 0,103° boylam
  36° K'de 9287.328 m, 60° K'de 5747.400 m; 2°'lik kare 39 510 856 108.41 m², çevre 799 896.451 m — pyproj Geod
  (WGS 84) bağımsız referansıyla gösterilen basamağa kadar aynı. Projeksiyonlu ölçü testleri (34) değişmeden geçer.
- `apps/desktop/src/map/map_marks.rs`: metre/derece 36° K'de 90163.682, 60° K'de 55799.994 (pyproj, 1 mm içinde);
  1 000 px/derecede çubuk 10 km; projeksiyonlu projede çarpan 1; 1:100 000 seçimi geri okunur; resim
  `.run/gui-evidence/cbs-kullanim/gis-040-olcek-cubugu.png`.
