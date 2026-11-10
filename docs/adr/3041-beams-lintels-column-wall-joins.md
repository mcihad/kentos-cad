# 3041 — Kiriş, lento, kolon–duvar birleşimi

Tarih: 2026-11-10. Durum: kabul (M41; ADV-18).
- §1: kiriş nesnesi; planda kesikli ve ince çizilir, 1:100'de numarası, 1:50'den numarası ve ölçüsü yazılır; altındaki
  duvar kirişin altında biter; `.kcad` 222.
- §2 (kolon malzemesi, kolon–duvar düğümü, kolon etiketinde ölçü) ve §3 (lento ve hatıl) sonraki bölümlerde.

## Sorun

- **Kiriş yoktu.** Mimari ruhsat planında kirişler kesit düzleminin üstünde kalır ve kesikli çizilir. KentOS'ta kiriş
  nesnesi olmadığından kullanıcı kirişi çizgilerle elle çiziyordu:
  - çizgiler kirişin ölçüsünü, katını, numarasını bilmiyordu;
  - altındaki duvar döşemenin altına kadar sayılıyordu: duvar kirişin içinden geçiyor, metraj kirişin döşemeden sarkan
    kısmını duvar yüzü olarak da sayıyordu.
- Plan maddesi ADV-18'in ilk kabulü "kiriş planda kesikli görünür" bu yüzden tutmuyordu.

## Araştırma (10 Kasım 2026)

### Türkiye mevzuatı

- **Türkiye Bina Deprem Yönetmeliği 2018** (Resmî Gazete 18.03.2018/30364 mükerrer):
  - 7.4.1.1 (kolonlarla çerçeve kuran ya da perdeye kendi düzleminde bağlanan kirişler):
    - (a) kiriş gövde genişliği en az 250 mm; kiriş yüksekliği ile kolonun kirişe dik genişliğinin toplamını geçmez;
    - (b) kiriş yüksekliği döşeme kalınlığının 3 katından ve 300 mm'den az olmaz, gövde genişliğinin 3,5 katını geçmez;
      sağlamayan eleman çerçeve kirişi sayılmaz;
    - (d) kolona mafsallı bağlı kiriş, bağ kirişi ve düğüm dışında saplanan ikincil kiriş için (a)–(c) zorunlu değil.
  - 7.8.1 aynı koşulları süneklik düzeyi sınırlı kirişlere de uygular.
  - Kiriş yüksekliği döşeme kalınlığıyla karşılaştırıldığı için **döşemeyi içine alır**: kirişin yüksekliği üstünden
    altına bütün derinliğidir.
- **Dijital Hazırlama Yönetmeliği** (Resmî Gazete 05.08.2026/33331):
  - EK-4 4.22: çizgi tipleri TS EN ISO 128-2'ye göre. Kesit düzleminin üstünde kalan eleman kesikli ince çizgiyle
    (02.1) gösterilir; döşemenin üst kattan görünen kenarı da böyle çizilir (ADR 3012).
  - EK-2 Tablo 2.3'ün öğe kodları: Kolon KOLN, Lento LENT, Hatıl HATL, Taşıyıcı TASI; **Kiriş öğesi yok.**
  - EK-5 Tablo 5.1: IfcBeam "Kiriş", kısaltma KRS.
- **Mimarlar Odası çizim standardı** (kolon için; ADR 3011): ön projede (1:100) kolon adı, uygulama projesinde (1:50)
  adı ve ölçüsü yazılır.

### Başka uygulamalar (yalnız görünüş için)

- Revit: kiriş plan görünümünde kesit düzleminin üstündeyse "Beyond/Hidden" çizgisiyle çizilir; etiketi kiriş boyunca,
  okunacak yöne döner.
- Archicad: kiriş kat üstüne göre konur, planda üstü kesikli.

## Karar

### 1. Kiriş

- **Model** (`BeamFields`, `.kcad` şema 222):
  - ekseni (en az 2, en çok 1000 nokta, düz parçalar);
  - gövde genişliği (0 < g ≤ 3 m) ve yüksekliği (0 < y ≤ 5 m; döşeme içinde, TBDY 7.4.1.1 b);
  - hiza: ortası, sol yüzü ya da sağ yüzü eksende (duvarınki gibi);
  - kot farkı (±10 m), numara ("K101"), malzeme (yoksa Betonarme), kat ve tadilat durumu.
  - Kirişli dosya 222 olur; kirişsiz dosya eski şemasıyla bayt bayt aynıdır. Şema 221 kirişi `unknown_kind` ile
    reddeder. Çok yeni şema örneği 223'e kaydı. Bağımsız Python okuyucusu ve yazıcısı aynı kuralla.
- **Tepesi ve altı:** kirişin tepesi katının üstündeki döşemenin taşıyıcı (betonarme) üstüdür; döşeme yoksa katın
  tepesi; kot farkı onu kaydırır. Altı: tepesi − yüksekliği. Döşemeyle birlikte dökülen kiriş döşemenin betonarmesinden
  aşağı sarkar.
