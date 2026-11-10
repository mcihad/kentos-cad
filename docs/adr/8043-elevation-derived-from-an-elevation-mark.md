# ADR 8043: Görünüş çıkar — görünüş işaretinden bina görünüşü, yönüne göre adı, ruhsat takımında görünüş paftası

- **Durum:** kabul edildi (10 Kasım 2026); `plan/architecture-advanced.md` ADV-19'un ikinci teslimi (M43 1. bölüm:
  §1–§6; 2. bölüm: §7–§9).
- **Bağlam belgesi:** ADR 0345 (görünüm işaretleri), ADR 8038 §6 (Türkiye'nin açılım çizgileri), ADR 8042 (Kesit çıkar:
  blok, çekirdek, eskime, ruhsat takımı; bu ADR onun üstüne kurulur).

## Bağlam

Planda görünüş işareti (Mimari › Belgele › İşaret ▾ › Görünüş) M42'den sonra da yalnız bir başvuruydu; Kesit çıkar
görünüş işaretini "kesit işareti değil" diye geri çeviriyordu. Görünüşler elle çiziliyordu. Bu teslim binanın dış
görünüşünü M42'nin kesit çekirdeğiyle, işaretin balonundan çıkarır.

## Araştırma

### Türkiye mevzuatı

- **Planlı Alanlar İmar Yönetmeliği Md. 57(2)(a)(6):** mimari proje kesitlerle birlikte "yeteri sayıda görünüş" içerir.
- **Mimarlar Odası, Mimari Proje Çizim ve Sunuş Standartları (görünüşler):** "Bütün görünüşler çizilir"; "Doğal zemin
  nokta nokta, önerilen zemin devamlı çizgi ile gösterilir ve kotlandırılır"; "Zemin altında kalan yapı kısımlarının
  dış hatları kesik çizgilerle belirtilir"; "Kapı ve pencere görünüşleri … açılan kanatlar işaretlenir"; "Cephe kaplama
  malzemesi ve renkler yazılır"; plan, kesit ve görünüş "aynı ölçekte olması tercih edilmelidir".
- **Açılım çizgileri (ADR 8038 §6, İTÜ MIM 244 notu Şekil 1):** Türkiye'de çizgiler menteşe tarafının köşelerinden
  başlar, kol tarafının ortasında birleşir; Britanya'nın tersi.
- **Dijital proje yönetmeliği ekleri (2026):** EK-2 Tablo 2.12 görünüş dosyasının türü **GR** (ADR 8037); 2.9.3 kesit ve
  görünüşte "Kat" yerine müellifin üç karakterli "Konum"u; EK-4 4.14.3 görünüşe giren yerlerin katmanı malzemeyle,
  derinliği 1–4 ekiyle; Tablo 4.4 blok adı `MM-XXXX-<ad>`.

### Başka programlar (yalnız görünüş için)

- **Revit, dış görünüş:** proje kuzeyine göre dört görünüş "North", "South", "East", "West" adıyla; görünüm düzlemi
  işaretin yerinde, düzlemin kestiği eleman kesilmiş çizilir
  (`.run/gui-evidence/mimari-m43/referans/revit-exterior-elevation.png`).
- **Revit, görünüş işareti:** paftaya konunca işaret paftanın numarasını yazar (`revit-elevation-tag.png`); KentOS bunu
  kesitte yapıyordu (ADR 8042 §6), görünüşte de yapar.

## Karar

### 1. Görünüş, işaretinin bloğudur

- **Görünüş çıkar** bir görünüş işaretinin baktığı cepheyi bir bloğa çizer ve yerleştirir. Bloğun kimliği kesitteki
  gibi işaretin kimliğinden türer; adı `MM-XXXX-GORUNUS` ve yönü (`MM-XXXX-GORUNUSKUZEY`). Yerleşimi
  `MM-XXXX-GORUNUS` katmanına konur (kesitinki `MM-XXXX-KESIT`); takımın plan ve vaziyet pencereleri yalnız kesit ya da
  görünüş yerleşimi taşıyan katmanları dondurduğu için (ADR 8042 §6) görünüş de planlarda görünmez.
- Geri alma adımları "Görünüş çıkar", "Görünüşü yeniden çiz" (kesitinkiler "Kesit çıkar", "Kesiti yeniden çiz").
- Komutlar kesitinkiler: `cad.sections.edit` (derive, refresh) ve `cad.sections.query`. Ayrı komut açılmadı: kontrol,
  eskime, takım ve yayın aynı; ayrı komut, aynı yolun ikinci kopyası olurdu. Detay işareti iki araçta da geri çevrilir
  ("İşaret detay işareti; kesit ve görünüş yalnız kesit ya da görünüş işaretinden çıkarılır.").

