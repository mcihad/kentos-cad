# 3041 — Kiriş, lento, kolon–duvar birleşimi

Tarih: 2026-11-10. Durum: kabul (M41; ADV-18).
- §1: kiriş nesnesi; planda kesikli ve ince çizilir, 1:100'de numarası, 1:50'den numarası ve ölçüsü yazılır; altındaki
  duvar kirişin altında biter; `.kcad` 222.
- §2: kolonun malzemesi; duvar kolonun yüzünde kesilir, kolon kendi malzemesindeki duvar katmanıyla tek kesittir
  (perde + kolon kesintisiz, tuğla kolonun yüzünde biter); etiketin `{size}`'ı; `.kcad` 223.
- §3 (lento ve hatıl) sonraki bölümde.

## Sorun

- **Kiriş yoktu.** Mimari ruhsat planında kirişler kesit düzleminin üstünde kalır ve kesikli çizilir. KentOS'ta kiriş
  nesnesi olmadığından kullanıcı kirişi çizgilerle elle çiziyordu:
  - çizgiler kirişin ölçüsünü, katını, numarasını bilmiyordu;
  - altındaki duvar döşemenin altına kadar sayılıyordu: duvar kirişin içinden geçiyor, metraj kirişin döşemeden sarkan
    kısmını duvar yüzü olarak da sayıyordu.
- Plan maddesi ADV-18'in ilk kabulü "kiriş planda kesikli görünür" bu yüzden tutmuyordu.
- **Duvar kolona birleşmiyordu (§2).** ADR 0771 §1'in sınırı: "duvar kolona birleşmez; kolon kendi çizilir".
  - Kolon duvarın üstüne koyu, yarı saydam bir dolgu olarak biniyordu. Perdenin ve tuğlanın çizgileri, aralarındaki
    gönye çizgisi ve tuğlanın taraması kolonun içinden geçiyordu.
  - Betonarme perde ile betonarme kolon iki ayrı kesit gibi görünüyordu: 1:50'de perde gri, kolon koyu; 1:20'de
    kolonun SN46'sı perdenin grisinin üstüne biniyordu. ADV-18'in ikinci kabulü "perde + kolon + tuğla düğümünde tarama
    süreklidir" tutmuyordu.
  - Kolonun malzemesi yoktu: her kolon 1:20'de betonarme (SN46) taranıyordu, çelik kolon da.

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

### Kesitte tarama (§2)

- **TS EN ISO 128-3** (2020; ISO 128-50:2001'in içeriği onun 7.2'sinde): aynı parçanın kesit kısımları aynı taramayla,
  bitişik farklı parçalar farklı açı ya da aralıkla taranır. Standardın metni ücretli; kural EPO'nun PCT kılavuzunun
  A-V 7.3.2'sinde (PCT Kural 11.13(b)) aynı sözlerle özetlenir: "All parts of the same item use the same hatching,
  while neighbouring different elements use different angles."
- **Dijital Hazırlama Yönetmeliği** EK-4 4.23: tarama TS EN ISO 128-3'e göre.
- **Mimarlar Odası standardı** §14.5.3 i (ADR 3014): taşıyıcı elemanlar (kolon, perde, duvar, pano) ayrı çizim
  tekniğiyle gösterilir, içleri koyulaştırılır.
- Sonuç: birlikte dökülen betonarme kolon ve perde tek parçadır, tek tarama ve aralarında çizgi yok; tuğla duvar ayrı
  parçadır, kolonun yüzünde biter ve sınır çizgisi kalır.

### Başka uygulamalar (yalnız görünüş için)

- Revit: kiriş plan görünümünde kesit düzleminin üstündeyse "Beyond/Hidden" çizgisiyle çizilir; etiketi kiriş boyunca,
  okunacak yöne döner.
- Archicad: kiriş kat üstüne göre konur, planda üstü kesikli.
- Revit (§2): birleşen elemanlar aynı malzemedeyse aralarında çizgi kalmaz, farklı malzemede kalır; mimari kolon
  birleştiği duvarın görünüşünü alır (Autodesk forumları, *About Wall Joins* yardımı).

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

### 2. Kolon malzemesi ve kolon–duvar düğümü

- **Model** (`ColumnFields.material`, `.kcad` şema 223): kolonun malzemesi; verilmezse Betonarme ve yazılmaz.
  - Malzemeli kolonu olan dosya 223 olur; olmayan eski şemasıyla bayt bayt aynıdır. Şema 222 malzemeyi
    `unknown_field` ile reddeder. Çok yeni şema örneği 224'e kaydı.
  - Boş malzeme `invalid_column` ile reddedilir.
  - `cad.columns.create` malzeme alır. `cad.arch.edit`'in `material` alanı kolona da yazar; null Betonarme'ye
    döndürür. Öznitelikler'de kolonun Malzeme satırı.
