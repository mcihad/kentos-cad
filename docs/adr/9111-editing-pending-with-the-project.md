# ADR 9111: Düzenleme — tabloda yeni satırlar, çizimden sonra alanlar, bekleyen düzenlemeler projeyle

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-14 (ikinci tur).
- **İstek:** sahibin sözü (21 Kasım 2026): “Hepsini düzelt eksikleri tamamla.” Kapatılan kalanlar:
  - ADR 9103 §8: kaydedilmemiş yeni satırlar tabloda listelenmiyor; alınan satırlar dosyada saklanmıyor; çizimden sonra
    form açılması ayrı bir iş.
  - ADR 9104 §5: alınan satırların proje dosyasında saklanması (çakışma denetimiyle).
- **Sözleşmeler:** komut ve `.kcad` değişmedi. Yeni olan, cihazdaki bekleyen düzenleme günlüğünün biçimi (v1, §3).
  `kentos-ui`'nin `Confirm`'ü dördüncü bir cevap alır (`also`). **Bağımlılık:** yok.

## §1 Karar: kaydedilmemiş yeni satırlar tablonun başında

**Sorun:** tablo satırları kaynaktan okur. Düzenlenen katmana çizilen yeni satır kaynakta henüz yok, bu yüzden tabloda
görünmüyordu. Durum satırı onları yalnız sayıyordu.