- **Duvar kirişin altında biter** (`Document::wall_height`; ADR 3040 §1'in güncellemesi): üstünde kiriş olan duvarın
  tepesi, döşemenin altı ile kirişin altından alçak olanıdır. "Üstündeki kiriş": bandı duvarın yolunun ortasını içeren,
  katın içinde kalan kiriş. Öznitelikler'in notu "Kirişin altına kadar". Metraj, değer sütunları ve iç görünüşler aynı
  kuralı okur; duvarın net yüzü kirişin sarkan kısmı × duvarın boyu kadar azalır.
- **Plan çizimi** (`geom::beam`):
  - Bandı (ekseninden hizasına göre kaydırılmış, köşeleri gönye) kesikli ve ince çizilir: kalın çizginin yarısı, üst
    kattaki döşemenin kenarıyla aynı kalem (ADR 3012). Çizim grubunun payı da uygulanır (1:100 planında çeyrek).
  - Yazısı kolonunki gibi ölçeğe göre: 1:200'de yok, 1:100'de numarası, 1:50'den numarası ve ölçüsü ("K101 25/50";
    ölçü santimetre, gövde/yükseklik). En uzun parçasının ortasında, aşağıdan ya da sağdan okunur.
  - Yazı **bandın üstüne değil yanına** (okuma yönünde üstüne, 1 mm boşlukla) konur. Bandın üstünde yazı hem kesikli
    çizgileri hem altındaki duvarın yüzlerini kesiyordu (gerçek pencerede görüldü).
  - Ekran, pafta, PDF ve DXF aynı parçaları çizer; yakalama ve seçme bandın köşelerinden ve kenarlarından.
- **Araç: Mimari › Yerleştir › Kiriş** ("KR"):
  - Eksenin noktalarına tıklanır, Enter yazar; araç açık kalır.
  - Ayarları: Genişlik (E), Yükseklik (Y), Hiza (H), Kot farkı (F, döşeme aracındaki gibi), Numara (N). Varsayılan **25/50**: TBDY 7.4.1.1'in
    en küçük gövdesi 25 cm; 50 cm, 15 cm'lik döşemenin 3 katından ve 30 cm'den büyük, 25 cm'nin 3,5 katından küçük.
  - Numara verilince her kirişten sonra ilerler (K101 → K102), kolon aracı gibi; verilmeyen numara uydurulmaz.
  - Katman anahtarları açıkken kendi katmanına yazar: **MM-TASI-KIRIS**. EK-2'de Kiriş öğesi olmadığından taşıyıcı
    öğesinin (TASI) altına alt bilgi olarak; kalem kategorisi "Kolon ve kirişler".
- **Komutlar:** `cad.beams.create` (doğrula, planla, yaz; iki kiriş tek adımda) ve `cad.arch.edit`'in kiriş alanları
  (genişlik, yükseklik, hiza, kot farkı, numara, malzeme, kat, tadilat durumu). Hatalı kiriş `invalid_beam`.
- **Öznitelikler:** Numara, Genişlik, Yükseklik, Hiza, Kat, Tadilat durumu, Kot farkı, Üst kot ve Alt kot (okunur),
  Malzeme, Uzunluk. Tür "Kiriş".
- **Kirişi gören öbür yerler:**
  - Seçim süzgecine **Kiriş** türü (Mimari nesneler'de Kolon'un altında; kaydedilmiş süzgeçlerin bitleri değişmesin diye
    listenin sonuna eklendi). Süzgeç açıkken kiriş önceden hiç seçilemiyordu.
  - Kat ver kirişe kat verir; bir katı silmeye çalışınca o katı anan kirişler de sayılır ("2 kiriş anıyor").
    Denetim › Mimari katsız ya da projede olmayan bir kattaki kirişi söyler ("Kirişin katı yok", `arch_level_missing`).
  - Kat ver'in reddi kat alan türlerin bir kısmını sayıyordu; şimdi hepsini sayar: "kat yalnız duvar, mahal, aks,
    kolon, merdiven, rampa ve mahal ayırıcıya verilir" → "kat yalnız duvar, mahal, döşeme, aks, kolon, kiriş, merdiven,
    rampa, mahal ayırıcı ve bloğa verilir".
  - Nokta yapımının paralel uzaklığı (bir noktadan nesnenin düz kenarına paralel) kirişin bandının kenarlarını okur.
  - GeoJSON, KML, GPX, sunucunun izdüşümü ve CBS türevi (`mimari_kiris`) kirişin bandını alan olarak yazar.
  - DXF kirişi kesikli çizgiler ve yazı olarak yazar. Sunucu `beam` türünü kabul eder (migration 0038).
  - Hızlı paket (pack) türü 39; düzenle, taşı, döndür, aynala, esnet, böl ve tutamaçlar diğer mimari nesneler gibi.

## Doğrulama