- **Duvar kolonda kesilir** (`geom::column_join::cut`): aynı kattaki kolonlar duvarı keser (ikisi de katsızsa da).
  - Duvarın çizgileri, uç kapakları, girintileri ve kapama çizgileri kolonun içinde kalan parçalarını yitirir. Kolonun
    kenarı üstünde kalan nokta içeride sayılmaz: yüzü kolonun kenarına denk gelen duvar o yüzünü korur.
  - Katmanların bantlarından ve duvarın gövdesinden kolonun ayak izi çıkar.
  - Kesme, duvarlar birbirine birleştikten sonra yapılır; köşedeki gönye kolonun içinde kalır.
  - Yuvarlak kolon 64 kenarlı halkasıyla keser. 1:50'de kenarlar çemberden kâğıtta yüzde bir milimetre sapar.
  - Kolonun her malzemesi keser: kolon taşıyıcıdır, duvar ona dayanır.
- **Tek kesit** (`geom::column_join::joined`): kolon, kendi malzemesindeki bir duvar katmanının bandıyla alan
  paylaşıyorsa o katmanla tek kesittir.
  - Malzemeler katlanarak karşılaştırılır: kenar boşlukları atılır, Türkçe harfler sadeleşir, küçük harfe çevrilir;
    "BETONARME" ile "Betonarme" aynıdır.
  - Kolon o katmanın görünüşünü alır. Şematikte duvarın dolgusunu, Uygulama ve Ayrıntılı'da katmanın taramasını (katman
    o ayrıntıda çiziliyorsa) alır. Tarama, duvardaki gibi duvarın ilk kenarının yönüne döner.
  - Kolonun dış çizgisi o katmanın içinde kalan yerde çizilmez. Böylece perde ile kolon arasındaki çizgi kalkar, iki
    gri bir olur.
  - Birden çok uygun katmanda, duvarların çizimdeki sırasıyla ilki görünüşü verir; çizgi hepsinin içinde gizlenir.
  - Farklı malzemede kolon kendi görünüşündedir ve aradaki çizgi kalır.
- **Kolonun kendi görünüşü:** 1:20'deki SN46 yalnız betonarme kolonda (adı "Betonarme" ile başlayan malzeme). Başka
  malzemede dolgu bayrağına göre dolu ya da boş.
- **Ekran, pafta, PDF ve DXF aynı çizer.** Depo kolonların ayak izlerini ve duvarların kesilmemiş çizimlerini tutar.
  Bir kolon konunca, taşınınca ya da silinince o yerdeki duvarlar yeniden birleşir ve kesilir, kolonların görünüşü
  yeniden bulunur. DXF yazıcısı (`walls::plain_joined`) aynı işlevleri çağırır.
  - DXF'te mimari nesne parçasının rengi artık yazılıyor. Önce yazılmıyordu ve birleşik kolonun grisi kayboluyordu
    (test yakaladı).
