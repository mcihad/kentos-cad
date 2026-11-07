# KentOS Harita CAD — modern harita mühendisliği ve üretim planı

İnceleme tarihi: **31 Ekim 2026**. İncelenen kod tabanı:
`5cabec02be37cd9b175f0764e3aafcfc8d89f90e` (`main`).

Bu belge **tam olarak 20 ana geliştirme maddesi** içerir. Kapsam yalnız imar
değildir: güncel ölçüm verisinin yönetilmesi, hassas harita çizimi, sayısal
kadastro, planlama, ortofoto/LiDAR, mühendislik yüzeyleri, saha tasarımı,
altyapı, uygulama kontrolü ve kurumsal teslim birlikte ele alınır.

**Sahibin 31 Ekim kararı:** yeni hedefler modern veri toplama ve sayısal üretim
üzerine kurulacaktır. Eski haritacılık teknikleri, elle yürütülen tarihsel
hesap iş akışları ve eski cihazlara özel yeni araçlar bu yol haritasına
alınmaz. Mevcut dosyaların okunabilmesi ve verinin korunması uyumluluk işidir;
eski yöntemi yeniden geliştirme hedefi değildir. GNSS/RTK/PPK/PPP, LiDAR,
fotogrametri, sensör kalite bilgisi, ilişkili tasarım ve açık veri standartları
esas alınır. Bir algoritmanın yaşından çok bugün gerekli olup olmadığı ve
doğrulanabilirliği önemlidir; sağlam geometri ve belirsizlik hesabı korunur.

Bu teslim araştırma ve plandır. Ürün kodu, `CLAUDE.md`, diğer planların
tamamlanma işaretleri ve başka ajanın çalışma alanındaki değişiklikleri
değiştirilmemiştir. Aşağıdaki maddelerin tamamı **açıktır**; mevcut temellerin
listelenmesi bu yeni kabul kapsamlarının tamamlandığı anlamına gelmez.

## İnceleme yöntemi ve bulguların sınırı

- Aşağıdaki kaynakçada **35 ayrı birincil web kaynağının içeriği** incelendi:
  üretici belgeleri, açık standartlar ve açık kaynak depoları. Arama sonuçları,
  açılamayan sayfalar ve aynı sayfanın tekrar açılması bu sayıya katılmadı.
- Netcad için GIS, NetSurf, Netmap, Planet, Netpro, Netkamu ve Nettop birlikte
  değerlendirildi. Dünyadaki karşılıklar için Civil 3D, Bentley OpenCities Map
  ve OpenSite, Trimble Business Center, 12d, ArcGIS Pro/Urban, QGIS ve GRASS
  incelendi. Ticari ürünler burada kurularak veya lisanslı uygulamalarında
  denenerek karşılaştırılmış sayılmaz.
- Kod incelemesinde sözleşme, hesap çekirdeği, komut işleyicisi, kalıcılık,
  masaüstü yüzeyi ve otomasyon yolları ayrıldı. Önemli bulgular aşağıda
  **K01–K45** ile dosya/işlev ve sabit commit bağlantısına bağlandı. Bir
  menünün veya eski planın işaretli olması tek başına uygulama kanıtı sayılmadı.
- QGIS'in gelişmiş çizim arayüzündeki gerçek kısıt/kenetleme çağrısı ve GRASS'ın
  temizleme araçlarını hangi sırada yürüttüğü de kaynak kodundan okundu.
  Böylece piyasa incelemesi yalnız ürün özellik listelerine dayanmadı.
- Bu oturumda derleme, hesap testi, saha testi veya güncel GUI kabulü
  çalıştırılmadı. Kodun bulunması kalite kabulü değildir. Her maddenin kendi
  kabul senaryosu ve belgenin sonundaki ortak teslim koşulları uygulanacaktır.
- Kapalı kaynak ürünlerin veri yapısı, GPU çekirdeği veya sayısal yöntemi
  açıklanmadığında bunlar hakkında çıkarım yapılmadı. Aşağıdaki mimari
  tercihler **KentOS için öneridir**; rakiplerin iç uygulamasının tarifi değildir.

## Korunacak kararlar ve diğer planlarla sınır

- KentOS Harita, KentOS GIS ve KentOS Mimari ayrımı korunur. CAD projesi ile
  CBS projesi görünüm düğmesiyle birbirine dönüştürülmez. Ortak hesap ve veri
  yetenekleri paylaşılır; veri devri açık dönüşüm/türetme işlemiyle yapılır.
- [ADR 0626](../docs/adr/0626-units-scale-and-true-size-texts.md) ve mevcut
  modül davranışı korunur: yeni Harita projesi metre ve gerçek koordinatla
  başlar. Model birimi, giriş birimi, yer/projeksiyon oranı, çizim/pafta
  ölçeği ve kamera yakınlaştırması birbirine karıştırılmaz.
- [ADR 0228](../docs/adr/0228-web-pause-desktop-and-cloud-first.md) gereği
  teslim masaüstü, ortak Rust, gereken bulut yolları, Python ve MCP içindir.
  Paylaşılan sözleşmeler gelecekte web/mobilce kullanılabilir; bu belge web
  geliştirmesini yeniden başlatmaz.
- [TODOS.md](../TODOS.md) içindeki `HYB-24` ertelenmiştir: gerçek GNSS alıcısı
  olmadan canlı cihaz entegrasyonu yapılmış veya doğrulanmış sayılmaz.
  Dosya/replay, veri sözleşmesi ve çevrimdışı kalite çalışmaları ayrı teslimdir.
- Aynı dosyadaki `GIS-06` ve `GIS-07` sahibiyle ayrı çalışılacak ve yol
  haritasının sonuna bırakılacaktır. Bu rapor resmî ifraz, tevhit, dağıtım,
  kamulaştırma veya kurum kabulünü erkene almaz. Araştırma ve teknik taslak
  üretimi, resmî sonuç üretme yetkisi anlamına gelmez.
- `NCZ-01` dağıtım kapısı korunur. İlgili kaynak için lisans/izin çözülmeden
  NCZ okuyucusunu içeren derleme dağıtılmaz. Yeni format araştırması bu kararı
  kendiliğinden kaldırmaz.
- [cad.md](cad.md) genel çizim/dosya/pafta yeteneklerinin;
  [gis.md](gis.md) ortak CBS verisinin; [road.md](road.md) yol geometrisi,
  koridor ve yol metrajının; [3d.md](3d.md) 3B sahne ve Digital Twin'in
  ayrıntılı planıdır. Burada aynı motorlar için ikinci uygulama oluşturulmaz.
  [form.md](form.md), [python.md](python.md) ve [plugins.md](plugins.md) form,
  Python ve `.kek` eklenti sözleşmelerinin sahipliğini korur.

## Piyasa karşılaştırmasından çıkan ürün yönü

| İncelenen ürün / teknoloji | Belgede gözlenen yaklaşım | KentOS için sonuç |
| --- | --- | --- |
| Netcad GIS, NetSurf, Netpro | CAD/CBS verisi, arazi ve yol üretimi ayrı uzmanlık akışlarıyla ele alınıyor. [S01], [S02], [S05] | Her şeyi şeride eklemek yerine kaynak, tasarım, denetim ve teslim arasında izlenebilir iş akışı kur. |
| Netmap, Planet, Netkamu, Nettop | Parsel, plan, kamulaştırma ve toplulaştırma yalnız çizimden oluşmuyor; ilişkili tablolar ve denetimler var. [S03], [S04], [S06], [S07] | Geometriye renk ve serbest metin eklemek alan modeli kurmakla aynı değildir. Hukuki kurallar ayrı sürümlensin. |
| Civil 3D | Ölçüm veritabanı çizimden ayrılıyor; yüzey sınırlarının türü ve işlem sırası sonuç için önemli. [S08], [S09] | Ham ölçümden türetilmiş noktayı ve yüzeyin kaynak tarifini sakla; sadece son üçgenleri kaydetme. |
| Bentley OpenCities Map / OpenSite | Şema ve kurallı harita verisi ile ilişkili saha tasarımı öne çıkıyor. [S10], [S11] | MicroStation tabanlı ekosistemi yalnız genel CAD komutlarıyla kıyaslama; harita nesnesinin anlamını ve tasarım etkisini incele. |
| Trimble Business Center / 12d | GNSS işleme, ağ kalitesi, bulut kaydı, nesne çıkarımı ve tasarımla gerçekleşenin karşılaştırılması bir üretim zinciri oluşturuyor. [S12]–[S16] | Sadece nokta dosyası açmak modern ölçüm sistemi sayılmaz; kalite, zaman, sensör ve kaynak bağı gereklidir. |
| ArcGIS Pro / Urban | Parsel ağı dengelemesinde analiz ile uygulama ayrılıyor; plan alternatifleri mevcut durumdan ayrılabiliyor. [S17]–[S19] | Önce farkı ve belirsizliği göster, sonra kontrollü uygula. Senaryo ana veriyi sessiz değiştirmesin. |
| QGIS / GRASS gerçek kodu | Çizim kısıtları ortak hizalama çağrısına taşınıyor; temizlik işlemleri açık sıra ve parametreyle uygulanıyor. [S21], [S23] | UI'da ayrı matematik yazma; onarım tarifini, sırasını, etkisini ve geri almasını görünür kıl. |
| PROJ, COPC, STAC, GeoParquet, OGC standartları | Datum/zaman, seçmeli büyük veri erişimi ve makinece okunabilir veri anlamı birbirinden farklı sözleşmeler. [S25], [S26], [S28], [S31]–[S34] | Tek bir dosyayı veya güzel 3B görünümü bütün mühendislik verisinin yerine koyma. |

Ürün sayfalarında belirtilen hızlar, doğruluklar ve mevzuat uyumu KentOS için
kanıt değildir. Açık kaynak incelemek de doğrudan kod kopyalama veya yeni
bağımlılık ekleme kararı değildir; lisans ve bakım değerlendirmesi uygulama
tasarımında yapılır.

## Koddan doğrulanan mevcut temel

Bağlantılar yukarıdaki sabit commit'e gider. Satır, işlevin veya ilgili veri
modelinin başlangıcını gösterir; çalışma alanında başka ajanın değişmekte olan
satırları bu kanıta karıştırılmamıştır. “Sınır”, belirtilen yolun sınırıdır;
bütün depoda benzer hiçbir iş bulunmadığı iddiası değildir.

| Kanıt | Okunan kaynak ve gözlenen davranış / sınır |
| --- | --- |
| K01 | [module_surfaces.rs][K01]: Harita yüzeyi metre, gerçek koordinat, kaynaklar, ölçme ve topoloji sunuyor; GIS'in zaman/senaryo/tematik yüzeyinden farklı. Yeni araçların nereye geleceği bu ayrımdan tasarlanmalı. |
| K02 | [document.rs — CadWork][K02] ve [DocumentSnapshotV2][K03]: Mimari/Harita işi ve proje ayarları kalıcı. Snapshot içinde ayrı bir ölçüm kampanyası veya düzenlenebilir mühendislik yüzeyi koleksiyonu tanımlı değil. |
| K04 | [crs/operation.rs][K04], `operation_with`: geoit ve hız ızgarası adımlarını da içeren işlem açıklaması mevcut. “Düşey dönüşüm hiç yok” diye yeniden iş açılmamalı. |
| K05 | [crs/geoid.rs][K05], `NodeGrid`, `read_gtx`, `transform_point`: geoit N, ENU hız, epok ve kapsam dışı retleri var. Resmî bir ızgaranın temini/kullanım hakkı bundan farklı bir konu. |
| K06 | [crs/ground.rs][K06], `grid_factor`, `point_scale`, `line_factors`, `ground_measures`: projeksiyon ve yer indirgemesi mevcut. Ortak kalite/sonuç kaydına bağlanması ve UX tutarlılığı incelenecek. |
| K07 | [wgpu/lib.rs][K07] ve [styled/gpu.rs][K08]: CPU f64; GPU'da yerel orijin ve yüksek/düşük f32 parçaları mevcut. Hassasiyet çözümü sıfırdan eklenmeyecek; yeni 3B/veri yolları bu değişmezi koruyacak. |
| K09 | [formats.rs — GnssPoint][K09] ve [gnss/nmea.rs][K10]: zaman, fix, uydu, HDOP, geoit ve yükseklikler okunabiliyor; RTK sabit/kayan ayrımı var. Bu sözleşme ham GNSS gözlemi, baz vektörü ve tam kovaryans modeli değil. |
| K11 | [fieldbook/mod.rs][K11]: editör durumu uygulama açıkken tutuluyor. [survey_points.rs][K12], `new_objects`: hesap sonucu genel Point ve Ad/Tür/Z özniteliklerine yazılıyor; bunlar kalıcı ölçüm kampanyasının yerine geçmiyor. |
| K13 | [adjust/horizontal.rs][K13], `KnownPoint`, `NetRow`, `NetworkInput`: mevcut yatay ağ doğrultu/mesafe modeli. [adjust/mod.rs][K14] 1.500 bilinmeyen sınırı koyuyor; [linalg.rs][K15] yoğun normal matris/Cholesky kullanıyor. Artık, doğruluk ve elips temeli zaten var. |
| K16 | [edge_shift.rs][K16]: hedef alanla kenar kaydırma var; yaylı kenar/komşu ve çok parçalı nesne açık ret kapsamı. Genel eğrisel/ilişkili harita tasarımı bu sınırlı işlemle tamamlanmış sayılmaz. |
| K17 | [topology_rules/mod.rs][K17]: 13 kural ve uzamsal dizin var. [incremental.rs][K18], `recheck_watched`: etkilenen kapsamı yeniden denetleme ve durdurma desteği de var. Yeni iş “topoloji ekle” değildir. |
| K19 | [parcels.rs][K19], `base`, `number_of`: parsel genel polygon ve altı öznitelikle üretiliyor; verilen yinelenen numara uyarıyla kabul edilebiliyor. Kalıcı hukuki kayıt, ortak sınır ve hak modeli bu yoldan çıkarılamaz. |
| K20 | [native/style/system.rs][K20]: masaüstü MPYY'yi içeren gömülü stil kitaplığını okuyor. Bu, tipli plan kararları ve PlanGML profil doğrulayıcısının bulunduğunu göstermez. |
| K21 | [scenarios_edit.rs][K21]: kaynak katmanlara `replaces` bağıyla senaryo kopyası ve tek adımda uygulama var. Mevcut Harita yüzeyi senaryo hücresini göstermiyor; GIS senaryo altyapısı tekrar yazılmayacak. |
| K22 | [rasters_georef.rs][K22], `fit`, `prepare`, `validate/plan/execute`: oturtma ve hazırlama mevcut. [STAC modülü][K23] katalog, tarih, erişim ve hazırlama; [COG yazımı][K24] karoları seçmeli erişime uygun yerleştirme sağlıyor. |
| K25 | [pointcloud/ops/mod.rs][K25], [ground.rs][K26] ve [query.rs][K27]: akan kayıt işlemleri, SMRF zemin sınıflama ve kaynak kayıt bilgisini sorgulama var. Bunlar kayıt/şerit dengelemesi veya semantik nesne çıkarımı paketi değil. |
| K28 | [delaunay.rs][K28]: exact yön/incircle predicate ve half-edge temeli var. [raster/contours.rs][K29] DEM'den marching squares eğrisi üretiyor. Kırık hatlı, tariften yeniden kurulan mühendislik TIN'i ayrı iş. |
| K30 | [cad_meshes.rs][K30]: UGRID/NetCDF mesh sorgusu, alan istatistiği ve profil var; komutlar salt okunur. Okunabilir mesh ile düzenlenebilir tasarım yüzeyi aynı yetenek değil. |
| K31 | [network.rs][K31]: çizimden türetilen analiz ağı tasarım güzergâhından açıkça ayrılıyor. [LRS çekirdeği][K32] M değerli/yaylı güzergâh sunuyor; bu da düşey kurp ve koridor tasarımı değildir. |
| K33 | [gis_derive.rs][K33] ve [gis_takeback.rs][K34]: kimlik/özetle izlenebilir CAD→GIS türetme ve kontrollü geri alma var. İki tarafta değişmiş kaynak reddediliyor; eğrinin kirişlere dönüşmesinin kaybı bildiriliyor. |
| K35 | [xref.rs][K35]: kaynak özeti ve yüklenmiş içerik korunuyor; kaynak değişikliği sessiz alınmıyor. Dosya bağı yönetimi zaten var. |
| K36 | [atlas.rs][K36], `atlas_plan`, [display/map.rs][K37] ve [pdf/geo.rs][K38]: atlas, koordinat çerçevesi ve coğrafi PDF temeli mevcut. [preflight.rs][K39] bulguları kod, önem ve düzeltmeyle taşıyor. |
| K40 | [processing/lib.rs][K40]: salt okunur girdi → ChangeSet → tek Undo; model birden çok aracı bağlayabiliyor. [server/jobs.rs][K41] lease, fence, iptal, kota ve revizyon korumalı sonuç yazımı içeriyor. |
| K42 | [job_cache.rs][K42]: çıktı önbelleği revizyon, erişim, kullanıcı/gün ve yazılım damgasına bağlı. Genel yeniden üretilebilir mühendislik bağımlılık grafiğinin var olduğu sonucu buradan çıkarılamaz. |
| K43 | [headless/dispatch.rs][K43], [python/lib.rs][K44] ve [mcp/tools.rs][K45]: ortak katalog/işleyici, Python ve MCP temeli mevcut. Her yeni Harita işinin bütün yüzeylerde eksiksiz olması ayrıca doğrulanmalı. |

## 20 ana geliştirme maddesi

### MAP-01 — Harita işini kaynak, tasarım, denetim ve teslim etrafında düzenle [ ]

**Öncelik:** P0. **Kod dayanağı:** [K01], [K02], [K03].
**Araştırma:** Netcad'in uzmanlık modülleri ve OpenCities'in şema yaklaşımı,
tek bir kalabalık CAD şeridinin bütün iş akışını anlatmaya yetmediğini
gösteriyor. [Netcad GIS][S01], [OpenCities Map][S10]

- Harita projesine anlaşılır bir iş yapısı getir: ölçüm kampanyaları,
  referans kaynaklar, çalışma verisi, tasarımlar, kontrol sonuçları ve teslim
  paketleri. Bunlar dosya uzantısına göre dağılmış pencerelerden erişilebilir
  proje öğelerine dönüşsün; mevcut Katmanlar/Kaynaklar/İşlemler korunarak bağlansın.
- Kaynağı, kaynaktan türetilmiş nesneyi ve kullanıcının bağımsız çizimini
  özelliklerde ayır. Nesne kimliği, kaynak revizyonu, üretim işlemi ve güncellik
  bilgisi birlikte erişilsin. Her geometriyi yeni özel entity türüne çevirmek
  yerine hangi anlamın sözleşme/ilişki/katman profilinde tutulacağı tasarlansın.
