# ADR 0539: Veri girişinin satır kapsamı

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-38'in ilk dilimi. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0411 (Veri girişi rolü, alan süzgeci, sunucunun yazma bekçisi), 0214 (`@` değerleri), 0445 (yayından
  kaldırılan form), 0015 (proje yetkisi).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - olayların (`project.changes`) kimlik listesinin süzülmesi: bugün Veri girişi olayda kapsamı dışındaki bir kaydın kimliğini ve
    işlemini görür, değerlerini görmez; o kaydı istediğinde yoktur;
  - kapsamın sağlayıcıya (GeoPackage, PostGIS sorgusuna) taşınması ve bağlı kaynağın DB rolüyle otoritesi;
  - rollerin dataset, katman, alan, dışa aktarma, medya, işlem ve onay düzeyinde tek çözücüde birleşmesi; yapay zekâ bağlamının ve
    önbelleklerin aynı süzgeçten geçmesi;
  - formun Veri girişi dışındaki rollere (Düzenleyici) kapsam vermesi.

## Bağlam

Veri girişi rolü (ADR 0411) kayıtların özniteliklerini yalnız yayımlanmış formla yazar. Formun göstermediği alanları okumaz, sunucu
onları süzer. Ama katmanın bütün kayıtlarını görür.

Belediyede tespit ekibi yalnız kendisine atanan binaları, mahalle görevlisi yalnız kendi mahallesini görmeli ve düzenlemelidir.
Plan, erişilemeyen kaydın varlığının sayımda ve listelerde açığa çıkmamasını, yetkinin commit anında yeniden doğrulanmasını ister.

## Karar

1. **Kapsam formun gövdesinde.** `FormBody.scope`, katmanın alanları ve `@` değerleri üzerinde bir ifadedir.
   - Yayımlanan sürümle birlikte donar; değiştirmek yeni bir sürüm yayımlamaktır (`forms.publish`).
   - En çok 2 000 harf, boş olamaz.
   - Ön denetim, derlenmeyen ya da katmanda olmayan alanı okuyan kapsamı söyler (`form_scope_invalid`). `cad.forms.edit`'in
     yayımlaması ve tasarımcının Yayınla'sı bu hatayla yapılmaz; sunucuya bir ayar yamasıyla gelen derlenmeyen kapsam hiçbir şey vermez.
2. **Kural (`kentos_forms::scope`).** Bir katmanın kayıtlarını onun canlı formları kapsar: yayımlanmış ve yayından kaldırılmamış
   formlar.
   - Bir kayda, formlardan birinin kapsamı yoksa ya da kapsamı onun için doğruysa ulaşılır.
   - Boş ya da yanlış cevap ulaşmaz: izin verildiği gösterilmeyen yoktur.
   - Derlenmeyen kapsam hiçbir şey vermez.
   - Canlı formu olmayan katman kapsanmaz.
3. **`@kullanici_eposta`.** Oturumdaki kişinin e-postası, yeni yerleşik `@` değeridir. `@kullanici` adıdır ve ad tekil değildir; kapsam
   kişiyi e-postasıyla ayırır. Sunucu formların kurallarını da artık oturumdaki kişinin adı ve e-postasıyla değerlendirir (önceden
   `@kullanici` boştu).
4. **Sunucu.**
   - Veri girişinin okumalarında (nesne sayfaları, kimlikle okuma, projenin bilgisindeki nesne sayısı) kapsamın dışındaki kayıtlar,
     görmediği proje gibi yoktur.
   - Sayfanın imleci okunan son satırdır, kapsamda olsun olmasın.
   - Yazmada, kapsamın dışındaki kayda yazma bulunamadı (404) olur; kaydı kapsamın dışına çıkaran yazma reddedilir (403).
   - Denetim proje kilidi altındaki commit'te, o anki formlarla yapılır: yetki ya da kapsam işlem sürerken değişirse yeni hâli geçer.
   - Düzenleyici ve üstündeki roller kapsanmaz.
5. **Masaüstü.**
   - Formlar penceresinde (tasarımcı) Kapsam alanı ve ipucu; ön denetim sorunu yayımlamayı kapatır.
   - `@kullanici_eposta` İfade oluşturucu'nun listesinde; masaüstü onu bulut hesabının e-postasıyla verir.
6. **Python ve MCP.** Formun gövdesi `cad.forms.edit` ile yazılır, kapsam onun bir alanıdır. Formun JSON Schema'sı
   (`catalog/v1/forms/form-v1.schema.json`) ve katalog yeniden üretildi.

## Doğrulama

- `KENTOS_TEST_DB=required cargo test -p kentos-application --test project_form_scope`, geçici PostGIS'te:
  - iki Veri girişi (Ali, Ayşe) ve düzenleyici;
  - okuma, sayma, kimlikle okuma;
  - kendi kaydını yazma, başkasınınki bulunamadı, kaydı kapsam dışına çıkarma reddi, yazılmadığının veritabanından denetimi;
  - derlenmeyen kapsamın hiçbir şey vermesi;
  - `project_forms` (ADR 0411) değişmeden geçer.
- `cargo test -p kentos-forms --lib scope`: kuralın durumları (kapsamsız form, iki kapsam, yayından kaldırılan form, boş cevap,
  derlenmeyen kapsam, ön denetim).
- `uv run --with jsonschema python3 scripts/fixtures/form_schema_cases.py --check`: formun şeması; `variable_form_cases.py --check`:
  yerleşik değerler.
- Resimler: `KENTOS_SHOTS_ONLY=form-kapsam` ve `form-kapsam-sorun`, `cargo test -p kentos-desktop tools_screens -- --ignored
  --nocapture` (`.run/shots/arac-form-kapsam-*`).
