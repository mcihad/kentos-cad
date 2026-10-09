# ADR 0209: Ağ analizi: tipli ağ modeli, en kısa yol, hizmet alanı, en yakın tesis, maliyet matrisi ve şebeke izleme

- **Durum:** kabul edildi (2026-10-09). Kapsamı sahibin GIS-10'daki sözüyle (“Benim seçmeme gerek yok sen sıradan devam et”) ben
  belirledim: ArcGIS Network Analyst'in ve Trace Network'ün, QGIS'in ağ analizi araçlarının ve pgRouting'in sık kullanılan işleri, KentOS'un
  kurallarıyla. İki platformda (masaüstü ve web), gerektiğinde bulut tarafıyla; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde
  tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-11`, ADR 0161 (İzle: çizgi işinde en kısa yol), ADR 0201 (Tampon ve birleştirme), ADR 0202 (topoloji
  kuralları: projede saklanan kurallar ve bulgu listesi), ADR 0084 (İşlemler), ADR 0100 (ifade dili), ADR 0199 (katman alanları), ADR 0149
  (ölçü doğruluğu ve gösterim), ADR 0026 ve 0033 (bulutta proje ayarları).

## Bağlam

Belediyeler ve altyapı kurumları yol ve şebeke verisini KentOS'ta çizer ve tutar; o veriyle her gün aynı sorular sorulur: iki adres arasında
en kısa ya da en hızlı yol, itfaiye istasyonunun beş dakikada ulaştığı yer, bir mahalleye en yakın okul ya da sağlık ocağı, bir boru
patladığında kapatılacak vanalar ve suyu kesilecek hatlar. ArcGIS bunu ağ veri kümesi (kenarlar, düğümler, yön, maliyet, kısıtlar) ve
çözücüleriyle, QGIS çizgi katmanından her çalıştırmada kurduğu grafla, Netcad şebeke modülleriyle yapar. KentOS'ta bu sorular için ne
veri modeli ne de hesap vardır. Yolun tasarım aksı (yatay ve düşey kurplar, istasyonlar; `CIVIL-*`) başka bir veridir: aks yolun nasıl
yapılacağını, ağ ise bugünkü yolların ve hatların birbirine nasıl bağlandığını anlatır. İkisi aynı veri tipine indirgenmez.

## Karar

### 1. Kapsam

- **Ağ modeli:** projenin ağları (`ProjectSettings.networks`): hangi çizgi katmanlarının kenar, hangi nokta katmanlarının düğüm olduğu,
  bağlanma kuralı ve toleransı, yön, maliyetler ve kısıtlar. Ağ katmanlardan türetilen analiz grafıdır, kalıcı bir kopya değildir: her
  analiz çizimin o anki nesnelerinden kurar. Aks (tasarım) ağın kaynağı olabilir (aks çizgisi kenar katmanındaysa), ama aks tanımı ağ
  modeline girmez.
- **Ağlar penceresi:** ağları tanımlama, düzenleme, silme ve Denetle (ağın sayıları ve sorunları, satırdan çizimde gösterme).
- **Etkileşimli araçlar:** En kısa yol (durakları tıklayarak; imleç bir sonraki durak gibi canlı izlenir), Hizmet alanı (tesisleri
  tıklayarak; aralıkların alanları canlı), Şebeke izleme (Bağlı, Akış yukarı, Akış aşağı, Yalıtım).
- **İşlemler araçları** (yeni kategori Ağ analizi): En yakın tesis, Maliyet matrisi, Hizmet alanları (tesis katmanından).
- **Komut:** `cad.network.define` (ağ tanımını yazar ya da siler); analizlerin sonuçları `cad.entities.create` ile yazılır.
- **Otomasyon:** Python `kentos.network`; MCP'de `cad.network.define`. Sunucu ağ tanımlarını projenin ayarlarıyla saklar ve kurallarını
  denetler.
- **Kapsam dışı (sonraki işler):** dönüş kısıtları ve cezaları, yol hiyerarşisi, zamana bağlı maliyet (trafik), araç rotalama (VRP),
  konum-tahsis, ağın elle düğüm düzenlemesi (bağlantı kuralı dışında), 3B uzunluk maliyeti, hidrolik hesap (`CIVIL-10`), ağın paftada
  çıktısı.

### 2. Veri modeli

`kentos_contracts::network`:

- **`NetworkDef`:** `id` (küçük harf, rakam ve tire, 1–40 karakter; projede tek), `name` (1–80 karakter, kırpılmış; projede tek),
  `kind` (`road` Yol ağı, `utility` Şebeke), `edges` (kenar katmanları, 1–16: `{ layer, filter? }`), `junctions` (düğüm katmanları, 0–16:
  `{ layer, role, filter?, closed? }`), `connect` (`ends` Uçlarda, `vertices` Köşelerde), `tolerance` (0,0001–10 m), `direction`,
  `costs` (0–8), `closed?` (kapalı kenarların ifadesi).
- **Düğümün rolü** (`role`): `junction` (Bağlantı: kenarı böler), `source` (Kaynak: depo, arıtma, trafo; Yalıtım'da beslemenin geldiği yer),
  `valve` (Vana: Yalıtım'da izin durduğu yer). Düğüm katmanının `closed` ifadesine uyan düğüm (kapalı vana) her analizde geçilmez.
- **Yön** (`direction`): `{ kind: "both" }` (her kenar iki yöne), `{ kind: "digitized" }` (her kenar çizildiği yöne: akışla çizilmiş borular),
  `{ kind: "field", field, forward, backward, closed }` (kenarın alanının değeriyle: `forward` listesindeki değer çizildiği yöne tek yön,
  `backward` ters yöne tek yön, `closed` iki yöne kapalı, başka değer ya da boş iki yöne açık). Değerler kırpılır, Türkçe büyük-küçük harf
  ayrımı olmadan karşılaştırılır; listelerde en çok 16 değer, değer en çok 40 karakter; bir değer iki listede olamaz.
- **Maliyetler** (`costs`): Uzunluk (metre, kenarın gerçek uzunluğu, yaylar kesin) her ağda vardır ve listede yazılmaz. Ek maliyet
  `{ name, kind, field, unit, speed? }`: `kind: "speed"` süre (dakika) = uzunluk / hız; hız `field`'dan km/sa, boş ya da okunamayan ya da
  sıfırdan küçük-eşit değerde `speed` (varsayılan hız, 0 < h ≤ 1000) alınır ve sayılır; `kind: "field"` kenarın bütününün maliyeti
  `field`'dan (birimi `unit`, en çok 12 karakter, boş olabilir); boş, okunamayan ya da eksi değerli kenar o maliyetle geçilmez ve sayılır.
  Sayılar `kentos.statistics/1`'in kuralıyla okunur (nokta ya da virgül). Adlar kırpılmış 1–40 karakter, tek, “Uzunluk” olamaz.
- Kenarın bir parçasının maliyeti uzunluğuyla orantılıdır (alan maliyeti kenara bölünmüş parçalara uzunluklarıyla dağılır).
- İfadeler (`filter`, `closed`) ifade dilindedir (ADR 0100), en çok 1000 karakter; uygulama değerlendirir, çekirdeğe yalnız sonuç gider.
  Değerlendirilemeyen nesne uymamış sayılır ve sayılır.
- Kurallar `network_problem` (ağ) ve `networks_problem` (liste: kimlik ve ad tekliği, en çok 32 ağ) okuyucularda, komutta ve sunucuda aynı.
  Katmanın çizimde olması kural değildir: silinmiş katman ağın kurulmasında söylenir.
- **`.kcad` şema 33** (`FORMATS_VERSION` 43): proje ayarlarının `networks`'ü; yalnız ağ varken şema 33 yazılır. Bağımsız Python okuyucusu
  ve yazıcısı, örnek `networks.kcad` ve bozuk dosyalar.

### 3. Ağın kurulması

Girdi: kenar katmanlarının süzgece uyan çizgi, çoklu çizgi (her parçası ayrı) ve yayları, düğüm
katmanlarının süzgece uyan noktaları (çok noktalı nesnenin her noktası). Başka türler alınmaz ve türüyle sayılır. Kenarlar katmanların
tanımdaki sırasıyla, katmanda belgenin sırasıyla dizilir.

1. **Bölme yerleri:** her kenarın iki ucu (`vertices`'te bütün köşeleri) kendi bölme yeridir. Bir uç başka bir kenara toleranstan
   yakınsa o kenar en yakın noktasından bölünür (T bağlantı); en yakın nokta o kenarın kendi ucunun tam kendisiyse ikinci bir bölme yeri
   açılmaz (o uç zaten bölme yeridir). Düğüm noktası toleranstan yakın her kenarı en yakın noktasından böler; hiçbir kenara yakın olmayan
   düğüm “ağın dışında”dır.
2. **Düğümler:** bölme yerleri ve düğüm noktaları, aralarındaki uzaklık toleranstan küçük-eşitse aynı düğümdür (geçişli: zincir boyunca
   birleşir). Düğümün yeri grubunun ilk üyesidir (kenarların sırasıyla, kenarda baştan sona; sonra düğüm noktaları sırasıyla).
3. **Parçalar:** her kenar bölme yerlerinde parçalanır; parça iki düğüm arasıdır, geometrisi kenarın kendi köşeleri ve yaylarıdır (uçtaki
   köşe düğümün yerine taşınmaz). Uzunluğu toleranstan küçük-eşit ve iki ucu aynı düğüm olan parça atılır. “Sıfır uzunluklu parça” verinin
   kendisidir: kenarın kendi bölme yerleri (uçları; Köşelerde köşeleri) arasında sıfırdan uzun, toleranstan kısa aralık ve uzunluğu sıfır
   kenar sayılır. Başka bir ucun ya da düğümün bir bölme yerinin yanında açtığı kısa artık (yayın ucu hesapla köşesinden bir kıl kısa
   düşer) sayılmadan atılır: yayın ucuna değen düz yol temiz bir çizimde sorun değildir.
4. **İçinden geçen kesişim bağlanmaz:** iki kenar uç ya da (Köşelerde) ortak köşe olmadan kesişiyorsa bağlı değildir (köprü, alt geçit,
   farklı derinlikte borular). Denetle bunları söyler.
5. Yön, maliyetler ve kapalılık parçaya kenarından geçer; tek yönlü kenarın parçaları kenarın çizildiği yöne göredir.

### 4. Konumlar, engeller ve arama

- **Konum:** bir nokta ağın en yakın yerine oturur (parça ve parçadaki uzaklık); arama uzaklığından uzaksa konumu yoktur. Etkileşimli
  araçlarda arama uzaklığı ekranda 20 pikseldir; İşlemler'de parametredir (varsayılan 100 m).
- **Arama:** Dijkstra; aynı maliyette düşük numaralı düğüm önce, bir düğüme eşit maliyetle ikinci kez ulaşmak öncekini değiştirmez.
  Konumlar parçanın içindeyse parça orada geçici olarak bölünür: aynı parçadaki iki konum arasında doğrudan gidilebilir. Ters arama (bir
  yere gelenler) parçaların iki yönünün maliyetini değiştirir.
- **Engel:** analizde tıklanan ya da verilen nokta; parçanın içindeyse o noktadan geçilmez (iki yanına ulaşılabilir), bir düğüme
  toleranstan yakınsa düğüm geçilmez. Kapalı kenar ve kapalı vana her aramada geçilmez.
- **Toplanan maliyetler:** yol seçilen maliyetle bulunur; yolun Uzunluk'u ve öbür maliyetleri aynı parçalar boyunca toplanır (o maliyetle
  geçilemeyen parça varsa o maliyet boş yazılır).

### 5. En kısa yol (rota)

Duraklar sırayla; her ardışık çiftin en kısa yolu bir bacaktır, rota bacakların birleşimidir (köşeleri ve yaylarıyla tek çoklu çizgi).
Ulaşılamayan bacak rotayı yazdırmaz, hangi duraklar arasında olduğu söylenir. **Sırayı iyileştir:** İlk durak sabit ya da ilk ve son
sabit; duraklar arasındaki maliyet matrisinden kesin en iyi sıra (Held–Karp), en çok 12 durak; eşit toplamlarda sözlük sırası küçük olan.
Yazılan: “Rota” katmanına (yoksa aynı adımda açılır) çoklu çizgi; öznitelikleri `Ağ`, `Maliyet` (seçilen maliyetin adı), `Duraklar`
(durak sayısı), `Sıra` (iyileştirilmişse ziyaret sırası, “1, 3, 2, 4”), `Uzunluk` ve her ek maliyetin adıyla toplamı.

### 6. Hizmet alanı

Tesisler; aralıklar artan artı sayılar (seçilen maliyetin biriminde, en çok 10); yön Tesisten ya da Tesise (ters arama); birden çok tesiste
Birleşik (her yerin maliyeti en yakın tesisinki) ya da Ayrı (her tesis kendi alanı).

- **Çizgiler:** ağın aralık içinde kalan kısımları: parçanın bir ucundan maliyetle ulaşılan uzunluğu (iki uçtan ve tesisin kendi
  parçasında tesisten) kesin aralıklarla; her çizgi en küçük aralığına yazılır (`Aralık` “0–300”).
- **Alanlar:** aralık başına, ulaşılan çizgilerin Kenar payı (varsayılan 50 m) kadar tamponlarının birleşimi (ADR 0201'in tamponu, yaylar
  kesin). Disk: aralığın bütün ulaşılanı; Halka: diskten bir öncekinin farkı.
- Yazılan: “Hizmet alanı” katmanına alanlar (`Ağ`, `Maliyet`, `Tesis`, `Başlangıç`, `Bitiş`), isteğe bağlı “Hizmet alanı çizgileri”.

### 7. En yakın tesis ve maliyet matrisi

- **En yakın tesis:** her olay için en yakın `k` tesis (1–10; isteğe bağlı üst sınır maliyet), yön Olaydan tesise ya da Tesisten olaya;
  eşit maliyette listede önce gelen tesis. Rotalar “En yakın tesis” katmanına (`Olay`, `Tesis`, `Sıra`, maliyetler); tablo.
- **Maliyet matrisi:** her başlangıçtan her varışa maliyet (isteğe bağlı üst sınır ve en yakın `k` varış); tablo (Başlangıç, Varış, Sıra
  ve seçilen maliyet; pano ve CSV) ve isteğe bağlı düz çizgiler.

### 8. Şebeke izleme

Başlangıç konumları (bir ya da birkaç), engeller:

- **Bağlı:** yön gözetmeden ulaşılan her şey.
- **Akış aşağı / Akış yukarı:** parçaların yönüyle (Akış yukarı ters yönle); iki yönlü parça iki yöne de izlenir.
- **Yalıtım:** başlangıçtan yön gözetmeden izlenir, açık vanalarda durulur; durulan vanalar kapatılacak vanalardır, izlenen kısım
  yalıtılan kısımdır. Ağda kaynak varsa, bu vanalar kapatılınca hiçbir kaynağa yön gözetmeden bağlı kalmayan kısım “beslemesiz kalan”dır.
- Sonuç seçimdir: izlenen parçaların nesneleri ve (Yalıtım'da) kapatılacak vanalar; satırı günlükte: izlenen uzunluk, nesne ve vana
  sayıları, vanaların adları (etiketi ya da `Ad` özniteliği; yoksa numarası).

### 9. Denetle

Ağın düğüm, parça ve toplam uzunluğu; bağlı bileşenleri (en büyüğü dışındakiler “kopuk parçalar”, uzunluklarıyla); sorunlar, yerleriyle:
“Yakın ama bağlı değil” (iki uç arası tolerans ile 1 m arasında), “Bağlanmayan kesişim” (§3.4), “Ağın dışında düğüm”, “Sıfır uzunluklu
parça”, “Okunamayan değer” (yön, hız ya da maliyet alanı), “Alınmayan nesne” (türü). Çıkmaz uç sayılır (yol ağında doğaldır), sorun
değildir.

### 10. Arayüz

- **Ağlar** (`network.manage`): sol listede ağlar (Ekle, Sil), sağda tanım: Ad, Tür, Kenar katmanları (katman ve isteğe bağlı süzgeç, ε),
  Düğüm katmanları (katman, rol, süzgeç, kapalı), Bağlanma ve Tolerans, Yön (Yok, Çizim yönü, Alan; alanla ileri, geri, kapalı değerleri;
  türün varsayılanları), Maliyetler tablosu, Kapalı kenarlar (ε); Denetle sonuçları altta (satır çizimde gösterir); Kaydet
  `cad.network.define` ile.
- **Araçlar:** En kısa yol (`tool.netRoute`), Hizmet alanı (`tool.netServiceArea`), Şebeke izleme (`tool.netTrace`); seçenekleri komut
  satırında: Ağ (A), Maliyet (M), Engel (E; sonraki tıklama engel koyar), En kısa yol'da Sıra (S), Hizmet alanı'nda Aralıklar (R), Yön (Y),
  Biçim (B), Birleşik (İ), Kenar payı (K), Çizgiler (Ç), Şebeke izleme'de Tür (T). Esc son noktayı geri alır, Enter yazar (izlemede seçer).
  Ağı olmayan projede araç Ağlar'ı açmayı önerir. Seçenekler oturum boyunca hatırlanır; Aralıklar maliyetin türüne göre ayrı (uzunluk
  metrede, süre dakikada, alan maliyeti kendi biriminde): Süre'ye geçmek uzunluğun aralıklarını dakika saymaz.
- **İşlemler:** yeni parametre türü `network` (Ağ ve Maliyet iki seçimde; projede ağ yoksa Ağlar… düğmesi).
- **Şerit:** CBS'nin Analiz sekmesinde **Ağ analizi** paneli: Ağlar, En kısa yol, Hizmet alanı, Şebeke izleme ve İşlemler'in üç aracı
  (İşlemler'in Ağ analizi kategorisi bu sekmede ayrı panel olmaz; araç kutusunda kategoridir). Sekme 1100 px'e sığsın diye iki panel çifti
  birleşti: Arazi ile Arazi analizi tek **Arazi** (Aplikasyon, Kot noktası, eş yükselti, profil, hacim, eğim), Topoloji ile Karşılaştırma
  tek **Denetim** (ADR 0202 §8'in “verinin denetimleri bir arada”sı). CAD şeridinde ve menülerinde yoktur (komut aramasında bulunur).
- **İkonlar** (sahibin sözüyle sorulmadan seçildi): Ağlar `networks` (düğümler ve kenarlar), En kısa yol `netRoute` (sokak ızgarasında iki
  durak arası yol), Hizmet alanı `netServiceArea` (tesis ve ulaştığı iki kuşak), Şebeke izleme `netTrace` (boru, dalı ve vana), En yakın
  tesis `closestFacility` (olay ve iki tesis, yakınının yolu kalın), Maliyet matrisi `odMatrix` (her başlangıçtan her varışa), Hizmet
  alanları `serviceAreas` (iki tesisin örtüşen alanları).

### 11. Komutlar ve otomasyon

- `cad.network.define` v1: `{ operation: "set" | "remove", network?, id?, expectedRevision? }`; `set` aynı kimlikli ağı değiştirir ya da
  sona ekler, `remove` siler. Kodlar `invalid_network`, `unknown_network`; çizimde olmayan katman uyarıdır. Proje ayarıdır: geri alma
  adımı değildir (topoloji kuralları gibi), çizimi kirli yapar.
- Python: `kentos.network`: `networks`, `define`, `remove`, `check`, `route`, `service_area`, `closest_facility`, `od_matrix`, `trace`
  (her biri ağı yeniden kurar) ve bir kez kurulup çok sorulan `Network` sınıfı (aynı yöntemler, `locate`); sonuçlar dataclass, yolun
  `Line.geometry()`'si ve alanın `shape`'i `cad.entities.create`'e olduğu gibi gider. Ağ, İşlemler'in araçlarındaki gibi çizimden kurulur
  (`kentos_processing::network::from_document`); ret uygulamaların sözüyle `NetworkError`'dur. MCP'de `cad.network.define`.

### 12. Performans

Sahibin önceliği performanstır (“Biliyorsun önceliğimiz performans”, 9 Ekim):

- **Kurma:** ağ, geometri deposundaki şekillerden (masaüstünde çizimin deposu, web'de nesnelerin tipli paketi; JSON yok) bir kez kurulur;
  çizimin sürümü değişmedikçe yeniden kurulmaz. Düğümler ızgara karmasıyla birleştirilir, komşuluk sıkıştırılmış satırlarda (CSR),
  parçalar paketlenmiş R-ağacında.
- **Arayüz beklemez:** kurma, aramalar ve hizmet alanının alanları masaüstünde ağın iş parçacığında, web'de ağın işçisinde çalışır;
  araç sonucu gelince önizlemesini değiştirir, yenisi istenince eskisi bırakılır. İşlemler'in araçları kendi iş parçacığında ya da
  işçisinde kurar.
- **Fare hareketi:** imleç her kımıldadığında yeni arama yoktur: tıklanan son yerin arama ağacı tutulur, imlece yol ağaçtan okunur
  (konum R-ağacından, yol öncüllerden geriye). Önizlemenin tamponları yeniden kullanılır.
- **Aramalar:** Dijkstra'nın uzaklık ve öncül dizileri sorgular arasında tutulur (nesil sayacıyla sıfırlanmadan), yığın yeniden kullanılır;
  üst sınır verilen aramada sınırı aşan düğüm açılmaz.
- **Hizmet alanının alanları:** tamponların birleşiminde bir yerin içeride olması yereldir: bir çekirdek çizgiye Kenar payından yakın mı.
  Kaplama bunu her parça için yakındaki çekirdeklere sorar (bütün düzeni kesen bir ışında dolanım saymaz); tek bir komşu tamponun içinde
  kalan düz kenar düzenlemeye hiç girmez; düz çekirdeğin tamponu dikdörtgendir, ucundaki yarım daire yalnız o uçta biten öbür düz
  çekirdeklerin dikdörtgenleri onu örtmüyorsa (çıkmaz uç, köşe) eklenir.
- **Bütçeler** (sentetik ızgara şehir, ≥ 100 000 parça, rastgele tek yönler ve hızlar; release): kurma ≤ 150 ms; konum ≤ 0,05 ms; bütün
  ağda Dijkstra ≤ 30 ms; arama ağacından imlece yol ≤ 0,1 ms; izleme ≤ 20 ms; 10 dakikalık hizmet alanının çizgileri ≤ 30 ms, alanları
  araçların Kenar payıyla ≤ 300 ms (önce çizgiler görünür); Denetle ≤ 1 s. Ölçümler Doğrulama'da.

## Uygulama

- **Sözleşme** `crates/shared/contracts/src/network.rs`: `NetworkDef` ve parçaları, `network_problem`, `networks_problem`, `cost_names`,
  sınırlar; `cad_networks.rs`: `cad.network.define`'ın girdisi ve çıktısı; `ProjectSettings.networks`. TS tipleri üretilir
  (`contracts/generated/Network*.ts`, `Junction*.ts`). Kuralların bağımsız başvurusu `network_rules_cases.py` (ortak
  `fixtures/network/v1/rules.json`; Rust `tests/all/network_rules.rs`, web `model/networkRules.ts`); Ağlar penceresinin form kuralları
  `network_form_cases.py` (`form.json`; web `model/networkForm.ts`, masaüstü `kentos_interaction::network::form`).
- **`.kcad` şema 33** (`FORMATS_VERSION` 43): `crates/shared/kcad/src/{encode,decode}/networks.rs`, `docs/specs/kcad-v2.md`; bağımsız
  Python okuyucu ve yazıcısı (`tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`); örnek `fixtures/kcad/v2/networks.kcad`,
  25 bozuk dosya (`broken/network-*`, `networks-*`, `schema-version-34`; eski `schema-version-33` artık geçerli sürümdür).
- **Çekirdek** `crates/shared/geometry-core/src/ops/network/`: `input` (kenarlar, düğümler, değerler; depodan yollar), `graph` (bölme
  yerleri, ızgarayla düğüm birleştirme, parçalar, CSR, R-ağacı, kısa parça kuralı), `search` (Dijkstra, geçici bölmeler, engeller, ters
  arama, nesil sayacı), `route` (bacaklar, Held–Karp), `area` (hizmet alanının çizgileri ve alanları), `closest` (en yakın, matris),
  `trace` (dört izleme, besleme), `check` (Denetle), `session` (tutulan ağ, JSON sarmalayıcıları), `rules`. Tamponların birleşimi:
  `ops::geoprocess::buffer::union_pieces` (dikdörtgenler, açıkta kalan uçlarda yarım daireler) ve `geom::overlay::overlay_buffers`
  (içerisi “bir çekirdeğe Kenar payından yakın” sorusuyla, `geom::arrangement`'ın `candidate_pairs` ve `classify_with`'iyle). WASM
  `crates/wasm/geometry-wasm/src/network.rs`. Bağımsız başvuru `network_cases.py` (kesirler ve mpmath; ortak `fixtures/network/v1/cases.json`).
- **Komut** `cad.network.define` v1: web `product/networkDefine.ts`, masaüstü `crates/native/application/src/network_define.rs`; durumlar
  `fixtures/commands/v1/cad.network.define.json` (başvuru `network_define_command_cases.py`); başsız sunucu, Python
  (`kentos.cad.network.define`) ve MCP. Sunucu projenin ayarlarındaki ağları aynı kuralla denetler (`crates/server/application`'ın
  `projects.rs` ve `changes.rs`'i).
- **İşlemler:** kategori Ağ analizi, parametre türü `network` (Ağ ve Maliyet, projede ağ yoksa Ağlar…); web
  `processing/builtin/network/` (`closestFacility.ts`, `odMatrix.ts`, `serviceAreas.ts`, `shared.ts`), masaüstü `kentos-processing`'in
  `builtin/network/`'ü ve `network.rs`'i (`from_document`: İşlemler ve Python ağı çizimden böyle kurar); durumlar
  `fixtures/processing/v1/network.json` ve `network.kcad` (başvuru `network_processing_cases.py`).
- **Web:** ağın işçisi `io/network/` (`worker.ts`, `protocol.ts`, `handle.ts`), servis `app/networks.ts` (çizimin sürümüyle kurma, meşgul
  işareti), komutlar `app/networkCommands.ts`, araçlar `tools/networkTool.ts` (ortak taban), `networkRouteTool.ts`, `networkAreaTool.ts`,
  `networkTraceTool.ts`, sonuç katmanı `tools/resultLayer.ts`, pencere `ui/networks/NetworksDialog.ts` ve `styles/networks.css`, girdiler
  ve yanıtlar `model/networkInput.ts`, `networkRead.ts`, `networkAnswers.ts`; şeridin `omit`'i (`app/ribbon.ts`).
- **Masaüstü:** `apps/desktop/src/networks/` (`engine.rs` ağın iş parçacığı, `window.rs` Ağlar penceresi, `mod.rs` durum, çizimi izleme
  ve `networks_reset`), araçlar `kentos_interaction::network` (`route`, `area`, `trace`, `base`, `ask`, `result`, `form`), İşlemler'in Ağ
  satırı `processing/fields.rs`, İfade oluşturucu formun ifadelerinde (`expression/mod.rs`); KentOS UI'ın `Dialog::fill`'i (gövdeyi
  dolduran parça).
- **Python** `kentos.network` (`python/kentos/network.py`: dataclass sonuçlar, `Network` sınıfı), yerel `Network`
  `crates/native/python/src/network.rs`, testler `python/tests/test_network.py`.
- **Sahne ve izler:** `fixtures/interaction/v1/networks.kcad` (`network_scene.py`: yaylısıyla 17 yol çizgisi, 3 okul, itfaiye,
  5 su hattı, 5 vana ve depo; 32 nesne), ortak izler `network-route.json`, `network-service-area.json`, `network-trace.json` (web oynatıcısı ağın işçisini
  bekler); resimler masaüstünde `networks::tests::screens` (`.run/shots/aglar-*`), web'de `shots.mjs networks`; `e2e:layout`'ta Ağlar'ın
  boş ve yeni ağ hâlleri ve İşlemler'in ağ aracı.

## Doğrulama

9 Ekim 2026, geliştirme makinesinde (Linux, derleme 4 işle):

- **Bağımsız başvurular** (KentOS kodu olmadan, `--check` ile hepsi geçer): `network_cases.py` (5 sahne, 29 soru: graf, konumlar,
  rotalar ve sıraları, engeller, hizmet alanının kesin çizgileri ve shapely'yle alanları, en yakın tesis, matris, dört izleme, Denetle;
  kesirler ve mpmath), `network_rules_cases.py` (64 ağ ve 8 liste, bağımsız KCAD okuyucusunun kurallarıyla), `network_form_cases.py`
  (25 durum), `network_define_command_cases.py` (21 komut durumu), `network_processing_cases.py` (11 çalıştırma), `network_scene.py`,
  `kcad_v2_reference.py` (452 dosya; örnek `networks.kcad` ve 25 bozuk dosya `tools/kcad/kcad.py` ile), `geoprocess_cases.py`
  (tamponların birleşimi değişince ADR 0201'in durumları aynı).
- **Çekirdek ve platformlar:** geometri çekirdeği `cases.json`'un bütün sorularını başvurunun toleranslarıyla verir; web aynısını
  WASM'dan (`app/networks.wasm.test.ts`); komut durumları web, masaüstü ve Python'da; İşlemler'in durumları iki platformun
  çalıştırıcılarında, masaüstünde çizimin kopyasında da; üç ortak iz iki platformda bütün varyantlarda.
- **Sunucu:** `check_networks`'ün birim testi ve geçici veritabanında (`KENTOS_TEST_DB=required`) HTTP testi
  (`http::networks_tests`): geçersiz ağla proje açılmaz (`settings.networks`), geçerlisi saklanır ve okunur; `project.changes`'te aynı
  ad iki kez reddedilir (`project.settings.networks`) ve hiçbir şey değişmez; ağsız ayar ağları kaldırır.
- **Takım:** `pnpm typecheck`; `pnpm test` (344 dosya, 4089 test geçti, 21 atlandı); `pnpm rust:test` (2824 geçti, 27 yok sayıldı;
  API'nin veritabanı testleri geçici veritabanlarında çalıştı, atlanan yok; clippy ve bağımlılık yönü temiz); `pnpm rust:test:desktop`
  (1255 geçti, 193 yok sayıldı; clippy'nin üç bulgusu düzeltildi, biri GIS-10'un `feed_window.rs`'inden, sonra temiz); `cargo test -p
  kentos-mcp` (7); `pnpm py:test` (47); `pnpm e2e:interaction` (136 iz × 3 varyant); masaüstünde 136 iz bütün varyantlarda; `pnpm e2e`
  (209 denetim); `pnpm build` (ağın işçisi ve Ağlar penceresi ayrı parçalar); `pnpm inventory:check`; `e2e:layout`'ta Ağlar'ın üç
  maddesi ve CBS'nin sekmeleri 1100×650 ve 1440×900'de, iki temada (44 görünüm, sorunsuz). Analiz sekmesi masaüstünde
  `ribbon_tests`'le 1100 px'e sığar.
- **Resimler:** masaüstü `networks::tests::screens` ve web `shots.mjs networks` (Ağlar: Yollar ve İçme suyu Denetle'yle, En yakın tesis,
  Maliyet matrisi, Hizmet alanları, şerit) iki boyutta ve iki temada; araçlar kullanılırken ortak izlerin `shot` adımlarıyla
  (`kentos-cad kullan`, `e2e:use`, yan yana `scripts/usage/compare.py`).
- **Süreler** (release; `network::timing`: 225 × 225 kavşaklı ızgara şehir, 50 m bloklar, 100 800 parça ve 50 625 düğüm, rastgele tek
  yönler, hızlar ve kapalı sokaklar; sorular ortadan):

  | İş | Süre | Bütçe |
  |---|---|---|
  | Kurma | 118,3 ms | 150 ms |
  | Konum, bir nokta (10 000 noktanın ortalaması) | 0,001 ms | 0,05 ms |
  | Bütün ağda Dijkstra (Süre) | 8,9 ms | 30 ms |
  | Arama ağacından imlece yol, bir nokta | 0,007 ms | 0,1 ms |
  | Köşeden köşeye rota (20,7 dk) | 6,0 ms | 30 ms |
  | İzleme, bağlı (99 754 nesne) | 19,9 ms | 20 ms (sınırda) |
  | 10 dakikalık hizmet alanının çizgileri (98 848) | 26,1 ms | 30 ms |
  | Aynısının alanları, Kenar payı 50 m | 283,5 ms | 300 ms |
  | Aynısının alanları, Kenar payı 25 m (tamponlar her blokta değer) | 826,5 ms | 300 ms: aşar, `GEO-02` |
  | 1, 2 ve 4 dakikalık alanlar, 50 m | 18,5, 36,8 ve 84,4 ms | 300 ms |
  | Denetle | 77,8 ms | 1 s |

  `network_timing` (aynı şehir bir çizim olarak): çizimden okuma (ifadeler ve alanlar) 35,6 ms ve kenarların yolları depodan 22,0 ms
  masaüstünde arayüzün iş parçacığında, ağın iş parçacığına geçmeden; İşlemler'in ve Python'un çizimden bütün kurması 231,8 ms.
- **Bilinen sınırlar:** `GEO-01` (kaplamanın birleşimi, kendi sonucu değen dairelerle yeniden birleşince alanı iki kez sayar; hizmet
  alanı her kuşağı kendi tamponlarından birleştirdiği için etkilenmez, testi yok sayılı), `GEO-02` (tam değen tamponlar); dönüş
  kısıtları ve öbürleri §1'in kapsam dışısı.
