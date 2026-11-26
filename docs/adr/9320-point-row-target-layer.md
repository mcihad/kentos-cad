# ADR 9320: Nokta satırının katmanı baştan bulunur ve gösterilir

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  11, P1). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 0153](0153-point-editor.md) (Noktalar), [ADR 0622](0622-interface-arrangement.md) §5 (katman
  türü), [ADR 9103](9103-edit-session.md) (düzenleme oturumu), [ADR 9107](9107-project-in-a-source.md) §7 (CBS'de
  yeni katman tablodur), [ADR 9196](9196-storage-by-project-kind.md) (saklama modeli).

## Bağlam

- **Kusur:** yeni CBS projesinde tek katman Açıklama etkinken Ölçme › Noktalar › Satır ekle ile Ad, Y, X, kot ve kod
  yazılıp Enter'a basıldı.
  - Değerler kabul edildi, yazma `layer_geometry_mismatch` ile reddedildi, nesne sayısı 0 kaldı.
  - Satır hedef katmanını göstermiyordu; asıl neden yalnız Komut geçmişinde görüldü.
- **Kod:** `AddRow` doğrudan taslak açıyordu. `write_draft` Enter'da etkin katmana yazmayı deniyordu; uygun katman olup
  olmadığı ancak yazma reddedilince anlaşılıyordu.
