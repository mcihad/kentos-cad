# 3035 — Yangın kaçışı: kaçış yolları, uzaklıklar, kişi yükü ve çıkışlar

Tarih: 2026-11-09. Durum: kabul (M35, iki bölüm; ADV-23; plan S23).

## Sorun

- **Kaçış yolu ölçülmüyordu.** Mevzuat grubu (ADR 3033, 3034) PAİY'nin piyes, merdiven ve kapı ölçülerini okuyordu;
  Binaların Yangından Korunması Hakkında Yönetmelik (BYKHY) hiç okunmuyordu.
  - Mimar en uzak noktadan merdivene uzaklığı cetvelle ölçüyor, kişi yükünü elle hesaplıyordu.
  - Çıkış sayısını ve merdiven sayısını da gözle denetliyordu.
- **Plan M35'in kabulü:** "Yangın kaçışı | Kaçış rotası ve çıkış genişliği". ADV-23'ün 6. kuralı: "40 cm içeriden rota,
  uzaklıklar, kullanıcı yükünden çıkış genişliği".

## Araştırma (9 Kasım 2026)

### Türkiye mevzuatı

**Binaların Yangından Korunması Hakkında Yönetmelik** (mevzuat.gov.tr `MevzuatMetin/21.5.200712937.pdf`):
- BKK 2007/12937, RG 19.12.2007/26735.
- Değişiklikler: 2009/15316 (RG 9.9.2009/27344), 2015/7401 (RG 9.7.2015/29411); son değişiklik RG 1.7.2025/32943.
- Metnin sonundaki çizelge yürürlük tarihlerini verir. Alıntılar `.run/gui-evidence/mimari-m35/m35-kaynaklar.png`.

Maddeler:
- **Tanımlar:**
  - 4/u: kaçış uzaklığı, bir mekânda durulabilen en uzak noktadan en yakın kat çıkışına yürüme yolunun uzunluğudur.
  - 4/i: çıkmaz koridor mesafesi, koridorun en uzak noktasından bir çıkışa ya da iki yönde kaçış imkânı olan noktaya
    kadardır.
  - 4/bbb: yapı yüksekliği bodrum dahil bütün katların toplam yüksekliğidir.
  - 4/ccc: yüksek bina, bina yüksekliği 21,50 m'den ya da yapı yüksekliği 30,50 m'den fazla olandır.
- **32 (uzaklık):**
  - 32/2: kapasite 50 cm'lik birim genişlikle hesaplanır.
  - 32/3–4: kaçış uzaklığı Ek-5/B'yi aşamaz.
  - 32/6: en uzak nokta duvarlardan 40 cm önde alınır.
- **33 (genişlik):**
  - 33/1 (2015): toplam çıkış genişliği ≥ kişi ÷ birim genişlik kişisi × 0,50 m.
  - 33/1, kişiye göre her kaçış yolu: 50–500 kişide 100 cm, 501–2000'de 150 cm, 2001 ve üstünde 200 cm. Koridor
    olarak da kullanılan kaçış yolu 110 cm. Hiçbiri 80 cm'den dar olamaz.
  - 33/2: yüksek binada kaçış yolları ve merdivenler en az 120 cm.
  - 33/5-a: merdivende küpeştenin 80 mm'si temiz genişliğe sayılır.
  - 33/6-b: bir kullanıcı başkalarının odalarından geçmeden çıkışa ulaşır.
- **39/2 (2009, çıkış sayısı):** 50 kişiyi aşan her mekânda 2 çıkış; 500'ü aşanda 3, 1000'i aşanda 4.
- **48 (konutlar, 2009):**
  - 48/1: bodrum dahil 4 katı geçmeyen konutta ve tek evde kaçış mesafesi aranmaz.
  - 48/2: konut içinde herhangi bir noktadan konut kapısına en çok 20 m; yağmurlamalı konutta 30 m.
  - 48/4: daire dışındaki uzaklık daire kapısından ölçülür.
  - 48/5, yapı yüksekliğine göre:
    - 21,50 m'nin altında korunumsuz normal merdiven yeter.
    - 21,50–30,50 m: en az 2 merdiven, biri korunumlu, her daireden ikisine ulaşılır.
    - 30,50–51,50 m: ikisi de korunumlu, birinde yangın güvenlik holü ya da basınçlandırma.
    - 51,50 m üstü: hollü ve basınçlandırılmış en az 2 kaçış merdiveni.
