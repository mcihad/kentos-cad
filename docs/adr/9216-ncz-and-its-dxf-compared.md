# ADR 9216: İmar planının NCZ'si ile DXF'i, katman katman

- **Durum:** kabul edildi, bulgular (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-18.
- **İstek:** sahibin 23 Kasım isteği: “NCZ ve DWG/DXF içe aktarımını kusursuz hâle getir”.
- **Örnek:** USB'deki `SIVAS_UIP_2025_06_HAZIRAN`. NCZ'si 72 MB, DXF'i 657 MB. DXF'i Netcad aynı plandan aktarmış.
- **Yöntem:** iki okuyucunun sonucu bir sondayla karşılaştırıldı (geçici, commit'lenmedi). Karşılaştırılanlar:
  - katman başına nesne türleri, çizgi uzunluğu (yay payı dahil) ve kapsam;
  - yazıların kümesi, KADASTRO'da konumlarıyla;
  - parsel sınırlarının 1 m'lik hücrelerde kapladığı yer.

## §1 Sayılar

| | NCZ | DXF |
|---|---|---|
| Okuma (hata ayıklama derlemesi) | 2,8 s | 70 s |
| Nesne | 453 534 | 3 199 462 |
| Katman | 137 | 136 |
| Blok | — | 56 (Netcad sembolleri) |
| Yazı | 111 010 | 119 712 |

DXF'in nesne çokluğu Netcad'in aktarımından geliyor:

- KADASTRO'nun 46 841 parsel poligonu 2,77 milyon tek çizgiye bölünmüş.
- Plan alanlarının her poligonunun yanına bir düz dolgu (SOLID HATCH) eklenmiş.
- Sembollü çizgi tipleri ve desenli dolgular çizgilere, dairelere ve blok yerleştirmelerine patlatılmış.

## §2 Aynı olanlar

- **Parsel sınırları:** ikisi de yaklaşık 5,6 milyon hücre kaplıyor. NCZ'nin yalnız 2 hücresinin, DXF'in yalnız 26
  hücresinin (tek bir yerde, yaklaşık 26 m) öbüründe 1 m yakınında karşılığı yok.
  - Uzunluk farkı (7 786 km'ye karşı 5 308 km) komşu parsellerin ortak kenarından geliyor. NCZ her parselin kenarını
    ayrı sayıyor; tekrarlar atılınca 5 377 km.
- **KADASTRO yazıları:** NCZ'nin 52 018 yazısı DXF'in 2,5 boylu 52 018 yazısıyla bire bir aynı. İçinde durdukları
  parselin etiketine göre sınıflanınca bütün sınıflar eşit: parsel no 40 266, ada no 5 688, tam etiket 2 851,
  başka 2 840, parsel dışında 373.
- **Plan alanları** (PL_*): poligon sayıları aynı. Çizgi uzunlukları ±%0,06 içinde (PL_PARK, PL_PASIF_YESIL,
  SNR_YAPIYAK, REFUJ…).
- **Katman renkleri:**
  - NCZ'nin renk tablosu gerçek RGB veriyor (konut #8C541A); DXF'teki ACI yaklaşımı #994C00.
  - NCZ'de siyah olan 40 katman (HAT_*, SM_*, TESCILLI_*, KA_*) KentOS'ta temaya uyan mürekkep rengiyle çiziliyor.
    Netcad bunları DXF'e #4C0000 yazmış.

## §3 NCZ'de olup DXF'te olmayanlar: okuyucumuz Netcad'in aktarımından fazlasını alıyor

- **Yapılaşma koşulu sembolleri:** SM_YOL, SM_YERLESIM ve SM_YAPILASMA katmanlarında 15 600 daire, 15 600 çizgi ve
  40 211 yazı var. Yazılar emsal (0.40, 2.00…), nizam (A, B, BL), “Yençok=12.50 m” ve “00” gibi.
  - DXF'te SM_YOL hiç yok; SM_YAPILASMA'da yalnız 1 843 yazı kalmış.
- **@ROL katmanı** (26 nesne) DXF'te yok. DXF'in kendi eklediği @PINDEX (pafta indeksi, 400 yazı) NCZ'de yok.
- **0,0'daki yazılar:** DXF'te 148 yazı 0,0'ın birkaç metre yanında (grup 10 = −1,1875, −1,5; hizalama yok). NCZ'de
  böyle yazı yok; DXF aktarımının kusuru.
- NCZ'de PL_TRAFO_ALANI'nda 0,0'da duran iki poligon var (RM-03'ün kapsam kuralı onu dışarıda tutuyor).

## §4 Netcad'in gösterip KentOS'un göstermedikleri: açık işler

1. **Parsel numaraları.**
   - Durum: NCZ'deki 46 841 parsel poligonunun 46 826'sı “Etiket” niteliği taşıyor (`1588/10`, `103/TH4`). Okuyucu
     bunu nitelik olarak saklıyor, yazı üretmiyor.
   - Netcad'in davranışı: her parselin içine etiketin parsel kısmını 3,0 boyunda yazıyor. DXF'te bu türden 46 605 yazı
     var; 42 551'i parselin kendi numarası (“10”).
   - Sonuç: 44 430 parselin numarası NCZ'nin ayrı yazısıyla zaten görünüyor; Netcad bunlarda numarayı iki kez
     çiziyor. **2 396 parsel KentOS'ta numarasız**, örneğin `103/TH4`, `4943/1`, `136/104`.
   - İlk ölçüm 4 149 demişti. O ölçüm bir yazıyı yalnız ilk bulduğu poligona saymıştı; üst üste parseller sayıyı
     şişirdi. Düzeltme ADR 9217'de.
2. **Semboller.**
   - Durum: SM_SEMBOL'ün 6 664 noktası (ve SM_SIMGE, PL_PARK, TESCILLI_BINA'nın noktaları) yalnız kodunu taşıyor
     (`"Sembol": "S84"`). Sembolün çizimi NCZ'de yok, Netcad'in kütüphanesinde.
   - DXF'te bunlar Netcad'in 56 bloğuna yerleştirme: “84” bloğu 71 nesne.
   - Sonuç: KentOS'ta nokta olarak çiziliyorlar.
3. **Sembollü çizgi tipleri ve desenli dolgular.**
   - Koruma bantları: KST_DEMIRYOLU_KORU, KST_KARAYOLU_KORUMA. Koruma alanının daire deseni: KA_KORUNMA_ALANI, DXF'te
     4 080 daire.
   - NCZ'deki poligonlar çizgi tipi ya da desen taşımıyor (`look` boş). Sonuç: düz çizgiyle çiziliyorlar.

## §4½ Katmanların stil kaydı (24 Kasım, çözüldü)

**Kayıt:** NCZ'nin `LEX.ST2` bloğu her katman için 256 baytlık bir kayıt tutuyor. Okuyucu bunun yalnız ilk 3 baytını
(renk) okuyordu. Katman adlarıyla eşlenince (renkler tutuyor: PL_KONUT #8C541A, PL_PARK #249C22) şu alanlar
çıktı:

| Bayt | Ne | Örnek |
|---|---|---|
| 0–2 | renk (RGB) | |
| 5 | çizgi tipinin numarası | HAT_IFRAZ 40, HAT_KADEME 41, HAT_DOGALGAZ 19, KADASTRO 35, KST_DEMIRYOLU_KORU 4 |
| 7–14 | çizgi kalınlığı, mm (f64) | KADASTRO 0,3; ADAKENARI 0,5 |
| 16 | dolgu türü | plan alanlarında 5 |
| 17 (19 = 1 iken) | katmanın sembol numarası | aşağıda |
| 28–43 | 16 baytlık bir kimlik (GUID) | desenin ya da sembolün kütüphanedeki kimliği |

**Sembol numarasının anlamı:**

- **Plan alanlarında** alanın lejant kodu: PL_ANAOKULU 75, PL_CAMI 96, PL_TRAFO_ALANI 152, PL_PARK 101. Bunlar SM_SEMBOL
  noktalarının kodları (ADR 9218).
- **Koruma kuşaklarında** kuşağın desen sembolü: karayolu 66, boru hattı 67, su kanalı 68, demiryolu 71, enerji nakil
  74.
- **Sitlerde ve tescilli yapılarda:** 1./2./3. derece arkeolojik sit 35/36/37, tarihi sit 42, tescilli anıt yapı 52,
  tescilli bina 53, sit etkileşim geçiş alanı 55.

**Sonuç:** kayıtlar çizgi tiplerinin ve desenlerin çizimini değil, Netcad'in kütüphanesine numarasını taşıyor; çizim
NCZ'de yok. Sembollerdeki gibi ancak KentOS'un MPYY sembollerine eşlenerek gösterilebilir. Bunun bir görünüm kararı
var (plan alanlarının rengi), sahibe soruldu. Karar ve uygulama: [ADR 9219](9219-ncz-overlays-with-legend-symbols.md)
(bindirmeler MPYY sembolüyle, bindirme alanları yarı saydam; plan alanları Netcad'in renginde).

## §5 Karar

- NCZ okuyucusu bu planda veri kaybetmiyor. Netcad'in kendi DXF aktarımından fazlasını, doğru konumda ve doğru renkle
  alıyor.
- Görünüşteki üç fark (§4) okuyucunun değil sunumun eksiği. Her biri ayrı bir karar ister:
  - parsel etiketinin varsayılan gösterimi;
  - Netcad sembol kodlarının KentOS sembollerine eşlenmesi; çizimleri yönetmeliğin lejantından, Netcad'in
    kütüphanesinden değil;
  - NCZ'nin çizgi tipi ve desen kayıtlarının çözülmesi.
- Bunlar sahibe soruldu. Kararları (24 Kasım):
  - parsel numarası yalnız eksiklere ([ADR 9217](9217-ncz-parcel-numbers-from-labels.md));
  - semboller yönetmelik lejantından KentOS'un kendi çizimiyle ([ADR 9218](9218-ncz-symbols-as-legend-marks.md));
  - NCZ'nin çizgi tipi ve desen kayıtları çözülecek ([ADR 9219](9219-ncz-overlays-with-legend-symbols.md)).

## Uygulama

Kod değişmedi. Sonda (`apps/desktop/src/probe_rm18.rs`, geçici) commit'lenmedi.
