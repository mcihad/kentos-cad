# ADR 9201: Veri katmanının düzenleme oturumu şeritte; katman menüsünde yok

- **Durum:** kabul edildi (2026-11-24). [ADR 9103](9103-edit-session.md) §5'in “Katmanlar menüsünden açılır”
  yolunu ve [ADR 9169](9169-session-in-the-edit-tab.md)'un Katmanlar panelindeki oturum düğmesini değiştirir.
- **İstek:** sahibin sözü (24 Kasım 2026): “katmanın edit moduna girmesi katman sağ tuş menüsünde bu çok saçma oradan
  kaldırılmalı hem edit hem de kayıt, eğer katman düzenlenebilir bir katmansa ribbon üzerinde olmalı edit moduna girme
  ve kaydetme, qgis gibi bir mantık çok iyi”.
- **Kural:** CLAUDE.md §25 kural 2 (her yüzey her modülde açıkça yerleşir) ve 6 (veri yalnız komutla değişir).
- **Web:** iptal edildi (sahibin sözü, 24 Kasım 2026). **`.kcad`:** değişmedi. **Sözleşme:** değişmedi; oturumun
  ürün komutu `cad.layers.session`'dır (ADR 9103), Python ve MCP onu kullanır.

## §1 Araştırma (24 Kasım 2026'da okundu)

- **QGIS, *Editing*:** her düzenleme oturumu “Toggle editing” ile başlar; araç sayısallaştırma araç çubuğunda,
  katmanın menüsünde, öznitelik tablosunda ve Düzen menüsündedir. Katmanlar salt okunur açılır, yanlışlıkla düzenleme
  olmaz. “Save Layer Edits” aynı araç çubuğundadır; “Current Edits” bütün katmanların kaydını ve geri alınmasını toplar.
  <https://docs.qgis.org/3.10/en/docs/user_manual/working_with_vector/editing_geometry_attributes.html>
- **Esri, ArcGIS Pro *Save or discard edits*:** Kaydet ve Vazgeç, şeridin Edit sekmesinin Manage Edits grubundadır;
  vazgeçmek son kayda döner. <https://pro.arcgis.com/en/pro-app/latest/help/editing/save-or-discard-edits.htm>
- **Esri, *Enable or disable editing*:** düzenleme şeritteki Edit sekmesinden açılıp kapanır.
  <https://pro.arcgis.com/en/pro-app/3.5/help/editing/enable-and-disable-editing.htm>

**Ortak örüntü:** oturumu açmak, kaydetmek ve vazgeçmek düzenleme araçlarının yanında, şeritte ya da araç çubuğunda
durur; katmanın menüsü onların asıl yeri değildir.

## §2 Karar

1. **Düzenle sekmesinin ilk paneli “Düzenleme”** (yalnız CBS'nin şeridinde; Mimari, Harita ve Yol'da CBS tablosu salt
   okunur altlıktır, ADR 9196 §2):
   - **Düzenlemeyi başlat** (büyük; `layer.edit`): etkin veri katmanının oturumunu açar. Açıkken düğme yanar ve
     “Düzenlemeyi bitir” der; basınca oturum kapanır, kaydedilmemiş değişiklik varsa önce sorulur (ADR 9103). İki
     satırlı yüzün ilk satırı aynıdır, genişlik değişmez.
   - **Değişiklikleri kaydet** (`layer.editSave`): düzenlenen bütün katmanların değişikliklerini kaynaklarına yazar;
     katmanlar açık kalır.
   - **Değişiklikleri geri al** (`layer.editRollback`): düzenlenen bütün katmanların değişikliklerini bir kez sorduktan
     sonra bırakır. Bırakmak geri alma adımı değildir (ADR 9103 §3), bu yüzden sorulur; katmanlar açık kalır.
   - **Değişiklikleri incele…** (`layer.editReview`): Katman değişiklikleri penceresi; etkin katmanın satırları
     bekliyorsa onda açılır.
