# ADR 9198: Katmanın mobildeki davranışı: göster, düzenle, gösterme

- **Durum:** kabul edildi (2026-11-23); plan [plan/ayrim.md](../../plan/ayrim.md) AY-08. [ADR 9197](9197-mobile-clients.md)
  §2.5'i uygular: mobilde katmanın davranışını masaüstündeki proje belirler.
- **İstek:** sahibin sözü (23 Kasım 2026): “Mobil görüntüleyici ve uygulama da geliştireceğiz ileride ona göre de ayrım
  yap”.
- **Kural:** CLAUDE.md §25 kural 2 (her yüzey her modülde açıkça yerleşir), 4 (tanım ortaktır), 6 (veri yalnız
  komutla değişir), 7 (görüntüleyici yazmaz).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** şema 256.

## §1 Araştırma (23 Kasım 2026'da okundu)

- **QFieldSync** (QGIS'in mobil istemcisinin eklentisi): her katmanın mobildeki eylemi projenin katman ayarıdır:
  çevrimdışı düzenleme, kopya, projeden çıkar ya da kaynağa doğrudan erişim. Bulut projeyi mobile paketlerken uygular.
  <https://docs.qfield.org/get-started/tutorials/get-started-qfs/>
- **Esri, *ArcGIS Field Maps: Configure the map*:** haritanın yeteneği düzenlenebilir ya da salt okunurdur; harita
  mobil uygulamadan gizlenebilir.
  <https://doc.esri.com/en/arcgis-field-maps/12.2/prepare-maps/configure-the-map.html>
- **Mergin Maps, *GIS features*:** anket katmanı olmayan vektör katman salt okunur yapılır; salt okunur katman mobilde
  düzenlenmez. <https://merginmaps.com/docs/gis/features/>

**Ortak örüntü:** katmanın mobildeki yeri masaüstündeki projenin katman ayarıdır; seçenekler “göster (salt okunur)”,
“düzenle” ve “gösterme”dir; düzenleme yalnız mobil istemcinin yazabildiği veride açılır.

## §2 Karar

1. **Katmanın alanı** `mobile` (`LayerNode.mobile`): `show` (gösterilir, salt okunur), `edit` (mobil uygulama
   düzenler), `hide` (mobilde yoktur). Alan yoksa katman gösterilir; dosyaya yalnız `edit` ve `hide` yazılır. Gruba da
   verilir: gizlenen grubun altındaki her şey gizlenir, altındaki `edit` de. Mobil istemci ağacı ortak işlevle okur
   (`kentos_contracts::layer_on_mobile`).
2. **Kim neyi seçer ortak tanımdadır** (`kentos_launch::LayerKind::mobile_choices`): `edit` yalnız mobil uygulamanın
   yazdığı katman türündedir (`LayerKind::written_by(Client::MobileApp, storage)`, ADR 9197 §2.4): CBS'nin veri
   tablosu. Öbür türler göster ya da gösterme alır. `catalog/v1/modules.json` her katman türünün seçeneklerini saklama
   türüne göre yazar.
3. **Mobil görüntüleyici** göster ve düzenle katmanlarını salt okunur gösterir, gösterme katmanını göstermez (kural 7).
4. **Komut:** `cad.layers.set`'in `mobile` işlemi (`layer`, `mobile`). Yazdır gibi projenin ayarıdır: çizim değişir,
   geri alma adımı değildir. Seçilemeyen seçenek `mobile_not_offered` ile, nedeni söylenerek reddedilir; `mobile`
   verilmezse `no_value`.
5. **Masaüstü:** katman menüsünde “Mobilde ▸ Göster / Düzenle / Gösterme” (`LayerEntry::Mobile`); her modülün
   profilinde açıkça yer alır, seçenekleri katmanın türünden ve projenin saklama türünden gelir.
6. **Python ve MCP** aynı komutu kullanır.
7. **Saklama türü ve seçenekler tek yerde:** komut da masaüstü menüsü de aynı işlevi sorar
   (`kentos_native_application::layer_kind::{storage, mobile_choices}`); istemci başına ayrı karar yazılmaz (§25 kural
   4).

## §3 Uygulama (23 Kasım 2026)

- **Ortak tanım** (`kentos-launch`): `Mobile` (`ALL`, `id`, `title`: Göster, Düzenle, Gösterme) ve
  `LayerKind::mobile_choices(storage)`. `catalog/v1/modules.json` her katman türünün seçeneklerini saklama türüne göre
  yazar (`mobile.database`, `mobile.file`): yalnız CBS'nin veri tablosu `edit` alır.
- **Sözleşme** (`kentos-contracts`): `LayerMobile`, `LayerNode.mobile`, `layer_on_mobile`. TS türleri (`LayerMobile`,
  `LayerNode`, `LayersSet`, `LayerSetOperation`), OpenAPI ve Python SDK yeniden üretildi.
- **`.kcad` şema 256** (`SCHEMA_WITH_LAYER_MOBILE`): katmanın `mobile` anahtarı yalnız `edit` ya da `hide`'dır;
  dosyadaki `show` ve bilinmeyen değer `bad_value`, şema 255'te bu alan `unknown_field`'dır
  (`docs/specs/kcad-v2.md`).
  - Bağımsız okuyucu `tools/kcad/kcad.py` ve üretici `scripts/fixtures/kcad_v2_reference.py` aynı kuralla.
  - Örnekler: `fixtures/kcad/v2/layer-mobile.kcad`; bozuklar `layer-mobile-in-schema-255`, `-show`, `-unknown`;
    `schema-version-256.kcad` → `schema-version-257.kcad`.
- **Komut** `cad.layers.set` işlemi `mobile` (`kentos_domain::Document::set_layer_mobile`): projenin ayarıdır, sürümü
  değiştirir, geri alma adımı değildir.
  - Retler: `no_value` (`mobile` yok), `mobile_not_offered` (nedeni ve yolu söylenir: “… mobilde düzenlenemez: mobil
    uygulama yalnız veri tablosunun satırlarını yazar. Göster ya da gösterme seçin.”).
  - Ortak durumlar: `fixtures/commands/v1/cad.layers.set.json`'a 21 durum (üretici
    `scripts/fixtures/layer_tree_command_cases.py`; Rust, Python ve masaüstü işleyicileri oynatır):
    - gösterme: geri alma adımı değildir; grup; servis katmanı; kilitli katman ve grup (kilit nesnelerindir); masaüstünde
      gizli katman; grubun içindeki katman; aynı seçim ikinci kez; doğrula yazmaz; sürüm çakışması;
    - göster: alanı kaldırır, zaten gösterilen değişmez;
    - düzenle: CBS'nin veri katmanında kabul ve sonra gösterme; CAD projesinin katmanında, CAD projesinin altlık veri
      tablosunda, CBS'nin açıklama katmanında, grubunda ve servis katmanında ret;
    - retler: değersiz, katmansız, olmayan katman.
