# ADR 9101: Veri katmanı — katman türleri ve kaynaktan canlı çizim

- **Durum:** kabul edildi (2026-11-20). Plan: [plan/veri.md](../../plan/veri.md) VY-02, VY-03.
- **İstek:** sahibin sözü (20 Kasım 2026): “postgresql bağlanınca bir katmanı ekliyorum ama bütün veriyi önbelleğe
  alıyor Bu da çok ciddi yavaşlamaya neden oluyor … Gerçek zamanlı olarak bunu veritabanından çekmesi lazım”; “edit
  modu veri kaydetme yapısı aktif katman katman tipleri gibi yapılar GIS için uygun değil … şu anki yapı uygun değil”.
- **Önceki kararla ilişkisi:** ADR 0503, 0505 ve 0757'nin bağlı katmanı (tablonun bütün satırları çizimin nesnelerine
  kopyalanır) Kaynaklar'dan ve PostGIS'e bağlan'dan eklenen tablolar için kalkar: tablo artık veri katmanı olarak
  eklenir. SQL sorgusundan yapılan katman (ADR 0582) bir dönem bağlı kopya olarak kaldı; VY-16'dan beri o da veri
  katmanıdır ([ADR 9113](9113-query-layers.md)). Satırları
  [ADR 9100](9100-data-drivers.md)'ün sürücüleri okur.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** belge şeması değişmez; CBS modül verisi sürüm 2 (§3).
  **Komut:** `cad.layers.data` v1 (§4).
- **Bağımlılık:** yeni dış kitaplık yok. Masaüstü `kentos-data`, `kentos-data-gpkg` ve `kentos-data-postgis`'i kullanır.

## §1 Bağlam ve araştırma

**Bağlam.** Bir tablo bağlanınca bütün satırları okunuyor ve çizime yazılıyordu. Bir milyon parsellik tablo belleği
dolduruyor, açılışı dakikalara çıkarıyordu. Katman ağacında yalnız iki tür vardı: nesneleri çizimde olan katman ve
servisten çizilen katman. “Satırları bir kaynakta duran katman” diye bir tür yoktu.

**Masaüstü CBS** (QGIS 4.2.3, bu makinede kurulu; ekranlar ayrı profille çekildi, `.run/gui-evidence/veri/qgis/`):

- **Katman türleri ayrıdır** (`13-katman-ekle-turleri.png`): vektör, raster, ağ, nokta bulutu, vektör karo, not ve
  grup. Vektör katman bir veri sağlayıcısına bağlıdır. Proje dosyası yalnız kaynağın adresini, tabloyu, anahtar ve
  geometri sütununu, türü, sistemi ve kutuyu (`<extent>`) yazar; satır yazmaz.
- **Kutuya güven:** Proje özellikleri › Veri kaynakları'nda “veri kaynağının üst verisi yoksa projeye güven” seçeneği
  vardır (`09-proje-ozellikleri-veri-kaynaklari.png`). İşaretliyse açılışta kutu kaynaktan sorulmaz, projeden okunur.
- **Çizim görünür alanı ister.** Her katman ayrı bir iş parçacığında çizilir; harita 250 ms'de bir ara görüntüyle
  güncellenir (Seçenekler › Görüntüleme, `17-ayarlar-cizim-sadelestirme.png`).
- **Kaydırma:** sürüklerken eski görüntü kaydırılır, yeni çizim bırakınca başlar.
- **Sadeleştirme** varsayılan olarak 1 piksel eşikle açıktır; “mümkünse sağlayıcıda sadeleştir” seçeneği vardır.
- **Katman ağacında** veri katmanı türünün simgesiyle görünür; ipucu kaynağı söyler (`01-katman-agaci-siniflar.png`).

**Türkiye mevzuatı:**

- 6698 sayılı Kişisel Verilerin Korunması Kanunu (RG 7.4.2016/29677) md. 12(1): veri sorumlusu, kişisel verinin hukuka
  aykırı işlenmesini ve erişilmesini önlemek için uygun güvenlik düzeyini sağlayan teknik ve idari tedbirleri alır.
  Parsel ve abone tablolarında malik adı, T.C. kimlik numarası ve adres gibi kişisel veriler bulunur.
  - Satırlar çizim dosyasına kopyalandığında, dosya e-postayla ya da taşınır bellekle dolaştıkça veri de veritabanının
    erişim denetiminin dışına çıkıyordu.
  - Veri katmanında satırlar kaynağında kalır. Proje yalnız adresi taşır; parolayı taşımaz.
