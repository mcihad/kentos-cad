# ADR 0764: Üst kaydın formu alt kayda izin vermez: alt katmanın formu, alanı ve satırı ayrı denetlenir

Durum: Kabul edildi (31 Ekim 2026)
Plan: plan/form.md FORM-REL-13
İlgili: ADR 0411 (Veri girişi, yazma bekçisi), 0440 (ilişkili kayıtlar öğesi, alt kayıtlar), 0539 (satır
kapsamı), 0550 (tek erişim çözücüsü), 0737 (Veri girişinin birleştirilmesi), 0762 (ilişkili kayıtlar öğesinin
eylemleri ve izinleri)

## Bağlam

Planın maddesi: “Parent data edit izni child edit izni vermesin; ilgili dataset/row/field/action yetkileri ayrıca
denetlensin.”

ADR 0762 formun sunduğunu (öğenin eylemleri) kullanıcının izninden ayırdı: kayıt ya da alt katman kilitliyse, rol
nesne yazmıyorsa, Veri girişiyse (oluşturamaz, silemez) eylem soluk ve gerekçeli. Eksik kalan, Veri girişinin alt
katmandaki kendi hakkıydı. Veri girişi (`forms.use`, `feature.write` yok) bir katmanın kayıtlarını yalnız o katmanın
yayımlanmış formuyla, o formun gösterdiği alanlarla ve formun kapsamındaki satırlarda yazar (ADR 0411 §2, 0539).
Üst kaydın formundaki ilişkili kayıtlar öğesi bu hakkı alt katmana taşımamalı. Bugün:

- Masaüstünde Veri girişine alt katmanın formu sorulmuyordu: alt katmanda yayımlanmış form yokken de Değiştir, Bağla,
  Bağı kopar açıktı; sunucu sonra reddediyordu.
- **Sunucuda sessiz kayıp vardı.** Veri girişinin yazması önce “okuduğunu yazar” kuralıyla birleşir (ADR 0411 §6):
  formların göstermediği alanlar saklı kopyadan alınır (`form_guard::merged`). Alt kaydın formu bağ alanını
  göstermiyorsa, üst kaydın formundan yapılan Bağla isteği bağ alanını yazar; sunucu o değeri sessizce eskisine
  döndürür ve yazmayı kabul eder. İstemci bağlandı sanır, sunucuda bağ yoktur.

### Üreticilerin çözümleri