- **Ek-5/A (kişi yükü, m²/kişi):**
  - Konferans salonu, lokanta, bekleme, sinema, düğün salonu: 1,5.
  - Dans salonu ve bar: oturulan kısım 1, ayakta 0,5. Sergi alanı, stüdyo: 1,5. Terminal: 3.
  - Derslik: 1,5. Resepsiyon, atrium, spor tesisi: 3. Mağaza, galeri, atölye, muayenehane: 5.
  - Ofis, öğrenci yatak odası, fabrika, mutfak: 10. Hastane ve otel yatak odası, laboratuvar: 20.
  - Depo ve makine dairesi: 30. Otopark: 30.
  - İlk dört satır net alanla, öbürleri brüt alanla hesaplanır. Konut satırı yoktur.
- **Ek-5/B (2009, 2015), metindeki sütun sırası:**
  - Tek yön ve iki yön en çok uzaklık, her biri yağmurlamasız ve yağmurlamalı.
  - Birim genişlik kişisi: dışarı çıkış kapısı, diğer kapılar ve koridor kapıları, kaçış merdivenleri, rampalar ve
    koridorlar.
  - Çıkmaz koridor, yağmurlamasız ve yağmurlamalı.
  - Örnek satırlar: büro 15/30, 45/75, 100/80/60/100, 15/20; apartman 15/30, 30/75, 50/40/30/50, 15/20.
  - W4 notlarındaki sütun belirsizliği (`⚠`) ve 48/5 eşiklerinin hangi yükseklik olduğu metinden çözüldü: yapı
    yüksekliği.

### Başka uygulamalar (yalnız görünüş)

