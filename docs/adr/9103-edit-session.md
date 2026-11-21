# ADR 9103: Veri katmanının düzenleme oturumu — tampon, kaydetme, geri alma

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-05.
- **İstek:** sahibin sözü (20 Kasım 2026): “veri kaydetme qgis gibi olmalı … verileri çiziyoruz ama kaydedecek yapı yok
  nasıl kaydedilecek belli değil, qgis incele edit modu veri kaydetme yapısı aktif katman katman tipleri gibi yapılar GIS
  için uygun değil”; “demek istediğim şu anki yapı uygun değil”.
- **Önceki kararla ilişkisi:** [ADR 9101](9101-data-layers.md) §2 “veri katmanı nesne tutmaz” der. Bu ADR onu daraltır:
  düzenlenen bir veri katmanının kaydedilmemiş satırları, kaydedilene kadar çizimde katmanın nesneleri olarak bekler
  (§2). Düzenlenmeyen veri katmanı yine nesne tutmaz; tutmasına bütünlük denetimi izin vermez.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** şema değişmez (§2). **Komut:** `cad.layers.session` v1 (§3).
- **Bağımlılık:** yeni dış kitaplık yok. Python ve MCP veri sürücülerini kaydeder (§3).

## §1 Bağlam ve araştırma

**Bağlam.** Etkin katman bir veri katmanıyken çizilen nesne belgeye yazılıyordu. Katmanın satırları ise kaynaktan
çiziliyordu; çizilen nesne ekranda görünmüyor, kaynağa yazılmasının bir yolu da yoktu. “Düzenleme modu”, “kaydet” ve
“geri al” yalnız çizimin kendisi içindi.