- **ArcGIS.** İlişkili tablonun düzenleme yetkisi kendi katmanının (sublayer) yetenekleridir; bir hizmette katman
  başına Create/Update/Delete ayrı verilir, ilişkili kaydı eklemek ilişkili tablonun “Create”ini ister. Alan düzeyinde
  düzenlenemeyen alan (`editable: false`) yazma isteğinde reddedilir; sessizce yok sayılmaz. Kaynak: [ArcGIS REST API,
  Apply Edits (Feature Service/Layer)](https://developers.arcgis.com/rest/services-reference/enterprise/apply-edits-feature-service-layer/),
  [ArcGIS Online: Manage hosted feature layers, editing settings](https://doc.arcgis.com/en/arcgis-online/manage-data/manage-editing-hfl.htm).
  Sonuç: alt katmanın yetkisi üst kaydınkinden ayrıdır, yazılamayan alan reddedilir.
- **QGIS.** İlişki düzenleyicisinin (Relation Editor widget) Ekle, Bağla, Bağı kopar düğmeleri alt katmanın düzenleme
  durumuna ve sağlayıcının yeteneklerine bağlıdır; alt katman düzenlemeye açık değilse düğmeler kapalıdır. Kaynak:
  [QGIS kullanıcı kılavuzu, Creating one or many to many relations](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/joins_relations.html).
  Sonuç: düğmenin durumu alt katmanın kendi hakkından okunur.
- **PostgreSQL.** Satır güvenliği (RLS) ve sütun yetkileri her tabloda ayrı tanımlanır; yabancı anahtarın varlığı
  alt tabloya yazma hakkı vermez, `UPDATE` yetkisi olmayan sütuna yazmak hata verir. Kaynak:
  [PostgreSQL GRANT](https://www.postgresql.org/docs/current/sql-grant.html),
  [Row Security Policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html).

## Karar

### 1. Veri girişinin yazma kuralı tek yerde

`kentos_forms::entry_write_refusal(reach, layer, before, after)`: Veri girişinin (`Caller::through_forms`) bir
kaydın özniteliklerini `before`'dan `after`'a değiştirmesi.

- Katmanın canlı formlarının (yayımlı, yayından kalkmamış) gösterdiği alan **okunur**: yokluğu silinmesidir.
- Göstermedikleri alan **gizlidir**: Veri girişi onu hiç okumadı; yokluğu onu korur (okuduğunu yazar, ADR 0411 §6),
  saklı değerden farklı bir değer vermesi okumadığını yazmasıdır.
- Hiçbir şey değişmediyse sorun yok. Değişiklik varsa ve katmanı canlı form yazmıyorsa `NoForm`; varsa ad sırasıyla
  ilk yazılan gizli alan `Hidden`.

`Reach::through_live_form(layer)` katmanı canlı bir formun yazıp yazmadığını söyler (ADR 0550'nin çözücüsünden).

### 2. Sunucu sessiz geçmez

`project.changes` Veri girişinin değişikliğini birleştirmeden önce bu kuralla sorar: gizli bir alan yazılmışsa
yazmanın bütünü 403 ile reddedilir (““bina” alanı “Sayaçlar” katmanının formlarında yok; Veri girişi onu
yazamaz…”). Formsuz katmanın reddi eskisi gibi bekçinin (`entry_write`) cümlesiyle. Saklı değeri okumadan aynısını
göndermek, ya da alanı hiç göndermemek değişiklik değildir.

### 3. Masaüstünde belge aynı kuralı uygular

Veri girişinin veritabanı projesinde (`ObjectPolicy::AttributesOnly`, ADR 0411 §5) belgenin bütünlük denetimi
(`Document::faults`) her adımın değiştirdiği kayıtları aynı kuralla sorar: formu olmayan katmanın kaydı ya da gizli
alanı yazan adım bütünüyle geri alınır, cümlesi sunucununkiyle aynı. Böylece Formla düzenle'nin alt kayıtları,
`cad.forms.submit`'in `children`'ı, `cad.entities.set`, Python ve ajan aynı yerden durur; sunucuya hiç gitmez.

### 4. İlişkili kayıtlar öğesi alt katmanın hakkını söyler

`ChildPermits.child` (`ChildEntry`: alt katmanın ve bağ alanının adı, canlı formu olup olmadığı, bağ alanını
gösterip göstermediği), yalnız Veri girişine verilir. Sıra ADR 0762'ninki, ardından:

| Eylem | Alt katmanın canlı formu yok | Form bağ alanını göstermiyor |
|---|---|---|
| Değiştir (`update`) | “… katmanının yayımlanmış formu yok; Veri girişi alt kayıtlarını değiştiremez.” | izinli |
| Bağla, Bağı kopar | aynı | “… alanı … katmanının formunda yok; Veri girişi alt kaydı bağlayamaz ya da bağını koparamaz.” |

Görmek ve göstermek her zaman açık. Bağ alanı ancak alt katmanın formundaki bir alan öğesiyle gösterilir; ilişkili
kayıtlar öğesi alan göstermez. Düzenleyiciye (`feature.write`) alt katmanın formu sorulmaz: onun hakkı proje
düzeyindedir (ADR 0015).

### 5. Satır

Satır kapsamı (ADR 0539) sunucudadır: Veri girişi kapsamı dışındaki alt kaydı hiç almaz (ADR 0550 §2), kaydı kapsamın
dışına çıkaran yazma reddedilir. Masaüstü belge `@kullanici_eposta`'yı bilmez; kapsamı yerel olarak yinelemez.

## Sonuçlar

- Üst kaydın formu alt katmana yetki taşımaz: alt katmanın formu, alanı ve satırı kendi kuralıyla.
- Sunucu artık Veri girişinin okumadığı alana yazmasını sessizce yutmaz; istemci ile sunucunun kopyası ayrılmaz.
- Veri girişinin projesinde formun göstermediği bir alanı yazan izleyici (alt ağ adı, olay bağı gibi aynı adımda
  başka katmana yazan kurallar) de durur; Veri girişi o katmanların alanlarını formla görmüyorsa bu doğrudur.

## Doğrulama

- `python3 scripts/fixtures/child_writes_cases.py --check`: 20 sunma ve 20 yazma durumu KentOS kodu olmadan
  (`fixtures/forms/v1/child-writes.json`); Rust'ta `cargo test -p kentos-forms --test all child_writes`.
- Sunucu: `cargo test -p kentos-application --lib form_guard`; geçici PostGIS'te
  `KENTOS_TEST_DB=required cargo test -p kentos-application --test project_forms a_veri_girisi_edit_is_merged`.
- Masaüstü belgesi: `cargo test -p kentos-native-application --test all object_policy`.
- Masaüstü: `cargo test -p kentos-desktop --lib related_actions`; resim
  `KENTOS_SHOTS_ONLY=form-alt-kayit-izni cargo test -p kentos-desktop --lib tools_screens -- --ignored`.

## Açık kalanlar

- Kaynağın izinlerine göre süzülen sayılar ve otomatik tamamlama (FORM-REL-15, FORM-ACL-09) bulutun ayrı işleridir.
- Formun salt okunur ya da hesaplanan alanının Veri girişince yazılması `entry_write`'ın formla uygulamasındadır
  (ADR 0411 §2); bu ADR yalnız görünmeyen alanı ekler.
- Gerçek pencerede elle denenmedi.