**Masaüstü CBS:** QGIS'te düzenleme kipinde eklenen detay öznitelik tablosunda hemen görünür. Kaydedilene kadar eksi bir
detay kimliği taşır (QGIS'in `FID_AS_NEW` kuralı).

**Karar:**

- Yeni satırlar tablonun başında, çizimdeki sıralarıyla durur. Anahtar hücresinde sarı “yeni” yazar: kayıttan önce
  anahtarları yoktur. Eksi sayı uydurulmaz, çünkü kayıt sonrası anahtarla karışabilir.
- Kaynağın satırları onların altından kayar; sayfa okuması yeni satırların sayısı kadar kaydırılarak sorulur.
- Yeni satıra basmak nesnesini çizimde seçer. Öznitelikler paneli de onu gösterir.
- Değerleri Öznitelikler'de yazılır. Tablodaki çift tık kaynağın satırları içindir.

## §2 Karar: çizilen satırın alanları çizimden sonra açılır

**Masaüstü CBS:** QGIS bir detay çizilince öznitelik formunu açar. Seçenekler › Sayısallaştırma'daki “Suppress
attribute form pop-up after feature creation” bunu kapatır, katmanın Öznitelik formu sekmesi de genel ayarı ezer. Alanı
olmayan katmanda form açılmaz.

**Karar:**

- Katmanın yayımlanmış formu varsa form açılır (ADR 0403 §4, değişmedi).
- Formu olmayan ama alanları olan, düzenlenen bir veri katmanında, çizim aracının yazdığı tek yeni satır seçilir ve
  Öznitelikler paneli sağda açılır. Değerler hemen yazılır.
- İkisini de aynı ayar kapatır: Çizim › “Çizimden sonra formu aç” (`drafting.formOnCreate`, varsayılan açık).
- Alanı olmayan katmanda hiçbir şey açılmaz.

## §3 Karar: bekleyen düzenlemeler projeyle saklanır, açılınca geri gelir

**Sorun:** dosya yeni satırları katmanın nesneleri olarak tutuyor. Alınan satırları ve almadan silinen anahtarları
tutmuyor (ADR 9103 §2). Düzenleme sürerken kaydedilen proje oturumun yarısını saklıyordu. Kapatırken soru yalnız
Kaydet ya da Kaydetme diyordu, yani bir düzenleme ertesi güne kalamıyordu.

İkinci bir açık da buldum. Proje kaydedildikten sonra yeni satırlar tabloya yazılırsa, çizimden dışarıdan bir
değişiklik gibi çıkıyordu. Bu bir düzenleme sayılmadığı için çizim kaydedilmemiş olmuyordu ve dosya o satırları hâlâ
yeni satır olarak tutuyordu. Proje yeniden açılıp kaydedildiğinde aynı satırlar tabloya **ikinci kez** yazılıyordu.

**Masaüstü CBS:** QGIS düzenleme tamponunu proje dosyasında saklamaz; kapatırken her katman için kaydet ya da bırak
diye sorar. Saklanan düzenleme Çevrimdışı Düzenleme eklentisinde vardır: veri yerel bir GeoPackage'a kopyalanır,
düzenlemeler ayrı tablolarda tutulur, bağlantı gelince Eşitle ile kaynağa yazılır.

**Mevzuat:**

- 6698 md. 12, veri güvenliği: alınan satırlar tablonun verisidir ve kişisel veri içerebilir (malik adı, TC kimlik
  no). Proje dosyası elden ele, e-postayla, portala gider. ADR 9103 §1 bu yüzden onları dosyaya yazmıyordu; bu karar
  değişmedi.
- Kişisel Verilerin Silinmesi, Yok Edilmesi veya Anonim Hale Getirilmesi Hakkında Yönetmelik md. 7: işleme sebebi
  kalkınca veri silinir. Günlük de bir satır yazılınca ya da bırakılınca kendiliğinden silinir.

**Karar:**

- **Günlük cihazda, dosyanın yanında değil.** Kullanıcının kendi durum klasöründe, projenin yerine bağlı bir dosya
  tutulur:
  - Windows: `%LOCALAPPDATA%\kentos-cad\veri-oturumlari`;
  - macOS: `~/Library/Application Support/kentos-cad/veri-oturumlari`;
  - öbürleri: `$XDG_STATE_HOME/kentos-cad/veri-oturumlari`.

  Proje yeri bir `.kcad` dosyası ya da veri kaynağındaki proje olabilir (ADR 9107). Dosyanın adı yerin FNV-1a özetidir;
  sürümden sürüme değişmez. Klasör 0700, dosya 0600 izinlidir: yalnız kullanıcı okur. Dosya önce ayrı yazılıp eskisinin
  üstüne adlandırılır.
- **İçinde ne var:** her katmanın alınan satırları (nesnenin şimdiki hâli ya da silindiği, satırın geldiği hâli), almadan
  silinen anahtarlar ve projenin kalıcı kimliği. Bir de dosyanın tuttuğu ama sonra tabloya yazılan ya da bırakılan
  nesnelerin kimlikleri (`gone`). Değişmemiş alınan satır yazılmaz: bekleyen bir şeyi yoktur.
- **Ne zaman yazılır:** proje kaydedilince, kaydın başladığı anki çizimden. Bekleyen bir şey yoksa günlük silinir.
  Kopyasını kaydet, bulut projesi ve kurtarma kopyası günlük tutmaz.
- **Tabloyu izler:** bir kayıt ya da geri alma tabloyu değiştirince (`Document::data_written`):
  - yazılan ya da bırakılan satırlar ve anahtarlar günlükten çıkar;
  - çizimden ayrılan nesneler sırayla `Document::data_left`'e yazılır ve günlüğün `gone` listesine geçer.

  Proje bir kayıt sürerken tabloya yazılsa da doğru kalır, çünkü sayaçlar kaydın başladığı andan alınır.
- **Açılınca:**
  - `gone`'daki nesneler çizime alınmaz (`Document::drop_left`): tabloya ikinci kez yazılmazlar. Nesnesi kalmayan katman
    düzenlemeden çıkar.
  - Alınan satırlar kendi kalıcı kimlikleriyle nesne olur, silinen anahtarlar geri gelir (`Document::restore_pending`).
    Bu bir düzenleme değildir: geri alma adımı oluşmaz, çizim kaydedilmiş kalır.
  - Ne geldiği söylenir: “Bekleyen veri düzenlemeleri geri geldi (Parseller: 1 yeni, 1 değişen, 1 silinen satır)…”.
  - Geri getirilemeyen satır ya da katman ayrı bir cümleyle söylenir: katman salt okunur olmuşsa, satır çizimde zaten
    varsa.
- **Çakışma:** geri gelen satır, geldiği hâliyle tabloya karşı denetlenir (ADR 9104 §3). Bu arada başkası değiştirdiyse
  kayıt önce sorar: Üstüne yaz ya da Vazgeç.
- **Başka projenin günlüğü:** aynı yere başka bir proje kaydedilmişse (kalıcı kimlik farklı) günlük kullanılmaz ve
  silinir; bu söylenir.
- **Projede tut:** çizimden çıkarken sorulan “Kaydedilmemiş katman değişiklikleri” sorusuna dördüncü bir cevap eklendi.
  - Değişiklikler tabloya yazılmaz.
  - Ardından çizimin kendi sorusu gelir. Kaydedilirse değişiklikler projeyle saklanır.
  - Çizim zaten kaydedilmişse günlük günceldir ve çizim kapanır.
  - Bulut projesinde bu cevap yoktur.
  - Düzenlemeyi bitir sorusunda da yoktur: orada düzenleme biter.

## §4 Kanıt

**Sınamalar:**

- Alan katmanı (`kentos-domain`, `data_edits`, 8 sınama):
  - dosyanın bıraktıkları dosyayla geri gelir: değişen, silinen ve anahtarla silinen satır; kaydetme aynı değişiklikleri
    bulur;
  - ikinci kez geri getirme satır satır reddedilir;
  - salt okunur katman söylenir;
  - dosya kaydından sonra yazılan ve bırakılan yeni satır yeniden açılan çizime alınmaz.
- Masaüstü (`data_journal_tests`):
  - Yeni satır tabloda; basınca seçilir.
  - Formsuz veri katmanında çizimden sonra Öznitelikler açılır; ayar kapatınca açılmaz.
  - Günlükle tam akış:
    1. Kaydet: dosyada “Okul” yok; günlükte 22 anahtarlı satır var; dosya 0600.
    2. Projede tut ile kapanır.
    3. Açılınca 1 yeni, 1 değişen, 1 silinen satır geri gelir; çizim kaydedilmiş kalır.
    4. Başka bir bağlantı 22'yi değiştirir; kayıt çakışmayı sorar.
    5. Üstüne yaz ile yazılır. Günlükte bekleyen kalmaz, yeni satırın kimliği `gone`'a geçer.
    6. Proje kaydedilmeden açılınca yeni satır ikinci kez gelmez.
    7. Proje kaydedilince günlük silinir.
- Masaüstünün veri, kayıt, açılış, sekme, form, stil, kurtarma ve ayrılış sınamaları geçti (213).
- `kentos-ui`'nin soru penceresi sınamaları geçti.

**Gerçek pencere:** bir milyon satırlık yalıtılmış PostGIS tablosu (Parseller).

- **Önce** (VY-05'in görüntüleri):
  - çizilen alan tabloda yoktu, Öznitelikler “Seçili nesne yok” diyordu;
  - çıkarken soru yalnız Kaydetme, Vazgeç, Kaydet sunuyordu.
- **Sonra:**
  - Çizilen alan seçili geldi; Öznitelikler'in Alanlar bölümünde ada, parsel, mahalle, alan ve kullanim boş
    bekliyordu. Tablonun başında seçili “yeni” satırı vardı.
  - 100/4 alınıp taşındı, 100/5 silindi. Durum satırı “kaydedilmemiş: 1 yeni, 1 değişen, 1 silinen satır” dedi.
  - Ctrl+S'ten sonra komut geçmişinde “Kaydedildi: …veri-katmanlari.kcad.”, ardından “Tabloya yazılmamış 2 satır
    değişikliği projeyle bu cihazda saklandı; proje açılınca geri gelir.” yazdı.
  - Çizimi kapatırken soru dört cevaplıydı. Projede tut çizimi kapattı.
  - Son dosyalar'dan yeniden açılınca “Bekleyen veri düzenlemeleri geri geldi (Parseller: 1 yeni, 1 değişen, 1 silinen
    satır)…” yazdı. Tablo yine “yeni” satırı ve aynı durum satırını gösterdi.
  - O arada başka bir bağlantı 4 numaralı parselin `kullanim`'ını “Ticaret” yaptı. Katmanın Kaydet'i “Satırlar
    başkasınca değiştirilmiş (değiştirilmiş: 4); hiçbir şey yazılmadı” diye sordu.
  - Değişiklikleri geri al'dan sonra tablo 4'ü “Ticaret” gösterdi: “kaydedilmemiş: değişiklik yok”.
  - Tablo sonra eski hâline getirildi: 1 000 000 satır, en büyük anahtar 1 000 000.
- Görüntüler `.run/gui-evidence/veri/vy14/{once,sonra}`.

## Kalan

- Kurtarma kopyası (ADR 0030) alınan satırları tutmaz: çökme sonrası geri yüklenen çizim yalnız yeni satırlarla gelir.
- Taşınan ya da yeniden adlandırılan `.kcad` günlüğünü bulamaz, çünkü günlük yerine bağlıdır. Bekleyenler eski yerin
  günlüğünde kalır.
- Hiç açılmayan projenin günlüğü için süre sınırı yok.

## Uygulama

- Alan katmanı: `crates/native/domain/src/data_pending.rs` (yeni: `pending_rows`, `restore_pending`, `drop_left`,
  `data_left`, `taken_row`, `deleted_keys`); `data_edits.rs` (`data_left` kaydı).
- Masaüstü:
  - `apps/desktop/src/gis/data_journal.rs` (yeni);
  - `features/data.rs` (yeni satırlar, `PressNew`);
  - `forms/mod.rs` (`edited_with_fields`);
  - `data_edits.rs` (`Answer::Keep`);
  - `shell/saving.rs` (`Saving::kept`), `shell/app/update.rs` (açılış, her mesajdan sonra);
  - `shell/document.rs` (`Document::kept`), `launch.rs`.
- Arayüz: `crates/ui/src/widget/dialog.rs` (`Confirm::also`).
