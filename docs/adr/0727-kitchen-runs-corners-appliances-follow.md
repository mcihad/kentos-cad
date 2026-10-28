# ADR 0727: Mutfak dizisi — L ve U, cihazlar, tezgâh kesimleri, kalıcı dizi ve duvar izleme

- **Durum:** kabul edildi (28 Ekim 2026); ARCH-LIB-02'nin kalanı, bütün madde.
- **Bağlam:** `plan/architecture.md` ARCH-LIB-02; ADR 0706 (düz dizi), ADR 0702 (kitaplığın mutfak blokları), ADR 0300
  (ilişkili dizi: belgede kayıt, öğeler sıradan yerleştirme), ADR 0695 (kaynağına bağlı ölçü dizisi, sahiplenilen
  üyeler), ADR 0339 ve 0340 (duvarın yüzleri), ADR 0317 (blok çizelgesi).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0706 düz diziyi tek adımda yazıyordu; L ve U mutfak iki ayrı diziyle kuruluyor, köşede iki dizinin dolapları ve
tezgâhları üst üste biniyor, bulaşık makinesi, fırın ve buzdolabı yoktu, tezgâhta evye ve ocak kesimi yoktu. Dizi
yazıldıktan sonra yalnız ayrı bloklardı: bir modülün enini değiştirmek sonrakileri kaydırmıyor, duvar taşınınca
dolaplar yerinde kalıyordu. Planın kabulü bunları ister: toplam uzunluk ve modül dağılımı, dar köşe, negatif kalan,
çakışma, tek modül düzenleme ve kaynak duvar taşınması.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Chief Architect — Creating a Corner Cabinet](https://www.chiefarchitect.com/support/article/KB-00330/creating-a-corner-cabinet.html):
  iki alt dolap köşede birleştirilince kör köşe olur; program gizli kalan dolabın ön parçalarını kaydırır ve tezgâhı
  köşede uzatır. [L-Shaped Cabinet Without a Center Stile](https://www.chiefarchitect.com/support/article/KB-03203/creating-an-l-shaped-cabinet-without-a-center-stile.html):
  otomatik kör köşe kapatılabilir (açık köşe). [Özellik listesi](https://www.chiefarchitect.com/products/features/):
  dolaplar arasında dolgu kendiliğinden ya da elle.
- [Autodesk Revit — Cut Geometry in a Family](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Customize/files/GUID-7C940A08-1F62-465F-89BF-3800E3446FC3.htm):
  evye ailesinin boşluğu tezgâhı keser; kesim evyeyle gider (evye taşınınca delik izler).
- [Vectorworks 2025 — Creating countertops](https://app-help.vectorworks.net/2025/eng/VW2025_Guide/FurnFixt/Creating_countertops.htm)
  (ADR 0706'da da): tezgâh dolaplardan türer, evye sembolü delik açar.
- [Home Designer — Planning a Kitchen Work Area](https://www.homedesignersoftware.com/support/article/KB-00844/planning-a-kitchen-work-area.pdf):
  bulaşık makinesi evyenin yanında, ocak ve evyenin iki yanında tezgâh; KentOS bunu öneri olarak alır, kural olarak
  değil (NKBA kılavuzunun özgün metni burada okunmadı; ölçüleri ergonomi standardı diye sunulmaz).

KentOS kararı: köşe Chief Architect'teki gibi öntanımlı **kör köşe**, istenirse **açık köşe**; dolgular görünür ve
söylenir (ADR 0706); tezgâh dizinin tek alanı, köşede gönyeli, evye ve ocak kesimleri evyenin ve ocağın bloğundan
türer (Revit'teki gibi kesim evyeyle gider). Dizi ADR 0300'deki gibi belgede bir kayıttır, nesneleri sıradan
yerleştirme ve alanlardır; kayıt duvarın yüzüne bağlıdır ve duvar değişince aynı adımda yeniden kurulur.

## Karar

### 1. Yol, köşe ve kollar

`cad.casework.run` v1'in girdisine eklenenler (hepsi isteğe bağlı; eski girdi aynı anlamı taşır):

- `through`: köşeler (en çok iki: L ya da U). Yol `from`, `through…`, `to`; her parçası bir **kol**. Köşe dik olmalı
  (|cos| ≤ 1e-6) ve dolapların yanına dönmeli (sol yanda sola). Kol boyu 1e-9'dan büyük.
- `corner`: `blind` (öntanımlı) ya da `open`. Kör köşede önceki kol köşeye kadar dolapla gider (köşe karesi onun son
  modülünde), sonraki kol köşeden dolap derinliği + 5 cm'de başlar; arada 5 cm'lik köşe dolgusu. Açık köşede iki kol
  da köşeden dolap derinliği uzakta durur, köşe karesi dolgudur. Dolap derinliği Alt dolap bloğunun kutusunun
  derinliğidir (kitaplığınki 0,60 m).
- Modüller `from`'dan yol boyunca 1'den numaralanır ve sırayla sığdıkça dizilir (1e-9 payla). L ve U'nun ilk kolu
  köşesinden geriye dizilir (dolgusu dizinin başında), öbür kollar başlarından (dolgusu sonlarında).
- Bir kolun kullanılabilir boyu bir modüle yetmezse ret: dar köşe ve **negatif kalan** (kolun numarası ve kalan boyla).

### 2. Cihazlar ve modül enleri

- `dishwasher`: bulaşık makinesinin modülü (0,60 m; Alt dolap yerine “Bulaşık makinesi”).
- `oven`: fırın ocağın altında (ocak modülü 0,60 m; Alt dolap yerine “Fırın”); ocaksız fırın reddedilir.
- `fridge`: `start` ya da `end`; buzdolabı yolun o ucunda 0,60 m tutar, tezgâh orada durur.
- `perModule`: bir modülün kendi eni (`width`) ve kapısının açılması (`hand`: `left` Sol, `right` Sağ). 0,60 m'den
  geniş modül çift kapılıdır (Çift); cihazın ve çift kapılının açılması verilmez.
- Enler: verilen en, yoksa evye modülü 0,90 m, ocak modülü en az 0,60 m (dizinin eni daha darsa 0,60), bulaşık
  makinesi ve fırın 0,60 m, öbürleri dizinin eni. Her en Alt dolap bloğunun Genişlik kuralına uymalı; cihazın eni
  0,60 m'den başka verilemez.
- **Çakışma:** evye, ocak ve bulaşık makinesi ayrı modüllerde olmalı; evye ve ocak bloğunun kutusu kendi modülüne
  ve tezgâha sığmalı (sığmayan reddedilir, modülün enini ya da tezgâh derinliğini söyler). Duvara bağlı bir kol o
  duvarın kapısının önüne geliyorsa (kapı ve geçit, ya da denizliği 0,90 m'den alçak pencere) uyarı
  `casework_clash`: hangi kol, hangi açıklık ve kaç metre.

### 3. Tezgâh ve kesimler

Tezgâh yol boyunca tek alandır (`depth` derin, köşelerde gönyeli iç köşe), buzdolabının 0,60'ı dışında. Evye ve ocak
kesimleri tezgâhın delikleridir: bloğun yerleştirildiği yerdeki kutusundan her yanda 2 cm içeride (kitaplığın evyesi
0,76 × 0,46 m, ocağı 0,56 × 0,48 m kesim). Kesim evyenin ve ocağın modülüyle gider.

### 4. Modüllerin verisi

Her modülün yerleştirmesi öznitelik taşır: `Modül` (numara), `Tür` (Alt dolap, Evye dolabı, Ocak dolabı, Bulaşık
makinesi, Fırın), `Açılma` (Sol, Sağ, Çift; cihazda yok). Buzdolabının `Tür`'ü Buzdolabı; üst dolapların `Modül`'ü
altındaki modülün. Blok çizelgesi (ADR 0317) `by: Tür` ve `columns: [Modül, Açılma]` ile modül tablosunu verir;
`cad.casework.query` aynı tabloyu komut olarak verir.

### 5. Kalıcı dizi ve duvara bağ

Her dizi belgede bir kayıttır (`casework`, KCAD şema 127, opt-in): kimlik, tarif (komutun girdisi, beklenen sürüm
olmadan), her kolun duvar bağı (`wall`, yüzün kenarı `edge`, yüzü `face`) ve üyeler (her nesnenin kalıcı kimliği ve
dizideki yeri: modül, evye, ocak, buzdolabı, üst dolap, kol dolgusu, köşe dolgusu, tezgâh). Bir kol, iki ucu bir
duvarın yüzünün düz kenarına 1 mm'den yakın ve kenarın içinde ise o kenara bağlanır (ilk bulunan duvar).

Bağlı duvar değişince belge diziyi aynı adımda yeniden kurar (`follow_casework`): bağlı kolun doğrusu yeni yüz
kenarıdır; köşeler kolların doğrularının kesişimi, serbest uçlar kolun eski boyuyla; tek kollu dizinin uçları eski
kenarın yakın ucuna uzaklıklarını korur. Üyeler yerlerine göre aynı kimlikle güncellenir, artan eklenir, eksilen
silinir. Duvar silinince ya da yeni yol diziyi kurmuyorsa dizi kırılır (`broken`: `wallGone`, `notBuilt`), nesneler
yerinde kalır.

Üyeler dizinindir: başka bir komut üyenin şeklini, katmanını, rengini, çizgi kalınlığını ya da dizinin
özniteliklerini değiştiremez ve dizinin bir kısmını silemez (`casework_owned`, adım bütünüyle geri alınır); dizinin
bütün üyeleri silinince kayıt da gider. Üyenin etiketi, sembolü, görünüşü, ekleri ve öbür öznitelikleri serbesttir ve
yeniden kurmada korunur.

### 6. `cad.casework.edit` ve `cad.casework.query`

- `cad.casework.edit` v1: `run` (dizinin kimliği ya da bir üyesinin kalıcı kimliği) ve işlem:
  `module` (modülün eni, açılması ve türü: `base`, `sink`, `cooktop`, `dishwasher`; `null` öntanımlıya döner),
  `set` (dizinin değerleri: modül eni, derinlik, köşe, evye, ocak, bulaşık makinesi, fırın, buzdolabı, üst dolap,
  renk, çizgi kalınlığı) ve `release` (dizi bırakılır, nesneler bağımsız kalır). Tek geri alma adımı (“Mutfak
  dizisini düzenle”, “Mutfak dizisini bırak”). Retler: `casework_not_found` ve yeniden kurmanın retleri.
- `cad.casework.query` v1: salt okunur; dizilerin (ya da birinin) kimliği, kolları (boy, dolgu, bağlı duvar),
  modülleri (numara, kol, tür, başlangıç, en, açılma, fırın), buzdolabı, kırıklığı ve eksik üyeleri.

### 7. Araç ve Öznitelikler

Mutfak dizisi aracının **Biçim**'i Düz, L, U: köşeler tıklanır (L bir, U iki), son nokta ve yan; Enter gösterilen
yolla yazar. Görünür ayarlar: Biçim, Köşe (Kör, Açık), Bulaşık makinesi (Yok, Otomatik: evyenin yanındaki modül, ya
da modül), Fırın, Buzdolabı (Yok, Başta, Sonda). Önizleme komutun planıdır. Dizinin bir nesnesi seçilince
Öznitelikler'de Mutfak dizisi bölümü: dizinin değerleri ve seçili modülün en, tür ve açılması (`cad.casework.edit`),
Bırak.

## Sınırlar

- Köşe dolabının kendi bloğu (karusel, köşe çekmecesi) yok: kör köşe Alt dolap'ın uzantısıdır, açık köşe dolgu.
- Uzun dolap (fırın kolonu, buzdolabı kabini), süpürgelik, tezgâh arkası, 3B ve kotlar açık.
- Dik olmayan köşe ve eğri duvar boyunca dizi açık; yalnız düz yüz kenarına bağlanır.
- Kapı önü uyarısı yalnız bağlı duvarın açıklıklarına bakar; kolon ve başka nesneyle çakışma denetlenmez.

## Doğrulama

(Uygulama bitince doldurulur.)
