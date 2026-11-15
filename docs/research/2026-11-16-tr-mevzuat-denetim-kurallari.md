# Türkiye mevzuatının ilk denetim kuralları: PAİY 29 ve BYKHY'nin kaçış uzaklıkları

Tarih: 16 Kasım 2026. Konu: `catalog/standards/tr-mevzuat` paketi (`tr.kentos.mevzuat` 0.1.0–0.3.0), plan/eklenti.md
ADIM 20'nin "ulusal/belediye kurallarının Python betikleri" maddesi. Kurallar yalnız KentOS'un Rust'ta ölçtüğü mimari
olguları (`facts.project.architecture`, ADR 8057 §39; 0.2.0'dan beri §53'ün mahal türü, dar kenar, yağmurlama ayarı ve
merdivene açılan kapı) okur; bu not hangi hükmün aynen ne dediğini, olguların hangisine karar verebildiğini ve neyin
neden dışarıda kaldığını yazar.

## Yöntem ve kaynaklar

- Resmî metinler **16 Kasım 2026**'da mevzuat.gov.tr'den PDF olarak indirildi, `pdftotext -layout` ile metne çevrildi;
  tablolar (PAİY 29/1, BYKHY Ek-5/B) ayrıca sayfa görüntüsüyle karşılaştırıldı. Aşağıdaki alıntıların her biri
  indirilen metinde aynen geçer (boşluklar tek boşluğa indirilerek denetlendi).
  - **PAİY** — Planlı Alanlar İmar Yönetmeliği, RG 3.7.2017/30113:
    <https://www.mevzuat.gov.tr/MevzuatMetin/yonetmelik/7.5.23722.pdf> (57 sayfa, SHA-256
    `4e94897ca8ac64deca17232c0498f24e0168ee4c283a1873f9a9e724b256cfd0`). Metin RG 14.1.2026/33137 değişikliğini
    işlemiş. Madde 29 sayfa 25'te, Madde 5/25 sayfa 10'da.
  - **BYKHY** — Binaların Yangından Korunması Hakkında Yönetmelik, BKK 27.11.2007/2007/12937, RG 19.12.2007/26735:
    <https://www.mevzuat.gov.tr/MevzuatMetin/21.5.200712937.pdf> (104 sayfa, SHA-256
    `d82d06d2d66410c3a23e86c44605355fb1282c85506a56544611fbe64a89a7d9`). Son değişiklik CK 30.6.2025/10026 (RG
    1.7.2025/32943, Geçici Madde 4). Ek-5/B sayfa 96'da. Metnin değişiklik tablosuna göre Madde 48 ve Ek-5/B
    2009/15316 sayılı BKK ile (yürürlük 9.9.2009), Ek-5/B ayrıca 2015/7401 sayılı BKK ile (yürürlük 9.7.2015)
    değişti; Madde 32 ve 33/6 ilk metindir.
- 0.2.0 için iki metin aynı gün (16 Kasım 2026, 01:31) yeniden indirildi: SHA-256'ları aynı, metin değişmemiş. 29/2,
  29/3, 29/4 ve BYKHY 48/3 bu metinden aynen alındı.