### 2. Görünüşün düzlemi

- Düzlem işaretin balonundan geçer, işaretin bakışına diktir; modelin o yöndeki genişliği kadar ve iki yanda 1 m uzar.
  Bakan kişi düzlemin sol ucundadır ve işaretin baktığı yöne bakar. Balonun arkası görünmez.
- Balon binanın dışındadır (olağan); içindeyse düzlemin kestiği kesilmiş çizilir (Revit gibi). Modelde hiçbir katı
  yoksa görünüş çıkarılmaz (`nothing_to_set`).

### 3. Yön ve ad

- Cephenin yüzü işaretin bakışının tersidir; adı planın kuzeyinden en yakın sekiz yönden biri: "Kuzey görünüşü",
  "Kuzeydoğu görünüşü" … Başlık büyük harfle "KUZEY GÖRÜNÜŞÜ", altında "Ölçek 1/50".
- **Planın kuzeyi** çizimin kuzey okundan (Kuzey oku döndürülmüşse onun yönü); ok yoksa çizimin yukarısı. Ok
  döndürülünce işaretlerin adları hemen değişir; çıkarılmış görünüşün başlığı artık yönünü söylemediği için görünüş
  eskir, Yeniden çiz başlığı ve bloğun adını (`MM-XXXX-GORUNUSDOGU`) günceller.
- Pafta Konumu (EK-2 2.9.3) yönün üç harfi: KUZ, KDG, DOG, GDG, GUN, GBT, BAT, KBT.

### 4. Çizim

- Kesitin çekirdeği (`geom::section`) aynen; görünüşte çoğu şey **görünen**dir: malzemesinin katmanında derinlik ekiyle
  (EK-4 4.14.3), en yakın iki dilim 0,25 mm, uzak iki dilim 0,18 mm.
- **Derinlik dilimi en yakın yüzden (2. bölüm):** kesit dilimi düzleminden ölçer; görünüş en yakın yüzünden ve dilimleri
  en az 1 m. 1. bölümde dilim balondan ölçülüyordu: görünüşte cephe hem en yakın hem en uzak şey olduğundan bütün cephe
  4. dilime, en ince kaleme (0,18 mm) düşüyordu; dilim cephenin kendi derinliğine göre ölçülünce de 2,5 cm çıkıntılı kolon
  1. dilimde, pencereler 4. dilimde kalıyordu. Şimdi düz bir cephe bütünüyle `-1`; 1 m geri çekilmiş bir kat `-2`.
- **Kapı ve pencere kendi katısı:** taban alanı olmayan, yalnız duvarın iki yüzünde çizgileri olan katı (çekirdek
  değişikliği: taban alanı olmayan katı önceden atlanıyordu; şimdi çizgileri, önünde bir şey yokken görünür). Çizgiler:
  açıklığın çevresi, kasa genişliği biliniyorsa kasanın iç çevresi, kanatlı açılımda **Türkiye'nin açılım çizgileri**
  (tek kanat: menteşe tarafının iki köşesinden öbür kasanın ortasına; çift kanat: her kanat kendi kasasından orta
  kayda, orta kayıt çizilir). Sürme ve sabit kanatta çizgi yok. Katmanları `MM-KAPI-GENEL-1`, `MM-PENC-GENEL-1` …
- **Zeminin altı kesik:** ±0,00'ın altında kalan görünen kenar (döşemenin, temelin yüzü) kesik çizgiyle; kenar
  ±0,00'ı geçiyorsa orada bölünür.
- Mahal adı yazılmaz (görünüş bir mahalden geçmez). Kotlar, kat yükseklikleri, akslar ve ±0,00'da tesviye zemini
  kesitteki gibi.
- Çizilmeyen ve listelenen kesitteki gibi: çatı, tabii zemin, rampa …

### 5. Eskime ve araç

- Kesitin kuralı: görünüş, yeniden çizildiğinde aynı nesneleri vermiyorsa eski. Bir pencere yükselince, bir duvar
  taşınınca eskir. Durum çubuğunda kırmızı "Kuzey görünüşü eski"; birden çoksa "2 görünüş eski", ikisi birlikteyse
  "1 kesit, 2 görünüş eski". Öznitelikler › Kuzey görünüşü › Tür "Görünüş · plandan çıkarılır (1/50)", Durum ›
  Yeniden çiz. İşareti silinmiş görünüş "kaynağı yok" ("MM-XXXX-GORUNUSKUZEY (işareti yok)"); önceden yalnız
  kesitler listeleniyordu.
- **Araç:** Mimari › Belgele › İşaret ▾ › Görünüş çıkar (komut GORUNUSCIKAR, BIMELEVATION). İşaretin balonuna
  tıklanır (araçtan önce seçili işaret alınır); önizleme imleci izler; tıklanan yer görünüşün sol ucunun ±0,00'ı
  (Enter: çizimin sağına). Çıkarılmış görünüşün işaretine tıklamak onu yeniden çizer.
