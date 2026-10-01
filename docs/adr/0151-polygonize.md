# ADR 0151: Çizgi ağından toplu alan

- **Durum:** kabul edildi (2026-10-01). Sıra sahibin kararıdır: TODOS.md §16.0'ın ikinci işi `HYB-02`, `HYB-01`'in (ADR 0148) üstüne. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-01
- **Bağlam belgesi:** TODOS.md `HYB-02` (ilgili: `HYB-01`, `HYB-03`, `GIS-04`), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ADR 0062 (Tarama'nın çizgilerden yüzleri), ADR 0065 ve 0069 (İçine tıklayarak alan, Alana çevir), ADR 0142 (köşe kotu ve taşınması), ADR 0145 (yazının yeri ve hizası), ADR 0148 (Topolojik temizlik), ADR 0149 (eğrilerin 0,1 mm'lik temsili).

## Bağlam

DXF'ten, NCZ'den ya da kâğıttan gelen parsel ve yapı çizimleri çoğu kez çizgidir. Her parsel kendi kapalı alanı değildir: komşularıyla paylaşılan çizgilerle çevrili bir bölgedir. Parsel numarası da bölgenin içinde duran bir yazıdır. CBS'ye geçiş, alan hesabı, tarama ve tematik gösterim için her bölgenin numarasıyla bir alan olması gerekir.

KentOS'ta bugün bölgeler tek tek alan olur: İçine tıklayarak alan (bir tık, bir bölge) ya da Alana çevir (kapalı bir çoklu çizgi). Yüzlerce parsellik bir pafta için bu, yüzlerce tık ve numaraların elle yazılması demektir.

Netcad bunu Topoloji › Otomatik Alan Kapat ve Obje Aktar ile yapar. ArcGIS Pro Feature To Polygon (etiket nesneleriyle) ve Construct Polygons ile, QGIS Polygonize ile yapar.

## Karar

### 1. Araç

**Toplu alan** (`tool.polygonize`): seçili ya da görünen çizgilerin kapattığı bütün bölgeleri tek işlemde alan yapar. Bölgenin içindeki yazının ya da adlı noktanın değeri alanın özniteliği olur. Önce gösterir, Enter tek adımda yazar.

- **Şerit:** Alan › Oluştur ve çevir, İçine tıklayarak alan'ın yanında.
- **Takma adlar:** TOPLUALAN, ALANKAPAT (Netcad'in Otomatik Alan Kapat'ı), POLYGONIZE (QGIS).

### 2. Girdi

- **Seçim varsa:** seçimdeki çizgi işleri bölgeleri kapatır, seçimdeki yazılar ve noktalar etikettir.
- **Seçim yoksa:** görünen katmanlardaki bütün çizgi işleri, yazılar ve noktalar.
- **Çizgi işi:** İçine tıklayarak alan'ın kuralıyla (`isBoundaryKind`): nokta, yazı, ölçü ve tarama dışındaki her nesne. Bunlar çizgi, çoklu çizgi, yay, daire, elips, eğri ve var olan alanların sınırlarıdır. Elips ve eğri 0,1 mm'lik kirişleriyle girer (ADR 0149).
- **Etiket:**
  - yazı: değeri metni (baştaki ve sondaki boşluklar atılır), yeri kutusunun ortası (projenin çizim yazı tipiyle, hizasıyla; ADR 0145);
  - nokta: değeri etiketi, yeri kendisi. Etiketi olmayan nokta etiket değildir.
- Gizli katman hiç katılmaz. Kilitli katmandaki çizgiler okunur, değişmez.
- Alanlar **etkin katmana** yazılır (çizim araçları gibi). Etkin katman kilitliyse komut reddeder.

### 3. Bölgeler

- Bölgeler çizgi işinin sınırlı yüzleridir. Ortak çekirdeğin yüz dizinini (`FaceIndex`) Tarama ve İçine tıklayarak alan da kullanır.
- Yüzün sınırında:
  - sarkan çizgiler (bir yere bağlanmayan uçlar, iki bölgeyi bağlayan köprüler) yoktur;
  - girişin köşeleri durur (komşunun T kavşağı da), yalnız iki çizginin kesişme noktası düz geçilen yerde düşer;
  - 1 µm'den ince şerit bölge sayılmaz.
