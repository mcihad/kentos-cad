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
4. Poligon hesabı'na aktarma (iki yönden kenarlar, kapanmalar).
5. Sokkia SDR33 ve Topcon GTS-7; Trimble JobXML ve Nikon RAW.
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
