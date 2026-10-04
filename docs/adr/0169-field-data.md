# ADR 0169: Saha verisi: ham ölçü dosyaları, karne indirgemesi ve alete gönderme

- **Durum:** kabul edildi (2026-10-04). `HYB-13`. Sahibin kararları (4 Ekim): dört total station biçimi (Leica GSI, Topcon ve Sokkia, genel CSV/TXT karne, Trimble ve Nikon) ile GNSS'in GPX ve NMEA'sı; toleranslar proje ayarıdır, varsayılan değerlerini sahip verir; kot farkına yer eğriliği ve refraksiyon düzeltmesi uygulanır, k = 0,13 proje ayarıdır; aplikasyon değerleri alete okunan biçimlerle ve CSV olarak gönderilir.
- **Tarih:** 2026-10-04
- **Bağlam belgesi:** TODOS.md `HYB-13` (ilgili: `GIS-05`, `HYB-14`, `NUM-07`), ADR 0070 ve 0071 (Hesap pencereleri: Kutupsal alım, Poligon hesabı, Aplikasyon), ADR 0075 (ölçü tablosu), ADR 0152 (ölçü noktası: ad, kod, kot), ADR 0153 (nokta editörü), ADR 0048 (koordinat listesi al ve ver), ADR 0167 (WGS 84'ten projenin sistemine dönüşüm, doğruluğuyla), ADR 0009 (biçim okuyucularının kuralları); Netcad Netveri Koordinat Editörü, Takeometrik Hesap (MIR), Yatay Kenar Düşey Açı (YDE), Karne Editörü, GPS Aracı, NETPRO Ölçüm Cihazına Aktar; ArcGIS traverse dosyası; QGIS Convert GPS data; üreticilerin biçim belgeleri (Leica GSI Online, Sokkia SDR33, Topcon GTS-7, Trimble JobXML, Nikon RAW), NMEA 0183, GPX 1.1.

## Bağlam

Kutupsal alım, Poligon hesabı ve Aplikasyon (ADR 0070, 0071) ölçüleri tablolarına elle ya da elektronik tablodan yapıştırarak alır. Saha ise ölçüleri aletin kendi dosyasında getirir: total station'ın ham gözlemleri (istasyon, alet ve prizma yüksekliği, yatay ve düşey açı, eğik uzunluk; çoğu kez iki durumda) ve GNSS alıcısının konumları (WGS 84'te enlem, boylam, yükseklik, kalite). Ölçmeci bunları bugün KentOS dışında indirger: iki durumu ortalar, eğik uzunluğu yataya ve kot farkına çevirir, kenarları iki yönden karşılaştırır, sonra hesaba yazar. Aplikasyon değerleri de alete elle girilir.

§23'ün kuralları burada da geçerlidir: kaynak değerler (okumalar, birimleri, yükseklikler) korunur, indirgemenin her adımı ve sabiti görünür, tolerans aşımı sessizce örtülmez, belirsiz kural (birimi bilinmeyen sözcük, eşleşmeyen durum) tahmin edilmez, söylenir.

## Karar

### 1. Okunan biçimler

- **Leica GSI-8 ve GSI-16:** sözcük sözcük (sözcük numarası, bilgi haneleri, birim hanesi, işaret, değer); nokta adı, yatay ve düşey açı, eğik ve yatay uzunluk, yükseklik farkı, kodlar, alet ve prizma yüksekliği, istasyon ve hedef koordinatları. Birim ve ondalık, sözcüğün birim hanesinden Leica'nın belgesine göre; tanınmayan birim söylenir, tahmin edilmez.
- **Sokkia SDR33 ve Topcon GTS-7:** kayıt türleriyle (istasyon, prizma yüksekliği, geri bakış, koordinat, gözlem); açı ve uzunluk birimleri dosyanın başlığından.
- **Trimble JobXML ve Nikon RAW:** istasyon kurulumu, geri bakış ve gözlem kayıtları; ham gözlemler (indirgenmiş değerler değil) okunur.
- **Genel CSV/TXT karne:** sütunları kullanıcının eşlediği düz metin (istasyon, nokta, Hz, V, eğik uzunluk, alet ve prizma yüksekliği, kod); eşleme pencerede, son eşleme hatırlanır.
- **GNSS:** GPX 1.1 (yol noktaları ve iz noktaları: enlem, boylam, yükseklik, ad, zaman) ve NMEA 0183 (GGA: konum, çözüm türü, uydu sayısı, HDOP, yükseklik; RMC zaman için). Konumlar WGS 84'tedir: projenin sistemine ADR 0167'nin dönüşümüyle, doğruluğu ve dayanağı yazılarak gelir; projenin sistemi yoksa içe aktarma kapalıdır, nedeni söylenir (CLAUDE.md §5). Yükseklik elipsoit yüksekliğidir; ortometrik yüksekliğe çevrilmez (`NUM-07`), bu yazılır.
- Bozuk ya da kötücül dosyada panik yoktur; boyut sınırı, satır ya da kayıt numarasıyla neden söylenir (ADR 0009).

