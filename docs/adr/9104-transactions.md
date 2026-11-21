# ADR 9104: Veri katmanlarında işlem yönetimi — düzenleme grupları ve çakışma

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-06.
- **İstek:** sahibin sözü (20 Kasım 2026): “transaction yönetimi de qgis gibi ayarlanabilmeli”.
- **Önceki kararla ilişkisi:** [ADR 9103](9103-edit-session.md)'ün düzenleme oturumu bir katmanın değişikliklerini
  kendi işlemiyle yazar. Bu ADR katmanların birlikte açılıp kapanmasını, aynı kaynağın katmanlarının tek işlemle
  yazılmasını ve başkasının değiştirdiği kayıtta çakışmanın söylenmesini ekler.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** CBS verisi sürüm 3 (§2). **Komut:** `cad.settings.set`'in `editGrouping`'i,
  `cad.layers.session`'ın `force`'u ve `rows_changed` kodu.
- **Bağımlılık:** yeni dış kitaplık yok. Sürücü sözleşmesine `Transaction::rows` eklendi (§3).

## §1 Bağlam ve araştırma

**Masaüstü CBS** (QGIS 4.2.3; `src/core/qgis.h`'deki `Qgis::TransactionMode`, `src/app/qgisapp.cpp`; 21 Kasım 2026'da
okundu). Proje özellikleri › Veri kaynakları'nda üç kip vardır:

- **Yerel düzenleme tamponu** (`Disabled`): “Edits are buffered locally and sent to the provider when toggling layer
  editing mode.” Her katman kendi başına açılır ve kaydedilir.
- **Otomatik işlem grupları** (`AutomaticGroups`): “on supported datasources (postgres and geopackage databases) the edit
  state of all tables that originate from the same database are synchronized and executed in a server side transaction.”
  Aynı veritabanının katmanları birlikte düzenlemeye açılır. Değişiklikler sunucuda açık tutulan bir işleme gider,
  kaydetmede işlenir.
- **Tamponlu işlem grupları** (`BufferedGroups`): “all editable layers in the buffered transaction group are toggled
  synchronously and all edits are saved in a local edit buffer. Saving changes is executed within a single transaction on
  all layers (per provider).”

Başkasının bu arada değiştirdiği kayıt için yerel tamponda bir denetim yoktur: kaydetme anahtara göre yazar, sonra
yazan kazanır. Silinmiş bir kayda yazma sağlayıcının hatasıyla düşer.

**Türkiye mevzuatı:** Bilgi ve İletişim Güvenliği Rehberi (Siber Güvenlik Başkanlığı, sürüm 1.1, Mart 2026):

- 3.2.7.13: verilerin yanlışlıkla silinmesine karşı geri döndürme yolları önceden kurulur. Başkasının değişikliğinin
  habersizce silinmesi de bir veri kaybıdır.
- 3.2.7.1: en az yetki; yazma kullanıcının kendi bağlantısıyla yapılır.

Bir masaüstü programının işlem kipleri hakkında hüküm yoktur; aşağıdaki kararlar KentOS'undur.

## §2 Karar: düzenleme grubu

Proje ayarı `editGrouping` (`ProjectSettings::edit_grouping`):

| Kip | Arayüzdeki adı | Ne birlikte | Nasıl yazılır |
|---|---|---|---|
| `layer` (varsayılan) | Her katman ayrı | yalnız katman | katman başına bir işlem |
| `source` | Aynı kaynaktakiler birlikte | aynı GeoPackage'ın ya da veritabanının katmanları | kaynak başına bir işlem |
| `all` | Bütün katmanlar birlikte | düzenlenebilen bütün veri katmanları | kaynak başına bir işlem |

- `cad.layers.session`'da adı verilen bir katmanın `start`, `save`, `rollback`, `cancel`, `stop` ve `status`'u grubuna
  genişler (`data_write::grouped`). Salt okunur katman gruptan atlanır; adıyla verilirse reddedilir. Gruptaki bir katmanda
  kaydedilmemiş değişiklik varken `stop` reddedilir.
- `save` grubun aynı kaynaktaki katmanlarını tek işlemle yazar (`data_write::write_group`). Biri yazılamazsa hiçbiri
  yazılmaz. Masaüstü bitirirken grubun değişikli bütün katmanlarını tek soruda sorar.
- **Sunucuda açık işlem tutulmaz.** QGIS'in otomatik kipi kullanıcı düzenlerken bir veritabanı işlemini açık tutar.
  KentOS'ta `source` kipi aynı katmanları birlikte açar ve tek işlemle yazar, ama değişiklikleri yerelde tutar. Nedenleri:
  - açık işlem, kullanıcı düzenledikçe dakikalarca, saatlerce satır kilidi tutar ve öbür kullanıcıları bekletir;
  - havuzun bir bağlantısını o süre boyunca bağlar;
  - bağlantı koparsa yapılan her şey gider;
  - çizim ve tablo başka bağlantılardan okur; açık işlemin yazılmamış değişikliklerini görmezler.

  Bedeli: varsayılan değerler ve tetikleyiciler kaydetmede çalışır, düzenlerken değil.
- **`.kcad`:** ayar yalnız proje seçtiyse `settings.editGrouping` olarak yazılır. Yazıcı o zaman CBS verisi sürüm 3 yazar;
  belge şeması değişmez. CBS 2 yükünde anahtar bilinmeyen alandır. Örnekler: `edit-grouping.kcad`,
  `broken/edit-grouping-in-gis-2.kcad`, `broken/edit-grouping-unknown.kcad`; bağımsız okuyucu (`tools/kcad/kcad.py`) ve
  başvuru yazıcısı da güncellendi.