- **Masaüstü:** katman menüsünde Görünürlük ▸ Mobilde ▸ (`LayerEntry::Mobile`, 43. öğe): seçenekler katmanın türünden
  ve projenin saklama türünden gelir, seçili olan işaretlidir. Dört modülün profili öğeyi açıkça yerleştirir; tablo
  sınaması 9 × 8 × 43 hücre.
- **Python ve MCP** aynı komutu kullanır; SDK'nın `cad.layers.set`'i `mobile` değişkenini alır.
- **Sınama:** `kentos-launch` (`only_a_cbs_data_table_is_edited_on_the_phone`), `kentos-contracts`
  (`a_hidden_group_hides_what_it_holds_on_the_phone`), `kentos-native-application` (`layer_kind`; ortak durumlar),
  `kentos-kcad` (şema 256 ve örnekler), masaüstü `layer_menu_tests` (`the_phone_edits_a_cbs_data_layer_alone`) ve
  `module_catalog_tests`.
- **Resim:** `.run/gui-evidence/kullanim-9198/`: CBS veri tablosunda Göster, Düzenle, Gösterme
  (`mobilde-cbs-veri-tablosu.png`); Harita'nın çizim katmanında Göster, Gösterme (`mobilde-harita-çizim.png`).

## §4 Kalan

- Mobil istemcinin kendisi: modül profilleri (hangi panel ve araç), paketleme ve okuma (`layer_on_mobile`).
- CBS'de çevrimdışı düzenleme: değişiklik günlüğü ve eşitlemede sunucu komutları.
- Mobilde CAD düzenleme kararı; mobil uygulamanın teknolojisi (ADR 9197 §4).
- Web'in katman menüsünde Mobilde (TODOS.md §0.2).
