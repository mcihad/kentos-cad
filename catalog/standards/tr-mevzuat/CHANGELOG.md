# Değişiklikler

## 0.5.0

- **Yüksek tehlikeli yerler** (BYKHY md. 8/1-ğ ve 17): KentOS olgularında bölümün niteliğinden ya da mahalin adından
  okunur (LPG, tüp deposu ve bayii, akaryakıt istasyonu, doğalgaz dolumu, parlayıcı ve patlayıcı madde, barut, yanıcı
  sıvı; binanın kendi yakıt deposu 56. maddenindir, sınıf değildir). `kacis-uzakligi` onları Ek-5/B'nin satırıyla
  (tek yön 10/20 m, iki yön 20/35 m) zaten ölçüyordu; artık olgularda görünüyorlar.
- **`cikmaz-koridor`**: koridorun sınıfı kaçış uzaklığındaki gibi okunur (bölümünün; bölümü yoksa kendi adının, o da
  yoksa binanın en ağır sınıfı); yüksek tehlikeli yerin koridorunda yağmurlama sistemi yoksa sınır 10 m (öbür bütün
  sınıflarda 15 m, yağmurlamayla hepsinde 20 m). Bulgu satırı ve sınıfın nereden okunduğunu söyler, madde 17'yi anar.
  İki yeni örnek: tüp bayiinin 12 m'lik çıkmaz koridoru yağmurlamasız hata, yağmurlamayla bulgusuz.

## 0.4.1

- **`kacis-uzakligi`**: bir bölüme ait olmayan mahal (ortak alan, bodrumdaki otopark ya da depo) yerel denetim gibi
  önce kendi adının sınıfıyla ölçülür (olguların yeni `RoomFacts.occupancy`'si: Otopark, Depo, Lokanta …), o da yoksa
  binanınkiyle; bulgu sınıfın mahalin adından okunduğunu söyler. Yalnız konut olan binanın bodrum otoparkı artık
  ölçülür. Sekizinci örnek: 27 m'lik tek yönlü otopark yürüyüşü, sınır 15 m.

## 0.4.0

- **`kacis-uzakligi`** (yeni): konut olmayan bölümlerde kaçış uzaklığı (BYKHY 32/3-4, Ek-5/B). Mahalin en uzak
  noktasından en yakın kat çıkışına yürüyüş bölümün kullanım sınıfının satırına, projenin yağmurlama ayarının
  sütunuyla: tek çıkışta bütün yol tek yön, iki çıkışta önce tek yönlü kısım, sonra bütün yol iki yön sınırına; aşarsa
  hata. Sınıf olguların `occupancy`'sidir (bölümün niteliğinden: Büro, Dükkân, Otel …); söylemeyen bölüm binanın
  bilinen en ağır sınıfıyla, o da yoksa büro sayılır ve bulgu bunu söyler. Endüstri yapısında dipnot (1)'in ½ artışıyla
  uygun olan "karar verilemedi". Konutun ve yalnız konut olan binanın mahalleri okunmaz. Yedi örnekle.

## 0.3.1

- **Her kuralın örnekleri** (`audits/<kural>.examples.json`, 20 örnek): kural, mimari olgular veri olarak verilip
  sınanır. PAİY 29/1: 8 m² yatak odası hata, tablonun en azındaki piyesler bulgusuz. 29/3: 1.00 m'lik hol hata, 1.20
  m'lik hol (sınırda) bulgusuz. BYKHY 4/i: 18 m çıkmaz koridor yağmurlamasız hata, yağmurlamayla (20 m) ve 12 m'de
  bulgusuz. 48/2: salondan kapıya 24 m hata, 18 m bulgusuz, kapıya yolu olmayan mahal hata, 4 katlı konut binasında
  (48/1) bulgusuz; iki katlı dairenin tek kapısı üst katta hata, üst kat net 72 m² hata, 65 m² bulgusuz. 48/3-4:
  kapıdan tek yönde 22 m yağmurlamasız hata, yağmurlamayla (30 m) bulgusuz, çıkışa ulaşmayan kapı hata. 31/3 ve
  33/6-b: kat çıkışı olmayan kat hata, çıkışa ulaşmayan mahal hata, hepsi ulaşınca bulgusuz. Eklentiler › Örnekleri
  çalıştır ve `kentos kek test` çalıştırır; kuralların kodu değişmedi.

## 0.3.0

- **`cikmaz-koridor`** (yeni): çıkmaz koridor (BYKHY 4/i, Ek-5/B). Türü koridor olan mahalin yürüyüşünün tek yön payı
  (en uzak noktasından bir çıkışa ya da iki yönün ayrıldığı noktaya) projenin yağmurlama ayarının sınırına: 15 m,
  yağmurlama sistemiyle 20 m; aşarsa hata. Yalnız konut bölümleri olan binada apartmanlar satırı ve 48/1'in muafiyeti;
  öbür binalarda Ek-5/B'nin yüksek tehlikeli yerler dışında her sınıfta aynı olan sınırı. Konutun kendi holü okunmaz.
- **`ara-katli-daire`** (yeni): BYKHY 48/2'nin ikinci ve üçüncü cümleleri. İki katlı konut bölümünde tek kapı üst
  katta olamaz; üst katta kapı yoksa üst katın döşeme alanı 70 m²'yi aşamaz (net alanı aşıyorsa hata, yalnız brütü
  aşıyorsa "karar verilemedi"). Üç ve daha çok katlı dairede karar verilmez.

