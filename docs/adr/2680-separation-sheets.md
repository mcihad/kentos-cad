# 2680 — Ayırma çapı paftaları (`cad.readjustment.sheets`)

Tarih: 2026-11-08. plan/map-cad.md MAP-11, dördüncü tur. Durum: taslak. Plan satırı: “Arazi düzenlemesi … hesap,
tutanak ve çıktıları denetlenebilir olsun”; ADR 2280'in “Sonra”sındaki “ayırma çapının paftası: parsel, sınır,
parçalar, alan ve koordinat tablosu tek sayfada”.

İlgili:
- [ADR 2060](2060-readjustment-scope-and-separation-sheet.md): düzenleme kapsamı, ayırma çapının sayıları;
- [ADR 2280](2280-separation-sheet-parts-and-coordinates.md): parçaların sınırları ve koordinatları;
- [ADR 0592](0592-common-sheet-creation-and-fit.md) ve [ADR 0718](0718-level-plan-sheets.md): şablondan pafta ve paftayı nesneye sığdırma
  (`cad.sheets.create`, `cad.sheets.fit`, `cad.sheets.levels`).

## Dayanak

- **Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik**, RG 22.02.2020/31047. Resmî Gazete'nin PDF'i, TÜRMOB'un kopyası
  (<https://www.turmob.org.tr/arsiv/mbs/resmigazete/31047-31047-1.pdf>; içindeki bağlantı
  <https://www.resmigazete.gov.tr/eskiler/2020/02/20200222-2-1.pdf>), 8 Kasım 2026'da metni çıkarılarak okundu:
  - **m. 4(1)(c):** ayırma çapı, bir kısmı düzenleme sahasında kalan parsel için ilgili idarenin onayladığı krokidir;
  - **m. 13(1):** bir kısmı düzenleme sahasında kalan parsel için ayırma çapı düzenlenir;
  - **m. 13(2):** çapta parselin tescilli ve hesaplanan yüzölçümü, fark ve yanılma sınırı, giren ve girmeyen kısımların
    yüzölçümü, sınırları ve koordinatları gösterilir; çaplar parselasyon planıyla birlikte onaylanır;
  - **m. 13(3):** dışarıda kalan parçalar birden çoksa her biri için ayrı çap gerekmez, hepsi aynı çapta gösterilir;
    her parça parsel numarası alır;
  - **m. 13(4):** çap parselasyon planının parçasıdır;
  - **m. 22(1)(ğ):** parselasyon planı dosyasında kısmen giren parsel varsa ayırma çapı ve tescil bildirimi bulunur;
    **m. 23(1)(a):** açıklama raporu ayırma çapı sayısını söyler;
  - **m. 31(2)** (ADR 2060'ın okuması): kısmen girende giren kısmın hesaplanan yüzölçümü esastır, hata girmeyen kısımda
    kalır;
  - **m. 21(4):** imar parsellerine 1'den, kuzey batıdan saat yönünde numara verilir. Bu parsel numarasının kuralıdır;
    yönetmelik çapın noktalarına numara kuralı vermez.
- **Autodesk Civil 3D, “Map Check Report”**
  ([belge](https://help.autodesk.com/cloudhelp/2017/ENU/Civil3D-UserGuide/files/GUID-3D99202F-BDD3-4585-89E1-02DE47ECFD3A.htm),
  8 Kasım 2026'da okundu): parselin her kenarının yönü ve uzunluğu, başlangıç ve bitiş koordinatları, çevre, alan ve
  kapanma hatası; başlangıç noktası rapor için seçilir, parsele yazılmaz.
- **Netcad:** ayırma çapının çıktısını anlatan resmî belgeye ulaşılamadı; bir şey ondan alınmadı.
- **KentOS bugün** (ADR 2280): kapsam parçaları kanonik halkalarla verir; Ayırma çapı görünümü köşeleri her parçada
  1'den sayar; Çapı çizime yaz parçaları tek katmana yazar. Çapın paftası, noktaların numarası ve m. 31(2)'nin girmeyen
  kısma kalan alanı paftada yoktu.

## Karar

1. **Noktaların numarası** (`cad.readjustment.scope`'un cevabında, ek alan; sürüm değişmez):
   - parselin `points`'i: parçaların köşeleri sırayla (her parçanın halkaları, her halkanın köşeleri), 1'den;
   - bir köşe yeniden gelirse numarasını korur. “Aynı köşe” tam aynı koordinattır: bir parselin parçaları tek bindirmeden
     (parsel ile sınır) çıkar ve ortak köşeyi bir kez verir. Bu yüzden iki farklı köşeyi birleştiren bir tolerans yoktur;
   - her halkanın `numbers`'ı köşelerinin numaralarıdır.

   Yönetmelik nokta numarasına kural vermez: bu KentOS'un sırasıdır (ADR 2280'in kanonik halkaları).
2. **`cad.readjustment.sheets` v1** (Harita + CBS, belge yazan, tek geri alma adımı “Ayırma çapı paftaları”):
   - **Girdi:** `scope` (kapsamın girdisi), `source` (yoksa `sys:ayirma-capi`), `options` (kâğıt, ölçek, öbür sorulara
     cevaplar).
   - **Pafta:** kısmen giren her parsel için bir pafta, kapsamın sırasıyla (m. 13(1), 13(3)). Adı “Ayırma çapı
     ‹Ada/Parsel›”, kimliği `ayirma-capi-k`. Şablonun il, ilçe, mahalle, ada, parsel soruları parselin
     özniteliklerinden; pafta numarası ve kontrol eden kullanıcınındır (ön denetim eksik der).
   - **Noktalar:** “Ayırma çapı noktaları” grubunda parselin adını taşıyan katmana numaralarıyla etiketli noktalar
     (öznitelikleri Parsel, Nokta).
   - **Harita:** parselin kutusu `cad.sheets.fit`'in kuralıyla (%5 kenar payı, kâğıt çizelgesinin standart ölçekleri)
     sığar; ölçek verilirse o ölçekte ortalanır. Görünüm kilitlenir. Öbür parsellerin nokta katmanları bu haritada
     dondurulur: iki parselin ortak köşesi her paftada kendi numarasını gösterir.
   - **Koordinat listesi:** parselin nokta katmanını okur, noktaları adlarıyla (numara, Y, X).
   - **Yüzölçümleri tablosu:**
     - tescilli (yazıldığı gibi, virgülü nokta);
     - hesaplanan, yanılma sınırı, giren ve girmeyen kısım: politikanın basamağı ve yuvarlamasıyla, her biri bir kez;
     - fark: yazılan hesaplanan eksi tescilli, kesin (kayıt ya da politika basamağının çoğuyla). Kayan noktalı farkın
       yuvarlanması 1,61 verebiliyordu (600 − 598,4), tablo 600,00 − 598,40 = 1,60 der;
     - girmeyen kısma kalan (tescilli − giren): m. 31(2), kapsamın `outsideRecorded`'ı;
     - olmayan değer “—”.
   - **Kısımlar tablosu:** Giren ve Girmeyen (bir yanda birden çok parça varsa numaralı), sınırı nokta numaralarıyla
     (“1-2-3-4-1”, delik “(delik 5-6-7-8-5)”), alanı.
   - **Retler:**
     - sürüm; kapsamın kendi retleri `scope.…` yeriyle;
     - `no_partial_parcels`;
     - `no_identity`, `duplicate_parcel`: pafta parselin kimliğiyle adlandırılır;
     - `unknown_template`; `invalid_options` (ad, merkez ya da parselin özniteliğine cevap); `not_fixed_map` (`harita`
       kimlikli sabit görünüm yok);
     - `separation_sheet_exists`: pafta ya da nokta katmanı zaten var.
3. **Şablon `sys:ayirma-capi`** (A4 ve A3 dikey):
   - üstte AYIRMA ÇAPI, il, ilçe, mahalle, ada, parsel, pafta;
   - karelajlı harita; altında ölçek çubuğu ve kuzey oku: haritanın içinde beyaz kutuları noktayı ve kenarı örtüyordu;
   - Yüzölçümleri, Kısımlar ve Koordinatlar;
   - açıklama (taslaktır, m. 13(2)'nin onayı) ve imzalar.
4. **Masaüstü** (Düzenleme dağıtım taslağı › Ayırma çapı): Çapın paftaları düğmesi pencerenin sınırı, politikası ve şerit
   toleransıyla komutu çalıştırır, pencereyi kapatır, paftaları açar. Görünümün Nokta sütunu artık paftanın numarasıdır
   (girmeyen kısım 3, 2, 5, 6). Kısım adları paftanınkiyle aynı kuraldır.
5. **Resmî değildir:** çap parselasyon planıyla onaylanır (m. 13(2)); KentOS onay vermez, kuruma göndermez.

## Kanıt

- **Ortak durumlar:**
  - `scripts/fixtures/readjustment_scope_cases.py`: 33 durumda numaralar (L parça, delikli parça, iki dış parça);
  - `scripts/fixtures/readjustment_sheets_cases.py`, 29 durum, KentOS kodu yok. Ölçek şablonun dosyasındaki çerçeveden
    ve `papers.json`'un standart ölçeklerinden, tablolar elle yuvarlanarak:
    - üç parsel; tek parsel; okunmayan kayıt; iki dış parça; L ve delikli parça;
    - verilen ölçek; şablonun öbür soruları; sıfır basamak; sıfırdan uzağa; 3 mm tolerans;
    - tamamı giren ve dışarıdaki parsel; CBS; plan; doğrulama;
    - geri alma ve yineleme;
    - yeniden yapınca ret; on bir ret.

  Rust ve Python geçer; yetenek denetimi geçer.
- **Python** (`python/tests/test_readjustment_sheets.py`): tescilli = giren + girmeyen kısma kalan ve fark kesin;
  parçalar yan toplamını tutar; sınırların numaraları paftanın noktaları; yeniden çağrı reddedilir, hiçbir şey yazılmaz.
- **Pafta çekirdeği:** şablon her kâğıtta iki yönde ön denetimden geçer, kendi kâğıtlarında uyarısızdır; altın SVG
  `fixtures/sheet/v1/svg/ayirma-capi.svg`; galeri sırası `fixtures/sheet/v1/profiles/modes.json`.
- **MCP** (`crates/native/mcp/tests/separation_sheets.rs`): üç pafta durumunkiler; ikinci çağrı reddedilir.
- **Masaüstü** (`readjustment_window_tests.rs`):
  - kimliksiz parselde ret söylenir, hiçbir şey yazılmaz;
  - 11'e Ada verilince dört pafta, dört nokta katmanı, etiketler 1–6;
  - ilk haritada öbür üç katman donuk, liste kendi katmanını okur;
  - kitap PDF'e çıkar; tek geri alma.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/ayirma-capi-paftasi-2680/`: pencere, iki pafta, ön denetim ve
  PDF sayfaları.

## Sonra

- Çapın tescil bildirimi (m. 22(1)(ğ)): biçimi yönetmelikte yok, TKGM'ninki beklenir. Dış parçalara parsel numarası
  (m. 13(3)) yapıldı: [ADR 3080](3080-separation-sheet-outside-part-numbers.md).
- Önceki paftaların haritalarında sonradan eklenen nokta katmanlarının dondurulması.
- Değer esaslı dağıtım (resmî metinden kural okunamadı), m. 8(5) tablosu, ikinci kabul (MAP-11'in öbür maddeleri).
- Web (TODOS §0.2).