- **Adalar (A):** açıkken bölgenin içindeki kapalı grup alanın deliği olur, kapalıyken alan grubu da kapsar. Grubun kendi bölgeleri her iki durumda da ayrıca alan olur. İlk değer açık (İçine tıklayarak alan gibi); oturum boyunca ondan ayrı hatırlanır.
- **Var olan alan:** girişteki bir alanın halkalarıyla aynı olan bölge yeniden yazılmaz, sayılır. Aynılık, halkaların dönülen köşelerinin 1 µm içinde ve aynı sırayla (yönü ve başlangıcı ne olursa olsun) örtüşmesidir; yaylı kenarlarda kabarıklık da eşittir.
- **Kot:** alanın köşeleri kotlarını çizgi işinden ADR 0142'nin taşıma kuralıyla alır: köşe, kotlu bir çizgi köşesindeyse onun kotunu, kotlu bir kenarın üstündeyse kenar boyunca kotu alır; yoksa kotsuzdur.

### 4. Etiket ataması

- Etiket, dış halkası onu içine alan **en küçük** bölgeye atanır. Adalar kapalıyken iç içe bölgelerde içteki alır; açıkken delikteki etiket deliği dolduran bölgenindir.
- Bir bölgenin ya da grubun sınırına 1 µm'den yakın etiket hiçbir bölgeye atanmaz, **sınırda** sayılır.
- **Tek etiketli bölge:** değeri alanın özniteliğine yazılır.
- **Etiketsiz bölge:** alan yazılır, öznitelik boş kalır; sayılır.
- **Çok etiketli bölge:** alan yazılır, öznitelik boş kalır; sayılır, değerleri iletide. Tahmin yoktur (CLAUDE.md §23.3).
- **Öznitelik adı (Ö):** ilk değer “Ad”. Kullanıcı yazar, oturum boyunca hatırlanır. Boş ad kabul edilmez.

### 5. Rapor

- **Sayılar:** yazılacak alan; bunların etiketsiz ve çok etiketli olanları; sınırdaki etiket; var olan alan; boşta kalan uç.
- **Boşta kalan uç:** açık bir yolun (çizgi, çoklu çizgi, yay, açık eğri ve elips yayı) başka hiçbir nesneye ve kendi öbür köşelerine değmeyen ucu (1 µm; Topolojik temizliğin tanımı, ADR 0148 §5). Kapanmamış bölgenin işaretidir; ileti Topolojik temizliği önerir.
- **Ayrıntı:** çok etiketli bölgelerin değerleri ve sınırdaki etiketler Uyarılar'a satır satır yazılır (ilk 20'si, kalanın sayısı).

### 6. Hesap (ortak çekirdek)