## 0.2.0

Olgular artık mahalin türünü ve dar kenarını, projenin yağmurlama ayarını ve konut kapısının kaçış merdiveni mahaline
açılıp açılmadığını veriyor (KentOS'un ADR 8057 §53'ü); kurallar bunlarla karar verir.

- **`piyesler`** (yeni; 0.1.0'ın `oturma-odasi` kuralının yerine): PAİY 29/1'in beş piyesi. Türü bilinen her piyesin
  net alanı ve dar kenarı tablonun en azına (hata, iki ölçüyü de söyler). Eksik piyes yönetmeliğin istisnalarıyla:
  3 veya daha az odalı konutta tuvaleti de karşılayan banyo (29/2, 29/4: en az 4.20 m²), mutfak nişi (29/4: odanın
  alanı ile 3.30 m²'nin toplamı), yatak nişi (5/25); istisna mümkünse "karar verilemedi" (bilgi), değilse hata.
  Piyeslerinin hiçbirinin türü okunamayan konutta 0.1.0'ın 12 m²'lik piyes denetimi sürer.
- **`koridor`** (yeni): hol ve koridor 1.20 m'den dar olamaz (PAİY 29/3).
- **`daire-ici`, `daire-kapisi`**: sınır projenin yağmurlama ayarınındır, aşılırsa hata; ayarı vermeyen eski bir
  KentOS'un olgularında iki sınır (uyarı ve hata) sürer.
- **`daire-kapisi`**: kaçış merdiveni mahaline açılan konut kapısı uygundur; hiçbir kat çıkışına ulaşmayan kapı her
  binada hata (BYKHY 48/3, 33/6-b).

## 0.1.0

- İlk sürüm, mimari çizimin ölçülmüş olgularıyla karar verilen dört kural:
  - Oturma odasının alanı: konut bağımsız bölümünde net alanı 12.00 m²'ye ulaşan piyes yoksa (PAİY 29/1, 5/25).
  - Daire içinde kapıya uzaklık: konutun içinde kapıya 20 m, yağmurlama sistemiyle 30 m (BYKHY 48/1, 48/2).
  - Daire kapısından kat çıkışına: Ek-5/B'nin apartmanlar satırı (BYKHY 32/3, 48/1, 48/4).
  - Kat çıkışına erişim: kat çıkışı olmayan kat, kendi bölümünden ve ortak alanlardan çıkışa ulaşmayan mahal
    (BYKHY 31/3, 33/6-b).