- Oda iç görünüşünün açıklamasındaki "Kesit/görünüş işareti yalnız başvurudur, çizim üretmez." artık yanlıştı;
  "Binanın kesit ve dış görünüşleri Kesit çıkar ve Görünüş çıkar ile çizilir." oldu.

### 6. Ruhsat takımı

- Proje › Ruhsat takımı her yerleştirilmiş görünüşe **kesitlerden sonra** kendi paftasını kurar (MO sunuş sırası:
  planlar, kesitler, görünüşler). Pafta adı "Kuzey görünüşü", EK-2 türü GR, Konum KUZ, MMC; işaretin balonunun altına
  paftanın numarası yazılır. Takımın notu "Kesit ve görünüşler çizimin ölçeğinde…" der.

### 7. Ek yeri çizilmez (2. bölüm)

- **Kural (çekirdek):** bir prizma yüzünün kenarının iki yanında da (1 µm öteden bakılınca) aynı düzlemde bir prizma yüzü
  varsa orası bir ek yeridir ve çizilmez: sıvalı duvar ile ona hizalı kiriş, kolon ve döşeme tek bir yüz olarak görünür.
  İki katının kaplaması varsa ve farklıysa (taş subasman üstünde sıva) ek yeri çizilir. Kaplaması olmayan katı (kiriş,
  kolon, döşeme) hizalı olduğu yüzün kaplamasını alır. İki ayrı düzlemin köşesi (açılı cephe) ve işaretler (açıklığın
  çizgileri) her zaman çizilir.
- **Kaplama:** duvar katmanının proje malzemesi (yoksa malzemesinin adı); katmansız duvarın kaplaması yok.
- **Dayanak:** Revit aynı malzemeli birleştirilmiş elemanların ortak kenarını çizmez, malzeme farklıysa çizer; Archicad
  aynı yüzeyli eş düzlemli elemanları birleşik gösterir. Türkiye kaynaklarında açık bir kural bulunmadı; sıvalı cephede
  kiriş ve döşeme alnı görünmez.
- **Ölçüm (kuzey cephe):** kiriş altları (+2,50, +5,50) ve döşeme altı (+2,85) boyunca çizgi 0 (önce cephe boyunca);
  30/30 kolonlar 2,5 cm önde olduğu için çiziliyor (8 düşey çizgi); cephenin bütün çizgileri `-1` diliminde.
- Bağımsız başvuruya 19. ve 20. durumlar: duvar–kiriş–döşeme ek yerleri gizli, önde duran kolon görünür; farklı
  kaplamanın ek yeri ve açılı yüzün köşesi görünür. Önceki 18 durumun hiçbiri değişmedi.

### 8. Tümü: bütün görünüşler bir adımda (2. bölüm)

- Görünüş çıkar'ın ilk adımında **Tümü (T)**: çizilmemiş her görünüş işaretinin görünüşü, kuzeyden başlayıp saat
  yönünde (KUZ, KDG, DOG …), her biri yer verilmemiş bir çıkarma gibi çizimin sağına, yani yan yana; tek geri alma adımı
  "Görünüşleri çıkar". Hepsi çıkarılmışsa bir şey yazılmaz, söylenir. Mimarlar Odası: "Bütün görünüşler çizilir".
- **İşaret yoksa dördü konur:** modelin katılarının kuzeye göre dönmüş kutusunun her yanının ortasına, 4 m öteye
  (aksların balonlarının ötesi: 1/50'de duvardan 2–2,5 m), binaya bakan dört görünüş işareti, 1–4 numaralı, işaret
  katmanında (`cad.viewmarks.create`); sonra dört görünüş. Bir ret olursa hiçbir şey yazılmaz.
- **Ölçüm (kesit binası, görünüş işaretsiz):** işaretler (6; 13,15) 270°, (16,15; 4,5) 180°, (6; −4,15) 90°, (−4,15;
  4,5) 0° — köşe kolonlarının yüzünden 4 m; tek geri alma işaretleri ve görünüşleri birlikte geri alır.

### 9. Denetim › Mevzuat: her cephenin görünüşü (2. bölüm)

- **Kural:** PAİY 57/2-a-6 "yeteri sayıda görünüş" ister; Mimarlar Odası standartlarında "Bütün görünüşler çizilir".
  Duvarı olan çizimde binanın cepheleri bulunur ve görünüşü Görünüş çıkar'la çıkarılmamış olanlar tek bulguda yazılır:
  `paiy_elevations_missing` — "Doğu, Güney ve Batı cephelerinin görünüşü çıkarılmamış (binanın 4 cephesi, 1 görünüşü
  var) — PAİY 57/2-a-6". Onarım: o cepheye bakan işaret ve Görünüş çıkar; hiç işaret yoksa Tümü (T).