`kentos_geometry_core::ops::polygonize` (WASM'da `polygonize`):

- **Girdi:** çizgi işi (nesneler, çizim sırasıyla), etiketler (yer ve değer), adalar.
- **Çıktı:**
  - bölgeler, küçükten büyüğe: alan (dış halka, delikler), atanan etiketlerin sırası, varsa aynısı olan girişteki alanın sırası;
  - sınırdaki etiketler;
  - boşta kalan uçlar.
- **Bağımsız başvuru:** `scripts/fixtures/polygonize_cases.py`. Düz kenarlı ağlarda yüzleri kesin kesirlerle, yarım kenarlarla kendisi bulur. Sarkanları, köprüleri, düz geçilen kesişmeleri ve ince şeritleri §3'ün kuralıyla temizler; etiket, sınır, ada, var olan alan ve boşta uç kurallarını uygular. `fixtures/polygonize/v1/cases.json`'a yazar; çekirdek ve web WASM'ı aynı sonucu verir. Yaylı, daireli, elipsli ve eğrili ağlar çekirdeğin kendi testlerindedir.

### 7. Komut

`cad.entities.create`'in yeni `polygonize` işlemi (adım adı “Toplu alan”): alanlar etkin katmana, kotları ve öznitelikleriyle, tek adımda. İki işleyici, ortak durum, katalog ve Python SDK'sı.

### 8. Araç ve arayüz

- **Akış:** araç açılınca girdiyi alır (§2) ve önizlemeyi hemen gösterir.
  - A adaları açıp kapatır; Ö öznitelik adını sorar.
  - Enter, Uygula ya da hızlı sağ tık yazar ve araçtan çıkar; Esc çıkar.
  - Yazılacak alan yoksa söyler, araçta kalır.
- **Önizleme:**
  - tek etiketli bölgeler vurgu renginde ince dolgu ve çizgi; etiketsizler vurgu renginde kesikli çizgi; çok etiketliler tehlike renginde ince dolgu ve çizgi;
  - var olan alanlar çizilmez;
  - boşta kalan uçlarda ve sınırdaki etiketlerde ekran boyutu sabit tehlike renginde işaret;
  - imlecin yanında sayılar.
- **İstem:** “24 alan; 2 etiketsiz, 1 çok etiketli; 3 uç boşta [Adalar (A): açık / Öznitelik (Ö): Ad / Uygula (Enter)]”.
- **İletiler:** açılışta bulgu, kapsamla; her değişiklikte yeni bulgu; yazınca sonuç.

### 9. Kapsam dışı

- Yazıyı bölerek birden çok özniteliğe yazma (“101/7” → Ada, Parsel): sonraki iş.
- Bölge içindeki öbür nesnelerin özniteliklerini taşıma (Obje Aktar'ın bütünü), alanlar arası topoloji ilişkisi (`NUM-10`).
- Kapanmayan çizgileri kendiliğinden kapatma: Topolojik temizliğin işi (ADR 0148).

### 10. İş sırası

1. **Çekirdek:** `ops::polygonize`, WASM bağlayıcısı, bağımsız başvuru ve ortak durumlar, başarım ölçümü.

   *(1 Ekim: tamam.)*
   - **Çekirdek:** `ops::polygonize` (`polygonize`, WASM'da aynı adla). Bölgeler yüz dizininin (`FaceIndex`) yeni `faces(islands)`'ından, en küçükten; etiket `smallest_at` ve `near_ring` ile; var olan alan, halkaların dönülen köşeleri (`turning`) karşılaştırılarak, en alttaki köşeye göre sıralanmış parçalar arasında; boşta uçlar paketli R-ağacıyla.
   - **Başvuru:** `polygonize_cases.py` 58 durum yazar: 17 elle kurulmuş (her biri başlangıçta ve TM koordinatlarında: dört parsellik ada, T kavşağı, taşan çizgiler, sarkan çizgi, köprü, ada açık ve kapalı, sınırdaki, dışarıdaki, iki ve sıfır etiket, var olan alan (ters yönde, fazladan köşeyle, delikli), açık kalan çerçeve, kendi üstünde biten çoklu çizgi, köşe olan kesişmeler, iç içe üç bölge) ve 24 rastgele sokak ağı (eksik çizgi, sarkan çizgiler, yapılar, var olan alanlar, rastgele etiketler).
   - **Sonuç:** çekirdek (`tests/polygonize.rs`) ve web WASM'ı (`model/ops/polygonize.test.ts`) hepsinde başvuruyla aynı (noktalar 1e-8 m içinde); değiştirilmiş beklentileri yakalar.
   - **Başarım:** TM koordinatlarında 202 uzun çizgilik 100 × 100 parsellik ağ, dörtte biri var olan alan, 10 000 etiketle 0,24 saniyede (release, `tests/polygonize.rs`'in elle çalıştırılan testi).
2. **Komut:** `CreateOperation::Polygonize`; iki işleyici, ortak durum, katalog ve Python SDK'sı. *(1 Ekim: tamam. Sözleşmede `polygonize` işlemi; adım adı iki işleyicide “Toplu alan”. Ortak durum (`create_command_cases.py`): iki bölge tek adımda, birincisi `Ad` özniteliği ve köşe kotlarıyla, ikincisi deliğiyle; geri alma ve yineleme adıyla; masaüstü, web ve Python SDK'sı geçer. TypeScript tipi, katalog ve SDK'nın tipleri üretildi.)*
3. **Araç ve arayüz:** iki platformda araç, şerit, takma adlar, önizleme, ortak iz (`fixtures/interaction/v1/polygonize.json`), testler ve resimler.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Çizgi olarak gelen pafta tek araçla, numaralarıyla alan olur; kapanmayan ve belirsiz bölgeler sayıyla söylenir, tahminle doldurulmaz.
- Var olan alanlar yinelenmez; araç güvenle yeniden çalıştırılabilir.
- `HYB-01` ile birlikte: önce Topolojik temizlik, sonra Toplu alan.

## Doğrulama

- **Hesap:** bağımsız Python başvurusu kesin kesirlerle; elle kurulmuş durumlar (parsel bloğu, T kavşağı, sarkan çizgi, köprü, ada açık ve kapalı, iç içe bölge, sınırdaki etiket, çok etiketli ve etiketsiz bölge, var olan alan, boşta kalan uç) ve TM koordinatlarında rastgele ağlar; çekirdek ve web WASM'ı aynı sonucu verir.
- **Komut:** ortak durum üç koşucuda.
- **Araç:** ortak iz iki platformda; resimler iki temada, 1440×900 ve 1100×650.
