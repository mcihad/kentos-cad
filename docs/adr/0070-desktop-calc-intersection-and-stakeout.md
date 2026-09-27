# ADR 0070: Masaüstünde Hesap pencereleri (1): Önden ve Geriden kestirme, Aplikasyon

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.7, §5, §7, §23; TODOS.md `UI-11`; ADR 0056 (pano), 0058 (paneller)
- **Sahibin yönü (26 Eylül):** web'deki araçlar ve pencereler masaüstüne birebir taşınır.
- **Web ajanının tarifi (27 Eylül, 13. görev):**
  - kaynak: `ui/calc/common.ts`, `read.ts`, `IntersectionDialog.ts`, `StakeoutDialog.ts` ve çekirdeğin `survey` modülü;
  - 15. görevde web'de üç düzeltme yapıldı (`af28188`); Aplikasyon'un durulan noktadaki hedef reddi bu dilime alındı.
- Poligon hesabı ve Kutupsal alım ikinci dilimdir ([ADR 0071](0071-desktop-calc-traverse-and-polar.md)). O dilim Aplikasyon'un tablosunu ortak ölçü tablosuna taşıdı; dört pencereyi de web'in yerleşimine getirdi.

## Bağlam

- Masaüstünde Hesap komutları "web'de var; masaüstüne henüz taşınmadı" diyordu.
- Hesaplar ortak çekirdekteydi: `survey::intersection::{forward_intersection, resection}`, `survey::polar::stakeout`. Web onları WASM ile çağırır; masaüstü `kentos_interaction::survey` üzerinden doğrudan çağırır.

## Karar

### Ortak parçalar (`apps/desktop/src/calc/`)

- **Bilinen nokta alanı:** yazı kutusu ("Nokta adı ya da Y,X"), "Çizimden" düğmesi ve altında noktanın çözümü. Çözüm şunlardan biridir:
  - "Y 486513.341  X 4420189.522";
  - hata;
  - "Henüz verilmedi" ya da alanın ipucu.
- **Okuma** (`read.rs`, web'in `read.ts`'i birebir):
  - "Y,X", "Y;X" ya da "Y X", nokta ondalıklı, doğu önce;
  - değilse çizimdeki ilk nokta nesnesinin adı: etiketi, yoksa Ad'ı; Türkçe büyük harfle karşılaştırılır.
  - Bulunmazsa: "“P9” adlı nokta çizimde yok. Adını denetleyin ya da Y,X yazın."
  - Sayılarda ilk virgül ondalık noktasıdır, boş değer yok sayılır, sayı olmayan geçersizdir.
- **Çizimden:**
  - Pencere kapanır, nokta alma aracı çalışır: "Aplikasyon: Durulan nokta (istasyon): haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]". Kenet ve yazılan koordinat çalışır.
  - Tık ya da yazılan nokta alana gelir: adı olan bir noktanın tam üstündeyse adı, değilse tam duyarlıkla "x,y".
  - Esc, Enter ya da kısa sağ tık hiçbir şeyi değiştirmez.
  - İki durumda da pencere kaldığı gibi yeniden açılır.
  - Araç katalogda değildir, yinelenmez. Noktayı ev sahibine `ViewChange::Picked` ile verir (`kentos_interaction::pick`).
- **Özet kutusu:** en çok altı sorun. Sonuç tablosu yalnız hesap geçerliyken görünür.
- **Rapor:** sekmeyle ayrılmış satırlar sistem panosuna yazılır, elektronik tabloya yapıştırılabilir.
  - İleti: "Aplikasyon raporu panoya kopyalandı (4 satır; elektronik tabloya yapıştırılabilir)."
  - Çizimin panosu uygulama içidir (ADR 0056); rapor düz metin olduğu için sistem panosuna gider.
- **Çizime ekle:**
  - tek adımdır, adı pencerenin başlığıdır;
  - her nokta adını etiket olarak, Ad, Tür ve Z (m) özniteliklerini taşır;
  - noktalar seçilir, pencere kapanır.
  - Kilitli katman: "“X” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin." Gizli katman yazıldıktan sonra söylenir: "“X” katmanı gizli; eklenen noktalar görünmüyor."
  - Başta web gibi belgeye doğrudan yazılıyordu. 27 Eylül'den beri iki platformda `cad.entities.create` ile yazılır (web `dbdacc1`). İşlemi pencerenin adını taşır: `forwardIntersection`, `resection`; Poligon hesabı ve Kutupsal alım için `traverse`, `polarSurvey` (ADR 0071). Kilitli ve gizli katman pencerenin kendi sözleriyle söylenir.
- **Yazılanlar** uygulama açık kaldıkça kalır; hangi çizim açık olursa olsun paylaşılır, kaydedilmez. Web'de de sayfa oturumu boyuncadır.

### Kestirme (`calc.forward`, `calc.resection`)

