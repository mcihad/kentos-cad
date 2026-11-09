# 3480 — Ayırma çapında dış parçanın numarasının çizimde yazılması

Tarih: 2026-11-09. plan/map-cad.md MAP-11, altıncı tur. Durum: taslak. ADR 3080'in “Sonra”sındaki “dış parçanın
numarasının çizimde, parçanın içinde yazılması; parçanın öznitelik olarak saklanması”.

İlgili:
- [ADR 2680](2680-separation-sheets.md): ayırma çapı paftaları;
- [ADR 3080](3080-separation-sheet-outside-part-numbers.md): dış parçaların parsel numaraları (m. 13/3).

## Dayanak

- **Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik**, RG 22.02.2020/31047 (ADR 3080'in okuması; TÜRMOB'un kopyası):
  - **m. 13(3):** düzenleme sınırı dışında kalan parçaların hepsi aynı çapta gösterilir. Dışarıda kalan her parça parsel
    numarası verilerek tescil edilir;
  - yönetmelik numaranın çapta nereye ve nasıl yazılacağını söylemez (ekleri: özet cetveli, araştırma formu, dağıtım
    cetveli, tescil sayfası, muhdesat ve irtifak cetvelleri).
- **KentOS bugün** (ADR 3080):
  - numaralar yalnız Kısımlar tablosunda parçanın adının yanında yazılıyordu (“Girmeyen 1 · parsel 101/24”);
  - çapın haritasında hangi parçanın hangi numarayı aldığı görünmüyordu;
  - parça çizimde bir nesne değildi, numarası hiçbir nesnede saklanmıyordu.

## Karar

1. **Numaralı dış parça çizime yazılır** (`cad.readjustment.sheets`, aynı geri alma adımında). Parselin nokta
   katmanının yanında, aynı grupta kendi katmanına: “‹Ada/Parsel› dış parçalar”. İki nesne:
   - parçanın kendisi alan olarak (halkaları, delikleri, yayları), öznitelikleri `Parsel`, `Kısım` (çaptaki adı,
     “Girmeyen 1”) ve `ParselNo` (verilen numara, kırpılmış);
   - içinde bir nokta: aynı öznitelikler, etiketi numara.
2. **Numaranın yeri** (`label_at`, belirli kural; bağımsız başvuru aynısını bulur):
   - parçanın dış halkasının köşelerinin orta yüksekliğindeki yatay çizgi alınır;
   - halkaların bu çizgiyi kestiği yerler sırayla ikişer eşlenir. Düz kenar, uçları çizginin iki yanındaysa keser (üst
     uç sayılır). Yay, çemberinin çizgiyle buluştuğu ve başlangıcından taradığı açının içinde kalan yerlerde keser (sonu
     sayılmaz);
   - en geniş aralığın ortası seçilir, eşitlerden ilki.

   Yer parçanın biçimi ne olursa olsun parçanın içindedir: bağlı bir parçanın her ara yüksekliği bir kesit verir.
   Ağırlık merkezi (alanın etiket çapası) parçanın dışına düşebilir.
3. **Nokta katmanı yalnız köşeleri taşır:** çapın Koordinat listesi parselin nokta katmanını okur. İlk resimde etiket
   noktaları listeye “101/24” satırı olarak giriyordu; bu yüzden parçalar ayrı katmandadır. Çapın haritası parselin iki
   katmanını gösterir, öbür parsellerin katmanlarını dondurur.
4. **Cevap:** her çapın `labels`'ı (ek alan; komut sürümü değişmez): parçanın adı, numarası ve yeri. Numara verilmeyen
   parsel için boştur ve nesne yazılmaz.
5. Numara verilir, üretilmez (ADR 3080); tescil bildirimi yapılmaz (biçimi TKGM'nin). Çap taslaktır; onay parselasyon
   planıyladır (m. 13/2).
6. **Kapsam işareti:** ADR 2680'deki gibi **Harita ve CBS**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/readjustment_sheets_cases.py`; kesişmeler ve yay çemberi betikte, KentOS kodu
  yok):
  - numaralı durumun ve yeni durumun çaplarında `labels` (35 durum);
  - yeni durum: üç parselin dış parçaları numaralı. İki yamuk ve iki dikdörtgen parça;
  - eski durumlar aynı. Rust ve Python geçer.
- **Python** (`python/tests/test_separation_labels.py`):
  - etiketler başvurununki;
  - dört parça alan olarak numarasıyla, dört etiketli nokta;
  - parçalar köşelerden ayrı katmanda; numarasız çağrıda parça yok.
- **MCP** (`crates/native/mcp/tests/separation_labels.rs`): etiketler durumunki; kaydedilen dosyada her numara bir alanın
  özniteliği ve bir noktanın etiketi. ADR 2680/3080'in MCP testi değişmeden geçer.
- **Masaüstü** (`readjustment_window_tests::the_parts_outside_take_the_typed_parcel_numbers`):
  - Dış parça no ile Çapın paftaları; 101/10'un parça katmanında iki alan ve içlerinde etiketleri;
  - nokta katmanında yalnız köşeler;
  - resimler `.run/gui-evidence/dis-parca-no-3480/`: 101/10'un çapında numaralar parçalarda, Koordinat listesinde
    yalnız köşeler.

  Düzenleme penceresinin öbür testleri ve çapraz denetimler geçer.

## Sonra

- ~~Numaranın yazısının parçanın içine sığdırılması (dar parçada etiket parçanın dışına taşıyor; yazının boyu çap
  ölçeğinde).~~ ADR 4280.
- Tescil bildirimi (m. 22(1)(ğ)): TKGM'nin biçimi beklenir.
- ~~Kapsam dışı kimlikte retin parseli adıyla söylemesi (ADR 3080 Sonra).~~ ADR 3880.
- Web (TODOS §0.2).