- Revit "Path of Travel" ([Autodesk yardım](https://help.autodesk.com/cloudhelp/2021/ENU/Revit-Analyze/files/GUID-1393ED58-3F0D-4BBA-AF89-87FBB9EACEB5.htm)):
  iki nokta arasında engellerden kaçan en kısa yol; uzunluğu çizelgeye girer.
- Revit engellerden 0,30 m uzak durur (vücut genişliği). Bu bir Revit değeridir; Türk mevzuatında karşılığı yoktur ve
  kullanılmadı.

## Karar

- **Kaçış yolları** (`kentos_domain::escape`, `Document`'ı okur, değiştirmez):
  - Mahaller boşluk ve dış alan olmayan mahallerdir. Yürüme, net sınırlarının (iç yüzlerin) içinde kalır.
  - Mahalden mahale geçilen yerler:
    - kapılar ve eşiksiz boşluklar (ADR 0722 §5): yan noktaları duvar yüzünden 10 cm öndedir;
    - mahal ayırıcılar: üzerlerinde metrede bir geçiş noktası alınır.
  - **Kat çıkışı:**
    - Merdiven adlı ya da kullanımlı ortak mahal (Merdiven evi, Yangın merdiveni; merdiven altı ve deposu değil):
      ona girmek çıkışa varmaktır.
    - Bir mahalde duran açık merdivenin ayağı (kendi katında) ve başı (üst katta).
    - ±0,00 ve altındaki katta dışarı açılan kapı. Üst katın dışarı açılan kapısı (balkon) çıkış değildir.
  - **Kim nereden geçer (33/6-b):** bir mahalin yolu kendi bölümünün ve ortak mahallerin içinden geçer, başka bir
    bölümün mahallerinden geçmez.
  - **En uzak nokta (32/6):** her dış köşe açıortay boyunca iki duvardan 0,40 m uzaklaşana kadar içeri alınır. Yürüyüşe
    göre en uzak olan köşe kalır. 0,80 m'den dar mahalde köşe noktası yoktur; mahalin etiket noktası alınır.
  - **İç köşeler** de 0,40 m'den dönülür. Yönetmelik yalnız başlangıç noktasının uzaklığını söyler; aynı ölçü
    seçildi.
  - Yol, görünür noktalar arasında en kısa yoldur (Dijkstra).
  - **Tek yön:** en yakın çıkışa giden yolun, en yakın öbür çıkışa giden yolla paylaştığı kısımdır. Tek çıkışlı katta
    yolun tamamıdır.
- **Kurallar** (`fire_audit.rs`, Mevzuat grubunda, kodları `bykhy_…`; madde ve RG `rule`'da):
  - **Kaçış uzaklığı (32/4, Ek-5/B):** konut olmayan mahal için.
    - Tek çıkışta bütün yol tek yön sınırını aşamaz.
    - İki çıkışta tek yönlü kısım tek yön sınırını, bütün yol iki yön sınırını aşamaz.
    - Koridorun kendi yolu çıkmaz koridor ölçüsüdür (4/i): tek yönlü kısım 15 m (yağmurlamalı binada 20 m).
  - **Kullanım** (Ek-5/B satırı):
    - Önce bölümün niteliğinden (Büro, Dükkân, Otel …), sonra mahalin adından okunur.
    - Bilinmeyen konut dışı bölüm büro sayılır.
    - Konut binasının ortak mahalleri ayrıca ölçülmez; 48/4 daire kapısından ölçer.
  - **Konut:**
    - Daire içinde en uzak noktadan daire kapısına 20 m (yağmurlamalı binada 30 m; 48/2).
    - Daire kapısından kat çıkışına Ek-5/B'nin apartman satırı (48/4).
    - Bodrum dahil 4 kata kadar konutta (mahali olan bodrumlar sayılır) ve tek bağımsız bölümlü konutta aranmaz (48/1).
  - **Ulaşmayan mahal:** katta çıkış varken ulaşamayan mahal, daire kapısına ulaşamayan daire mahali
    (`bykhy_escape_no_route`).
  - **Kişi yükü (Ek-5/A):**
    - Mahal adından okunur (Ofis, Toplantı salonu, Derslik, Depo …).
    - İlk dört satır net alanla hesaplanır; öbürleri mahalin eksen (duvar ortası) alanıyla, brüt alan yerine.
    - Adı bir satıra uymayan mahal kullanımının satırını alır: büro 10, mağaza 5 m²/kişi.
    - Islak hacim, koridor, merdiven ve şaft sayılmaz (31/6).
    - Kişi sayısı yukarı yuvarlanır.
  - **Çıkış genişliği (33/1):**
    - Toplam genişlik = katın kişisi ÷ birim genişlik kişisi × 0,50 m. Katın kullanımı birden çoksa en sıkı olanı
      alınır (31/5).
    - Zemin katta dışarı açılan kapıların net genişlikleri toplanır (dış kapı sütunu). Üst katlarda aşağı inen,
      bodrumda yukarı çıkan merdivenlerin kolları toplanır (merdiven sütunu).
    - Her kaçış yolu 50/501/2001 kişide 1,00/1,50/2,00 m olmalıdır.
    - Yüksek binada konut dairesi dışındaki her merdiven 1,20 m (33/2).
  - **Çıkış sayısı (39/2):** 50 kişiyi aşan mahalde kapı ve boşluk sayısı en az 2 olmalıdır. Kat 500'ü aşarsa 3, 1000'i
    aşarsa 4 çıkış ister.
  - **Koridor (33/1, 33/2):** kaçış yolu olan ortak ya da konut dışı koridor 1,10 m (yüksek binada 1,20 m). Ortak
    koridorda PAİY 29/3 (1,20 m) bulgusu da çıkabilir; iki yönetmelik ayrı ayrı yazılır.
  - **Konut merdivenleri (48/5):** yapı yüksekliği 21,50/30,50/51,50 m'yi aşınca en az 2 kaçış merdiveni istenir.
    - Sayılan merdivenler zemin kattaki, daire dışındaki merdivenlerdir.
    - Korunumluluk, yangın güvenlik holü ve basınçlandırma çizimden okunmaz; iletide istenenler yazılır.
  - **Yükseklikler:**
    - Yapı yüksekliği bütün katların yüksekliklerinin toplamıdır. Yüksekliği verilmemiş katta üstteki katın kotu,
      o da yoksa 3 m alınır.
    - Bina yüksekliği ±0,00 ve üstündeki katların toplamıdır ("kot aldığı noktadan saçağa" yaklaşığı).
- **Yağmurlama (sprinkler) sistemi** (`ArchSettings.sprinklers`; KCAD 214):
  - Proje ayarları › Mimari › **Yangın güvenliği** › "Yağmurlama (sprinkler) sistemi var".
  - Ek-5/B'nin ve 48/2'nin yağmurlamalı sütunlarını okutur. İleti "yağmurlamalı" der.
- `dwelling_nature` tek yerdedir (`escape.rs`); Mevzuat grubunun konut kuralı da onu okur. Türkçe sadeleştirme â, î, û
  harflerini de çevirir ("Dükkân").
- **Yeni kodlar** (uyarı): `bykhy_escape_distance_over`, `bykhy_escape_no_route`, `bykhy_exit_width_below`,
  `bykhy_exit_count_below`, `bykhy_corridor_narrow`, `bykhy_stair_count_below`.

### §2 Kaçış yollarının planda çizimi (M35 2. bölüm)

- **Kaçış yolları** (`tool.escapeRoutes`; Mimari › Proje ▾, Parsel'in yanında; KACIS, KAÇIŞYOLU, YANGINKACISI):
  - Etkin katın yollarını çizer ve biter (`escape_tool.rs`, `fire_marks::draw`). Tek geri alma adımıdır.
  - Yollar Mevzuat grubunun ölçtüğü yollardır (`fire_audit::walks`): bulgu ile çizim aynı ölçümden gelir.
- **Her yol:**
  - Çizgisi en uzak noktadan çıkışa uzanır; sonunda ok başı vardır (iki çizgi, son kola 25° açıyla).
  - Başında, yolun gittiği yöne ve mahalin içine doğru yazısı durur: ölçtüğü ve sınırı ("tek yön 28,07 m > 15,00 m",
    "daire içi 12,40 m ≤ 20,00 m", "çıkmaz koridor 26,80 m > 15,00 m").
  - Yazı ilk sürümde ortalıydı; komşu odaların köşedeki yazıları üst üste biniyordu.
  - Sınırı aşan kırmızı (#d32f2f), uyan yeşil (#2e7d32) çizilir. Yönetmeliğin aramadığı yol (48/1) gri çizilir ve
    yalnız uzunluğunu yazar.
- **Ölçülen yollar:**
  - Konut olmayan mahallerin yolu ve koridorların çıkmaz ucu.
  - Her dairenin kat kat en uzak yolu (48/2) ve her daire kapısının yolu (48/4).
  - Kapısız mahalin yolu yoktur; Denetim "ulaşmıyor" der.
- **Yeniden çizim:** her işaret `Kaçış` özniteliğini taşır. Yeniden çalışınca bütün katların eski işaretleri silinir,
  yalnız etkin katınki çizilir; plan bir anda tek katın yollarını gösterir. İşaretler çizgi ve yazıdır, çizimi izlemez:
  değişiklikten sonra araç yeniden çalıştırılır.
- **Yeni ikon `escapeRoutes`:** kapısı açık bir oda, içinden kesikli yol ve kapıdan çıkan ok.

## Ölçüm

- **Test** (`escape::tests`):
  - 4 × 3 m mahalin dört köşe noktası duvarlardan 0,40 m içeridedir; 0,70 m'lik mahalde köşe noktası yoktur.
  - L biçimli mahalde iç köşenin noktası köşeden 0,40 m uzaktadır. Uzak uçlar birbirini görmez, ikisi de bu noktayı
    görür.
- **Test** (`fire_audit::tests`): Büro, Dükkân, Daire, Düğün salonu kullanımları; Ofis 10, Toplantı salonu 1,5 net,
  Derslik 1,5, Depo 30 m²/kişi; Koridor, WC ve Merdiven evi sayılmaz; Ek-5/B büro ve apartman satırları.
- **Test** (`fire_escape_tests::an_office_storey_s_distances_people_exits_and_corridor_are_said`): iki katlı büro.
  1\. kat: 1,00 m koridor, batı ucunda tek Merdiven evi, dört ofis, 63 kişilik toplantı salonu (tek kapı), kapısız
  Ofis 5, 0,90 m merdiven.
  - Ofis 2, 3, 4 ve Toplantı salonu: 16,07, 22,07, 28,07 ve 18,10 m > 15 m (büro, tek yön). Ofis 1 bulgusuzdur.
  - "Koridor çıkmaz koridor uzunluğu 26.80 m > 15.00 m (büro) — BYKHY 4/i, Ek-5/B (9.7.2015)".
  - "Ofis 5 kat çıkışına ulaşmıyor — BYKHY 33/6-b".
  - "“1. kat” kişi yükü 76: her kaçış yolu en az 1.00 m; merdiven 0.90 m" ve "en az 2 çıkış gerekir, 1 var".
  - "Toplantı salonu kişi yükü 63: en az 2 çıkış gerekir, 1 var"; "Koridor net eni 1.00 m < 1.10 m (kaçış yolu)".
  - Yağmurlamayla ofisler sınırın içinde kalır (30 m); yalnız koridor kalır: "26.80 m > 20.00 m (büro, yağmurlamalı)".
- **Test** (`fire_escape_tests::a_tall_dwelling_building_s_walks_and_stairs_are_said`): 8 × 3 m konut (yapı yüksekliği
  24 m). Zemin katta üç mahalli "1" dairesi, 1,15 m ortak koridor, doğu ucunda tek merdiven.
  - "“1” bölümünde en uzak noktadan daire kapısına 22.00 m > 20.00 m — BYKHY 48/2 (9.9.2009)".
  - "“1” bölümünün kapısından kat çıkışına 26.15 m > 15.00 m (apartman, tek yön) — BYKHY 48/4, Ek-5/B (9.7.2015)".
    Merdivenin yanındaki "2" dairesinin kapısı bulgusuzdur.
  - "Yapı yüksekliği 24.00 m: en az 2 merdiven, biri korunumlu ve her daireden ikisine de ulaşılan gerekir; zemin
    katta 1 ortak merdiven var — BYKHY 48/5-b (9.9.2009)"; "Koridor net eni 1.15 m < 1.20 m (yüksek bina) — BYKHY 33/2".
  - Yağmurlamayla yalnız koridor ve merdiven sayısı kalır.
- **Test** (M33'ün küçük konutu): bölüm Büro olunca 0,85 m koridor yalnız BYKHY 33/1'e takılır.
- **Gerçek pencere** (1920 × 1080; testlerin sakladığı çizimler):
  - `fixtures/interaction/real/yangin-kacisi.txt` (8 katlı konut):
    - Önce (M34 ikilisi): Mevzuat'ta 10 bulgu, yangın yok. Sonra: 14 bulgu, son dördü BYKHY.
    - Yangın güvenliği anahtarı açılıp Kaydet, Denetle: 48/2 ve 48/4 kalkar.
  - `fixtures/interaction/real/yangin-buro.txt` (büro katı): Mevzuat'ta 20 satır, son 10'u BYKHY.
  - Resimler: `.run/gui-evidence/mimari-m35/m35-konut-once-sonra.png`, `m35-buro-once-sonra.png`, `m35-ayar.png`,
    `m35-kaynaklar.png`.
- **KCAD:** `sprinklers.kcad`; kırık `sprinklers-in-schema-213`; 245 geçerli dosya, 1711 dosya Python okuyucusuyla
  tutarlı.
- **Test, §2** (`fire_escape_tests::the_escape_routes_tool_draws_the_storey_s_walks_with_their_limits`): 8 katlı
  konutun zemin katında `tool.escapeRoutes`.
  - "daire içi 22,00 m > 20,00 m" ve "tek yön 26,15 m > 15,00 m" kırmızı yazılır; "2" dairesinin içi yeşildir.
  - Her yazı için bir çizgi ve bir ok başı çizilir.
  - Yeniden çalışınca aynı sayıda işaret kalır. İlk geri alma ilk çizimi geri getirir, ikincisi hepsini kaldırır.
  - `fire_marks::tests`: yazı biçimleri.
- **Gerçek pencere, §2** (1920 × 1080):
  - `fixtures/interaction/real/kacis-yollari.txt`: Menülerde ara "kaçış yolları", Enter. Konutta 4 yol çizildi,
    2'si kırmızı; durum çubuğu "4 kaçış yolu çizildi (“Zemin kat”); 2 tanesi sınırı aşıyor (kırmızı)" der.
  - `fixtures/interaction/real/kacis-yollari-buro.txt`: Kat › 1. kat, sonra araç. Büroda 6 yol çizildi, 5'i kırmızı.
  - Resim: `.run/gui-evidence/mimari-m35/m35-kacis-yollari.png`, ikon `m35-simge.png`.
- **Fikstürler:** 19 280 komut vakası değişmedi.

## Sonraki adımlar

- Kaçış yolu işaretlerinin kat bilgisi taşıması ve çizimi izlemesi; yangın kaçış paftası (ADV-24).
- Kapının açılış yönü (47/2: 50 kişiyi aşan mekânda kaçış yönüne), korunumlu merdiven ve hol nesneleri, yangın dayanımı.
- İki çıkış arasındaki uzaklık (39/3–4: köşegenin 1/2'si, 1/3'ü) ve kaçış merdivenlerinin birbirine alternatif
  konumu (40).
- Asma kat ve galeri; zemin kat dükkânının tek çıkışı (32/8); Ek-14 yüksek tehlikeli yerler.
