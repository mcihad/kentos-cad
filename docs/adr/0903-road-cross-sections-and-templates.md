# ADR 0903 — Enkesit ve tip kesit: kesit hattı takımı, arazi ve platform kesiti, şevlerin araziye bağlanması

Tarih: 2026-11-02. plan/road.md Y5 (ROAD-SEC-01…08, 13; ROAD-COR-01…04, 10). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D:** kesit hatları (*sample lines*) bir aksın üstünde km aralığıyla (başlangıç, bitiş, aralık; isteğe bağlı
  geometri noktaları), tek km'de, koridor km'lerinden, çizilmiş çoklu çizgiden ya da noktalardan kurulur; şerit genişliği
  (*swath width*) sol ve sağ için ayrı, metre ya da başka bir aksa kenetli.
  [By Station Range](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-EEECE0FB-8D39-465B-9D52-8794D3648150.htm),
  [To Work With Sample Lines](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-C5A6AA74-1974-4D97-8758-7C0694D63E2E.htm),
  [Creating Sample Lines](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Tutorials/files/GUID-408653EF-997A-46BD-96A2-40E832FDBBCA.htm).
- **OpenRoads Designer:** tip kesit (*template*) bileşenlerden kurulur; uç koşulu (*end condition*) açık bir bileşendir,
  bir yüzeyi, kotu ya da aksı hedefler; yarma ve dolgu şevi uç koşuludur. Dış hendekte önce şevin araziyle buluştuğu nokta
  (*daylight*) aranır, hendek oradan içe kurulur.
  [Template Library (CTDOT)](https://portal.ct.gov/dot/engineering-applications/ctdot-connect-dde-ebook-volume-003---openroads-designer-roadway-modeling/section-3---the-template-library),
  [Template Components](https://portal.ct.gov/dot/engineering-applications/ctdot-connect-dde-ebook-volume-003---openroads-designer-roadway-modeling/3_3-template-components),
  [Create Template Overview](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/406410/GUID-E29336A5-38BC-4E0C-9B70-F093204AF641.html),
  [Brow ditch](https://communities.bentley.com/products/road___site_design/m/road_and_site_design_gallery/271042).
- **Netcad Netpro:** tip kesit platform, şev ve duvar tanımlarından oluşur, enkesitler modelden alınır, yarma ve dolgu
  alanları çizelgelenir ([NETCAD NETPRO](https://www.academia.edu/4871392/NETCAD_NETPRO),
  [Yol Projesi (Netpro)](https://publish.mersin.edu.tr/index.php/geoengbooks/article/download/722/620)). Türk uygulamasında
  yarma şevi zemin cinsine, dolgu şevi dolgu yüksekliğine göre seçilir (3/2, 1/1 gibi;
  [OMÜ ders notu](https://avys.omu.edu.tr/storage/app/public/sibel.uzun/137389/en_kesit_boy_kesit2023.pdf)). Değerler
  KGM'nin standardından gelir; KentOS öntanımlı sayı uydurmaz, kullanıcı yazar (ADR 0902 §7).

## Karar

1. **Kesit hattı (ROAD-SEC-01, 02).** Bir kesit hattı aksın bir km'sinde (aksın kendi km'si) aksa diktir; soldan sağa
   `left` ve `right` metre (şerit genişliği). Öteleme sağa artı, sola eksidir. Hat yatay düzlemde düz çizgidir; arazi kesiti
   ADR 0900'ün `Surface::section`'ıyla (üçgen kenarlarında kesin, yüzeyin dışı boşluk).
2. **Kesit hattı takımı ve yöntemleri (ROAD-SEC-03).** Bir takım bir aksın, isteğe bağlı boykesitinin, tip kesitinin ve
   yüzeyinin kesitleridir; km'leri üç yoldan: `range` (başlangıç, bitiş, her `every` metrelik yuvarlak km — km eşitliğiyle
   okunur — ve isteğe bağlı `geometry`: kurp noktaları ve PVI'ler), `stations` (yazılan km'ler), `lines` (çizimdeki çizgi
   ve çoklu çizgilerin aksı ilk kestiği yer). Km'ler sıralı ve tektir (1 mm içindekiler bir).
3. **Tip kesit (ROAD-SEC-04, ROAD-COR-01…03).** Sol ve sağ yan ayrı (`both` ikisini aynı verir). Bir yanın bileşenleri
   eksenden dışa: şerit (`lane`) ve banket (`shoulder`), her biri genişlik ve enine eğim (yüzde, dışa doğru düşen eksi).
   Uç koşulu: yarma şevi `cut` ve dolgu şevi `fill` (bir düşeye yatay, 1,5 = 3/2), isteğe bağlı hendek (`ditch`: iç şev
   `fore`, derinlik, taban genişliği) yalnız yarmada.
4. **Platform ve şevlerin araziye bağlanması (ROAD-SEC-05, ROAD-COR-04, 10).** Eksenin kotu boykesitin kırmızı kotudur
   (boykesit yoksa kesit platformsuzdur). Bileşenler eksenden dışa sırayla kotları belirler; son bileşenin dış ucu menteşedir.
   Menteşede arazi menteşenin üstündeyse yarma: hendek varsa iç şevle derinliğe iner, tabandan geçer, yarma şevi yukarı;
   yoksa yarma şevi menteşeden yukarı. Değilse dolgu: dolgu şevi menteşeden aşağı. Şevin araziyle ilk buluştuğu yer
   daylight'tır, kesin (iki doğrunun kesişimi). Şerit içinde buluşma yoksa yan `open` kalır ve uyarılır; kot uydurulmaz.
5. **Yarma ve dolgu alanı (ROAD-SEC-07).** İki daylight arasında proje çizgisi ile arazi çizgisinin farkının integrali:
   proje üstte dolgu, altta yarma; kırılma yerleri ve kesişimler kesin, alanlar kapalı biçimde (parçalı doğrusal). Bir yan
   açıksa alan o yanın şerit ucuna kadar sayılır ve uyarı verilir. Kübaj (alanlardan hacim) Y8'dedir.
6. **Saklama.** Tip kesitler `settings.road.templates`, kesit hattı takımları `settings.road.sections`, geri alma
   geçmişinde. Kesitlerin kendisi saklanmaz; aks, boykesit, tip kesit ya da yüzey değişince yeniden çıkar. Takım kurulduğu
   aksın revizyonunu tutar (ADR 0902 §6 gibi).
7. **Görünüş (ROAD-SEC-06, 08, 13).** Enkesit penceresi takımın kesitlerini tek tek gösterir: arazi siyah, platform ve şev
   kırmızı, eksen kotu, ötelemeler ve kotlar, yarma ve dolgu alanı; çizime yazılan enkesit sayfası ızgarayla dizilir.

## Aşamalar

Y5.1 kesit hattı ve arazi kesiti çekirdeği; Y5.2 tip kesit, daylight ve alanlar çekirdeği; Y5.3 komutlar
(`road.templates.*`, `road.sections.*`), `.kcad`, Python, MCP; Y5.4 Enkesit penceresi ve çizime enkesit.

## Sonra

Dever ile enine eğimin değişmesi (Y6), koridor ve yüzeyleri (Y7), sıyırma ve kübaj (Y8), duvar ve bordür bileşenleri,
şevde basamak (*benching*), çok katmanlı üstyapı.
