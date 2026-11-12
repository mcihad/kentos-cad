# ADR 8050: Oda modülünün kapsamı — kolon, merdiven, rampa, döşeme ve eğri duvar

- **Durum:** sürüyor (12 Kasım 2026); `plan/architecture-advanced.md` ADV-05, M50. 1. bölüm yapıldı: modüle kolon,
  merdiven, rampa, döşeme ve eğri duvar girer (§1); bağlı döşemenin iç noktası taşınınca onunla gider (§2); kata göre
  yazılan değerler örneğin kendi katınındır (§3). 2. bölüm: mahal ayırıcı (§4). Mutfak dizisi sonraki bölüm.
- **Bağlam belgesi:** ADR 0775 (oda modülleri), ADR 0779 (üye ekleme/çıkarma, elle değişen nesne, daire), ADR 0796 (çok
  katlı modül, katları tekrarla), ADR 0811 (tek katlı örnek), ADR 0725 §2–3 (kolonun aynası), ADR 0709 (eğri merdivenin
  aynası), ADR 0339 §1 (duvar yolunun aynası), ADR 0771 §4 (kaynağa bağlı döşeme), ADR 0723 §2 ve 0724 §1 (kata bağlı
  rampa ve merdiven).

## Bağlam

Plan ADV-05'in 7 Kasım notu: "modüle kolon, merdiven, rampa, döşeme, mahal ayırıcı ve mutfak dizisi girmiyor, eğri duvar
reddediliyor (`arch_modules.rs:224-235`) → M50". M50'nin görünen sonucu: "Kolon, merdiven, döşeme, mutfak, eğri duvar
modülde". Bir banyo ya da daire modülü kolonsuz, merdivensiz ve döşemesiz olunca tip katın asıl parçaları modülün dışında
kalıyor, her örnekte elle yeniden çiziliyordu; eğri duvarlı bir oda hiç modül olamıyordu.

Başka programlar:

- **Revit** modeli grupları her model nesnesini alır (kolon, merdiven, döşeme dahil); akslar ve katlar (datum) grupta
  olmaz, grup kopyalanınca kopyanın akslarla ilişkisi kendi yerinden kurulur. Örnekteki tek nesneyi gruptan ayrık tutmak
  "Exclude"dur (bizde yerel istisna, ADR 0779). Kaynak: BIM Pure, Revit grupları üzerine ipuçları.
- **Archicad**'in modülleri (hotlink) bütün nesne türlerini taşır; Graphisoft forumunda aynalanmış modülde merdiven ve
  korkuluğun yanlış döndüğü bildiriliyor — bizde merdiven Aynala'nın kuralıyla aynalanır (ADR 0709), modüle özel bir yol
  yok.

Mevzuat: Planlı Alanlar İmar Yönetmeliği (RG 3.7.2017/30113) **Md. 31/2-a** basamak yüksekliğini asansörsüz binada
0,16 m, asansörlüde 0,18 m ile sınırlar. Rıht yüksekliği merdivenin bulunduğu katın yüksekliğinden çıkar: aynı merdiven
modülü 3,00 m'lik katta 16,7 cm, 3,24 m'lik katta 18 cm rıhtlıdır. Bu yüzden rıht yüksekliği modülün değil örneğin
katınındır (§3); Mimari denetim (ADR 8034, 8045) her örneği kendi katında ölçer.

## Karar

### 1. Modülün yeni türleri

- `module_kind_ok` (sözleşme, `arch_modules.rs`) kolonu, merdiveni, rampayı ve döşemeyi alır; eğri duvarın reddi kalktı.
  Tanımlarken ve üye eklerken aynı denetim çalışır (`modules_edit.rs` `check`, `modules_more.rs` `add_members`).
- **Akslara bağlı kolon alınmaz** (`module_kind`, yol `operation.uids[i]`): "Akslara bağlı kolon modüle alınmaz: yeri
  aksların kesişimidir. Önce Bağı kopar'ı seçin ya da kolonu seçimden çıkarın." Akslar binanındır (Revit'te de datum
  grupta olmaz); örnek taşınınca kolon akslarının kesişiminde kalır, modülün dediği yere gidemezdi. Aksın kendisi
  modülde değildir (eski ret metni yeni türleri sayar).
