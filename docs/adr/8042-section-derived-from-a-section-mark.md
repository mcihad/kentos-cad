# ADR 8042: Kesit çıkar — kesit işaretinden 1:50 bina kesiti, eskime işareti

- **Durum:** kabul edildi (10 Kasım 2026); `plan/architecture-advanced.md` ADV-19'un ilk teslimi (M42 1. bölüm).
- **Bağlam belgesi:** ADR 0345 (görünüm işaretleri), ADR 0347 §3 (otomatik kesitin tasarımı), ADR 0799 (türetilmiş çizim:
  oda iç görünüşü), ADR 8008 (EK-4 katman anahtarı), ADR 8013 (ölçeğe göre çizgi grupları), ADR 8014 (TS 5319
  taramaları), ADR 8040 (düşey model), ADR 8041 (kiriş, lento, kolon–duvar düğümü).

## Bağlam

Planda kesit işareti (Mimari › Belgele › İşaret ▾ › Kesit) yalnız bir başvuruydu; kesitin kendisi elle çiziliyordu. ADR
0347 §3 otomatik kesitin önce düşey modeli (döşeme, duvar tepesi) istediğini yazmıştı; M40 ve M41 onu kurdu: duvarın
tabanı ve tepesi, döşemenin katmanları, kiriş, lento, kolonun malzemesi. Bu teslim kesiti o modelden çıkarır.

## Araştırma

### Türkiye mevzuatı

- **Planlı Alanlar İmar Yönetmeliği (RG 3.7.2017/30113; son değişiklik 1.7.2026/33297) Md. 57(2)(a)(6):** mimari proje
  "kat ve çatı planlarına ilişkin bir tanesi ortak merdivenden geçmek üzere en az iki adet kesit ve yeteri sayıda
  görünüş" içerir. Bu teslim kesiti çıkarır; iki kesit ve merdiven denetimi 2. bölümde (Denetim › Mevzuat).
- **Mimarlar Odası, Mimarlık Hizmetleri Şartnamesi, Mimari Proje Çizim ve Sunuş Standartları:** uygulama projesinde
  ölçek 1/50; plan, kesit ve görünüş aynı ölçekte; "kesit çizgisi ve bakış yönü planda gösterilir; kesitte geçilen
  mahallerin kodu ve adı yazılır"; en az iki kesit, biri merdivenden, biri konstrüksiyonu gösteren; kat yükseklikleri,
  iç yükseklikler ve kotlar; tabii ve tesviye zemin.
- **Mimarlık ve Mühendislik Projelerinin Elektronik Ortamda Teslimi ve Yönetilmesi Hakkında Yönetmelik ekleri (2026):**
  - **EK-4 4.14.3:** plan ve kesit çizimlerinde kesite ve görünüşe giren yerlerin katmanı malzemeyle adlandırılır;
    aynı malzemenin farklı derinlikteki öğeleri ada "-" ve 1–4 eklenerek ayrılır (1 en yakın, 4 en uzak).
  - **4.14.5:** taranacak öğe adının sonuna "-T" eklenmiş ayrı katmanda taranır.
  - **4.23:** taramalarda TS EN ISO 128-3 esas alınır; farklı malzeme farklı taramayla.
  - **4.27 ve Şekil 4.3:** kesit ve görünüşte kot işareti: değer bir çizginin üstünde, altında ucu kota değen üçgen
    ("+3.75", "±0.00", "-2.75").
  - **4.28 ve Şekil 4.4:** kesit hattı noktalı kesikli çizgiyle çizilir; bakış yönünü gösteren okun ucu bakış tarafına.
  - **4.29 ve Şekil 4.5:** akslar noktalı kesikli, numaraları sürekli çizgili yuvarlakta.
  - **4.30 ve Şekil 4.6:** tabii zemin kotu ve tesviye kotu.
  - **EK-2 Tablo 2.12:** kesit dosyasının türü KS; **2.9.3:** kesit ve görünüşte "Kat" yerine müellifin verdiği üç
    karakterli "Konum". **Tablo 4.4:** blok adı `MM-XXXX-<ad>`.