- Geometri (`geom::beam`): ortalı bant ±0,125; sol yüzü eksende bant 0 ile −0,25 arası; L kirişte dış köşe gönyeli.
  Kesikli ve yarım kalem; 1:200'de yazı yok, 1:100'de "K101", 1:50'de "K101 25/50"; dikey kiriş 90°, yazısı bandın
  0,125 + 1 mm solunda; sağdan sola çizilen kiriş 0°, bandın üstünde.
- Depo (`a_beam_is_drawn_dashed_by_the_view_and_snapped_to`): üç ölçekte kesikli parça ve yazılar; bandın köşesine
  yakalanır; kenarından seçilir; 1:100 çizim grubunda kalemi 0,25.
- Domain (`a_wall_under_a_beam_stops_at_its_underside`): 15 cm döşemenin altında 2,85 m olan duvar, 25/50 kirişin
  altında 2,50 m; kirişin bandı dışındaki duvar 2,85 m'de kalır. Döşemesiz katta kiriş katın tepesinden asılır, kot farkı
  −5 cm iken duvar 2,45 m.
- Araç (`beam_tool`): iki tık ve Enter K101'i 25/50, sol hizalı MM-TASI-KIRIS'e yazdı; sonraki K102 30/60; numara
  silinince numarasız. Sıfır genişlik uyarıyla reddedildi.
- Ortak durumlar: `cad.beams.create` 28 durum, `cad.arch.edit` 7 kiriş durumu; bütün durum dosyaları Rust'ta bağımsız
  başvuruyla aynı.
- KCAD: `beams` örneği ve şema 221'deki bozuk dosyası (`unknown_kind`); 1727 dosya bağımsız okuyucuyla tutarlı.
- Gerçek pencere (iz `fixtures/interaction/real/kiris.txt`, 1920×1080, iki katlı bina, 1. katın döşemesi ara kat
  dizilişli):
  - Mimari › Kiriş: araç paneli Katman MM-TASI-KIRIS, 0.250, 0.500, Orta, 0.000. Numara K101 yazıldı.
  - Salon ile Merdiven holü arasındaki duvarın üstüne K101, öbür bölmenin üstüne K102, Salon'un içinden K103 çizildi;
    durum çubuğu "Kiriş çizildi: K103 25/50.".
  - Kirişler kesikli; yazıları kirişin yanında "K101 25/50" (dikey kirişlerde döndürülmüş).
  - K103 seçildi: Numara K103, Üst kot 2.930, Alt kot 2.430, Uzunluk 4.950, Katman MM-TASI-KIRIS.
  - K101'in altındaki duvar: önce "2.780 · Üst döşemenin altına kadar", sonra "2.430 · Kirişin altına kadar".
  - Bulunan ve düzeltilen hatalar: (1) depo kirişi çizim listesinde tanımıyordu, yazılan kiriş ekranda görünmüyordu;
    (2) yazının açısı radyanla verilmişti (yazı açısı derecedir), dikey kirişin yazısı 1,57° dönük, yatay çıktı;
    (3) Öznitelikler'de tür "Nesne" yazıyordu.
  - ÖNCE (M39'un ikilisi): şeritte Kiriş yok.
- Testler: 18 paket 9855 geçti, masaüstü dahil (tek kırmızı işten önce de kırmızı olan
  `module_surface_tests::each_program_docks_its_own_panels`). MCP 167 geçti; iki kırmızı bu işten önceki yol işinden
  (`a_stakeout_is_sent_to_an_instrument`: 7 Kasım'dan beri aplikasyon dosyası kot sistemi ister, örnek dosyada yok;
  `a_surface_is_made_asked_and_refused_outside_a_road_project`: 6 Kasım'dan beri arazi yüzeyi Harita projesine açık).
- Testlerin yakaladıkları: Değiştir'in kataloğu `invalid_beam`'i anmıyordu; Kat ver ve nokta yapımı kirişi alıyor ama
  katalogları kirişi saymıyordu; kiriş aracının kot farkı K harfindeydi, döşeme aracınınki F (aynı ada aynı harf
  kuralı, ADR 3003); kiriş çiziminde çekirdeğin belirlenimci matematiği yerine Rust'ın kendi sin/cos/atan2'si vardı.

## Sınırlar

- Kirişlerin metrajı (boy, betonarme hacmi) yok: lentoyla birlikte §3'te.
- TBDY 7.4.1.1 ölçü denetimi yok: kirişin çerçeve kirişi mi ikincil kiriş mi olduğu çizimden okunmuyor ((d)
  istisnaları); ölçüler taşıyıcı sistem projesinindir.
- Mahalin iç yüksekliği kirişe bakmıyor: sarkan kiriş mahalin tavanını yer yer alçaltır ama PAİY 28'in iç yüksekliği
  döşemeden tavan kaplamasına ölçülür.
- Kiriş kolona ya da öbür kirişe birleşmez; kesişen bantlar üst üste kesikli çizilir. Eğri kiriş yok.
- Kirişin yazısı mahalin yazısıyla çakışabilir; çakışmayı kullanıcı kirişi ya da mahalin yazısını taşıyarak çözer.
