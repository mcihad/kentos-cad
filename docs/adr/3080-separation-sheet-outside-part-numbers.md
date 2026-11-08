# 3080 — Ayırma çapında dışarıda kalan parçaların parsel numaraları

Tarih: 2026-11-09. plan/map-cad.md MAP-11, beşinci tur. Durum: taslak. ADR 2680'in “Sonra”sındaki “dış parçalara parsel
numarası (m. 13(3))”.

İlgili:
- [ADR 2060](2060-readjustment-scope-and-separation-sheet.md): düzenleme kapsamı, ayırma çapının sayıları;
- [ADR 2280](2280-separation-sheet-parts-and-coordinates.md): parçaların sınırları ve koordinatları;
- [ADR 2680](2680-separation-sheets.md): ayırma çapı paftaları (`cad.readjustment.sheets`).

## Dayanak

- **Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik**, RG 22.02.2020/31047. TÜRMOB'un kopyası
  (<https://www.turmob.org.tr/arsiv/mbs/resmigazete/31047-31047-1.pdf>), 9 Kasım 2026'da metni çıkarılarak yeniden
  okundu:
  - **m. 13(3):** parselin düzenleme sınırı dışında kalan parçaları birden çoksa her biri için ayrı çap gerekmez, hepsi
    aynı çapta gösterilir. Dışarıda kalan her parça parsel numarası verilerek parselasyon planıyla birlikte tescil edilir;
  - numaranın nasıl verileceği yazılmaz. **m. 21(4)**'ün kuralı (1'den, kuzey batıdan saat yönünde) imar adası içindeki
    imar parselleri içindir, dış parçalar için değildir;
  - **m. 22(1)(ğ):** parselasyon planı dosyasında, kısmen giren parsel varsa ayırma çapı ve tescil bildirimi bulunur.
    Yönetmeliğin ekleri şunlardır: özet cetveli (Ek-1), mal sahipleri araştırma ve özet formu (Ek-2), tescile esas
    dağıtım cetveli (Ek-3), tescil sayfası (Ek-4), yapı-muhdesat cetveli (Ek-5), irtifak hakkı cetveli (Ek-6). Tescil
    bildiriminin biçimi verilmez.
- **KentOS bugün** (ADR 2680):
  - Kısımlar tablosu dışarıdaki parçaları “Girmeyen 1”, “Girmeyen 2” diye sayıyordu;
  - parçanın tescil edileceği numara yazılamıyordu.

## Karar

1. **`numbers`** (`cad.readjustment.sheets`, ek alan; komut sürümü değişmez):
   - kısmen giren parselin kalıcı kimliğiyle, dışarıda kalan parçalarının sırasıyla birer parsel numarası;
   - Kısımlar tablosunda parçanın adının yanında yazılır: “Girmeyen 1 · parsel 101/24”;
   - numara verildiği gibi yazılır, yalnız baştaki ve sondaki boşluk atılır. KentOS numara üretmez, biçimini (taksimli
     olup olmadığını) denetlemez;
   - numarası verilmeyen parsel eskisi gibi yalnız sayıyla.
2. **Retler** (`invalid_input`, `numbers.‹uid›`; hiçbir şey yazılmaz):
   - kimlik kapsamda kısmen giren bir parsel değilse;
   - numara sayısı dışarıda kalan parça sayısını tutmuyorsa (“… 2 parçası var; 1 parsel numarası verildi”);
   - numara boşsa ya da 20 karakterden uzunsa (`numbers.‹uid›[k]`);
   - aynı parselin iki parçasına aynı numara verilmişse.
3. **Masaüstü** (Düzenleme dağıtım taslağı, Ayırma çapı görünümü):
   - **Dış parça no** kutusu: “101/10: 101/24, 101/25; …” (Ada/Parsel, iki nokta, numaralar virgülle, parseller noktalı
     virgülle);
   - parselin adı kapsamın kısmen giren parsellerinden kimliğe çevrilir. Okunmayan yazım ya da kısmen girmeyen parsel,
     Çapın paftaları'na basılınca uyarıyla söylenir;
   - açıklama satırı m. 13/3'ü ve “KentOS numara vermez”i söyler.
4. **Şablon:** KentOS'un ayırma çapında (`sys:ayirma-capi`) Kısımlar tablosunun Kısım sütunu 19 mm'den 31 mm'ye, m²
   sütunu 22 mm'den 16 mm'ye geçti. İlk resimde numaralı ad iki satıra kırılıyordu; şimdi tek satırda.
5. **Tescil bildirimi** yapılmaz: biçimi yönetmelikte yok, TKGM'nin biçimi beklenir.
6. **Kapsam işareti:** ADR 2680'deki gibi Harita ve CBS (pencere Yol ve Mimari'de sunulmaz).

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/readjustment_sheets_cases.py`, KentOS kodu yok): üreteç numaralı adı kendisi
  yazar.
  - Kabul: 101/10'un iki dış parçası “101/24” ve “ 101/25 ”; tablo “Girmeyen 1 · parsel 101/24”, “Girmeyen 2 · parsel
    101/25”;
  - dört ret: sayı tutmuyor, kısmen girmeyen parsel, aynı numara iki parçada, boş numara;
  - 34 durum (29'dan). Şablonun sütun genişlikleri durumun cevabında da değişti. Rust ve Python geçer.
- **Şablon:** `fixtures/sheet/v1/svg/ayirma-capi.svg` yeniden yazıldı; fark yalnız Kısımlar'ın başlıklarının yeri ve
  sütun çizgileridir. Şablon testleri geçer.
- **MCP** (`crates/native/mcp/tests/separation_sheets.rs`, `the_parts_outside_take_their_parcel_numbers_over_mcp`):
  - bir numara eksikken ret; ileti ve yol durumunki;
  - ardından ikisiyle Kısımlar durumunki (ret bir şey yazmamıştır, pafta “zaten var” denmez).
- **Masaüstü** (`readjustment_window_tests.rs`, `the_parts_outside_take_the_typed_parcel_numbers`):
  - kutunun okunması: ad kimliğe çevrilir; kısmen girmeyen parsel ve iki noktasız yazım reddedilir; boş kutu numarasızdır;
  - bir numara eksikken komutun reddi iletide; çizim değişmez, pencere kalır;
  - ikisiyle paftalar yapılır.

  Düzenleme penceresinin testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/dis-parca-3080/` (pencere,
  101/10'un paftası, paftaların PDF'i).

## Sonra

- Tescil bildirimi (m. 22(1)(ğ)): TKGM'nin biçimi beklenir.
- Dış parçanın numarasının çizimde, parçanın içinde yazılması; parçanın öznitelik olarak saklanması.
- Kapsam dışı kimlikte retin parseli adıyla söylemesi (bugün kimlikle; masaüstü adıyla söylüyor).
- Web (TODOS §0.2).
