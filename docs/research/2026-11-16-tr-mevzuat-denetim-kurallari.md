# Türkiye mevzuatının ilk denetim kuralları: PAİY 29 ve BYKHY'nin kaçış uzaklıkları

Tarih: 16 Kasım 2026. Konu: `catalog/standards/tr-mevzuat` paketi (`tr.kentos.mevzuat` 0.1.0), plan/eklenti.md ADIM
20'nin "ulusal/belediye kurallarının Python betikleri" maddesinin ilk dilimi. Kurallar yalnız KentOS'un Rust'ta ölçtüğü
mimari olguları (`facts.project.architecture`, ADR 8057 §39) okur; bu not hangi hükmün aynen ne dediğini, olguların
hangisine karar verebildiğini ve neyin neden dışarıda kaldığını yazar.

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

> (3) Hol ve koridor genişlikleri 1.20 metreden az olamaz.

> (4) Mutfak nişi ve oda ile banyo ve tuvaletin aynı mekânda düzenlenmesi halinde her mekân için öngörülen en az alanların toplamı kadar alan düzenlenmek zorundadır.

"Müstakil konut" burada her konut bağımsız bölümüdür: 5/25'in 2025 ek cümlesi aynı beş piyesin istisnasını
"parseldeki bağımsız bölüm" sayısıyla kuruyor. KentOS Mimari'nin yerel denetimi de 29/1'i her konut bölümüne uygular.

### BYKHY

Madde 4 (tanımlar):

> Kaçış uzaklığı: Herhangi bir katta bir mekân içinde durulabilen en uzak noktada bulunan bir kullanıcının kendisine en yakın kat çıkışına kadar almak zorunda olduğu yürüme yolunun uzunluğunu,

> Tek yönlü kaçış mesafesi: Bir mekân içindeki kişilerin sadece tek bir yönde hareket ederek bir çıkışa veya alternatifli iki yönde kaçış imkânına sahip olduğu noktaya kadar olan mesafeyi,

Madde 31 (Kaçış Yolları), üçüncü fıkra:

> Her katta, o katın kullanıcı yüküne ve en uzun kaçış uzaklığına göre çıkış imkânları sağlanır.

Madde 32 (Çıkış kapasitesi ve kaçış uzaklığı):

> (3) Kaçış uzaklığı, kullanım sınıfına göre Ek-5/B’de belirtilen değerlerden daha büyük olamaz.

> (6) Kaçış uzaklığı ölçülecek en uzak nokta mekân içinde mekânı çevreleyen duvarlardan 40 cm önde alınır.

Madde 33 (Kaçış yolu sayısı ve genişliği), altıncı fıkranın (b) bendi:

> b) Bir yapıda veya katlarında bulunan her kullanıcı için, diğer kullanıcıların kullanımında olan odalardan veya mekânlardan geçmek zorunda kalınmaksızın, bir çıkışa veya çıkışlara doğrudan erişim sağlanması gerekir.

Madde 48 (Konutlar; 2009/15316 sayılı BKK ile değişik):

> (1) Bodrum katlar dâhil 4 katı geçmeyen konutlar ile tek evler, ikiz evler ve sıra evler gibi konutlarda, tek bir kullanıma hizmet veren binalarda veya böyle bir binanın ayrılmış bir bölümünde kaçışlar, kaçış mesafesi aranmaksızın normal merdivenlerle sağlanabilir.

> (2) Birinci fıkrada belirtilenler dışındaki konutlarda, konut içindeki herhangi bir noktadan konut çıkış kapısına kadar olan uzaklığın 20 m’yi, yağmurlama sistemi olan konutlarda 30 m’yi geçmemesi gerekir.

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

## Olgular neye karar verebiliyor

`ext.ArchitectureFacts` (ADR 8057 §39) katları (kotlarıyla), kat başına kat çıkışı sayısını, her mahalin net ve eksen
alanını, bağımsız bölümleri (`kind`: unit, common, outdoor, annex; `dwelling`), mahalin en uzak noktasından (duvarlardan
40 cm önde) en yakın kat çıkışına yürüyüşü (uzunluk, tek yön payı, ulaşılan çıkış sayısı), konutun mahalinden kapısına
ve konut kapısından kat çıkışına yürüyüşü verir.

Vermedikleri, kuralların sınırını çizen:

- **Mahalin türü yok.** `usage` programın serbest yazısıdır ve örnek binada yalnız balkonda dolu; `name` ("Salon",
  "Yatak odası 1") serbest yazıdır. Görev gereği addan tür çıkarılmadı.
- **Mahalin dar kenarı (genişliği) yok.**
- **Binada yağmurlama sistemi olup olmadığı yok** (projenin mimari ayarında var, KCAD 226; olgulara girmiyor).
- **Konut olmayan bölümün Ek-5/B'deki kullanım sınıfı yok:** `nature` tapudaki serbest yazıdır ("Dükkân", "Büro").
- **Konut kapısının yürüyüşü yoksa nedeni yok:** kapı kaçış merdiveni mahaline mi açılıyor (uygun), hiçbir çıkışa mı
  ulaşmıyor (aykırı), olgular ayırmıyor.

## Kurallar