- **Etiketin `{size}`'ı** kolonda planın yazdığı ölçüdür: "30/50", yuvarlakta "Ø40" (`ColumnFields::size_text`).

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
- Kolon–duvar düğümü, §2:
  - Geometri (`geom::column_join`):
    - 20 cm tuğla duvar 40 × 40 kolonda biter: yüzleri x 3,8'de durur, kolondaki ucu gider, bandı 3,8 × 0,2.
    - Ortadaki kolon duvarı ikiye böler.
    - 25 cm perde ile kolon tek kesittir: SOLID, dış çizginin 1,6 m'sinden 1,35'i çizilir.
    - Çelik kolon birleşmez.
  - Bağımsız başvuru (`scripts/fixtures/column_join_cases.py` → `fixtures/walls/v1/column-joins.json`, kesirli): 15
    durumun çizgileri, uç kapakları, bant alanları ve kolon görünüşleri Rust'ta aynı. Durumlar:
    - kolonda biten ve ortadan bölünen duvar;
    - yüzü kolonun kenarında kalan ve kolona yalnız değen duvar;
    - dışarıdaki, başka kattaki ve aynı kattaki kolon;
    - sıvalı üç katmanlı duvar;
    - perde ile kolon, büyük harfli malzeme, çelik kolon;
    - iki yandan birleşen kolon (1,1 m çizilir) ve perdeden derin kolon (1,75 m);
    - iki kolonun üçe böldüğü duvar.
  - Depo (`a_shear_wall_a_column_and_a_brick_wall_meet_as_one_section`), batıdan perde, kuzeyden tuğla, köşede kolon:
    - kolon 1:50'de #A6A6A6, 1:100'de #3C3C3C; dış çizgisinin 1,35 m'si çizilir;
    - tuğlanın yüzleri y 0,2'de durur; iki duvar da kolonun içine çizmez;
    - çelik kolon kendi görünüşünde, dış çizgisi 1,6 m;
    - taşınınca perde ucuna kadar döner.
  - DXF (`a_column_cuts_the_wall_it_stands_in_and_joins_its_layer_of_one_material`): perde 3,8'de durur; kolonun taraması
    #A6A6A6, dış çizgisi 1,35 m; çelik kolon kendi yolundan.
  - Sözleşme ve etiket: malzeme yoksa Betonarme, boşu reddedilir, "30/50", "Ø27.5"; etiket `{tag} {size}` "S1 30/50",
    "S2 Ø40".
  - Ortak durumlar: kolon oluşturmada "çelik kolon" ve "boş malzemeli kolon reddedilir", Değiştir'de kolonun malzemesi ve
    null'u; bütün durum dosyaları Rust'ta bağımsız başvuruyla aynı.
  - KCAD: `column-material` örneği ve şema 222'deki bozuk dosyası; 1729 dosya bağımsız okuyucuyla tutarlı; Rust 254
    geçerli örnek.
  - Gerçek pencere (iz `fixtures/interaction/real/kolon-duvar.txt`, 1920×1080): batıdan Betonarme perde 25, kuzeyden ve
    doğudan İç duvar 15 (sıva + tuğla + sıva), köşede 40 × 40 dolu kolon S1; Uygulama ve Ayrıntılı ayrıntıda.
    - ÖNCE (M39'un ikilisi): kolon duvarların üstüne koyu bir dolgu olarak biner. Perdenin yüzleri, gönye çizgisi ve
      tuğlanın çizgileri kolonun içinden geçer; 1:20'de kolonun SN46'sı perdenin üstünde.
    - SONRA: perde ile kolon tek gri, aralarında çizgi yok; tuğla duvarlar kolonun yüzünde biter, çizgileri ve
      taramaları kolona girmez. Ayrıntılı'da sıva çizgileri de kolonda durur.
    - Kolon seçilince Öznitelikler'de Malzeme "Betonarme".
  - Testler: 18 paket 9863 geçti (masaüstünün tek kırmızısı yine `each_program_docks_its_own_panels`); MCP 167 geçti,
    iki kırmızısı §1'de yazılan yol işinden. Clippy kolon kodunda çekirdeğin yasakladığı `min`/`max`/`powi`'yi yakaladı;
    yerlerine `js_min`/`js_max`/`js_hypot`.
  - Disk, takım koşarken doldu (`No space left on device`). Yeniden üretilebilen artımlı derleme klasörü (122 GB)
    silindi ve kalan paketler yeniden koşuldu.

## Sınırlar

- Kirişlerin metrajı (boy, betonarme hacmi) yok: lentoyla birlikte §3'te.
- TBDY 7.4.1.1 ölçü denetimi yok: kirişin çerçeve kirişi mi ikincil kiriş mi olduğu çizimden okunmuyor ((d)
  istisnaları); ölçüler taşıyıcı sistem projesinindir.
- Mahalin iç yüksekliği kirişe bakmıyor: sarkan kiriş mahalin tavanını yer yer alçaltır ama PAİY 28'in iç yüksekliği
  döşemeden tavan kaplamasına ölçülür.
- Kiriş kolona ya da öbür kirişe birleşmez; kesişen bantlar üst üste kesikli çizilir. Eğri kiriş yok.
- Kirişin yazısı mahalin yazısıyla çakışabilir; çakışmayı kullanıcı kirişi ya da mahalin yazısını taşıyarak çözer.
- §2: duvarın metrajı (net yüz, hacim) kolonun kapladığı parçayı düşmez; duvarın kendi geometrisinden ölçülür.
- §2: kolonun malzemesi bir metindir. Proje malzemesine (`materialId`, ADR 0785) bağlanmaz, eşleşme ada göredir.
- §2: kolon katmanı gizlense de duvar kolonda kesik kalır, çünkü kolon modelde durur. Tasarım seçeneği gösterilmeyen
  duvarlar (ADR 0788 §4) kesilmez.
- §2: kesilen eğri duvar çizgisi, kesik parçalarında kısa düz parçalarla çizilir.
- §2: kolon–duvar birleşiminde kullanıcı kararı yok: duvar her zaman kolonda biter.