- Yeni türler katlarını örnekten alır (`arch_mut`/`arch_of`: çok katlı modülde üyenin katı örneğin tabanından sayılır,
  ADR 0796). Modül verisinin denetimi (`arch_modules_problem`) üst katı yalnız duvar, mahal ve donatıya tanıyordu:
  1. katında kolonu olan iki katlı modül "Modül nesnesinin katı … yalnız duvar, mahal ve donatının olur" ile
  tanımlanamıyordu (ölçüldü); artık kendi katı olan her tür (`holds_own_storey`) üst katta olabilir. Yerleştirme, aynalama ve döndürme Taşı/Döndür/Aynala'nın kuralıdır (`transform::by_matrix`): duvar yolu
  aynada geri döner, büküm korunur; kolonun noktası aynalanır, dönüşü aynanın açısından kendi dönüşü çıkarılarak, L'nin eni
  ile derinliği ve kolları yer değiştirir, hizası takas edilir (ADR 0725 §2–3); merdiven ilk noktasından çıkmaya devam
  eder, yayı ters döner (ADR 0709); döşemenin halkaları, rampanın ekseni nokta nokta.
- **Kaynağı güncelle aynalı örnekten de çalışır:** örneğin nesnesi ters matrisle tanıma döner. Aynalı ve 90° döndürülmüş
  örnekte L kolonun kolları [0,25; 0,2] iken [0,3; 0,2] yapılırsa tanımın kolları [0,2; 0,3] olur, öbür örnekler kendi
  aynalarıyla alır. Kaynakla aynı örnekten güncellemek "güncellenecek bir şey yok" der (her tür geri dönüşte kaynağına
  denk).
- Modül penceresinin tanımı yeni türleri sayar; duvarsız seçimde (merdiven ve döşemesi, kolonlar) taban noktası
  nesnelerin noktalarının sol altıdır — eskiden çizimin başlangıcıydı ve her yerleşim o kadar uzağa düşüyordu.

### 2. Bağlı döşemenin iç noktası taşınınca gider

Ölçüm (eski sürüm): duvarlarına bağlı döşeme (ADR 0771 §4, odanın iç yüzleri, iç noktası (7, 2)) duvarlarıyla birlikte
20 m taşınınca iç nokta (7, 2)'de kaldı; belge sınırı o noktanın çevresinde yeniden aradı, bulamadı: döşeme
**kopuk** (`lostEnclosure`, çizimde uyarı üçgeni). Aynı oda modül olup 20 m sağa yerleşince örneğin döşemesi **kaynak
odaya geri döndü** (5,075–7,9), kaynağın duvarlarına bağlandı: kaynak odada üst üste iki döşeme, yeni odada hiç yok.

Karar: döşemenin çekirdekteki şekli bağlı döşemenin iç noktasını mahalinki gibi taşır (`kentos_formats::spaces::core_slab`
→ `Spec.seed`; `geometry::with_shape` geri yazar). Taşı, Döndür, Aynala, Ölçekle ve modül yerleşimi noktayı halkalarla
birlikte götürür; belge sınırı taşınan noktanın çevresinde bulur. Ölçüm (yeni): taşınan döşeme (25,075–27,9), kopuk
değil; örneğin döşemesi kendi duvarlarına bağlı (25,075–27,9), örneğin sağ duvarı 1 m dışa kayınca yalnız onun döşemesi
büyür (28,9), kaynağınki aynı kalır. Döşeme tek başına başka odaya taşınırsa o odanın sınırını alır — mahal gibi.

### 3. Belgenin yazdığı değerler örneğin kendi katınındır

Modül bir nesneyi "elle değişmiş" sayarken ve kaynakla karşılaştırırken belgenin kendi yazdığı alanlara bakmaz
(`modules_more.rs` `written_by_the_document`). Yeni türlerin böyle alanları:

- **Merdiven**, yüksekliği katından (`heightFromLevel`, ADR 0724 §1): rıht yüksekliği (`riserHeight`) örneğin katının
  yüksekliği bölü rıht sayısı — 3,00 m'lik zeminde 3/18, 3,24 m'lik 1. katta 0,18 m.
- **Rampa**, katına bağlı (`baseOffset`, ADR 0723 §2): başlangıç kotu (`base`) örneğin katının kotu artı payı — 1. katta
  3 m.
- **Döşeme**: serbest kenarların çizgileri (`edge.runs`); bağlı olanın halkaları, sınırlayan duvarları (`bound.uids`) ve
  kopukluğu (`bound.broken`) — belge her adımda yeniden bulur.
- **Kolon**: aks yeri (`place`) ve kopukluğu (`broken`) — modüldeki kolon akslara bağlı olmadığı için boş kalır.