- Nokta, yüzey, plan alanı veya altyapı öğesi seçildiğinde yalnız ilgili
  bağlamsal araçlar gelsin. Ağır toplu araçlar İşlemler'de kalsın; şeritte işe
  giriş ve sık kullanılan düzenlemeler yer alsın. Araç arama aynı kayıtları kullansın.
- Tipli ölçüm/plan/varlık katmanları tür ve alan kurallarını uygulasın; serbest
  CAD çalışma katmanları korunabilsin. Bilinmeyen kaynak veya eksik CRS için
  kullanıcıyı her adımda tekrar sorgulayan bir proje akışı tasarlanmasın.
- İşin durumu “taslak / denetlenecek / onaylı teslim revizyonu” gibi açık
  kapsamla tutulabilsin. Teknik kontrolü geçen sonuç otomatik kurum onayı almasın.

**CBS ortaklığı:** kaynak kimliği, tipli veri profili, güncellik ve kontrol
sonucu ortak sözleşmedir. Harita'nın CAD çizim araçları ve GIS'in tematik/
senaryo yüzeyi kendi programında kalır; ortaklık proje türünü değiştirmez.

**Kabul:** aynı projede GNSS kaydı, ortofoto, CAD referansı ve tasarım yüzeyi
eklendiğinde her nesnenin kaynağı bulunabilsin. Kaynak taşınınca tek yerden
yeniden bağlansın; güncel olmayan türevler gösterilsin. Kaydet–aç ve salt
okunur kullanıcı aynı ilişkiyi korusun; mevcut Mimari/GIS yüzeyleri değişmesin.

**6 Kasım 2026 — MAP-01.1 (dal `map-01-is-yapisi`, [ADR 1200](../docs/adr/1200-map-work-structure-and-provenance.md)).**
Kanıtlanan alt maddeler: (1) iş yapısı — `settings.mapWork` (`.kcad` şema 188): altı türde öğe
(kampanya, referans, çalışma, tasarım, denetim, teslim), katmanları ve varsa dosyası (SHA-256 revizyonu); masaüstünde
yalnız Harita'da Harita işi paneli, Katmanlar/Kaynaklar/İşlemler yerinde. (2) Kaynak / türetilmiş / bağımsız çizim —
anlam katman profilinde, ilişki türetme kaydında (nesne, kaynak öğe, revizyon, işlem), yeni nesne türü yok; Öznitelikler'in
Kaynağı bölümü kimliği, kaynak revizyonunu, işlemi ve güncelliği birlikte gösterir. (5) Durum taslak / denetlenecek /
onaylı teslim revizyonu: ofisin kaydı, kurum onayı değil; güncel olmayan türevi taşıyan öğe onaylanmaz. Kabulden
kanıtlananlar: aynı projede GNSS kampanyası, ortofoto, CAD altlığı ve tasarım yüzeyi; her nesnenin kaynağı bulunur;
kaynağın yeni yeri tek komutla (öğe ve onu gösteren raster/resim tek geri alma adımında) bağlanır; içeriği değişen
kaynağın türevleri “Güncel değil”; kaydet–aç ve salt okunur okuma aynı ilişkiyi verir; Mimari, GIS ve Yol'da panel
ve satırlar yok. `map.work.query` 22, `map.work.edit` 40 ortak durum (bağımsız `scripts/fixtures/map_work_cases.py`).
**Açık:** içe aktarma ve çizim komutlarının türetme kaydını kendiliğinden yazması (bugün `derive` açık komutla), dış
başvuru ve bağlı kaynağın öğeyle aynı yerden yeniden bağlanması, dosyasız öğe revizyonu ve teslim damgası, seçime göre
bağlamsal araçlar (alt madde 3), tipli katman kuralları (alt madde 4), göreli yollar, gerçek kullanıcı projesinde GUI
kabulü. Madde `[ ]` kalır.

**6 Kasım 2026 — MAP-01.2 ([ADR 1201](../docs/adr/1201-map-work-relative-files-relink-and-contextual-tools.md)).**
Kapsam işareti: **yalnız Harita (CAD)** — harita işi `cadWork: map` projesinin ayarıdır; komutlar yalnız Harita'nın
bağlamsal panellerindedir, KentOS GIS/Mimari/Yol sunmaz. **Ortak (Harita + CBS):** göreli yol kuralı, dosya arama
(`links::find`), Paketle ve `cad.assets.check`. Araştırma: AutoCAD `REFPATHTYPE` (varsayılan göreli), QGIS/ArcGIS Pro
göreli kaynak yolu, ArcGIS Pro'nun kırık bağları tek yerden onarması, Civil 3D'nin seçime göre bağlamsal sekmesi ve Launch
Pad'i, veri başvurusunun promote'u; Netcad'in bu konuda resmî belgesi bulunamadı. Düzeltilen hata: harita işi dosyası
yalnız mutlak yol taşıdığından klasörüyle taşınan ya da Paketle'yle gönderilen projede bütün kaynak öğeler “Dosyası yok”
oluyordu. Kanıtlanan alt maddeler: (a) öğe dosyasının göreli yolu (`.kcad` şema 193): kayıtta yazılır, açılışta içeriği
aynıysa alınır, `map.work.query drawing` göreli yolu ve arama yollarını dener (`via`, `at`), Paketle kopyalar,
`cad.assets.check` listeler; (b) tek yerden yeniden bağlama aynı içeriği yükleyen dış başvuruyu da geçirir, ötekini ve
bağlı GeoPackage'ı uyarıyla bırakır; (c) `release` (Bağımsız yap); (d) alt madde 3'ün ilk parçası: Seçim/Nokta/Çizgi/
Alan/Raster bağlamsal sekmelerinde Harita işi paneli (Kaynağını göster, Bağımsız yap; seçimin rolüne göre açık/kapalı) ve
panelde Seçimi türet; sahibin düzeltmesiyle arayüzde “kampanya” yok (Arazi ölçümleri, Altlıklar, Denetimler, Teslimler) ve
panelin üstü tek araç satırı. Ortak durumlar `map.work.query` 27, `map.work.edit` 48 (bağımsız `map_work_cases.py`;
Python koşucusundaki `$tmp:` hatası düzeltildi, 21 eski Python düşüşü kapandı). **Açık:**
içe aktarmanın türetme kaydını kendiliğinden yazması (sahip kararı), bağlı GeoPackage'ın geri alınabilir yeniden bağlanması,
nesne türüne göre (yüzey, plan alanı, altyapı) ayrı bağlamsal araçlar, tipli katman kuralları, dosyasız öğe revizyonu.
Madde `[ ]` kalır.

### MAP-02 — Hassasiyet, koordinat, epok ve ölçeği tek güvenilir zincire bağla [ ]

**Öncelik:** P0. **Kod dayanağı:** [K04]–[K08], [K09].
**Araştırma:** PROJ düşey ızgara ve zamana bağlı deformasyonu ayrı işlemler
olarak tanımlar; bunlar ekran ölçeği değildir. [PROJ vgridshift][S25],
[PROJ deformation][S26]

- Mevcut geoit/hız ızgarası ve yer–projeksiyon hesabını genişletirken
  işlem kaydına yatay/düşey CRS, eksen/birim, koordinat epoğu, kullanılan
  ızgara özeti/sürümü, kapsam ve doğruluk dayanağını bağla. Uygulamanın
  desteklediği CRS ailesi açık olsun; kayıtlı her EPSG kodu çözülebiliyor varsayılmasın.
- UI, Python, MCP, sorgu, düzenleme ve dışa aktarma aynı koordinat
  sözleşmesini kullansın. Birime çevrilmiş ekran metni yeniden hesap kaynağı
  olmasın. Alan/uzunluk için düzlem, elipsoit veya yer hesabı açıkça seçilsin.
- Model f64, sağlam geometrik predicate ve gerektiğinde kesin kayıtlı sayı
  politikası korunsun. GPU'ya mevcut yerel orijin + hi/lo f32 yolu üzerinden
  gidilsin. Yeni 3B karo, bulut veya seçim yolu büyük koordinatı doğrudan tek
  f32'ye indirip sonra ölçüm sonucu üretmesin.
- Ölçek anlatımını sadeleştir: çizim/pafta ölçeği anotasyonu etkiler; kamera
  zoom'u geometriyi etkilemez; projeksiyon/yer faktörü yalnız tanımlı ölçme
  hesabını etkiler. İçe aktarmadaki birim kararı bir kez uygulansın; ikinci
  kez blok, yazı veya kot ölçeklenmesin. Ayrıntı özelliklerde, günlük kullanımda
  tek anlaşılır özet olsun.
- Izgara bulunamadığında, kapsam dışına çıkıldığında veya epok belirsizken
  “başarılı hassas dönüşüm” sonucu üretilmesin. Önizleme farkları ve nedenleri
  göstersin; kaynak veri üzerine sessiz CRS ataması yapılmasın.

**CBS ortaklığı:** CRS işlemi, ölçüm semantiği, hata/uyarı ve kaynak kaydı
Harita ve GIS'te aynı çekirdekten gelir. GIS ekran etiketi ile CAD gerçek
boylu metin render davranışı birleştirilmez.

**Kabul:** yaklaşık E=500.000/N=4.400.000 m çevresinde milimetrelik fark,
uzak iki bölge, yerel→gerçek koordinat ve epok/geoit örnekleri bağımsız
başvuruyla sınansın. Zoom/pan, render orijini değişimi ve save–reopen ölçüm
sonucunu değiştirmesin. Sayısal bütçe her işlem için tanımlansın; sadece f64
kullanılmış olması doğruluk kanıtı sayılmasın.

**6 Kasım 2026 ilerlemesi ([ADR 1230](../docs/adr/1230-coordinate-chain-epoch-grids-and-budget.md)), madde açık:**
`cad.crs.transform`'un cevabı koordinat zincirini taşır: iki tarafın yatay sistemi (kod, aile, datum, birim, KentOS'un
ve EPSG'nin eksen sırası, WGS 84 datum topluluğunun 2 m'si), düşey sistemi, epoku ve epokun nereden geldiği; okunan
ızgaraların türü, SHA-256'sı, düğüm kutusu ve projenin doğruluğu; her adımın doğruluk dayanağı; `complete`/`limited`
kararı ve nedenleri (`chain_limited` uyarısı); sayısal bütçe (1e-6 m, 1e-9°, 1e-6 m kot). Kayıtta olmayan proje kodu
`unsupported_srid` ile reddedilir. Kanıtlananlar: E=500 000/N=4 400 000'de milimetre, Edirne–Hakkari, yerel → gerçek,
hız ızgarasıyla epok, geoit ve ızgara dışı, tek yanda epok PROJ'a karşı 21 yeni ve 53 genişlemiş ortak durumla
(`crs_chain_cases.py`, `crs_vertical_cases.py`, `geoid_cases.py`); zoom/pan, çizim orijini değişimi ve `.kcad`
kaydet–aç ölçüyü ve İçine tıkla halkasını bit bit değiştirmez (masaüstü `coordinate_chain_tests`); işlem başına bütçe
tablosu ADR 1230 §4'te. Açık: ölçek anlatımının tek özeti ve içe aktarmada birimin bir kez uygulandığının sınaması;
sorgu/düzenleme/dışa aktarmanın aynı zinciri taşıması; önizlemede nokta başına fark; geoit ve hız modelinin doğruluğu;
zamana bağlı Helmert ve resmî TUREF hız alanı; web.