- **Cephe:** modelin katılarının tabanlarının birleşimi (delikleri dışarıda) bina dış çizgisidir; her kenarı dışa baktığı
  yöne göre, kuzey okuna göre sekiz yönden birine sayılır. En uzun yönün beşte birinden ve 2 m'den uzun yön bir
  cephedir: 12 m'lik duvarların yanında 1,20 m'lik pah kendi cephesi değildir, komşularının görünüşünde görünür.
- Görünüşün yönü işaretten (§3) okunur; cephe ile aynı yöndeki çıkarılmış görünüş o cepheyi karşılar.
- **Ölçüm:** görünüş binasında cepheler KUZ, DOG, GUN, BAT (12,3 ve 9,3 m; köşe kolonlarının 2,5 cm'lik çıkıntıları
  eşiğin altında); hiç görünüş yokken dördü, kuzey çıkarılınca üçü istenir, Tümü'den sonra bulgu yok. Gerçek pencerede
  Denetim › Mevzuat 24 → 25 bulgu. Dört katlı yapının masaüstü denetiminde de dört cephe istenir.

## Doğrulama

- **Çekirdek:** bağımsız başvuruya (`scripts/fixtures/section_cases.py`) 18. durum: doğramanın çizgileri kendi
  katısında, taban alanı olmadan; `geom::section` durumları 18/18.
- **Ortak durumlar:** `cad.sections.edit` 30, `cad.sections.query` 26 (görünüş işaretinden çıkarma, geri alma adımı
  "Görünüş çıkar", işareti silinmiş görünüş, detay işaretinin reddi); masaüstünün işleyicileriyle aynı.
- **Uygulama** (`fixtures/arch/v1/facade-building.kcad`: kesit binası ve duvarlarının 3,5 m dışında dört görünüş
  işareti; KentOS kodu olmadan `scripts/fixtures/facade_building.py` yazdı, 81 nesne): `facades` 4 test — kuzey
  görünüşü bloğunda başlık, ölçek, üç kot; mahal adı ve tarama yok; pencereler ve giriş kapısı kendi katmanlarında,
  ikisinde de eğik açılım çizgileri; zeminin altındaki döşeme kenarı kesik; geri alma adı ve yerleşim katmanı; dört
  işaretin adları ve kuzey oku 90° döndürülünce yer değiştirmeleri, çıkarılmış görünüşün eskimesi ve Yeniden çiz'den
  sonra "DOĞU GÖRÜNÜŞÜ" ile `MM-XXXX-GORUNUSDOGU`; P15 penceresi 30 cm yükselince görünüş eski; takımda görünüş paftası
  kesitin hemen arkasında, GR/KUZ, işaretin altında paftanın numarası, eski görünüşün notu "paftası eski görünüşü
  gösterir". Hedefli koşu: uygulama 14, pafta 5, etkileşim 2, yayın 6, masaüstü 85.
- **Gerçek pencere** (Xvfb 1920 × 1080, gerçek fare ve klavye, sahibin üç ayar dosyasının kopyası;
  `.run/gui-evidence/mimari-m43/`): menü aramasında "Görünüş çıkar" (Mimari › Belgele) yeni simgesiyle; balona tıklayınca
  önizleme "Kuzey görünüşü · çizilmeyen: çatı, tabii zemin" ile imleci izler; ikinci tıkla görünüş yerleşir (nesne 81 →
  82, Bloklar'da `MM-XXXX-GORUNUSKUZEY`). P04'ün Konum'u Öznitelikler'den 1,50 → 2,00 m: durum çubuğunda kırmızı "Kuzey
  görünüşü eski"; tıklayınca yerleşim seçilir (Katman `MM-XXXX-GORUNUS`), Durum › Yeniden çiz → "Güncel", pencere
  görünüşte 0,5 m sağa geçer. Ruhsat takımı: UP-05 "Kuzey görünüşü" A2 (594 × 420 mm) 1:50, antette "Kuzey görünüşü",
  1/50, UP-05.
- **Görülen ve 2. bölümde düzeltilen:** kolon ve kiriş alınları, duvarın yüzüyle aynı düzlemde oldukları için görünüşte
  çizgi olarak çıkıyordu (§7); bütün cephe en ince kalemdeydi (§4).

## Sonraya kalan (M43 2. bölüm)

- Cephe kaplama malzemesi ve renk yazısı (MO: "Cephe kaplama malzemesi ve renkler yazılır").
- Tabii zemin "nokta nokta" (modelin tabii zemini olunca) ve çatı (ADV-20).
