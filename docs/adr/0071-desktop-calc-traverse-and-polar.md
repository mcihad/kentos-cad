# ADR 0071: Masaüstünde Hesap pencereleri (2): Poligon hesabı, Kutupsal alım ve ölçü tablosu

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.7, §5, §23; TODOS.md `UI-11`; ADR 0070 (Hesap pencereleri, 1. kısım)
- **Sahibin yönü (26 Eylül):** web'deki araçlar ve pencereler masaüstüne birebir taşınır.
- **Web ajanının tarifi (27 Eylül, 13. ve 15. görev):**
  - kaynak: `ui/calc/common.ts` (`Grid`), `read.ts` (`readPolar`), `TraverseDialog.ts`, `PolarDialog.ts` ve çekirdeğin `survey::traverse`, `survey::polar` modülleri;
  - 15. görevin iki düzeltmesi (`af28188`) Kutupsal alımdadır: çekirdeğin iletisi tablonun satırını söyler; sayı olmayan istasyon kotu ve alet yüksekliği reddedilir.

## Bağlam

- ADR 0070 Kestirme ve Aplikasyon'u taşıdı; `calc.traverse` ve `calc.polar` masaüstünde hâlâ "web'de var; masaüstüne henüz taşınmadı" diyordu.
- İki pencerenin de hesabı ortak çekirdekteydi: `survey::traverse::traverse` ve `survey::polar::polar_survey`. Masaüstü ikisini `kentos_interaction::survey` üzerinden doğrudan çağırır.
- Web'in iki penceresi çok sütunlu bir ölçü tablosu (`Grid`) kullanır. Aplikasyon'un ADR 0070'teki tek sütunlu tablosu bu tablonun özel bir hâliydi.
- Web'in `e2e:layout` görüntüleriyle karşılaştırınca ADR 0070'teki pencerelerin yerleşimi web'inkinden ayrılıyordu.

## Karar

### Ölçü tablosu (`apps/desktop/src/calc/grid.rs`)

- **Tablo sözleşmesi:** web'in `GridModel`'i `Table` özelliğidir:
  - sütun ve satır sayısı, hücrenin değeri;
  - sabit hücre (`readonly`);
  - satırdan sonra yeni satır açılıp açılamayacağı ve açılması;
  - satırın silinip silinemeyeceği ve silinmesi.
- Poligon hesabı, Kutupsal alım ve Aplikasyon bu sözleşmeyle aynı tabloyu kullanır.
- **Görünüş:** web'in `calc-grid`'i gibi.
  - Çerçeveli, alan renginde bir kutudur. Başlık satırı başlık renginde ve kutunun üstünde sabittir.
  - Satırlar ince çizgiyle ayrılır ve 280 px'ten sonra kayar.
  - Başlıklar sola dayalıdır, birim soluk yazılır ("Kırılma açısı (g)").
  - Hücreler kenarsızdır; yazılan hücrede vurgu kenarı çıkar. Sayı hücreleri sağa dayalıdır, rakamları eş aralıklıdır.
  - Okunamayan sayının kenarı kırmızıdır (web'in `data-bad`'i).
  - Sabit hücre soluk ve kalın yazılır; boşsa "—" gösterir.
  - Satırın sonunda "Satırı sil" ipuçlu × düğmesi, altta "Satır ekle" vardır.
- **Enter:** aynı sütunda açık olan sonraki satıra iner. Sonrası yoksa ve açılabiliyorsa yeni satır açar, oraya geçer.
- **Satır ekle:** yeni satır açılabilen son satırdan sonra açılır. İmleç yeni satırın ilk açık hücresine geçer.
- **Yapıştırma:** birden çok satır ya da hücre, yapıştırılan hücreden aşağı ve sağa dağılır.
  - Satırlar gerektikçe açılır. Sabit hücre atlanır; poligonun bitiş satırından önce yeni satır açılır.
  - Ayraçlar web'in `/\t|;|\s{2,}|\s(?=[-+\d.])/`'idir: sekme, noktalı virgül, iki ya da daha çok boşluk, sayıdan önce tek boşluk.
  - Boşluk ve kırpma JavaScript'in tanımıyla yapılır (`is_js_space`, `js_trim`).
  - Tek bir değer kutunun kendi yapıştırmasıyla kalır.