- Değerler KentOS Mimari'nin yerel denetimiyle karşılaştırıldı: `crates/native/application/src/arch/fire_audit.rs`
  (`Use::walks`, apartmanlar `[[15, 30], [30, 75]]`, 48/2'nin 20/30 m'si, 48/1'in kat sayımı) ve
  `regulation_audit.rs` (29/1'in 3.00 m ve 12.0 m²'si). Hepsi aynı.

## Okunan hükümler (aynen)

### PAİY

Madde 4/1-ğ-3 ve 4/1-nnn (tanımlar), kuralın "net alan" ve "piyes" dediği:

> Bağımsız bölüm net alanı: İçerden bağlantılı piyesleri ile birlikte bağımsız bölümün içerisindeki boşluklar hariç, duvarlar arasında kalan temiz alanı

> Piyes: Bağımsız bölüm içerisinde iç duvar veya bölme elemanları ile çerçevesi tanımlanmış ya da niş oluşturularak meydana getirilmiş, belirli bir işlev gören bölümü,

Madde 5 (Genel İlkeler), yirmi beşinci fıkra (ikinci cümlesi RG 13.8.2025/32985 ile eklendi):

> (25) Her müstakil konutta en az; 1 oturma odası, 1 yatak odası, 1 mutfak veya yemek pişirme yeri, 1 banyo veya yıkanma yeri ve 1 tuvalet bulunur.

> Parseldeki bağımsız bölüm sayısının %20’sinden fazla olmamak üzere oluşturulacak bağımsız bölümlerde; oturma odası dahilinde dar kenarı 2.50 metreden ve alanı 9 m2’den az olmamak üzere yatak nişi teşkil edilmesi halinde ayrıca yatak odası yapılmayabilir.

Madde 29 (Yapı piyesleri ve ölçüleri):

> MADDE 29- (1) Her müstakil konutta en az aşağıdaki piyesler bulunur:

| Piyes | Dar kenarı | Net alanı |
|---|---|---|
| 1 oturma odası | 3.00 metre | 12.0 m² |
| 1 yatak odası | 2.50 metre | 9.00 m² |
| 1 mutfak veya yemek pişirme yeri | 1.50 metre | 3.30 m² |
| 1 banyo veya yıkanma yeri | 1.50 metre | 3.00 m² |
| 1 tuvalet | 1.00 metre | 1.20 m² |

> (2) 3 veya daha az odalı konutlarda banyo/yıkanma yeri ile tuvalet aynı yerde düzenlenebilir.

> (3) Hol ve koridor genişlikleri 1.20 metreden az olamaz. (Ek cümle:RG-13/8/2025-32985) Yaşlı bakım kuruluşlarında hol ve koridorlarda yerden 80 cm yüksekliğinde küpeşte yapılır.

> (4) Mutfak nişi ve oda ile banyo ve tuvaletin aynı mekânda düzenlenmesi halinde her mekân için öngörülen en az alanların toplamı kadar alan düzenlenmek zorundadır.

"Müstakil konut" burada her konut bağımsız bölümüdür: 5/25'in 2025 ek cümlesi aynı beş piyesin istisnasını
"parseldeki bağımsız bölüm" sayısıyla kuruyor. KentOS Mimari'nin yerel denetimi de 29/1'i her konut bölümüne uygular.

### BYKHY

Madde 4 (tanımlar):

> Kaçış uzaklığı: Herhangi bir katta bir mekân içinde durulabilen en uzak noktada bulunan bir kullanıcının kendisine en yakın kat çıkışına kadar almak zorunda olduğu yürüme yolunun uzunluğunu,

> Tek yönlü kaçış mesafesi: Bir mekân içindeki kişilerin sadece tek bir yönde hareket ederek bir çıkışa veya alternatifli iki yönde kaçış imkânına sahip olduğu noktaya kadar olan mesafeyi,

4/i (2009/15316 sayılı BKK ile değişik), 0.3.0'ın çıkmaz koridoru:

> i) (Değişik: 10/8/2009-2009/15316 K.) Çıkmaz koridor mesafesi: Mekân içerisinden mekânın koridora bağlanan kapısına kadar olan mesafe göz önüne alınmaksızın, kaçışta, mekânların bağlı olduğu koridorun en uzak noktasından koridor boyunca bir çıkışa veya iki yönde kaçış imkânına sahip olunan noktaya kadar olan mesafeyi,

Madde 30 (beşinci fıkranın son cümlesi), çıkmaz alanlardan söz eden tek öbür hüküm (sayısal sınır vermez):

> Bir yangın hâlinde veya herhangi bir acil durumda, kullanıcıların yanlışlıkla çıkmaz alanlara girmemeleri ve kullanılan odalardan ve mekânlardan geçmek zorunda kalmaksızın bir çıkışa veya çıkışlara doğrudan erişmeleri için gerekli tedbirler alınır.

Madde 31 (Kaçış Yolları), üçüncü fıkra:

> Her katta, o katın kullanıcı yüküne ve en uzun kaçış uzaklığına göre çıkış imkânları sağlanır.

Madde 32 (Çıkış kapasitesi ve kaçış uzaklığı):