### 2. Karne

- Okunan gözlemler **karne** olur: istasyonlar ve her istasyondaki gözlemler (geri bakış, ileri bakışlar, iki durumu eşleşmiş ya da tek); kaynak dosyanın adı ve satırı her gözlemde kalır.
- Karne editörü penceresinde görülür, düzeltilir (bir gözlem kullanılmaz, bir nokta adı düzeltilir), indirgenir ve hesaba aktarılır. Ham ölçülerin projeye (`.kcad`'e) yazılması sonraki karardır (§7); hesaplanan noktalar ölçü noktası olarak (ADR 0152) çizime, kaynakları özniteliklerinde gelir.

### 3. İndirgeme

- **İki durum:** yatay açıda II. durum okuması yarım tur farkla I. durumla ortalanır; düşey açıda indeks hatası (I + II − tam tur) / 2, ortalama zenit açısı I − indeks hatası. Durum farkları ve indeks hatası gösterilir.
- **Uzunluk ve kot farkı:** yatay uzunluk eğik uzunluk × sin(zenit); kot farkı eğik uzunluk × cos(zenit) + (1 − k) · yatay uzunluk² / (2R) + alet yüksekliği − prizma yüksekliği; R yerin ortalama yarıçapı (6 371 000 m), k proje ayarıdır (varsayılan 0,13). Kutupsal alım da kot farkına aynı düzeltmeyi projenin k'sıyla uygular: karneden aktarılan gözlemin kotu karnedekiyle aynı çıkar (ADR 0071'in “uygulanmaz”ı bununla değişti). Projeksiyon ve yükseklik düzeltmeleri (zemin, ölçek faktörü) bu ADR'de değildir (`HYB-14`).
- **Toleranslar:** proje ayarları (iki durumda yatay açı farkı, indeks hatası ve eğik uzunluk farkı; bir kenarın iki yönden uzunluk farkı, poligonun açı ve koordinat kapanması Poligon'a aktarmayla, 4. adımda). Varsayılan değerlerini sahip verir; verilene dek boştur: denetim yapılmaz, farklar yine gösterilir. Aşan değer karnede uyarı rengindedir; hesaba aktarmayı durdurmaz, aktarılan sonucun raporunda yazılır.
- **Hesaba aktarma:** bir istasyonun indirgenmiş gözlemleri Kutupsal alım'a (istasyon, geri bakış, yatay açılar, yatay uzunluklar ve kot farkları), istasyonlar zinciri Poligon hesabı'na (kırılma açıları, iki yönden ortalanmış kenarlar) tek adımda, pencere doldurulmuş açılır; hesap ve çizime yazma bugünkü pencerelerin işidir.

### 4. Alete gönderme

- Aplikasyon noktaları (Aplikasyon penceresinin tablosu ya da seçili noktalar) aletin içe aldığı koordinat dosyası olarak yazılır: okunan biçimlerin koordinat kayıtlarıyla (GSI, SDR33, GTS-7, JobXML, Nikon) ve CSV. Ad, kod ve kot taşınır; biçimin taşıyamadığı (ad uzunluğu, karakter) söylenir, sessizce kırpılmaz.

### 5. Ortak çekirdek ve başvuru

- Okuyucular `crates/shared/formats`'ta (yeni `field` modülü), indirgeme `geometry-core`'un `survey`'inde; web'e WASM ile, iki platform aynı kodla.
- **Bağımsız başvurular:** biçimlerin belgelerinden, KentOS kodu olmadan yazılmış Python okuyucuları ve elle yazılmış örnek dosyalar (her biçimde iki durum, tek durum, birimler, bozuk kayıtlar); indirgeme mpmath'le 50 basamak. Örnek dosyalar `fixtures/field/v1`'de.

### 6. Arayüz

- **Karne editörü** (iki platformda): dosya aç (biçim içerikten tanınır; tanınmazsa sorulur), istasyonlar ve gözlemler tablosu, iki durumun eşleşmesi, farklar ve toleranslar, Kullan; Kutupsal alım'a ve Poligon hesabı'na aktar; noktaları çizime yaz (tek geri alma adımı).
- **GNSS:** içe aktarmanın biçimleri arasında GPX ve NMEA: noktalar ölçü noktası olarak, çözüm türü, uydu sayısı, HDOP ve zaman öznitelik olarak; dönüşümün doğruluğu raporda.
- **Alete gönder:** Aplikasyon penceresinde ve nokta editöründe (ADR 0153) biçim seçilerek dosyaya.

### 7. Kapsam dışı ve sahibin kararları

- **[M] Tolerans değerleri:** sahip verir (BÖHHBÜY ya da kurumun değerleri).
- RINEX ve sonradan işleme, geoit ve ortometrik yükseklik (`NUM-07`), zemin ve projeksiyon indirgemesi (`HYB-14`), ham ölçülerin projeye kaydı, aletle doğrudan bağlantı (seri port, Bluetooth).

### 8. İş sırası

1. Çekirdek: karne modeli ve indirgeme (iki durum, uzunluk, kot farkı, k); genel CSV/TXT karnenin okuyucusu; bağımsız başvuru.

   *(4 Ekim: tamam.)* İndirgeme çekirdekte `survey::fieldbook::reduce` (web'e `fieldReduce`, `model/geom/surveyCalc.ts`): yüzler başucu açısından (yarım turdan küçük I, büyük II; 0, yarım ve tam tur gözlem değildir, söylenir), aynı hedefin ilk eşlenmemiş I. ve II. durumu sırayla çift olur; çiftte II. durum okuması yarım tur döndürülür, fark (−yarım, yarım] aralığına sarılır, ortalama I − fark/2; indeks hatası (I + II − tam tur)/2, ortalama başucu I − indeks; eğik uzunluk ikisinin ortalaması (farkıyla), prizma yüksekliği I. durumunki; tek II. durum I'e çevrilir; başucu açısı olmayan gözlem yalnız doğrultudur (geri bakış), çift olmaz, uzunluğa çevrilmez; yatay uzunluk S·sin Z, kot farkı S·cos Z + (1 − k)·D²/2R + alet − prizma, R = 6 371 000 m. Bağımsız başvuru `scripts/fixtures/field_reduce_cases.py` (`fixtures/field/v1/reduce.json`, 10 durum, mpmath 50 basamak; açı 1e-11, metre 1e-9). Okuyucu `kentos_formats::field::read_csv` (web'e biçim işçisiyle `readFieldCsv`, sözleşmeler `FieldCsvOptions`, `FieldBookRead`, `FieldStation`, `FieldObservation`; `FORMATS_VERSION` 19): kodlama koordinat listesindeki gibi sezilir; ayırıcı ilk satırın en sık sekme, noktalı virgül ya da virgülü; sayılar Hesap pencerelerinin okuyuşuyla (ayırıcı virgül değilse ilk virgül nokta); istasyon sütunu yeni istasyonu başlatır, prizma yüksekliği boşsa istasyonun sonuncusu; bozuk alet yüksekliği satırı, gözlemin bozuk değeri yalnız noktayı dışarıda bırakır, yatay açısız nokta okunmaz, hepsi satırıyla söylenir. Bağımsız başvuru `scripts/fixtures/field_csv_cases.py` (`fixtures/field/v1/csv.json`, 5 durum: noktalı virgül ve virgüllü ondalık, virgül, sekme, istasyonsuz, bozuk değerler). İki platform aynı dosyaları geçer (`geometry-core/tests/field_reduce.rs`, `formats/tests/field_csv.rs`; web `wasm/fieldReduce.wasm.test.ts`, `io/formats.wasm.test.ts`).
2. Leica GSI-8 ve GSI-16 okuyucusu; başvuru ve örnek dosyalar.

   *(4 Ekim: tamam.)* Okuyucu `kentos_formats::field::gsi::read` (web'e biçim işçisiyle `readFieldGsi`; `FieldBookRead`'in `unit`'i
   karnenin açı birimi, ilk açınınki; `FieldStation`'ın `east`, `north`, `height`'ı istasyonun koordinatları; `FORMATS_VERSION` 20).
   Her dolu satır bir bloktur, `*` ile başlayan GSI-16; sözcüğün ilk iki karakteri numarası, altıncısı birimi, yedincisi işareti,
   kalanı değeridir; sekiz karakterden kısa ya da işaretsiz sözcük bloğu okutmaz. Açılar 2 (gon) ve 3 (ondalık derece) beş ondalıkla,
   4 derece, dakika, saniye ve saniyenin onda biri; karnenin birimi ilk açınınki, başka birimdeki açı ona çevrilir; tam dönüşten büyük
   açı, mil ve ayak okunmaz, söylenir. Uzunluklar 0 (mm), 6 (0,1 mm) ve 8 (0,01 mm). Her değer tam ondalık ya da altmışlık değerinin
   en yakın float64'üdür (bir kez yuvarlanır). WI 21'li blok gözlemdir: nokta WI 11 (yoksa okunmaz), başucu 22, eğik uzunluk 31,
   prizma yüksekliği 87 (yoksa istasyonun sonuncusu), kod 71; eğik uzunluk yok da WI 32 varsa söylenir, gözlem doğrultudur. WI 21'siz,
   84–86 ya da 88'li blok istasyondur (adı WI 16, yoksa 11); kod ve ayar blokları geçilir; istasyondan önceki gözlem adsız ilk
   istasyonundur. Bağımsız başvuru `scripts/fixtures/field_gsi_cases.py` (`fixtures/field/v1/gsi.json`, Leica'nın “GSI ONLINE for Leica
   TPS and DNA” belgesinden; 4 durum: GSI-8 istasyon ve iki durum, gon ve mm; GSI-16 derece, DMS ve gon karışık, 1/100 ve 1/10 mm, CR LF;
   ayak; bozuk sözcük, numarasız gözlem, mil, yalnız yatay uzunluk, kod bloğu, tam dönüşten büyük açı, eksi değerler). İki platform aynı
   dosyayı geçer (`formats/tests/field_gsi.rs`; web `io/formats.wasm.test.ts`).
3. Karne editörü iki platformda; Kutupsal alım'a aktarma; proje ayarları (k, toleranslar). Üç parçada: 3a proje ayarları ve Kutupsal
   alım'ın k'sı, 3b Karne editörü, 3c Kutupsal alım'a aktarma.

   *(4 Ekim: 3a tamam.)* Sözleşmede `ProjectSettings::survey` (`SurveySettings`: `refraction`, `faceHz`, `index`, `faceSlope`; açılar
   radyan, uzunluk metre; `sanitized` yalnız tutanı ve varsayılandan başka k'yı tutar, `refraction()` yoksa 0,13), `.kcad` şema 14
   (spesifikasyon §6.4.2; kodek, bağımsız Python okuyucusu ve yazıcısı, `survey.kcad` örneği, beş bozuk dosya; `schema-version-14`
   `schema-version-15` oldu). Proje ayarları'nda Ölçme bölümü iki platformda (masaüstü `project/survey.rs`, web
   `ProjectSettingsDialog.ts`'in `surveySection`'ı): Refraksiyon katsayısı (k) ve üç tolerans; toleransların açıları gon projesinde cc,
   derece projesinde ″ ile, uzunluk mm ile yazılır; tutmayan alanın altında nedeni, Kaydet bekler. Form kuralları
   `kentos_project::survey_form` ve `model/surveyForm.ts`, ortak durumlar `fixtures/project/v1/survey-form.json` (bağımsız başvuru
   `scripts/fixtures/survey_form_cases.py`). Kutupsal alım kot farkına projenin k'sını uygular (çekirdek `PolarInput::refraction`,
   `survey::fieldbook::curvature`; başvuru `geometry_call_reference.py`'nin k'lı durumu), açıklaması k'yı söyler.

   *(4 Ekim: 3b tamam.)* Çekirdek: indirgeme her gözlemin durumunu (I, II, doğrultu ya da durum değil) söyler ve her çifti projenin
   toleranslarıyla karşılaştırır (kitabın birimine çevrilmiş; aşan fark satırın `over`'ında: `faceHz`, `index`, `faceSlope`; eşit aşmaz);
   `fieldReduce` toleransları dördüncü değer alır. Biçim içerikten tanınır (`formats::field::sniff`: ilk dolu satırın ilk sözcüğü GSI
   sözcüğüyse Leica GSI) ve tek giriş okur (`field::read`, web'de `readFieldBook`, `FORMATS_VERSION` 21; `readFieldCsv` ve
   `readFieldGsi`'nin yerine): GSI olduğu gibi, metin karne eşlemesiyle ya da eşlemesiz yalnız ilk satırının hücreleri; `FieldBookRead`
   biçimini söyler. Bağımsız başvurular `field_sniff_cases.py` (13 durum) ve ilk satırıyla `field_csv_cases.py`; indirgemenin başvurusu
   durumları ve toleransları da hesaplar (iki yeni durum). Karne editörü iki platformda (masaüstü `calc/fieldbook/`, web
   `ui/calc/FieldBookDialog.ts`; Hesap › Saha › Karne, CAD'de Giriş › Ölçme ▾, `calc.fieldbook`): Dosya aç (GSI içeriğinden; metin
   karnede Sütunlar: sekiz alan, İlk satır başlık, Açı birimi; Nokta ve Yatay açı zorunlu, eşleme oturum boyunca hatırlanır), İstasyon
   seçimi, Alet yüksekliği, dosyadaki koordinatlar; Gözlemler tablosu (Kullan, Nokta düzeltilir; Durum, Yatay açı, Başucu açısı, Eğik
   uzunluk, Prizma, Kod, Satır), İndirgenmiş tablo (farklar cc ya da ″ ve mm ile; toleransı aşan kırmızı), özet (okunmayan satırlar,
   durum olmayan gözlemler, toleranslar ve aşımlar, k), Raporu kopyala. Resimler ve akış testleri `fixtures/field/v1/sample.gsi` ile
   (`field_sample_gsi.py`'nin elle yazılmış GSI-16 karnesi; 5. adımda `field_samples.py`, her biçimde aynı karne).

   *(4 Ekim: 3c tamam; 3. adım bitti.)* Kutupsal alım'a aktarma: çekirdekte `survey::fieldbook::polar_transfer` (web'e `fieldPolar`):
   seçilen indirgenmiş satır geri bakıştır (hedefi ve okuması yöneltme), eğik uzunluğu ve başucu açısı olan her öbür satır bir
   nokta (adı, okuması, eğik uzunluğu, başucu açısı, prizma yüksekliği), olmayanlar adlarıyla dışarıda kalır; açılar karnenin
   biriminden projeninkine çevrilir. Bağımsız başvuru `field_reduce_cases.py`'nin her durumunda iki birim için (`polar`). Karne
   editöründe istasyonun yanında Geri bakış (indirgenmiş satırlar, ilki), altta Kutupsal alım'a aktar: istasyon çizimde adıyla varsa
   adıyla, yoksa dosyadaki koordinatlarıyla, istasyon kotu dosyanınki, alet yüksekliği yazılan; değerler açılarda 8, uzunluklarda 6
   ondalıkla yazılır (aletin çözünürlüğünün çok altında); Kutupsal alım açılır, karne kalır; kaç nokta aktarıldığı ve aktarılmayan
   doğrultular söylenir. Kutupsal alım aynı k'yı uyguladığı için kot farkları karnedekilerle aynıdır (masaüstü testi 1e-6 m).
4. Poligon hesabı'na aktarma (iki yönden kenarlar, kapanmalar). İki parçada: 4a aktarma, 4b toleranslar (iki yönden uzunluk farkı,
   poligonun açı ve koordinat kapanması; `.kcad` şema 15).

   *(4 Ekim: 4a tamam.)* Çekirdekte `survey::fieldbook::traverse_transfer` (web'e `fieldTraverse`): poligon karnenin istasyonlarından
   sırayla geçer (iki ve daha çok istasyon); ilk istasyon kendi geri bakışına, sonuncusu seçilen ileri bakışa yöneltilir (yoksa
   orada açı yok). Her istasyonda kırılma açısı ileri hedefin okuması eksi geri hedefinki, [0, tam tur); hedef istasyonun o adlı
   ilk indirgenmiş satırıdır (önceki ve sonraki istasyon). Kenar ST(i) → ST(i+1): ileri yatay uzunluk ST(i)'de, geri ST(i+1)'de
   (doğrultu satırının uzunluğu yoktur); ikisi varsa ortalaması ve farkı, biri varsa o; hiçbiri yoksa kenar eksik diye söylenir;
   bulunmayan hedefler de (istasyon, hedef) söylenir; açılar projenin birimine çevrilir. Bağımsız başvuru
   `scripts/fixtures/field_traverse_cases.py` (`fixtures/field/v1/traverse.json`, 6 durum; indirgemenin mpmath başvurusuyla).
   Karne editöründe (iki ve daha çok istasyonda) Poligon bölümü: istasyon zinciri, Bitişte bakılan (son istasyonun satırları ya da
   —), Kenarlar tablosu (İleri, Geri, Ortalama, Fark); her istasyonun Geri bakış'ı kendisinindir, ilkinki poligonun da başlangıçta
   bakılan noktasıdır. “Poligon hesabı'na aktar” Poligon hesabı'nı Bağlı doldurur (bitişte yöneltme ileri bakış seçildiyse):
   başlangıç ve bitiş istasyonları çizimde adlarıyla varsa adlarıyla, yoksa dosyadaki koordinatlarıyla; ara istasyonlar yeni
   noktalar; açılar 8, kenarlar 6 ondalıkla; bulunmayan gözlemler söylenir. Örnek `sample.gsi` iki istasyonlu bir poligon oldu.

   *(4 Ekim: 4b tamam; 4. adım bitti.)* Proje ayarları › Ölçme'de Poligon grubu: Kenarın iki yönden farkı (mm), Açı kapanması (cc ya
   da ″), Koordinat kapanması (mm); `.kcad` şema 15 (`twoWay`, `traverseAngle`, `traverseCoord`; şema 14'te bu anahtarlar ve sıfır ya
   da eksi tolerans reddedilir; bağımsız Python okuyucu ve yazıcısıyla). Çekirdekte kenarın iki ucundan ölçülen yatay uzunlukların
   farkı `traverse_transfer`'in toleransıyla denetlenir (`BookLeg.over`: |fark| > tolerans), poligonun kapanmaları
   `survey::traverse::closure` ile (web'e `surveyTraverseClosure`): açı kapanması birimine çevrilen toleransı aşınca (|fβ| > t), fs
   toleransı aşınca; eşit olan içindedir, kapanması olmayan (açık poligon, bitişte yöneltme yok) denetlenmez. Bağımsız başvuru
   `field_traverse_cases.py` (kenarların `over`'ı 2,5 ve 5 mm'lik toleranslarla; `closures`, iki birimde ve sınırda). Karne editöründe
   aşan kenarın Fark'ı uyarı rengindedir, altta toleranslar ve “n kenarda iki yönden fark toleransı aşıldı.”; Poligon hesabı'nda
   kapanma satırlarının altında toleransla karşılaştırma (aşıyor uyarı, içinde bilgi), raporun kapanma satırları Tolerans, değeri ve
   aşıyor ya da içinde ile biter; tolerans verilmediyse “Hata sınırı verilmedi (Proje ayarları › Ölçme) …”. Aktarma toleransla durmaz.
5. Sokkia SDR33 ve Topcon GTS-7; Trimble JobXML ve Nikon RAW. Dört parçada (5a SDR, 5b GTS-7, 5c JobXML, 5d Nikon RAW), her biri
   üreticinin belgesinden yazılmış bağımsız başvurusuyla; örnek karne tek gözlem tablosundan her biçimde yazılır (`field_samples.py`),
   her biçimin örneği GSI örneğiyle aynı karneyi verir (testleri değer değer).

   *(4 Ekim: 5a tamam.)* Sokkia SDR: kaynak “Interfacing with the SOKKIA SDR Electronic Field Book” (yazılım 04-04.xx, Sokkia
   Technology, Ekim 1999), bölüm 3'ün kayıt düzenleri (3.6.1 SDR2x, 3.6.2 SDR33). Çekirdekte `formats::field::sdr` (web'e
   `readFieldBook`, biçim `sdr`; `FORMATS_VERSION` 22): satırlar sabit alanlıdır (sondaki boşluklar kesilmiş olabilir), başlığın sürümü
   “SDR33” ile başlıyorsa 16 karakterlik alanlar, değilse SDR2x'in 4 haneli numaraları ve 10 karakterlik sayıları; işin birimleri
   başlıkta (açı 1 derece, 2 gon; uzunluk 1 metre; açı yönü 1): mil, ayak, başka yön ve karnenin biriminden başka birimli iş söylenir,
   okunmaz. Sayılar belgenin biçimiyle (isteğe bağlı eksi, rakam, nokta ve rakam), ondalığın en yakın float64'ü; açı sıfırla tam dönüş
   arasında; eğik uzunluk eksi olamaz. İstasyon (02) koordinatları ve alet yüksekliğiyle, prizma yüksekliği (03) iş boyunca sürer, ham
   gözlemler 09 F1, F2 ve MD; alet kaydının (01) düşey açı seçeneği 2 ise başucu = çeyrek tur − okuma (tam sayılarla, tek yuvarlama);
   alet kaydı yoksa düşey açılar başucu sayılır ve bir kez söylenir. Düzeltilmiş gözlemler (09MC) ve koordinat kayıtları (08) ham gözlem
   değildir: ilk satırlarında sayılarıyla bir kez söylenir. STX ve ETX çerçeve satırları atlanır. İçerikten tanıma: ilk dolu satırı GSI
   sözcüğü değilse, çerçeve olmayan ilk beş satırdan biri SDR başlığıysa (`00`, iki büyük harf, `SDR`). Bağımsız başvuru
   `field_sdr_cases.py` (`sdr.json`, 4 durum) ve 3000 rastgele karnede Rust ile birebir; tanıma `field_sniff_cases.py`'de. Karne
   editörü biçimi “Sokkia SDR” diye adlandırır, dosya süzgecinde `.sdr`; resimler `karne-sdr-*`, `fieldbook-sdr-*`.

   *(4 Ekim: 5b tamam.)* Topcon GTS-7: kaynak Topcon Link Reference Manual (P/N 7010-0522), Ek C “GTS-7 Raw Format” ve örnek
   dosyası. Çekirdekte `formats::field::gts7` (biçim `gts7`, `FORMATS_VERSION` 23): her satır bir denetim sözcüğü ve virgülle
   ayrılmış alanları; UNITS'in ilk alanı M (metre; F ve başkası okunmaz), ikincisi D (derece, DDD.MMSS: kesrin ilk iki rakamı dakika,
   sonraki ikisi saniye, kalanı saniyenin kesri; eksik rakamlar sıfır) ya da G (gon, ondalık); UNITS'ten önceki ölçüler bir kez
   söylenir, okunmayan ya da karnenin biriminden başka birimli UNITS'ten sonrakiler atlanır. STN istasyonu (alet yüksekliğiyle)
   başlatır, hemen ardından gelen XYZ (doğu, kuzey, kot) onun koordinatlarıdır; başka bir kayıttan sonraki XYZ bir noktanın
   hesaplanmış koordinatıdır, atlanır. BS, FS ve SS sonraki ölçülerin noktasını (prizma yüksekliği verilmezse istasyonun sonuncusu, FS
   ve SS'te kod) verir; HV ve SD bu noktanın gözlemidir (yatay açı, başucu, SD'de eğik uzunluk), HD indirgenmiş ölçüdür, söylenir;
   sıfır olmayan OFFSET önündeki gözlemi dışarıda bırakır, söylenir. Sayılar en çok 30 rakam; açılar kesin kesirden tek yuvarlamayla
   (`field::exact`), eksi yatay okuma turuna çevrilir (Topcon'un örneğindeki −37.2644 = 322°33'16"), tam dönüşten büyük açı ve eksi
   başucu okunmaz. İçerikten tanıma: boş olmayan ilk on satırdan biri `GTS-7` ile ya da UNITS veya STN sözcüğü ve virgüllü alanlarla
   başlıyorsa. Bağımsız başvuru `field_gts7_cases.py` (`gts7.json`, 3 durum; Topcon'un örneği dahil) ve 6000 rastgele karnede Rust
   ile birebir; örnek `sample.gt7` GSI örneğiyle aynı karne; dosya süzgecinde `.gt7`, `.raw`; resimler `karne-gts7-*`,
   `fieldbook-gts7-*`.

   *(4 Ekim: 5d tamam; JobXML'den, 5c'den önce.)* Nikon RAW: kaynak Total Station Nivo Series Instruction Manual (Nikon-Trimble),
   “Nikon raw record formats” ve “Data examples” (Nikon RAW data format V2.00). Çekirdekte `formats::field::nikon` (biçim `nikon`,
   `FORMATS_VERSION` 24): virgülle ayrılmış kayıtlar; birimler indirmenin yorum kayıtlarından: “CO,Dist Units:” (Met… metre; başkası
   okunmaz), “CO,Angle Units:” (DDDMMSS: DDD.MMSS derece; Gon, Gons, Grad, Grads: gon; Mils ve başkası okunmaz, tahmin edilmez),
   “CO,Zero VA:” (Zenith; Horizon: başucu = çeyrek tur − açı, kesin; Compass ve başkası okunmaz; yazılı değilse düşey açılar başucu
   sayılır, bir kez söylenir). Birimler yazılmadan önceki ölçüler bir kez söylenir. Koordinat kayıtları (UP, MP, CC, RE: kuzey, doğu,
   kot) bir adın son koordinatlarıdır; ST istasyonu (alet yüksekliği, geri bakış noktası) başlatır, koordinatları adının son kaydından.
   Gözlemler F1, F2 (nokta boşsa istasyonun geri bakışı), SS, CP, SO; prizma yüksekliği boşsa istasyonun sonuncusu, eğik uzunluk
   boşsa yalnız doğrultu. Sayılar ve açılar GTS-7'nin kurallarıyla (`field::exact`). İçerikten tanıma: boş olmayan ilk on satırdan
   biri `CO,` ile başlıyorsa. Bağımsız başvuru `field_nikon_cases.py` (`nikon.json`, 3 durum) ve 6000 rastgele karnede Rust ile
   birebir; örnek `sample-nikon.raw` GSI örneğiyle aynı karne; resimler `karne-nikon-*`, `fieldbook-nikon-*`. Kılavuzun “Data
   examples” örneği kendi içinde tutarsızdır (SS'in değerleri DDD.MMSS değil), örnek olarak alınmadı.
6. GNSS: GPX ve NMEA, WGS 84'ten projenin sistemine.
7. Alete gönderme: okunan biçimlerin koordinat kayıtları ve CSV.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Saha dosyası KentOS'ta okunur, indirgenir ve hesaba aktarılır; Netcad'in Netveri, MIR, YDE, Karne Editörü ve GPS Aracı karşılanır.
- İndirgemenin sabitleri ve toleransları proje ayarıdır, her sonuçta yazılıdır.

## Doğrulama

- **Okuyucular:** biçim başına bağımsız Python okuyucusu ve elle yazılmış örnekler; iki platform (yerli ve WASM) aynı sonucu verir; bozuk dosyalar nedenleriyle reddedilir.
- **İndirgeme:** mpmath başvurusu (50 basamak); iki durum, uzunluk, kot farkı ve düzeltme terimleri.
- **Arayüz:** ortak izler ve resimler iki platformda.