### Poligon hesabı (`calc.traverse`)

- **Pencere:** "Poligon hesabı", 940 px.
- **Poligon türü:** Bağlı, Kapalı ya da Açık. Seçeneğin açıklaması, web'deki gibi üstünde durunca ipucu olarak görünür.
- **Bitiş anahtarı:** yalnız Bağlı'da "Bitişte yöneltme açısı ölçüldü" anahtarı vardır; açıktır.
- **Bilinen noktalar:**
  - Başlangıç noktası (A);
  - Başlangıçta bakılan nokta ("İlk açı bu noktadan ölçülür");
  - Bağlı'da Bitiş noktası (B);
  - Bağlı'da anahtar açıkken Bitişte bakılan nokta ("Son açı bu noktaya ölçülür").
- **Tablo:** Nokta, Kırılma açısı, Sonraki noktaya kenar.
  - İlk satır başlangıç istasyonudur. Adı sabittir: noktanın adı, yoksa "A".
  - Sonra yeni noktalar gelir: başta iki boş satır, adları boşsa P1, P2 …
  - Bağlı ve Kapalı'nın son satırı bitiş istasyonudur. Adı sabittir: B'ninki ya da "B"; Kapalı'da başlangıcınki. Kenarı yoktur; açısı yalnız Kapalı'da ve yöneltmeli Bağlı'da yazılır.
  - Açık'ta son yeni noktanın açısı ve kenarı yoktur.
  - Sondaki boş satırlar hesaba girmez.
- **Uyarılar** web'in sırasıyla:
  - "… verilmedi.";
  - "Başlangıç satırında / 2. satırda kırılma açısı yok. / bir sayı değil.";
  - "… kenar uzunluğu yok.";
  - "Son satırda (başlangıca dönüşte) kırılma açısı yok." ya da "Bitiş satırında kırılma açısı yok.";
  - "Açık poligonda en az bir yeni nokta olmalı.";
  - çekirdeğin iletileri.
- **Özet:**
  - "Açı kapanma hatası fβ = 0.00095 g (9.5 cc); her açıya -0.00024 g (-2.4 cc) düzeltme verildi." (derecede ″ ile);
  - yöneltmesiz Bağlı'da "Bitişte yöneltme yok: açı kapanması denetlenmedi.";
  - "Koordinat kapanma hatası fy = …, fx = …, fs = … mm; kenarlara uzunluklarıyla orantılı dağıtıldı (toplam 39.018 m, 1/10764)." (fs sıfırsa oran yazılmaz);
  - Açık'ta "Açık poligon: kapanma denetimi ve dengeleme yok (toplam …).";
  - her zaman "Hata sınırı uygulanmaz: kapanma hatalarını ölçü sınıfınızın sınırlarıyla karşılaştırın."
  - **Ek (4 Ekim 2026, ADR 0169 §3, 4b):** bu satırın yerine kapanmalar projenin toleranslarıyla karşılaştırılır (Proje ayarları ›
    Ölçme › Poligon: Açı kapanması, Koordinat kapanması): “Açı kapanma hatası toleransı (… cc) aşıyor.” uyarı, “… toleransın (…)
    içinde.” bilgi, fs için de; tolerans verilmediyse “Hata sınırı verilmedi (Proje ayarları › Ölçme): kapanma hatalarını ölçü
    sınıfınızın sınırlarıyla karşılaştırın.” Raporun kapanma satırları Tolerans, değeri ve aşıyor ya da içinde ile biter. Karar
    çekirdekte `survey::traverse::closure`.
  - Yuvarlanınca −0 çıkan değer 0 yazılır (web'in `unsigned`'ı).