> (3) Kaçış uzaklığı, kullanım sınıfına göre Ek-5/B’de belirtilen değerlerden daha büyük olamaz.

> (6) Kaçış uzaklığı ölçülecek en uzak nokta mekân içinde mekânı çevreleyen duvarlardan 40 cm önde alınır.

Madde 33 (Kaçış yolu sayısı ve genişliği), altıncı fıkranın (b) bendi:

> b) Bir yapıda veya katlarında bulunan her kullanıcı için, diğer kullanıcıların kullanımında olan odalardan veya mekânlardan geçmek zorunda kalınmaksızın, bir çıkışa veya çıkışlara doğrudan erişim sağlanması gerekir.

Madde 48 (Konutlar; 2009/15316 sayılı BKK ile değişik):

> (1) Bodrum katlar dâhil 4 katı geçmeyen konutlar ile tek evler, ikiz evler ve sıra evler gibi konutlarda, tek bir kullanıma hizmet veren binalarda veya böyle bir binanın ayrılmış bir bölümünde kaçışlar, kaçış mesafesi aranmaksızın normal merdivenlerle sağlanabilir.

> (2) Birinci fıkrada belirtilenler dışındaki konutlarda, konut içindeki herhangi bir noktadan konut çıkış kapısına kadar olan uzaklığın 20 m’yi, yağmurlama sistemi olan konutlarda 30 m’yi geçmemesi gerekir. İkiden çok ara kat bulunmayan apartman dairelerinde tek kapı bulunması hâlinde, bu kapı üst katta düzenlenemez. Üstteki katın döşeme alanı, bu kat için ayrı bir çıkış sağlanmadıkça 70 m2’yi aşamaz.

> (3) Konut birimlerinden bütün çıkışların, kaçış merdivenlerine veya güvenli bir açık alana doğrudan erişim imkânı sağlayacak şekilde olması gerekir.

> (4) Kaçış uzaklığı, apartman dairelerinin kapısından başlanarak ölçülür.

Ek-5/B (Çıkışlara Götüren En Uzun Kaçış Uzaklıkları ve Birim Genişlikleri), kaçış uzaklığı sütunları (m):

| Kullanım sınıfı | Tek yön, yağmurlama yok | Tek yön, yağmurlamalı | İki yön, yağmurlama yok | İki yön, yağmurlamalı |
|---|---|---|---|---|
| Yüksek tehlikeli yerler | 10 | 20 | 20 | 35 |
| Endüstri amaçlı yapılar ⁽¹⁾ | 15 | 25 | 30 | 60 |
| Yurtlar, yatakhaneler | 15 | 30 | 45 | 75 |
| Mağazalar, dükkânlar, marketler | 15 | 25 | 45 | 60 |
| Büro binaları | 15 | 30 | 45 | 75 |
| Otoparklar ve depolar | 15 | 25 | 45 | 60 |
| Okul ve eğitim yapıları | 15 | 30 | 45 | 75 |
| Toplanma amaçlı binalar | 15 | 25 | 45 | 60 |
| Hastaneler, huzurevleri | 15 | 25 | 30 | 45 |
| Oteller, pansiyonlar | 15 | 20 | 30 | 45 |
| **Apartmanlar** | **15** | **30** | **30** | **75** |

> Kolay alevlenici malzeme üretimi yapmayan endüstriyel amaçlı yapılarda tek ve iki yönlü uzaklık ½ oranında artırılabilir.

Aynı tablonun "Çıkmaz koridor en çok uzaklık(m)" sütunu ("Koridorlar"; yağmurlama sistemi yok, yağmurlama sistemli):
yüksek tehlikeli yerler 10 ve 20; endüstri amaçlı yapılar, yurtlar, mağazalar, büro binaları, otoparklar ve depolar,
okul ve eğitim yapıları, toplanma amaçlı binalar, hastaneler, oteller ve **apartmanlar 15 ve 20**. Dipnot (1) yalnız
"tek ve iki yönlü uzaklık"ı artırır, çıkmaz koridoru değil.

Madde 43/1, "ara kat"ı kattan ayrı sayan hüküm (yönetmelik "ara kat"ı tanımlamaz):