- **TS EN ISO 128-3 (görünüş, kesit ve kesim):** aynı parçanın kesitleri tek tarama, kesilen kalın çizgiyle çevrilir
  (ADR 8041'deki 7.2 özeti).
- **Merdiven plağı:** TS 500 ve TBDY 2018'de merdiven plağı için doğrulayabildiğim bir en küçük kalınlık yok (aramada
  yalnız ders notlarında 12 cm örneği çıktı). Kesit plağın kalınlığını merdivenin vardığı döşemenin taşıyıcısından alır;
  döşeme yoksa merdiveni dolu çizer ve söyler.

### Başka programlar (yalnız görünüş için)

- **BricsCAD BIM, *Generating drawings*:** BIMSECTION kesit nesnesi, BIMSECTIONUPDATE kesiti ayrı çizime üretir; Project
  Browser'da modelle uyumsuz pafta görünümü **kırmızı**, hiç güncellenmemiş olan **turuncu** noktayla işaretlenir
  (`.run/gui-evidence/mimari-m42/referans/bricscad-GD_generatingdrawings_sheets.png`).
- **Archicad, *Model Display in the Section Window*:** kesilen elemanlar katmanlarının taramasıyla; "Marked Distant Area"
  uzaktaki elemanları ayrı renkte çizer — EK-4 4.14.3'ün 1–4 derinlik ekine karşılık
  (`archicad-SectionComposite.png`, `archicad-SectionEx3.png`).

## Karar

### 1. Kesit, işaretinin bloğudur

- **Kesit çıkar** bir kesit işaretinin kesitini bir bloğa çizer ve bloğu yerleştirir. Bloğun kimliği işaretin
  kimliğinden türer (her dosyada aynı); adı EK-4 Tablo 4.4'e göre `MM-XXXX-KESITA` (işaretin harfi). Bloğun taban
  noktası kesit çizgisinin bakan kişiye göre sol ucunda ±0,00'dır.
- Neden blok: kesit paftada ve DXF'te tek nesnedir; içindeki nesneler EK-4'ün malzeme katmanlarındadır (4.14.3), blok
  yeniden çizilince her yerleşimi kendiliğinden güncellenir. ADR 0799'un iç görünüşü kendi katmanına dağınık nesneler
  yazıyordu; EK-4'ten önceydi.
- Kırıklı (üç ve daha çok noktalı) kesit henüz çıkarılmaz (`section_jogged`).

### 2. Çekirdek: kesilen ve görünen (`geom::section`)

- Kesit düzlemi işaretin ilk iki noktasından geçer; bakan kişi çizginin soluna bakar (çevrilmişse sağına).
- **Kesilen:** düzlemin her iki yanında taban alanı olan katı (düzlemin üstündeki yüz kesilmez, görünür); prizma
  yüksekliği boyunca dikdörtgen, açıklık kutusu çıkarılır; merdiven kolu basamaklarıyla ve eğik plağıyla.
- **Görünen:** düzlemin ötesinde bakan kişiye dönük düşey yüzler, merdivenin rıhtları ve geriye dönük plak altı;
  kenarları önlerinde bir şey yokken çizilir (yüzün içinde ya da kenarında ve ondan bir mikrometreden çok gerideyse
  gizli). Açıklık yüzde kapalı sayılır (kapı ya da pencere); çevresi yüzde işaret olarak görünür.
- **Kesilen, düzlemin içindeki kenarı da gizler.** Kesit çizgisi planda bir rıhtın üstüne düşerse (çizgi rıhta
  yakalanarak çizildiğinde olağan) rıht kesilenin kendi yüzüdür; alt kenarı (basamak burnu) taramanın içinden geçmez.
  Gerçek pencerede A-A (y 2,40, ilk rıhttan 5 × 0,28) kolun gri kesitini ortadan bir çizgiyle bölüyordu.
- Bağımsız başvuru: `scripts/fixtures/section_cases.py` (KentOS kodu olmadan, tam kesirlerle, 17 durum; 17.si rıhtın
  üstünden geçen kesit).

### 3. Modelin katıları