**6 Kasım 2026 — MAP-02.2 ([ADR 1231](../docs/adr/1231-scale-summary-and-crs-factors.md)): ölçek anlatımının tek özeti.**
Kapsam işareti: **Harita + CBS** (Yol da; Mimari'nin yerel projesinde yok). Araştırma: Civil 3D'nin Units and Zone /
Transformation ayrımı ve birleşik çarpanı, Trimble Access'in zemin ölçek çarpanı. Kanıtlanan: durum çubuğunun ölçek
menüsünde (CAD'de çizim ölçeği, CBS'de görünüm ölçeği) **Ölçeklerin özeti ▸** alt menüsü — çizim ölçeği, ekran, ekranın
ortasında projeksiyon ölçeği, yükseklik ve birleşik çarpan (ppm ile), uzunlukların indirilip indirilmediği; satır değeri
panoya kopyalar; zoom yalnız ekran satırını değiştirir. Python/MCP için salt okunur `cad.crs.factors` (24 ortak durum,
PROJ'un `get_factors`'ına karşı 10⁻⁹). Açık: içe aktarmada birimin kot ve iç içe blokta bir kez uygulandığının sınaması,
ölçme pencerelerinin çarpanı bu komuttan okuması, sorgu/düzenleme/dışa aktarmanın aynı zinciri taşıması, nokta başına
fark önizlemesi, geoit/hız modeli doğruluğu, zamana bağlı Helmert.

### MAP-03 — Modern ölçüm kampanyası ve GNSS veri arşivi kur [ ]

**Öncelik:** P0 veri modeli; P1 çevrimdışı işleme. **Kod:** [K09]–[K12], [K03].
**Araştırma:** Civil 3D gözlem/kontrol/ekipman verisini çizimden ayırır;
TBC baz işleme öncesinde oturum, anten ve yörünge seçimlerini ele alır.
[Survey Databases][S08], [GNSS baseline processing][S12]

- Kampanya, oturum, sensör/anten, operatör, zaman aralığı, orijinal veri
  dosyası, koordinat/epok ve kalite kaydını kalıcı modelde tasarla. Ham
  gözlem, sağlayıcının çözümü ve projeye kabul edilen nokta ayrı kimlikler taşısın.
- Mevcut GPX/NMEA okuyucusunu yeniden yazma. Fix/float, uydu/HDOP ve
  yükseklik bilgisini koruyan içe aktarmayı kampanyaya bağla. HDOP'u metre
  cinsinden doğruluk veya “RTK sabit” bilgisini tek başına doğruluk garantisi sayma.
- Modern GNSS için RINEX ve çözüm/baz vektörü değişim profilleri belirle;
  anten yüksekliği/referansı, düzeltme kaynağı, yaş/gecikme, zaman sistemi,
  tam kovaryans ve işleme raporunu taşı. Önce denetlenebilir sonuç alışverişi
  teslim edilsin; PPK/PPP çözücüsü adaptörü ayrı doğruluk/lisans değerlendirmesiyle gelsin.
- Aynı işaretin farklı günlerdeki ölçülerini üstüne yazma. “Son ölçülen”,
  “kabul edilen” ve “dengelemeye giren” çözümler anlaşılır olsun; veri dışlama
  nedeni ve yeniden işleme tarifi saklansın. Kalitesiz ölçüyü kayıttan silmeden dışlayabilsin.
- Canlı RTK/NTRIP ve sensör bağlantısı opsiyonel adaptördür. Önce kayıtlı
  oturum replay'i, kesilen veri, float'a düşüş ve zaman atlaması sınansın;
  gerçek alıcı kapısı açılmadan canlı ürün özelliği teslim edildiği yazılmasın.

**CBS ortaklığı:** kabul edilmiş ölçü noktaları, kalite ve kaynak ilişkileri
GIS'te okunup yetkili formdan incelenebilmeli. GNSS arşivi ve dönüşüm aynı
çekirdektir; GIS'e çizim karnesi penceresi taşımak zorunlu değildir.

**Kabul:** bir noktaya iki oturum ve iki çözüm bağla; anten/epok kararını
değiştirip yeniden üret. Eski rapor korunurken hangi nokta ve yüzeyin
güncelliğini yitirdiği görülsün. Kapat–aç, eksik kaynak ve iptal hiçbir ham
veriyi kaybetmesin. Gerçek donanım kabulü ayrıca beklesin.

**6 Kasım 2026 — MAP-03.1 (dal `map-03-gnss-kampanya`, [ADR 1340](../docs/adr/1340-gnss-campaign-archive.md)), madde açık.**
CAD/GIS işareti: arşivin düzenlenmesi (GNSS arşivi paneli, Ölçüm oturumu ekle, `map.survey.edit`) yalnız KentOS
Harita'nın; kabul edilen noktalar ve kalitesi (Kabul edilen noktalar, Öznitelikler › GNSS ölçümü, `map.survey.query`)
Harita ve GIS'in. Kanıtlanan alt maddeler: (1) kalıcı model `settings.mapSurvey` (`.kcad` şema 194, geçici numara):
arazi ölçümü (harita işi öğesine yalnız kimliğiyle bağlı; epok kararı, hız ızgarası), ölçüm oturumu (işaret, zaman ve
zaman sistemi, alıcı, anten: ARP'ye düşey ya da eğik √(S²−R²)−offset, operatör, orijinal dosya SHA-256'sıyla, ham
ölçümler), çözüm (alıcı, işlenmiş, PPP; çerçeve ve epok, uygulanmış anten yüksekliği, fix, epok/uydu/HDOP, σ, tam
kovaryans, düzeltme kaynağı ve yaşı, işleme raporu, yeniden işleme tarifi), kabul edilen nokta (sonucu ve eski
sonuçları) ve dayanan; ham ölçüm, sağlayıcının çözümü ve kabul edilen nokta ayrı kimliklerdedir. (2) Mevcut GPX/NMEA
okuyucusu yeniden yazılmadan oturuma bağlandı (fix, uydu, HDOP, yükseklikler korunur; HDOP birimsiz, RTK sabit
doğruluk garantisi sayılmaz). (3) RINEX 2.11/3.05/4.00 gözlem başlığı okunur (işaret, alıcı, anten ve DELTA H/E/N,
ilk/son gözlem zamanı ve zaman sistemi, aralık); işlenmiş/PPP çözümü kovaryansı ve raporuyla alınır. (4) Aynı işaretin
günleri ayrı oturumdur; son ölçülen, kabul edilen ve dengelemeye giren ayrı görünür; çözüm nedeniyle silinmeden
dışlanır; anten ya da epok kararı değişince nokta güncel değildir, yeniden üretme eski sonucu korur, nesneyi taşır ve
ona dayanan yüzeyi güncel değil gösterir. (5) Kayıtlı oturum oynatması: kesik satır, RTK sabitten düşüş ve zaman
atlaması sayılır. Kabulden kanıtlananlar: bir noktaya iki oturum ve iki çözüm; anten kararı değişip yeniden üretildi
(eski rapor korundu, yüzey güncel değil); epok kararı hız ızgarasıyla yeniden üretildi, ızgarasız reddedildi;
kapat–aç (`.kcad`), eksik kaynak dosyası ve kapatılan dosya penceresi ham veriyi kaybetmez. `map.survey.query` 22,
`map.survey.edit` 73 ortak durum (bağımsız `scripts/fixtures/map_survey_cases.py`: kurallar ADR'den, RINEX sütunlarla,
NMEA `gnss_nmea_cases.py` ile, projeye taşıma PROJ ile). **Açık:** baz vektörü ve tam kovaryanslı ağ dengelemesi
(`adjust` bugün işarettir), dış merkezli anten indirgemesi ve ANTEX, RINEX gözlemlerinin kendisi, PPK/PPP çözücü
adaptörü, GGA'nın düzeltme yaşı, dayanan kaydının yüzey komutlarınca kendiliğinden yazılması, gerçek kullanıcı
projesinde GUI kabulü, web. Canlı RTK/NTRIP (HYB-24) ertelendi; gerçek alıcı kabulü ayrıca bekler. Madde `[ ]` kalır.

**7 Kasım 2026 — Arşivden GNSS ağı ([ADR 1740](../docs/adr/1740-gnss-network-from-archive.md)), MAP-03.2 ve MAP-05.2
birlikte.** Kapsam: **Harita + CBS** (düzenleme ve pencere Harita'nın; `map.survey.network` Harita ve CBS'nin, salt
okunur). Araştırma:
- NGS'nin GVX 1.0 belgesi: işaretten işarete ECEF vektörü, SD/P korelasyon matrisi, GPS zamanı;
- TBC: dengelenmiş koordinat ek kayıtla son değer olur, karşılaştırma raporu;
- Leica Infinity: önce iç kısıtlı, sonra kısıtlı dengeleme, sonuç “saklanınca” yazılır;
- BÖHHBÜY Madde 15, 20, 22: tek noktaya dayalı serbest dengeleme ve uyuşumsuz baz testi, sonra TUSAGA-Aktif'e dayalı
  dengeleme; C1/C2/C3 baz ve nokta σ sınırları; güven 0,95.

Kanıtlanan alt maddeler:
- **Model:** arşivde baz vektörü (`bz-N`, tam kovaryans, dosyası ve kimliği, dışlama, dengeleme seçimi), bilinen nokta
  (`bn-N`, sabit ya da σ'lı) ve dengeleme kaydı (`da-N`: bazlar, bilinen noktalar, tutulan nokta, çarpan, sınıf, iki
  aşamanın m0'ı ve serbestliği). `.kcad` şema 199; bağımsız okuyucu da.
- **Veri alışverişi:** GVX 1.0 ve KentOS baz metninden içe aktarma.
- **“Dengelemeye giren”:** artık gerçekten dengelemeye giriyor.
- **Kabulden kanıtlanan:** anten ya da epok değil, bir baz dışlanınca ağın noktaları güncel değil, nedeni yazılı.

Ortak durumlar: `map.survey.network` 30, `map.survey.edit`'e 33 yeni (bağımsız `map_survey_network_cases.py`).
**Açık:** GVX oturum çapraz korelasyonları, bilinen noktaların hız ızgarasıyla ölçme epokuna taşınması, arşiv
panelinde bazlar, gerçek kullanıcı projesinde GUI kabulü, web. Madde `[ ]` kalır.

### MAP-04 — Semantik nesne kataloğu ve kontrollü otomatik harita üretimi sağla [ ]

**Öncelik:** P1. **Kod dayanağı:** [K12], [K20], [K25]–[K27].
**Araştırma:** TBC nokta bulutundan çıkarılan öğeyi kod ve özniteliği olan
nesneye dönüştürüyor; yalnız renklendirme ayrı bir yetenek.
[Point feature extraction][S15], [Scanning and point clouds][S14]

- Yol kenarı, bordür, bina izi, kapı/adres, direk, rögar, vana, ağaç ve
  kontrol noktası gibi belediye varlıkları için sürümlü katalog tanımla:
  kalıcı tür kodu, geometri/Z gereksinimi, zorunlu alan, domain, ilişki,
  sembol ve kalite kuralları. Katalog proje/kurum dosyasında taşınabilsin.
- Mevcut domain/form/profil altyapısını kullan. Katman adı, nesne etiketi ve
  sembol kimliği nesnenin gerçek türünün yerine geçmesin. Aynı varlık birden
  çok kaynakta görüldüğünde eşleme kimliği ve uyuşmazlık kaydı olsun.
- LiDAR/ortofoto destekli yarı otomatik çıkarımı önce aday olarak üret:
  kaynak bölge/kayıt, yöntem/model sürümü, güven/kalite bilgisi, gözden
  geçiren kişi ve düzeltme saklansın. Belirsiz aday sessiz kesin varlığa dönüşmesin.
- Çizgi/alan çıkarımında devamlılık, kesişim, yükseklik sıçraması ve
  örtülmüş bölüm ayrı işaretlensin. “Görüntüde görünmüyor” durumunda varsayılan
  kesin geometri uydurulmasın. Toplu kabul/ret ve kaynağa gitme mümkün olsun.
- Kurum katalog düzenleyicisi örnek nesneyle sonucu önizletsin; katalog
  değişikliği mevcut veri için geçiş raporu üretsin. AI modeli indirme/çalıştırma
  isteğe bağlı, iptal edilebilir ve verinin nereye gittiği anlaşılır olsun.

**CBS ortaklığı:** tür, domain, form, ilişki, varlık kimliği ve çıkarım
inceleme katmanı GIS'te de kullanılacaktır. Harita çizim sunumu ile GIS
tematik gösterimi aynı semantik verinin ayrı temsilleri olur.

**Kabul:** aynı sokağın bulutundan ve ortofotosundan gelen direk/bordür
adaylarını uzlaştır; yinelenen varlığı önle, eksik zorunlu alanı bul,
yanlış sınıfı düzelt. Model/katalog değiştiğinde kabul edilmiş insan
düzeltmeleri korunabilsin; işlemin tamamı bir kez geri alınabilsin.

**6 Kasım 2026 — MAP-04.1 ([ADR 1360](../docs/adr/1360-field-codes-and-kodla-ciz.md)): saha kodları ve Kodla çiz.**
Kapsam işareti: **Harita + CBS** (Mimari ve Yol'da yok). Araştırma: TBC'nin özellik kodu kütüphanesi (FXL, denetim
kodları), Civil 3D'nin açıklama anahtarları, OpenCities Map'in şemalı özellikleri; Netcad'in kod tablosu ve BÖHHBÜY obje
kataloğunun metni bulunamadı. Yeni katalog kurulmadı: veri profilinin sınıflarına saha kodları (`codes`) eklendi, yerleşik
**Harita varlıkları** profili (kontrol noktası, rögar, vana, direk, ağaç, kapı, bordür, yol kenarı, duvar, bina izi, yeşil
alan) geldi; `map.features.code` kodlu noktaları sınıfın katmanına koyar, BRD1/BRD2 gibi dizileri çoklu çizgi, K ile
kapanan alan dizilerini alan yapar (B/K/S denetim kodları). 26 ortak durum (bağımsız `map_features_cases.py`), masaüstü
penceresi ve Profili uygula. Açık: sembol/blok, eğri denetim kodları, iki kaynaktan aday eşleme ve inceleme, AI çıkarım,
katalog değişikliğinin geçiş raporu.

**7 Kasım 2026 — MAP-04.2: iki kaynaktan aday uzlaştırma ([ADR 1760](../docs/adr/1760-asset-candidate-reconciliation.md)).**
Kapsam: **Harita** (pencere ve yazma); komut Python ve MCP'de. Araştırma:
- Esri Detect Feature Changes: arama mesafesiyle eşleme; N, D ve NC işaretleri; sonucun gözden geçirilmesi;
- Esri Detect Objects'in güven puanı ve NMS'i: yineleneni siler; KentOS silmez, işaretler.

Kanıtlanan alt maddeler:
- `map.features.reconcile` nokta ve çizgi adaylarını Harita varlıkları profilinin sınıflarıyla gruplar. Gruplar uyuşan
  (iki kaynak bir sınıfta), tek, çelişen ve zaten var olandır. Yinelenen aday işaretlenir; eksik zorunlu alan bulunur.
- Belirsiz aday sessizce kesin varlığa dönüşmez: tek kaynak kabul ister, çelişki düzeltme ister.
- Kabul edilmiş insan düzeltmeleri korunur: aday `Uzlaştırma` ile işaretlenir ve bir daha okunmaz. Kabul edilmiş
  varlığın yanındaki yeni aday yeni varlık yapmaz.
- Köken (Kaynaklar, Adaylar, Yayılım, İnceleme) özniteliktedir.
- İşlemin tamamı tek geri alma adımıdır.
- Kabulden kanıtlanan: aynı sokağın LiDAR ve ortofoto adaylarıyla yinelenen önlendi, KapiNo eksiği bulundu, ağaç adayı
  direğe düzeltildi.
- 25 bağımsız durum (`map_features_reconcile_cases.py`); masaüstü Adayları uzlaştır (Ölçme › Noktalar).

**Açık:** alan sınıfları, inceleme kaydının özniteliğe değil ayrı kayda yazılması ve katalog geçiş raporu, AI çıkarımı,
pencerede eksik alan girişi, büyük veri. Madde `[ ]` kalır.

### MAP-05 — Üç boyutlu belirsizlik ve büyük ağ kalite yönetimini geliştir [ ]

**Öncelik:** P1. **Kod dayanağı:** [K13]–[K15], [K04]–[K06].
**Araştırma:** TBC ağ sonuçlarında kovaryans/kalite ve yeniden hesap etkisini
gösterir; DynAdjust 3B jeodezik ağlar için incelenebilecek açık referanstır.
[Adjust Network][S13], [DynAdjust][S24]

- Mevcut yatay/kot en küçük kareler ve artık/elips hesaplarını koru.
  Yeni kapsam GNSS baz vektörü, korelasyonlu XYZ/ENU kovaryansı, 3B kontrol
  ve datum kısıtlarıdır. Kaynaktaki belirsizlik ile kullanıcının varsaydığı
  önsel belirsizlik ayrı gösterilsin; bilinmeyen değer sıfır hata olmasın.
- Serbest/kısmen bağlı ağ ve rank eksikliği için açık datum stratejisi
  tasarla. Aykırı gözlem otomatik silinmesin; robust seçenek, ağırlık değişimi,
  test varsayımı ve kullanıcı dışlaması raporda ayırt edilsin.
- Yoğun normal matrisi büyüterek ölçeklemeye çalışma. Seyrek yapı,
  koşulluluk, seçili kovaryans çözümü ve büyük ağın bölümlü işlenmesini
  bağımsız doğruluk ve bellek ölçümüyle değerlendir. Küçük ağda mevcut
  sonuçla eşdeğerlik, büyük ağda kestirilen bellek/CPU sınırı aransın.
- Analiz sonucu önce fark vektörü, artık, kontrol noktası etkisi ve
  güven bölgesiyle gösterilsin. Kullanıcı kabul ettiğinde seçilen çözüm
  yazılsın; bağlı geometri/yüzeyler değişiklik grafiğine girsin.
- Farklı epoklu kampanyalar ve deformasyon izlemede ortak referans ve
  belirsizlik hesaba katılsın. Bir görüntü farkı veya iki XYZ farkı kendiliğinden
  istatistiksel anlamlı deformasyon sayılmasın.

**CBS ortaklığı:** kontrol noktası, artık/fark vektörü, güven bölgesi ve
zaman serisi GIS'te katman/sorgu olarak erişilebilir. Tek dengeleme ve
datum motoru kullanılır; GIS'e ikinci çözücü yazılmaz.

**Kabul:** bağımsız 3B ağ başvurusuyla sabit/ağırlıklı kontrol,
korelasyonlu baz, eksik datum ve aykırı ölçü sınansın. Analiz kaynak veriyi
değiştirmesin; kabul ve Undo bütün etkileri kapsasın. 1.500 bilinmeyeni
aşan hedef veri için gerçek bellek/süre raporu olmadan ölçeklenebilirlik işaretlenmesin.

**6 Kasım 2026 — MAP-05.1 ([ADR 1380](../docs/adr/1380-gnss-baseline-network-adjustment.md)): GNSS baz ağı dengelemesi.**
Kapsam işareti: **Harita + CBS** (Mimari ve Yol'da yok). Araştırma: TBC'nin minimal/tam kısıtlı ağ akışı, χ² ve
referans faktörü, vektör ağırlık ölçeği; GVX; DynAdjust. Kanıtlanan: korelasyonlu ECEF kovaryanslı baz vektörleri
(Cholesky ile korelasyonsuz), sabit ve ağırlıklı 3B kontrol, minimal datum, m0/χ²/Baarda (kaba hata silinmez,
işaretlenir), istasyonların doğu/kuzey/yukarı sapmaları; analiz hiçbir şey yazmaz. `cad.survey.baselines` (25 ortak
durum, numpy/scipy/pyproj bağımsız başvurusu), GNSS baz ağı penceresi. Açık: seyrek çözüm ve büyük ağ ölçümü, robust
kestirim, iç kısıtlı serbest ağ, GVX/SINEX, arşivden ağ kurma, kabul edilen çözümün yazılması, deformasyon.

**7 Kasım 2026 — MAP-05.2: arşivden ağ ve kabul ([ADR 1740](../docs/adr/1740-gnss-network-from-archive.md)).** Kapsam:
**Harita + CBS**. Kanıtlanan:
- **Kaynak ve önsel belirsizlik ayrı:** bilinen noktanın σ'sı kuzey/doğu/yukarı verilir, ECEF'te ilişkili kovaryansa
  döner. Çekirdek tam kovaryanslı ağırlıklı istasyonu öğrendi.
- **İki aşama:** tek noktaya dayalı serbest dengeleme ve bilinen noktalara dayalı dengeleme; her ikisinde χ² ve Baarda.
  Sınıfın güveni varsa (C1: 0,95) test onunla yapılır.
- **Analiz:** fark vektörü (kabul edilmiş noktadan dx, dy, dz) ve güven bölgesi (σ kuzey/doğu/yukarı); kaynak veri
  değişmez.
- **Kabul:** uyuşumsuz ölçü ya da düşen model testiyle reddedilir. Kabulde `da-N`, yeni noktalar ve eski sonucu
  korunarak taşınan noktalar ile ona dayananların güncelliği tek geri alma adımıdır. Aynı ağ ikinci kez kabul edilmez.
- **Sınıf:** BÖHHBÜY C1/C2/C3 baz ve nokta sınırları veri olarak (kural paketi).

Kabulün “kabul ve Undo bütün etkileri kapsasın” satırı kanıtlandı (ortak durumlar ve masaüstü testi). **Açık:** seyrek
çözüm ve 1500 bilinmeyen üstü ölçüm, robust kestirim, Madde 15(1)(c) benzerlik dönüşümü, deformasyon. Madde `[ ]`
kalır.

### MAP-06 — Kesin ve ilişkili harita tasarımını doğrudan sahnede yap [ ]

**Öncelik:** P0 temel etkileşim; P1 ileri ilişkiler. **Kod:** [K16], [K06], [K43].
**Araştırma:** QGIS'in gelişmiş çizim kodu açı/mesafe/X/Y/Z/M kısıtlarını
ortak hizalama çağrısında topluyor. Bu, UI ile komutun ayrı matematik
uygulamasını önlemek için somut örnektir. [QGIS gerçek kaynak kodu][S21]

- Mevcut kenetleme, kesin giriş ve kısıt altyapısını Harita'nın gerçek
  koordinatlarına uygula: referans çizgi/kenar, paralellik, diklik, teğet,
  öteleme, hedef kot/eğim ve seçilmiş geometriye bağlı mesafe.
  Giriş her zaman birimini ve referansını anlaşılır göstersin.
- “Şimdi bu değere getir” ile “bu ilişkiyi koru” farklı olsun. Kalıcı
  ilişki kullanıcı kararıyla kurulsun; serbest CAD nesnesi fark etmeden
  mühendislik modelinin bağlı öğesine dönüşmesin.
- Düz kenarla sınırlı mevcut alan/kenar işlemini olduğu gibi koru.
  Eğrisel ve çok parçalı kapsam yeni matematik/kabul işi olarak açılsın;
  yay doğruluğunu düşürüp sessiz kirişlere dönüştürerek çözülmesin.
- Önizleme hareket edecek nesneyi, sabit kalacak referansı, komşu sınır ve
  kot etkisini göstersin. Çelişkide hangi ilişkinin neden çözülemediği
  sahnede bulunsun; otomatik tolerans büyütme veya bağ koparma yapılmasın.
- Tek Enter/sağ tuş Onayla tek işlem, Esc sıfır yazım olsun. Sayısal
  alanlarda yazarken her tuşta bütün çizim hesaplanmasın; yerel taslak ve
  kontrollü önizleme kullanılsın. Araçlar mevcut sağ tuş/Undo sözleşmesini korusun.

**CBS ortaklığı:** kesin çizim yardımcıları ve uygun geometrik ilişkiler
GIS'in tipli katman düzenlemesinde de kullanılmalı. Örneğin bir çizgi
katmanında alan üretmeye çalışan UI/API aynı tür kuralını uygulamalı.

**Kabul:** gerçek koordinatta yaylı bordür/ada kenarı ve bağlı varlık
üzerinde mesafeli düzenleme yap; bağımsız geometri başvurusuyla karşılaştır.
Kaynak kısıtı değiştir, kilitli komşu ve çelişkili ilişkiyi dene. Çift onay,
imleç sıçraması, ölçekten doğan konum değişimi ve kısmi Undo olmamalı.

**Yaylı kenarda paralel kaydırma (6 Kasım 2026, [ADR 1024](../docs/adr/1024-parallel-shift-on-arcs.md)):**
Paralel kaydır artık yaylı kenarı (aynı merkezli yay) ve yay komşusunu (kendi çemberinde) kaydırır; hiçbir yay kirişe
dönmez, hedef alan yaylı durumda ikiye bölmeyle bulunur, düz durum aynen kalır. Harita koordinatında yaylı bordür dahil
12 yeni durum bağımsız mpmath başvurusuyla (50 basamak) sınandı; etkileşim izi ve resim bakıldı. Kalıcı ilişki, ortak
sınır, kot/eğim hedefi ve çok parçalı nesne açık; madde `[ ]` kalır.

**6 Kasım 2026 — Eğimle kot ver ([ADR 1290](../docs/adr/1290-grade-between-vertices.md)).** Kapsam işareti: **CAD (Harita
+ Yol)**. Araştırma: Civil 3D özellik çizgisinin Set Grade/Slope Between Points'i; parselin güzergâhı kendiliğinden
izlemediği (kalıcı ilişki ayrı karardır, ADR 0334 kısıtları). Kanıtlanan: hedef kot/eğim alt maddesi — `cad.entities.grade`
iki köşe arasına bitiş kotu ya da eğimle, yaylı kenarda yay boyunca kot verir (21 ortak durum), pencere Çizgi/Alan
sekmesinde. Formlar sahibin isteğiyle tek etiket sütunlu düzene geçti. Açık: köşeleri sahneden seçmek, kalıcı ilişkiyi
kurmak, ortak sınır, çok parçalı nesne.

### MAP-07 — Topolojik üretimi açıklanabilir kalite sürecine dönüştür [ ]

**Öncelik:** P0. **Kod dayanağı:** [K17], [K18], [K19].
**Araştırma:** GRASS temizleme araçlarının sıra ve eşiklerini açık tutar;
kodda birbirini izleyen işlemlerin topolojiyi hangi aşamada kurduğu görülür.
[v.clean belgesi][S22], [v.clean kaynak kodu][S23]

- Mevcut 13 kuralı, artımlı denetimi ve iptal desteğini kullan. Yeni iş;
  kurum/iş profili, bulgunun yaşam döngüsü, ortak sınır düzenlemesinin etkisi
  ve tekrar üretilebilir onarım tarifidir.
- Hata, bilinçli istisna ve henüz değerlendirilmeyen bulgu ayrışsın.
  İstisna nesne kimliği, kural sürümü, gerekçe ve kapsam taşısın; geometri
  değişince istisnanın hâlâ geçerli olup olmadığı yeniden değerlendirilsin.
- CAD yakınlık toleransı, koordinatın depolama hassasiyeti ve kurumun
  kabul eşiği ayrı olsun. Büyük toplu snap/temizleme orijinal ölçüm
  koordinatlarını sessiz yuvarlamasın; taşınan toplam/maksimum mesafe raporlansın.
- Ortak sınırın bir kez düzenlenmesi bütün yetkili komşularda atomik
  uygulansın. Kilitli/yetkisiz komşu, yay, delik, üçlü birleşim ve çoklu
  köşe durumlarında kısmi tutarsız veri yerine açıklanabilir ret/önizleme verilsin.
- Onarımın sırası ve parametreleri kaydedilsin. Önce/sonra farkı, alan
  etkisi, etkilenen varlıklar ve kaynak gözlemden sapma görülsün; toplu
  düzeltme kör bir “hepsini onar” düğmesi olmasın.

**CBS ortaklığı:** kural motoru, bulgu/istisna kaydı, ortak sınır ve toplu
onarım Harita/GIS'te aynıdır. GIS'te de Processing, veri formu ve MCP
üzerinden aynı denetim sonucu erişilir.

**Kabul:** komşu yaylı alanlar, delikli alan, kilitli komşu ve kaynak
değişimiyle artımlı/tam denetim sonuçları eşleşsin. İptal kaynak geometriyi
değiştirmesin. Bir onarım tarifi yeniden açılan projede aynı bulgulara
uygulandığında aynı değişiklik kümesini üretsin.

**İlerleme (6 Kasım 2026, dal `map-07-topoloji-kalite`, [ADR 1260](../docs/adr/1260-topology-quality-review-and-repair-recipe.md)):**
kanıtlanan alt maddeler: bulgunun yaşam döngüsü (`state`: değerlendirilmedi, hata, istisna, yeniden değerlendir) ve
`cad.topology.exceptions`'ın `error`/`reason`/`scope`'u; işaretin nesne kimliği, geometri özeti, kural sürümü
(`ruleDigest`), gerekçesi ve kapsamı, geometri ya da kural değişince düşmesi; kurum/iş kalite profili (ad, depolama
hassasiyeti, kabul eşiği; tolerans hassasiyetin en az iki katı) ve profilin gerekçe istemesi; `cad.topology.repair`
onarım tarifi (adım sırası ve düzeltmesi, adım öncesi yeniden denetim, nesne başına alan önce/sonra, bit bit yerinde
kalan ve taşınan kaynak köşe, en uzak/toplam taşınma, alan farkı, önce/sonra açık bulgu; istisna, kilitli komşu ve kabul
eşiği retleri; tek geri alma adımı); `.kcad` şema 189; masaüstü Durum sütunu, Gerekçe, Hata işaretle, Tarifle
onar ve kural penceresinde profil. Kabulün üç satırı: yaylı komşu, delikli alan ve yeniden okunan komşu ile artımlı/tam
denetim eşitliği (`topology_quality.rs`; kilitli komşu tarif retiyle ortak durumda); iptalin kaynak geometriyi
değiştirmemesi (`topology_repair::tests`); yeniden açılan projede aynı tarifin aynı değişiklik kümesi (ortak durum
`reopen`). **Açık:** ortak sınırın bütün yetkili komşularda atomik düzenlenmesi ve kilitli/yetkisiz komşu, yay, delik,
üçlü birleşim, çoklu köşe için açıklanabilir ret; büyük toplu snap/temizliğin (ADR 0148) taşınma raporu; bağlı kaynak
satırının değiştiği artımlı denetimin uygulama düzeyinde (band) sınanması; GIS Processing/veri formu yolu; GUI'de
gerçek fare/klavye ile tarif akışı kanıtı ve büyük veri ölçümü. Madde açık kalır.

**6 Kasım 2026 — kilitli komşuda durdur ([ADR 1261](../docs/adr/1261-topological-editing-stops-at-a-locked-neighbour.md)).**
Kapsam işareti: **Harita + CBS**. Araştırma: QGIS ve ArcGIS Pro'da topolojik düzenleme yalnız düzenlenebilir katmanları
taşır (kilitli komşu ayrılır). KentOS ikisini de sunar: ilk değer bugünkü ayırma; **Kilitli komşuda durdur** açıkken
tutamaç, Köşe tablosu, Esnet ve `cad.entities.vertex` kilitli komşuyla paylaşılan sınırı hiç yazmaz
(`shared_boundary_locked`). Açık: yetkisiz komşu, önizlemede kilitli komşunun vurgusu, yay/delik/üçlü birleşim.

**6 Kasım 2026 — taşınma raporu ([ADR 1262](../docs/adr/1262-topology-clean-move-report.md)).** Kapsam işareti:
**Harita + CBS**. Araştırma: AutoCAD Map 3D Drawing Cleanup'ın hata listesi ve işaretleri; ArcGIS Integrate'in taşınmayı
raporlamadığı (önce/sonra karşılaştırma önerir). Kanıtlanan: büyük toplu temizliğin taşınma raporu — `cad.topology.clean`
ADR 0148'in temizliğini komut yapar; her taşınma, nesne başına yerinde kalan/taşınan köşe, en uzak ve toplam taşınma,
alan/boy farkı, kabul eşiğini aşan taşınma (yazma reddeder); Taşınma raporu penceresi ve CSV (20 ortak durum, Rust ve
Python). Açık: ortak sınırın atomik düzenlenmesi, taşınmaların sahnede işaretlenmesi, büyük veri ölçümü.

### MAP-08 — Sayısal kadastro için kayıt, sınır ve tarihçeyi ayır [ ]

**Öncelik:** P1 teknik veri modeli; resmî işlemler son aşama kapısında.
**Kod:** [K19], [K17], [K09]. **Araştırma:** ArcGIS parsel ağında kayıtlı
ölçü ile geometrik koordinatı ve analiz ile uygulamayı ayırıyor.
[Parcel least squares][S17], [Analyze / apply][S18]

- Mevcut parsel polygonunu ortadan kaldırmadan sayısal parsel modelini
  tasarla: parsel kimliği, kaynak kayıt/belge, sınır ve köşe ilişkisi,
  ölçülmüş/kayıtlı değer, geçerlilik zamanı ve önceki/sonraki nesne bağı.
- Ortak sınır yalnız iki polygonun tesadüfen çakışan koordinatları
  olmasın. Mantıksal sınır kimliği üzerinden komşuluk ve kaynağı izlenebilsin;
  CAD/GIS geometrisi bunun denetlenebilir temsili olsun.
- Geometrik alan, kaynakta yazan alan ve kurumca kabul edilen alan
  farklı değerler olarak kalsın. Bir hesap güncellemesi kayıtlı alanı veya
  belgeyi değiştirmesin. Aynı il/ilçe/mahalle/ada/parsel bağlamında kimlik
  kontrolü tasarlansın; yalnız parsel numarası tekil varsayılmasın.
- Hak, pay, yükümlülük ve belge ilişkileri erişim kontrollü ayrı kayıtlar
  olsun. Kişisel bilgiler geometri dağıtılırken veya AI sorgulanırken
  varsayılan olarak bütün kullanıcılara açılmasın.
- Değişiklik taslağı ile onaylanmış kayıt ayrışsın. Birleştirme/bölme
  geçmişi ve kaynak uyuşmazlığı veri modelinde tasarlanabilir; resmî işlem
  kuralları bu maddenin teknik modeli hazır diye etkinleştirilmez.

**CBS ortaklığı:** parsel kimliği, ortak sınır grafiği, kayıt/hak ilişkisi,
tarihçe ve erişim denetimi GIS'te de kullanılmalı. Aynı belediyede CAD ve
GIS için çelişen iki parsel/hak veritabanı kurulmaz.

**Kabul:** aynı parselin iki farklı ölçüm ve belge revizyonunu tut; bir
sınır güncellendiğinde komşuluk ve geometrik alan değişsin, kaynak belge
değeri değişmesin. Yetkisiz kullanıcı malik/pay verisini sorgu, dışa aktarım
ve MCP ile alamamalı. Kabul resmî tescil uygunluğu iddiası içermez.

**6 Kasım 2026 — Parsel alan denetimi ([ADR 1400](../docs/adr/1400-parcel-area-check-and-identity.md)).** Kapsam işareti:
**Harita + CBS**. Araştırma: Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik (RG 27.09.2022/31966)
Madde 4, 7, 8; Madde 8(2) tablosu üç ikincil kaynaktan (resmî metinle karşılaştırma bekliyor); Netcad'in tapu alanını baz
alan özeti. Kanıtlanan: geometrik ve kayıtlı alan ayrı — `cad.parcels.check` hesap alanını tapu alanıyla (yazıldığı gibi)
karşılaştırır, fark ve yanılma sınırı, dört durum; kimlik İl/İlçe/Mahalle/Ada/Parsel birlikte, yinelenen ve kimliksiz
parsel (21 ortak durum). K19 düzeltildi: `cad.parcels.create` yinelenen numarayı ve sıradaki numarayı adanın içinde arar.
Açık: kalıcı parsel kaydı (belge, geçerlilik, tarihçe), mantıksal ortak sınır, erişim denetimli hak/pay, geçici
koordinatlı yöntem.

### MAP-09 — İmarı sembol çiziminden tipli ve denetlenebilir plan modeline taşı [ ]

**Öncelik:** P1. **Kod dayanağı:** [K20], [K19], [K33].
**Araştırma:** Planet plan verisi/denetimiyle, XPlanung makinece okunur
model ve doğrulama ayrımıyla değerli karşılıklardır. XPlanung Türkiye için
PlanGML şeması değildir. [Planet][S04], [XPlanung FAQ][S20]

- Plan, plan bölgesi, karar/işlev alanı, yapılaşma koşulu, ulaşım/koruma
  sınırı, plan notu ve dayanak belgeyi tipli nesne/ilişki olarak tanımla.
  Mevcut MPYY sembolünü bunlara bağla; renkten veya taramadan anlam tahmin etme.
- Plan seviyesi, sürümü, taslak/onay/yürürlük durumu ve üst/alt plan
  ilişkisi saklansın. Plan değişikliği kaynak revizyona bağlı taslak
  oluştursun; yürürlükteki veriyi doğrudan değiştirmesin.
- Kullanım türü, yoğunluk/kapasite, yükseklik, yapı yaklaşma ve özel
  koşulları birimli alan/domain olarak işle. Uygulanamayan veya bilinmeyen
  değer boş/0 ile aynı sayılmasın; plan notunun hangi alana uygulandığı belli olsun.
- Türkiye PlanGML için yürürlükteki resmî şema ve kurum profilini uygulama
  öncesi edinip sürümünü sabitle. Genel GML okuyucusunu yeniden yazma;
  şema eşleme, kod listesi, plan seviyesi, ilişki ve geometri denetimi ekle.
  Şema doğrulaması ile planlama/mevzuat denetimi ayrı sonuçlar versin.
- Alan sınırı, karar çelişkisi, eksik plan notu, kopuk yol devamlılığı ve
  üst/alt plan tutarsızlığı sahnede bulunabilsin. Kullanıcı bulgudan ilgili
  karar, kaynak belge ve etkilenen alanlara gidebilsin.

**CBS ortaklığı:** plan nesneleri, kod listeleri, durum, not/belge bağı ve
doğrulama zorunlu ortak veridir. GIS sorgu/servis/analiz olarak tüketmeli;
Harita hassas çizim ve plan düzenleme yüzeyini sunmalıdır.

**Kabul:** örnek planı sembol + tipli karar + plan notuyla kur, başka
profilli kaynağı eşle. Sembol değişikliği plan kararını değiştirmesin.
Desteklenen resmî profil için bağımsız doğrulayıcıyla sonuç karşılaştırılsın;
edinilmeyen veya doğrulanmayan profil “uyumlu” diye listelenmesin.

**6 Kasım 2026 — İmar planı denetimi ([ADR 1420](../docs/adr/1420-zoning-plan-typed-check.md)).** Kapsam işareti:
**Harita + CBS**. Araştırma: Bakanlığın plan standartları sayfası (MPYY ekleri, PlanGML doğrulama projesi; XSD
edinilemedi), QGIS planx_mpyy_studio (MPYY 1.1.7 alanları ve denetimleri, ikincil), Netcad Planet, PAİY tanımları.
Kanıtlanan: tipli plan nesneleri — yerleşik `imar-plani` profili (plan sınırı: kademe/durum/sürüm/üst plan/dayanak;
karar alanı: kullanım, nizam, birimli TAKS/KAKS/kat/Hmax/çekme, planda verilmeyen değerler, plan notları; plan notu;
yol aksı) ve `cad.zoning.check` (15 bulgu türü: boş ≠ 0 ≠ verilmeyen, karar çelişkisi, kararsız kısım, plan dışı,
eksik/anılmayan not, üst/alt plan kademesi; 28 ortak durum), pencere Seç ve göster ile sahnede. Açık: resmî şema ve
eşleme, plan değişikliği taslağı, üst plan sınırı kapsaması, yol devamlılığı, sembolün karardan seçilmesi.

### MAP-10 — Plan ve saha alternatiflerini ölçülebilir etkiyle karşılaştır [ ]

**Öncelik:** P1. **Kod dayanağı:** [K21], [K30], [K31], [K40].
**Araştırma:** ArcGIS Urban'da çalışma alanı/alternatifler mevcut durumdan
ayrılabiliyor; OpenSite ilişkili saha tasarımına odaklanıyor.
[Create a plan][S19], [OpenSite][S11]

- GIS'teki senaryo temelini kullanarak Harita tasarım alternatifinin
  sınırını tanımla: yalnız tasarım öğeleri ve referans aldığı mevcut durum
  revizyonu. GIS'in bütün zaman/senaryo arayüzü Harita'ya topluca kopyalanmasın.
- Donatı, kullanım alanı, nüfus/kapasite, yaya erişimi, arazi eğimi,
  altyapıya yakınlık, geçirimsizlik ve bilinen risk katmanları için kaynaklı
  karşılaştırma üret. Ölçüm birimi, analiz tarihi ve varsayımlar sonuçla taşınsın.
- Metraj/alan değişiminin hangi karar veya geometri değişikliğinden
  kaynaklandığı izlenebilsin. Ana veri değişince alternatif güncelliği
  bozulduğunu göstersin; otomatik olarak yeni veriye uymuş sayılmasın.
- Yerleşim/yükseklik zarfı gibi hafif 3B değerlendirmeleri [3d.md](3d.md)
  altyapısıyla ilişkilendir. Bu gösterim ruhsatlı BIM, mühendislik güvenlik
  analizi veya afet simülasyonu sonucu diye sunulmasın.
- Karşılaştırma tablo, harita ve raporda aynı sayıyı kullansın. Tek bir
  puanla “en iyi plan” seçmek yerine ölçütler, ağırlıklar, kısıtlar ve
  veri eksikleri anlaşılır olsun. AI önerisi mevcut duruma kendiliğinden uygulanmasın.

**CBS ortaklığı:** senaryo, analiz modeli, ağ/alan istatistikleri ve
rapor verisi GIS ile ortak. Harita'daki düzenlenebilir tasarım nesnesi
ile GIS'teki analitik çıktı sahipliği açık kalmalıdır.

**Kabul:** iki alternatifte bir yol/yeşil alan kararını değiştir; alan,
erişim ve arazi işi farklarını yeniden üret. Taban veriyi güncelleyip eski
sonucun neden geçersiz olduğunu göster. Yanlış birimli veya eksik risk
verisiyle kesin uygunluk sonucu verilmesin.

**6 Kasım 2026 — Alternatif etkisi ([ADR 1440](../docs/adr/1440-design-alternative-impact.md)).** Kapsam işareti:
**Harita + CBS**. Araştırma: ArcGIS Urban plan panosu (alan, FAR, türetilmiş ölçüler; senaryoları yan yana, tek puansız).
Kanıtlanan: alternatifin sınırı (senaryonun yerine geçen tasarım katmanları ve mevcut durum), birimli ve yöntemli ölçüler
(alan, uzunluk, inşaat alanı = alan × KAKS), kullanıma göre alanlar, anahtarla izlenen değişen kararlar, veri eksikleri
ve varsayımlar, taban damgasıyla eskime (`cad.scenarios.impact`, 21 ortak durum; kabuldeki yol/yeşil alan değişikliği).
Açık: damganın çizimde saklanması, erişim/eğim/altyapı/geçirimsizlik/risk ölçüleri, zarf ve harita gösterimi.

### MAP-11 — Arazi yönetimi işlemlerini sürümlü ve denetlenebilir taslaklarla ele al [ ]

**Öncelik:** P2; `GIS-06/GIS-07` gereği en son, sahibiyle ayrı kapsam.
**Kod:** [K19], [K21], [K40]. **Araştırma:** Netmap, Netkamu ve Nettop
geometriyle birlikte dağıtım/hak/tablo iş akışlarını ele alıyor.
[Netmap][S03], [Netkamu][S06], [Nettop][S07]

- Düzenleme, kamulaştırma/irtifak ve arazi toplulaştırması için ortak
  teknik çekirdek tasarla: giriş kayıtları, kapsam geometrisi, değer/hak
  ilişkisi, öneri senaryosu, denetim, itiraz/düzeltme ve teslim revizyonu.
  Bunları yalnız poligon kesme araçları olarak tanımlama.
- Sayı semantiği açık olsun: geometrik alan, kayıtlı alan, pay/payda,
  dağıtım değeri ve parasal değer farklı türlerdir. Paylarda rasyonel,
  parasal/dağıtım değerlerinde tanımlı decimal/rounding politikası kullan;
  UI'daki yuvarlanmış sayılar yeniden hesap girdisi olmasın.
- Kural paketi ülke/kurum, kaynak belge, yürürlük ve sürüm taşısın.
  Hiçbir kanuni oran/sınır kaynak koduna isimsiz sabit olarak gömülmesin.
  Güncel hukuk ve kurum süreçleri doğrulanmadan resmî uygunluk kararı verilmesin.
- Dağıtım veya öneri üretimi kısıtları, hedef fonksiyonu, çözümsüzlük
  nedenini ve hangi hak sahibinin nasıl etkilendiğini gösterebilsin.
  Sonuç tekrarlanabilir taslak olsun; kullanıcı incelemesi ve yetkili
  onayı olmadan kabul edilmiş kayıtların yerine geçmesin.
- Belge/kişisel veri yetkileri, sürüm karşılaştırma, toplam kontrolü ve
  sayısal artığın nereye dağıtıldığı raporda izlenebilsin. Kuruma gönderim,
  bu plan kapsamında otomatik dış işlem olarak tasarlanmasın.

**CBS ortaklığı:** geometri kapsamı, hak/kayıt, profil kuralları, senaryo,
inceleme ve kontrollü rapor erişimi GIS'te ortak kullanılacaktır. Resmî
işlemlerin açılması her iki program için aynı kurumsal kapıya bağlıdır.

**Kabul:** önce sentetik veriyle toplam alan/pay/değer korunumunu ve
kesin yuvarlamayı bağımsız hesapla doğrula. Daha sonra sahibiyle seçilecek
anonim gerçek örneği yetkili uzman/kurumla karşılaştır. Bu ikinci kabul
olmadan madde tamamlanamaz ve “resmî işlem hazır” denemez.

**6 Kasım 2026 — Düzenleme dağıtım taslağı ([ADR 1460](../docs/adr/1460-readjustment-distribution-draft.md)).** Kapsam
işareti: **Harita + CBS**. Araştırma: 3194/18 ve Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik (DOP %45'e kadar, oran
yedi basamak; ikincil kaynaklar), Netcad Netmap/Netkamu/Nettop. Kanıtlanan (kabulün ilk adımı): kural paketi veri olarak
(kurum, belge, yürürlük, sürüm, oran, sayı politikası; kodda oran yok), kesin pay ve ondalıkla kesinti/net/hak, artığın
kime gittiği, toplamın kesin korunumu; sonuç hep taslak (`cad.readjustment.distribute`, 30 ortak durum, Python fractions).
Açık: değer esaslı dağıtım, kamulaştırma/toplulaştırma, kapsam geometrisi, itiraz/sürüm, yetkiler ve ikinci kabul.

### MAP-12 — Ortofoto ve görüntü kaynaklarını kalite kontrollü üretime bağla [ ]

**Öncelik:** P1. **Kod dayanağı:** [K22]–[K24], [K04], [K40].
**Araştırma:** STAC veri varlığının zaman/kaynak/metadatasını, GeoParquet
analitik aktarımda koordinat anlamını taşır; bunlar doğruluk sertifikası
değildir. [STAC][S33], [GeoParquet 1.1][S34]

- Mevcut COG, STAC, GCP/oturtma ve yeniden örnekleme yollarını koru.
  Ortofoto üreticisi, uçuş/çekim zamanı, GSD, CRS/epok, düşey kaynak,
  denetim noktası doğruluğu ve işleme raporu kaynak kaydına bağlansın.
- Modern fotogrametri araçlarından gelen ortofoto/DSM/DTM ve kalite
  raporları için aktarım profili oluştur. Baştan tam bir fotogrametri
  motoru yazmak yerine önce kanıtlı çıktıyı doğru tüketme ve denetleme sağlansın.
- Otomatik görüntü eşleme/yeniden hizalama aday kontrol noktaları
  üretsin; uyum için kullanılan noktalarla bağımsız denetim noktaları
  ayrışsın. RMSE, yönlü hata ve kapsam dışı davranış görülsün; yalnız
  görüntü güzel çakıştı diye kaynak koordinatlar değiştirilmesin.
- CAD çizgisi ve semantik varlık çıkarımında görüntü çözünürlüğü,
  örtülme ve kaynak doğruluğu hesaba katılsın. Bir piksel seçimi milimetre
  hassasiyetli ölçü gibi sunulmasın; elle gözden geçirme kaynak bağıyla kaydedilsin.
- Uzak büyük rasterde kapsam/zaman filtresi, erişim süresi dolması,
  yerel çevrimdışı paket ve kaynak yenileme akışı tamamlanmış yeteneklerle
  bütünleşsin. Orijinal veriyi tekrar tekrar tam indirmeye dayalı bir UX kurulmasın.

**CBS ortaklığı:** kaynak keşfi/erişimi, kalite bandı, raster hazırlama,
maske, koordinat işlemi ve türetilen katman GIS'te de aynı altyapıyı kullanır.
Harita için ayrı raster okuyucu veya katalog kurulmaz.

**Kabul:** zamanları farklı iki ortofotoyu bağımsız kontrol noktalarıyla
karşılaştır; yanlış CRS, düşey veri ve erişilemeyen kaynak durumlarını dene.
Görüntüden türeyen nesne kaynağa izlenebilsin. İptal ve çevrimdışı açılış
orijinal dosyayı değiştirmesin; yanlış kalite sonucu üretmesin.

**6 Kasım 2026 — Ortofoto doğruluk denetimi ([ADR 1480](../docs/adr/1480-orthophoto-accuracy-check.md)).** Kapsam
işareti: **CAD + CBS** (raster bağlam sekmesi). Araştırma: BÖHHBÜY (RG 26.06.2018/30460 Mük.) m. 54, 61(2), 67(5)-(6),
91 resmî metinden; Pix4D kalite raporu (GCP ile bağımsız denetim noktası ayrımı); ASPRS Edition 2 (RMSE, en az 30
nokta). Kanıtlanan: uyum ve bağımsız denetim noktaları ayrı, KOH_X,Y/KOH_Z, ortalama (yönlü) ve en büyük hata, m. 91(2)
sınırları ve m. 54 YÖA kural paketiyle (veri), yarım piksel altı uyarısı, yanlış CRS reddi (`cad.orthophoto.check`,
27 ortak durum). Açık: iki zamanlı ortofoto karşılaştırması ve mozaik kayıklığı, otomatik eşleme, kalite raporu
aktarımı, türeyen nesnenin izi, DSM/DTM.

**7 Kasım 2026 — MAP-12.2: iki ortofotonun karşılaştırılması ([ADR 1780](../docs/adr/1780-two-orthophotos-compared.md)).**
Kapsam: **CAD + CBS** (raster bağlam sekmesi); komut Python ve MCP'de. Araştırma:
- BÖHHBÜY m. 67(6): mozaikteki ortofotolar arasındaki kayıklık ±2·YÖA;
- ASPRS 2014 ve 2024: dikiş hattı uyumsuzluğu sınıf RMSE'sinin iki katı (ikincil kaynak);
- ArcGIS Pro'nun denetim noktası sapması ve Metashape'in iki dönemi ortak işaretle hizalaması; ikisi de iki zamanı
  noktalarla karşılaştırmaz.

Kanıtlanan alt maddeler:
- `cad.orthophoto.compare` iki görüntüde okunan aynı noktaların kaymasını, sistematik kaymayı ve kaymaların KOH'unu verir.
- Nokta başına sınır kural paketinin verisidir (`mosaic` × iri YÖA); paket `mosaic` vermiyorsa ret.
- Arazi noktalarıyla her görüntü ayrıca denetlenir; yanlış CRS reddedilir.
- İki zamanın farkı deformasyon diye sunulmaz.
- Kabulden kanıtlanan: zamanları farklı iki ortofoto bağımsız kontrol noktalarıyla karşılaştırıldı.
- 29 bağımsız durum (`orthophoto_compare_cases.py`); masaüstü Raster sekmesi › Karşılaştır.

**Açık:** düşey veri (DSM/DTM) ve erişilemeyen kaynak durumları, görüntüden türeyen nesnenin kaynağa izi, otomatik
eşleme, dikiş hattının kendiliğinden ölçülmesi, kalite raporu aktarımı. Madde `[ ]` kalır.

### MAP-13 — LiDAR ve mobil haritalamayı ölçülebilir nesne üretimine taşı [ ]

**Öncelik:** P1; 3B sahne [3d.md](3d.md) ile ortak. **Kod:** [K25]–[K28], [K07].
**Araştırma:** COPC seçmeli erişim için hiyerarşi tanımlar; TBC kaydı,
sınıflamayı, çıkarımı ve gerçekleşen–tasarım farkını ayrı işler olarak
sunar. PDAL SMRF mevcut zemin süzgeci için açık başvurudur.
[COPC][S31], [TBC scanning][S14], [PDAL SMRF][S27]

- Mevcut LAS/LAZ/COPC, akan işlemler, zemin sınıflama ve kayıt sorgusunu
  kullan. Önce ihtiyaçlı yeni kapsamı ayır: kayıt/şerit uyumu, kesit/düzlem
  üzerinde düzenleme, kaynak bazlı sınıf düzeltmesi ve semantik çizgi/nesne çıkarımı.
- Kaydın kimliği dosya özeti + kalıcı kayıt konumundan üretilebilsin.
  LAS `Point Source ID` alanı tek başına bütün noktaların benzersiz kimliği
  sanılmasın. Kaynak sınıfı ile kullanıcının değişikliği ayrı tutulabilsin.
- Otomatik kayıt/ICP sonucunda kontrol noktası, örtüşme, artık,
  güvenilir kapsam ve dönüşüm tarifi raporlansın. SLAM/IMU yörüngesi,
  zaman eşleşmesi ve global kontrol bilinmiyorsa bulut “ölçüye kesin hazır” sayılmasın.
- Görsel LOD ve ölçüm/çıkarım ayrı yetenek olsun: kamera önizlemesi
  seyrek olabilir; kullanıcı nokta/mesafe/kesit sorguladığında tanımlı
  tam kaynak kapsamı okunsun. GPU önizleme sonucunun doğruluk sınırı açık olsun.
- Büyük bulutta görünür karo, öncelik, RAM/VRAM bütçesi ve iptal
  korunsun. Gaussian splatting veya foto-gerçekçi mesh eklenirse görsel
  temsil olarak bağlansın; fiziksel LiDAR/geometri ve kontrol doğruluğunun
  yerine ölçüm otoritesi yapılmasın.
- Kent ölçeğinde fotogrametrik mesh ve varlık bağlamı gerekiyorsa 3D
  Tiles 1.1 akışı [3d.md](3d.md) ile ortak değerlendirilsin. COPC'nin tam
  kayıt ölçümüyle 3B içerik akışının görevleri ayrı kalsın. [OGC 3D Tiles][S32]
- Yeni E57/diğer tarama kaynakları veya öğrenilmiş sınıflama gerekiyorsa
  ayrı sağlayıcı/eklenti yeteneği olarak değerlendirilip sözleşme, lisans,
  doğruluk ve performans kapısından geçsin; mevcut biçim desteği yeniden yazılmasın.

**CBS ortaklığı:** tam kayıt sorgusu, bölge/istatistik, sınıflama,
çıkarım sonucu ve kaynak/kalite bilgisi GIS'te de Processing olarak bulunur.
Yalnız Harita'ya özel dosya içinde saklanmış görünmez analiz sonucu oluşmaz.

**Kabul:** eğimli zemin, bordür, örtülmüş yol ve çoklu tarama içeren veriyle
işle. LOD değişirken ölçülen noktanın kimliği/değeri değişmesin. Kaynak
sınıfları korunsun, düzeltme geri alınsın; kayıt ve nesne çıkarımı bağımsız
kontrolle değerlendirilsin. Gerçek veri/süre/bellek raporu olmadan “milyar nokta hızlı” denmesin.

**6 Kasım 2026 — Tarama uyumu denetimi ([ADR 1500](../docs/adr/1500-scan-overlap-check.md)).** Kapsam işareti:
**Harita + CBS**. Araştırma: BÖHHBÜY LiDAR'a ölçüt vermiyor (yalnız m. 4/h tanım; m. 91(2) KOH_Z ürün için); USGS LBS
QL2 şeritler arası RMSDz ≤ 8 cm (örnek, bağlayıcı değil); TBC/Cyclone/TerraMatch'te kayıt ve şerit uyumu ayrı iş.
Kanıtlanan: iki taramanın tam çözünürlükte, hücre düzlemleriyle (eğim fark sayılmaz) RMSDz, sistematik kayma, en kötü
hücreler; GNSS kontrol noktalarında her taramanın KOH_Z'si; kontrolsüz bulut “ölçüye kesin hazır” sayılmaz
(`cad.pointclouds.overlap`, 26 ortak durum: eğimli zemin, kaymış şerit, yoldaki araç). Açık: ICP/kayıt, kalıcı kayıt
kimliği, sınıf düzeltme katmanı, kesit düzenleme, çıkarım, gerçek veri ölçümü.

**7 Kasım 2026 — MAP-13.2: tarama kaydı denetimi ([ADR 1800](../docs/adr/1800-scan-registration-check.md)).** Kapsam:
**Harita + CBS** (nokta bulutu bağlam sekmesi); komut Python ve MCP'de. Araştırma:
- CloudCompare ICP: durma ölçütleri, son RMS yalnız kullanılan noktalardan, 4×4 matris;
- Cyclone REGISTER 360: bulutlar arası hata, örtüşme, strength; yalnız onlara güvenilmemesi;
- Low 2004: nokta-düzlem ICP'nin doğrusal en küçük karesi.

Kanıtlanan alt maddeler:
- `cad.pointclouds.register` ikinci taramayı birinciye nokta-düzlem ICP ile oturtur ve raporlar: dönüşüm tarifi
  (merkez, ω φ κ, öteleme, 4×4 matris), önce/sonra RMS, örtüşme payı, geometrinin gücü, güvenilir kapsam hücreleri,
  kontrol noktalarında önce/sonra KOH_Z.
- Düz zeminde yatay kaymanın belirsizliği söylenir; kontrolsüz kayıt ölçüye kesin hazır sayılmaz.
- Bulut taşınmaz (salt okunur).
- Kabulden kanıtlanan: kayıt bağımsız kontrolle değerlendirildi (KOH_Z 5,0 → 0,5 cm); eğimli zemin ve çoklu tarama
  durumları.
- 31 bağımsız durum (`pointcloud_register_cases.py`); masaüstü Nokta bulutu sekmesi › Tarama kaydı.

**Açık:** dönüşümün buluta uygulanması, çoklu tarama demeti, kaba hatalı eşleşmeyi atan ağırlıklar, kalıcı kayıt
kimliği ve sınıf düzeltme katmanı, kesit düzenleme, çıkarım, gerçek veri ölçümü. Madde `[ ]` kalır.

### MAP-14 — Kırık hatlı ve kaynağı izlenen mühendislik yüzeyi geliştir [ ]

**Öncelik:** P1, MAP-02/03/07 üzerine. **Kod:** [K28]–[K30], [K03].
**Araştırma:** NetSurf araziyi üretim kaynağı olarak kullanıyor; Civil 3D
sınırın türü ve işlem sırasının etkisini açıklar. [NetSurf][S02],
[Surface Boundaries][S09]

- Mevcut Delaunay motoru üzerine kalıcı yüzey tarifi tasarla: kaynak
  nokta kümeleri, kabul edilmiş sınıflar, kırık hat, dış/iç sınır, maskeler,
  düzenleme adımları ve sürüm. Son üçgen listesi tek kaynak olmasın.
- Constrained Delaunay/kırık hat ekleme, çakışan kot, aynı XY farklı Z,
  çok yakın nokta, tekrarlı nokta ve geçersiz sınır politikasını açıklaştır.
  2.5B z=f(x,y) TIN ile düşey/üst üste yüzey içeren gerçek 3B mesh ayrı yetenek olsun.
- Nokta ekle/sil/kot değiştir, kenar değiştir, üçgen dışla, sınır/kırık
  hat düzenle ve yüzey birleştir işlemleri geçmişten yeniden üretilebilsin.
  Bir görsel gizleme kararı analizden veri çıkarma kararıyla karıştırılmasın.
- Kaynak değiştiğinde yalnız etkilenen bölge yeniden kurulabilsin;
  kaynakları eksik veya eski olan yüzey durumunu söylesin. Tüm bağlı
  kontur, profil, hacim ve pafta sonuçları aynı yüzey revizyonuna bağlansın.
- Mevcut DEM kontur/analizini kullan; TIN konturu gerekiyorsa analitik
  üçgen kesişiminden üret. Yumuşatma yalnız sunum türevi olsun; orijinal
  yüzeyi ve hesap kotunu değiştirmesin. Kontur sıklığı kalite belirsizliğini saklamasın.
- CPU doğruluk modeli ve GPU görüntü/hesap hızlandırması ayrışsın.
  Tile sınırında çatlak, komşu yüzeyde dikiş ve bellek sınırı gerçek
  büyük veriyle sınansın. Kod içindeki 400 milyon nokta sabiti performans kanıtı değildir.

**CBS ortaklığı:** yüzey kaynağı, sorgu, kontur, türev raster ve kalite
GIS'te de tüketilir. UGRID/NetCDF mesh okuyucusu korunur; düzenlenebilir
mühendislik yüzeyiyle aynı sözleşmeymiş gibi genişletilmez.

**6 Kasım 2026 — Harita projesinde arazi yüzeyi ([ADR 1520](../docs/adr/1520-map-project-terrain-surfaces.md)).**
Kapsam işareti: **Harita + Yol** (CBS yüzeyi analiz sekmesinden tüketir). Araştırma: BÖHHBÜY m. 4(1)(l) SAM'ı üçgensel
biçimde tanımlar, m. 45(1)(c) ≥ 25 nokta/ha, m. 63 ve m. 78(1)(c–g) eş yükseklik eğrisi (aralık ölçeğe bağlı, her
beşinci kalın); NetSURF yüzeyi harita üretiminin kaynağı yapar; Civil 3D yüzeyi tanım listesidir. Kanıtlanan: yol'un
yüzey tarifi ve bütün `road.terrains.*` komutları Harita projesinde çalışır, Arazi sekmesinin Yüzey/Analiz/İşlemler
panelleri Harita'da açılır (Tesviye, Drenaj Yol'da kalır); `.kcad` şema 195 harita projesinin yalnız yüzeylerini
yazar; eğri aralığı m. 78(1)(c)'ye göre ölçekten başlar. Açık: düzenleme adımları ve geçmişten yeniden üretim, aynı
XY/farklı Z raporu, bölgesel yeniden kurma ve revizyon bağı, m. 78(1)(e–g) kartografik kurallar, gerçek veri ölçümü.

**7 Kasım 2026 — Yüzey düzenlemeleri ([ADR 1560](../docs/adr/1560-terrain-surface-edits.md)).** Kapsam: **Harita +
Yol**. Araştırma: Civil 3D yüzey tanımının Edits listesi (Add Point, Modify Point, Add Line, Swap Edge…), BricsCAD TIN
tanımları; BÖHHBÜY m. 63(1) morfoloji, m. 45(1)(c) karakteristik noktalar. Kanıtlanan: tarifin sıralı `edits`'i (nokta
ekle, kot değiştir, kenar ekle/çevir, üçgen sil) her kuruluşta kaynaklardan sonra uygulanır; yapılamayan silinmez,
nedeniyle söylenir; `.kcad` şema 196; 17+1 bağımsız (Triangle) durum; masaüstünde çizimden seçimli dört araç, silinen
üçgen çizimde görünür. Açık: sınır/kırık hattın pencereden düzenlenmesi, yüzey birleştir, bölgesel yeniden kurma ve
revizyon bağı, düzenlemeyi kapatma.

**7 Kasım 2026 — MAP-14.2: pafta eş yükselti eğrileri ([ADR 1820](../docs/adr/1820-sheet-contours-by-article-78.md)).**
Kapsam: **Harita + Yol**; komut Python ve MCP'de. Araştırma: BÖHHBÜY 2018 m. 78(1)(c)–(g) ve Ek-3 0001–0005 resmî
metinden (aralık, 0,25/0,13 mm, 2 ve 20 mm eşikleri, 200 mm'de 1,4 mm kot yazısı, kesik yardımcı eğri, detay kesmeme).

Kanıtlanan alt maddeler:
- `road.terrains.sheetcontours` eğrileri paftaya m. 78'e göre çizer: dik yerde yalnız kalın, düz yerde yarım aralıkta
  yardımcı eğri (üçgen başına), kalın eğrilere maskeli kot yazısı (üstü yokuşa), detay katmanlarının kapalı nesneleri
  kesilir; değerler kural paketinin verisi.
- Görsel gizleme veri çıkarmaz: yazının boşluğu maskedir, eğri bölünmez; yüzey ve hesap kotu değişmez.
- Çekirdeğin seviye izlemesine üçgen süzgeci eklendi; mevcut eğriler bit bit aynı.
- 27 bağımsız durum (`road_sheet_contour_cases.py`, Triangle); masaüstü Arazi › Yüzey › Pafta eğrileri.

**Açık:** yardımcı eğrinin 5/1 mm deseni, çift çizgili detayın kenar çizgilerinden tanınması, yazı genişliğine göre
gerçek boşluk, yeniden çizimde tutulması, m. 78(1)(ğ) karakteristik kotlar; düzenleme adımlarının kapatılması, yüzey
birleştirme, bölgesel yeniden kurma. Madde `[ ]` kalır.

**Kabul:** bordür kırık hattı, gölet deliği, şev, aynı XY/farklı Z ve
kaynak nokta revizyonu içeren yüzey kur. Kırık hat korunumu, sınır dışı
üçgenler, kot sorgusu ve kontur bağımsız başvuruyla doğrulansın. Save–reopen
ve tariften yeniden kurma aynı sonucu versin; iptalde eski geçerli yüzey kalsın.

### MAP-15 — Saha kotlandırması, profil ve kazı/dolguyu aynı tasarıma bağla [ ]

**Öncelik:** P1; tam yol/koridor için [road.md](road.md) esas. **Kod:** [K30]–[K32], [K40].
**Araştırma:** OpenSite ilişkili saha/tesviye tasarımı, Netpro yol ve
kesit/metraj akışı sunuyor. Bunların bütünü bir polylineda M değeri
tutmakla elde edilmez. [OpenSite][S11], [Netpro][S05]

- Meydan, otopark, park, platform, şev ve basit bağlantı rampası için
  hedef kot/eğim, referans kenar/yüzey ve gün ışığına çıkış gibi ilişkili
  tasarım öğeleri oluştur. Düşey değer yalnız serbest öznitelik olmasın.
- Mevcut LRS/istasyon ve mesh profilini tekrar kullan. Bir profil;
  güzergâh ve yüzey revizyonunu, istasyon tanımını, düşey abartıyı ve kot
  referansını bilsin. Boy/en kesit tek seferlik çizgi çıktısından canlı türeve taşınsın.
- Tasarım ile mevcut yüzeyin karşılaştırmasını tanımlı ortak kapsamda
  yap. TIN–TIN, raster veya kesit yöntemi seçildiyse yöntem, çözünürlük,
  eksik alan ve sayısal hata sınırı raporda bulunsun; farklı sonuçlar
  açıklamasız aynı “hacim” başlığı altında sunulmasın.
- Kazı/dolgu sınırı, sıfır hattı, malzeme bölgesi, kabarma/sıkışma
  katsayısı ve stok sahası ayrı parametre olsun. Geometrik hacim ile
  iş/malzeme miktarı ve maliyet farklı hesaplar olarak tutulabilsin.
- Akış yönü/çukur/su birikme denetimi yüzey kalitesini ve çıkış
  sınırını dikkate alsın. Basit eğim analizi hidrolik taşkın simülasyonu
  diye sunulmasın; ilgili çözücü/afet işi [3d.md](3d.md) ile bağlansın.
- Yol yatay/düşey güzergâhı, dever, şablon ve koridor için paralel
  motor yazma. Ortak yüzey ve tasarım bağı üzerinden road planının
  yeteneklerini Harita projesinde kullanılabilir hale getir.

**CBS ortaklığı:** yüzey/profil sorgusu, kazı-dolgu bölgeleri, miktar
tablosu, kaynak ve kapsam GIS'te analiz aracı/katmanı olarak kullanılmalı.
GIS'in tematik fark haritası aynı hesap sonucunu göstermeli.

**Kabul:** bir platformun kotunu değiştir; şev, kesit, hacim ve rapor
birlikte güncellensin. Delikli/örtüşmeyen yüzeylerde kapsam ve dışlanan
bölge açıklansın. Basit prizmatik örnekte analitik hacim; gerçek örnekte
bağımsız yöntem karşılaştırması yap. İptal ve Undo bütün türevleri kapsasın.

**7 Kasım 2026 — Harita projesinde tesviye ve drenaj ([ADR 1580](../docs/adr/1580-map-project-grading-and-drainage.md)).**
Kapsam: **Harita + Yol**. Araştırma: OpenSite saha tasarımını yoldan ayrı sunar (arazi, kazı/dolgu, drenaj); Civil 3D
grading koridordan bağımsız; Planlı Alanlar İmar Yönetmeliği tabii/tesviye edilmiş zemin. Kanıtlanan: platform (taban
kotu, eğim/yön, şevler, gün ışığı, dengeleme) ve drenaj (gölet, damla, havza, debi) Harita projesinde; `.kcad` şema 197
harita projesinin platformlarını yazar; Arazi sekmesinin Tesviye ve Drenaj panelleri Harita'da. Açık: hacim yöntemi ve
hata sınırı raporu, kabarma/sıkışma ve malzeme, stok sahası, canlı kesit türevi.

**7 Kasım 2026 — Hacim yöntemi ve katsayılar ([ADR 1600](../docs/adr/1600-surface-volume-method-and-factors.md)).**
Kapsam: **Harita + Yol** (CBS aynı hesabı tüketir). Araştırma: Civil 3D Volumes Dashboard (kazı/dolgu katsayısı,
düzeltilmiş ve düzeltilmemiş hacim ayrı); KTŞ 2013'te hacim dönüşüm katsayısı yok, değer uydurulmadı. Kanıtlanan:
`road.terrains.compare` `method` (tin | grid+cell), ızgarada kesin TIN–TIN hacmi yanında (yöntem farkı açık),
`cutFactor`/`fillFactor` ile ayrı `adjusted`; 15 bağımsız durum. Açık: kesit yöntemi, malzeme bölgesi, stok, sıfır
hattının çizimi, tematik fark haritası.

**7 Kasım 2026 — Sıfır hattı ([ADR 1620](../docs/adr/1620-cut-fill-zero-line.md)).** Kapsam: **Harita + Yol**.
Araştırma: Civil 3D hacim yüzeyinin sıfır eğrisi, OpenSite'ın kazı/dolgu ayrımı. Kanıtlanan: `road.terrains.zeroline`
iki yüzeyin farkının sıfır olduğu çizgileri tabanın kotuyla 3B çoklu çizgi yazar (tek adım “Sıfır hattı”), çakışan alanı
söyler; 23 bağımsız durum; Fark'ta Sıfır hattını çiz. Açık: kazı/dolgu bölgelerinin alan olarak yazılması, canlı
güncelleme.

**7 Kasım 2026, ikinci tur — Kazı ve dolgu bölgeleri ([ADR 1840](../docs/adr/1840-cut-and-fill-zones.md)).** Kapsam:
**Harita + Yol** (CBS aynı sayıları öznitelikten okur). Araştırma: ArcGIS Surface Difference (Volume/SArea/Code
çokgenleri, aynı sınıfın bitişik parçaları tek çokgen), Civil 3D Volumes Dashboard (datumla bantlama), OpenRoads
Create Cut Fill Volumes. Kanıtlanan: `road.terrains.cutfill` farkın bölgelerini delikli alan olarak yazar (bantsız
kazı/dolgu ya da `interval` bantları), alan ve hacim öznitelikte, bölgelerin hacmi `road.terrains.compare`'in kesin
hacmi; çakışık alan bölge olmaz; çizim iki yüzeyin damgasıyla tutulur (yüzey değişince eskir, yeniden üretilir),
`.kcad` şema 205; 33 bağımsız durum (shapely ile birleştirme, proje koordinatlarında da); Fark'ta Kazı ve dolgu
bölgelerini çiz. Açık: malzeme bölgesi, stok sahası, bölge yazısı ve miktar tablosu, kesit yöntemi.

### MAP-16 — Akıllı altyapı tasarımı ile ağ analizini birleştir [ ]

**Öncelik:** P1 teknik envanter/tasarım; P2 uzman çözücüler. **Kod:** [K31], [K32], [K40].
**Araştırma:** EPANET basınçlı su, SWMM drenaj/hidroloji hesapları için
farklı motorlardır. Mevcut grafik ağında izleme yapmak bu çözümleri
sağlamaz. [EPANET][S29], [SWMM][S30]

- Su/atıksu/yağmursuyu ve uygun enerji/telekom varlıkları için tipli
  hat/düğüm, sabit varlık kimliği, kot/derinlik, çap/malzeme, işletme
  durumu ve bağlantı portu modeli tasarla. Branşman ve servis bağlantısı
  çizim çizgisinden ve analiz kenarından ayırt edilsin.
- Hat profili; zemin yüzeyi, boru taban/eksen kotu, eğim, kapak/akar
  kotu ve örtü koşullarını birlikte göstersin. Plan veya kot değişince
  profil ve çakışma denetimi güncellensin.
- Mevcut ağ/tracing/kapatma/servis alanı yeteneklerini koru. Geometrik
  kesişim ile fiziksel bağlantı aynı sayılmasın; farklı seviyedeki
  geçiş, vana/anahtar durumu ve port bağlantısı anlamlı olsun.
- Hidrolik/hidrolojik analiz isteğe bağlı adaptör/Processing aracı
  olarak gelsin. Girdi eşleme, birimler, sınır koşulları, çözücü sürümü,
  yakınsamama, sonuç zamanı ve kalibrasyon bilgisi kayıtlı olsun.
  Sonuç katmanı üretmeden önce hata/girdi raporu görülsün.
- Diğer disiplinlerle yatay/düşey açıklık ve yüzey kaplaması denetlensin.
  Tasarlanan, uygulanan ve ölçülmüş durum ayrı revizyonlardır; tüm
  katmanları birleştirip işletme verisini taslakla değiştirme.

**CBS ortaklığı:** varlık modeli, portlar, bağlantı, işletme durumu,
analiz girdisi/sonucu ve bakım formu GIS'te de kullanılmalı. Harita profil/
hassas tasarım; GIS işletme ve tematik inceleme yüzeyini sağlar.

**Kabul:** küçük drenaj ve basınçlı su örneklerini ayrı çalıştır;
bağlantısız hat, ters eğim, yetersiz örtü ve yanlış birimi yakala.
Girdi/sonucu motorun bağımsız örneğiyle doğrula. Yakınsamayan analiz
başarılı diye sahneye yazılmasın; iptal UI'ı ve ana veriyi kilitlemesin.

**7 Kasım 2026 — Boru hattı denetimi ([ADR 1640](../docs/adr/1640-gravity-pipe-check.md)).** Kapsam: **Harita + Yol +
CBS**. Araştırma: Civil 3D'nin Cover and Slope ve Pipe Drop Across Structure kuralları; yönetmeliğin sayısal sınırları
(Atıksu Toplama ve Uzaklaştırma Sistemleri Hakkında Yönetmelik EK-1) bakanlık sunucusunun sertifikası doğrulanamadığı
için okunamadı, İller Bankası şartnamesine de erişilemedi: sınırlar kullanıcınındır, KentOS değer koymaz. Kanıtlanan:
`cad.pipes.check` bir altyapı ağının borularını cazibeli hat olarak denetler: bağlantısız parça, bacasız uç, ters eğim,
eğimsiz, çıkışı girişten yüksek baca, örtü (yüzeyden her TIN kenarı kesişiminde ya da kapak kotundan), cm/mm ya da
derinlik yazılmış kot, başka birimde yazılmış çap; 30 bağımsız durum; Boru hattı denetimi penceresi. Açık: basınçlı su
(EPANET) ve SWMM adaptörleri, hat profili, disiplinler arası açıklık, tasarlanan/uygulanan/ölçülen revizyonlar, EK-1
değerlerinin kural paketi.

**7 Kasım 2026, ikinci tur — Hat profili ([ADR 1860](../docs/adr/1860-pipe-run-profile.md)).** Kapsam: **Harita + Yol +
CBS**. Araştırma: Türk kanalizasyon boykesiti (çap, eğim, boy, akar, zemin, derinlik; düşey ölçek yatayın 10 katı),
Civil 3D'nin Pipe Data bandı; İller Bankası şartnamesine yine erişilemedi. Kanıtlanan: `cad.pipes.profile` bir borudan
akış yönünde hattı izler (bitiş borusu, çıkış ya da çatal), değerleri boru hattı denetiminin kurallarıyla okur ve
boykesiti yol boykesitinin kurallarıyla çizer: çerçeve ve kot ızgarası, zemin, akar ve tepe, bacalar, dokuz bant; 35
bağımsız durum (çizilen nesneler sayılır); Boru hattı denetimi penceresinde Hat profilini çiz. Açık: profilin canlı
güncellenmesi, disiplinler arası açıklık, EPANET/SWMM adaptörleri, tasarlanan/uygulanan/ölçülen revizyonlar.

### MAP-17 — Modern ve kayıp raporlu CAD/CBS veri devri sağla [ ]

**Öncelik:** P0 mevcut aktarımı koruma; P1 mühendislik profilleri.
**Kod:** [K33]–[K35], [K09], [K23]. **Araştırma:** InfraGML ölçüm/ekipman,
arazi bölümü ve mühendislik verisi için parçalı model sunar; her altyapı
tipini kapsayan tek format değildir. [InfraGML][S28], [GeoParquet][S34]

- Mevcut CAD açma, birim/CRS eşleme, xref, GIS türetme ve geri alma
  yollarını koru. Tam/koşullu destek matrisi nesne ve özellik düzeyinde
  olsun: eğri, Z/M, metin/blok, stil, ilişki, kimlik, kalite ve kaynak.
- DWG/DXF/NCZ mevcut desteklerinin sınırlarını raporla; güncel üretim
  dosyalarını aksatan sorunları [cad.md](cad.md) ile tek iş listesinde
  gider. Tarihsel cihaz biçimlerini yeni ürün hedefi yapma. NCZ dağıtım
  kapısını koru. Bir okuyucunun bulunması aynı biçimde kusursuz
  yazıcı/round-trip garantisi sayılmasın.
- Yeni ihtiyaçlarda önce GeoPackage/GeoParquet, COG/COPC, sürümlü
  GML/profil ve mühendislik model değişimini değerlendir. LandXML gibi
  sahada yaşayan değişim biçimi gerekiyorsa sınırlı dönüştürücü olsun;
  KentOS'un iç veri modelini eski biçime göre şekillendirme.
- InfraGML/IFC gibi adaylar için tam standart desteği vaat etmek yerine
  gereken alt modeli ve işveren/kurum profilini seç. Dönüşen, korunan,
  yaklaşıklaştırılan ve kaybolan bilgi her aktarımda açıklansın.
- Kaynak–türev eşlemesi ve iki yönlü değişiklik çatışması mevcut
  digest/revizyon denetiminden geçsin. Eğrinin kirişleşmesi, yükseklik
  kaybı veya ilişki kopması açık kullanıcı kararı olmadan uygulanmasın.
- Teslim paketi çizim + bağlı kaynaklar + kullanılan profil/ızgara
  bilgisi + kayıp/kalite raporu taşısın. Gizli anahtar, imzalı URL ve
  yetkisiz kişisel veri pakete kopyalanmasın.

**CBS ortaklığı:** ortak kaynak/kimlik, açık format sağlayıcıları,
profil eşleme ve kayıp raporu zorunludur. Bu iş CAD↔GIS doğrudan mod
değişimini yeniden getirmez; kontrollü veri devrini güçlendirir.

**Kabul:** yay/Z/M/blok/ilişki içeren gerçek koordinatlı veri için
KentOS→dış biçim→KentOS karşılaştırması yap. Sayısal hata ve semantik
kayıplar ayrı raporlansın. Hem kaynak hem türev değiştiğinde geri alma
çatışma versin; kaybolan dış kaynakta proje açılabilsin.

**7 Kasım 2026 — Dışa aktarma kaybı denetimi ([ADR 1660](../docs/adr/1660-exchange-round-trip-check.md)).** Kapsam: **CAD + CBS**
(her modül). Kanıtlanan: `file.exchange.check` çizimi GeoPackage, GeoParquet, FlatGeobuf, GML, Shapefile ya da DXF'e
KentOS'un yazıcısıyla yazıp okuyucusuyla geri okur; sayısal sapma (köşe kayması, kot, ölçü, uzunluk, alan) ile anlamsal
kayıplar (yazılmayan, tür, kiriş, Z/M, öznitelik, stil, kimlik, fazla) ayrı raporlanır. Altı biçim GDAL `ogrinfo` ve
ezdxf ile bağımsız sınandı. Denetim GeoParquet yazıcısında büyük koordinatta küçük dairenin 0,41 mm kayan kirişlerini
buldu, düzeltildi. Açık: blok, ilişki ve tablo nesneleri, teslim paketinin kayıp raporu, kaynak–türev çatışması, LandXML/IFC
alt modelleri.

**7 Kasım 2026, ikinci tur — Destek matrisi ([ADR 1880](../docs/adr/1880-exchange-support-matrix.md)).** Kapsam: **CAD +
CBS** (her modül). Kanıtlanan: `file.exchange.matrix` açık çizimi altı biçimin her biriyle file.exchange.check'in
gidiş-dönüşünden geçirir ve on satırda (nesneler, tür, eğriler, Z, M, öznitelikler, stil, kimlik, köşe konumu, fazla)
korunan, yaklaşıklaşan, dönüşen, eklenen ve kaybolanı verir; örnek çizimin matrisi ADR 1660'ın tablosudur; Python'da
her hücre kuralıyla ayrıca sınanır, GDAL testleri aynen geçer; pencerede Tüm biçimler. Açık: blok/ilişki/tablo örnekleri,
teslim paketinin matrisi taşıması, kurum profilleri.

### MAP-18 — Harita anotasyonu, pafta ve teslimi tek revizyondan üret [ ]

**Öncelik:** P0 mevcut okunurluk; P1 üretim bağları. **Kod:** [K20], [K36]–[K39].
**Araştırma:** Netcad/NetSurf üretim akışı ve açık veri yaklaşımının sonucu,
hesap–harita–belgenin aynı kaynağı göstermesidir. Bu KentOS için
tasarım kararıdır. [Netcad GIS][S01], [NetSurf][S02]

- Mevcut pafta/atlas/GeoPDF/ön denetimi yeniden yazma. Yüzey, parsel,
  plan, ölçüm kalite kaydı, profil ve altyapıdan gelen anotasyon/tabloları
  kalıcı kaynak kimliği ve revizyonuna bağla.
- Kot, nokta/varlık adı, eğri kotu, kenar/alan değeri, istasyon ve
  plan koşulunu uygun görünüş tarifiyle üret. CAD gerçek boylu metni ve
  GIS dinamik etiketini ayır; zoom'da kaybolup gelen ölçü veya çift ölçek UX'i yaratma.
- Harita grid'i, gerekiyorsa ikinci koordinat sistemi, kuzey türü,
  ölçek çubuğu, datum/epok ve kalite notları aynı proje bilgisinden gelsin.
  UI'daki eksen adları ile dış kurumun teslim adlandırması açıkça eşlensin.
- Mühendislik nesnesi güncellenince etkilenen pafta/tablolar listelensin.
  Son yayındaki kaynak revizyonu ile güncel model farkı görülsün;
  eski paftaya yeni tarih basarak güncelmiş gibi yayınlama.
- Resmî profil çıktısı kapısı genel pafta üretiminden ayrı olsun.
  Eksik/erişilemeyen kaynak, güncelliğini yitirmiş analiz, okunmayan anotasyon
  ve kayıp font/stil teslim öncesi bulguya dönüşsün.
- Büyük pafta seti iptal edilebilir arka plan işi olarak hazırlansın;
  yalnız değişen sayfaların yeniden üretimi ve teslim paketi manifesti
  mevcut atlas/job altyapısına bağlansın.

**CBS ortaklığı:** şablon, atlas, grid, rapor verisi, kaynak revizyonu ve
ön denetim GIS'te de aynı sistemdir. Harita ve GIS aynı kabul edilmiş
veri için farklı alan/metraj değeri basmamalıdır.

**Kabul:** 1:500/1:1000/1:5000 görünüşlerde aynı veriyle çıktı üret;
zoom boyunca yazı/ölçü, grid ve çizgi konumu gözle denetlensin. Bir
yüzey/parsel revizyonu yalnız ilgili sayfaları güncellesin. PDF'deki
koordinat/ölçü bağımsız okuyucuda sınansın; resmî profil onaysızken genel çıktı çalışabilsin.

**6 Kasım 2026, ilk dilim (ADR 1320, dal `map-18-pafta-teslim`):** her yayın `yayin.json`'a sayfaların kaynak
dökümünü yazar (görünüm penceresinin bastığı, tablosunun okuduğu nesneler kalıcı kimlikleriyle; her nesnenin yuva
kimliği olmadan içerik özeti; pafta kaydının ve katman/stil/blok/ayar/kitap parçalarının özetleri).
`file.publish.status` her sayfanın teslim tarihini ve kaynak değişimini verir (değişen, giren, çıkan nesneler; pafta
düzeni; değişen parçalar). `file.publish.sheets {previous}` önceki teslimin güncellemesini yeni klasöre kurar: sayfası
aynı çizen ve PDF'i sağlam pafta baytı baytına ve kendi tarihiyle kopyalanır, değişenler bugünün tarihiyle yeniden
üretilir, revizyonu ilerletilmemiş değişen paftaya `revision_unchanged` uyarısı; önceki teslim değişmez. Masaüstü
yayın penceresinde Durum satırları ve Güncelle. Kanıt: 25 bağımsız ortak durum (`sheet_update_cases.py`); aynı 60
parsel 1:500/1:1000/1:5000'de, 20 m kenar 40/20/4 mm, çerçeve köşeleri GDAL'ın GeoPDF okumasında 1 mm içinde, bir
parselin değişimi yalnız onu gösteren sayfaları yeniden üretiyor. **Kalan:** atlas sayfalarının kaynak dökümü, tek
PDF ve devam ettirilebilir güncelleme; anotasyon/tabloların kaynak kimliği ve revizyonuna bağlanması, CAD gerçek boylu
yazı ile GIS dinamik etiketin ayrılması, datum/epok/kalite notları ve kurum eksen adları, resmî profil kapısı, eksik
kaynak/okunmayan anotasyon bulguları. Madde açık kalır.

**7 Kasım 2026 — Paftada koordinat bilgisi ([ADR 1680](../docs/adr/1680-sheet-coordinate-facts.md)).** Kapsam: **CAD + CBS**
(pafta). Kanıtlanan: pafta yazıları `@datum`, `@elipsoit`, `@projeksiyon`, `@kot_sistemi`, `@epok`, `@ikinci_sistem` ve
`@epsg_kodu`yu projenin kendi bilgisinden okur (EPSG kaydı ve projenin kot/epok ayarı; ayrılmış ad değil, projenin
değişkeni önce gelir); baskı öncesi denetim projenin sisteminden başka EPSG kodu ya da datum yazan yazıyı
`crs_text_mismatch` ile bulur, dönüşüm notunu bulmaz. Açık: kurum eksen adları, kuzey türü notu, ölçme kalite notu,
resmî profil kapısı.

**7 Kasım 2026, ikinci tur — Paftada kuzey bilgisi ([ADR 1900](../docs/adr/1900-sheet-north-facts.md)).** Kapsam: **CAD +
CBS** (pafta). Kanıtlanan: pafta yazıları `@yakinsama` (GK–CK), `@manyetik_sapma` (CK–MK, WMM2025, paftanın tarihinde)
ve `@grid_manyetik` (GK–MK) okur; bağlı haritanın ya da ilk haritanın merkezinde, kuzey okunun hesabıyla; değerler
PROJ (pyproj) ve pygeomag'ın WMM2025'iyle bağımsız sınandı. Açık: kurum eksen adları, ölçme kalite notu, resmî profil
kapısı.

### MAP-19 — Modern saha uygulaması ve gerçekleşen durum kontrolünü tamamla [ ]

**Öncelik:** P1 çevrimdışı karşılaştırma; canlı cihaz işi donanım kapısında.
**Kod:** [K09]–[K12], [K30]–[K32], [K40]. **Araştırma:** TBC ve 12d
gerçekleşen veriyi tasarımla karşılaştıran akışları vurguluyor.
[TBC scanning][S14], [12d Survey][S16]

- Tasarımdan sahaya çıkacak paketi açık tanımla: seçilen revizyon,
  nokta/hat/yüzey/varlık kimlikleri, CRS/epok, kot referansı, kabul
  eşiği ve gerekli kalite notları. Sadece isimsiz koordinat listesi olmasın.
- Alıcı/robotik cihaz/makine kontrolü adaptörleri opsiyonel olsun;
  güncel cihaz SDK/protokolü, lisans ve gerçek donanım testine göre seçilsin.
  Kontrolsüz otomatik dış cihaz komutu uygulama akışının varsayılanı olmasın.
- Gerçekleşen ölçüm geldiğinde tasarım kimliğiyle veya önerilen
  mekânsal eşlemeyle karşılaştır. Yatay/düşey/normal fark, kapsam,
  ölçüm belirsizliği ve seçilmiş kabul profilini raporla; en yakın nokta
  her zaman doğru varlık değildir.
- As-built, aplikasyon kontrolü ve dönemsel deformasyon ölçümlerini
  ayrı iş türleri yap. Tasarım değişikliği ile fiziksel hareketi
  ayırt et; zaman serisinde datum/epok ve ortak kontrolü koru.
- Sahadan gelen düzeltme ana tasarıma doğrudan yazılmasın. Yetkili
  inceleme, uyuşmazlık ve kabul edilen gerçekleşen revizyon kaydı olsun.
  Gelecekte mobil istemci aynı sözleşmeyi tüketebilsin; mevcut web/mobil ertelemesi korunsun.

**CBS ortaklığı:** varlık eşleme, gerçekleşen katmanı, fark/kalite,
yetkili inceleme ve zaman serisi GIS'te de kullanılmalı. Ölçüm cihazı
bağlantısı için ikinci bir GIS uygulaması/motoru yazılmaz.

**Kabul:** tasarım yüzeyi ve altyapıya iki tarihte ölçüm getir; eksik
varlık, yanlış eşleme, kot farkı ve düşük kaliteli çözümü ayır. Eski
tasarım revizyonuyla ölçülen kayıt daha sonra yeni tasarıma sessiz
bağlanmasın. Canlı kabul gerçek cihazla, çevrimdışı kabul kayıtlı veriyle ayrı işaretlensin.

**7 Kasım 2026 — Uygulama kontrolü ([ADR 1700](../docs/adr/1700-asbuilt-check.md)).** Kapsam: **Harita + Yol + CBS**.
Araştırma: Trimble Business Center'ın Point/String Conformance raporları; Türk mevzuatında aplikasyon kontrolü için
sayısal sınır bulunamadı, sınırlar kullanıcının. Kanıtlanan: `cad.asbuilt.check` ölçülen noktaları tasarım noktalarına
(adla; yarıçapla öneri, iki aday belirsiz), tasarım yüzeyine ve tasarım çizgilerine (işaretli dik uzaklık, yalnız çizgi
noktaları) karşı denetler; ölçülmeyen tasarım noktaları, σ'sı yüksek noktalar ayrı; tasarım özeti eski tasarımla
ölçülen kaydın yeni tasarıma sessiz bağlanmasını önler; 26 bağımsız durum; Uygulama kontrolü penceresi (Ölçme › Saha).
Açık: saha paketi, iş türleri (as-built/aplikasyon/deformasyon) ve zaman serisi, yetkili inceleme kaydı, kapsam
boşlukları, canlı cihaz.

**7 Kasım 2026, ikinci tur — Saha paketi ([ADR 1920](../docs/adr/1920-field-package.md)).** Kapsam: **Harita + Yol +
CBS**. Araştırma: Trimble Access'in işi (sistem, birimler, bağlı dosyalar, notlar) ve aplikasyon farkları, LandXML 1.2
MVD başlığı (EPSG, düşey sistem; epok alanı yok), 12d Field'in denetim izi, Leica Captivate'in tasarım alımı; hiçbiri
dönen kaydın denetleyeceği bir tasarım özeti taşımıyor; TKGM 2023/5'in aplikasyon maddesine erişilemedi. Kanıtlanan:
`cad.field.package` tasarımı (noktalar, çizgiler, yüzey) tek `.zip`'e yazar: `saha-paketi.json` (revizyon, Uygulama
kontrolünün tasarım özeti, EPSG ve ad, epok, kot sistemi, yalnız verilen kabul sınırları, kalite notları, yüzeyin
kaynak özeti), `noktalar.csv` (Ad;Y;X;Z;Kimlik, yuvarlanmadan) ve `cizgiler.csv`; sahadan dönen kayıt bu özetle
`expectedDesign` olarak denetlenir; 29 bağımsız durum, Python'un `zipfile`/`csv` okuyucusuyla geri okuma; Uygulama
kontrolü penceresinde Saha paketi bölümü (kalite notları, Saha paketi yaz…). Açık: iş türleri ve zaman serisi, dönen
kaydın paketle otomatik eşlenmesi, yetkili inceleme kaydı, altyapı ve yol varlıklarının kimlikleri, cihaz biçimleri,
canlı cihaz.

### MAP-20 — Bütün iş akışlarını yeniden üretilebilir, durdurulabilir ve otomasyona açık teslim et [ ]

**Öncelik:** P0, diğer 19 maddeye yayılır. **Kod:** [K40]–[K45], [K42].
**Araştırma:** QGIS/GRASS kaynaklarında ortak hesap çağrısı ve açık işlem
tarifi; STAC/GeoParquet'te anlamlı veri sözleşmesi görülebiliyor. Bunların
üstüne aşağıdaki teslim ilkesi KentOS için önerilir. [S21], [S23], [S33], [S34]

- Kaynak→dönüşüm→kabul edilmiş ölçüm→yüzey→tasarım→analiz→pafta için
  yeniden üretilebilir işlem grafiği tasarla. Girdi kimlik/özetleri,
  işlem sürümü, birim/CRS/epok, parametreler ve çıktı revizyonu kaydedilsin.
  Mevcut Processing modelini ve job önbelleğini bunun temeli olarak kullan.
- Her ağır işlem UI iş parçacığından ayrı yürüsün; ilerleme, bellek/
  CPU/VRAM bütçesi ve iptal kanalı olsun. Durumlar “bekliyor / çalışıyor /
  iptal ediliyor / iptal / başarısız / tamamlandı” diye ayrışsın. Eski
  kaynakla veya kaybedilen yetkiyle sonuç yazımı revizyon/yetki kapısından geçmesin.
- Her yeni iş GUI, komut, Python, MCP ve uygun olduğunda Processing
  üzerinden aynı hesap/işleyiciye erişsin. `validate`/`plan` yazmasın;
  `execute` atomik uygulansın. Analiz aracı için yalnız sahne tıklama
  otomasyonu üretmek API desteği sayılmasın.
- Python API örneği, birim/CRS açıklaması, girdi/çıktı şeması, ret
  kodları ve gerçek senaryo belgesi aynı teslimde üretilecek. KentOS
  UI bileşenleri, `.kek`, isteğe bağlı uv ortamı/bağımlılık ve eklenti
  onayı için [plugins.md](plugins.md) ve [python.md](python.md) izlenecek.
- AI mevcut kayıtlı araçları kullanabilsin: veriyi keşfet, kapsamı
  belirle, planı/farkı üret, sonucu açıkla ve yetki dahilinde uygula.
  Sağlayıcı seçimi opsiyonel olsun; erişilemeyen/gizli veri sorgu,
  model girdisi veya çıktı dosyası üzerinden sızmasın. API anahtarı projeye yazılmasın.
- Bulutta aynı parsel/sınır/yüzey üzerinde eşzamanlı değişiklikler
  fark ve revizyonla uzlaştırılsın. Mevcut PostgreSQL/PostGIS ve taşınabilir
  dosya altyapısı kullanılsın; rakip destekliyor diye yeni kurumsal DB motoru eklenmesin.
- Büyük veri testleri gerçek ölçümle ayrı raporlansın: ilk görünüm,
  etkileşim p95/p99, tam kaynak sorgusu, yeniden hesap, RAM/VRAM ve
  iptal gecikmesi. GPU'nun hızlı önizlemesi toplam iş süresi diye sunulmasın.

**CBS ortaklığı:** iş grafiği, erişim/işlem günlüğü, Processing, Python,
MCP, eklenti ve AI sözleşmesi bütün ürünlerin ortak altyapısıdır. Harita
için ayrı, GIS'ten kopuk otomasyon ekosistemi kurulmaz.

**Kabul:** aynı küçük üretim işini GUI, Python ve MCP ile yap; geometri,
kimlik ilişkisi, retler ve kalıcı sonuçlar eşdeğer olsun. Ağır işi iptal et,
veri/izin değişirken sonucu yazmayı dene, yeniden açıp aynı tariften üret.
Bir yüzey noktası değiştiğinde hangi profil/hacim/paftanın yeniden
hesaplanması gerektiği görülsün; güncel olmayan çıktı başarıyla teslim edilmesin.

**7 Kasım 2026 — Harita projesinde proje ağacı ve yeniden üretim ([ADR 1720](../docs/adr/1720-map-project-tree-and-rebuild.md)).**
Kapsam: **Harita + Yol**. Kanıtlanan: `.kcad` şema 198 (Harita'nın çizim kayıtları ve kendiliğinden yeniden üretme;
sıfır hattı çizimi her projede) Harita'da planda platform çizildikten sonra kaydedilemeyen projeyi düzeltir; sıfır hattı
iki yüzeyinin damgasıyla tutulur; `road.project.query/rebuild/edit` Harita'yı kabul eder: bir yüzey kotu değişince
sıfır hattı “güncel değil” görünür, yeniden üretilir ya da açıksa aynı adımda kendiliğinden üretilir; masaüstünde
“Proje ağacı” paneli; eskimiş çizimi gösteren paftanın haritası baskı öncesi denetimde `outdated_drawing` hatası alır,
yayın ve kitabın PDF'i yeniden üretilene dek durur. Açık: atlas sayfaları, profil/hacim/pafta zincirinin tek görünüşü,
iş grafiği, iptal ve GUI/Python/MCP eşdeğerliği.

**7 Kasım 2026, ikinci tur — İş grafiği ([ADR 1940](../docs/adr/1940-project-graph.md)).** Kapsam: **Harita + Yol**.
Araştırma: ArcGIS Pro'nun geoprocessing geçmişi (parametreler, ortam, zaman, durum; projede ve metaveride; yeniden
açılır), Civil 3D'nin Prospector'daki “out of date” bayrağı ve otomatik yeniden kurma, QGIS Processing geçmişi, STAC
processing alanları; hiçbiri katmandan paftaya bütün zinciri göstermiyor. Kanıtlanan: `road.project.graph` yüzeylerin
katmanlarını, proje ağacının satırlarını ve bir çizimi gösteren paftaları kenarlarıyla (kurar, dayanak, kaynak: damga
o zaman/şimdi, gösterir: harita) okur; her çizimin tarifi (komut, sürüm, saklanan girdi, nesneler, damga) ve projenin
sistemi, epoku, kot sistemi; güncel olmayan çizimi gösteren pafta “teslim edilemez”; paftanın zemini ve sınaması baskı
denetimiyle ortak; 21 bağımsız durum (Rust, Python), aynı işin MCP üzerinden aynı grafikleri verdiği test; Proje ağacı
panelinde İş grafiği… penceresi, Tarifi kaydet. Açık: çizim kaydında işlem sürümü ve çıktı revizyonu (şema), ölçüm
arşivinin zincire bağlanması, tariften başka projede üretim, atlas sayfaları, iptal.

## Uygulama sırası ve sahiplik

Ana madde sayısı 20'dir; aşağıdaki sıra yeni madde veya yeni tamamlanma
işareti oluşturmaz.

| Aşama | İşler | Geçiş koşulu |
| --- | --- | --- |
| Temeli koru | MAP-01, MAP-02, MAP-06, MAP-07; MAP-17/18 mevcut aktarım ve okunurluk; MAP-20 ortak sözleşmeleri | Mevcut ölçek, render, seçim, sağ tuş, tipli katman ve dosya açma regresyonları temiz; güncel masaüstü GUI kabulü var. |
| Kaynağı ve yüzeyi tamamla | MAP-03, MAP-04 temel katalog, MAP-05, MAP-12, MAP-13, MAP-14 | Ham veri/çözüm/türev ayrı; bağımsız doğruluk ve kaynak izleme; gerçek veriyle iptal/bellek kanıtı. Donanımsız kısım ayrı. |
| Tasarıma ve üretime bağla | MAP-15, MAP-16, MAP-18, MAP-19 çevrimdışı; MAP-17 seçilmiş mühendislik profilleri | Bir kaynağın değişimi yüzey, tasarım, analiz ve teslimde kontrollü izleniyor; GIS tüketimi ve otomasyon tamam. |
| Planlama ve kurumsal veri | MAP-08 teknik model, MAP-09, MAP-10; MAP-04 ileri çıkarım | Tipli semantik veri, yetki, senaryo, profil doğrulaması ve kaynaklı raporlar var. Resmî işlem kapıları açık değil. |
| Cihaz / kurumsal son kabul | MAP-03/19 canlı donanım; MAP-08 resmî kapsam, MAP-11 ve MAP-09/18 resmî profil çıktıları | Gerçek cihaz sağlanmış ve saha testi yapılmış; mevzuat kapsamı sahibiyle ayrı çalışılmış, güncel profil ve yetkili uzman/kurum kabulü alınmış. |

MAP-08'in teknik veri modeli MAP-09/10'la erken tasarlanabilir; resmî parsel
işlemleri bundan ayrı son kapıda kalır. MAP-20 sona bırakılan toplu bir
“API ekleme” işi değildir: her teslimin parçasıdır. [road.md](road.md),
[3d.md](3d.md), [gis.md](gis.md) veya diğer planlarda zaten biten ortak iş
yeniden yazılmaz; bu plandaki yeni kabul kapsamı için tekrar kullanılır.

## Her madde için ortak kalite ve kabul koşulları

- **Mevcut davranışı koruma:** önce güncel commit'ten masaüstü derlemesi
  ve mevcut ilgili örneklerle başlangıç kaydı alınır. İşlev, sadece yeni
  test sahnesinde değil kullanıcının gerçek koordinatlı ve karışık kaynaklı
  projesinde de sınanır. Test penceresi/derlemesi ve commit belirtilir.
- **Gerçek etkileşim:** fare, klavye, sayısal alan, Enter, sağ tuş Onayla,
  Esc, Undo/Redo, dar/geniş pencere ve art arda zoom/pan denenir. Hesap
  testinin geçmesi GUI kabulünü kapatmaz. Ekran görüntüsü tek başına
  etkileşim kanıtı da değildir; işlem adımı ve önce/sonra veri durumu gerekir.
- **Görüntü kanıtı:** ekran görüntüleri yerelde, repo dışında tutulur.
  Kabul kaydı gerekirse görüntünün konumunu, senaryosunu ve sonucunu
  belirtir; büyük ekran görüntüleri Git'e eklenmez.
- **Bağımsız sayısal kanıt:** test bekleneni aynı uygulama fonksiyonunu
  çağırarak üretmez. Analitik örnek, yüksek hassasiyetli hesap veya uygun
  bağımsız motor/veri kullanılır. Datum, birim, epok ve tolerans belirtilir.
- **Kalıcılık ve işlem bütünlüğü:** validate/plan/ret/iptal belgeyi,
  dirty durumunu ve undo zincirini değiştirmez. Başarılı uygulama tek
  mantıksal değişikliktir; kaynak/ilişki/sonuç save–reopen ile korunur.
- **Erişim ve birlikte çalışma:** kilitli katman, salt okunur kullanıcı,
  alan/kayıt yetkisi, eşzamanlı kaynak değişimi ve tekrar gönderilen
  istek sınanır. Bir analiz/AI/eklenti yolu GUI'nin veri kuralını aşamaz.
- **Performans:** veri boyu, nokta/üçgen/nesne sayısı, cihaz/işletim
  sistemi, CPU/GPU yolu, ilk/ısınmış ölçüm ve bellek tepe değeri raporlanır.
  Önizleme ile tam çözünürlüklü analiz ayrı ölçülür; bütçe uygulama öncesi
  belirlenir. Başarılmayan hedef sayısı başarı iddiasına dönüşmez.
- **Eksiksiz yüzey:** uygun UI, komut kataloğu, Python örneği, MCP,
  Processing ve paylaşılmış CBS yeteneği birlikte gözden geçirilir.
  Kapsam dışı donanım/profil açık yazılır; bu kısım tamamlanmış işaretlenmez.

## İncelenen web kaynakları

Erişim/inceleme: **31 Ekim 2026**. Aşağıdaki **35** satır, gerçekten içeriği
okunan ayrı kaynaktır. Sürüm varsa korunmuştur. Ürün sayfası yeteneğin üretici
beyanını gösterir; kod, doğruluk testi veya KentOS performans kanıtı değildir.
Her kaynak için KentOS'a aktarılan bilgi kısaca belirtilmiştir.

| No | Kaynak | İncelemede kullanılan bilgi / sınır |
| --- | --- | --- |
| S01 | [Netcad GIS][S01] | CAD/CBS birlikte üretim, kaynaklar ve veri alışverişi; kapalı motorun matematiği hakkında çıkarım yok. |
| S02 | [NetSurf][S02] | Arazi kaynakları, yüzey güncelleme, eğri, profil ve hacim ilişkisi. |
| S03 | [Netmap][S03] | Parsel, malik/dağıtım ve grafik/tablo denetimi; güncel hukuk doğrulaması sayılmadı. |
| S04 | [Planet][S04] | Plan seviyeleri, plan verisi ve kontroller, PlanGML ve alan istatistiği; incelenen ürünün adı Planet. |
| S05 | [Netpro][S05] | Yol geometrisi, kesit, metraj ve belgeleme akışının ilişkisi. |
| S06 | [Netkamu][S06] | Kamulaştırma verisi, sahiplik/tablo ve teslim denetiminin birlikte ele alınması. |
| S07 | [Nettop][S07] | Arazi toplulaştırmasında öneri, değerlendirme, sahiplik ve kontrol akışı. |
| S08 | [Civil 3D 2027 — Survey Databases][S08] | Gözlem/kontrol/ekipman verisinin çizimden ayrı tutulması. KentOS için aynı depolama motoru zorunlu değil. |
| S09 | [Civil 3D 2027 — Surface Boundaries][S09] | Dış/göster/gizle/veri kırpma sınırları ve işlem sırası; görsel gizleme ile veri çıkarma farklı. |
| S10 | [Bentley OpenCities Map][S10] | Şema/kurallı CAD+GIS verisi ve bağlam kaynakları. Ticari veritabanı listesi KentOS'a bağımlılık kararı olarak alınmadı. |
| S11 | [Bentley OpenSite][S11] | İlişkili saha/tesviye ve arazi işi tasarımı. Üretici hız beyanı KentOS hedefinin kanıtı değil. |
| S12 | [TBC — Understanding Baseline Processing][S12] | GNSS oturumu, anten/yörünge seçimi ve çözüm üretiminin nokta içe aktarmadan farklı oluşu. |
| S13 | [TBC — Adjust Network][S13] | Ağ, kontrol, kalite, hata bilgisi ve yeniden hesap akışı. |
| S14 | [TBC — Scanning and point clouds][S14] | Kayıt, sınıflama, çıkarım, as-built karşılaştırma. Bazı yeteneklerin veri/lisans kapsamı ürüne özgü. |
| S15 | [TBC — point feature extraction][S15] | Buluttan nokta varlığı üretimi ve kod/öznitelik atama tanımı. |
| S16 | [12d — Surveying][S16] | Modern LiDAR, gerçekleşen veri ve tasarım uygunluk akışları. Sayfadaki eski yöntemler yeni hedefe alınmadı. |
| S17 | [ArcGIS Pro 3.5 — Parcel least squares][S17] | Kayıtlı ölçü, ağ ve ağırlıklı analiz; parsel polygonunun ötesindeki veri ilişkisi. |
| S18 | [ArcGIS Pro — Run least squares adjustment][S18] | Analiz çıktısını inceleme ile kaynağa uygulamanın ayrılması. Ürünün varsayılan eşikleri kopyalanmadı. |
| S19 | [ArcGIS Urban 11.5 — Create a plan][S19] | Çalışma alanı, mevcut durum ve alternatiflerin ayrılması; KentOS senaryo tasarımına referans. |
| S20 | [XLeitstelle — XPlanung FAQ][S20] | Makinece okunur plan ve şema/profil doğrulaması; teknik doğrulamanın uzmanlık/karar onayı olmayışı. |
| S21 | [QGIS release-4_2 — gelişmiş çizim kaynak kodu][S21] | `QgsAdvancedDigitizingDockWidget`: sert/yumuşak kilitler, X/Y/Z/M ve `QgsCadUtils::alignMapPoint` çağrısı. Gerçek kod okundu; testing belgesi kararlı sürüm diye sunulmadı. |
| S22 | [GRASS stable 8.5 — v.clean][S22] | İşlem türü, sıra, eşik, hata çıktısı ve veri birimi ayrımı. |
| S23 | [GRASS — vector/v.clean/main.c][S23] | Temizleme adımlarının sıralı yürütülmesi ve topoloji kurma çağrıları gerçek koddan kontrol edildi; dal değişebilir. |
| S24 | [Geoscience Australia — DynAdjust][S24] | Resmî depodaki 3B ağ/kovaryans ve büyük ağ yaklaşımı açıklaması. Çözücünün iç kodu satır satır denetlenmiş sayılmadı. |
| S25 | [PROJ stable — Vertical grid shift][S25] | Düşey ızgara, kapsam ve zaman boyutunun koordinat işlemine etkisi. |
| S26 | [PROJ stable — Kinematic datum shifting][S26] | ENU hız, referans zaman ve zamana bağlı dönüşüm modeli. |
| S27 | [PDAL — filters.smrf][S27] | Zemin sınıflama parametreleri ve çıktı sınıfı. PDAL kurma veya bağımlılık ekleme kararı alınmadı. |
| S28 | [OGC — InfraGML][S28] | Parçalar halinde mühendislik/ölçüm/arazi bölümü modeli. Sayfanın ilk kapsamı ıslak altyapıyı bütünüyle içermiyor; hidrolik model standardı diye sunulmadı. |
| S29 | [US EPA — EPANET][S29] | Basınçlı su ağı hidrolik/su kalitesi hesaplarının analiz grafiğinden farkı. |
| S30 | [US EPA — SWMM][S30] | Yağmursuyu/hidroloji/hidrolik çözüm kapsamı; eğim haritasının simülasyon yerine geçmemesi. |
| S31 | [COPC 1.0 specification][S31] | LAZ 1.4 tabanlı seçmeli okuma, octree/hiyerarşi ve parça düzeni. Belgenin yaşı tekniği eski iş akışı yapmıyor; mevcut desteğin üzerine ihtiyaç tanımlandı. |
| S32 | [OGC — 3D Tiles standard sayfası][S32] | 1.1 standart bağlantısı ve büyük 3B içerik akışının amacı incelendi. Büyük tam normatif belge bu oturumda açılamadı; tam standarda uygunluk denetimi yapılmış sayılmaz. |
| S33 | [STAC — The STAC Specification][S33] | Item/Collection/Asset/zaman ve keşif sözleşmesi. Sayfadaki örnek sürüm en yeni sürüm iddiası yapılmadı. |
| S34 | [GeoParquet 1.1.0][S34] | Geometri kolonları, CRS/PROJJSON, epoch ve kapsama metadata'sı. İç CAD ilişkilerini bütünüyle saklama garantisi değil. |
| S35 | [TKGM — 2025/4 genelgesi][S35] | Belgenin kapsamı ve harita/plan tesliminde kayıtlı teknik bağlam incelendi. 12 Mayıs 2025 tarihli bu belge 31 Ekim 2026'da bütün kuralların yürürlük doğrulaması olarak kullanılmadı; oran/sınır türetilmedi. |

### Erişim sınırları ve uygulamadan önce tamamlanacak kaynak doğrulaması

- Netcad teknik wiki sayfalarında zaman aşımı alındı; ürün sayfalarının
  içeriği okundu, açılamayan teknik wiki bu 35 kaynağa sayılmadı. Özellikle
  belirli komutun bütün parametreleri doğrulanmış gibi yazılmadı.
- Türkiye PlanGML resmî şema uç noktasının güncel içeriği bu oturumda
  edinilemedi. MAP-09'un uygulama kapısı: resmî şema/profil, sürüm, kod
  listeleri ve örnek geçerli/geçersiz teslim paketlerinin edinilmesidir.
- HGM'nin güncel jeoit dağıtım sayfası arama dizininde görüldü fakat
  içerik açılışı zaman aşımına uğradı; kaynak sayısına alınmadı. Üretim
  ızgarası için resmî temin, kapsam ve kullanım koşulları doğrulanacak.
  Kamuya açık gösterim/örnek ızgarası otomatik üretim modeli kabul edilmez.
- 3D Tiles tam normatif 1.1 HTML belgesi araç boyut sınırını aştı;
  standart sayfasının amaç/sürüm bilgisi okundu. İçerik türü/uzantı ve
  uygunluk testi, [3d.md](3d.md) uygulaması sırasında normatif kaynakla yapılır.
- Canlı kaynaklar değişebilir. Uygulama başlarken gerçekten seçilen
  standart/şema/model sürümü ve açık kaynak commit'i sabitlenir; bu
  rapordaki araştırma tarihi sonsuz geçerlilik iddiası değildir.

[S01]: https://www.netcad.com/tr/urunler/netcad-gis
[S02]: https://www.netcad.com/tr/urunler/netsurf
[S03]: https://www.netcad.com/tr/urunler/netmap
[S04]: https://www.netcad.com/tr/urunler/planet
[S05]: https://www.netcad.com/tr/urunler/netpro
[S06]: https://www.netcad.com/tr/urunler/netkamu
[S07]: https://www.netcad.com/tr/urunler/nettop
[S08]: https://help.autodesk.com/cloudhelp/2027/ENU/Civil3D-UserGuide/files/GUID-E0B6AAE2-A5EC-438F-8687-B5E023F85643.htm
[S09]: https://help.autodesk.com/cloudhelp/2027/ENU/Civil3D-UserGuide/files/GUID-3EF4691D-2645-4357-819C-E3FA3E14BF21.htm
[S10]: https://www.bentley.com/products/opencities-map/
[S11]: https://www.bentley.com/products/opensite/
[S12]: https://help.fieldsystems.trimble.com/tbc/1922.htm
[S13]: https://help.fieldsystems.trimble.com/tbc/396.htm
[S14]: https://help.fieldsystems.trimble.com/tbc/discover-scanning-point-clouds.htm
[S15]: https://help.fieldsystems.trimble.com/tbc/26260.htm
[S16]: https://www.12d.com/product/surveying.html
[S17]: https://pro.arcgis.com/en/pro-app/3.5/help/data/parcel-editing/least-squares-parcel-fabric.htm
[S18]: https://doc.esri.com/en/arcgis-pro/latest/help/data/parcel-editing/runleastsquaresadjustment.html
[S19]: https://doc.arcgis.com/en/urban/11.5/help/help-create-plan.htm
[S20]: https://xleitstelle.de/XPlanung/faq
[S21]: https://raw.githubusercontent.com/qgis/QGIS/refs/heads/release-4_2/src/gui/qgsadvanceddigitizingdockwidget.cpp
[S22]: https://grass.osgeo.org/grass-stable/manuals/v.clean.html
[S23]: https://raw.githubusercontent.com/OSGeo/grass/main/vector/v.clean/main.c
[S24]: https://github.com/GeoscienceAustralia/DynAdjust
[S25]: https://proj.org/en/stable/operations/transformations/vgridshift.html
[S26]: https://proj.org/en/stable/operations/transformations/deformation.html
[S27]: https://pdal.org/en/latest/stages/filters.smrf.html
[S28]: https://www.ogc.org/standards/infragml/
[S29]: https://www.epa.gov/water-research/epanet
[S30]: https://www.epa.gov/water-research/storm-water-management-model-swmm
[S31]: https://copc.io/
[S32]: https://www.ogc.org/standards/3dtiles/
[S33]: https://stacspec.org/en/about/stac-spec/
[S34]: https://geoparquet.org/releases/v1.1.0/
[S35]: https://www.tkgm.gov.tr/sites/default/files/2025-05/2025_4%20Sayili%20Tescile%20Konu%20Harita%20ve%20Planlarin%20Yapimi%20ve%20Kontrolu%20Genelgesi.pdf

[K01]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/apps/desktop/src/module_surfaces.rs#L214
[K02]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/contracts/src/document.rs#L91
[K03]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/contracts/src/document.rs#L1040
[K04]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/crs/operation.rs#L203
[K05]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/crs/geoid.rs#L50
[K06]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/crs/ground.rs#L90
[K07]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/render/wgpu/src/lib.rs#L27
[K08]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/render/wgpu/src/styled/gpu.rs#L1339
[K09]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/contracts/src/formats.rs#L704
[K10]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/formats/src/gnss/nmea.rs#L26
[K11]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/apps/desktop/src/calc/fieldbook/mod.rs#L1
[K12]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/application/src/survey_points.rs#L67
[K13]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/survey/adjust/horizontal.rs#L23
[K14]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/survey/adjust/mod.rs#L61
[K15]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/survey/adjust/linalg.rs#L1
[K16]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/ops/edge_shift.rs#L22
[K17]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/ops/topology_rules/mod.rs#L28
[K18]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/ops/topology_rules/incremental.rs#L145
[K19]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/application/src/parcels.rs#L19
[K20]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/style/src/system.rs#L1
[K21]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/application/src/scenarios_edit.rs#L1
[K22]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/application/src/rasters_georef.rs#L485
[K23]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/services/src/stac/mod.rs#L1
[K24]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/formats/src/raster/cog.rs#L1
[K25]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/pointcloud/src/ops/mod.rs#L1
[K26]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/pointcloud/src/ops/ground.rs#L17
[K27]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/pointcloud/src/ops/query.rs#L11
[K28]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/geom/delaunay.rs#L1
[K29]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/raster/src/contours.rs#L1
[K30]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/commands/src/cad_meshes.rs#L1
[K31]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/contracts/src/network.rs#L1
[K32]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/geometry-core/src/ops/lrs/mod.rs#L1
[K33]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/application/src/gis_derive.rs#L240
[K34]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/application/src/gis_takeback.rs#L1
[K35]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/contracts/src/xref.rs#L45
[K36]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/sheet/src/atlas.rs#L137
[K37]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/sheet/src/display/map.rs#L1
[K38]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/sheet/src/pdf/geo.rs#L1
[K39]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/shared/sheet/src/preflight.rs#L1
[K40]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/processing/src/lib.rs#L1
[K41]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/server/application/src/jobs.rs#L1
[K42]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/server/application/src/job_cache.rs#L1
[K43]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/headless/src/dispatch.rs#L1
[K44]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/python/src/lib.rs#L1
[K45]: https://github.com/mcihad/kentos-cad/blob/5cabec02be37cd9b175f0764e3aafcfc8d89f90e/crates/native/mcp/src/tools.rs#L1