- **Sonuç tablosu:** Nokta, Semt, Kenar, ΔY, ΔX, Y, X. Her kenar ulaştığı noktanın adıyla yazılır; bilinen bitişin Y ve X'i "bilinen"dir.
- **Rapor:** vY ve vX sütunlarıyla, kapanma satırları sonda.
- **Çizime ekle:**
  - Varsayılan katman `poligon`'dur, yoksa etkin katmandır.
  - Tür "Poligon noktası", adım "Poligon hesabı": `cad.entities.create`, işlem `traverse` (web `dbdacc1`).
  - İleti: "Poligon hesabı: 3 poligon noktası çizime eklendi (Ctrl+Z geri alır)."
  - Yeni nokta yoksa kapalıdır.

### Kutupsal alım (`calc.polar`)

- **Pencere:** "Kutupsal alım", 940 px.
- **Bilinen noktalar:** Durulan nokta (istasyon) ve Bakılan nokta ("Alet bu noktaya yöneltilir").
- **Sayılar:** 130 px'lik alanlar.
  - "Bakılan noktanın okuması (g)", başta 0;
  - "İstasyon kotu (m)", ipucu "kot yoksa boş";
  - "Alet yüksekliği (m)", ipucu 0.
- **Tablo:** Nokta (ipucu satırın numarası), Yatay açı okuması, Uzunluk, Başucu açısı ("yatay uzunluksa boş"), Reflektör yüksekliği. Başta üç satır; boş satırlar atlanır.
- **Okuma ve denetim** (`read::read_polar`, web'in `readPolar`'ı birebir):
  - bilinen noktalar;
  - okumanın, istasyon kotunun, alet yüksekliğinin sayı olması;
  - "Tabloya en az bir nokta yazın.";
  - satır satır okuma, uzunluk, "bir değer sayı değil";
  - çekirdeğin "2. noktanın …" iletisi tablonun satırıyla yazılır ("3. noktanın uzunluğu sıfırdan büyük olmalı.").
- **Özet:**
  - "3 nokta hesaplandı, 2 noktanın kotu ile.";
  - kot farkı olup istasyon kotu yoksa "İstasyon kotu verilmedi: yükseklik farkları hesaplandı, kotlar yazılmadı."
- **Sonuç tablosu:** Nokta, Semt, Yatay uzunluk, Y, X, Z. Z kot, "Δ fark" ya da "—" olabilir.
- **Ek (4 Ekim 2026, ADR 0169 §3):** kot farkına yer eğriliği ve refraksiyon düzeltmesi, (1 − k)·D²/2R, projenin k'sıyla uygulanır
  (Proje ayarları › Ölçme, varsayılan 0,13); tablonun üstündeki açıklama k'yı söyler. Çekirdekte `PolarInput`'un `refraction`'ı; karnenin
  indirgemesiyle aynı terim (`survey::fieldbook::curvature`).
- **Çizime ekle:**
  - Varsayılan katman etkin katmandır.
  - Tür "Alım noktası"; kot noktanın Z'si ve "Z (m)" özniteliği olur. `cad.entities.create`, işlem `polarSurvey`, adım "Kutupsal alım".
  - İleti: "Kutupsal alım: 3 nokta çizime eklendi (Ctrl+Z geri alır)."

### Ortak parçalar web'in yerleşimine getirildi (dört pencere)

- **Bilinen noktalar** web'in `calc-knowns` ızgarası gibi satırda üçtür; kısa satır sütunlarını korur (Aplikasyon, Poligon, Kutupsal). Kestirme'de tek sütundur.
- **"Çizimden"** düğmesinde web'in ipucu vardır: "Çizimde gösterin (bir noktaya kenetlenirse adı alınır)".
- **Alanlar:**
  - Kestirme'nin α ve β alanları 130 px, "Yeni noktanın adı" 280 px'tir.
  - Sığmayınca alt satıra geçerler (web'in `io-row`'u).
  - Kroki yanda, başlık renginde ve kenarlı bir kutudadır. Kutu alanlar kadar uzar, kroki kutunun genişliğine göre çizilir.
