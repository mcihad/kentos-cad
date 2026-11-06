# 0977 — Yolun eğimleri oran olarak tutulur, ekranda yüzde gösterilir

Tarih: 2026-11-06. Durum: kabul (Y70). Kapsam ROAD-DEVER-08; ROAD-NUM-11'in eğim kısmı.

## Sorun

ROAD-DEVER-08: "Grade/crossfall kaynakları oran biriminde canonical olsun; UI yüzde gösterimi açık çevrilsin."

Yol modülünde aynı tür sayı iki birimle yazılıyordu:

- **Yüzde:** tip kesit bileşeninin enine eğimi (`slope`), şev basamağının eğimi (`fall`), dever çizelgesinin normal
  eğimleri, kurbun oranı ve satırları, menfezin akış eğimi, platformun eğimi, kübajın diş kazısı eşiği, ön güzergâhın
  en dik eğimi, kavşak bağlamasının enine eğimi ve eğim değişimi.
- **Oran:** boykesitin eğimleri (`grade`, `gIn`, `gOut`), deverin göreli eğimi, arazinin noktadaki eğimi, havzanın
  eğimi, menfez hidroliğinin eğimi, platformdaki suyun en küçük eğimi.

Aynı ad iki anlam taşıyordu. Örneğin `grade`, `road.profiles.query`'de −0,0029 (oran), `road.platforms.edit`'te 2,0
(yüzde) idi. Deverin `road.superelevations.query` cevabı yüzde, `road.superelevations.check` cevabı oran veriyordu. Bir
API kullanıcısı ya da yapay zekâ ajanı birinden okuduğunu ötekine yazınca yüz kat yanılırdı.

## Araştırma (6 Kasım 2026)