Ölçüm (eski kural, rampa eklenmeden): 1. kattaki örnekten güncellemek rampanın 3 m'lik başlangıcını tanıma yazdı
(`changedKeys`'te rampa), rampanın eni kaynakta değişince güncelleme "Elle değiştirilmiş 1 örnek nesnesi kaynağın
değeriyle yazıldı" diye yanlış uyardı. Yeni kuralla ikisi de olmuyor.

### 4. Mahal ayırıcı

Açık plandaki mahal ayırıcı (ADR 0679: `spaceSeparator` rolü taşıyan çoklu çizgi, fiziksel duvar değil) modüle girer;
rolü olmayan sıradan çoklu çizgi girmez (`module_kind`, metin "… mahal, mahal ayırıcı, donatı …"). Ayırıcının katı
rolünün `ArchInfo`'sundadır: tanımda kat silinir, örnek kendi katını yazar — yoksa 1. kattaki örneğin mahalleri zemin
kattaki ayırıcıyı görmez, ayrılmadan tek mahal olurdu. Mahallerin kaynak listesindeki ayırıcı, duvarlar gibi örneğin
kendi ayırıcısına bağlanır.

Başka programlar: Revit'te odalar modeli grubunda olur (numarası örnekte değişebilir, adı değişmez); oda ayırma çizgileri
oda sınırlayıcı model çizgileridir ve grup kopyalarında üst üste binen ayırma çizgisi uyarısı verilir (Autodesk "About
Room Separation Lines", "Groups of Elements"; AUGI forumu "Rooms separation line in groups").

Ölçüm (ADR 0679'un açık planı, 10 × 8 m, ayırıcı x = 5): aynalı 1. kat örneğinde iki mahal 38,22 + 38,22 m²; kaynağın
ayırıcısı 1 m doğuya kayıp güncellenince örneğinki 1 m batıya kayar, mahaller 46,02 ve 30,42 m² (kaynakla aynı).

## Sınırlar

- Mutfak dizisi modülde değil (sonraki bölüm).
- Katları tekrarla'nın modülsüz kopyası kolon, merdiven, rampa ve döşemeyi hâlâ kopyalamaz (ADR 0796 `kind`); modül
  örneğinin içindekiler tekrarlanır.

## Doğrulama

- Ortak durumlar `cad.modules.edit.kinds.json` (24, `module_kinds_cases.py`; kolonun aynası
  `arch_command_cases.mirrored_column` ile, KentOS kodu olmadan): tanım; 1. kata, aynalı+90°, yalnız aynalı, 45°,
  aynalı+180° yerleşim; tek adım geri alma; aynalı örnekten ve 1. kattaki örnekten "kaynakla aynı"; kolon eni, aynalı
  örnekte L kolu, döşeme kalınlığı, merdiven basamak sayısı, rampa eni, eğri duvarın taşınması, kolon dönüşü
  güncellemeleri; istisna, ayırma, elle değişen kolon; serbest kolonu ekleme, rampayı çıkarma; akslara bağlı kolon ve aks
  retleri. Masaüstü işleyicisinde ve Python'da geçti; `cad.modules.edit*` dosyalarının hepsi geçti.
- `cad.slabs.bound.json` 33 durum (32'den): duvarlarıyla 20 m taşınan bağlı döşemenin iç noktası (27, 2), sınırı taşınan
  oda, kopuk değil.
- Uygulama testi `module_kinds.rs`: taşınan bağlı döşeme bütün kalır; modül örneğinin döşemesi kendi duvarlarına bağlı,
  "kaynakla aynı"; örneğin duvarı kayınca yalnız onun döşemesi büyür; açık planın aynalı örneğinde mahaller 38,22 +
  38,22 m², kaynağın ayırıcısı 1 m kayınca 46,02 / 30,42 m². Masaüstü `room_modules_tests` 9 test (duvarsız modülün
  tabanı (0,5; 0,5)).
- Mahal ayırıcı (§4): kinds dosyasında 5 durum daha (29; tanım, aynalı 1. kat yerleşimi — ayırıcının katı ve
  mahallerin kaynakları örneğin, kaynakla aynı, ayırıcının güncellenmesi, sıradan çoklu çizginin reddi).
- Gerçek pencere (`fixtures/interaction/real/modul-kapsami.txt`, `bagli-doseme-tasi.txt`; Xvfb 1920×1080, gerçek
  fare/klavye): eğri duvarlı, kolonlu, merdivenli, bağlı döşemeli 3 × 5 m Merdiven evi. Eski sürüm Seçimden tanımla'da
  "Eğrisel duvar ilk modül kapsamında değil" dedi; yenisi tanımladı (4 nesne), Modül yerleştir Aynalı açık, taban 8,0:
  örneğin cumbası batıya, kolonu kuzeydoğu köşesine, merdiveni doğu yanına geçti (8 nesne). Oda döşemesiyle 3,5 m
  taşınınca eski sürümde kırmızı uyarı üçgeni (kopuk döşeme), yenisinde yok. Görüntüler
  `.run/gui-evidence/mimari-m50/`.
