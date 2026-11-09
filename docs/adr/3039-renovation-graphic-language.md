# 3039 — Tadilat grafik dili: tadilat durumu, açıklığın tadilat durumu, görünüme göre kesim, tadilat takımı

Tarih: 2026-11-09. Durum: kabul (M39; ADV-17).
- 1. bölüm: açıklığın kendi tadilat durumu; tadilat görünümüne göre duvar birleşimi ve kesimi.
- §0: kavramın adı "Evre"den "Tadilat durumu"na geçti.
- §3: araçlarda tadilat durumu (yeni çizilen, açık görünümün durumunu alır).
- §4: DXF'te tadilat durumu katmanları; DXF görünümün gösterdiğini alır.
- §5: Tadilat projesi görünümü (Mimarlar Odası'nın üç tekniği tek planda) ve tadilat lejantı.
- §6: tadilat takımı: Ruhsat takımında her kat üç pafta (mevcut durum, tadilat projesi, yeni durum), her biri kendi
  adlı görünümüne bağlı; tadilatta değişen odalar ve tadilat projesinde iptal oda.

## Sorun

- **Kapı ve pencerenin kendi tadilat durumu yoktu.** Tadilat durumu duvarda, mahalde, kolonda, merdivende, rampada ve
  blokta vardı (ADR 0346). Açıklık onu duvarından okuyordu (`Entity::phase`). Tadilatın en sık iki işi bu yüzden
  çizilemiyordu: **mevcut duvara açılan yeni kapı** ve **kapatılan mevcut pencere**.
- **Gizlenen nesne birleşmeye ve kesmeye devam ediyordu.** Görünüm nesneyi yalnız gizliyordu (`set_phase_hidden`);
  duvar birleşimi ve açıklık kesimi görünümden habersizdi.
  - Mevcut durum planında yeni kapının boşluğu duvarda delik olarak kalırdı.
  - Yeni durum planında yıkılan duvarın birleştiği duvarların yüzünde boşluk kalıyordu (ölçüldü: iki yatak odası
    arasındaki duvar iptal edilince alttaki ve üstteki duvarın yüzü T birleşiminin yerinde kesikti).
- **Paftadaki harita da aynı.** Bir harita kendi görünümünü gösterebiliyordu (ADR 0774 §3), ama kesim çizimin
  görünümüne göre yapılıyordu.
- **"Evre" sözcüğü yerinde değildi** (sahibin bildirimi, 9 Kasım: "Evre mantıksız bir kavram, daha düzgün bir kavram
  kullan"). Revit'in "Phase"inin çevirisiydi; Türkiye'deki tadilat işinin dili değil (§0).

## Araştırma (9 Kasım 2026)

### Türkiye mevzuatı ve meslek kuralı

- **Planlı Alanlar İmar Yönetmeliği (RG 3.7.2017/30113; mevzuat.gov.tr 7.5.23722, güncel metin):**
  - 4(1)(y) **esaslı tadilat:** taşıyıcı unsuru etkileyen ya da inşaat alanını, emsale konu alanı, taban alanını,
    bağımsız bölüm sayısını, ortak alanları, bağımsız bölüm alanını, kullanım amacını ya da **ruhsat eki projeleri**
    değiştiren işler; ruhsata tabidir.
  - 4(1)(dddd) **tadilat projesi:** ruhsat eki onaylı projelerde istenen değişiklik ve ilavelerle ilgili uygulama
    projelerinin bütünü.
  - 4(1)(i) **basit tamir ve tadil:** esaslı tadilat olmayan, taşıyıcı sistemi, bağımsız bölümün **dış cephesini**,
    ıslak hacimlerin yerini ve sayısını değiştirmeyen işler (doğrama tamiri dahil).
  - Sonuç: dış duvardaki pencereyi kapatmak cepheyi değiştirir, basit tamir değildir; tadilat projesi gerekir.
  - 58(2)–(4): ruhsatlı yapıda değişiklikte mimari proje yeniden düzenlenir; tadilat projeleri idarece mühürlenir; ilave
    ve tadilat projeleri 5 takım, 57. maddeye göre onaylanır.
- **Mimarlar Odası, "Mevzuat — Çizim tekniği ile ilgili sorular ve cevaplar":**
  - "İlave tadilat projelerinin çizim tekniği nasıl olmalı? Projede mevcut, iptal ve ilaveler üç farklı teknikle ifade
    edilmelidir."
  - Örnek tadilat lejantı: **mevcut** düz kontur, **ilave** eğik tarama, **iptal** kesikli kontur.
  - "Koçanda kayıtlı olan kısım mevcut tekniğinde, kayıtlı olmayan rölevesi yapılacak olan kısım yeni proje tekniğinde
    çizilmelidir."

- **Tadilat paftaları (§6):** PAİY 4(1)(dddd) tadilat projesini "uygulama projelerinin bütünü" sayar, 58(4) onu 57.
  maddeye göre takım olarak onaylatır. Uygulamada tadilat projesi mevcut durum (onaylı proje ya da rölöve) ile öneri
  (yeni durum) planlarını birlikte sunar: Salt Araştırma arşivindeki Bodrum konutu tadilat projesi "hali hazır kat
  planları" ile "öneri kat planları"nı birlikte verir (<https://archives.saltresearch.org/handle/123456789/191180>);
  Kültür ve Turizm Bakanlığı koruma kurullarının kamu hizmet standartları "mevcut durum ile öneri proje" ister
  (<https://korumakurullari.ktb.gov.tr/Eklenti/106359,kamu-hizmet-standartlaripdf.pdf>). Belediyeye özel bir pafta
  listesi bulunamadı; plan maddesi ADV-17 "üç tadilat paftası aynı modelden çıkar" der.

### Başka uygulamalar (yalnız görünüş için, kural değil)

- Revit: kapının kendi "Phase Created/Demolished"ı vardır. Yıkılan kapının yeri duvarla doldurulur (infill). Görünümün
  süzgeci gizlediği öğeyi birleşimden de çıkarır.
- Archicad: Renovation Status (Existing / To Be Demolished / New) ve süzgeç.

## Karar

### 0. Kavram: Tadilat durumu (eski adı "Evre")

Mimarlar Odası'nın üç sözcüğü kullanılır: **mevcut, iptal, ilave**. "İptal" duvarda da (kaldırılır) açıklıkta da
(boşluğu kapatılır) doğrudur, açıklığa ayrı sözcük gerekmez. Dosyalardaki ve komutlardaki adlar değişmedi (`phase`:
`existing`, `demolish`, `new`, `temporary`; `phaseView`); yalnız kullanıcının gördüğü sözcükler değişti:

| Eski | Yeni |
|---|---|
| Evre (Öznitelikler, mahal ayırıcı, oda modülü, çizelge sütunu ve süzgeci) | Tadilat durumu |
| Mevcut, Yıkılacak, Yeni, Geçici | Mevcut, **İptal**, **İlave**, Geçici |
| Şeritte Evre ▾; Evre: Tümü / Mevcut durum / Yıkım planı / Yeni durum | **Tadilat ▾**; Tadilat: Tümü / Mevcut durum / Yıkım planı / Yeni durum |
| Durum satırında "Evre: Yeni durum." | "Tadilat görünümü: Yeni durum." |
| Görünüş tarifinin ve pafta tarifinin evresi | Tadilat görünümü |
| Komut iletileri ("kat, evre, …", "kat, evre, durum ya da disiplin") | "kat, tadilat durumu, …", "kat, tadilat durumu, yapım durumu ya da disiplin" |
| Ruhsat takımı penceresinde "Evre ve planların ölçeği" (ön proje, ruhsat, uygulama) | **Proje aşaması** ve planların ölçeği |
| Dalga çizgisi stilinde "Evre" (Çizgi başından) | **Başlangıç** |

Görünüm adları (Tümü, Mevcut durum, Yıkım planı, Yeni durum) yerleşik Türkçe olduğu için kaldı. Desen kesiklerinin
"evresi" (taramada kesiğin kayması) başka bir kavramdır, dokunulmadı.

### 1. Açıklığın kendi tadilat durumu

- `OpeningFields.phase` (isteğe bağlı): yoksa duvarınki. `Entity::phase` önce açıklığın kendisininkini okur; açıklığın
  etiketi de onu. Çizelgenin süzgeci (`schedule_takes`) ve "Tadilat durumu" sütunu (`arch_value("phase")`) da.
- `cad.openings.edit` `set` `phase`; `null`: duvarınki.
- Öznitelikler'de açıklığın **Tadilat durumu** satırı: "Duvarınki (Mevcut)", Mevcut, İptal, İlave, Geçici. Satır ana
  satırlar arasındadır (Numara, Tür, Genişlik, Tadilat durumu).
- `.kcad` şema 218: açıklığın `phase`'i. Yoksa dosya eski şemasıyla bayt bayt aynı; şema 217 dosyasında `unknown_field`.
  Bağımsız Python okuyucusu ve yazıcısı aynı kuralla.

### 2. Tadilat görünümüne göre birleşim ve kesim

Görünümün gizlediği duvar başka duvarla birleşmez, gizlediği açıklık duvarı kesmez. Bu, tasarım seçeneklerinin
kuralının aynısı (ADR 0788 §4) ve aynı yoldan: `Store::set_option_out`.

| Görünüm | Mevcut açıklık | İptal açıklık | İlave açıklık |
|---|---|---|---|
| Mevcut durum | keser | keser | gizli, kesmez (duvar bütün) |
| Yıkım planı | keser | kesikli, keser | gizli, kesmez |
| Yeni durum | soluk, keser | gizli, kesmez (duvar dolu) | keser |

- **Ekranda** `Spatial::sync_phases` görünümün gizlediklerini bir kez bulur; duvar ve açıklıkları seçeneklerle birlikte
  dışarıda bırakır.
- **Paftada** (PDF ve ekrandaki pafta) kendi tadilat görünümünü gösteren harita, görünümü çizimden farklıysa kendi
  kopyasından çizilir: `Document::with_phase_view`. Kopyanın anahtarı görünümün adını taşır. Aynı görünümdeki ya da
  görünümsüz harita çizimin kendisinden çizilir.
- `viewport_scope::phase_out`: kapsamlı pafta deposu da aynı kuralla.

### 3. Araçlarda tadilat durumu

Yeni çizilen yapı öğesi, çizildiği görünümün durumunu alır; ayrı bir seçim yoktur:

| Görünüm | Yeni çizilenler |
|---|---|
| Tümü | durumsuz (eskisi gibi) |
| Mevcut durum, Yıkım planı | Mevcut (rölöve çizilir) |
| Yeni durum | İlave (işler çizilir) |

- Revit'in kuralının aynısı: öğe, görünümün evresinde yaratılır. Ayrı bir "etkin durum" ayarı yok: bir ayar daha
  unutulurdu, görünüm zaten açık.
- Tadilat ▾ bunu söyler: "Tadilat görünümü: Yeni durum. Bundan sonra çizilenler İlave olur."
- Duvar (Duvar, Dikdörtgen duvar, Bölme, Çizgiden duvar, Oda), kolon, merdiven, rampa, döşeme, mahal, mahal ayırıcı ve
  blok yerleştirme bunu `Context::new_arch` / `Context::phased` ile yazar. Aks (`level_arch`) yazmaz: aks bir dayanak
  çizgisidir, tadilatı yoktur.
- Kapı ve pencere: görünümün durumu duvarınkinden farklıysa kendi tadilat durumu olarak yazılır. Yeni durumda mevcut
  duvara konan kapı İlave olur; ilave duvara konan kapı duvarınkini alır, kendine ayrı durum yazılmaz.
  `cad.openings.create` buna `phase`'i aldı.
- Mahal: Yeni durumda bulunan oda İlave olur ve sınırını işlerden sonraki duvarlardan bulur (ADR 0798 §3: iptal duvar
  sınır değildir). İki odayı birleştiren tadilatın yeni odası böylece doğru çıkar.

### 4. DXF'te tadilat durumu

- **Katman:** EK-4 Tablo 4.1'in katman adı üç alanlıdır: disiplin, öğe, malzeme/tip/gösterim. Üçüncü alan malzemeyi
  tuttuğu için durum onun arkasına gelir. 4.14.3 derinliği ("-1…4"), 4.14.5 taramayı ("-T") aynı yerden ekletir.
  - Örnekler: MM-DUVA-TUGLA-IPTAL, MM-KAPI-GENEL-ILAVE, MM-DUVA-TUGLA-MEVCUT, …-GECICI.
  - Taraması onun "-T"si: MM-DUVA-TUGLA-IPTAL-T.
  - Sözcükler 4.11'e göre büyük harf ve Türkçe harfsiz: MEVCUT, IPTAL, ILAVE, GECICI (`ek4_state_layer`, `EK4_STATES`).
  - `is_ek4_layer` dördüncü alan olarak yalnız bunları kabul eder.
  - Örneği AIA'nın "status" alanı (A-WALL-DEMO).
- **Parçalar** durum katmanına gider. İptal katmanı kesikli yazılır (Mimarlar Odası'nın iptal tekniği). Katman yalnız ona
  parça düşerse açılır: çizgisi olmayan boşluk katman açmaz. KentOS verisini taşıyan nesnenin kendisi kendi katmanında
  kalır, geri okunduğunda yeri değişmez.
- **Görünüm:** DXF, tasarım seçeneklerindeki gibi (ADR 0788 §5) çizimin görünüşünü alır. Tadilat görünümünün gizlediği
  nesne yazılmaz ve rapor bunu söyler ("Tadilat görünümünün gizlediği nesne"). Yazılmayan açıklık duvarı kesmez, yazılmayan
  duvar birleşmez. Sonuç:
  - Tümü: bütün durumlar kendi katmanlarında. Tadilat projesi DXF'i budur.
  - Mevcut durum: rölöve; yeni kapının yeri duvarda boşluk değil.
  - Yeni durum: işlerden sonraki hâl.
  ADR 0346'nın "DXF'e evre görünüşü yazılmaz" kuralı böylece değişti.
- **Rapor:** "Tadilat durumu: N nesne durum katmanlarına yazıldı".
- **Bulgu (düzeltilmedi):** GDAL'ın DXF okuyucusu Türkçe harfli katman adlarını (Kapı-IPTAL) `DXF_ENCODING=UTF-8`
  verilmeden bozuk okur. EK-4 adları ASCII olduğu için bu yolu etkilemez.

### 5. Tadilat projesi görünümü ve lejantı

Mimarlar Odası tadilat projesini **tek planda üç teknikle** ister. Mevcut durum, Yıkım planı ve Yeni durum bunu
vermiyordu, çünkü her biri bir durumu gizliyordu. Beşinci görünüm **Tadilat projesi** (`PhaseView::Renovation`,
`renovation`) bütün durumları birlikte çizer:

| Nesne | Çizim |
|---|---|
| Mevcut (ya da durumsuz) | düz kontur: duvarın düz dolgusu bu görünümde çizilmez (lejantın boş kutusu) |
| İlave | gövdesi 45° taralı (ANSI31, görünümün ölçeğinde) |
| İptal, geçici | kesikli |
| Kalan duvarda kapatılan kapı/pencere (`RenovationMark::Infill`) | duvarı kesmez; pervazları arasındaki yer ilave taramayla dolar |
| Kalan duvara açılan yeni kapı/pencere (`RenovationMark::CutIn`) | duvarı keser; kaldırdığı duvarın iki yüzü kesikli |

- `Entity::renovation_mark` işareti verir. `Spatial::sync_phases` işaretleri depoya `Store::set_renovation` ile
  verir: düz dolgu kalkar, ilave gövde taranır, dolgu ve kesik yüz parçaları eklenir. Kapatılan açıklık seçenek
  dışındakiler gibi kesimin dışında kalır.
- Çizim motoru dolgu açıklığını kesikli çizmez, onu depo çizer (`Phasing::of`).
- Bu görünümde çizilen İlave olur (§3). DXF'te hiçbir şey gizlenmez; her nesne kendi durum katmanında (§4).
- Görünüm adlı görünümün tarifinde de seçilir.
- `.kcad` şema 219: projenin `phaseView`'u ya da bir tarifin `phase`'i `renovation` ise. Şema 218 dosyasında bu değer
  `bad_value`. Bağımsız Python okuyucusu ve yazıcısı aynı kuralla.
- **Tadilat lejantı** (`tool.renovationLegend`, Tadilat ▾'nin ikinci bloğu): Mimarlar Odası'nın örnek lejantı bir blok
  olarak (`MM-XXXX-TADILATLEJANTI`, lejant katmanı) imleçten yerleşir.
  - İçerik: başlık, ardından Mevcut düz kutu, İlave 45° taralı kutu, İptal kesikli kutu.
  - Ölçüler pafta ölçeğinde kâğıt milimetresi: yazı 2,5 mm, başlık 3,5 mm, kutu 15 × 6 mm, tarama 3,175 mm aralıkla.
- Simgeler `phaseRenovation` ve `renovationLegend`: lejantın üç kutusu küçükte
  (`.run/icons-m39/ikonlar.png`).

### 6. Tadilat takımı

Ruhsat takımı (ADR 3037) penceresinde **Tadilat projesi** kutusu. Çizim tadilatsa işaretli açılır
(`permit_set::renovates`): bir duvar duruyor (Mevcut ya da durumsuz) ve bir duvar, kapı ya da pencere İptal ya da İlave.

- Her kat üç pafta, katlar alttan üste: "Zemin kat planı (mevcut durum)", "(tadilat projesi)", "(yeni durum)". Düz
  takımdaki tek planın yerine geçer. EK-2'de türü, tipi ve kat kodu aynı olduğundan sıra no 01, 02, 03 olur
  (`…_PL_MMC_K00_02_000`).
- Her planın haritası takımın adlı görünümüne bağlıdır (`Source::ViewLink`, `permit::Page::linked`):
  "Ruhsat takımı · Mevcut durum", "· Tadilat projesi", "· Yeni durum". Tarifte yalnız tadilat görünümü ve aşamanın
  ölçeği (1/100, 1/50) vardır, kutusu planların kutusudur; kat haritanın kalır. Paftalar çizim hangi görünümde açık
  olursa olsun aynıdır. Önce planın haritası çizimin o anki görünümünü çiziyordu.
- Vaziyet planı "Ruhsat takımı · Vaziyet planı"na bağlıdır: Tadilat projesi tekniği (Mimarlar Odası: "Projede mevcut,
  iptal ve ilaveler üç farklı teknikle ifade edilmelidir"), kendi ölçeğinde (1/500, 1/1000).
- Görünümler projeye yazılır: değişkenler gibi bir ayar, geri alma adımı değil. Takım yeniden kurulunca aynı adlı
  görünüm yerinde yenilenir: kutusu, tadilat görünümü ve ölçeği. Tarifin geri kalanı (kalem, siyah beyaz) ve kimliği
  kalır; eski paftaların bağı kopmaz. 100 görünüm sınırı aşılacaksa takım kurulmaz, söylenir.
- **Mahaller.** Her mahal kendi durumundaki duvarlarla sınırlanır (ADR 0798 §3): Mevcut oda mevcut ve iptal duvarlarla,
  İlave oda mevcut ve ilave duvarlarla, durumsuz oda bütün duvarlarla.
  - Tadilatta değişip durumu boş ya da Mevcut kalan oda paftada orada olmayan bir duvarı oda sınırı olarak gösterir.
    Mevcut durumda yeni bölme, yeni durumda kaldırılan duvar görünür. Ölçüldü: rölöve dairede durumsuz Salon'un sınırı
    yeni bölmenin etrafından dolanıyordu; mevcut durum paftası olmayan bölmeyi çiziyordu.
  - Değişen oda: İptal ya da İlave bir duvarın 30 cm'den uzun boyunca sınırladığı ya da kestiği oda (köşede değmek
    sayılmaz) (`permit_set::rooms_to_restate`).
  - Pencere bu odaları kutunun altında adıyla söyler, takım kurulunca iletide de söylenir. Doğru akış: değişen odalar
    İptal, yenileri Yeni durumda Mahal aracıyla çizilir (İlave olur).
- **Tadilat projesinde iptal oda ve etiketi gizlidir** (`Entity::look_in`). Bu, görünüşün tek kuralıdır: ekran, pafta,
  PDF, DXF ve görünümün kapsamı onu okur.
  - Mimarlar Odası'nın üç tekniği yapı elemanları içindir; tadilat projesi planı odaları olacakları gibi adlandırır.
  - Gösterilseydi eski odanın adı ve alanı yenisinin üstüne binerdi: "02 Yatak odası 1 · 16,20 m²" ile "Yatak odası ·
    28,58 m²" aynı yerde. İptal odanın kesikli sınırı da iptal duvarın kesikli yüzüne kayık adımla binip onu düz
    gösteriyordu. Ölçüldü: kâğıtta ikisi de 3,0 / 1,5 mm kesikli.
  - Yıkım planında iptal oda kesikli kalır.
- İleti "Ruhsat takımı (tadilat projesi): N pafta (UP-01 … UP-N)"; yayın klasörü "Ruhsat-tadilat-<tarih>".

## Doğrulama

- Etkileşim (`crates/native/interaction/tests/all/phase_cuts.rs`): 6 m mevcut duvar, 1,5 m'de ilave 90'lık kapı, 4 m'de
  iptal 120'lik pencere. Duvar yüzündeki boşluklar santimetreyle:
  - Tümü: kapı (1,05–1,95) ve pencere (3,40–4,60).
  - Mevcut durum: yalnız pencere.
  - Yıkım planı: yalnız pencere.
  - Yeni durum: yalnız kapı.
  - Kapının kendi tadilat durumu kaldırılınca duvarınkini (Mevcut) alır ve Mevcut durumda da keser.
  - İptal köşe duvarı Yeni durumda birleşmez: komşu duvarın dış yüzü 6,15 → 6,00 m'de biter.
- Pafta (`book.rs` `phase_tests`, `sheets.rs` `a_map_with_its_own_phase_draws_a_copy_cut_as_it_stands`): çizim Mevcut
  durumdayken Yeni durum haritası kapıyı keser, pencereyi kapatır. Aynı görünümdeki ya da görünümsüz harita kopya
  açmaz; çizim değişmez.
- Öznitelikler (`an_openings_renovation_row_writes_its_own_phase`): satır, sözcükler, tek adım, Duvarınki ile geri alma.
- Araçlar (`phase_cuts.rs` `what_a_tool_draws_takes_the_view_s_state`):
  - Mevcut durumda çizilen duvar Mevcut.
  - Yeni durumda çizilen duvar İlave.
  - Yeni durumda mevcut duvara konan kapının kendi durumu İlave; ilave duvara konanınki yok, duvarınkini (İlave) alır.
  - Tümü'nde çizilen duvar durumsuz.
  - Durum satırı dört görünümde (`arch_project.rs` testi).
  - `cad.openings.create`'e `phase`'li durum.
- Ortak komut durumları: `cad.openings.edit`'e iki yeni durum (kendi tadilat durumu ilave; iptal, sonra kaldırılır);
  bağımsız başvuru `scripts/fixtures/opening_command_cases.py`. §0'ın sözcükleri 19 başvurunun durum adlarında ve
  iletilerinde; 27 durum dosyası yalnız bu sözcüklerle değişti. Beş dosyanın üreticisi bugünkü dosyayı yeniden
  üretmiyordu: başka işlerden kalan bir sapma. O dosyalarda yalnız sözcükler değiştirildi.
- KCAD 218: `opening-phases` örneği ve şema 217'de bozuk dosyası; çok yeni şema örneği 219'a kaydı (geçerli örnek 249).
- DXF (§4):
  - `dxf_write.rs` `walls_with_a_renovation_state_go_on_their_state_layers`: iptal, ilave ve durumsuz duvar.
    - Çizgiler MM-DUVA-TUGLA-IPTAL (kesikli), -ILAVE ve MM-DUVA-TUGLA'da.
    - Taramalar -IPTAL-T, -ILAVE-T ve -T'de.
    - Rapor 2 der.
  - `ek4.rs` testi: durum katmanı adları ve `is_ek4_layer`.
  - Başsız `renovation_dxf.rs`:
    - Örnek Tümü'nde Duvar-MEVCUT (16 parça), Kapı-ILAVE (2), Kapı-IPTAL (3, kesikli).
    - Mevcut durumda yeni kapı yazılmaz ve söylenir; duvarda onun boşluğu yok.
    - Yeni durumda iptal pencere yazılmaz.
  - Dosya GDAL 3.12.2 `ogrinfo` ile okundu: aynı katmanlar ve sayılar; katman tablosunda Kapı-IPTAL DASHED. KentOS'ta
    açıldığında katman paneli aynı üç katmanı gösterir.
- Gerçek pencere (1920×1080, rölöve daire: bütün duvarlar Mevcut; iz `fixtures/interaction/real/tadilat-durumu.txt`):
  - İki yatak odası arasındaki duvar İptal yapıldı.
  - Salon ile yatak odası 1 arasına kapı kondu ve İlave yapıldı.
  - Salonun batı penceresi İptal yapıldı.
  - Mevcut durumda yeni kapı yok ve duvar bütün.
  - Yıkım planında duvar kesikli, kapı yok.
  - Yeni durumda kapı var, pencere kapalı, iptal duvarın birleşim yerlerinde komşu yüzler kesintisiz.
  - ÖNCE (M38 ikilisi) açıklığın bu satırı yoktu ve yeni kapı Mevcut durumda da görünüyordu.
- Tadilat projesi (`phase_cuts.rs` `the_renovation_project_marks_each_state`):
  - Mevcut duvarın düz dolgusu Tümü'nde var, bu görünümde yok.
  - İlave duvarın gövdesinde ANSI31 taraması var.
  - İptal pencere duvarı kesmez: yalnız kapının boşluğu kalır. Pencerenin tek parçası pervazlar arasında (3,40–4,60) bir
    tarama.
  - Yeni kapının kendi çizgilerinin yanında iki kesikli çizgi var.
  - Tümü'ne dönünce her şey eski hâline gelir.
- Görünüş tablosu: `phase_cases.py` Tadilat projesi sütunuyla (mevcut ve ilave olduğu gibi, iptal ve geçici kesikli).
- Lejant (`renovation_legend_tests.rs`):
  - Blok adı, dört yazı (başlık ve üç sözcük), bir tarama, üçüncü kutu kesikli.
  - Kutu eni 15 mm × pafta ölçeği.
  - Tek adım, geri alınır.
- KCAD 219: `renovation-view` örneği, şema 218'de `bad_value`; çok yeni şema örneği 220'ye kaydı (geçerli örnek 250).
- Gerçek pencere, §5 (iz `fixtures/interaction/real/tadilat-projesi.txt`): rölöve dairede şu adımlar yapıldı:
  - yatak odaları arasındaki duvar İptal;
  - Yeni durumda salon ile yatak odası 1 arasına kapı ve salona yeni bir bölme;
  - batı penceresi İptal;
  - ardından Tadilat: Tadilat projesi ve Tadilat lejantı.

  Sonuç: bölme taralı, pencerenin yeri taralı dolgu, iptal duvar kesikli, lejant planın altında.
- Gerçek pencere, §3 (iz `fixtures/interaction/real/tadilat-araclar.txt`): rölöve dairede Yeni durumda salon ile yatak
  odası 1 arasına kapı konur. Kapı İlave olur; Mevcut durumda yoktur ve duvar bütündür. ÖNCE (§0'ın ikilisi) kapı
  "Duvarınki (Mevcut)" oluyor ve Mevcut durumda görünüyordu.
- Testler: 9 paket 4222 geçti; masaüstü 2033 geçti (tek kırmızı işten önce de kırmızı olan
  `module_surface_tests::each_program_docks_its_own_panels`).
- Tadilat takımı, §6:
  - Pafta (`sheet/tests/all/permit.rs` `a_tadilat_plan_draws_its_view_s_state_on_its_own_storey`): üç bağlı plan;
    adlar, EK-2 sıra no 01–03, haritanın kaynağı bağ ve kilitli; çizilen haritanın tadilat görünümü, katı ve 1/100'ü;
    görünüm yoksa harita çizilmez.
  - Uygulama (`permit_set.rs` `a_set_s_views_are_renewed_in_place`): yeniden kurulumda görünüm yerinde yenilenir,
    siyah beyaz ve başka adlı görünüm kalır, sürüm 2.
  - Masaüstü (`renovation_set_tests.rs`): iki katlı bina, zemin katta iptal duvar ve ilave bölme.
    - Değişen odalar "Salon, Merdiven evi, Salon". İptal yapılıp yenileri İlave çizilince söylenecek oda kalmaz.
    - 8 pafta ve dört görünüm. Her haritanın tadilat görünümü (vaziyet: Tadilat projesi, 1/500), katı ve 1/100'ü.
    - Antette pafta adı ve EK-2 adı; PDF/A-2 sekiz dosya uygun (K00_01…K01_03).
    - Yeniden kurulum UP-09'dan sürer, görünümler 4 kalır. Görünümler silinince harita çizilmez.
    - Pencere: yeni yapıda kutu boş, tadilatta işaretli ve odalar söylenir. İleti "Ruhsat takımı (tadilat projesi): 10
      pafta (UP-01 … UP-10)" ve oda uyarısı. Yayın klasörü Ruhsat-tadilat-<tarih>, 10 pafta seçili.
  - Kitap (`book.rs` `a_map_with_its_own_phase_leaves_out_what_its_view_hides`): çizim Tümü'ndeyken mevcut durum
    haritası İlave duvarı, yeni durum haritası İptal duvarı çizmez. Tadilat projesinde iptal duvarın bütün çizgileri
    kesikli, ilave duvarınki değil; iptal oda çizilmez.
  - Stil (`phases.rs` `tadilat_projesi_leaves_out_a_room_pulled_down_and_its_label`): iptal oda ve etiketi tadilat
    projesinde gizli, yıkım planında kesikli; iptal duvar kesikli.
  - Gerçek pencere (iz `fixtures/interaction/real/tadilat-takimi.txt`). Rölöve dairede duvarlar ve odalar Mevcut,
    bilgiler ve parsel girilmiş.
    - Tadilat yapıldı: iptal duvar, ilave kapı ve bölme, iptal pencere.
    - Ruhsat takımı'nda kutu işaretli; uyarı "01 Salon, 02 Yatak odası 1, 03 Yatak odası 2". Takım geri alındı.
    - Üç oda İptal yapıldı. Yeni durumda Mahal aracıyla Yatak odası 28,58, Salon 26,55, Mutfak 8,32 m² çizildi.
    - Takım yeniden kuruldu ve yayınlandı: 8 PDF/A-2 dosyası (K00_01, _02, _03).
    - Mevcut durum: eski odalar (Salon 36,00 m²); bölme, yeni kapı yok, pencere var.
    - Tadilat projesi: iptal duvar kesikli, bölme ve pencerenin dolgusu taralı, yalnız yeni oda adları.
    - Yeni durum: birleşen yatak odası. Ekrandaki paftalar aynı, yerleşim denetimi temiz.
    - ÖNCE (§5'in ikilisi): kutu yok. Tek "Zemin kat planı" çizimin o anki görünümünü (Tümü) çiziyor; mevcut, iptal ve
      ilave aynı çizgiyle karışık.
  - Testler: 13 paket 7521 geçti; masaüstü 2038 geçti (tek kırmızı yine
    `module_surface_tests::each_program_docks_its_own_panels`).

## Sınırlar

- Metraj tadilat durumuna göre ayrılmıyor: mevcut duvarın net yüzü bütün açıklıklarını düşüyor (ilave kapının kırım
  metrajı yok).
- Mahal görünüme göre yeniden bulunmuyor: odanın sınırını kendi tadilat durumu belirler (ADR 0798 §3). Değişen odaları
  kullanıcı İptal yapar ve yenilerini çizer; takım ve penceresi yalnız söyler (§6).
- Mahal listesi paftası ve metraj tadilat durumuna göre ayrılmıyor: iptal ve ilave odalar birlikte listelenir.
- Hesap tabloları çizimin katmanında ve katsız: kat planı ve vaziyet planı haritaları onların kesik bir parçasını
  gösterebiliyor. Bu M37'den beri var ve tadilattan bağımsız; ayrı iş.
- Parçasal iptal (duvarın bir kesimini kaldırmak) duvarı bölmekle yapılır (Duvarı böl); ayrı bir araç yok.