- Duvar: plandaki birleşimiyle (duvar birleştirici, kolonların kesmesi; ADR 8041 §2), her katmanı ayrı katı, tabanından
  tepesine (ADR 8040 §1); açıklıklar katın döşemesine göre denizlikten lentoya çıkarılır; lentosu yoksa duvarın
  tepesine kadar (listelenir).
- Döşeme: üstünden aşağı kaplama, şap, yalıtım, su yalıtımı, taşıyıcı, sıva (ADR 8040 §2).
- Kolon: altındaki taşıyıcıdan katının üstündeki taşıyıcıya. Kiriş, lento, hatıl: alt yüzünden üstüne (ADR 8041).
- Merdiven: her düz kolu bir kol, sahanlığı bir döşeme; plağı vardığı döşemenin taşıyıcısı kalınlığında.
- Çizilmeyen ve listelenen: çatı (ADV-20), tabii zemin, eğri merdiven, rampa, niş ve baca.

### 4. Çizim (1/50)

- **Kesilen:** malzeme malzeme birleşir (betonarme döşeme, kiriş, kolon, merdiven ve perde tek kesit); duvarın sıvası
  çekirdeğiyle tek çevre. Çevre 0,50 mm (döşemenin kaplama katmanları 0,25 mm); tarama planın 1:50'deki gibi
  (tuğla ANSI31, betonarme gri, ısı yalıtımı TS 5319 SN55), `-T` katmanında.
- **Görünen:** malzemesinin katmanında derinlik ekiyle (`MM-DUVA-TUGLA-1` … `-4`, en uzağa göre dört dilim); 1 ve 2
  0,25 mm, 3 ve 4 0,18 mm.
- **Açıklığın boşluğu:** kasa derinliği biliniyorsa kasanın iki yüzü, değilse camın çizgisi.
- **Kotlar** solda EK-4 Şekil 4.3 gibi (katların döşemesi, en üst katın tepesi, ±0,00); **kat yükseklikleri** sağda.
  İkisi de binanın ucuna göre durur (kesilen ve görünenin ucundan 1 m), kesit çizgisinin ucuna göre değil: çizgi binayı
  1,5 m aşınca kotlar binadan 2,4 m uzakta kalıyordu.
- **Akslar:** çizginin kestiği düz akslar noktalı kesikli, numaraları altta tek sırada yuvarlakta (ilk sürüm her aksta
  bir sonrakini bir balon aşağı itiyordu).
