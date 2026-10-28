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
  değil. [NKBA Kitchen Planning Guidelines](https://kb.nkba.org/uploads/2022/05/Kitchen-Planning-Guidelines.pdf),
  Guideline 13 (önerilen): bulaşık makinesinin yakın kenarı evyenin kenarından en çok 914 mm, kenarıyla ona dik duran
  dolap ve tezgâh arasında en az 533 mm durma yeri. Ölçüleri ergonomi standardı diye sunulmaz, denetlenmez; yalnız
  önü kapanan kör modül söylenir (§1).

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
- **Kör modül:** kör köşeye giren kolun köşeden bir dolap derinliği içindeki modülü kördür (çıktıda `blind`): önünü
  sonraki kolun dolapları kapatır. Evye, ocak (fırınıyla) ya da bulaşık makinesi orada olursa uyarı
  `casework_blind_corner` (modül, kapanan boy, hangi kol); yazılır ama önünde durulamaz, kapağı açılamaz.
  [NKBA Kitchen Planning Guideline 13](https://kb.nkba.org/uploads/2022/05/Kitchen-Planning-Guidelines.pdf)
  bulaşık makinesinin kenarıyla ona dik duran dolap ve tezgâh arasında en az 533 mm (21″) durma yeri önerir; kör
  modüldeki makinede bu yer eksidir. KentOS yalnız önü kapanan modülü söyler (Sınırlar). Gerçek GUI'de U mutfağın
  Otomatik bulaşık makinesi bu modüle düşünce bulundu.
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

- `cad.casework.edit` v1: `run` (dizinin kimliği ya da bir üyesinin kalıcı kimliği) ve `edit`:
  `module` (bir modülün kendi eni `width` ve açılması `hand`, ikisi de yoksa modülün kendi değerleri kalkar; `holds`
  ne taşıdığı: `base`, `sink`, `cooktop`, `dishwasher`; ocağı kalkan dizinin fırını da kalkar), `recipe` (dizinin
  bütün tarifi: `cad.casework.run`'ın girdisi; derinlik, köşe, cihazlar, buzdolabı, üst dolap, renk ve kalınlık
  böyle değişir) ve `release` (dizi bırakılır, nesneler bağımsız kalır). Tek geri alma adımı (“Mutfak dizisini
  düzenle”, “Mutfak dizisini bırak”). Retler: `casework_not_found` (`run`), yeniden kurmanın retleri (modül
  düzenlemesinde yol `edit`, tarifte `edit.recipe.` ve tarifin yolu; modül düzenlemesinin uyarıları da `edit`'te) ve
  beklenen sürüm.
- `cad.casework.query` v1: salt okunur; dizilerin (ya da birinin) kimliği, kolları (boy, dolgu, bağlı duvar),
  modülleri (numara, kol, tür, başlangıç, en, açılma, fırın, kör), buzdolabı, kırıklığı ve eksik üyeleri.

### 7. Araç ve Öznitelikler

Mutfak dizisi aracının **Biçim**'i (B) Düz, L, U: köşeler tıklanır (L bir, U iki); her köşe ve son nokta önceki kola dik
gelir (imleç dikliğe izdüşer). Düz dizi son noktadan sonra yanın tıklanmasıyla, L ve U son noktanın tıklanmasıyla
döndüğü yanda yazılır; Enter gösterilen yolla yazar. Görünür ayarlar: Biçim, Köşe (Kör, Açık), Bulaşık makinesi (Yok,
Otomatik: evyenin yanındaki modül, ya da modül), Fırın, Buzdolabı (Yok, Başta, Sonda). Otomatik evye, ocak ve bulaşık
makinesi kör modülü atlar (evye ikinci modülden sonraki ilk açık modüle geçer); elle kör modüle konan cihaz ve kolun
önüne geldiği kapı ya da alçak pencere (`casework_clash`) önizlemenin etiketinde ve yazınca söylenir. Önizleme komutun
planıdır. Dizinin bir nesnesi seçilince Öznitelikler'de Mutfak dizisi bölümü: Dizi (Düz/L/U ve modül sayısı; Dizinin
tamamını seç, Diziyi bırak, Dizinin tamamını sil), durum (kırık dizide), Köşe, Tezgâh derinliği, Üst dolap, Fırın,
Buzdolabı, Dolgu ve seçili modülün sırası (kör köşedeyse söylenir), Tür, Modül eni (cihazın eni yalnız gösterilir) ve
Açılma; her değişiklik `cad.casework.edit` ile tek adım. Dizinin verisi (Modül, Tür, Açılma) Öznitelik bilgileri'nde
ayrıca düzenlenmez.

## Sınırlar

- Köşe dolabının kendi bloğu (karusel, köşe çekmecesi) yok: kör köşe Alt dolap'ın uzantısıdır, açık köşe dolgu.
- NKBA'nın bulaşık makinesi için 533 mm durma yeri ve evyeye 914 mm önerisi denetlenmez; yalnız kör köşede önü kapanan
  modüldeki evye, ocak ve bulaşık makinesi söylenir.
- Uzun dolap (fırın kolonu, buzdolabı kabini), süpürgelik, tezgâh arkası, 3B ve kotlar açık.
- Dik olmayan köşe ve eğri duvar boyunca dizi açık; yalnız düz yüz kenarına bağlanır.
- Modüller kola 1e-9 payla sığar. Kenetsiz tıklamanın ekran yuvarlaması (yaklaşık 10⁻⁷ m) tam sığan bir modülü
  sonraki kola atabilir (gerçek GUI resimlerinde görüldü); kesin ölçülü dizi yazılan noktayla, yazılan uzunlukla ya
  da duvar köşesine kenetle çizilir. Pay büyütülmedi (CLAUDE.md §23.3).
- Kapı önü uyarısı yalnız bağlı duvarın açıklıklarına bakar; kolon ve başka nesneyle çakışma denetlenmez.
- Sunucu dizilerin sözleşme kurallarını denetler (`casework_problem`); adlandırdıkları nesnelerin ve duvarların
  projede olduğunu denetlemez, onları belge tutar. Komut protokolünün durum damgaları dizileri ayrıca saymaz; beklenen
  sürüm yeter.
- Dizinin bütün nesneleri birlikte taşınır ya da döndürülürse dizi de gider; ölçekleme ve aynalama dizinin ölçüsünü
  ve yanını bozacağından sahiplik onu reddeder (dizi düzenlenir ya da bırakılır). Bağlı duvar ölçeklenir ya da
  aynalanırsa dizi duvarın yeni yüzüne göre yeniden dizilir.

## Doğrulama

- Ortak durumlar KentOS kodu olmadan yazılmış bağımsız başvurudan (`scripts/fixtures/casework_model.py`,
  `casework_cases.py --check`): `cad.casework.run` 80 (düz, L, U, kör ve açık köşe, kör modüldeki bulaşık makinesi, evye
  ve fırınlı ocak uyarısı ve açık köşedeki karşılığı, cihazlar, buzdolabı, modülün kendi eni ve kapısı, çizimin kendi
  blokları, birimsiz blok, kapı ve alçak pencere uyarısı, dar köşe ve negatif kalan dahil 42 ret), `cad.casework.edit`
  39 (modül, tarif, bırak; kör modüle konan bulaşık makinesi; duvarı taşınan L ve U, düz dizi, silinen duvar, bütün
  nesneleriyle taşınan dizi, tek nesneye ve tezgâha `casework_owned`, bütün nesneleri silinen dizi, geri alma, sürüm),
  `cad.casework.query` 22. Rust'ta (`every_command_case`) ve Python'un tipli katmanında aynı sonuç; yetenek testi
  (`casework_owned` protokol kodu).
- KCAD şema 127: `casework.kcad` bağımsız yazıcıyla (başvurunun L dizisi, iki duvar bağı, üst dolap, bulaşık
  makinesi), 8 bozuk örnek; `tools/kcad/kcad.py` ve Rust kodeki aynı sonucu verir, Rust aynı baytları yazar; taşıma
  testi; `schema-version-128.kcad`.
- Etkileşim: düz dizi, yazılan uzunluk ve ayarlar, kısa dizi, L (köşe dikliğe izdüşer, döndüğü yanda yazılır), U (açık
  köşe, fırın, buzdolabı), Otomatik'in kör modülü atlaması ve elle kör modüle konan cihazın etiketi, kapı çakışmasının
  önizlemenin etiketinde ve yazınca söylenmesi, noktaları yazılan L'nin yazdığı diziyle söylenmesi; 65 + 796
  etkileşim testi.
- Masaüstü (`casework_scenes`, resimlerin sahneleri testte de): L iki duvara bağlı ve kör köşe dolgusu, açık köşe;
  U'da Otomatik evye 2. ve bulaşık makinesi 1. modülde, kör modüllerde yalnız düz dolap, batı kolunda dolgu yok;
  Modül eni 90 cm; kuzey duvar 40 cm taşınınca dizi izler (doğu kolu boyunu korur); kapı çakışması önizlemede, yazınca
  günlükte; Öznitelikler'in bölümü sorgunun modülleriyle, Tezgâh derinliği, Diziyi bırak ve Dizinin tamamını sil.
  Sekiz gerçek GUI resmi `.run/shots/arac-mutfak-*` (1440×900 ve 1100×650, açık ve koyu).
- Gerçek GUI'de bulunup düzeltilenler: U'nun Otomatik bulaşık makinesi kör köşedeki modüle düşüyordu (kör modül, §1);
  son noktası yazılan dizinin başarı iletisi önizlemenin eski planını anlatıyordu (artık yazılan dizinin
  çıktısından); kapı çakışması yalnız yazınca günlükteydi (artık önizlemenin etiketinde de); resimlerde kenetsiz
  tıklamanın yuvarlaması tam sığan modülü sonraki kola atıyordu (sahneler noktaları yazar; Sınırlar).
- Sunucu, geçici PostGIS'te: `project_casework` (diziler nesneleri ve bloklarıyla yazılır, projenin bilgisinde ve
  `.kcad` görüntüsünde döner, projeyle kopyalanır, sözleşmeyi bozan reddedilir, boş liste kaldırır) ve
  kentos-application'ın 33 test ikilisi (`backup` Homebrew libpq'nun `pg_dump`'ıyla); kentos-api 65 + 21 + 10
  (`job_run_db` dahil). Komut geçmişinin Python çağrıları (11 735, `history_calls.py`) SDK'yla aynı.
- Takımlar: alan, etkileşim, uygulama (67 + 243), KCAD (6 + 232), bulut (107 + 4 + 2), sözleşmeler (1081 + 33),
  komutlar (922), başsız (11 + 6 + 65), MCP, Python SDK'sı (374); manifest, SDK, şerit ve KCAD üreteçleri güncel;
  değişen crate'lerde clippy `-D warnings`.
