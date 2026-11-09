# 3032 — Vaziyet planı: parsel, bahçe mesafeleri, oturum ve ±0,00

Tarih: 2026-11-09. Durum: kabul (M32 1. bölüm, ADV-22; plan S22).

## Sorun

- **Parsel yalnız bir çizgiydi.** Mimar parseli çiziyor, bahçe mesafelerini (çekme mesafeleri) elle ölçüp yaklaşma
  çizgisini elle ofsetliyordu; kat sayısı değişince çizgiyi yeniden çiziyordu.
- **Binanın parselde oturumu denetlenmiyordu.** Bina yaklaşma çizgisini aştığında bunu yalnız göz söylüyordu.
- **±0,00'ın mutlak kotu (röper) yazılmıyordu.**
- **Plan M32'nin kabulü:** "Vaziyet planı | Parsel, bahçe mesafeleri, oturum, ±0,00". ADV-22: "parsel (Harita işinden ya
  da koordinat tablosundan); bahçe mesafeleri ve kat adedine göre artış (PAİY 23); bina oturumu, ±0,00 ve röper,
  kuzey; ağaç hesabı".

## Araştırma (9 Kasım 2026)

- **Planlı Alanlar İmar Yönetmeliği 23** (mevzuat.gov.tr; kesitler `.run/gui-evidence/mimari-m32/m32-once-sonra.png`):
  - 23/1-a: ön bahçe, yol kenarına rastlayan ve kamusal alanlara komşu bahçe mesafeleri en az 5,00 m.
  - 23/1-b, c: yan ve arka bahçe en az 3,00 m.
  - 23/1-ç: yan ve arka bahçe, zemin üstünde kalan bodrumlar dahil dörtten fazla katlı binalarda 4 katın üzerindeki
    her kat için 0,50 m artırılır; park alanına komşu cephede uygulanmaz.
  - 23/1-d: bu kat adedi, o cephede zeminin altında kalan katlar hariç katların toplam yüksekliğinin üçe bölünmesiyle
    bulunur; 2,50 m'yi aşan artık bir kattır.
  - 23/1-g, ğ: 60,50 m'den yüksek binalarda en az 15 m ve kat başına 0,50 m (bu bölümde yok).
  - 23/6: konut ve benzeri parsellerde binanın zemine oturduğu alanın dışında kalan alanın her 30,00 m²'si için bir ağaç.
  - 4 (şşşş) yapı yaklaşma mesafesi; 57/3: çıkma izdüşümleri ve yaklaşma mesafeleri projede gösterilir.
- **W4 §3 ve tablo 1** (`docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`): vaziyet planında parsel, yapı
  yaklaşma çizgileri ve bahçe mesafeleri, oturum alanı, ±0,00 ve röper, ağaçlar.
- **Revit "Property Line"**
  ([Autodesk yardım](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Model/files/GUID-44A38002-3CD5-42A1-B80F-0DDDF398DFE1.htm)):
  - Parsel sınırı çizimle ya da tapu verisiyle (yön ve uzunluk tablosu) girilir; çekme mesafesini ve yapılabilir
    alanı göstermek için kullanılır.
  - Çekme çizgisini kendisi üretmez; uygulamada elle ofsetlenip ölçülendirilir. Genel uygulama, çekme çizgisini parsel
    kenarlarına paralel kesikli çizgiyle ve mesafesiyle göstermektir.

## Karar

- **Vaziyet planı** (`ArchSettings.site`, `SitePlan`; KCAD 212):
  - **Parsel:** çizimdeki bir kapalı çizginin ya da alanın kalıcı kimliği. Parsel bir kopya değil, o çizginin
    kendisidir; kullanıcı çizgiyi düzenleyince her şey onu izler.
  - **Kenarlar:** sınırın sırasıyla, her kenarın türü (`SiteEdgeKind`: Yan bahçe, Ön bahçe, Arka bahçe, Kamusal alan)
    ve planın kendi mesafesi. Listenin ötesi yan bahçedir.
  - **±0,00 kotu** (`datum`) ve sınırın çizgisi (`envelope`, kalıcı kimlik). Boş vaziyet planı yazılmaz.
- **Kenarın mesafesi:** planın verdiği; yoksa türününki. Yolda ve kamusal alanda 5 m; yanda ve arkada 3 m + 0,50 m ×
  (kat − 4). Kamusal alana komşu kenarda artış uygulanmaz (park, 23/1-ç).
- **Kat adedi (23/1-d):** kotu ±0,00 ve üstündeki katların yükseklikleri toplanıp üçe bölünür, 2,50 m'yi aşan artık bir
  kattır. Katın kendi yüksekliği yoksa bir üstteki katın kotu, o da yoksa 3 m alınır.
- **Oturum:** zemin katın TAKS'a sayılan mahallerinin konturları birlikte (M30'un taban alanı). Kenarın "binadaki"
  mesafesi oturumun ona en yakın noktasıdır; keserse 0.