- **Mahaller:** çizginin geçtiği mahallerin numarası ve adı, hiçbir çizgiye değmeyen ilk yerde (TS EN ISO 3098-0:
  yazının içinden çizgi geçmez): önce ortada ve yüksekliğin %45'inde, sonra 25 cm adımlarla yanlarda, sonra %60, %30,
  %75, %15 yükseklikte; yer yoksa ortada ve maskeli. Ortada bir aks (B-B'de 2 aksı merdiven holünün tam ortası) ya da
  merdiven (A-A'da) yazıyı kesiyordu.
- **Zemin:** tesviye ±0,00'da kalın, kesilenin içinden geçmez. **Başlık** "A-A KESİTİ", altında "Ölçek 1/50".
- **Plandaki kesit çizgisi** EK-4 4.28'e göre noktalı kesikli (önce düzdü); oklar ve harfler değişmedi.

### 5. Eskime

- Kesit, yeniden çizildiğinde aynı nesneleri veriyorsa **güncel**, vermiyorsa **eski**; işaretin bloğu yoksa
  **çıkarılmadı**, bloğun işareti silinmişse **kaynağı yok** (`cad.sections.query`).
- **Yeniden çiz** (`cad.sections.edit` refresh) bloğu modelin bugünkü hâliyle yazar; her yerleşim izler. Elle yapılan
  değişiklik blokta tutulmaz (doğrama listesi gibi, ADR 8038).
- **Görünür kılınması:** eski bir kesit varken durum çubuğunda kırmızı "B-B kesiti eski" (birden çoksa "2 kesit eski");
  tıklanınca kesitin yerleşimi seçilir ve görünüm ona yaklaşır; Öznitelikler › B-B kesiti › Durum'un açılır listesinde
  Yeniden çiz. Durum satırı işaret seçiliyken de görünür.
- **Araç:** Mimari › Belgele › İşaret ▾ › Kesit çıkar (komut KESITCIKAR, BIMSECTION; menü aramasında "Kesit çıkar").
  Kesit işaretinin çizgisine tıklanır (araçtan önce seçili işaret alınır); önizleme imleci izler ve modelin çizmediğini
  söyler ("çizilmeyen: çatı, tabii zemin"); tıklanan yer kesitin sol ucunun ±0,00'ı (Enter: çizimin sağına). Kesiti
  çıkmış işarete tıklamak onu yeniden çizer; tek geri alma adımı.

## Doğrulama

- **Çekirdek:** `geom::section` birim testleri 12 (rıhtın üstünden geçen kesit dahil); bağımsız başvuruyla 17 durum
  1e-7 içinde aynı (`kentos-geometry-core` tests/all/sections.rs).
- **Ortak durumlar:** `cad.sections.edit` 28, `cad.sections.query` 24 (`scripts/fixtures/section_command_cases.py`,
  KentOS kodu olmadan): doğrula, plan, yaz, geri al, yinele; Yeniden çiz; kesitin bloğunun kimliği (SHA-256 kuralı),
  adı, yerleşimleri, çizilmeyenler (eğri duvar, rampa, lentosuz kapı, plaksız merdiven, niş); güncel, eski,
  çıkarılmamış, kaynağı yok; her ret kodu ve iletisi. Çizilen nesneler orada karşılaştırılmaz (çekirdeğin kendi
  durumları onları sınar). Yazıcının ilk sürümü A-A'nın bakış yönüne taşınan pencereyi "eski" sanıyordu; pencere
  kesilen kuzey duvarının arkasında kalıyor ve çekirdek doğru biçimde "güncel" diyordu: durum belirsiz olmayan bir
  taşımayla yazıldı.
- **Uygulama** (`fixtures/arch/v1/section-building.kcad`, iki katlı 12 × 9 m bina, KentOS kodu olmadan yazıldı; 77
  nesne): B-B bloğu `MM-XXXX-KESITB`, sözleri (başlık, ölçek, üç kot, iki katta "Merdiven holü"), EK-4 katmanları
  (`-T`, `-1`…); merdiven kolu boyuna kesitte dokuz basamak; aks balonları tek sırada, mahal adının içinden aks geçmez;
  giriş kapısı 10 cm yükselince eski, Yeniden çiz'le güncel, ikinci Yeniden çiz "değişiklik yok"; red kodları. 6 test,
  0,16 s.
- **Gerçek pencere** (Xvfb 1920 × 1080, gerçek fare ve klavye, sahibin ayarlarının kopyası; `.run/gui-evidence/mimari-m42/`):
  - Önce (M41 ikilisi): menü aramasında "Kesit çıkar" → "menülerde yok"; plandaki kesit çizgisi düz.
  - Sonra: Alt+Q "Kesit çıkar" → B-B çizgisine tıkla → önizleme "B-B kesiti · çizilmeyen: çatı, tabii zemin" →
    tıkla: nesne 77 → 78, blok 148 parça. Giriş kapısı Öznitelikler'de Konum 6,20 → 5,00: durum çubuğunda "B-B kesiti
    eski" → tıkla: kesit seçilir, Durum "Eski: plan değişti, yeniden çizin" → Yeniden çiz: kesitte kapı boşluğu kapandı
    (148 → 140 parça), Durum "Güncel", uyarı kalktı. Bütün senaryo (ekran görüntüleri için yakınlaştırmalar dahil) KLM'ye göre 21 tuş, 19 işaretleme, 38 düğme/tekerlek
    basışı: 35,4 s.
  - Bu turda bulunup düzeltilenler: aks balonlarının merdiven gibi aşağı inmesi; mahal adının aksın ve merdivenin
    üstüne düşmesi; rıhtın üstünden geçen kesitte basamak burnunun taramayı bölmesi; kotların binadan 2,4 m uzakta
    kalması.

## Sonraya kalan

Kırıklı kesit, rampa, çatı (ADV-20), tabii zemin, paftaya KS olarak yerleştirme ve iki kesit denetimi (2. bölüm), görünüş
(M43), bağı koparma (Patlat bugün yapar).