- 49 sayılı Cumhurbaşkanlığı Kararnamesi, Coğrafi Veri İzinleri Yönetmeliği ve TUCBS Birlikte Çalışabilirlik Rehberi
  (ayrıntısı ADR 9100 §2'de) verinin paylaşımını ve standardını düzenler. Bir masaüstü programının katman türleri,
  çizerken neyi bellekte tuttuğu ya da görünümü nasıl okuduğu hakkında hükümleri yoktur. Bu ADR'nin kararları
  KentOS'undur.

## §2 Karar: katman türleri ve veri katmanı sözleşmesi

Katman ağacındaki bir katmanın türü, nesnelerinin nerede olduğudur:

| Tür | Nesneleri | Tanıyan alan |
|---|---|---|
| Çizim katmanı | çizimin içinde | `data`, `service`, `raster`, `cloud` yok |
| **Veri katmanı** (nokta, çizgi, alan, tablo) | bir GeoPackage dosyasının ya da veritabanının tablosunda | `data` |
| Servis katmanı | bir harita servisinde | `service` |
| Raster, nokta bulutu | bir dosyada | `raster`, `cloud` |
| Grup | — | `kind: group` |

`LayerNode.data: Option<DataLayer>` (`crates/shared/contracts/src/gis/data_layer.rs`):

- `driver` (`gpkg`, `postgis`), `address`, `relative` (yalnız dosyada; `/` ile), `table`, `key`;
- `geometry`: `{column, typeName, srid, z, m}` (geometrisiz tablo: yok);
- `extent`: tablonun kataloğunun son söylediği kutu, tablonun sisteminde;
- `readOnly`: kaynağın yazma yetkisi yoksa.

Sözleşmenin kuralları (`DataLayer::problem`):

- PostGIS adresi `postgresql://` ile başlar ve parola taşımaz;
- `table` ve `key` boş olamaz;
- geometri türü OGC'nin büyük harfli adlarından biridir, sistem 0 olamaz, geometri sütunu anahtar olamaz;
- kutu dört sonlu sayıdır, en küçüğü en büyüğünü geçmez ve yalnız geometrisi olan tabloda bulunur.

Katmanın türü (`geometry`: nokta, çizgi, alan) tablonun geometri türünden gelir; geometrisiz tablo “Tablo”dur.
Tablonun sütunları katmanın alanları olur. İki kez geçen bir ad (Türkçe katlanarak) ya da alan kurallarına uymayan
bir ad alınmaz ve söylenir.

**Veri katmanı nesne tutmaz.** Belge onun için hiçbir nesne saklamaz; görünüm, tablo ve araçlar satırları kaynaktan
okur (§6).

## §3 Karar: `.kcad` — CBS modül verisi sürüm 2

Veri katmanı katman düğümünün `data`'sıdır. Yazıcı veri katmanı varken kökün `modules`'üne `gis: 2` yazar (ADR 8054
§10'un modül sürümleri). Belge şeması değişmez; başka her çizim eskisiyle bayt bayt aynıdır.

- **Eski okuyucu** `gis: 2`'yi daha yeni bir sürümle yazılmış dosya diye reddeder. Veri katmanını boş bir katman sanıp
  bir sonraki kayıtta düşürmez.
- **Satır ve parola dosyaya yazılmaz.**
- **Bağımsız okuyucular** aynı kuralları denetler: `tools/kcad/kcad.py` ve `scripts/fixtures/kcad_v2_reference.py`.
- **Örnekler:** geçerli örnek `fixtures/kcad/v2/data-layers.kcad`; yedi bozuk örnek
  `fixtures/kcad/v2/broken/data-layer-*.kcad` (modülsüz yük, grubun tablosu, adreste parola, yazılmış `false`, servisi
  de olan katman, tanınmayan tür, ters kutu).
- Ayrıntı: `docs/specs/kcad-v2.md` “CBS verisi sürüm 2”.

## §4 Karar: `cad.layers.data` v1

Veri katmanını ekler (`add`), değiştirir (`update`: adı, tablonun yeri, alanları) ya da kaldırır (`remove`: tablo
kaynakta olduğu gibi kalır). Her biri işlemin adını taşıyan tek bir geri alma adımıdır: “Veri katmanı ekle”, “Veri
katmanını değiştir”, “Veri katmanını kaldır”.

**Komut kaynağı okumaz.** Ev sahibi (masaüstü, Python, MCP) tablonun kataloğunu okuyup `data` ve `fields` olarak verir.
Komut böylece sunucusuz da sınanır ve aynı girdiyle her yerde aynı sonucu verir.

Ret kodları, bu sırayla:

1. `no_layer`;
2. `empty_name`;
3. `no_data`;
4. `invalid_data` (`DataLayer::problem`);
5. `invalid_fields`;
6. `invalid_revision`, `revision_conflict`;
7. `layer_not_found`;
8. `not_a_layer` (grup);
9. `not_a_data_layer`.

Ortak durumlar: `scripts/fixtures/layer_data_command_cases.py` bağımsız Python betiği
`fixtures/commands/v1/cad.layers.data.json`'a 33 durum yazar. Rust uygulaması hepsini geçer. Python SDK
`kentos.cad.layers.data`, MCP ise katalogdan çağırır.

## §5 Karar: Katmanlar'da ve gezinmede

- **Katmanlar paneli:** veri katmanı satırında türünün simgesi katmanın renginde çizilir. Yanında kaynağın imi durur:
  dosya ya da veritabanı. İpucu türünü (“Veri katmanı (alan)”), yerini (“kent@db:5432/kbs › public.parseller”) ve
  varsa “Salt okunur”u söyler.
- **Katmana yaklaş, Hepsini göster ve projenin açılışı** veri katmanını `extent`'inden çerçeveler. Tablonun sistemi
  başkaysa kutu projenin sistemine çevrilir. Bunlar için kaynağa sorulmaz; bir milyon satırlık tablo için bile
  yaklaşmak anında olur.

## §6 Karar: canlı çizim (`apps/desktop/src/gis/data_view.rs`)

**Her karede** çizim alanı görünen kutuyu, ölçeği (metre başına piksel), görünüşü ve stil kitaplığını
`DataView::want`'a verir. Gösterilen her veri katmanı için:

- **Ne zaman okunur:** son okuma görünümü kapsıyorsa ve anahtarı değişmediyse hiçbir şey okunmaz. Anahtar tablo,
  katmanın stili, görünüş, kitaplığın sürümü ve ölçeğin çeyrek oktav basamağıdır. Kapsamamışsa yeni okuma başlar:
  - stil, tablo ya da renkler değiştiyse hemen;
  - görünüm yalnız kaydıysa görünüm durunca.

  Önceki okuma iptal anahtarıyla durdurulur.
- **Ne kadarı okunur:** görünüm ve her yandan yarısı kadar pay. Pay içinde kalan kaydırma hiçbir şey okumaz.
- **Hangi sütunlar okunur:** yalnız stilin (sınıflandırıcı, etiketler, süzgeç) adını andığı alanlar.
- **Sadeleştirme:** geometri yarım piksele sadeleştirilir; PostGIS'te sunucuda yapılır (ADR 9100 §6). Coğrafi
  sistemde (EPSG:4326, 4258, 4269) eşik dereceye çevrilir.
- **Sistem:** tablonun sistemi projeninkinden başkaysa kutu tablonun sistemine çevrilip istenir. Gelen geometri nokta
  nokta projenin sistemine çevrilir (`TableSystem`).
- **Parçalar:** satırlar 4096'lık parçalarla gelir. Her 8192 satır geçici nesnelere çevrilir ve kendi deposunda çizim
  katmanlarıyla aynı stil motorundan (`kentos_plot::layers::build_whole`) geçer. Kategorili, dereceli ve kurallı
  stiller bu yüzden aynen çalışır.
  - Her parça hazır olunca pencereye haber verilir (`Message::DataReady`), parça çizilir.
  - Sahne, veri katmanlarının parçalarını katman ağacının sırasına koyar.
- **Titreme yok:** yeni okuma bitene kadar eski parçalar çizili kalır. Yalnız ilk okuma parça parça görünür.
- **Hata:** okunamayan katman bir kez söylenir; aynı hata tekrarlanınca yeniden söylenmez.

Okuma kendi iş parçacığında yürür, arayüz donmaz. Belgeye hiçbir şey yazılmaz: veri katmanlı çizimde nesne sayısı 0'dır.

**Etiketler** (`gis/data_labels.rs`). Etiketi açık bir katmanın okuması bütünüyle bittiğinde nesneleri kendi küçük
belgesine konur: katmanın kendisi (veri bağı olmadan), projenin ayarları ve stilleri, okunan satırlar.

- Belge okuma iş parçacığında kurulur.
- Etiket deposu (`Spatial`) arayüz iş parçacığında kurulur: etiket motorunun belleği o iş parçacığına bağlıdır.
- Etiket motoru bu nesneleri çizimin nesneleri gibi yerleştirir: aynı sınıflar, şablonlar, boyutlar ve haleler.
- Veri katmanının etiketleri çizimin etiketlerinin altında durur.
- Okunan satır **20 000**'i geçerse etiket kurulmaz. O genişlikte sözcükler okunmaz, motor zaten neredeyse hepsini
  dışarıda bırakırdı. Bir milyon parselde bütün kent ve ilçe ölçeğinde etiket yoktur, mahalle ölçeğinde vardır.

**Bilgi** (`gis/data_info.rs`). Bilgi önce çizimin nesnesine bakar. Tıklanan yerde nesne yoksa araç ev sahibinden
veri katmanlarına sormasını ister (`ViewChange::DataInfo`).

- Görünen veri katmanları üstten alta sorulur. Tıklanan yerin seçim payı kadar çevresi tablonun sistemine çevrilir ve
  `exact` istekle okunur: en çok 64 satır, bütün sütunlar.
- Satır çizimin nesneleri gibi seçilir: önce en yakın kenar, yoksa noktayı içeren en küçük alan.
- Kart katmanın kart şablonundan geçer. Şablon yoksa başlık katmanın adıdır ve sütunlar tablodaki sırasıyla
  dizilir. Başlığın altında satırın yeri yazar: tablo ve anahtar (“… › public.parseller · fid 492776”).
- Katmanın eylemleri karttadır; ek yoktur, çünkü satır çizimin nesnesi değildir.
- Bilgi bulunan satırı halkayla işaretler (`Tool::data_found`). Esc önce kartı kapatır.
- Sorgu arayüzün dışında yürür. Yanıt gelmeden yapılan yeni tıklama eskisinin yanıtını bayatlatır.

## §7 Ekleme yolları

- **Kaynaklar:** bir GeoPackage'ın ya da bağlantının tablosunda “Veri katmanı olarak ekle” (eskiden “Bağlı katman
  olarak aç”).
- **PostGIS'e bağlan:** seçilen tablolar veri katmanı olarak eklenir.

Her iki yolda tablonun kataloğu arayüzün dışında okunur ve her tablo bir geri alma adımıyla katman ağacının en üstüne
eklenir. Sonra ilk tablonun kutusuna yaklaşılır. Alan olarak alınmayan sütunlar uyarı olarak söylenir.

## §8 Planda olmayan kararlarım

1. **Kutu projede saklanır** (`extent`). Yaklaşmak ve projeyi açmak kaynağa gitmez. Kutu tablo eklenirken ve
   değiştirilirken okunur.
2. **SQL sorgusundan yapılan katman bağlı kopya olarak kaldı.** Sürücü sözleşmesi tablo okuyor; sorgunun kendisini
   okumak ayrı bir iş (§10). Kapandı: VY-16, [ADR 9113](9113-query-layers.md).
3. **Okumanın sınırları:** görünümün yarısı kadar pay, çeyrek oktav basamak, yarım piksel sadeleştirme.
4. **Yeni okuma görünüm durunca başlar.** Eski parçalar yeni okuma bitene kadar durur.
5. **Yalnız stilin andığı sütunlar okunur.** Alan JSON'da adıyla geçiyorsa okunur; birkaç fazla sütun, değeri olmadan
   çizilen bir sınıftan ucuzdur.
6. **Yeni veri katmanı ağacın en üstüne eklenir**, çizim katmanlarının üstünde çizilsin diye.
7. **Etiket 20 000 satırla sınırlı.** Daha geniş bir görünümde etiket kurulmaz.
8. **Bilgi önce çizimin nesnesine bakar**, nesne yoksa veri katmanlarına üstten alta sorar.
9. **Satırın kartında** başlık, şablon yoksa katmanın adıdır. Sütunlar tablodaki sırasıyla dizilir; altında tablonun
   yeri ve anahtarı yazar.

## §9 Kanıt

Gerçek pencerede doğrulandı (Xvfb, sahibin ayarlarının kopyası; `scripts/usage/real/session.sh`, ekranlar
`.run/gui-evidence/veri/vy03/`). Projede iki kaynak vardı:

- PostGIS 3.6.3'te 1 000 000 parsel (kategorili stil);
- bir GeoPackage'ta yollar ve direkler.

Bütün kent, ilçe, mahalle ve sokak ölçeğinde ve bir kaydırmada katmanlar kaynaktan çizildi. Çizimde 0 nesne vardı.

- **Parça parça çizim:** açılıştan 1,2 saniye sonra parsellerin bir bölümü çizilmiş, üst şerit hâlâ okunuyordu
  (`00a-ilk-parcalar.png`).
- **Etiketler:** mahalle ölçeğinde (1:1 652) her parselin üstünde `{ada}/{parsel}` etiketi çıktı
  (`.run/gui-evidence/veri/vy03b/06`, `07`).
- **Bilgi:** bir parsele tıklandı. Satır PostGIS'ten bütün sütunlarıyla okundu, kart açıldı ve parsel halkayla
  işaretlendi (`08`). Esc kartı ve halkayı kapattı (`09`).

## §10 Kapsam dışı ve kalan

- **Seçim** tabloyla birlikte VY-04'te, **kenet** düzenleme oturumuyla birlikte VY-05'te.
- **Python ve MCP'de veri katmanının satırları:** Bilgi'nin bu yüzlerdeki karşılığı, VY-04'te tablonun satırlarını
  okuyan komuttur.
- **Katmanın süzgeci** (ADR 0211) veri katmanında çizimde uygulanır; gönderilebilir parçası görünümün isteğiyle kaynağa
  gider (VY-12, [ADR 9109](9109-expression-to-the-source.md) §3).
- **Stil penceresinin sınıflandırması** veri katmanında değerleri kaynaktan okur (VY-13,
  [ADR 9110](9110-classes-from-the-source.md) §1).
- **Göreli yol:** `relative` sözleşmede ve dosyada var. Masaüstünün bunu yazması ve dosya taşınınca çözmesi VY-09'da.
- **SQL sorgusundan yapılan katman** kaynağında canlı okunur ve salt okunurdur (VY-16,
  [ADR 9113](9113-query-layers.md)).
- **Küçük ölçekte ince çizgiler:** kâğıt kalınlıkları yüzünden çizgiler soluk görünüyor; ölçeğe bağlı kalınlık stil
  işi.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/gis/data_layer.rs` (`DataLayer`, `DataGeometry`, `kind_words`,
  `place_words`); `LayerNode.data`; kodlar `no_data`, `invalid_data`, `not_a_data_layer`.
- **Komut:** `crates/shared/commands/src/gis/cad_layers_data.rs`, `crates/native/application/src/gis/layers_data.rs`;
  katalog, yetenek, dağıtım ve geçmiş başlığı.
- **Alan ve geçmiş:** `NewLayer.data`, `ServedBy.data`, `set_layer_service`'in denetimi.
- **`.kcad`:** `crates/shared/kcad` (`GIS_WITH_DATA_LAYERS`, `modules_of`, katmanın `data`'sı); `tools/kcad/kcad.py`,
  `scripts/fixtures/kcad_v2_reference.py`, `docs/specs/kcad-v2.md`.
- **Masaüstü:**
  - `gis/data_layers.rs`: sürücülerin kaydı, cihazın parola bulucusu, tablonun tanımı, ekleme;
  - `gis/data_view.rs`: canlı çizim;
  - sahne (`cad/style/scene.rs`), çizim alanı (`shell/viewport/state.rs`), Katmanlar satırı (`shell/view/area.rs`);
  - yaklaşma: `cad/layering.rs`, `cad/selecting.rs`, `shell/app/settings.rs`;
  - ekleme yolları: Kaynaklar, PostGIS'e bağlan.
- **Etiket ve Bilgi:** `gis/data_labels.rs`, `gis/data_info.rs`, `labels::data_layer`, kartın `Shown`'u
  (`cad/info_card.rs`). Etkileşim crate'inde `ViewChange::DataInfo`, `Tool::data_found` ve `Session::data_found`;
  Bilgi'nin halkası.
- **Uygulama crate'i:** `geopackage::object_of`, `sources::TableSystem`.

## Doğrulama

- `cargo test -p kentos-contracts`: sözleşmenin kuralları.
- `cargo test -p kentos-kcad`: geçerli ve bozuk örnekler; bağımsız okuyucu aynı sonuçları verir.
- `KENTOS_CASES_ONLY=cad.layers.data cargo test -p kentos-native-application --test all every_command_case_matches`:
  33 durum.
- `cargo test -p kentos-desktop --lib data_`:
  - GeoPackage tablosu satırları kopyalanmadan veri katmanı olarak eklenir;
  - başka sistemdeki tablo projenin sisteminde çizilir;
  - okumanın kapsamı ve basamağı doğru;
  - Bilgi nesne olmayan yere tıklayınca veri katmanlarına sorar; bulunan satırın kartı tablonun sırasıyla, yeriyle ve
    anahtarıyla; eski yanıt bayattır; iki parselin arası boştur;
  - etiketli katmanın okuması belgesini tutar ve etiket motoru ondan `ada`yı okur; etiketsiz katman hiçbir şey tutmaz.
- `cargo test -p kentos-desktop --lib attachments_tests`: çizimin nesnelerinin kartları değişmedi.
- `KENTOS_TRACES_ONLY=identify … every_trace_passes_in_every_variant`: Bilgi'nin ortak izi her türevde geçti.
- Gerçek pencere: §9.