**Masaüstü CBS** (QGIS 4.2.3, bu makinede kurulu; ekranlar `.run/gui-evidence/veri/qgis/`; kaynak kodu
`src/app/qgisapp.cpp`, `src/core/vector/qgsvectorlayer.h`, 21 Kasım 2026'da okundu):

- **Düzenleme katman başınadır.** Katman açıldığında salt okunurdur; “Düzenlemeyi aç/kapat” onu düzenlemeye açar
  (sağ tuş, öznitelik tablosu, sayısallaştırma araç çubuğu). Ağaçta düzenlenen katman kalem simgesiyle görünür
  (`04-duzenleme-modu-kaydedilmemis.png`).
- **Değişiklikler katmanın tamponunda bekler:** eklenen, değişen, silinen kayıtlar. Kaydetmek
  (`commitChanges(stopEditing=false)`) katmanı düzenlemede bırakır.
- **Kaydetme reddedilirse tampon dokunulmadan kalır.** `QgsVectorLayer::commitChanges` belgesi: “If a commit fails …
  the in-memory changes are left untouched and are not discarded. This allows editing to continue if the commit failed
  on e.g. a disallowed value in a Postgres database - the user can re-edit and try again.” Yazma aşamalar hâlinde
  yapılır (öznitelik ekle, kayıt ekle, değer değiştir, geometri değiştir, kayıt sil); bir aşama düşerse geri dönmek
  güçleşebilir ve hata iletisi hangi aşamanın düştüğünü söyler.
- **“Tüm düzenlemeler” menüsü** (`06-tum-duzenlemeler-menusu.png`): seçili katmanlar ve bütün katmanlar için Kaydet,
  Geri çağır (tamponu bırakır, düzenlemede kalır) ve Vazgeç (tamponu bırakır, düzenlemeden çıkar).
- **Düzenlemeyi kapatırken** katmanda değişiklik varsa sorulur: “Do you want to save the changes to layer %1?” —
  Kaydet / Kaydetme / Vazgeç (`07-duzenlemeyi-bitir-sorusu.png`; `QgisApp::toggleEditing`).
- **Proje kapanırken** her değişikli katman için aynı soru tek tek sorulur (`QgisApp::checkUnsavedLayerEdits`). Proje
  kaydedildiğinde tampon projeye yazılmaz: “Project has layer(s) in edit mode with unsaved edits, which will NOT be
  saved!”.
- **Geri alma katman başınadır:** her katmanın kendi geri alma yığını vardır; geri alma penceresi etkin katmanınkini
  gösterir (`mUndoWidget->setUndoStack(layer->undoStack())`).
- **Sayısallaştırma** etkin katman düzenlemedeyken ve aracı katmanın geometri türüne uyuyorsa açıktır; çizimden sonra
  öznitelik formu açılır. Seçili kayıtlar haritadan ya da tablodan seçilip silinir.

**Türkiye mevzuatı:**

- Bilgi ve İletişim Güvenliği Rehberi (Siber Güvenlik Başkanlığı, sürüm 1.1, Mart 2026; dayanağı 2019/12 sayılı
  Cumhurbaşkanlığı Genelgesi), §3.2.7 “Veri Tabanı ve Kayıt Yönetimi”:
  - 3.2.7.1: her veri tabanı kullanıcısının ayrı hesabı olur, ortak hesap kullanılmaz; kullanıcıya gereken en az yetki
    verilir;
  - 3.2.7.13: verilerin yanlışlıkla silinmesine karşı geri döndürme yolları önceden kurulur;
  - 3.2.7.16 ve 3.2.7.18: yetkili kullanıcıların, üst düzeyde bütün kullanıcıların işlemleri için denetim kayıtları
    oluşturulur.
- 6698 sayılı Kişisel Verilerin Korunması Kanunu md. 12(1): veri sorumlusu kişisel verinin hukuka aykırı işlenmesini
  önlemek için uygun teknik tedbirleri alır.

Bu hükümlerden KentOS'un çıkardığı:

- Değişiklik kaydedilene kadar veri tabanına hiçbir şey gitmez; kullanıcı görür, geri alır ya da bırakır (3.2.7.13).
- Yazma, kullanıcının kendi bağlantısıyla ve kendi hesabının yetkisiyle yapılır. Yetkinin ve sürücünün izin vermediği
  yazma önceden reddedilir (3.2.7.1).
- Bir katmanın değişiklikleri tek işlemle yazılır. Veri tabanının kendi denetim kaydı yazmayı o kişiye bağlar (3.2.7.16).
- Alınan satırlar proje dosyasına yazılmaz; kişisel veri tablosunda kalır (6698 md. 12).

## §2 Karar: oturum ve tampon

Bir veri katmanı düzenlemeye açılır (`Document::start_editing`). Oturum belgenin kendisinde durur ama dosyaya
yazılmaz. Açmak düzenleme sayılmaz: geri alma adımı oluşmaz, çizim kaydedilmemiş olmaz. Salt okunur katman açılmaz.

Tampon üç şeydir:

| Ne | Nerede | Geri alma |
|---|---|---|
| **Yeni satır** | katmana çizilen nesne (anahtarı yok) | çizimin her adımı gibi |
| **Alınan satır** | tablodan okunup katmanın nesnesi yapılan satır; anahtarı ve okunduğu hâli yanında (`Taken`) | alma adım değildir; nesneye yapılan her şey adımdır |
| **Silinen anahtar** | almadan silinen satırın anahtarı | “Satırları sil” adımı (`GisOp::DataDeleted`) |

Katmanın yazacakları istendiğinde hesaplanır (`Document::data_changes`):

- anahtarsız nesne yeni satırdır;
- okunduğu hâlden farklı alınan nesne satırı değiştirir;
- yok olan alınan nesne (silinmiş ya da başka katmana taşınmış) satırı siler; silinen anahtarlar da öyle.

Bundan çıkan:

- **Çizimin bütün araçları veri satırlarında çalışır:** taşı, döndür, köşe düzenle, böl, sil, öznitelik yaz. Ayrı bir
  düzenleme aracı ailesi yoktur.
- **Geri alma tek sıradır.** Çizimin adımları ve tamponun adımları aynı sırayla geri alınır; QGIS'in katman başına
  yığınından ayrıdır. Nedeni: bir CAD+CBS programında bir adım hem çizime hem tampona dokunabilir (bir çizim nesnesini veri
  katmanına taşımak), ve Ctrl+Z'nin etkin katmana göre değişmesi birden çok katmanla çalışırken şaşırtır.
- **Kaydetmek, geri almak ve bitirmek** katmanın nesnelerini çizimden başka birinin değişikliği gibi çıkarır (geri alma
  adımı olmadan, `apply_external`) ve o katmanın nesnelerine dokunan geri alma adımlarını düşürür. Tabloya yazılmış satır
  buradan geri alınmaz; QGIS de kaydetmede katmanın yığınını boşaltır.
- **Kaydetme sürerken yapılan değişiklik kaybolmaz:** yazılan hâlinden farklı kalan nesne çizimde kalır. Yazılan hâle
  karşı alınmış sayılır; sonraki kaydetme farkı yazar.
- **Bütünlük:** düzenlenmeyen bir veri katmanına nesne yazan adım reddedilir (`data_layer_not_editing`). Bu kural
  araçların, Python'un, yapıştırmanın ve katman değiştirmenin yazmasını tek yerden tutar. Katmanın türüne uymayan nesneyi
  ADR 0622'nin kuralı zaten reddeder: alan katmanına çizgi yazılmaz.
- **Dosya:** kaydedilen `.kcad` yeni satırları katmanın nesneleri olarak tutar. Alınan satırları yazmaz; onlar
  tablosundadır. Böylece çizilen hiçbir şey kaybolmaz ve aynı satır iki kez yazılmaz. Açılan dosyada nesne tutan veri
  katmanı düzenlemededir: eski projelerde veri katmanına çizilmiş nesneler artık kaydedilebilir.

## §3 Karar: `cad.layers.session` v1

`action`:

- `start`: düzenlemeye açar;
- `save`: her katmanın değişikliklerini kaynağının tek işlemiyle yazar, katman açık kalır;
- `rollback`: değişiklikleri bırakır, katman açık kalır;
- `cancel`: bırakır ve kapatır;
- `stop`: değişikliği olmayan katmanı kapatır;
- `take`: satırları anahtarlarıyla çizime alır;
- `delete`: satırları anahtarlarıyla, almadan siler (bir geri alma adımı);
- `status`: oturumu söyler, hiçbir şey yazmaz.

`layers` verilmezse düzenlenen bütün katmanlar (`save`, `rollback`, `cancel`, `stop`, `status`). `start` en az bir,
`take` ve `delete` tek bir katman ister; `keys` 1–10 000 anahtar.

Retler sırasıyla:

- `no_layer`, `one_layer`, `no_keys`, `invalid_count`;
- `layer_not_found`, `not_a_data_layer`, `data_layer_read_only`;
- `data_layer_not_editing`, `data_edits_unsaved`;
- `source_unreachable`, `data_save_failed`.

Birkaç katman kaydedilirken yazılamayanlar `data_save_failed` uyarısıyla söylenir, ötekiler yazılır; hepsi yazılamazsa
ret olur. Çıktı her katmanın oturumunu söyler: düzenlemede mi, kaç yeni, değişen ve silinen satır, kaydetmede yeni
satırların anahtarları, yazılamadıysa nedeni. `take` alınan nesnelerin kalıcı kimliklerini anahtarların sırasıyla
verir.

- **Ortak durumlar:** 29 durum (`fixtures/commands/v1/cad.layers.session.json`). Bağımsız betik
  (`scripts/fixtures/layer_session_command_cases.py`) GeoPackage'ını sqlite3 ile yazar: R*Tree dizini, §F.3
  tetikleyicileri, boş geçilemez bir alan ve bir CHECK kısıtı. Her durum kendi kopyasına yazar.
- **Python ve MCP:** `kentos.cad.layers.session`; MCP aracı katalogdan gelir ve yıkıcı işaretlidir. Python eklentisi ve
  MCP sunucusu GeoPackage ve PostGIS sürücülerini başlangıçta kaydeder. VY-04'ün `cad.layers.rows`'u da Python'da böylece
  çalışır: önce sürücü kaydedilmediği için kaynağı açamıyordu.

## §4 Karar: tabloya yazma

`data_write::write` (`crates/native/application/src/gis/data_write.rs`):

- **Geometri:** katmanın nesnesi tablonun sütununa uydurulur:
  - çizgi sütununa alanın sınırı, alan sütununa kapalı çizgi;
  - eğri tutmayan sütuna yaylar en çok 5°'lik kirişlerle;
  - tablonun koordinat sistemine çevrilir;
  - Z ve M sütunun tuttuğu gibi;
  - çoklu sütuna tekil parça çoklunun tek parçası olarak (`kentos_data::fit`).
- **Değerler:** alanın türüne göre denetlenir (`kentos_data::coerce`); boş değer NULL'dur. Boş geçilemeyen ve varsayılanı
  olmayan alan, yazmadan önce nesneyle birlikte söylenir. Yeni satır sahip olduğu değerleri alır, ötekiler tablonun
  varsayılanını. Değişen satıra yalnız değişen değerler ve değiştiyse geometri yazılır. Anahtar, kaynağın kendi ürettiği
  alan ve tabloda olmayan ad hiç yazılmaz.
- **Anahtar:** sözleşmeye `Schema::key_made` eklendi: yeni satırın anahtarını kaynak mı verir. GeoPackage'ta her zaman
  verir; PostGIS'te varsayılanı ya da kimlik (identity) tanımı olan sütunda verir. Vermiyorsa yazıcı tablonun en büyük
  anahtarından sonrakileri verir. Başka biri arada aynı anahtarı alırsa birincil anahtar yazmayı reddeder, hiçbir şey
  yazılmaz. QGIS böyle bir tabloda kaydedemez.
- **İşlem:** bir katmanın satırları tek işlemle yazılır: önce eklenenler, sonra değişenler, sonra silinenler. Bir hata
  hepsini geri alır, tampon olduğu gibi kalır.
- **Yetki:** tablonun izinleri (`Capabilities`) yazılacak işe karşı önceden denetlenir: ekleme, geometri değiştirme, değer
  değiştirme, silme.

## §5 Karar: masaüstü

- **Katmanlar:** veri katmanının sağ tuşunda Düzenlemeyi başlat ya da Düzenlemeyi bitir, Değişiklikleri kaydet,
  Değişiklikleri geri al. Başka katmanlar da düzenleniyorsa Bütün katmanları kaydet ve Bütün katmanları geri al.
  Düzenlemeyi başlatmak katmanı etkin katman yapar: çizim araçları ona çizer.
- **Satır:** düzenlenen katmanda kalem imi. Kaydedilmemiş değişiklik varsa im uyarı renginde olur; ipucu kaç satırın
  yazılacağını söyler.
- **Soru:** değişikli katman bitirilirken ve çizimden çıkılırken (pencere kapanırken, başka proje açılırken, sekme
  kapanırken) sorulur: “… değişiklikler tablosuna kaydedilsin mi?” Kaydet / Kaydetme / Vazgeç. Çizimden çıkarken
  değişikli bütün katmanlar tek soruda sayılır; QGIS katman katman sorar. Katman sorusu projenin kendi sorusundan önce
  gelir.
- **Kaydetme** arayüzün iş parçacığında yapılmaz. Bitince bir bildirim çıkar: ““Parseller” kaydedildi — 1 yeni, 1
  değişen, 1 silinen satır tabloya yazıldı.” Reddedilirse neden söylenir; suçlu nesne varsa seçilir.
- **Çizim:** canlı okuma düzenleme oturumunun alınan ve silinen satırlarını atlar. Sahne katmanın nesnelerini kaynağın
  satırlarının üstüne çizer.
- **Seç:** çizimde nesne bulamayan tıklama ve her çerçeve görünen veri katmanlarına sorulur (`ViewChange::DataSelect`):
  - düzenlenen katmanın satırları çizime alınır ve seçime eklenir; tıklama, çizimin kendi seçimi gibi en yakın kenarı,
    yoksa noktayı tutan en küçük alanı alır;
  - soldan sağa çerçeve içinde kalanları, sağdan sola değenleri alır; 10 000 satırı geçen çerçeve sayısıyla reddedilir;
  - düzenlenmeyen etkin veri katmanında satırlar tablonun seçimine girer.
- **Kenet:** düzenlenen katman sadeleştirilmeden okunur. Okumanın satırları, 20 000'i geçmiyorsa, etiketlerin tuttuğu
  gibi küçük bir belgede tutulur. Çizim araçlarının keneti çizimin kendi keneti ile bu belgelerinkinin yakınını alır;
  satıra kenet çizimde nesne göstermez (`NO_OBJECT`).
- **Öznitelik tablosu:** düzenlenen katmanda hücre çift tıklanınca düzenlenir; Enter yazar, Esc bırakır. Alınmamış
  satır önce kaynaktan arka planda okunup alınır, sonra değeri yazılır. Her değer bir geri alma adımıdır
  (`cad.entities.set`). Tablo alınan satırları tampondaki değerleriyle gösterir, silinecekleri soluk gösterir; durum
  satırı kaydedilmemişleri sayar. Düzenlenmeyen katmanda çift tık nasıl başlatılacağını söyler.

## §6 Arayüz sözcükleri

- Düzenlemeyi başlat, Düzenlemeyi bitir, Değişiklikleri kaydet, Değişiklikleri geri al, Bütün katmanları kaydet, Bütün
  katmanları geri al.
- Soru: Kaydet, Kaydetme, Vazgeç. QGIS'in Türkçesindeki “Geri çağır” yerine “Geri al” kullanılır: çizimin geri almasıyla
  aynı anlamdadır ve kullanıcı “çağırmak” sözcüğünü bu işle bağdaştırmaz.

## §7 Kanıt

- `cargo test -p kentos-domain --test all data_edits`: 6 sınama:
  - yalnız veri katmanı açılır, salt okunur açılmaz;
  - düzenlenmeyen katmana nesne yazılmaz;
  - yeni, değişen, silinen satırlar; geri alma ve yineleme;
  - kaydetme yazdığını çıkarır, arada değişeni tutar;
  - geri alma ve bitirme;
  - dosya yeni satırları tutar, alınanları tablosuna bırakır.
- `KENTOS_CASES_ONLY=cad.layers.session … every_command_case_matches_the_desktop_handlers`: 29 durum; Python'ın tipli
  katmanından da (`cad.layers.rows` 28 durumuyla birlikte).
- Uygunluk takımı iki sürücüde de geçer; yeni denetimle birlikte: sürücünün yaptığı tabloda anahtarı kaynak verir.
- `cargo test -p kentos-desktop --lib data_`: 24 sınama. Yeni ikisi:
  - Katmanlar'dan açılan oturumda çizilen, Seç'le alınıp taşınan, anahtarla silinen satırlar kaydedilir, GeoPackage'ta
    okunur; bitirirken ve çizimden çıkarken sorulur;
  - Seç'in çerçevesi içinde kalanları ya da değenleri alır.
- Kenet ve tablo: `a_layer_being_edited_is_read_whole_and_kept_for_the_snap` (kenet satırın kendi köşesine düşer),
  `a_cell_of_an_edited_layer_takes_its_row_and_writes_the_value_into_the_session`. Seçim, kenet, tutamaç ve Bilgi'nin 12
  ortak izi her türevde geçti.
- Gerçek pencere (`scripts/usage/real/session.sh`, 1440×900; 1 000 000 parsellik PostGIS tablosu
  `public.perf_parseller`; resimler `.run/gui-evidence/veri/vy05/`):
  - Parseller'in sağ tuşunda Düzenlemeyi başlat; kaydedilecek bir şey yokken Kaydet ve Geri al kapalı.
  - Seç'in tıklaması 100/4'ü aldı ve seçti. Taşı onu 60 piksel (9,86 m) güneye kaydırdı. Eski yeri boşaldı; yeni hâli
    kesik turuncu çerçeveyle çizildi.
  - Kapalı alan ile çizilen yeni alan, stilinde sınıfı olmadığı hâlde kesik çerçeveyle göründü. 100/5 Delete ile
    silindi.
  - Katmanlar'ın ipucu: “Düzenleniyor; kaydedilmemiş: 1 yeni, 1 değişen, 1 silinen satır”; kalem turuncu.
  - Değişiklikleri kaydet: ““Parseller” kaydedildi — 1 yeni, 1 değişen, 1 silinen satır tabloya yazıldı.” Çizimde nesne
    kalmadı. Veritabanında 4'ün geometrisi 9,857 m güneyde, 5 yok. Tablonun anahtarının varsayılanı olmadığı için yeni
    satır 1 000 001 anahtarını aldı; satır sayısı yine 1 000 000.
  - Yeni bir alanla Düzenlemeyi bitir: soru “Parseller: 1 yeni satır”. Kaydetme değişikliği bıraktı; kalem kalktı.
  - Soldan sağa çerçeve 100/4 ile 100/5'i aldı (“2 seçili”); Delete ikisini sildi. Ctrl+O çizimden çıkmadan önce
    “Kaydedilmemiş katman değişiklikleri — Parseller: 2 silinen satır” diye sordu. Vazgeç; Değişiklikleri geri al ikisini
    geri getirdi, tablo değişmedi.
  - Kapalı alan aracında imleç 100/4'ün köşesine gelince “Uç nokta”: Y 587092,000 X 4400028,000, satırın kendi köşesi.
    Üst kenarın ortasındaki köşe noktası da (587105,000) kenet aldı.
  - Tabloda 3 numaralı satırın kullanim hücresi çift tıklanıp “Okul” yazıldı. Satır alındı; durum satırı “düzenleniyor;
    kaydedilmemiş: 1 değişen satır” dedi. Kaydedince veritabanında “Okul” oldu.

## §8 Kalan

- Kaydedilmemiş yeni satırlar tabloda listelenmez (tabloda değiller); durum satırı onları sayar.
- Değeri boş alan etiket şablonunda yer tutucusuyla görünür (“{ada}/{parsel}”); çizim katmanlarında da böyledir,
  etiket motorunun kuralıdır.
- İşlem kipleri ve başkasının değiştirdiği kayıtla çakışmanın söylenmesi VY-06'da.
- Alınan satırların dosyada saklanması (çakışma denetimiyle) VY-06'da.
- Çizimden sonra açılan öznitelik formu: bugün nesne şablonunun değerlerini alır; form açılması ayrı bir iş.