- **IFC 4.3 (ISO 16739-1:2024):** Düşey hizalamanın `StartGradient`'i ve `EndGradient`'i `IfcRatioMeasure`'dür, yani
  birimsiz bir orandır ([IfcAlignmentVerticalSegment](https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/lexical/IfcAlignmentVerticalSegment.htm)).
  ODA'nın IFC kitaplığı değerin "normalde −1 ile 1 arası" olduğunu yazar
  ([setStartGradient](https://docs.intellicad.org/files/oda/2023_10/oda_ifc_docs/OdIfc4x3__IfcAlignmentVerticalSegment__setStartGradient@double.html)).
  IFC'nin `IfcSlopeMeasure`'ü ise bir açıdır; eğim için oran kullanılır.
- **LandXML 1.1/1.2:** `Superelevation/FullSuperelev` tam deveri yüzde yazar. Japon J-LandXML profili de enine eğimi
  yüzde, sola düşen eksi ister ([J-LandXML sürüm notları](https://www.mlit.go.jp/tec/content/J-LandXML_schema_v18_releasenotes.pdf),
  [NILIM LandXML 1.2 tabanlı şema](https://www.nilim.go.jp/lab/qbg/researchfields/pdf/h30_basedLandXML1.2.v1.2_beforeafter.pdf)).
  Yani yüzde, alışveriş biçiminin birimidir.
- **Mühendisin yazdığı:** KGM'nin tasarım rehberleri, Netcad ve Civil 3D'nin dever pencereleri enine eğimi yüzde
  gösterir. Civil 3D API'si `SuperelevationCriticalStation.GetSlope` için birim söylemez
  ([API](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/7de26688-8624-b90e-4f10-229bc97c9e4f.htm)).
- **AASHTO'nun K'sı:** Düşey kurbun K = L/A'sında A yüzdedir (tanımı böyledir, metre bölü yüzde). Burada yüzde bir
  birim seçimi değil, katsayının tanımıdır.

### Türkiye mevzuatı

- **3516 sayılı Ölçüler ve Ayar Kanunu** (RG 21.01.1989, sayı 20056): Uluslararası Birimler Sistemi'ni (SI) benimser.
  SI'da eğim birimsiz bir sayıdır, birimi `1`'dir. `%` yalnız 0,01 sayısının yazılışıdır, ayrı bir birim değildir. Bu
  yüzden oran da yüzde de yasal yazımdır; aynı alanda ikisinin karışması ise bir yazım yanlışıdır.
  ([lexpera](https://www.lexpera.com.tr/mevzuat/kanunlar/olculer-ve-ayar-kanunu-3516)). Uygulama yönetmeliği "Uluslararası
  Birimler Sistemine Dair Yönetmelik"tir; RG 07.12.1990, sayı 20718 künyesi ikincil bir kaynaktan alındı, Resmî Gazete'de
  doğrulanmadı.
- **KGM, Karayolları Kesin ve Ön Projeleri Mühendislik Hizmetleri Teknik Şartnamesi (2019)**
  ([kgm.gov.tr](https://www.kgm.gov.tr/SiteCollectionDocuments/KGMdocuments/MerkezBirimler/EtutProjeveCevreDairesiBaskanligi/Calismalar/tekniksartname.pdf)):
  - "Düşey hat eğim hesaplarında eğim yüzde cinsinden ifade edilecektir. Eğimler virgülden sonra üç rakam alınacaktır.
    (+%5.241, -%3.202)" (bölüm Düşey hat).
  - "(2x3) şeritli yollarda %2.50, diğer yollarda %2.00 dever değeri kullanılacaktır" (enkesit).
  - Some tablosu "uygulanan dever (S)" yazar; sapma açısı grad cinsindendir; profilin altında dever bandı bulunur.

  Yani idareye teslim edilen belgede eğim ve dever yüzdedir, eğim üç ondalıklıdır.
- **KGM Karayolu Tasarım El Kitabı:** Enine eğimi ve deveri yüzde yazar. Açık kopyalarında (2005 baskısı) çok şeritli
  yolda %2,5, iki şeritlide %2 bulunur; emax yerleşim dışında %8, şehir geçişinde %4'tür
  ([Enkesit elemanları](https://www.kentharita.com/wp-content/uploads/2017/02/Enkesit_elemanlarinin_tasarimi.pdf)).
  2022 baskısı ücretlidir ve okunamadı. Bu sayılar KentOS'a öntanımlı olarak girmez: kurumun standart profilinde,
  kaynağıyla birlikte durur.

Mevzuat bir iç hesap birimi buyurmaz, idareye teslim edilen belgenin yazımını buyurur. KentOS'un oranı içte tutup
ekranda ve belgede yüzde yazması iki yükümlülüğü de karşılar.

Sonuç olarak iki kabul görmüş kural var: hesap ve değişim biçiminde oran (IFC), ekranda ve mühendis belgesinde yüzde.
KentOS'un boykesiti zaten orandır.

## Karar

### Kaynakta oran

Bu alanlar artık orandır (düşey bölü yatay; −0,025 %−2,5'tir; dışa ya da yukarı yükselen artı):

| Kaynak | Alanlar |
|---|---|
| Tip kesit | `components[].slope`, `bench.fall`, `fillBench.fall` |
| Dever çizelgesi | `normalLeft`, `normalRight`, `method.curves[].rate`, `method.rows[].left/right` |
| Menfez | `slope` |
| Platform | `grade` |
| Kübaj | `bench.slope` (diş kazısı eşiği) |

Komutların girdisi ve cevabı da aynı birimi kullanır:

- **Dever:** `road.superelevations.edit` ve `.query` cevabındaki normal eğimler ve kırık noktaları (`station-grade`
  çiftleri), `at[].left/right`; `road.superelevations.transitions` satırlarının `rate`'i.
- **Ön güzergâh:** `road.routes.find` ve `.draw`'un `grade` girdisi ve `steepest` cevabı.
- **Kavşak bağlama:** `road.junctions.tie`'ın `crossfall` ve `maxGradeChange` girdisi; cevabındaki `grades`,
  `changes` ve bağların `crossfall`'ı.

**Birim sözlüğü.** Şemada iki yeni birim var:

- `grade`: "Eğim oranı";
- `station-grade`: (km, eğim) çifti.

Zaten oran olan alanlar da `grade` ile işaretlendi: boykesitin eğimleri, göreli eğim, `e`, noktadaki eğim, havzanın ve
menfez hidroliğinin eğimi, suyun en küçük eğimi, profil denetiminin en çok ve en az eğimi.

Eskiden yanlışlıkla K'nın birimi `m/%` ile işaretli olan dever kırık noktaları `station-grade` oldu.

**Doğrulama sınırları.** Sınırların anlamı aynı kaldı, oranla yazıldı:

- enine ve normal eğim |s| ≤ 1;
- basamak 0–0,5;
- menfez 0–1;
- platform |g| ≤ 1;
- diş kazısı (0, 10];
- en dik eğim (0, 10].

Ret iletisi birimi söyler: "enine eğim oranla yazılır (−0,025 %−2,5'tir), en çok 1 (%100) olur". Eski alışkanlıkla
−2,5 yazan çağrı böylece sessizce %−250 almaz, reddedilir ve nedenini okur.

**Değişmeyenler, gerekçeleriyle:**

- **Şev oranları** (`cut`, `fill`, hendeğin `fore`'u, korkuluk eşiğinin `slope`'u): yatay bölü düşeydir (1,5 = 3:2).
  Ayrı bir büyüklüktür, birimi `1` kalır.
- **A:** yüzdedir, K = L/A'nın tanımıdır.
- **Yuvarlanma direnci:** yüzde kalır (taşıma, Caterpillar el kitabının birimi). Bir geometri eğimi değildir.
- **Paylar ve kabul yüzdeleri:** pay, oran değildir.
- **Arazi analizinin eğim sınıfları** (`road.terrains.analysis` `slopes`, ızgara-üçgen eğim karşılaştırması): yüzde
  kalır. Bir CBS çıktısıdır, kaynak değildir. ROAD-NUM-11'in açığı olarak kalır (§Açık).

### Çekirdek

`kentos-road` yüzdeden orana geçti:

- tip kesitin kotları `z += w·s`;
- basamağın düşüşü;
- üstyapı;
- platform düzlemi;
- menfezin akış kotları;
- ön güzergâhın en dik adımı;
- dever fiziğindeki eksen kalkışı `W·(n − s)`;
- göreli eğim `d(W·s)/ds`;
- merkeze doğru dever.

Dever çizelgesinin kendisi doğrusaldır, birimden bağımsızdır; yalnız açıklamaları değişti.

### Kayıt: KCAD şema 187

- **Alanlar aynı, birim değişti.** Yazıcı yol ayarlarında tip kesit, dever, menfez, platform ya da diş kazılı kübaj
  varsa 187'yi seçer. Eski bir okuyucu oranı yüzde sanacağı için dosyayı açmaz, "daha yeni" der.
- **Okuyucu:** 186 ve öncesinin yol ayarlarında bu alanları 100'e böler (`RoadSettings::grades_from_percent`; IEEE
  bölmesi). Eski dosya bugünkü çizim olarak açılır.
- **Eski örnekler:** Her eski yol şemasının (162, 166–179) örneği `road-*-percent.kcad` olarak, kendi yazıcısının
  baytlarıyla durur (`rewrite: false`). Bunlar dalın önceki `road-*.kcad` dosyalarıyla bayt bayt aynıdır. İçerikleri
  `road-*.json`'dur: Rust ve Python okuyucuları onları oranla okur.
- **Şema kuralı:** "Her şemanın özelliği bir öncekinde reddedilir" kuralı 187'de bilerek atlanır. Alan eklenmedi,
  reddedilecek bir şey yok; onun yerine eski örneklerin yeni birimle okunması sınanır
  (`crates/shared/kcad/tests/all/schemas.rs`).

### Ekran

Pencereler yüzde gösterir ve yüzde alır. Çeviri tek yerdedir (`road_dialog::percent_text` ve `grade_of`):

- dever penceresinin normal eğimleri, kurp oranları ve satırları, çizelge diyagramı;
- geçiş önerisinin rampaları ve tablosu;
- tip kesitin enine eğimleri ve basamak eğimleri;
- menfezin ve platformun eğimi;
- diş kazısının eğimi;
- ön güzergâhın en dik eğimi ve cevabı;
- kavşak bağlamasının enine eğimleri, en büyük eğim değişimi, tablosu ve iletisi.

Alan başlıkları "(%)" yazmaya devam eder. Ekranda yazılan sayı değişmedi: kullanıcı yine −2,5 yazar.

### Bağımsız başvurular

Python başvuruları kendi modellerini mühendisin yazdığı gibi yüzdeyle kurar. Yazarken KentOS'un biçimine
`scripts/fixtures/road_grades.py` ile çevirirler: her değer 100'e bölünür, okuyucuyla aynı IEEE bölmesi.

- **Kopya üzerinde çevirme:** Çeviri kopya üzerinde yapılır. Betiklerin durumları aynı tip kesit sözlüğünü paylaşır;
  yerinde çevirmek bir değeri paylaşan sayısınca bölüyordu (ölçüldü: −2,5 → −2,5·10⁻¹¹⁰).
- **Güncellenen derlemler:** Çekirdek derlemleri (`templates`, `daylight`, `curve-volumes`, `pavements`,
  `superelevation-physics`) ve bütün yol komutlarının ortak durumları bu yoldan yazıldı.

## Ölçüm

- **KCAD:** 1622 dosya; Rust ve Python okuyucuları aynı. Eski şemanın 15 yol örneği önceki baytlarıyla okunur ve yeni
  `road-*.json` ile aynı çizimi verir.
- **Ortak durumlar:** Yol komutlarının ve dışa aktarmaların (glTF, IFC, LandXML) 2724 ortak durumu Rust masaüstünde ve
  Python tipli katmanda geçer. Ret iletileri birimi söyleyecek biçimde güncellendi.
- **Başvuru betikleri:** 42 komut başvurusu ve 5 çekirdek derlemi betiği yazarken çevirir; `--check`'ler geçer. Ana dalda
  da düşen 10 betik (yalnız son basamak geometri farkları ve bayat bir kurulum) bu işle ilgisizdir. Kurpta kübaj derlemi
  önceden betiğin gerisinde kalmıştı: yeniden üretilince basamaklı kesitlerin durumları da girdi.
- **Çekirdek:** `kentos-road` 105 test.
- **Pencereler:** dever, geçiş önerisi, tip kesit, menfez, platform, ön güzergâh ve kavşak bağlama pencerelerinin 49
  testi geçer. Resimler `.run/gui-evidence/` altındadır: menfezde %1, 21,94 m'de 106,589 → 106,370; ön güzergâhta
  %8 sınırla en dik %7,68; kavşak bağında eğim farkı %1,13.

## Açık

- **Arazi analizi:** Eğim sınıfları ve ızgara-üçgen karşılaştırması yüzdedir (ROAD-NUM-11).
- **`superelevation.json` derlemi:** Çizelgenin değerlerini yüzde sayılarıyla tutar. Çizelge doğrusal olduğu için birim
  sonucu değiştirmez, ama derlemin kendisi birim söylemez.
- **Boykesit paftasının eğim yazısı:** Bugün iki ondalıklı yazılıyor (`road.profiles.draw`'un `%{:.2}`'si). KGM 2019
  şartnamesi üç ondalık ister. Bu iş sıradaki madde olarak plana yazıldı.
- **LandXML ile dever alışverişi:** Henüz yok. Geldiğinde `FullSuperelev` yüzdesi okurken 100'e bölünecek, yazarken
  100 ile çarpılacak.
