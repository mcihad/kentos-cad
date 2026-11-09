# 3520 — Pafta eğrilerinde ölçmecinin seçtiği noktaların kotları (m. 78(1)(ğ))

Tarih: 2026-11-09. plan/map-cad.md MAP-14, altıncı tur. Durum: taslak. ADR 3120'nin “Sonra”sındaki iki madde:
ölçmecinin seçtiği öbür noktalar (“gerekli görülen diğer noktalar”, bir nokta katmanı) ve noktanın kendi ölçülmüş kotunun
yüzeyinkinden ayrıldığı yerin bildirilmesi.

İlgili:
- [ADR 1820](1820-sheet-contours-by-article-78.md): BÖHHBÜY m. 78'e göre pafta eğrileri (`road.terrains.sheetcontours`);
- [ADR 2320](2320-sheet-summit-and-pit-heights.md): tepe ve çukur kotları (`spots`, kot noktası yazısı);
- [ADR 3120](3120-sheet-slope-top-and-bottom-heights.md): şev üstü ve dibi kotları (`slopes`).

## Dayanak

- **BÖHHBÜY** (RG 26.06.2018/30460 Mükerrer; ADR 1820'nin kopyası, Çanakkale Onsekiz Mart Üniversitesi Harita
  Mühendisliği; PDF'in metni 9 Kasım 2026'da yeniden okundu, s. 6986):
  - **m. 78(1)(ğ):** tepe, çukur, şev, dip ve üstlerindeki karakteristik noktalar ile *gerekli görülen diğer noktalar*
    paftada işaretlenerek yükseklik değerleri dm'ye kadar yazılır;
  - yönetmelik “diğer noktaların” hangileri olduğunu söylemez. Gereğini gören ölçmecidir;
  - **m. 63(1):** eğrilerle gösterilemeyen düz arazide ve yerleşim içindeki boş alanlarda yükseklikler kot noktalarıyla
    gösterilir. **m. 63(2):** yerleşik alan ve yollarda haritada yaklaşık 2 cm'de bir, çatı ve teraslarda uygun
    köşelere yükseklik verilir (stereo kıymetlendirme bölümü);
  - **Ek-3 0004:** kot noktasının yazısı 1,2 mm (paketin `spotHeight`'ı).
- **Civil 3D 2023 Kullanıcı Kılavuzu** (9 Kasım 2026'da okundu):
  - [To Create a Point at a Surface Elevation](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-FDBDB4BD-86C1-4A01-9EC3-FEAEF7DF0A97.htm):
    nokta kotunu yüzeyden, verilen yerde alır; yer yüzeyin sınırı içinde verilir;
  - [To Change the Elevations of Points](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-F3A6538B-1C25-4A4B-8C12-1E537CF0BB0D.htm):
    Elevations From Surface ayrı ve açık bir komuttur; seçilen noktaların kotu yüzeyin o yerdeki kotu olur;
  - [About Surface Spot Elevation Labels](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-UserGuide/files/GUID-3CC7CAC3-E7F6-4286-B864-09EE6EA444F8.htm):
    yüzey kot etiketi yüzeyin içindeki herhangi bir yeri kotuyla etiketler.

  Yani noktanın kendi kotu ile yüzeyin kotu ayrı şeylerdir; birini öbürüyle değiştirmek kullanıcının açık işidir.
- **KentOS bugün** (ADR 3120):
  - tepe, çukur ve şevin köşeleri yazılıyordu;
  - ölçmecinin öbür noktaları için yol yoktu;
  - ölçülmüş kotu olan nokta da yüzeyden yazılırdı. Ölçü ile yüzey arasındaki fark görünmezdi.

## Karar

1. **`points` girdisi** (`road.terrains.sheetcontours`, ek alan; komut sürümü değişmez): nokta katmanları. Boşsa
   istenmemiş sayılır.
2. **Nokta:** bu katmanlardaki nokta nesneleri, çok parçalı noktanın her parçası. Çizimin sırasıyla; aynı yer (aynı
   koordinatlar) bir kez. Kotu:
   - noktanın kendi kotu varsa o (ölçmecinin ölçtüğü). Nokta yüzeyin içindeyse yüzeyin kotu cevabın `surface`'ındadır;
   - yoksa yüzeyden: noktanın üçgeninin düzlemi;
   - kendi kotu olmayan ve yüzeyin dışında (ya da boşluğunda) kalan nokta yazılmaz, sayılır. Kendi kotu olan nokta
     yüzeyin dışında da yazılır;
   - detay kuralı uygulanmaz: noktayı ölçmeci seçmiştir (ADR 3120).

   KentOS ölçülen kotu yüzeyinkiyle değiştirmez, yüzeyi de noktaya uydurmaz. Fark yalnız bildirilir (5).
3. **Yazı:** tepe, çukur ve şevinki gibi: kot dm'ye kadar, yarım yukarı (858,62 → “858.6”). Noktanın sağında, maskeli.
   Yazı yüksekliği paketin `spotHeight`'ı × ölçek.
   - **Nokta yazılmaz:** ölçmecinin noktası zaten çizimdedir; (ğ)'nin “işaretlenerek”i odur. Tepe, çukur ve şevde nokta
     yazılmaya devam eder (orada nokta yüzeyin köşesidir, çizimde yoktur);
   - `spotHeight` vermeyen pakette ret (`invalid_contours`, `rules`, “seçilen noktaların kotları yazılmaz”).
4. **Cevap:** `spots`'ta tür `chosen`, tepe/çukur ve şevlerden sonra. `SheetSpot`'a ek alan `surface` (yalnız kendi kotu
   olan ve yüzeyin içinde kalan noktada).
5. **Not:**
   - kaç noktanın kotu yazıldığı, yazı yüksekliği ve kaynağın kuralı;
   - kendi kotuyla yazılan nokta sayısı; yalnız kotun yazıldığı, noktanın ölçmecinin olduğu;
   - kendi kotunun yüzeyinkinden en çok ne kadar ayrıldığı ve nerede (Y, X; eşitlerden ilki);
   - yazılmayan nokta sayısı.
6. **Retler:** olmayan katman `layer_not_found` (`points`, ““‹ad›” katmanı çizimde yok.”).
7. **Masaüstü** (Pafta eğrileri):
   - **Seçilen noktalar (ğ)** satırı: **Nokta kotları** kutusu (ilk değer kapalı), **Katman** seçimi (Yok seçilebilir) ve
     “Kendi kotu varsa o, yoksa yüzeyden” açıklaması;
   - katman adından tahmin edilir: “seçilen”, “karakteristik” ya da “kot noktası”. Tahmin edilen katman detay
     listesinden çıkar; kutu işaretliyken Kesmeyeceği satırında görünmez.
8. **Kapsam işareti:** ADR 1820'deki gibi **Harita + Yol** (Arazi › Yüzey).

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/road_sheet_contour_cases.py`, uv ile `triangle`; KentOS kodu yok):
  - çizime “Seçilen noktalar” katmanı ve beş nokta nesnesi eklendi (önceki nesnelerin kimlikleri aynı kalsın diye en
    sona): şevde kendi kotu olan bir nokta, kotsuz bir nokta, iki parçalı bir nokta (ikinci parçası alınmış bir yer),
    yüzeyin dışında kendi kotu olan ve olmayan birer nokta;
  - başvuru kotları Triangle'ın üçgeninin düzleminden kendisi hesaplar. Farkı ve yerini, yazılmayan sayısını kendisi
    sayar;
  - yeni durumlar: seçilen noktalar (dört nokta, ikisi kendi kotuyla, en büyük fark 0,145 m, bir nokta yazılmaz), plan
    yazmaz, şevlerle birlikte (şevler önce), `spotHeight`'sız paket, olmayan katman. 1/5000'de;
  - 49 durum (44'ten). Eski durumların cevapları aynı. Rust ve Python geçer.
- **Python** (`python/tests/test_sheet_chosen_points.py`):
  - planın noktaları ve notu durumunki; plan çizime yazmaz;
  - şevlerle birlikte çizilince nokta yalnız şevin köşelerine yazılır. Her kotun yazısı noktasının sağındadır;
  - `spotHeight`'sız paket reddedilir, çizim değişmez.
- **MCP** (`crates/native/mcp/tests/sheet_spot_heights.rs`): seçilen noktaların ve şevlerle birlikte durumun planı
  durumlarınki; kotlar ve `surface` 10⁻⁹ m içinde. Önceki beş durum değişmeden geçer.
- **Masaüstü** (`sheet_contours_window_tests::the_chosen_points_heights_are_the_shared_case_s`):
  - “secilen” katmanı adından tahmin edilir. Kutu işaretlenince noktalar ve `surface` ortak durumunkidir;
  - not farkı (0,145 m) ve yazılmayan noktayı söyler;
  - Çiz her seçilen nokta için yalnız bir yazı ekler.

  Pafta eğrilerinin öbür testleri ve çapraz denetimler (module_tests, ribbon_tests, history::text, catalog, settings)
  geçer. Resimler `.run/gui-evidence/secilen-noktalar-3520/`: pencere, çizim ve ölçülmüş noktanın yakını (858.6), açık
  geniş ve koyu dar.

## Sonra

- m. 63(2)'nin yerleşik alanda yaklaşık 2 cm'de bir kot noktası: hangi noktaların seçileceği ölçmecinindir. KentOS'un
  önereceği bir yerleştirme kuralı kaynak ister.
- Farkın bir eşiği aşınca uyarı olması: eşik yönetmelikte yok, kaynak beklenir.
- Yüzey birleştirme; bölgesel yeniden kurma (MAP-14'ün öbür maddeleri).
- Web (TODOS §0.2).