2. **Kapalıyken nedeni söylenir:** etkin katman veri katmanı değilse (adıyla), salt okunur bağlantıdansa, yazma sürüyorsa
   Düzenlemeyi başlat kapalıdır; bekleyen değişiklik yoksa Kaydet ve Geri al, yazılmamış satır yoksa İncele kapalıdır.
   Komut ara ya da komut satırından çalıştırılınca aynı sözle reddedilir.
3. **Katman menüsünde oturum yok.** `LayerEntry::EditSession` kaldırıldı: Düzenlemeyi başlat/bitir, Değişiklikleri
   kaydet, geri al, incele, Bütün katmanları kaydet ve geri al hiçbir modülde, hiçbir katman türünde yoktur. Matris
   sınaması bunu her hücrede sınar.
4. **Katmanlar panelindeki oturum düğmesi kalktı** (ADR 9169 §2.1): Katman paneli düğmesi yerine döndü; alanın
   ipucu oturumu Düzenle sekmesindeki Düzenlemeyi başlat'a yönlendirir.
5. **Şeritteki yeri:** Düzenleme, Katmanlar (etkin katman), Oluştur, Değiştir, Alan işlemleri, Şablonlar. Düzenleme,
   Oluştur ve Değiştir korunan panellerdir (ADR 9200). Ölçüler (varsayılan yazı):
   - Düzenleme 243 px, Katmanlar 263, Oluştur 436 (küçük 312), Değiştir 277, Alan işlemleri 129, Şablonlar 246;
     hepsi 1595 px.
   - 1440 px'te yalnız Şablonlar katlanır.
   - 1100 px'te Düzenlemeyi başlat büyük kalır, Oluştur'un dört aracı adlarıyla küçülür; Katmanlar, Alan işlemleri ve
     Şablonlar katlanır. Kapalı katmana çizilmediği için oturum düğmesi çizim araçlarından sonra katlanır.
6. **Simgeler:** kalem (Düzenlemeyi başlat), Kaynağa yaz'ın simgesi (Değişiklikleri kaydet), yeni `editsRollback`
   (geri al oku ve ayrı kalem), yeni `editsReview` (satırlar ve büyüteç). Seçenek sayfası
   `.run/gui-evidence/ikon-duzenleme-oturumu/` (A–D seçenekleri, 16 ve 20 px, açık ve koyu).

## §3 Uygulama

- `apps/desktop/src/gis/edit_commands.rs`: dört komut, kapalılığın nedeni, yanık durum ve şeritteki yüz; oturumun
  olayları `data_edits.rs`'tedir. Geri alma sorusu `data_edits::After::Rollback`.
- Katalog: `catalog/v1/additions.json` (komutlar ve iki simge), `catalog/v1/ribbon.json` ve `scripts/ui/ribbon.py`
  (panel), `apps/desktop/ported.json`, `catalog/v1/operations.json` (düzeyleri), `catalog/v1/modules.json` (CBS'nin
  komut kümesi), CAD manifestosu (`docs/cad/manifest.*`).
- Bağımlılık tabanı küçüldü: kabuk → CBS çekirdeği 43 → 42 (`scripts/dev/module_deps.baseline.json`).

## §4 Kanıt

- `edit_commands_tests`: şeritten başlatma ve bitirme (yüz ve yanık durum), veri katmanı olmayan ve salt okunur katmanda
  nedenli kapalılık ve ret, geri almanın sorusu (Vazgeç tutar, Geri al bırakır, katman açık kalır), panelin yeri ve
  boyları.
- `ribbon_tests`: Düzenle 1440 px'te çizim araçlarını büyük, 1100 px'te Düzenlemeyi başlat'ı büyük tutar; her sekme
  1100 px'e sığar.
- Programlar: `layer_menu_tests` (42 girişlik tablo; hiçbir katman menüsünde oturum sözü yok) ve öbür programlar arası
  sınamalar (37) geçer; CBS paketinin 57 sınaması geçer.
- Resim: `.run/gui-evidence/kullanim-9201/` (oturum kapalı, açık ve değişiklikli, geri alma sorusu; veri katmanının
  menüsü). Git'e eklenmez.