| Kimlik | Hüküm | Karar | Önem |
|---|---|---|---|
| `oturma-odasi` | PAİY 29/1 (5/25) | Konut bölümünde (`dwelling`) net alanı 12.0 m²'ye ulaşan piyes yoksa oturma odası hangi mahal olursa olsun sağlanamaz. Boşluk, dış mahal ve kaçış merdiveni piyes sayılmaz; sınırı bulunamayan mahal varsa ve ölçülenlerin hiçbiri yetmiyorsa karar verilmez. | hata; ölçülemeyende bilgi |
| `daire-ici` | BYKHY 48/1, 48/2 | Konutun her katında mahallerin en uzak noktasından konut kapısına en uzun yürüyüş: 20 m'yi aşarsa uyarı (yağmurlama sistemi yoksa aşılıyor), 30 m'yi de aşarsa hata. Kapısı o katta olan konutun kapıya yol bulamayan mahali hata. | uyarı / hata |
| `daire-kapisi` | BYKHY 32/3, 48/1, 48/4, Ek-5/B apartmanlar | Konut kapısından en yakın kat çıkışına: tek çıkışta bütün yol tek yöndür; iki çıkışta önce yolun tek yönlü kısmı tek yön sınırına, o aşılmıyorsa bütün yol iki yön sınırına. Yağmurlama sistemsiz sınırı (15/30 m) aşarsa uyarı, yağmurlamalı sınırı da (30/75 m) aşarsa hata. | uyarı / hata |
| `cikis-erisimi` | BYKHY 31/3, 33/6-b | Mahali olup kat çıkışı olmayan kat hata (kat başına bir bulgu); çıkışı olan katta kendi bölümünün mahallerinden ve ortak alanlardan geçerek hiçbir kat çıkışına ulaşmayan mahal hata. | hata |

- **48/1'in okunuşu** KentOS Mimari'nin yangın denetimiyle aynı: kat sayısı ±0.00 ve üstündeki katlar ile mahali olan
  alt katlardır; konut binası en az bir konut bölümü olan binadır; tek konut bölümü olan bina tek ev sayılır. Bu
  binalarda `daire-ici` ve `daire-kapisi` bir şey söylemez.
- **Yağmurlama bilinmediği için iki eşik:** yağmurlama sistemi olmayan binanın sınırını aşan ölçü uyarıdır ve bulgu
  bunu söyler; yağmurlamalı binanın sınırını da aşan ölçü her durumda aykırıdır.
- Bulgular maddeyi ve fıkrayı sözün sonunda söyler ("— PAİY md. 29/1 (RG 3.7.2017/30113)", "— BYKHY md. 48/2
  (2009/15316 sayılı BKK ile değişik)"); nesnesi mahal ya da kapıdır (Göster onu seçer), katın bulgusunun nesnesi
  yoktur.

## Dışarıda kalanlar ve nedenleri

- **PAİY 29/1'in öbür satırları (yatak odası 9.00 m², mutfak 3.30 m², banyo 3.00 m², tuvalet 1.20 m²) ve bütün dar
  kenarlar:** mahalin türü ve dar kenarı olgularda yok. İkinci bir 9 m²'lik piyesin aranması da karar veremez: 5/25'in
  yatak nişi istisnası parseldeki bölümlerin %20'sine kadar ayrı yatak odasını kaldırıyor ve hangi bölümün istisnayı
  kullandığı olgularda yok.
- **PAİY 29/3 (hol ve koridor 1.20 m):** genişlik ve koridor türü yok.
- **BYKHY 32/3-4 ve Ek-5/B'nin konut olmayan bölümleri:** kullanım sınıfı yok; sınıftan bağımsız bir eşik de dürüst
  değil (yağmurlamasız tek yön 15 m bütün sınıflarda ortak değil: yüksek tehlikeli yerlerde 10 m, Ek-5/B'nin (1)
  dipnotuyla kolay alevlenici malzeme üretmeyen endüstri yapılarında 22.5 m).
- **BYKHY 4/i ve Ek-5/B'nin çıkmaz koridor sütunu:** hangi mahalin koridor olduğu olgu değil (adı).
- **BYKHY 48/2'nin ikinci ve üçüncü cümleleri** (ara katlı dairede tek kapı üst katta olamaz; üst kat ayrı çıkışsız
  70 m²'yi aşamaz): dubleks dairenin hangi katının "üst kat" olduğu ve döşeme alanının neti mi brütü mü olduğu bu
  dilimde çözülmedi; olgular (`has_entrance`, `UnitLevel`) buna yaklaşıyor, sonraki kural adayı.
- **Kapısı yürüyüşsüz konut kapısı:** yukarıdaki belirsizlik yüzünden karar verilmez.
- **BYKHY 33/1-2 (genişlikler), 39/2 (çıkış sayısı), 48/5 (yüksekliğe göre merdivenler), Ek-5/A (kişi yükü):**
  genişlik, kullanım sınıfı ve korunumluluk olgularda yok.

## KentOS'a öneriler (SDK)

1. `ArchitectureFacts.sprinklers`: projenin mimari ayarındaki yağmurlama sistemi (KCAD 226). Gelince `daire-ici` ve
   `daire-kapisi` uyarıyı karara çevirir.
2. Bağımsız bölümün Ek-5/B kullanım sınıfı (sabit bir kod listesi, programda bildirilen): konut olmayan bölümlerin kaçış
   uzaklığı bir tablo okuması olur.
3. Mahalin türü (programın tarifinden: oturma odası, yatak odası, mutfak, banyo, tuvalet, hol/koridor) ve dar kenarı
   (KentOS Mimari'nin `width_of`'u zaten ölçüyor): PAİY 29/1'in tamamı ve 29/3.
4. `DoorEscape`'te "kaçış merdiveni mahaline açılıyor" ile "hiçbir çıkışa ulaşmıyor" ayrımı.