- **Tür seçimi:** Kestirme ve Poligon türü seçeneklerinin açıklaması ipucudur. KentOS UI'da `Segmented::hints` bunun için eklendi.
- **Gövde ve alt çubuk:**
  - Pencerenin gövdesi kayar; başlık ve alt çubuk görünür kalır. Pencere en çok 880 px boydadır.
  - Alt çubuğun "Katman" yazısı web'deki gibi soluktur.
- **Katman seçimi** web'in `layerChoice`'u gibidir:
  - pencere açılırken seçilen katman çizimde varsa kalır;
  - yoksa pencerenin kendi varsayılanı seçilir: `poligon`, yoksa etkin katman (Kutupsal'da etkin katman).

## Web'den ayrılanlar

- **Tablo klavyesi:** ↑ ile satır yukarı çıkılmıyor; ↓ Enter'in işini yapmıyor. Iced'in yazı kutusu okları tabloya bırakmıyor. Web'de ikisi de var, açık iş olarak kaldı. (27 Eylül, düzeltme: yazı kutusu okları bırakıyordu; onları açık pencerenin tuş yönlendirmesi yutuyordu. İkisi [ADR 0075](0075-desktop-layer-search-and-keys.md)'te geldi.)
- **Okunamayan sayı:** masaüstünde tablo her çizildiğinde işaretlenir. Web'de işaret yazarken konuyordu, yapıştırma ya da satır ekleyince siliniyordu. Web ajanı bunu 12. görevle düzeltti (`1fd4120`); iki taraf aynı olacak.
- **İpucu yazı tipi:** sayı hücresi boşken ipucu arayüzün yazı tipiyle yazılır, değer yazılınca eş aralıklı rakamlara geçer. Web sayıları arayüz yazı tipinin eş genişlikli rakamlarıyla yazar; masaüstünün yazı tipinde bu seçenek yok.

## Doğrulama

- `apps/desktop/src/calc/` testleri (değerler elle hesaplandı):
  - **Kapalı poligon:** kare, 100 m kenar, P2'de 40 cc hata.
    - Satırlar tablodan yapıştırılır; bitiş satırından önce yeni satır açılır.
    - Bitiş satırının hücreleri sabittir.
    - fβ = 0.004 g, her açıya −0.0008 g düzeltme verilir; dengelenmiş kenarlar başlangıçta kapanır.
    - Noktalar 1 cm içinde köşelerdedir; raporun açı satırı "0.00400 g (40.0 cc)".
    - Çizime tek adımda eklenir, noktalar seçilir; ileti ve geri alma adı doğrulanır.
  - **Açık poligon:** sabit hücreler ve uyarıların sırası; Enter'le satır açma ve satır silme; başlangıç satırı silinmez.
  - **Kutupsal alım:** yatay nişanda eğik uzunluk yatay uzunluğa eşittir, kot istasyondan gelir. "Z (m)" ve Tür öznitelikleri, ileti.
  - **Katman seçimi:** açılışta seçilir, değiştirilen katman yeniden açılışta kalır.
  - **Web'in `read.test.ts`'i:** çekirdeğin iletisi tablonun satırını söyler; sayı olmayan kotlar reddedilir; kot hesabı.
  - **Tablo:** yapıştırmanın bölünmesi (sekmeli boşluklar, baştaki boşluklar), aşağı ve sağa dağılması, sabit hücrenin atlanması.
  - **ADR 0070'in testleri** yeni tabloyla geçer.
- Denetimler: `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm e2e:interaction`, `pnpm inventory:check`.
- Görüntüler (`calc::tests::screens`, `.run/shots/hesap-*`), koyu ve açık, 1440×900 ve 1100×650:
  - Poligon (bağlı, sonuçlu);
  - Poligon (bağlı, projenin toleranslarıyla: açı kapanması aşıyor, fs içinde; `hesap-poligon-tolerans`, ADR 0169 4b);
  - Poligon (açık, uyarılar ve kırmızı hücre);
  - Kutupsal alım (üç nokta, kotlar);
  - Önden kestirme, Geriden kestirme ve Aplikasyon (yeni yerleşimle).