- **Pencere:** "Kestirme", 820 px. "Kestirme türü" (Önden, Geriden) ve seçilenin açıklaması.
- **Alanlar:**
  - A ve B noktası, Geriden'de C noktası;
  - α ve β açıları, projenin açı biriminde, etiketleri web'inkiler ("α: A'da B'den P'ye (g)");
  - "Yeni noktanın adı" (boşsa P).
- **Kroki:** web'in SVG çizimi, Iced canvas ile. Noktaların yeri ve hangi açının hangisi olduğu görünür; ölçekli değildir. Altında açıklaması vardır.
- **Uyarılar**, web'in sırasıyla: "A noktası verilmedi.", "B noktası: …", "C noktası verilmedi.", "α açısını yazın.", "β açısını yazın." ve çekirdeğin iletileri (üçgen kurmayan açılar, tehlike dairesi …).
- **Sonuç:**
  - "Y1 noktası hesaplandı."
  - Geriden'de sağlamlık 0,05'ten küçükse tehlike dairesi uyarısı.
  - Tablo: Nokta, Y (sağa), X (yukarı).
- **Alt çubuk:**
  - Raporu kopyala;
  - Katman: varsayılan `poligon`, yoksa etkin katman; kilitli katman "(kilitli)" yazar;
  - Çizime ekle: tür "Kestirme noktası", adım "Önden kestirme" ya da "Geriden kestirme";
  - Kapat.

### Aplikasyon (`calc.stakeout`)

- **Pencere:** "Aplikasyon", 860 px. Durulan nokta ve Bakılan nokta ("Verilirse ondan dönülecek açı da hesaplanır").
- **Tablo:** tek sütun "Aplike edilecek nokta (ad ya da Y,X)", başta üç satır.
  - Enter aşağı iner; sonda yeni satır açar.
  - Satır silinebilir, son satır kalır. "Satır ekle" vardır.
  - Elektronik tablodan çok satırlı yapıştırma sütunu aşağı doldurur, gerekirse satır ekler. Ayraçlar web'inkilerdir: sekme, noktalı virgül, iki ya da daha çok boşluk, sayıdan önce boşluk.
- **"Seçili noktaları ekle":**
  - seçili nokta nesnelerini adıyla (yoksa "x,y") dolu satırların ardına ekler;
  - seçimde nokta yoksa: "Seçili nokta yok: aplike edilecek noktaları seçip pencereyi yeniden açın."
- **Uyarılar:**
  - "Durulan nokta verilmedi." / "Durulan nokta: …" / "Bakılan nokta: …";
  - "2. satır: …";
  - "2. satırdaki nokta durulan noktayla aynı yerde; semt tanımsız." (web'in 15. görevi);
  - "Tabloya aplike edilecek en az bir nokta yazın.";
  - çekirdeğin iletileri.
- **Sonuç:**
  - "2 nokta için semt ve uzunluk hesaplandı."
  - Bakılan nokta yoksa: "Bakılan nokta verilmedi: dönülecek açılar yok, aleti semte göre yöneltin."
  - "Aplikasyon değerleri" tablosu: Nokta, Semt, Yatay uzunluk (m), Bakılan noktadan açı ("—" bakılan nokta yoksa).
- Çizime bir şey yazılmaz. Raporu kopyala ana düğmedir.

## Web'den ayrılanlar

- **Tablo klavyesi:** ↑ ve ↓ ile satırlar arasında gezinme yok; Enter aşağı iner. Iced'in yazı kutusu okları kendisi kullanıyor.
- **Katman seçimi:** web kilitli katmanı listede soluk gösterir. Masaüstünde "(kilitli)" yazar; seçilirse alınmaz.
- **Yapıştırma:** Iced yazı kutusu yapıştırılan metindeki satır sonlarını ve sekmeleri atar. Bu yüzden pano ayrıca okunur ve tablo olarak dağıtılır. Tek bir değer kutunun kendi yapıştırmasıyla kalır.

## Doğrulama

- `apps/desktop/src/calc/` testleri (8; değerler elle hesaplandı):
  - okuma: koordinatlar ve sayılar web'in kurallarıyla;
  - Önden kestirme (0, 0) ve (100, 0), 50 g ve 50 g → (50, −50); çizime ekleme, seçim, ileti, tek adım;
  - Geriden'in uyarılarının sırası;
  - Çizimden: yazılan noktanın alana gelmesi, adlı noktanın adı, Esc;
  - Aplikasyon'un semti, uzaklığı ve açısı;
  - durulan noktadaki hedef;
  - yapıştırmanın bölünmesi ve aşağı doldurması.
- `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm e2e:interaction`, `pnpm inventory:check`.
- Görüntüler (`calc::screens`, `.run/shots/hesap-*`), koyu ve açık, 1440×900 ve 1100×650:
  - Önden (sonuç);
  - Geriden (eksik alanlar);
  - Aplikasyon (iki nokta).
