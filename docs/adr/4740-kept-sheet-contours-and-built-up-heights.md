# 4740 — Pafta eğrileri kayıtlı çizim; yeniden üretim yalnız değişeni yazar; m. 63(2) kotları ve m. 93(1)(c) karşılaştırması

Tarih: 2026-11-10. plan/map-cad.md MAP-14, dokuzuncu tur. Durum: taslak. MAP-14'ün sıradaki işi: pafta eğrilerinin
kayıtlı çizim olması (yüzey sürümüne bağ); kendiliğinden yeniden üretimin yalnız değişeni yazması; fark eşiği ve m. 63(2)
yerleşimi (kaynak bekleniyordu).

İlgili:
- [ADR 0911](0911-road-project-tree.md): kayıtlı çizimler, kaynak damgaları, kendiliğinden yeniden üretim;
- [ADR 1820](1820-sheet-contours-by-article-78.md), [2320](2320-sheet-summit-and-pit-heights.md),
  [3120](3120-sheet-slope-top-and-bottom-heights.md), [3520](3520-sheet-chosen-point-heights.md): pafta eğrileri ve
  kotları (3520'nin “Sonra”sı: m. 63(2) yerleşimi ve farkın eşiği);
- [ADR 4320](4320-regional-redraw-of-surface-drawings.md): yalnız değişenin yazılması (bu ADR kuralını genişletir).

## Dayanak

- **BÖHHBÜY** (RG 26.06.2018/30460 Mük.; CK 10455, RG 04.10.2025 bu maddeleri değiştirmedi; metin 10 Kasım 2026'da
  mevzuat.gov.tr PDF'inden okundu; alt ajanın notu `.run/research/map-cad/mevzuat/MAP-14.9.md`):
  - **m. 63(2):** “Yerleşik alanlar ve yollarda harita üzerinde yaklaşık 2 cm'de bir, çatı ve teraslarda ise uygun
    köşelere yükseklik değerleri verilir.” “Yaklaşık”ın hoşgörüsü, yerleşik alanın sınırı ve “uygun köşe” yazılı
    değildir. Madde fotogrametri bölümündedir; yersel alımın karşılığı m. 45(1)(c)'dir (eğri için gerekli noktalar, en
    az 25 nokta/ha).
  - **m. 91(2):** KOH_X,Y (cm) ≤ ±(−1,665·10⁻⁶·S² + 0,01745·S − 1,166), KOH_Z (cm) ≤ ±1,33·KOH_X,Y; S ölçeğin paydası.
  - **m. 93(1):** kesit kontrolü yüksekliği bilinen iki kontrol noktası arasında yapılır; (b) kesit üzerinde en çok 20 m
    aralıklı noktalara geometrik ya da trigonometrik nivelmanla yükseklik taşınır; (c) ölçülen ve eş yükseklik
    eğrilerinden hesaplanan yüksekliklerin farklarının KOH'u, m. 91'e göre bulunan KOH_Z'nin 1,5 katından küçük olur.
  - Eğrinin hangi yüzey sürümünden çizildiğinin kaydı, yeniden üretim ve yeniden üretimde “değişti” sayılacak en küçük
    fark için hüküm yoktur (taranan: m. 45, 62–72, 78, 81–95, 98, Ek-1–3). m. 98 haritaların ve hesapların arşivini
    ister; kayıt onunla çelişmez.
- **Benzer uygulamalar** (alt ajanın notu `.run/research/map-cad/uygulama/MAP-14.9.md` ve ana oturumun eki, ekran
  resimleri `.run/research/map-cad/uygulama/MAP-14.9-resim/`, 10 Kasım 2026'da okundu):
  - Civil 3D: eğri yüzeyin biçemidir, yüzeyle canlıdır; Extract Objects ile çıkarılan eğri bağsızdır; veri kısayolunun
    başvurusu eskiyince ağaçta işaret ve Synchronize. **Spot Levels On Grid** (2024) ve **Create Points – Surface › On
    Grid** (2023): ızgaranın taban noktası sol alt köşe, X ve Y aralığı, isteğe bağlı dönme; kot yüzeyden.
  - OpenRoads: arazi modeli kaynağına “kurallı” bağlanır (Link Graphic Features), kural kilitlenir ya da kaldırılır.
  - ArcGIS Pro: Contour raster işlevi canlı, Contour aracının yazdığı eğri bağsızdır.
  - Netcad NetSURF: çevrim içi belge okunamadı (help.netcad.com SSL hatası); değer uydurulmadı.

## Karar

1. **Kayıtlı çizim.** `road.terrains.sheetcontours`'ın çizdiği, projenin çizimi olarak tutulur (ADR 0911 §1): ana
   kaynağı yüzeydir; okuduğu her katman da kaynaktır, yeni kaynak türü **`layer`**: detay, şev üstü ve dibi, seçilen
   noktalar, yerleşik alan ve yol katmanları, bu sırayla, her biri bir kez. Katmanın damgası nesnelerinin geometrisidir
   (nesnenin JSON'u, tabanındakiler — numarası, katmanı, görünüşü, öznitelikleri, etiketi — çıkarılarak), çizimin
   sırasıyla: rengi değişen bina eğriyi kaydırmaz ve eskitmez, kaydırılan bina eskitir. Katmandaki her nesne sayılır;
   okunmayan bir nesnenin değişimi de eskitir (güvenli yön). Ağaçta adı “Pafta eğrileri”, yüzeyin altında; katmanın
   sözcükleri “katman”, “katmanı” (“Bina” katmanı çizildiğinden beri değişti). `road.project.rebuild` ve kendiliğinden
   yeniden üretim onu da çizer.
2. **Yalnız değişeni yaz kuralı genişledi** (ADR 4320'nin kuralının yerine). Komutun sırasıyla her yeni nesne, alınmamış
   ilk tıpatıp aynı eski nesnenin yerinde kimliğiyle kalır; sırada önündeki bir nesne yazılıyorsa (yazılanlar çizimin
   sonuna gider, ondan sonra çizilir) ya da daha sonraki bir yerden kalıyorsa ve ikisi öbür sırada başka bir resim
   çizebiliyorsa yazılır, eski eşi sonraki bir eşe açık kalır. Her sırada aynı resmi yalnız yazısız, aynı renk ve
   saydamlıktaki çizgi ve çoklu çizgiler çizer (aynı renkli iki çizginin kapladığı pikseller hangisi önce çizilirse
   çizilsin aynıdır); maskeli yazı, nokta, dolgu sırasını korur. ADR 4320'nin çizgi çizimi bunun özel durumudur.
   - Pafta eğrilerinde aynı çıkan eğriler kalır; bir eğri yazılınca ondan sonraki kotlar da yazılır, maskeleri eğrilerin
     üstünde kalsın diye.
   - `region` her değişen çizimde verilir: silinen ve yazılan nesnelerin kutusu, yazı ekleme noktasıyla (yazının kutusu
     yazı tipine bağlıdır).
   - Elle boyanan eğri artık yalnız kendisi, çizimin rengiyle yeniden yazılır (ADR 4320'de bütün çizim yazılırdı; ortak
     durum yeni kurala göre güncellendi).
3. **Proje ayarı “Yalnız değişeni yaz”** (`road.rebuildWhole`; `road.project.edit`'in `regional`'ı). Yeniden üretim,
   Proje ağacı'ndan elle de kendiliğinden de (ADR 0911 §3) yalnız değişeni yazar; projenin ilk seçimi budur, eski
   projeler dahil. İşaret kaldırılınca her çizim bütün çizilir. `road.project.rebuild`'in kendi `regional` girdisi
   değişmedi (Python ve MCP açıkça verir). Ağacın cevabında `regional`; geri alma adımı “Yalnız değişeni yaz”, ikisi
   birden değişince “Yeniden üretim”.
4. **m. 63(2) kotları** (`builtUp`: `areas`, `roads`, `every`). Paketin `heightEvery`'si 20 mm (m. 63(2)'nin yaklaşık
   2 cm'si; “yaklaşık”a hoşgörü uydurulmadı, aralığı kullanıcı değiştirir).
   - Yol katmanındaki her çizgi ve çoklu çizginin (her parçasının; kapalı olan başına döner) başından yarım aralıkta
     başlayıp her aralıkta bir kot, yerinde bir kez.
   - Sonra aralığın ızgarası (doğuda ve kuzeyde aralığın katları; Civil 3D'nin ızgara kotu gibi, taban noktası koordinat
     başlangıcı), güneyden kuzeye, batıdan doğuya: yerleşik alanın içinde (çift-tek), detayın dışında ve konan bütün
     kotlara aralığın yarısından uzak noktalar.
   - Kotlar yüzeyden, dm'ye kadar, Ek-3 0004'ün yazısıyla (nokta ve yazı); yüzeyin dışındaki yer sayılır. Izgara
     1 000 000 noktayı aşarsa ret.
   - Çatı ve terasların “uygun köşeleri” ölçmecinin seçtiği noktalardır (ADR 3520); not bunu, m. 63'ün fotogrametri
     maddesi olduğunu ve yersel alımda m. 45(1)(c)'yi söyler.
5. **Farkın eşiği: m. 93(1)(c) karşılaştırması** (`check`). Kendi kotu olan, yüzeyde ve yüzeyin köşesi olmayan seçilen
   noktalarda (köşenin farkı sıfırdır; karşılaştırılmaz, sayılır) ölçülen − yüzey farklarının KOH'u, en büyüğü ve yeri.
   Sınır paketin `sectionCheck`'inden: KOH_Z = 1,33 · (−1,665·10⁻⁶·S² + 0,01745·S − 1,166) cm, sınır 1,5 · KOH_Z
   (1/1000: 0,292 m; 1/5000: 0,887 m). KOH sınırı aşarsa uyarı `section_check_exceeded`. Not: “Bu bir karşılaştırmadır;
   m. 93'ün kesit kontrolü yüksekliği bilinen iki kontrol noktası arasında, en çok 20 m aralıklı ve nivelmanla ölçülmüş
   noktalarla yapılır.” Uygunluk söylenmez.
6. **Paket** `bohhbuy-2018-m78` sürüm 4: `heightEvery` 20, `sectionCheck` {xy: [−1,665·10⁻⁶; 0,01745; −1,166], zFactor
   1,33, factor 1,5}; kaynak metnine m. 63(2), 91(2), 93(1)(c). Bozuk değerli paket reddedilir.
7. **Masaüstü.**
   - Pafta eğrileri penceresine **Yerleşik alan (63/2)** satırı: “2 cm'de bir kot”, Alan, Yol. Katmanlar adlarından
     tahmin edilir (yerleşik, yerleşim, iskân; yol ekseni, eksen, aks) ve detaylardan çıkarılır.
   - Özette m. 93(1)(c) satırı (sınırı aşınca uyarı rengiyle) ve m. 63(2)'nin sayıları.
   - Çiz'in sözü: “çizim yüzeyine ve okuduğu katmanlara bağlı tutulur”.
   - Proje ağacı'nın “Yalnız değişeni yaz”ı projenin ayarıdır.
8. **Kendiliğinden yeniden üretimin sözleri her yazmanındır:** `drawings_rebuilt`, `drawing_edited`, `drawing_not_rebuilt`
   protokolün kodları arasına alındı (kısıtların çözümünün kodları gibi): kendiliğinden yeniden üretim açıkken her yazma
   onları verebilir (pafta durumlarındaki kot düzenlemesi verir).
9. **`.kcad` şema 238:** pafta eğrisi çizimi, `layer` kaynağı ve `rebuildWhole`; şema 237 üçünü de değer hatasıyla
   reddeder. Bilinmeyen sürüm örneği 239'a taşındı. Bağımsız okuyucu (`tools/kcad/kcad.py`) aynı kuralı uygular.

## Sınırlar

- Hesap bütündür, yalnız yazma bölgeseldir (ADR 4320). Karolu hesap gerçek veride ölçülmedi.
- Katman damgası katmanın bütün nesnelerini okur. Büyük çizimde katman dizininden okuma ayrı iştir.
- m. 63(2)'nin yerleşimi bir öneridir: ızgara koordinat başlangıcına bağlıdır; sokak ortası, kavşak gibi yerleri
  ölçmeci seçer. Yerleşik alanda eğrinin gösterilip gösterilmeyeceği (m. 63(1)) detay ve katman seçimiyle kullanıcınındır.
- m. 93(1)(c) karşılaştırması kesit kontrolü değildir. Yüzeyin kotu eğrilerden enterpolasyonla değil, eğrilerin
  çizildiği üçgen yüzeyden okunur.
- `region`'a yazının kutusu girmez (ekleme noktası girer).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/road_sheet_contour_cases.py`; Triangle, KentOS kodu yok): 49 durumdan 71'e.
  - m. 63(2): 1/1000'de yolda 20, yerleşik alanda 31 kot (binanın içi atlanır, yolun yüzey dışındaki ucu sayılır);
    elle 40 mm; 1/5000 planı; yalnız yol; tepe kotuna yarım aralıktan yakın ızgara noktası yazılmaz.
  - m. 93(1)(c): KOH 0,080 m < 0,292 m; aşan 0,443 m > 0,292 m (uyarı); 1/5000'de aynı noktalar 0,887 m'nin altında;
    sınır vermeyen pakette karşılaştırma yok. Var olan “Seçilen noktalar” durumu da karşılaştırmayı söyler (0,145 m).
  - Kayıtlı çizim: zeminin bir kotu değişince 135 nesne yerinde kalır, 20'si silinip yazılır; aynı değişiklik bütün;
    bina kaydırılınca katmanın damgasıyla eskir; okumadığı katman değişince eskimez; kendiliğinden yeniden üretim yalnız
    değişeni, proje bütün derse bütün yazar.
  - 7 yeni ret (katmansız, aralık 0, aralık vermeyen paket, yazı yüksekliği vermeyen paket, olmayan iki katman, bozuk
    sınır).
  - Rust ve Python koşucuları geçer; yetenek testi geçer. Bozma denemesi: aşan durumun KOH'u 10⁻⁶ ve kayıtlı çizimin
    `kept`'i 1 oynatılınca koşucu tam o iki durumu (71'den 2) yakalar.
- **`road_regional_rebuild_cases.py`:** elle boyanan eğri yeni kurala göre (1 silinen, 15 kalan, bölgesiyle); öbür 22
  durum aynen.
- **`road_project_cases.py`:** ağacın `regional`'ı; `road.project.edit` için 6 yeni durum (kapanır ve geri alınır,
  yeniden açılır, zaten öyle, ikisi birden tek adım, kendiliğinden açıkken, plan).
- **Rust birim testleri** (`road_project_regional`): sıra kuralının yürüyüşü (eğri yazılınca yazı yazılır, ters sıradaki
  iki yazı, aynı görünüşteki iki çizgi, yazılanın eşi sonrakine kalır), görünüş.
- **`.kcad`:** `map-sheet-contours` ve şema 237'de reddi; bağımsız okuyucu aynı sonucu verir.
- **Python** (`python/tests/test_sheet_contour_drawing.py`, testin kendi okumalarıyla: yol kotları eksenin üstünde ve
  başından 10 + 20·k m'de, ızgara kotları 20 m'nin katlarında, alanın içinde ve binanın dışında; KOH ve sınır testin kendi
  hesabı; kot değişince eskir, yalnız değişeni yazınca kalan nesne eski haliyle aynı, sonra güncel); pafta, seçilen nokta
  ve bölgesel yeniden üretim testleri ve Python koşucusu geçer.
- **MCP** (`crates/native/mcp/tests/sheet_contour_drawing.rs`): m. 63(2) kotları, m. 93(1)(c) uyarısı, `regional`'ın
  seçimi; bölgesel yeniden üretimin, yardımcı eğrinin ve çizim kaydının testleri geçer.
- **Masaüstü** (`sheet_contours_window_tests`: yerleşik alan ve karşılaştırma durumları, çizimin kaynakları;
  `road_project_regional_tests`: kutu projenin ayarı; Proje ağacı testleri; çapraz denetimler 135 test). Resimler
  `.run/gui-evidence/yerlesik-alan-4740/` (pencere, sınırı aşan karşılaştırma, yerleşik alanın kotları, eskiyen çizim).
- Çok katmanlı çizimde Kesmeyeceği (g) satırı pencereden taşıyordu; satır artık alt satıra kayar.

## Sonra

- Katman damgasının katman dizininden okunması; karolu hesap (gerçek veri).
- m. 63(2)'nin ızgarasının bir taban noktası ve dönmeyle verilmesi (Civil 3D gibi); yerleşik alandaki eğrilerin seçimi.
- Kesit kontrolünün kendisi (iki kontrol noktası arasında kesit, 20 m aralık, nivelman ölçüsü) ayrı bir denetim.
- Yalnız değişen atlas sayfalarının PDF'i (MAP-18) bu bölgeyle.
- Web (TODOS §0.2).