- **Komut:** `cad.settings.set`'in `editGrouping` anahtarı. Masaüstünde Proje ayarları › Genel › Veri katmanlarının
  düzenlenmesi › Birlikte düzenlenen katmanlar.

## §3 Karar: çakışma

Kaydetmede yazmadan önce, aynı işlemin içinde, alınmış her satır tablodan yeniden okunur. Okunan satır alındığı hâliyle
karşılaştırılır:

- **Okuma:** sürücü sözleşmesine `Transaction::rows(table, keys)` eklendi: satırları bütün alanları ve geometrinin
  tamamıyla okur, işlem bitene kadar tutar. PostGIS'te `SELECT … FOR UPDATE`; GeoPackage'ta işlem (`BEGIN IMMEDIATE`)
  dosyanın yazma kilidini zaten tutar. Uygunluk takımına denetimi eklendi; iki sürücü de geçer.
- **Karşılaştırma:** satır, alınırken olduğu gibi katmanın nesnesine çevrilir. Değerleri ve geometrisi alınan hâlinden
  farklıysa değiştirilmiş, yoksa silinmiş sayılır.
- **Sonuç:** hiçbir şey yazılmaz. Komut `rows_changed` çakışmasıyla döner (yeniden denenebilir). İleti hangi anahtarların
  değiştiğini ve silindiğini, sonra ne yapılabileceğini söyler (`rollback` ve `take`, ya da `force`); `uids` alınmış
  nesneleri verir.
- **Üstüne yaz:** `force: true` denetimi atlar. Yazma ADR 9103'teki gibidir: satırın yalnız çizimde değiştirilen
  alanları ve şekli yazılır. Başkası aynı alanı da değiştirdiyse onun değeri gider; başka alanlardaki değişikliği
  kalır. Silinmiş satıra yazma yine sürücünün hatasıyla düşer.
- **Masaüstü:** “Satırlar başkasınca değiştirilmiş” sorusu çıkar: Üstüne yaz / Vazgeç. Çakışan nesneler seçilir.
  Değişikliği bırakmak için Vazgeç, sonra katmanın menüsünde Değişiklikleri geri al: satırlar tablodaki şimdiki
  hâliyle görünür.
- Almadan, anahtarıyla silinen satırın alınmış bir hâli yoktur; onu yalnız varlığı denetlenir (sürücü silinecek
  satırı bulamazsa işlem düşer).

QGIS'te yerel tamponda böyle bir denetim yoktur; sonra yazan öncekinin değişikliğini habersizce siler.

## §4 Kanıt

- `KENTOS_CASES_ONLY=cad.layers.session … every_command_case_matches_the_desktop_handlers`: 34 durum. Yeni beşi:
  - aynı kaynaktakiler birlikte açılır, tek işlemle yazılır;
  - biri yazılamazsa hiçbiri yazılmaz;
  - bütün katmanlar birlikte açılır ve kapanır;
  - başkasının değiştirdiği satırda çakışma; üstüne yazınca yalnız değiştirilen alan yazılır, başkasının öbür
    alandaki değişikliği kalır;
  - başkasının sildiği satırda çakışma.
- `cad.settings.set`: 146 durum (yeni: düzenleme grubu).
- `cargo test -p kentos-kcad`: 281 sınama; geçerli örnek sayısı 288, bağımsız başvuru dosyalarıyla bayt bayt aynı.
- Uygunluk takımı (`Transaction::rows` dahil) GeoPackage'ta ve yalıtılmış PostGIS'te geçer.
- Masaüstü: `cargo test -p kentos-desktop --lib data_`, 26 sınama. Hücre sınamasına eklendi: kayıttan sonra tablo
  bulunduğu yeri yeniden okur, okuma sürerken eski satırlar görünür kalır.
- Gerçek pencere (`scripts/usage/real/session.sh`), bir milyon satırlık yalıtılmış PostGIS tablosu ve bir GeoPackage:
  - Proje ayarları › Genel'de “Birlikte düzenlenen katmanlar” seçimi görünür.
  - 100/4 parseli Seç'le alındı. Ardından başka bir bağlantı `kullanim`'ını “Ticaret” yaptı. Parsel taşınıp kaydedildi.
    “Satırlar başkasınca değiştirilmiş (değiştirilmiş: 4)” sorusu çıktı, tablo değişmedi.
  - Üstüne yaz: şekil 9,857 m aşağı yazıldı, `kullanim` “Ticaret” kaldı. Harita ve Öznitelik tablosu satırı
    tablodaki hâliyle gösterdi.
  - Bu sınamada bulunup düzeltildi: kayıttan sonra tablo eski değeri (“Eğitim”) göstermeye devam ediyordu. Artık
    çizimin yazma sayacı (`Document::data_written`) değişince tablo yeniden okunur.
  - “Aynı kaynaktakiler birlikte” seçiliyken Yollar düzenlemeye açıldı: aynı GeoPackage'taki Direkler de açıldı,
    PostGIS'teki Parseller açılmadı. Direkler bitirilince ikisi birlikte kapandı.

## §5 Kalan

- ~~Alınan satırların proje dosyasında saklanması.~~ Kapandı (VY-14,
  [ADR 9111](9111-editing-pending-with-the-project.md) §3): bu cihazda projenin yanında saklanır; geri gelen satır §3'ün
  çakışma denetiminden geçer.
- Kaydetme sırasında varsayılan değerlerin ve tetikleyicilerin sonucunun düzenlerken görünmesi (sunucuda açık işlem)
  bilerek yapılmadı (§2).