- **Yanlış yol:** ret iletisi katmanı Karışık yapmayı öneriyordu. Ayrı geometri türlü tablolarla çalışan CBS projesinde
  bu kötü bir kaçış yoludur (ADR 9196 §2: CBS'nin nesneleri tek türlü tabloların satırlarıdır).

## Araştırma

- **ArcGIS Pro:** Create Features bölmesi görünmeyen ya da düzenlenemeyen katmanların şablonlarını gizler ve bunu
  bölmenin başındaki bildirimle söyler. Şablonu olmayan katman için şablon yapmayı önerir
  ([şablon uyarıları](https://pro.arcgis.com/en/pro-app/3.4/help/editing/feature-template-messages.htm)).
  - Hedef, nesne çizilmeden önce seçilir.
- **QGIS:** katmanlar öntanımlı olarak salt okunur açılır. Nesne ekleme araçları Düzenlemeyi aç/kapat'tan sonra çalışır;
  veri sağlayıcısı ve dosya yazılabilir olmalıdır
  ([düzenleme](https://docs.qgis.org/3.10/en/docs/user_manual/working_with_vector/editing_geometry_attributes.html)).
- **KentOS kararı:** satırın katmanı, satır açılırken bulunur ve tablonun üstünde gösterilir. Satırı alamayan katman
  listelenmez. Hiç katman yoksa ya da oturum kapalıysa çıkış yolu aynı satırdadır. Yazılan değerler satırda kalır.

## Karar

### 1. Satırın katmanı

- **CAD projesi (Harita, Yol):** nokta alan çizim katmanı (nokta türünde ya da karışık), kilitli değil.
- **CBS projesi:** yazılabilir veri katmanı, nokta tablosu ya da her geometriyi alan tablo, satırları yazı değil,
  kilitli değil.
  - Salt okunur bağlantı (sunucunun yetkisi ya da kullanıcının seçimi) listelenmez.
  - Oturumu kapalı tablo listelenir; satır bunu söyler.
  - CBS'nin çizim katmanları nesne tutmaz (ADR 9196 §2).
- **Seçim sırası:**
  1. satırın listesinde seçilen (hâlâ uygunsa);
  2. etkin katman, yazmaya açıksa;
  3. yazmaya açık ilk katman;
  4. etkin katman;
  5. ilk uygun katman.
- **Satır yazılınca** sonraki satır aynı seçimi taşır, adı bir artar.

### 2. Tablonun üstündeki satır

Satır ekle açıkken bar ile tablo arasında “Yeni satırın katmanı” satırı durur. Taslak satırın Katman hücresi de aynı
katmanı adlandırır. Satırda şu durumlardan biri gösterilir:

- **Uygun katman yok:** “Nokta yazılabilecek katman yok.” ve Nokta katmanı oluştur düğmesi.
- **Oturumu kapalı tablo:** liste, “Düzenlemeye kapalı; satır yazılmaz.” ve Düzenlemeyi başlat düğmesi.
- **Yazmaya açık tablo:** liste ve “Enter satırı katmanın düzenlemesine yazar; tabloya Değişiklikleri kaydet ile geçer.”
- **Çizim katmanı:** liste ve “Enter satırı bu katmana yazar.”

Düğmeler ve liste, açık hücreye yazılanı önce satıra alır.

### 3. Satır açılırken ve Enter'da

- **Satır ekle:** uygun katman yoksa ya da oturum kapalıysa bu, satır açılırken bir kez söylenir; satır yine açılır.
- **Enter:**
  - Katman yoksa ya da oturum kapalıysa nedeni ve yolu söylenir: “… Yazdıklarınız satırda duruyor.” Hiçbir şey
    yazılmaz, hücre açık kalır.
  - Ad, Y, X, kot ve kod kaybolmaz; katman sonradan bulunduğunda Enter onları yazar.

### 4. Nokta katmanı oluştur

- **CBS:** yeni katman penceresi nokta tablosu için hazır açılır: `ad` ve `kod` metin sütunları, geometride kot (Z).
  Oluştur'dan sonra tablo satırın katmanı olur ve düzenleme oturumu açılır.
- **CAD:** nokta türünde çizim katmanı hemen eklenir ve satırın katmanı olur.

### 5. Veri katmanında ad, kod ve kot

- **Ad ve kod:** tablonun bu adlı sütunlarına yazılır. Sütun, katlanmış adı `ad`, `adı`, `adi` ya da `name` (kod için
  `kod` ya da `code`) olan ilk alandır; büyük ya da küçük harf fark etmez.
  - Taslaktaki nesnenin adı (`label`) ve `Kod` özniteliği Noktalar tablosunda görünmek için kalır.
- **Kot:** geometrinin Z'sine yazılır.
- **Yeri olmayan değer:** satır yazıldıktan sonra söylenir: “… tablosunda ad sütunu, kod sütunu, geometride kot (Z)
  yok; değişiklikler kaydedilince bunlar tabloya yazılmaz.” Eksik bir sütunsa “Sütunu Alanlar'da ekleyebilirsiniz.”
  eklenir.

### 6. Ret iletisi CBS'de Karışık önermez

- **Ortak denetim (`integrity.rs`):** tür uyuşmazlığında CBS projesinde “Nesneyi türüne uyan bir veri katmanına yazın.”
  der. CAD projesinde Karışık önerisi kalır.
  - Bu ortak denetim olduğu için masaüstü, Python, MCP ve ajan aynı iletiyi alır.
- **Masaüstünde etkin katmanın uyuşmazlığı (`layer_types::misfit`):** CBS'de türüne uyan veri katmanını adlandırır, yoksa
  Veri ekle ve Yeni katman'ı söyler.
- **CAD'de yalnız açıklama katmanı varken:** Katmanlar panelinde Yeni katman ile türüne uyan bir katman eklenmesini
  söyler; açıklama katmanını karışık yapmayı önermez.

### 7. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Satırın katmanı baştan, tablonun üstünde | yok: Noktalar sekmesi yok | var: çizim katmanı | var: çizim katmanı | var: veri katmanı, oturumu ve yetkisiyle |
| Nokta katmanı oluştur | yok: aynı gerekçe | var: nokta türünde çizim katmanı | var: aynı | var: `ad`, `kod`, Z ile nokta tablosu |
| Veri katmanında ad, kod, kot sütunları | yok: veri katmanı CBS'nindir | yok: aynı gerekçe | yok: aynı gerekçe | var |
| Ret iletisinde Karışık önerisi | var (CAD) | var (CAD) | var (CAD) | yok: veri katmanı önerilir |

### 8. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | `cad.point.create` ile, `layerId` ve tablonun sütunlarıyla (`attrs`). Aynı ortak denetimden geçer: CBS'de veri katmanı önerilir (`test_layer_types.py`). Satırın görünür seçimi masaüstünün sunumudur; komut aynıdır. |
| MCP | var | Aynı komut ve aynı denetim. |
| Yapay zekâ ajanı | var | Aynı komut ve aynı denetim. |
| Eklenti | var | `document.write` izniyle aynı komut. |

## Açık

- Noktalar sekmesi yalnız çizimdeki nesneleri listeler. Veri katmanının kaydedilmiş, alınmamış satırları burada
  görünmez; satır tablodan ve öznitelik tablosundan okunur.
- CBS'nin Nokta aracında verilen adın tablonun `ad` sütununa yazılması ayrı bir karardır; bu madde yalnız Noktalar'ın
  satırını kapsar.
- Tek noktaya yakınlaşan görünümün ölçeği “1:0” gösteriyor (incelemenin 8. maddesi).
- Gerçek pencerede fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü çekirdeği:** `cargo test -p kentos-desktop --lib -- points:: new_layer_reveal wizard::tests geometry_tests
  gpkg_tests layer_types module_tests ribbon_tests catalog history::text settings`: 257 geçti; tek kırmızı aşağıdaki
  bağımlılık tabanı.
  - `points::target_tests` (5 sınama):
    - yeni CBS projesinde satır açılırken katmanın olmadığı söyleniyor, Enter hiçbir şey yazmıyor ve değerler kalıyor;
    - Nokta katmanı oluştur `ad`, `kod`, Z'li tabloyu yapıyor, oturumu açıyor, satırı yazıyor;
    - Değişiklikleri kaydet'ten sonra GeoPackage uygulamadan ayrı okunuyor: `ad`=QA-A, `kod`=QA, (500000, 4320000, 12.5);
    - oturumu kapalı tabloda Düzenlemeyi başlat ve yeri olmayan değerlerin iletisi çalışıyor;
    - salt okunur kaynak listelenmiyor;
    - CAD'de seçilen katman ve sonraki satır doğru; nokta katmanı yokken katman yapılıp satır yazılıyor.
  - `target::tests`: sütun adları büyük ya da küçük harfle bulunuyor.
- **CBS paketi:** `cargo test -p kentos-desktop-cbs points_target_tab_tests`: CBS'nin Noktalar sekmesi uyarıdan sonra
  ekranda kalıyor.
- **Ortak çekirdek:** `cargo test -p kentos-native-application --test all -- layer_types` (yeni sınama: CBS'de veri
  katmanı, CAD'de Karışık) ve `cargo test -p kentos-domain`.
- **Python:** `python/tests/test_layer_types.py`, yerel modül yeniden derlendikten sonra (3 sınama).
- **Resim** (`.run/gui-evidence/kullanim-9320/`, açık ve koyu tema, CBS paketinin sınamasından; büyütülerek bakıldı):
  - `01-katman-yok`: değerleri yazılmış satır, üstünde Nokta katmanı oluştur;
  - `02-nokta-tablosu-penceresi`: `ad`, `kod`, Z ile hazır pencere;
  - `03-satir-tabloya-yazildi`: yazılan satır ve sonraki satırın katmanı;
  - `04-duzenlemeye-kapali`: Düzenlemeyi başlat, dar pencerede tek satır.
- **Önceden kırmızı, bu işin değil:** `scripts/dev/module_deps.py --check`'in kabuk → CAD ve CBS → CAD tabanı
  (`shell/approved_layer.rs`, `shell/gpu_frame.rs`, `gis/data_selection.rs`).