> MADDE 43- (1) Dairesel merdivenler; yanmaz malzemeden yapılmaları ve en az 100 cm genişlikte olmaları hâlinde, kullanıcı yükü 25 kişiyi aşmayan herhangi bir kattan, ara kattan, veya balkonlardan zorunlu çıkış olarak hizmet verebilir.

## Olgular neye karar verebiliyor

`ext.ArchitectureFacts` (ADR 8057 §39) katları (kotlarıyla), kat başına kat çıkışı sayısını, her mahalin net ve eksen
alanını, bağımsız bölümleri (`kind`: unit, common, outdoor, annex; `dwelling`), mahalin en uzak noktasından (duvarlardan
40 cm önde) en yakın kat çıkışına yürüyüşü (uzunluk, tek yön payı, ulaşılan çıkış sayısı), konutun mahalinden kapısına
ve konut kapısından kat çıkışına yürüyüşü verir. §53 (0.2.0'ın okuduğu) bunlara ekledi:

- `RoomFacts.kind`: `living`, `bedroom`, `kitchen`, `bath`, `toilet`, `corridor` ya da yok: KentOS Mimari'nin mahali
  adından ya da kullanımından okuyuşu (Salon/Oturma, Yatak/Ebeveyn/Çocuk/Oda, Mutfak, Banyo/Duş/Yıkanma, WC/Tuvalet,
  Hol/Koridor/Antre). Kural bu okuyuşu kullanır, kendisi addan tür çıkarmaz.
- `RoomFacts.narrow`: net genişlik (net sınırın içine sığan en büyük dairenin çapı, KentOS Mimari'nin dar kenarı);
  sınırı bulunamayan mahalde yok.
- `ArchitectureFacts.sprinklers`: projenin yağmurlama ayarı (KCAD 226; ayarın varsayılanı "yok").
- `DoorEscape.into_stair`: konut kapısı kaçış merdiveni mahaline açılıyor (yürüyüşü yok, kapı kat çıkışında).

Hâlâ vermedikleri:

- **Konut olmayan bölümün Ek-5/B'deki kullanım sınıfı** (`nature` tapudaki serbest yazıdır: "Dükkân", "Büro").
- **Niş ve donanım:** bir odanın içindeki mutfak ya da yatak nişi, bir mahalde hem banyo hem tuvalet donanımı olduğu.
- **Parseldeki bağımsız bölüm sayısı** (5/25'in %20'si): çizim parselin tamamı olmayabilir.
- **Olguların sürümü:** §53'ün alanları eski bir KentOS'un olgularında okununca boş ya da `False` gelir; "yağmurlama
  yok" ile "ayar verilmedi" Python'da ayırt edilmez. Paket olguların yeni olduğunu sınırı bulunan bir mahalin dar
  kenarından tanır (yeni KentOS bunu her zaman yazar).

## Kurallar (0.2.0 ve 0.3.0)

| Kimlik | Hüküm | Karar | Önem |
|---|---|---|---|
| `piyesler` | PAİY 29/1, 29/2, 29/4, 5/25 | Konut bölümünde türü bilinen her piyesin net alanı ve dar kenarı tablonun en azına: biri yetmiyorsa iki ölçüyü de söyleyen hata. Eksik piyes istisnalarıyla (aşağıda). Piyeslerinin hiçbirinin türü okunamayan konutta yalnız 12.0 m²'lik piyesin varlığı (0.1.0'ın kuralı). | hata; karar verilemeyende bilgi |
| `koridor` | PAİY 29/3 | Türü `corridor` olan mahalin dar kenarı 1.20 m'den azsa hata; bulgu fıkrayı aynen söyler. Konutun ve ortak alanın holleri ile bir bölüme ait olmayanlar okunur. | hata |
| `daire-ici` | BYKHY 48/1, 48/2 | Konutun her katında kapıya en uzun yürüyüş projenin ayarının sınırına (20 m, yağmurlama sistemiyle 30 m): aşarsa hata ve hangi sınırın okunduğunu söyler. Kapıya yol bulamayan mahal hata. | hata |
| `daire-kapisi` | BYKHY 32/3, 48/1, 48/3, 48/4, 33/6-b, Ek-5/B apartmanlar | Merdivene açılan kapı uygun; hiçbir kat çıkışına ulaşmayan kapı her binada hata (48/3, 33/6-b). Yürüyüş apartmanlar satırına, ayarın sütunuyla (15/30 m, yağmurlamalı 30/75 m): aşarsa hata. | hata |
| `cikis-erisimi` | BYKHY 31/3, 33/6-b | Değişmedi: kat çıkışı olmayan kat, kendi bölümünden ve ortak alanlardan çıkışa ulaşmayan mahal. | hata |
| `cikmaz-koridor` (0.3.0) | BYKHY 4/i, Ek-5/B | Türü koridor olan mahalin yürüyüşünün tek yön payı ayarın sınırına (15 m, yağmurlamalı 20 m): aşarsa hata. Yalnız konut bölümleri olan binada apartmanlar satırı ve 48/1; öbürlerinde her sınıfın ortak sınırı. | hata |
| `ara-katli-daire` (0.3.0) | BYKHY 48/2 (ikinci ve üçüncü cümleler) | İki katlı konut bölümünde tek kapı üst katta olamaz; üst katta kapı yoksa üst kat 70 m²'yi aşamaz (net aşıyorsa hata, yalnız brüt aşıyorsa bilgi). | hata; karar verilemeyende bilgi |

### Çıkmaz koridor (0.3.0)

- **Ölçü KentOS Mimari'nin:** yerel yangın denetimi (`fire_audit.rs`) koridorun kendi yürüyüşünün tek yön payını
  çıkmaz koridor sayar ("from its farthest point to an exit or to where two ways part") ve `Use::dead_end` ile 15/20 m'ye
  (yüksek tehlikeli yerde 10/20) bakar. Olgularda aynı yürüyüş var (`escape.exit.one_way`, tek çıkışta bütün yol).
  Başlangıç noktası mahallerdeki gibi duvarlardan 40 cm öndedir (32/6); 4/i "koridorun en uzak noktası" der, fark
  yerel denetimdekiyle aynıdır.
- **Kullanım sınıfı gerekmiyor:** çıkmaz koridor sütunu yüksek tehlikeli yerler dışında her satırda 15/20'dir. Konut
  olmayan bölümün ya da karma binanın koridorunda bu sınırı aşan çıkmaz her sınıf için aykırıdır, karar verilir. Yalnız
  yüksek tehlikeli yerin yağmurlamasız 10 m'si sınıfa bağlıdır; yüksek tehlikeli yer olgularda tanınmaz, 10-15 m arası
  söylenmez. (Görevde konut olmayan sınıfların atlanması istenmişti; sütun sınıftan bağımsız olduğu için karar
  verilebildi.)
- **Yerel denetimden ayrıldığı yer:** yerel denetim yalnız konut bölümleri olan binanın ortak koridorunu okumuyor
  (kullanımı `None` kalıyor, kural geçiyor); paket onu apartmanlar satırıyla okur ve 48/1'i uygular (çıkmaz koridor
  mesafesi bir kaçış mesafesidir). Karma binada 48/1 uygulanmaz: ortak koridor konut olmayan bölümlere de hizmet eder
  (31/5'in en sıkı kullanıma göre okuyuşu). Konutun (ve konutun eklentisinin) kendi holü 48/2'nindir, okunmaz.
- **Eski olgular:** türü vermeyen bir KentOS'un olgularında koridor tanınmaz, kural bir şey söylemez; yağmurlama
  ayarı 0.2.0'daki gibi okunur.

### Ara katlı daire (0.3.0)

- **Karar verilebilen:** iki katlı konut bölümü (`UnitFacts.levels`'ta iki kat). "Ara kat" yönetmelikte tanımlı değil
  ve 43/1 onu kattan ayrı sayar; ama iki katlı daire her okuyuşta "ikiden çok ara kat bulunmayan" dairedir ve "üst kat"
  yüksek olan kattır. Kapılar olguların konut kapılarıdır (`doors`, katlarıyla): üst katta kapı varsa o katın ayrı
  çıkışıdır.
- **Tek kapı üst katta:** hata. **Üst katta kapı yok:** "döşeme alanı"nın net mi brüt mü olduğu yazılı değil; net alan
  70 m²'yi aşıyorsa her okuyuşta aykırıdır (hata), yalnız brüt aşıyorsa karar verilemez (bilgi), brüt aşmıyorsa uygun.
- **Kapsam:** fıkra "Birinci fıkrada belirtilenler dışındaki konutlarda" diye başlar; aynı fıkranın cümleleri olduğu
  için 48/1'in muaf tuttuğu binalarda okunmaz (yanlış bulgu yerine eksik bulgu yönünde).
- **Karar verilmeyen:** üç ve daha çok katlı daire (kaç ara katı olduğu ve hangisinin "üstteki kat" olduğu belli değil).

### Karar: PAİY 29/1 tek kural

29/1'in beş piyesi, 29/2, 29/4 ve 5/25 tek bir kuraldır (`piyesler`; 0.1.0'ın `oturma-odasi`'nın yerine). Eksik
piyesin kararı konutun bütün piyeslerine bakar: 29/2 oda sayar, 29/4 nişi taşıyacak odanın alanını toplamla
karşılaştırır, türü okunamayan konutta 12 m²'lik denetim tür denetiminin yerini alır. Ayrı kurallar aynı konutu ayrı ayrı
çözer ve birbiriyle çelişen ("oturma odası yok" ile "12 m²'lik piyes yok") ya da yinelenen bulgular verirdi. 29/3 ayrı
kuraldır (`koridor`): başka bir fıkra, başka bir alan (ortak alanın holleri de).

### Ölçüler her piyese

Tablo piyesin en az ölçüsü olarak okundu ve türü bilinen her piyese uygulanır (ikinci yatak odası da 9.00 m²'yi
karşılar); KentOS Mimari'nin yerel denetimi de böyle okur. Sınırı bulunamayan piyesin ölçüsü denetlenemez (bilgi).

### Eksik piyesin istisnaları (aynen okunan fıkralardan)

- **Oturma odası:** istisnası yok, eksikse hata.
- **Yatak odası (5/25):** oturma odası yatak nişini (dar kenarı 2.50 m, alanı 9 m²) taşıyabiliyorsa karar verilemez
  (bilgi): olgular nişi ve parseldeki %20'yi görmez. Oturma odası yoksa ya da nişi taşıyamıyorsa hata.
- **Mutfak (29/4):** mutfak nişi bir odada olabilir, o oda kendi en az alanı ile mutfağın 3.30 m²'sinin toplamı kadar
  olur (oturma odasında 15.30 m², yatak odasında 12.30 m²). Böyle bir oda varsa karar verilemez (bilgi), yoksa hata.
- **Banyo ve tuvalet (29/2, 29/4):** oda sayısı oturma ve yatak odalarıdır (KentOS Mimari'nin yerel denetimi de böyle
  sayar). 3 veya daha az odalı konutta tuvalet banyoyla aynı yerde olabilir; o banyo iki piyesin toplamı kadar, en az
  4.20 m² olur: böyle bir banyo varsa tuvaletin yokluğu bulgu değildir, banyolar daha küçükse hata. 4 ve daha çok
  odalı konutta tuvalet ayrıdır, yoksa hata. Banyo yoksa ve 3 veya daha az odalı konutta 4.20 m²'lik bir tuvalet varsa
  o yer ikisini birlikte karşılıyor olabilir: karar verilemez (bilgi).
- **Adı türünü söylemeyen mahal:** eksik türün en az ölçüsünü karşılıyorsa o tür olabilir; karar verilemez, bulgu
  mahalin adının düzeltilmesini söyler (bilgi). Karşılamıyorsa piyes ya yoktur ya ölçüsü yetmez, ikisi de aykırıdır.

### Yağmurlama ayarı ve eski olgular

Yeni olgularda sınır projenin ayarınındır: ayar işaretliyse yağmurlamalı sütun, değilse yağmurlamasız sütun okunur ve
aşan ölçü hatadır; bulgu hangi sınırın okunduğunu ve ayarı söyler. Ayarın varsayılanı "yok"tur (`.kcad` 226: "Absent:
none"); KentOS Mimari'nin yangın denetimi de ayar verilmemiş projeyi yağmurlamasız okur (`fire_audit.rs`:
`arch.sprinklers`, ayarsız projede `false`). Ayarı vermeyen eski bir KentOS'un olgularında ise ayar bilinmez: 0.1.0'ın
iki eşiği sürer (yağmurlamasız sınırı aşan uyarı, ikisini de aşan hata) ve bulgu olguların eski olduğunu söyler.

### Merdivene açılan kapı ve çıkışa ulaşmayan kapı

`into_stair` olan kapı kat çıkışındadır: uygundur. Yürüyüşü olmayan ve merdivene de açılmayan kapı hatadır; dayanağı
48/3 ("Konut birimlerinden bütün çıkışların, kaçış merdivenlerine veya güvenli bir açık alana doğrudan erişim imkânı
sağlayacak şekilde olması gerekir") ve 33/6-b'dir. 48/4 yalnız uzaklığın nereden ölçüleceğini söylediği için ulaşılmayan
çıkışın dayanağı olarak 48/3 seçildi. 48/1 uzaklığı muaf tutar, çıkışı değil: bu bulgu her binada verilir. Eski bir
KentOS'un olgularında iki durum ayırt edilmez, karar verilmez.

- **48/1'in okunuşu** KentOS Mimari'nin yangın denetimiyle aynı: kat sayısı ±0.00 ve üstündeki katlar ile mahali olan
  alt katlardır; konut binası en az bir konut bölümü olan binadır; tek konut bölümü olan bina tek ev sayılır.
- **29/3'ün alanı** KentOS Mimari'ninki gibi: konutun ve ortak alanın holleri ile bir bölüme ait olmayanlar; dükkânın,
  büronun ya da eklentinin kendi holü okunmaz.
- Bulgular maddeyi ve fıkrayı sözün sonunda söyler ("— PAİY md. 29/2, 29/4 (RG 3.7.2017/30113)", "— BYKHY md. 48/3 ve
  33/6-b"); nesnesi mahal ya da kapıdır, konutun eksik piyesinin ve katın bulgusunun nesnesi yoktur.

## Dışarıda kalanlar ve nedenleri

- **BYKHY 32/3-4 ve Ek-5/B'nin konut olmayan bölümleri:** kullanım sınıfı olgularda yok; sınıftan bağımsız bir eşik de
  dürüst değil (yağmurlamasız tek yön 15 m bütün sınıflarda ortak değil: yüksek tehlikeli yerlerde 10 m, Ek-5/B'nin (1)
  dipnotuyla kolay alevlenici malzeme üretmeyen endüstri yapılarında 22.5 m).
- **5/25'in %20 sınırı:** parseldeki bağımsız bölüm sayısı olgu değil; yatak nişi olan bölümlerin payı sayılamaz.
- **Çıkmaz koridorda yüksek tehlikeli yerin yağmurlamasız 10 m'si:** yüksek tehlikeli yer olgularda tanınmaz (0.3.0).
- **48/2'nin ara katlı daire cümleleri, üç ve daha çok katlı dairede** ve brütü aşıp neti aşmayan üst kat (0.3.0:
  karar verilmez ya da "karar verilemedi").
- **BYKHY 33/1-2 (genişlikler), 39/2 (çıkış sayısı), 48/5 (yüksekliğe göre merdivenler), Ek-5/A (kişi yükü):**
  kullanım sınıfı ve korunumluluk olgularda yok.

## KentOS'a öneriler (SDK)

1. ~~`ArchitectureFacts.sprinklers`~~, ~~mahalin türü ve dar kenarı~~, ~~`DoorEscape.into_stair`~~: §53 ile geldi, 0.2.0
   onlarla karar veriyor.
2. Bağımsız bölümün Ek-5/B kullanım sınıfı (sabit bir kod listesi, programda bildirilen): konut olmayan bölümlerin kaçış
   uzaklığı bir tablo okuması olur. Modelde yok.
3. Olguların hangi alanları taşıdığını söyleyen bir işaret (ör. bölümde bir sürüm ya da `sprinklers`'ın verilip
   verilmediği): paket bugün bunu dar kenarın varlığından çıkarıyor.
