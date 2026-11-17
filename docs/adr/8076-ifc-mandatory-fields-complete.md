# ADR 8076: IFC'nin zorunlu alanları — saha, kat kotu ve yükseklikleri, posta kutusu, duvar kaplaması, gerçek kuzey

- **Durum:** yapıldı (17 Kasım 2026); IFC-02'nin ilk kabulü tuttu, kalanlar §7'de. Plan:
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-02, [bim-plan.md](../../plan/bim-plan.md) BIM-07/BIM-14.
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) (binanın IFC'si), [8040](8040-vertical-model.md) (döşeme
  katmanları, kotlar), [8068](8068-fire-performance-material-declarations.md) (yangın ve performans beyanları, zorunlu
  satırların ölçüsü), [8070](8070-missing-building-elements.md) §6 (kaplama kalınlıkları, K44),
  [0721](0721-architectural-symbols.md) §3 (kuzey oku, meridyen yakınsaması), [8074](8074-ek9-from-the-written-file.md)
  (EK-9 yazılmış dosyadan, IDS, iki bağımsız okuyucu), [8075](8075-bim-permit-package.md) (BIM ruhsat paketi).
- **Sorun:** 8075'in ilk diliminde gerçek kuralla hiçbir paket yayımlanamıyordu: künyeli kesit binasının modelinde 30
  zorunlu satır eksikti. Bir kısmı yazarın beyanıydı (yangın sınıfı, acil çıkış, kaplama adları, imar durumu); bir
  kısmını KentOS hiç yazmıyordu: sahanın nesne tipi ve toplam alanı, katın taşıyıcı döşeme kotu, saçak yüksekliği, posta
  kutusu, duvar kaplamasının kalınlığı, bağlamın gerçek kuzeyi.

## §1. Araştırma

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| EK-6 Tablo 6.55 (PDF s. 94–96) | `IfcSite` Nesne Tipi zorunlu; açıklaması: kullanıcı tipi "somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa" bu özniteliğe girilir. `Pset_SiteCommon.TotalArea` "arsaya ilişkin toplam planlanan alan" | K52, K53 |
| IFC 4.3 `IfcSite` | Ön tanımlı tip özniteliği yok | Tip `ObjectType`'ta (Tablo 6.55'in 2. durumu) |
| IFC 4.3 `TotalArea` (buildingSMART IFC4.3.x-development, docs/properties/t/TotalArea.md) | "Total planned area for the site. Used for programming the site space." | İmar durumunun parsel alanı |
| EK-6 Tablo 6.8 (s. 45–46) | `ElevationOfSSLRelative` "taşıyıcı döşeme seviyesinin üst yüzey kotu, yerel sıfır yüksekliğine göre … seviye değişkenlik gösteriyorsa ve belirgin biri baskın kot bulunmuyorsa, bu özellik girilmeyebilir" | K54 |
| IFC 4.3 `ElevationOfSSLRelative` (docs/properties/e/ElevationOfSSLRelative.md) | "above the local zero height … if there is no significantly more prominent elevation, then this property may be omitted … the geometric representation takes precedence" | ±0.00'a göre; baskınlık alanın yarısından çoğu |
| EK-6 Tablo 6.7 (s. 42–45), `Height` | "standart brüt yüksekliği; inşaat döşemesinin üst yüzeyinden bir üst döşemenin veya çatının üst yüzeyine … sadece yüksekliğin sabit olduğu durumda" | K55 |
| EK-6 Tablo 6.7, `EavesHeight` | Türkçe adı (bilgi amaçlı) "Saçak Yüksekliği"; açıklaması "bu katın standart net yüksekliği: inşaat döşemesinin üst yüzeyinden bir üst döşemenin veya çatının alt yüzeyine … sadece yüksekliğin sabit olduğu durumda" | K55: açıklama bağlar, saçak kotu değil |
| IFC 4.3 `EavesHeight` (docs/properties/e/EavesHeight.md), `Qto_BuildingBaseQuantities` | Aynı metin: "standard net height … to the bottom surface of the construction floor or roof above. Only provided is there is a constant height" | Tablonun açıklamasıyla aynı |
| EK-6 Tablo 6.7, `Pset_Address.PostalBox` | Zorunlu; "tanımlı bir posta kutusuyla ilişkilendirilen adres" (IFC 4.3: "an address that is implied by an identifiable mail drop") | K56 |
| EK-6 Tablo 6.58, Tablo 6.20 dipnotu | Mahalin `Pset_SpaceCoveringRequirements` kaplama adları ve kalınlıkları; kaplama mahalin setiyle ya da `IfcCovering` ile | K57 (K44'ün birinci yolu) |
| EK-7 Tablo 7.8 (s. 231–232) | `TrueNorth` zorunlu: "gerçek kuzey yönü (true north) veya coğrafi kuzey yönü, projenin kendi koordinat sistemine göre … iki boyutlu bir yön vektörü … `IfcMapConversion` kullanılmışsa True North yalnızca bilgilendirme amaçlıdır" | K58 |
| IFC 4.3 `IfcGeometricRepresentationContext.TrueNorth` (docs/schemas/resource/IfcRepresentationResource) | "Direction of the true north, or geographic northing direction, relative to the underlying project coordinate system … If not present, it defaults to 0. 1." | Bilinmeyen kuzey yazılmaz: varsayılan bir söz olurdu |
| IFC 4.3 sözlüğü (`data/ifc4x3-sozluk.json`) | `TotalArea` IfcAreaMeasure, `ElevationOfSSLRelative` IfcLengthMeasure, `WallCoveringThickness` IfcPositiveLengthMeasure, `PostalBox` IfcLabel, `EavesHeight` Q_LENGTH | Değerler bu tiplerle |

## §2. Karar

- **K52 — Sahanın tipi "Parsel".** KentOS'un sahası projenin parselidir: adı MM-SAH-Parsel, setleri parselin
  (`Pset_LandRegistration`, `TREpys_ParselOzellikSeti`, imar durumunun oranları). IFC 4.3'te `IfcSite`'ın ön tanımlı tipi
  olmadığından tipi `ObjectType`'a yazılır (Tablo 6.55'in 2. durumu).