- **Yapı yaklaşma sınırı:** parselden her kenara kendi mesafesinden yakın noktalar çıkarılır (kenar boyunca uçları
  yuvarlak bir şerit). Köşede iki mesafe de korunur, iç bükey parselde de doğrudur.
  - Çizgi kesikli, kırmızı (#d32f2f) bir kapalı çoklu çizgidir; etiketi "Yapı yaklaşma sınırı".
  - Belge onu adım içinde yeniden yazar: parsel çizgisi, bir mahal ya da duvar değişince (mahal duvarını aynı adımda
    izler) veya kat ayarı değişince. Proje ayarları kaydedilince ve geri alma, yineleme sonrasında da (ayrı adım
    olmadan) yeniler.
  - Birden çok parçaya bölünürse en büyük parçanın dış halkası çizilir.
- **Ağaç (23/6):** parselin oturum dışındaki her tam 30 m²'si için bir ağaç.
- **Parsel ve bahçe mesafeleri** (`tool.siteParcel`; Mimari › Proje ▾; VAZIYET, BAHCEMESAFESI, YAKLASMASINIRI):
  - Kapalı çizgiye ya da alana kenarından tıklanınca parsel olur.
  - Kenara tıklandıkça türü değişir: Yan → Ön → Arka → Kamusal → Yan.
  - Her kenar türünü ve mesafesini yazar; dar kenar kırmızıdır. Sınır kesikli gösterilir.
  - Kart parsel alanını, kat adedini, her türün mesafesini ve binadaki mesafeyi, ağaç sayısını söyler.
  - Enter ayarlara yazar, sınırı çizer (ya da var olanı korur) ve dar kenarları söyler.
  - İmar durumu olmayan projede parselin çizili alanı parsel alanı olur. Parseli olan projede araç onu türleriyle açar.
  - Kimlik `siteParcel`: KentOS Harita'nın parsel çizme aracı `tool.parcel` ayrıdır.
- **Görünür sonuçlar:**
  - Durum çubuğu rozeti dar kenarda kırmızıdır, ipucu kenarları sayar.
  - Öznitelikler › İmar durumu "Bahçe mesafeleri: uygun / dar: …" der.
  - Emsal ve TAKS tablosu (ADR 3030) her kenar türü için gerekli, binadaki ve kalan mesafeyi (eksi kırmızı), ağaç
    sayısını ve ±0,00 kotunu yazar. Mesafeler metre ve iki basamakla yazılır ("5.00 metre"). Parsel çizgisi
    değişince tablo da yenilenir.
  - Proje ayarları › Mimari › İmar durumu'na "±0,00 kotu (m)" eklendi.
- **Yeni ikon `siteParcel`:** parsel, kesikli çekme çizgisi ve içinde bina.

## Ölçüm

- **Test** (`site_plan_tests::a_parcel_s_garden_distances_draw_the_setback_line_and_follow_the_storeys`): 15 × 6 m,
  dört 3 m katlı bina; çevresinde 30 × 18 m parsel (güneyden 4, doğudan 10, kuzeyden 8, batıdan 5 m).
  - Araçla güney kenar bir tıkla ön, kuzey iki tıkla arka yapıldı. Kenarlar Ön, Yan, Arka (sondaki yan varsayılan).
  - Mesaj: "Parsel 540,00 m², kat adedi 4, ağaç 15; yapı yaklaşma sınırı çizildi. Bahçe mesafesi dar: 1. kenar (Ön
    bahçe) 4,00 m < 5,00 m".
  - Sınır: x −2…22, y 1…11, kesikli. Binadaki mesafeler 4, 10, 8, 5.
  - Tablo: Ön 5.00 / 4.00 / −1.00 (kırmızı), Yan 3.00 / 5.00 / 2.00, Arka 3.00 / 8.00 / 5.00, ağaç 15. Rozet kırmızı.
  - Beşinci kat eklenince kat adedi 5, yan ve arka 3,50 m. Sınır kendiliğinden x −1,5…21,5, y 1…10,5 oldu.
- **Gerçek pencere** (`fixtures/interaction/real/vaziyet-plani.txt`, 1440 × 900; testin sakladığı `parsel.kcad`):
  - Arama "parsel ve bahçe", parsele ve kenarlara tıklama: etiketler, kırmızı ön bahçe, kesikli sınır, kart.
  - Enter: kesikli kırmızı "Yapı yaklaşma sınırı", durum çubuğunda kırmızı "!".
  - Proje ayarlarında emsal 1,5, TAKS 0,4, ±0,00 1285,40. Emsal ve TAKS tablosu bahçe satırlarıyla.
- **KCAD:** `site-plan.kcad`; kırık `site-plan-in-schema-211` ve `site-plan-negative-setback`. 243 geçerli dosya;
  1707 dosya Python okuyucusuyla tutarlı.

## Sonraki adımlar

- Çizime basılan vaziyet planı ögeleri: binadan parsel kenarlarına ölçüler, ±0,00 ve röper işareti, kuzey oku, ağaç
  simgeleri (M32 2. bölüm).
- Kenar kenar kat adedi (23/1-d "o cephede", eğimli arazi), 60,50 m üstü binalar (23/1-g, ğ), birden çok bina (24).
- Parseli Harita işinden ya da tapu koordinat tablosundan almak.
- Açık çıkmaların bahçe mesafesine taşma kuralları (çıkma izdüşümü, 57/3).
