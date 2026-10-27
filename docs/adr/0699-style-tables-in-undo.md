# ADR 0699: Stil tabloları geri alma adımında

- **Durum:** kabul edildi (27 Ekim 2026).
- **Bağlam:** `plan/architecture.md` ARCH-BASE-01'in açık maddesi; ADR 0698'in gerçek GUI denemesinde 131–135 başarısız
  Undo/Redo kaydı. ADR 0183 (yazı ve ölçü stilleri), 0326 (kılavuz stilleri), 0339 (duvar stilleri) tabloyu “proje ayarı,
  geri alma adımı değil” saymıştı; bu karar onların o kuralının yerine geçer.
- **Web:** ertelendi (ADR 0228); web'in `styles.edit`'i TODOS.md §0.2'de.

## Bağlam

`cad.styles.edit` stil tablosunu `set_settings` ile geri almanın dışında yazıyor, stile uyan nesneleri (yazı, ölçü, kılavuz,
duvar) ise bir geri alma adımında güncelliyordu. Bunun üç sonucu vardı:

1. Yalnız adı değişen stil hiçbir nesneyi değiştirmediği için boş adım geçmişe girmiyordu: Geri al ya hiçbir şey yapmıyor ya da
   **önceki ilgisiz adımı** geri alıyordu (ADR 0698, 131–135).
2. Stili güncelleyip geri almak nesneleri eski hâline döndürüyor, tabloyu yeni hâlinde bırakıyordu: nesne ile tablo ayrışıyordu.
3. Silinen stili geri almak nesnelere stil bağını geri veriyor, stili tabloya getirmiyordu: nesneler **olmayan bir stile** bağlı
   kalıyordu. Ortak durumlar (`cad.styles.edit.json`) bu tutarsızlığı beklenen sonuç olarak kodlamıştı.

Araştırma (üretici belgeleri):

- [AutoCAD — transaction rollback](https://help.autodesk.com/cloudhelp/2026/ENU/OARX-DevGuide-Managed/files/GUID-C3DE4301-EAC3-4745-A434-9F28EE87AB73.htm):
  bir işlemin bütün değişiklikleri birlikte kesinleşir ya da geri döner; stil tablosu (sembol tablosu) kaydı da işlemin içindedir.
- [Qt QUndoStack — komut grupları](https://doc.qt.io/qt-6/qundostack.html): birden çok değişiklik tek geri alınabilir komut
  olarak gruplanır.

(Bu iki bağlantı ARCH-BASE-01'in açık maddesinde zaten karşılaştırma dayanağı olarak verilmişti.)

KentOS'un kararı bu genel ilkeyi kendi eşzamanlı düzenleme modeline uygular; ürünlerin çatışma politikasını kopyalamaz.

## Karar

1. **Tablo, izleyen nesnelerle aynı adımda.** `cad.styles.edit`'in yazması (`styles_edit::write`) tabloyu ve izleyen nesneleri
   `Document::with_style_tables` içinde yazar. Gövde bitince yazı, ölçü, kılavuz ve duvar stili tablolarının farkı yeni geçmiş
   işlemi `Op::StyleTables` olarak aynı adıma kaydedilir. Yalnız ad değişse de adım vardır; adımın adı türün adıdır (“Yazı stili”,
   “Ölçü stili”, “Kılavuz stili”, “Duvar stili”).
2. **Stil stil, kimlikle.** İşlem bütün tabloyu değil, adımın değiştirdiği her stili kimliğiyle, önceki ve sonraki tanımı ve
   tablodaki yeriyle tutar. Geri al ve Yinele yalnız bu stillere dokunur:
   - stil hâlâ adımın bıraktığı gibiyse eski tanımına döner (silinen stil eski yerine gelir, yeni stil kalkar);
   - **başka düzenleyici** (bulut eşitlemesi) stili o arada değiştirdiyse onunki kalır;
   - adımın dokunmadığı stiller, sonradan eklenenler dahil, olduğu gibi kalır;
   - geri gelen stil, o arada başka stilin aldığı bir adla tabloyu geçersiz kılacaksa gelmez (tablo hiç geçersiz kalmaz).
3. **Bulut.** Adımın raporu proje ayarlarını değişen parça sayar (`parts` ve `undo_parts`); geri al ve yinele ayarları yeniden
   gönderir. Başka düzenleyicinin ayarı geldiğinde adım düşmez; yukarıdaki stil stil kural onun değişikliğini korur.
4. **Değişmeyen.** Plan yazmaz. `set_settings`'in öteki ayarlar için “düzenleme, geri alma adımı değil” kuralı, ayar pencereleri,
   şablonun `TemplateSettings`'i ve dosya biçimi değişmez. Komutun girdisi, çıktısı ve ret kodları aynıdır. Geçmiş belleğe
   yazılır, `.kcad`'e girmez.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/style_edit_command_cases.py --check` yeni kuralı kodlar: oluştur, yeniden adlandır, al
  artık birer adım; güncellemenin ve silmenin geri alınması tabloyu da getirir. Kullanıcının yaşadığı kusur için iki yeni durum:
  duvar stilinin yalnız adı (güncelle ve yeniden adlandır yollarıyla) değişir, Geri al eski adı, Yinele yenisini getirir;
  silinen duvar stili geri alınınca stil ve duvarın bağı birlikte gelir (61 durum).
- **Belge:** `crates/native/domain/tests/all/style_tables.rs`: yalnız ad, silinen stilin yerine dönmesi, boş değişikliğin adım
  olmaması, başka düzenleyicinin sonraki değişikliğinin ezilmemesi ve alınmış ada geri dönmeme.
- **Masaüstü:** Duvar stilleri penceresinde Kaydet ve araç çubuğundaki Geri al / Yinele (gerçek oynatıcıyla resimler).