- **K53 — Sahanın toplam alanı parselin alanı.** `Pset_SiteCommon.TotalArea` imar durumunun parsel alanıdır
  (`Qto_SiteBaseQuantities.GrossArea` ile aynı değer). İmar durumu yoksa yazılmaz, eksiktir.
- **K54 — Katın taşıyıcı döşeme kotu.** Her mahal üstünde durduğu döşemenin taşıyıcısının üstünü söyler (ADR 8040:
  döşemenin üstü eksi üst katmanlar); katın kotu, mahallerin alanının yarısından çoğunun taşıdığı değerdir (IFC 4.3'ün
  "significantly more prominent" sözünün KentOS okuması), binanın ±0.00'ına göre. Yarıyı geçen değer yoksa yazılmaz.
  - Mahalin döşemesi: noktasının altındaki döşeme; noktası döşemenin bir boşluğunun (merdiven boşluğu) üstündeyse,
    mahalin içindeki 9 × 9 noktalık ızgarada en çok noktanın altındaki döşeme. Izgara yalnız hangi döşeme olduğunu seçer;
    okunan değerler döşemenin kendisinindir. Eğimli döşemenin kotu tek değer değildir: sayılmaz.
- **K55 — Katın ve binanın yükseklikleri inşaat döşemesinden.** Tablo 6.7'nin ve IFC 4.3'ün metni "inşaat döşemesinin
  üst yüzeyinden" ölçer: katın `GrossHeight`'ı taşıyıcı üstünden üstündeki döşemenin taşıyıcı üstüne, `NetHeight`'ı
  taşıyıcı altına; mahallerinkinden K54'ün kuralıyla. Binanın `Height`'ı katların brüt, `EavesHeight`'ı net yüksekliğidir;
  ikisi de yalnız bütün katlarınki aynıyken yazılır ("sadece yüksekliğin sabit olduğu durumda").
  - Eskiden katın yüksekliği kat kotundan kat kotuna (bitmiş döşemeden) yazılıyordu; döşeme katmanları farklı katlarda
    (zemine oturan 13 cm, ara kat 7 cm, teras 17 cm) bu, tanımdaki yükseklik değildir.
  - `EavesHeight`'ın Türkçe adı "Saçak Yüksekliği"dir, ama tablonun başlığı adı "bilgi amaçlı" sayar; açıklama ve IFC
    4.3'ün tanımı net kat yüksekliğidir. KentOS açıklamayı tutar; saçak kotunu yazmaz.
  - Koşul ölçüsü: katların yükseklikleri dosyada farklıysa binanın iki satırı sorulmaz (`required::not_constant`). Aynı
    koşulu bağımsız okuyucu (IfcOpenShell ile form okuyucusu) ve standart kitaplıkla IDS denetimi kendi kodlarıyla okur.
    IDS 1.0 koşul yazamaz: profil iki satırı her zaman ister, gerekliliğin sözü koşulu söyler ("yalnız katların yüksekliği
    aynıyken sorulur"); IfcTester karşılaştırması bu durumda satırı "koşullu" diye ayrı tutar, ayrışma saymaz.
- **K56 — Posta kutusu künyeden.** Künyeye "Posta kutusu (IFC modelinde; yoksa “Yok”)" kutusu eklendi; değer
  `Pset_Address.PostalBox`'a ve binanın `IfcPostalAddress.PostalBox`'ına yazılır. KentOS bir şey doldurmaz: posta kutusu
  olmayan yapıda "Yok" müellifin kendi beyanıdır; boş kutu eksiktir.
- **K57 — Mahalin duvar kaplamasının kalınlığı duvarların yüzünden.** Duvar katmanlarını sol yüzünden sayar; mahalin
  düz bir kenarı duvarın düz bir parçasının yüzünde (yüzün çizgisine bir mikrometre içinde ve ona paralel) duruyorsa,
  mahal sol yüzde ilk, sağ yüzde son katmanı görür. Bu katman bir kaplamaysa (öncelik 5 ve zayıfı: sıva, levha; ADR
  0340 §1) kalınlığı o kenarın değeridir; daha güçlü katman (çekirdek) kaplama değildir. Mahalin değeri, yüzdeki
  kenarlarının uzunluğunun yarısından çoğunun taşıdığı kalınlıktır; yoksa yazılmaz. Duvarlardan ayrı çizilmiş mahal ve
  eğri duvar şimdilik değer almaz.
- **K58 — Gerçek kuzey projenin sisteminden, yoksa kuzey okundan.** Projenin koordinat sistemi varsa gerçek kuzey,
  duvarların ortasında meridyen yakınsamasıdır (kuzey okunun hesabı, ADR 0721 §3: `kentos_geometry_core::System::true_north`);
  model düzleminde birim vektör olarak bağlama yazılır. Sistem yoksa çizimdeki kuzey oklarının ortak dönüşü müellifin
  kuzeyidir. Hiçbiri yoksa ya da oklar farklı yönleri gösteriyorsa yazılmaz (IFC'nin varsayılanı, bilinmeyeni bilinir
  gösterirdi); neden olguların `export.northWhy`'ında, EK-9/9'un onarımında söylenir.
- **Kullanım türü künyeden de.** `OccupancyType` bina programının kullanımıdır; program yoksa künyenin elle yazılan
  "Kullanım amacı"dır (paftalar da onu okur). Önceden program yoksa yazılmıyordu.

## §3. Uygulama

- `kentos_ifc::arch` (`building`): sahanın `object_type` ve `TotalArea`'sı; katların `Pset_BuildingStoreyCommon`'u ve
  `Qto_BuildingStoreyBaseQuantities`'in `GrossHeight`/`NetHeight`'ı (`Vote`, `prominent`); binanın `Height` ve
  `EavesHeight`'ı; mahalin `WallCoveringThickness`'ı; `Input::north`; `Particulars::postal_box`.
- `kentos_ifc::building`: `Building::true_north`, `north_why`; bağlamın `TrueNorth`'u (`IFCDIRECTION`), bilinmezse `$`;
  `Address::postal_box`.
- `kentos_ifc::required`: miktarın değeri (`Carrier::quantities`); binanın yükseklik satırlarının koşulu (`CONSTANT`,
  `not_constant`). Okuyucu (`read::facts`) aynı değeri dosyadan verir.
- `kentos_ifc::ids`: iki satırın gereklilik sözü koşulu söyler; `data/temel-2026.ids` yeniden yazıldı.
- `kentos_domain`: `room_structure` (`RoomStructure`), `room_floor_slab` (K54'ün ızgarası; `room_coverings` da kullanır),
  `room_walls::room_wall_finish` (K57).
- `kentos_native_application`: `north::model_north` (K58), `arch_ifc` (mahallerin yapısı ve duvar kaplaması, kuzey,
  posta kutusu, kullanımın künyeden okunması), `ifc_facts` (`export.northWhy`).
- `kentos_sheet::permit::PARTICULARS`'a `posta_kutusu`; Ruhsat takımı'nın grubu "Dijital teslim (EK-1, EK-2, EK-6, EK-7)".
- `dijital_proje.form` (s9): gerçek kuzeyin onarımı olgulardaki nedenden.

## §4. Sınamalar

- `kentos-ifc`: `required::the_building_s_heights_are_asked_only_where_its_storeys_have_one_height`,
  `building::true_north_is_the_model_context_s_direction_and_an_unknown_one_is_not_written`; okuyucunun örneklerine
  `kesit-binasi-tam` (IfcOpenShell'in okuması `fixtures/ifc/v1/okuma/`, olgular `fixtures/ifc/v1/olgular/`).
- `kentos-domain`: `a_room_s_construction_floor_is_its_slab_s_structure_and_the_slab_over_it_bounds_its_heights`,
  `room_walls::a_room_s_wall_covering_is_the_finish_of_the_faces_it_lies_along` (sol/sağ yüz, yarıdan çoğu, çekirdek,
  5 cm açıkta duran mahal).
- Uygulama (`tests/all/ifc_required.rs`): yazar gibi doldurulmuş kesit binası (künye, imar durumu, vaziyet planının
  parseli, `cad.arch.performance` ile yangın sınıfları ve çıkışlar, kaplamalar) — zorunlu satırların hiçbiri eksik değil;
  katların kotu −0,13 ve +2,93, yükseklikleri 3,06/2,91 ve 2,90/2,75, binanın yükseklikleri yazılmıyor; saha Parsel ve
  400 m², posta kutusu Yok, her mahalin duvar kaplaması 0,02, gerçek kuzey birim vektör.
  `KENTOS_WRITE_IFC_BUILDING=1` `fixtures/arch/v1/ifc/kesit-binasi-tam.ifc`'yi yazar.
- Başsız yayın (`tests/headless/bim_package.rs`): çizildiği gibi bina gerçek kuralla durur (kuzeyin bulgusu "Kuzey oku"
  onarımıyla); yazar gibi doldurulmuş bina gerçek kuralla yayımlanır: beş sonuçtan dördü Tutuyor, idare Bekleniyor; model
  klasörden geri okunur, formun bütün satırları Evet.
- Masaüstü (`bim_permit_tests`): `a_declared_building_s_package_holds_and_is_published`; resimler
  `.run/gui-evidence/bim-ruhsat-paketi-8075/3-gecen-paket-*.png`, `.run/gui-evidence/ifc-zorunlu-8076/`.
- Bağımsız okuyucular: `arch_ifc_check.py` katların kotunu ve yüksekliklerini çizimin kendi tanımının döşemelerinden
  hesaplar; `ifc_form_reference.py`, `ids_ifctester_check.py`, `ids_check.py` beş dosyada.

## §5. Kabul (ifc-uyum IFC-02 ilk kabul)

- **Zorunlu satırlar:** yazar gibi doldurulmuş künyeli kesit binasında katalogun zorunlu satırlarından eksik yok
  (`required::gaps` boş); çizildiği gibi binada her eksik nesnesiyle bulgudadır.
- **Bağımsız okuyucular:** IfcOpenShell'in doğrulayıcısı üç dosyada (kesit-binasi, -kunye, -tam) 0 bulgu; tanımdan
  hesaplanan kotlar ve yükseklikler dosyayla aynı. IfcOpenShell ile form okuyucusu `kesit-binasi-tam`'da 9. soruyu
  geçti buldu (bağlamın kuzeyi dahil). IfcTester 0.9.0 beş dosyada KentOS'la ayrışmadı; `kesit-binasi-tam`'da eksik
  satırı yok (katların yüksekliği farklı olduğundan iki koşullu satır ayrı).
- **Gerçek paket:** başsız yayında ve masaüstünde gerçek kuralla yayımlandı. Klasörün bağımsız okuması: qpdf/Poppler
  (`pdfa_bagimsiz.py`) dört paftada ve teslim listesinde 0 bulgu; modelde IfcOpenShell'in doğrulayıcısı 0 bulgu;
  IfcTester'da 78 tanımdan yalnız Tablo 6.7'nin binası koşullu iki satırla (katlar 3,06 ve 2,90 m).
- **Çatının ve merdivenin tipi:** ADR 8070'te dolu; bu dilimde değişmedi.

## §6. Ölçüm

100 dairede model eşlemesinin süresi (ADR 8074 §6'nın ölçümü, Apple M5, release) bu dilimden sonra aşağıdadır; mahal
başına duvar yüzü taraması ve boşluk üstündeki mahalin ızgarası eklendi.

## §7. Açık işler

- **Baskın kot yoksa:** Tablo 6.8 kotun "girilmeyebileceğini" söyler; KentOS yazmaz ama zorunlu ölçü bunu henüz ayırmaz,
  eksik sayar (bina katında nadir; ayrım ölçünün nedenini taşımasını ister).
- **Çatı altındaki son kat:** üstünde döşeme olmayan katın yükseklikleri bilinmez; binanın yükseklik satırları o zaman
  koşulla ayrılamaz, eksik görünür. Çatının alt yüzü değişkendir: yükseklik "sabit" değildir; ölçünün bunu söylemesi
  sonraki iştir.
- **Duvar kaplaması:** duvardan ayrı çizilmiş mahal ve eğri duvar değer almıyor.
- **Örnek çizimin yeri:** kesit binasının koordinatları sistemin başlangıcının yanında; gerçek kuzeyi neredeyse 0'dır.
  Gerçek bir parselin koordinatlarında örnek sonraki ölçümdedir.
- **IFC-02'nin kalanları:** `Tag` politikası (elemanın numarası ya da tip kodu), kişinin adı ve soyadı ayrı alanlardan
  (bugün son kelimeden), EK-7 Tablo 7.9 ölçüm anotasyonunun uygulanabilirliği ve ölçüm kaynağına bağlanması.
