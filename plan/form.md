# KentOS — Ortak CBS form standardı, tasarımcı ve merkezi veri girişi

İnceleme tarihi: **12 Ekim 2026**. Bu belge form sisteminin bağımsız uygulama planıdır. Önceden [Python ekosistem planında](python.md) bulunan bütün `FORM-*` görevleri, ayrıntıları ve ortak JSON örneği bu dosyaya taşınmıştır. CAD/komut tamlığı için [cad.md](cad.md), isteğe bağlı eklenti runtime ve `.kek` dağıtımı için [python.md](python.md) esas alınır.

**Ürün önceliği:** Form sistemi KentOS'a eklenecek ilk yeni özellik olabilir. Önerilen sıra, önce mevcut katman/öznitelik altyapısı üzerinde çalışan masaüstü ve web formu, ardından merkezi yayın ve yetkili veri girişidir. İleri domain/ilişki, mobil ve çevrimdışı kapsam aynı standardın sonraki dilimleridir.

**Bağımsızlık:** Temel form tasarımcısı, form görüntüleme/doldurma, taşınabilir kurallar ve merkezi veri girişi; uv kurulumu, NumPy, Python eklenti runtime'ı, `.kek` paket yöneticisi veya eklenti bulut kataloğunu beklemez. KentOS'un kendi UI bileşenleri kullanılır. Python/AI/komut erişimi mevcut sözleşme/katalog/SDK altyapısından aynı işlemlere bağlanır; Python kullanmak isteyen için bu erişim bulunur, form kullanıcısına Python kurma zorunluluğu getirilmez.

Bu belge araştırma, ürün sözleşmesi ve yapılacak iş listesidir; uygulama kodu değildir. API adları, JSON örneği, izin aileleri ve profiller öneridir. Kaynak incelemesi ile hedef kabul testleri ayrıdır; burada hedeflenen form özelliklerinin bugün çalıştığı veya test edildiği iddia edilmez.

**İlerleme (16 Ekim 2026):** işaretli maddeler masaüstünde, çekirdekte, komutlarda, Python'da ve MCP'de yapıldı
(ADR 0400–0405; TODOS.md §16.6). Web ertelendiği için (ADR 0228) web'in tarafı TODOS.md §0.2'dedir; web isteyen
maddeler web gelene dek işaretsiz kalır.

## 1. Ürün hedefi ve kesin şartlar

- Merkezde yetkili kişi form tasarlar, inceler ve yayınlar; saha personeli atanmış kayıt/alan/görev kapsamında veri girer.
- Form tasarımı, formu kullanma, yayın yönetimi, veri okuma/yazma, kaynak şeması yönetimi ve veri onayı ayrı yetkilerdir.
- Web, masaüstü ve mobil **aynı sürümlü veri tanımından** form üretir. Alan kimlikleri, domain kodları, veri bağları, kurallar ve doğrulama anlamı korunur; ekran düzeni cihaza uyarlanır.
- Form; sekme, grup, responsive düzen, master–detail, ilişkili kayıtlar, domain/subtype, bağlı seçenekler, join, dış lookup, hesap, koşullu zorunluluk, ekler ve iş akışlarını kapsar.
- Form tanımı veri kaynağı izinlerinin yerine geçmez. Gizli veya salt okunur widget tek başına veri güvenliği sağlamaz; bulut yetkisi ve doğrulama sunucuda uygulanır.
- Form sahibi olmak bütün kaynak verisini görmek veya düzenlemek anlamına gelmez. Formu okuyabilmek de veriyi yazma izni vermez.
- Yayınlanmış form sürümü değişmez; tasarımcı değişikliği yeni taslak/sürüm oluşturur. Saha verisi hangi sürümle toplandığını taşır.
- Zorunlu veri bütünlüğü kuralları form, tablo, import, komut, Python, eklenti ve AI yazma yollarında aynı kaynak politikasına bağlanır.
- Yerel kişisel form ile kurumun merkezi formu sahiplik ve otorite bakımından ayrıdır; dosya içi yetki iddiası bulutta grant oluşturamaz.
- Python ve AI, arayüzdeki bütün tasarım/preview/veri girişi/yayın işlemlerini aynı komutlarla yapabilir; yetki, onay ve doğrulama aşamalarını atlayamaz.
- Standart ilk günden mobil uyumlu tasarlanır. Gerçek mobil renderer, auth ve offline istemci ayrı teslimdir; yalnız JSON tanımı hazırlanması mobil uygulamanın tamamlandığı anlamına gelmez.
- Temel form özelliği isteğe bağlı Python/uv sisteminden bağımsızdır. İsteğe bağlı Python/Processing action varsa gereken executor ve çevrimdışı sınırı formda açık belirtilir.

## 2. Mevcut altyapı, doğrulanan eksikler ve kıyas

### 2.1 Kaynak kanıtları

| Kanıt | Kaynak / anlamı |
|---|---|
| E03 | [Python UI ADR](../docs/adr/0150-python-surfaces.md), [AI Surface ADR](../docs/adr/0154-ai-surfaces.md): mevcut KentOS UI Python/AI bileşenleri |
| E04 | [Ürün kataloğu](../crates/shared/contracts/src/catalog.rs), [headless dispatcher](../crates/native/headless/src/dispatch.rs), [MCP](../crates/native/mcp/src/tools.rs): ortak otomasyon yolu |
| E08 | [Alan sözleşmesi](../crates/shared/contracts/src/fields.rs), [katman alanları ADR](../docs/adr/0199-layer-fields-and-feature-table.md): tür, aralık, seçim listesi, zorunluluk ve otomatik alan düzenleme |
| E09 | [Sorgu/join ADR](../docs/adr/0200-spatial-and-attribute-queries.md): mekânsal sorgu ve anahtar üzerinden değer aktarma; kalıcı 1:N/N:M ilişki formu ayrı gelecek kapsam |
| E10 | [KentOS UI Form](../crates/ui/src/widget/form.rs), [widget modülü](../crates/ui/src/widget/mod.rs), [web alan düzenleyici](../apps/web/src/ui/properties/fieldRows.ts): mevcut form yapı taşları |
| E11 | [Proje yetki ADR](../docs/adr/0015-project-ownership-and-access.md), [gerçek yetki değerlendirme](../crates/server/application/src/access.rs): roller, paylaşım, sunucu ve satır güvenliği temeli |
| E12 | [Proje snapshot](../crates/shared/contracts/src/document.rs), [kalıcı kimlik ADR](../docs/adr/0014-persistent-entity-identity.md): yerel/bulut kayıt ve UUID temeli |
| E13 | [Öznitelik ekleri ADR](../docs/adr/0225-object-attachments.md), [Attachment sözleşmesi](../crates/shared/contracts/src/attachment.rs): fotoğraf/belge ekleri için mevcut kaynak modeli |
| E14 | [İfade çekirdeği](../crates/shared/expression/src/lib.rs), [ifade ADR](../docs/adr/0100-expression-engine.md): ortak hesap/ifade altyapısı; form profiline sınırlandırılacak |
| E15 | [Yol haritası](../TODOS.md), [CAD planı](cad.md): Python ortam, AI kapsam, ilişkiler ve platform hedefleri |

### 2.2 Mevcut / kısmi / yeni iş ayrımı

| Alan | Gözlenen durum | Form planının tamamlayacağı iş |
|---|---|---|
| Katman alanları | Beş tür: text/integer/decimal/date/boolean; kanonik metin değerleri [E08] | Stable field binding, typed codec/adapter, açık null/absent/empty politikası; geniş tip/domain modeli aşamalı |
| Şema tabanlı alan UI | Alanlar bölümü ve KentOS UI form yapı taşları var [E08, E10] | Bağımsız FormDefinition, temel tasarımcı, renderer ve ayrı draft/session |
| İfade motoru | Ortak hesap/ifade çekirdeği var [E14] | Taşınabilir form profili, dependency graph, client/server conformance ve commit validation |
| Kimlik/kalıcılık | Proje snapshot ve kalıcı UUID temeli var [E12] | Form draft/release/deployment metadata, source mapping, session revision ve submission kaydı |
| Yetkilendirme | Project/feature erişimi için sunucu temeli var [E11] | Form design/publish ile data edit ayrımı; row/field/task/deployment guard'ları |
| Join | Anahtarla değer aktarma mevcut [E09] | Kalıcı ilişki, live joined view ve yetkili edit routing ayrı kaynak türleri |
| Ekler | Fotoğraf/belge kaynağı var [E13] | Form capture/upload/staging, offline outbox ve record/review bağlantısı |
| API/AI | Katalog/headless/MCP ve Python/AI surface temeli var [E03, E04] | Form lifecycle/session/record komutları, generated wrapper, aynı business rule ve coverage |
| Mobil form | Masaüstü/web ile eşdeğer ayrı mobil ürün kabulü görülmedi | Ortak standardı kullanan gerçek mobil renderer, auth/query/offline/media adapter'ları |

**Kıyasın doğru sınırı:** QGIS form tasarımını paylaşmak, veri kaynağını herkesin düzenleyebildiği anlamına gelmez; kaynak izinleri ayrıca belirleyicidir. QGIS sekmeler/gruplar, ilişkili kayıtlar ve alan davranışları sunar. KentOS hedefi bu yetenekleri merkezi sürüm/yayın, sunucuda kişi/görev/alan yetkisi, ortak mobil tanım ve güçlü iş akışlarıyla bütünleştirmektir. [QGIS Attributes Form](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html#attributes-form-properties), [QGIS ilişkiler](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/joins_relations.html)

Bu tabloda mevcut temel ile yeni hedef ayrılmıştır. Diğer geliştiricinin devam eden değişiklikleri nedeniyle uygulamaya başlanırken ilgili kaynaklar tekrar incelenmeli; ADR veya widget bulunması tamamlanmış form ürünü sayılmamalıdır.

## 3. FORM-FIRST — İlk eklenecek özellik için uygulanabilir dilim — P0

### 3.1 İlk ürünün kapsamı

İlk somut kullanıcı akışı: **mevcut bir katmanı seç → alanlarından form oluştur → sekme/grup ve alan düzenini değiştir → koşullu zorunluluk ekle → önizle/kaydet → yetkili form sürümünü aç → kaydı doğrulayarak güncelle**.

İlk dilim yalnız mevcut beş alan türünü, temel choice/range kısıtlarını, basit sekme/grup düzenini ve ortak saf kuralları zorunlu tutar. Yeni bağımsız tablo, N:M, subtype/contingent sistemi, dış servis, mobil offline istemci ve Python action ilk çalışan formun önkoşulu değildir. Bu yeteneklerin bağlantı noktaları v1 şemada baştan sürümlenir; desteklenmeyen zorunlu yetenek sessizce düşürülmez.

Yerel masaüstü/web dilimi erken denenebilir. **Merkezi saha kullanımı için ilk yayın kapısı**, form tasarımı/yayını ile veri girişinin sunucuda ayrılması, yetkili projection/write guard ve değişmez form release/deployment'ının çalışmasıdır. Bu kapı tamamlanmadan “yalnız seçilen kişiler düzenleyebilir” diye bulut teslimi yapılmaz. Karmaşık bir genel policy motoru yerine ilk kapsam için açık dataset/row/field allowlist ve actor/task bağı kurulabilir; güvenlik sonucu aynı olmalıdır.

### 3.2 İlk dilimin işleri

- [x] **FORM-FIRST-01:** Mevcut fields/attribute edit/persistence/ACL yollarını tek bir dikey form senaryosunda eşle; yeni modelin genişleteceği kaynakları ve ortak validator'ı belirle. — *Yapıldı (16 Ekim; ADR 0401 bağlam bölümü).*
- [x] **FORM-FIRST-02:** `core/1` profili için FormDefinition, field binding, layout, session, validation ve command/result sözleşmelerinin asgari alanlarını kesinleştir. — *Yapıldı (16 Ekim; ADR 0401 §1–§4, `kentos.forms/1 core`).*
- [x] **FORM-FIRST-03:** Mevcut layer/object UUID ve field şemasından kararlı schema/field ID mapping üret; ID/name ayrımını metadata'da sakla. Eski alanlara ID backfill deterministik/idempotent olsun; rename/drop explicit mapping değişikliği gerektirsin. — *Yapıldı (16 Ekim; ADR 0401 §2, alanın kararlı `id`'si).*
- [x] **FORM-FIRST-04:** Beş mevcut tür için lossless draft↔canonical attrs adapter'ı kur; boş/absent/null davranışını açık belgele. Depolama null'u ayıramıyorsa unsupported durumu göster; sahte null desteği ilan etme. — *Yapıldı (16 Ekim; ADR 0401 §5: boş değer özniteliğin yokluğudur, null ayrı tutulmaz).*
- [x] **FORM-FIRST-05:** Bütün attribute depolamasını yeni tip sistemine taşımadan adapter ile ilk formu çalıştır; eski/bilinmeyen attribute'ları patch dışında koru. — *Yapıldı (16 Ekim; ADR 0401 §5).*
- [x] **FORM-FIRST-06:** Form draft ve local published snapshot için `.kcad`/proje snapshot persist yolu ekle; save/reopen, eski dosya uyumu, owner/scope ve version/digest metadata'sını doğrula. — *Yapıldı (16 Ekim; ADR 0401 §3, `.kcad` şema 60; bulut kaynakları FORM-F'de).*
- [x] **FORM-FIRST-07:** Mevcut katman alanlarından varsayılan form üret; kullanıcı otomatik düzen yerine kendi taslağını seçebilsin. — *Yapıldı (16 Ekim; ADR 0401 §7).*
- [x] **FORM-FIRST-08:** İlk tasarımcıda field paleti, sekme/grup ekleme, taşıma/sıralama, label/help/widget ve required/read-only özellik düzenleyicisini çıkar. — *Yapıldı (16 Ekim; ADR 0401 §10, masaüstü).*
- [x] **FORM-FIRST-09:** Text/textarea, integer, decimal, date, boolean ve basit kodlu seçim renderer'larını mevcut KentOS UI/web bileşenleriyle bağla. — *Yapıldı (16 Ekim; ADR 0400, 0401 §10, masaüstü; web TODOS.md §0.2).*
- [x] **FORM-FIRST-10:** Saf visible/required/default/validate kurallarını mevcut expression çekirdeğinde sınırlandırılmış profille çalıştır; dependency cycle ve gizli-zorunlu çelişkisini preflight'ta yakala. — *Yapıldı (16 Ekim; ADR 0401 §4, ADR 0402).*
- [x] **FORM-FIRST-11:** Record edit session; base revision, projection, tek draft owner, dirty state, kaydet/vazgeç, hata alanına gitme ve uygun kullanıcı undo davranışı taşısın. — *Yapıldı (16 Ekim; ADR 0401 §10, ADR 0403 §7: sürüm, kaydedilmemiş işaret, Kaydet/Vazgeç, ilk soruna git, tek geri alma adımı).*
- [x] **FORM-FIRST-12:** Kaydetme tek semantic command/changeset kullansın; schema/constraint/policy bütünlüğü final commit'te doğrulansın. Validation ve revision conflict'te belge kısmen değişmesin. — *Yapıldı (16 Ekim; ADR 0401 §8–§9).*
- [ ] **FORM-FIRST-13:** Aynı core tanımını desktop/web'de aç; aynı input/validation sonucu, focus/draft preservation ve responsive sekme/grup davranışı için ortak fixture kullan.
- [ ] **FORM-FIRST-14:** Bulutta form metadata için draft/release/deployment kaynakları oluştur; published snapshot immutable ve dataset binding server-owned olsun. — *16 Ekim, kısmen: taslak ve yayınlar projenin ayarında, yayımlanmış sürüm sunucuda değişmez (ADR 0411 §3); ayrı dağıtım kaynağı yok.*
- [x] **FORM-FIRST-15:** İlk merkezi kapsamda `forms.design`, `forms.publish`, `forms.use` ve dataset/row/field read/write izinlerini ayrı denetle; saha editörü form JSON/schema/domain değiştiremesin. — *Yapıldı (16 Ekim; ADR 0411 §1–§3: üç izin ayrı denetlenir; Veri girişi form JSON'unu, şemayı ve değer listelerini değiştiremez).*
- [x] **FORM-FIRST-16:** Okuma projection'ı ve mutation guard'ı ortak sunucu access resolver'a bağla; ham feature/attribute komutu, tablo, import, Python ve AI aynı denetimden geçsin. Geçiş varsayılanları mevcut verinin yetkisini sessiz genişletmesin. — *Yapıldı (16 Ekim; ADR 0411 §4, §6: yazma bekçisi `project.changes`'te bütün istemciler için; Veri girişinin okuması formun alanlarıyla, indirme kapalı).*
- [x] **FORM-FIRST-17:** Bir tasarımcı v1 yayınlasın; yalnız atanmış veri giriş kullanıcısı izinli alanları düzenlesin, viewer yalnız görsün. Her mutation revision/idempotency ve actor audit taşısın. — *Yapıldı (16 Ekim; ADR 0411: tasarımcı yayınlar, Veri girişi yalnız formun açık alanlarını yazar, görüntüleyici yalnız görür; her yazma `project.changes`'in sürüm, idempotency ve denetim kaydından geçer).*
- [x] **FORM-FIRST-18:** Tasarım/preview/session/validate/save/publish komutlarını aynı katalogdan Python wrapper ve AI/MCP yüzeyine çıkar; tüm UI seçenekleri argüman/default/result/hata düzeyinde eşdeğer olsun. Bunun için uv/plugin manager bekleme. — *Yapıldı (16 Ekim; ADR 0401 §9, ADR 0402 §6).*
- [ ] **FORM-FIRST-19:** İlk teslimde local/cloud capability ve desteklenen core profile açık listelensin; mobile/offline/relations/extension action henüz yoksa kullanıcıya gerçek durum gösterilsin.
- [ ] **FORM-FIRST-20:** Bölüm 15'teki I01–I08 ilk teslim kabulünü geçir; ardından ileri FORM-* işleri seçilerek açılır. İlk ürün kabulünü bütün sonraki fazların tamamlanmasına bağlama.

### 3.3 Standardın aşamalı profilleri

Bütün profiller aynı `kentos.forms/1` sözleşme ailesini kullanır. Profil ve required capabilities manifest'te ayrı ilan edilir; format sürümü, profile sürümü ve widget/executor capability sürümü birbirine karışmaz.

| Profil | İçerik | Sıra / gereksinim |
|---|---|---|
| `core/1` | Mevcut typed field adapter, field binding, tab/group, temel inputs/choice, saf kurallar, session/validation/commit | İlk masaüstü/web dilimi; Python/uv gerektirmez |
| `central/1` | Immutable release/deployment, kaynak binding, server ACL/projection/write guard, actor/task audit | İlk kontrollü merkezi veri giriş teslimi; bulut için atlanamaz |
| `relations/1` | Domain/subtype genişletmesi, 1:N/N:M, master–detail, source/lookup/join lineage | Core/central üzerinde sonraki dilimler |
| `mobile-offline/1` | Gerçek mobil renderer, görev paketi, replica/outbox, media ve current-policy sync | Kaynak/ilişki/yetki ve release altyapısı yeterli olduktan sonra |
| `extension-actions/1` | Kayıtlı Python/Processing/custom-widget action bağımlılıkları | İsteğe bağlı [Python planı](python.md); portable core formu engellemez |

Tam standardın aşağıdaki örneği domain/ilişki dahil ileri hedefi gösterir. İlk dilim compiler'ı yalnız desteklediği profilin alanlarını kabul eder; o profile dışındaki zorunlu relation/action'ı olan örneği eksik çalıştırmaz.

## 4. FORM-STD — Masaüstü/web/mobil ortak form standardı — P0

### 4.1 Standardın kapsamı ve otorite

**Önerilen ad:** `kentos.forms/1`. Formun tek kaynak tanımı versioned JSON olacak; Rust contracts kaynağından JSON Schema, TS ve Python tipleri üretilecek. Masaüstü, web ve mobil aynı schema/binding/rule/validation anlamını uygulayacak; aynı pixel düzeninde olmak zorunda değiller.

Bir formu dört katmanla tanımla:

1. **Veri kontratı:** DatasetSchema + fields + domains/subtypes/relations.
2. **Form kontratı:** Widget tree, field binding, taşınabilir kurallar, actions, presentation/capabilities.
3. **Yayın kontratı:** Immutable form release, deployment, source mapping ve desteklenen schema sürümleri.
4. **Erişim kontratı:** Sunucu tarafından bağlanan actor/task/row/field policy. Form JSON'u kendi kendine kullanıcı yetkisi veremez.

| Sözleşme | Asgari zorunlu alanlar |
|---|---|
| DatasetSchema | ID/revision, feature/table türü, stable field IDs, storage mapping, identity, null/type/default/constraint, CRS/geometry capability |
| DomainDefinition | ID/version, owner/scope, value type, coded/range values, status, subtype/contingent bağları |
| RelationDefinition | ID/version, origin/destination datasets/keys, cardinality, min/max, ownership, unlink/delete policy |
| SourceDefinition | Source ID/kind/version, credential ref, typed query/lookup, permitted fields, cache/offline policy |
| FormDefinition | Format/schema version, form ID/revision, data/schema refs, layout tree, rules/actions, required capabilities ve localization |
| FormDeployment | Immutable form digest/version, project/dataset binding, audience, task scope, publication state, accepted schema versions |
| FormSession | Session/form/deployment/dataset/feature IDs, base revisions, mode, permitted projection, draft, validation state |
| SubmissionEnvelope | Operation/patches, entity/related/temp UUIDs, form/schema/domain/rule revisions, expected versions, idempotency key, attachment handles |
| ValidationResult | Field/widget/entity/relation path, stable code, severity, message args, source/revision ve correction info |
| RuntimeCapabilities | Widget/profile version, device features, available action executors, offline data coverage ve effective server grants |

### 4.2 Binding ve değer kuralları

- Binding, label veya alanın görünen adıyla değil **stable dataset/field ID** ile kurulmalıdır. Alan adı değişince bağlı form/relationship bozulmamalıdır.
- Domain label'ı kullanıcıya gösterilir, **kod** saklanır; localization kodu değiştirmez.
- Date, datetime/timezone, integer, decimal, boolean, UUID/reference, attachment ve geometry değerleri tipli olmalıdır.
- Mevcut kanonik metin attribute depolamasından göçte null, absent ve boş metin ayrımı korunmalıdır; eski bilinmeyen alanlar sessizce silinmemelidir.
- Büyük integer ve exact decimal JSON float üzerinden kaybolmamalıdır; value codec, canonical encoding ve gösterim/input locale farklı katmanlarda tanımlanmalıdır.
- Form draft değeri, persisted kaynak değeri, calculated değer ve lookup/join gösterimi farklı kaynak/provenance taşımalıdır.
- Join/aggregate display alanları varsayılan read-only'dir; yazılabilir hedef canonical kaynağa açık command mapping gerekir.
- Veri yetkisi olmayan bir alan payload/projection içine hiç alınmamalıdır. `visible=false` hassas veriyi korumak için yeterli değildir.

### 4.3 Taşınabilir ifade/kural profili

- Profil, mevcut KentOS expression çekirdeğinin uygun parçalarını kullanır; form kuralları için desteklenen tip/fonksiyonlar sürümlenmelidir.
- AST veya normalize edilmiş program; stable field/source refs, typed literal, boolean/comparison/arithmetic/date/string, null ve kontrollü geometri/aggregate işlevlerini kapsar.
- Network/disk/SQL/Python `eval` ifade motorunun serbest yan etkisi olmamalıdır. Harici lookup/action ayrı descriptor ve broker çağrısıdır.
- `visibleWhen`, `enabledWhen`, `requiredWhen`, `default`, `calculate`, `validate` ve `choiceFilter` ayrı rule kind'larıdır.
- Default yalnız tanımlı create/reset aşamasında uygulanır. Calculated değer yeniden hesaplama, user override ve writeback politikasını açık taşımalıdır.
- Alan değişince dependency graph yalnız etkilenen kuralları çalıştırır; cycle ve hesap bütçesi kontrol edilir.
- Tarih/saat, kullanıcı ve lookup context'i değerlendirme snapshot'ında açık sabitlenir; web/mobile/server farklı “şimdi” değerleriyle farklı sonuç üretmemelidir.
- Client sonuçları UX içindir; commit sırasında kaynak schema/domain/ACL/validation otoritesi sunucudadır.
- Python/server action gerektiren ek işlev capability olarak ilan edilir. Gerekli executor yoksa input/submit akışı başarılıymış gibi devam etmez.

### 4.4 Asgari renderer davranışı

- Aynı tanım için aynı alan, domain kodu, ilişkili kayıt, calculated değer ve validation sonucu oluşmalıdır.
- Geniş ekranda tab/grid/master–detail; dar ekranda accordion/tek kolon/ayrı detail screen kullanılabilir. Değişen yalnız presentation'dır.
- Unsupported required widget/action/version varsa form açık compatibility hatası verir; kullanıcıdan alanı atlayıp submit etmesi istenmez.
- Salt okunur widget veya fallback, kaynak veriyi bozmadan gösterir; zorunlu input yeteneği yoksa yalnız görüntüleme modu sunulur.
- Klavye, screen reader, focus/error navigation, touch target, locale ve font accessibility kuralları standartta bulunur.

### 4.5 Örnek form bundle — sekme, domain, koşul ve master–detail

Bu örnek kendi dataset/domain/relation tanımlarını içerir. `ui.*` izin istekleri veya relation action listesi sunucu grant'i değildir; form deployment gerçek veri kaynaklarına ve yetki politikalarına sunucuda bağlanır. Mantıksal ID'ler okunabilir örneklerdir; üretim resource kimlikleri global/tenant kapsamlı kararlı kimlikle tutulmalıdır.

```json
{
  "format": "kentos.formBundle",
  "schemaVersion": 1,
  "id": "example.building-inspection",
  "version": "1.0.0",
  "datasets": [
    {
      "id": "buildings",
      "revision": "1",
      "kind": "features",
      "geometryKind": "polygon",
      "crsBinding": "deployment-source",
      "identityField": "buildingId",
      "fields": [
        { "id": "buildingId", "name": "id", "type": "uuid", "nullable": false },
        { "id": "condition", "name": "durum", "type": "text", "nullable": false, "domainRef": "condition@1" },
        { "id": "floorCount", "name": "kat_sayisi", "type": "integer", "nullable": false, "min": "1" },
        { "id": "note", "name": "not", "type": "text", "nullable": true }
      ]
    },
    {
      "id": "inspections",
      "revision": "1",
      "kind": "table",
      "identityField": "inspectionId",
      "fields": [
        { "id": "inspectionId", "name": "id", "type": "uuid", "nullable": false },
        { "id": "buildingRef", "name": "bina_id", "type": "uuid", "nullable": false },
        { "id": "inspectionNote", "name": "not", "type": "text", "nullable": true }
      ]
    }
  ],
  "domains": [
    {
      "id": "condition",
      "version": "1",
      "kind": "coded",
      "valueType": "text",
      "values": [
        { "code": "good", "label": "İyi" },
        { "code": "review", "label": "Ek inceleme gerekli" }
      ]
    }
  ],
  "relations": [
    {
      "id": "building-inspections",
      "version": "1",
      "origin": { "dataset": "buildings", "key": "buildingId" },
      "destination": { "dataset": "inspections", "foreignKey": "buildingRef" },
      "cardinality": "one-to-many",
      "ownership": "association",
      "deletePolicy": "restrict"
    }
  ],
  "forms": [
    {
      "format": "kentos.form",
      "schemaVersion": 1,
      "id": "building-inspection",
      "revision": "1",
      "schemaRef": "buildings@1",
      "expressionProfile": "kentos.forms/1",
      "presentation": { "smallScreenTabs": "accordion" },
      "rules": {
        "needsNote": {
          "op": "eq",
          "args": [
            { "ref": "record.condition" },
            { "literal": "review", "type": "text" }
          ]
        }
      },
      "layout": {
        "id": "root",
        "type": "tabs",
        "children": [
          {
            "id": "general",
            "type": "tab",
            "label": "Genel",
            "children": [
              { "id": "conditionInput", "type": "field", "fieldRef": "condition", "widget": "choice" },
              { "id": "floorInput", "type": "field", "fieldRef": "floorCount", "widget": "integer" },
              {
                "id": "noteInput",
                "type": "field",
                "fieldRef": "note",
                "widget": "textarea",
                "visibleWhen": "needsNote",
                "requiredWhen": "needsNote"
              }
            ]
          },
          {
            "id": "details",
            "type": "tab",
            "label": "İncelemeler",
            "children": [
              {
                "id": "inspectionsEditor",
                "type": "relationEditor",
                "relationRef": "building-inspections@1",
                "childForm": { "mode": "schema-generated" },
                "actions": ["create", "update"]
              }
            ]
          }
        ]
      }
    }
  ]
}
```

Burada “Ek inceleme gerekli” seçilince not görünür ve zorunlu olur. Aynı rule web/desktop/mobile ve commit doğrulamasında değerlendirilir. “İncelemeler” sekmesi child kayıtları gerçek ilişki üzerinden gösterir; parent ID/FK, visible label veya join sonucu üzerinden tahmin edilmez. Parent ve child create/update işlemleri ilgili dataset/field/row izinlerini ayrı ayrı gerektirir.

Yeni kaydın identity UUID'si host tarafından oluşturulur; kullanıcı input'u gibi düzenlenmez. Geometry/CRS kaynağı gerçek deployment binding'inden gelir. Üretim verisinin bütün girişlerinde zorunlu olan koşul, bu UI tanımının yanında authoritative dataset/workflow write profile'a da bağlanır; yalnız widget rule'u kullanmak yeterli değildir.

### 4.6 Standart uygulama işleri

- [ ] **FORM-STD-01:** FormBundle/FormDefinition ve ilgili bütün resource şemaları için sürümlü sözleşme ve JSON Schema oluştur.
- [ ] **FORM-STD-02:** Format/version/digest, unknown-property/version, compatibility ve migration kurallarını yaz.
- [x] **FORM-STD-03:** JSON'u önce validate/compile et; bozuk field/domain/relation/action binding'leri UI açılmadan raporla. — *Yapıldı (16 Ekim; ADR 0401 §4, ön denetim).*
- [ ] **FORM-STD-04:** Shared compiled form model ve expression profile; native/WASM/server ortak fixture'larla aynı sonuç versin.
- [x] **FORM-STD-05:** Stable node/field/rule IDs, deterministic ordering ve dependency graph korunmalı; rename/move binding'i bozmasın. — *Yapıldı (16 Ekim; ADR 0401 §2, ADR 0402 §2 (hesapların bağımlılık grafı)).*
- [x] **FORM-STD-06:** Field değer codec'ini existing canonical attrs ile kontrollü adapter/migration üzerinden eşle. — *Yapıldı (16 Ekim; ADR 0401 §5).*
- [ ] **FORM-STD-07:** Form template ve deployed form ayrı olsun; template'deki mantıksal kaynaklar deployment'ta gerçek dataset/source IDs'ye bağlansın.
- [ ] **FORM-STD-08:** Template dağıtımı kaynak veri, secret veya tenant grant taşımasın; source remapping ve permission yeniden değerlendirilsin.
- [ ] **FORM-STD-09:** Renderer semantic conformance suite kur; aynı bundle ve draft için values/validation/available actions eşdeğer olsun.
- [ ] **FORM-STD-10:** Responsive/a11y/localization profili ve unsupported-required-capability politikası standardın parçası olsun.
- [ ] **FORM-STD-11:** Portable form Python/uv/plugin kurulumu gerektirmeden web/mobile'da veri girişi yapabilsin.
- [ ] **FORM-STD-12:** Custom Python hook/Processing action bağımlılığı form capabilities'e yazılsın; yerel/server/offline yürütme yolu açık seçilsin.

**Kabul:** Tek FormBundle değiştirilmeden masaüstü/web/mobil renderer'a verilir; aynı domain kodu, koşullu zorunluluk ve child relation sonucu oluşur. Sunum farkları veri anlamını değiştirmez.

## 5. FORM-DATA — Alan şeması, domain, subtype ve bağlı seçenekler — P0/P1

ArcGIS'in domain yaklaşımı; kodlu değer veya aralık, yeniden kullanım ve subtype bazında farklı default/domain kuralları sağlar. Contingent values, birden çok alanın geçerli değer kombinasyonlarını sınırlar. KentOS bu kavramları formdan bağımsız, bütün veri yazma yollarında uygulanan kaynak tanımlarına dönüştürmelidir. [ArcGIS domains](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/create-modify-and-delete-domains.html), [ArcGIS veri kavramları](https://pro.arcgis.com/en/pro-app/3.5/help/data/geodatabases/overview/geodatabase-terminology.htm)

### 5.1 Dataset ve alan şeması

- [x] **FORM-DATA-01:** Geometrili layer ile bağımsız tablonun ortak DatasetSchema/record API'sini tasarla; ilişki için sahte geometri nesnesi üretme. — *Yapıldı (16 Ekim; ADR 0410: geometrisiz kayıt `record` geometrili nesnelerle aynı kayıt API'sinden; ilişki için sahte geometri yok).*
- [x] **FORM-DATA-02:** Dataset/field immutable IDs ve current storage names ayrışsın; alias/rename ref migration gerektirmeden çalışsın. — *Yapıldı (16 Ekim; ADR 0401 §2).*
- [ ] **FORM-DATA-03:** Primary/global UUID, unique keys, nullable, required, type, default, precision/length/range ve indexing alanlarını tanımla.
- [x] **FORM-DATA-04:** Date/time/datetime/timezone, reference/UUID, enum/collection, attachment ve geometry desteklerini aşamalı şema genişletmesiyle ekle. — *Yapıldı (16 Ekim; ADR 0412 tarih ve saat, saat; ADR 0409 başvuru/UUID ve çok değerli alan; ADR 0407 ekler; geometri nesnenin kendisi). Saat dilimi açık.*
- [ ] **FORM-DATA-05:** Mevcut canonical string attributes için lossless migration; null/blank/absent, unknown attr ve geçmiş kaydı koru.
- [x] **FORM-DATA-06:** Kayıt oluştururken incomplete draft ile committed/submitted kayıt farklı validation profile kullansın; eksik zorunlu değeri sessizce kalıcı geçerli kayıt yapma. — *17 Ekim: iş akışının taslak durumundaki kayıt eksik kaydedilir, formun zorunluluğu uyarı olur; geçerli kayıt isteyen adım tamlığı zorlar; katmanın zorunluluğu hep hata (ADR 0432).*
- [x] **FORM-DATA-07:** Field rename/drop/type change için bağlı form/domain/relation/processing/expression kullanım grafını göster; breaking değişiklik planlı migration olsun. — *Yapıldı (16 Ekim; ADR 0414: `cad.fields.usage`, Alanlar penceresinde kullanıldığı yerler ve silme uyarısı; yeniden adlandırmada ifade, şablon ve adlar aynı adımda izler, anahtar kalır. Tür değişimi ve işleme modelleri açık).*
- [x] **FORM-DATA-08:** Schema owner/design/publish izinleri data edit'ten ayrışsın; saha personeli feature.write ile şemayı değiştiremesin. — *Yapıldı (16 Ekim; ADR 0411: `forms.design`, `forms.publish` ayrı izin; Veri girişi şemayı değiştiremez).*
- [ ] **FORM-DATA-09:** Local `.kcad`, cloud dataset ve offline replica aynı schema/version/value codec'ini taşısın.

### 5.2 Domain ve subtype

- [ ] **FORM-DATA-10:** Domain registry: personal/project/tenant scope, owner, stable ID/version, label/description ve published/draft durumu. — *17 Ekim, kısmen: projenin değer listeleri (ADR 0430: kimlik, ad, açıklama; `cad.domains.edit`); kişi ve kurum kapsamı, sahip ve yayın durumu bulutta açık.*
- [ ] **FORM-DATA-11:** Coded domain; typed code/label, sıralama, localization, description, disabled/retired değerler ve geçerlilik aralığı taşısın. — *17 Ekim, kısmen: kod listesi kod, etiket, sıra ve kullanımdan kalkan kodla (ADR 0418, 0430); dil ve geçerlilik aralığı açık.*
- [ ] **FORM-DATA-12:** Range domain; inclusive/exclusive min/max, unit, exact decimal ve numeric/date type kurallarını desteklesin. — *17 Ekim, kısmen: aralık listesi dahil en az ve en çok, kesin ondalıkla (ADR 0430); birim ve tarih aralığı açık.*
- [x] **FORM-DATA-13:** Aynı domain birden çok dataset/field'da kullanılabilsin; kullanım grafı ve type mismatch doğrulaması olsun. — *17 Ekim: aynı liste birçok katmanın alanında, kullanan alanlar ve tür uyumsuzluğunun reddi (ADR 0430).*
- [x] **FORM-DATA-14:** Domain code veya type değişikliği schema/data migration gerektirsin; label değişimi kodu ve geçmiş veriyi değiştirmesin. — *17 Ekim: kod çıkarmak kayıtların göçünü (`recode`) ya da kullanımdan kaldırmayı ister, tür değişmez; etiket değişimi kodu ve kayıtları değiştirmez (ADR 0430).*
- [x] **FORM-DATA-15:** Retired domain değeri eski kayıtta görülebilsin; yeni seçim/yazma politikasını ayrı tanımla, kaydı otomatik boşaltma. — *Yapıldı (16 Ekim; ADR 0418: `retired`, tutan kayıtta kalır ve görünür, yeni seçimde sunulmaz, formla yeniden yazılmaz).*
- [ ] **FORM-DATA-16:** Domain listesi büyükse yetkili search/page/cache kullansın; tüm değerler her forma indirilmesin.
- [x] **FORM-DATA-17:** Subtype; discriminator field/code, subtype label ve alanlara özel domain/default/required/style bağlantısı taşısın. — *Yapıldı (17 Ekim; ADR 0422: `subtypes`, alanın değerleri, varsayılanı ve zorunluluğu; stil açık).*
- [x] **FORM-DATA-18:** Subtype değişiminde eski değerler için preserve/clear/revalidate önerisini kullanıcıya göster; geçerli değerleri gereksiz silme. — *Yapıldı (17 Ekim; ADR 0422 §2: değerler silinmez, `subtype_misfit`, formda Uymayanları boşalt).*
- [x] **FORM-DATA-19:** Contingent field groups; izinli combinations, wildcard/null, subtype ve restrictive/advisory policy ile sürümlensin. — *Yapıldı (17 Ekim; ADR 0423: gruplar, herhangi ve boş, alt türe özel bileşim, kısıtlayıcı ve uyarıcı).*
- [x] **FORM-DATA-20:** Üst seçim değişince alt seçenekler aynı rule engine ile filtrelensin; önceki değer artık geçersizse açık validation ver. — *Yapıldı (16 Ekim; ADR 0402 §1 (`choiceFilter`, `form_choice_filtered`)).*
- [x] **FORM-DATA-21:** Split/merge default/duplicate/sum/proportional gibi attribute politikaları domain/field modelinde açık olsun; geometri işlemlerine bağlantısı korunsun. — *Yapıldı (16 Ekim; ADR 0419: `split` ve `merge`, alan işlemleri ve Parçala, Kır, Birleştir aynı kuralla; toplam korunur).*
- [ ] **FORM-DATA-22:** Domain/subtype/contingent create/edit/publish/import/export işlemleri UI/Python/AI'da aynı komut olsun. — *17 Ekim, kısmen: değer listeleri arayüzde, Python'da ve ajanda aynı komut (`cad.domains.edit`, ADR 0430); alt türler ve bileşimler kendi komutlarıyla (ADR 0422, 0423); içe ve dışa aktarma açık.*
- [x] **FORM-DATA-23:** Final commit bütün API'lerde authoritative dataset/domain/contingent validation yapsın; form widget listesi tek denetim olmasın. — *Yapıldı (16 Ekim; ADR 0411 §4: Kuralları uygula açık formun katmanına her API'den gelen yazma sunucuda formun kurallarıyla denetlenir).*
- [x] **FORM-DATA-24:** Invalid legacy data için audit/repair raporu ve planlı correction sun; görüntüleme sırasında sessiz düzeltme yapma. — *Yapıldı (16 Ekim; ADR 0406 §1 (`cad.forms.audit`, Kayıtları denetle; düzeltme Formla düzenle ve `cad.forms.fill` ile)).*

**Kabul:** Aynı domain iki katmanda kullanılır, subtype ve üst seçim alt seçenekleri sınırlar. UI payload'ına elle illegal code yazan Python/HTTP/AI isteği aynı validation koduyla reddedilir.

## 6. FORM-REL — Kalıcı ilişkiler, join ve master–detail — P0/P1

QGIS ilişkili child form gösterimi ve reference widget desteğine sahiptir. ArcGIS de join, relate ve relationship class kavramlarını ayırır. KentOS'ta mevcut anahtarla değer aktarmayı kalıcı ilişki ve canlı join gibi sunmadan bu üç iş ayrı modellenmelidir. [QGIS ilişkiler](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/joins_relations.html), [ArcGIS ilişki seçenekleri](https://doc.esri.com/en/arcgis-pro/latest/help/data/relationships/data-relationship-options.html)

### 6.1 İlişki kaynak modeli

- [ ] **FORM-REL-01:** 1:1, 1:N ve N:M relation; dataset IDs, PK/FK, type, optionality/min/max ve stable relation ID taşısın.
- [ ] **FORM-REL-02:** N:M için gerçek link table; ilişki satırı kendi UUID/attributes/revision'ını taşıyabilsin.
- [x] **FORM-REL-03:** Association ile composition sahipliği ayrı olsun; restrict/cascade/unlink/set-null açık delete policy olarak tanımlansın. — *Yapıldı (16 Ekim; ADR 0405 §1–§2: unlink, cascade (bileşik), restrict, keep; bütün silme yollarında).*
- [ ] **FORM-REL-04:** Delete planı etkilenen children/link rows/attachments'ı saysın; cascade bütün source izinlerini ve transaction limitini doğrulasın.
- [ ] **FORM-REL-05:** FK type/unique/orphan/cardinality kuralları server transaction ve uygun DB constraints ile uygulanmalı.
- [ ] **FORM-REL-06:** Parent UUID unsaved create sırasında üretilebilir olsun; aynı submission'da child FK eşlemesi deterministik ve idempotent olsun.
- [ ] **FORM-REL-07:** Local/offline UUID eşlemesi sync sırasında korunmalı; slot/FID veya label anahtar olmamalı.
- [x] **FORM-REL-08:** Geometrili parent/child ve geometri olmayan table record ilişkileri aynı record API'sinden erişilsin. — *Yapıldı (16 Ekim; ADR 0410: geometrisiz kayıt `record`, `cad.records.create`; düzenleme, silme, form ve ilişkiler geometrili nesnelerle aynı komutlardan).*
- [ ] **FORM-REL-09:** Relation rename/type/cardinality/key change için kullanım grafı ve veri migration kontrolü olsun.

### 6.2 Master–detail runtime

- [x] **FORM-REL-10:** Form relationEditor; table/cards/tree ve child form görünümünü aynı relation üzerinden üretsin. — *Yapıldı (17 Ekim; ADR 0425: Liste ve Tablo, sütunlar, ondalık toplamlar; kart ve ağaç açık).*
- [ ] **FORM-REL-11:** Child create/update/delete/link/unlink/view/select-on-map action'ları ayrı yetki ve capability ile gösterilsin.
- [ ] **FORM-REL-12:** İlişkili lookup/relation reference; yetkili parent search, display fields, filter ve nested parent preview sunsun.
- [ ] **FORM-REL-13:** Parent data edit izni child edit izni vermesin; ilgili dataset/row/field/action yetkileri ayrıca denetlensin.
- [ ] **FORM-REL-14:** Parent/child patch'leri tek logical submission ve atomik transaction içinde uygulanabilsin; ayrı commit gerekiyorsa durum/kayıp sınırı açık olsun.
- [ ] **FORM-REL-15:** Detail row pagination/virtualization, lazy tab loading ve aggregate count filtrelenmiş izinli veri üzerinden çalışsın.
- [ ] **FORM-REL-16:** Child editing ve parent tab değişimi draft kaybettirmesin; kaydet/vazgeç ve validation summary bütün graph'ı kapsasın.
- [ ] **FORM-REL-17:** Relation graph döngüsü/derinlik ve recursive form opening bütçesi tanımlansın.
- [x] **FORM-REL-18:** Duplicate/clone/split/merge işlemlerinde child ownership ve copying policy açık olsun; unwanted cascade copy yapma. — *Yapıldı (16 Ekim; ADR 0415: bileşik ilişkinin alt kayıtları kopyalanır, öbürleri kopyalanmaz; tekil alan boş kalır).*

### 6.3 Join türleri ve yazma yönlendirmesi

- [ ] **FORM-REL-19:** Materialized value transfer, live joined view, relationship reference ve spatial lookup ayrı source/binding tipleri olsun.
- [ ] **FORM-REL-20:** Join key type/collation/null ve duplicate match policy açık seçilsin; ilk eşleşmeyi sessiz doğru kaynak sayma.
- [ ] **FORM-REL-21:** 1:1/M:1 live join için source field lineage, source revision, missing/multiple match ve refresh policy dönsün.
- [ ] **FORM-REL-22:** N:M join sonuçlarında satır çoğalması/count/aggregate semantics açık olsun; ambiguous field edit varsayılan reddedilsin.
- [ ] **FORM-REL-23:** Join edit gerekiyorsa exact canonical source record/field'ı hedefleyen typed command kullansın; her iki kaynağın izinleri ayrı doğrulansın.
- [ ] **FORM-REL-24:** Cross-service edit distributed transaction gibi gösterilmesin; source capability'ye göre read-only veya explicit workflow/saga kullan.
- [ ] **FORM-REL-25:** Source erişimi kaldırılınca join/lookup/result cache masklensin ve yeni istek reddedilsin; daha önce indirilmiş verinin geri silinebildiği garanti edilmesin.
- [ ] **FORM-REL-26:** Query count, autocomplete, aggregate, event ve AI context joined kaynak ACL'lerini de izlesin.

**Kabul:** Bina formunda denetimler 1:N, malik bağlantıları N:M olarak görüntülenip yetkiye göre düzenlenir. Joined owner display alanına yazmak bütün owner kayıtlarını değiştirmez; ambiguous yazma reddedilir.

## 7. FORM-SRC — Başka kaynaktan veri alma ve güvenilir lookup — P1

- [ ] **FORM-SRC-01:** Source registry; KentOS dataset/table, static dictionary, file table, kurumsal API ve desteklenen servis adapter'larını aynı reference modelinde sunsun.
- [ ] **FORM-SRC-02:** Source connection/credential user/tenant sahipliği ile saklansın; form JSON'u veya mobil offline bundle parola/API key taşımasın.
- [ ] **FORM-SRC-03:** Lookup/query şeması parametre, projection, sort, limit/cursor ve source version taşısın; serbest SQL/URL string normal binding yolu olmasın.
- [ ] **FORM-SRC-04:** Lookup input'u typed parameters olarak bind edilsin; injection, key/type/collation ve özel karakterler denetlensin.
- [x] **FORM-SRC-05:** Cascading choice örnekleri kur: il→ilçe→mahalle, varlık tipi→model→parça, subtype→domain kombinasyonu. — *Yapıldı (16 Ekim; ADR 0404 §1: il → ilçe → mahalle zinciri başka katmanlardan; varlık tipi ve subtype örnekleri sonra).*
- [ ] **FORM-SRC-06:** Autocomplete debounce/cancel, minimum search, pagination ve stale response sequence; eski sonuç yeni alana uygulanmasın.
- [ ] **FORM-SRC-07:** Value copy ile live display/reference ayrı eylemler olsun; kopyalanan değerin kaynak/tarih/revision provenance'ı kalsın.
- [ ] **FORM-SRC-08:** GIS spatial lookup; picked feature, containing/nearest/intersecting kaynak ve CRS/tolerance politikasını açık alsın.
- [ ] **FORM-SRC-09:** Harici kaynak timeout/offline/unauthorized/multiple match/changed result durumlarını ayrı validation ve retry ile sunsun.
- [ ] **FORM-SRC-10:** Dış network bağlantıları broker/server adapter'ından, seçilmiş endpoint ve scope ile yürüsün; form açılması arbitrary network execution olmasın.
- [ ] **FORM-SRC-11:** Cache; user/tenant/permission/source-revision/query bazlı ayrışsın; private lookup sonucu başka kullanıcıya verilmesin.
- [ ] **FORM-SRC-12:** Offline paket yalnız görev için izinli reference data subset'ini taşısın; cache expiry ve stale domain davranışı açık olsun.
- [ ] **FORM-SRC-13:** Server-required validation varsa offline kayıt taslak/pending kalabilsin; kullanıcıya tamamlandı/onaylandı gibi gösterilmesin.
- [ ] **FORM-SRC-14:** Lookup action isteği caller context'ini devralsın; plugin veya AI farklı hesap kullanarak source yetkisini genişletemesin.
- [ ] **FORM-SRC-15:** Yeni source provider plugin ile gelebilir; descriptor/izin/schema ve onaylı version/digest bağımlılığı form deployment'ta görünür olsun.

**Kabul:** Bir alan başka katman veya yetkili servisten seçim/değer alır; kullanıcı kaynak verisini göremiyorsa form, autocomplete ve aggregate de onu sızdırmaz.

## 8. FORM-RULE — Hesap, validation ve veri yazma standardı — P0/P1

- [ ] **FORM-RULE-01:** Rule profili form/widget validation ile authoritative dataset/workflow constraint'i ayırsın; aynı kural kaynağına reference kullanılabilsin.
- [x] **FORM-RULE-02:** Veri bütünlüğü için zorunlu koşullar bütün yazma girişlerinde uygulanmalı: form, table, command, import, Python, plugin ve AI. — *Yapıldı (16 Ekim; ADR 0401 §8, ADR 0402 §4 (belgenin izlemesi)).*
- [x] **FORM-RULE-03:** Yalnız form davranışı olan görünürlük/presentation kuralı dataset verisini otomatik silmesin veya güvenlik izni üretmesin. — *Yapıldı (16 Ekim; ADR 0401 §6; silme yalnız formun `clearHidden`'ıyla, ADR 0402 §1).*
- [x] **FORM-RULE-04:** Conditional required ile görünmez alan çelişkisini compiler/preflight yakalasın; kullanıcı dolduramadığı mandatory alanla kilitlenmesin. — *Yapıldı (16 Ekim; ADR 0401 §4).*
- [ ] **FORM-RULE-05:** Field-level, cross-field, cross-record, relation/cardinality, geometry/topology ve workflow validation farklı scope'larla tanımlansın.
- [x] **FORM-RULE-06:** Error/warning/info; stable code, field/widget/record path, source ve mesaj parametreleri taşısın. — *Yapıldı (16 Ekim; ADR 0401 §4, ADR 0402 (uyarı denetimi)).*
- [x] **FORM-RULE-ek:** Form değişince eski kayıtların hesapları tek komutla, geri alınabilir biçimde yenilensin. — *Yapıldı (17 Ekim; ADR 0421: `cad.forms.recompute`, Formlar penceresinde Hesapları yenile).*
- [ ] **FORM-RULE-07:** Validate-on-change/blur/save/submit politikaları olsun; hesap sonucu authoritative commit validation'ı azaltmasın.
- [x] **FORM-RULE-08:** Computed field; pure computed/read-only, persisted computed veya user-overridable modunu açık taşısın. — *Yapıldı (16 Ekim; ADR 0417: kilitli hesap ve değiştirilebilir hesap (`overridable`); saf hesap olmayan kalıcı alan).*
- [ ] **FORM-RULE-09:** Recompute trigger; source değişimi, related count/aggregate, geometry, subtype/domain ve lookup revision üzerinden dependency graph'ta izlesin.
- [ ] **FORM-RULE-10:** Kurallar belirsiz async lookup sonucu beklerken “geçerli” sayılmasın; pending/blocked durumları ve timeout politikası olsun.
- [x] **FORM-RULE-11:** Calculated ve user-entered değerin çakışması merge/reset politikasıyla görünür olsun; geç ağ cevabı kullanıcının yeni değerini ezmesin. — *Yapıldı (16 Ekim; ADR 0417: yazılan değer hesabı ezmez, ↺ ile hesaba dönülür; geç ağ cevabı yok, hesap yereldir).*
- [ ] **FORM-RULE-12:** Rule engine offline/native/web/server'da null, boolean, decimal, date/timezone ve comparison bakımından aynı çalışsın.
- [x] **FORM-RULE-13:** Sunucu authoritative rule/data profile'ı seçsin; client başka form/version ID vererek mandatory kuralı atlayamasın. — *Yapıldı (16 Ekim; ADR 0401 §8).*
- [x] **FORM-RULE-14:** JSON patch hedefleri field IDs ve record refs ile whitelist'li olsun; UI hidden/read-only alanlarına forged write reddedilsin. — *Yapıldı (16 Ekim; ADR 0401 §6, §8).*
- [ ] **FORM-RULE-15:** Form submission expected versions, accepted schema/domain/form releases ve idempotency ile doğrulansın.
- [ ] **FORM-RULE-16:** Tüm doğrulama ve related writes DB transaction'da veya güvenilir staged commit'te birlikte uygulansın; network/attachment upload beklenirken DB transaction açık kalmasın.
- [ ] **FORM-RULE-17:** Validation için gerekli kaynağa erişim yoksa veya snapshot değişmişse stale/needs_input/conflict sonucu dönsün; eski computed değer final gerçek gibi kaydedilmesin.
- [ ] **FORM-RULE-18:** Form draft, workflow submit ve approved state ayrı olsun; geçerli veri automatically approved veri değildir. — *17 Ekim, kısmen: taslak, gönderilmiş ve onaylı durumlar iş akışıyla ayrı; taslakta eksik kayıt, ilerlemede tamlık (ADR 0428, 0432); rollerle onay bulutta açık.*

**Kabul:** “Ek inceleme gerekli ise not zorunlu” kuralı sunucuya bağlanmış authoritative write profile'da uygulanır. UI kuralını silen veya generic feature command kullanan istemci bu veriyi notsuz final kaydedemez.

## 9. FORM-DESIGN — Gelişmiş form tasarımcısı — P1

QGIS'in tab/group/relation/widget yaklaşımı ve ArcGIS Field Maps'in visible/required/editable/calculated expression yaklaşımı başlangıç kıyasıdır. KentOS'ta gelişmişlik; daha çok kontrolün yanında **taşınabilirlik, merkezi sürüm/yayın, yetki, offline, ilişki, workflow ve test** ile ölçülmelidir. [QGIS tasarımcı](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html#the-drag-and-drop-designer), [Field Maps form](https://doc.arcgis.com/en/field-maps/latest/prepare-maps/configure-the-form.htm)

- [ ] **FORM-DESIGN-01:** Tasarımcı; dataset/field/domain/relation paleti, component paleti, layout tree/canvas ve property/rule inspector içersin.
- [ ] **FORM-DESIGN-02:** Drag/drop ve keyboard reorder; tabs, groups, accordion, grid/columns, splitter, stepper ve reusable section desteklesin.
- [ ] **FORM-DESIGN-03:** Field ID üzerinden binding; alias/help/placeholder/widget/format/domain seçimiyle type-safe yapılsın.
- [ ] **FORM-DESIGN-04:** Aynı field birden çok yerde gösterilebilir; tek draft value owner'ı olsun, mirror display ile bağımsız edit kutuları birbirini ezmesin.
- [ ] **FORM-DESIGN-05:** Master–detail, relation reference, embedded child form, lookup panel ve record grid eklenebilsin.
- [ ] **FORM-DESIGN-06:** Hesaplı summary, related aggregate, chart, image/attachment, map location ve inline validation bileşenleri olsun. — *17 Ekim, kısmen: hesaplı not, ilişkili kayıtların tablosu ve toplamları, ekler, harita konumu ve satır içi doğrulama (ADR 0402, 0407, 0425, 0433); grafik açık.*
- [ ] **FORM-DESIGN-07:** Choice widget; dropdown/radio/chips/autocomplete/tree seçeneği domain kodunu değiştirmeden sunulabilsin. — *17 Ekim, kısmen: açılır liste, seçenek düğmeleri, parçalı seçim, çoklu seçim, aranır liste ve etiketler (ADR 0401, 0408, 0431); ağaç açık.*
- [x] **FORM-DESIGN-08:** Rule builder; görsel koşul düzenleyici ve expression editörü aynı AST/programı değiştirsin. — *Yapıldı (16 Ekim; ADR 0401 §10: İfade oluşturucunun Metin ve Akış'ı).*
- [x] **FORM-DESIGN-09:** Required/visible/editable/calculate/default/choice filter bağları dependency graph'ta görülebilsin. — *Yapıldı (16 Ekim; ADR 0414 §5: tasarımcıda Bağlar; döngü ön denetimde `form_calc_cycle`).*
- [ ] **FORM-DESIGN-10:** Rules/sections için reuse/template desteği olsun; nested component version ve binding parametreleri açık olsun.
- [x] **FORM-DESIGN-11:** Conditional section/tab, subtype bazlı view, record status bazlı read-only ve task mode tasarlanabilsin. — *Yapıldı (17 Ekim; ADR 0426: koşullu sekme ve alt türe göre görünüm; kayıt durumuna göre salt okunur ve görev kipi iş akışıyla açık).*
- [ ] **FORM-DESIGN-12:** Layout pixel çizimi yerine responsive constraints/layout hints taşısın; desktop/web/mobile preview aynı kaynak tanımı kullansın.
- [x] **FORM-DESIGN-13:** Tasarımcı mock/test record ve seçilmiş gerçek yetkili record ile preview sunsun; test verisi production kaydına yazılmasın. — *Yapıldı (16 Ekim; ADR 0403 §3: seçili gerçek kayıtta ya da yeni kayıtta önizleme, hiçbir şey yazılmaz).*
- [ ] **FORM-DESIGN-14:** Role/task preview sunucunun effective grant modelini simüle etsin; designer rolü gerçek saha kullanıcısına impersonation yetkisi vermesin.
- [ ] **FORM-DESIGN-15:** Validation preflight; missing field/domain/action, unsupported mobile widget, rule cycle, required-hidden conflict ve source izin ihtiyaçlarını listelesin.
- [ ] **FORM-DESIGN-16:** Theme tokens, accessibility labels, error location, tab/focus order ve localization editörü olsun.
- [ ] **FORM-DESIGN-17:** Tasarım undo/redo, draft autosave, source diff, version compare ve review comments desteklesin. — *17 Ekim, kısmen: geri alma ve yineleme (ADR 0403), sürüm karşılaştırması (ADR 0427: `cad.forms.compare`, Değişiklikler); kendiliğinden kayıt ve yorumlar açık.*
- [x] **FORM-DESIGN-18:** Schema/domain değişikliği form kullanımını etkilediğinde impact view ve migration önerisi sun. — *Yapıldı (16 Ekim; ADR 0414: Alanlar penceresinde alanın kullanıldığı yerler, yeniden adlandırmada kuralların izlenmesi, silmede uyarı).*
- [ ] **FORM-DESIGN-19:** Pure form ile extension/Python-dependent form ayrımı tasarımcıda görülsün; mandatory mobile executor eksikliği yayın engeli olsun.
- [ ] **FORM-DESIGN-20:** Form şablonu import/export veya `.kek` içinden alındığında source refs ve ACL yeniden bağlansın; başka tenant'ın grant'i kopyalanmasın.
- [ ] **FORM-DESIGN-21:** Eklenen action yalnız kayıtlı descriptor'dan seçilsin; raw SQL/URL/script normal field property'ye yazılamasın.
- [ ] **FORM-DESIGN-22:** Tasarım çıktısı portable JSON olsun; bütün formu platforma özel Python/TS UI kodu olarak saklama.
- [ ] **FORM-DESIGN-23:** AI tasarım önerisi düzenlenebilir draft/diff olsun; doğrudan published formu değiştirmesin.
- [x] **FORM-DESIGN-24:** Kullanıcı kendi form test senaryolarını kaydedebilsin; replay values/validation/action görünürlüğünü otomatik karşılaştırsın. — *Yapıldı (16 Ekim; ADR 0420: senaryolar, `cad.forms.test`, tasarımcıda Senaryolar).*

**Kabul:** Tasarımcı aynı formda sekme, master–detail, domain, cascading lookup ve calculated summary kurar; üç platform preview'da bütün field/rule kimlikleri aynıdır. Published form ayrı draft açılmadan değişmez.

## 10. FORM-ACL — Form tasarımı, veri girişi ve merkezi yetkilendirme — P0

### 10.1 Ayrı izinler ve rol örneği

İzin adları **önerilen yeni kaynak izinleridir**; mevcut `project.edit`/`feature.write` adlarını yeniden yorumlayan hazır bir uygulama yoktur. Mevcut ProjectAccess altyapısı korunup form/dataset/row/field/workflow kapsamlarıyla genişletilmelidir.

| İzin ailesi | Anlam |
|---|---|
| `forms.read/use` | Yetkili form tanımını görme/kullanma |
| `forms.design` | Taslak layout/rule/binding değiştirme |
| `forms.review/publish` | İnceleme ve form release/deployment yayınlama |
| `schemas/domains/relations.manage` | Veri kaynak kontratını değiştirme; form design'dan da ayrı |
| `datasets.read/create/update/delete` | İzinli veri kaynağındaki kayıt işlemleri |
| `fields.read/write` | Hangi alanları okuyup değiştirebildiği |
| `relations.link/unlink` | İzinli kaydı ilişkilendirme; child update'ten ayrı |
| `forms.submit` / `records.review` | Workflow geçişleri ve veri onayı |
| `assignments.manage` | Kullanıcı/görev/bölge/deployment atama |
| `attachments.read/create/delete` | Eklerin scope ve yaşam döngüsü |

| Rol örneği | Form tasarla | Form yayınla | Veri oku/yaz | Veri onayla |
|---|---|---|---|---|
| Form tasarımcısı | Yetkili draft | Ayrı publish grant varsa | Ayrı dataset grant varsa | Ayrı review grant varsa |
| Yayın sorumlusu | Ayrı design grant varsa | Yetkili deployment | Ayrı dataset grant varsa | Ayrı review grant varsa |
| Saha veri giriş elemanı | Hayır | Hayır | Atanmış kayıt/alan/görev kapsamında | Hayır |
| Kontrolör | Hayır | Hayır | Kontrol görevine izinli veri | Yetkili kayıt/workflow |
| Görüntüleyici | Hayır | Hayır | İzinli projection, read-only | Hayır |

Rol adları şablondur; kişi/grup/kurum ve kaynak kapsamıyla gerçek grant verilir. Form sahibi olmanın bütün dataset verisine erişim verdiği varsayılmamalıdır.

### 10.2 Otorite ve yazılabilirlik

`effective editable = source/dataset grant ∩ row/task scope ∩ field write policy ∩ deployment/workflow policy ∩ form rule`.

Formdaki editable kuralı yalnız kısıtlama getirebilir, kaynak yetkisini yükseltemez. Client, role/user/permission değerlerini payload'a yazıp yetki elde edemez; actor verified oturumdan gelir.

- [ ] **FORM-ACL-01:** Form resource ve data resource yetkilerini ayrı server access resolver üzerinden değerlendir; UI disabled durumunu güvenlik sayma.
- [ ] **FORM-ACL-02:** Form tasarımını değiştirme endpoint'i sadece forms.design; yayın endpoint'i ayrı publish/review grant gerektirsin.
- [ ] **FORM-ACL-03:** FeatureWrite sahibi saha kullanıcısı form JSON'u, domain, relation ve source credential değiştiremesin.
- [ ] **FORM-ACL-04:** Form deployment audience ve task assignment; kişi/grup/bölge/dataset/record türü ve süre sınırını taşısın.
- [ ] **FORM-ACL-05:** Row policy; assigned-to-user, created-by-user, workflow status, spatial/task kapsamı gibi kuralları authoritative actor ile uygulasın.
- [ ] **FORM-ACL-06:** Fields read/write allowlist; payload/projection/query/aggregation/search/event/export ve AI result'ta aynı policy'yi uygulasın.
- [ ] **FORM-ACL-07:** Gizli field ve dataset, önceden form binding'i bilinse bile query API'den dönmesin; schema/lookup labels de hassas sayılabiliyorsa projection'a dahil et.
- [ ] **FORM-ACL-08:** Viewer/read-only form, sahte patch göndererek data update yapamasın; generic command/table/import route policy'yi atlayamasın.
- [ ] **FORM-ACL-09:** Parent izinleri child/linked/joined source için ayrı değerlendirilir; orphan/policy-denied child bilgisi count veya autocomplete ile sızmasın.
- [ ] **FORM-ACL-10:** Submit sırasında güncel yetki, assignment ve record versions yeniden doğrulansın; eski form session grant'i sınırsız süreli kullanılmasın.
- [ ] **FORM-ACL-11:** UI rule engine'e verilen `$user/$task/$permissions` bağlamı broker/server tarafından filtrelenmiş olsun; trusted auth claim client'tan alınmasın.
- [ ] **FORM-ACL-12:** Kurum admin/owner policy ile explicit source/form grant çatışmasını açık çöz; mevcut projenin highest-role modelini dar field policy'nin yerine geçirme.
- [ ] **FORM-ACL-13:** Server dataset RLS ve field/projection/write guard ekle; application resolver tek kaynaktan izin kararı üretsin.
- [ ] **FORM-ACL-14:** Unauthorized source/form/record enumeration; list/count/search/cache ve event akışlarında engellensin.
- [ ] **FORM-ACL-15:** Local imported package ACL claim'i güvenilir sayılmasın; cloud'a alınırken server deployment/grant yeniden oluşturulsun.
- [ ] **FORM-ACL-16:** Çalışan form/long job sırasında erişim iptali fark edilip yeni sorgu/yazma durdurulsun; sonuç commit'i tekrar izin denetlesin.
- [ ] **FORM-ACL-17:** Audit; actor, form/deployment/version, data diff, task/source, validation/review ve event zamanını sır içermeden tutsun.
- [ ] **FORM-ACL-18:** Form designer'ın role preview veya AI generated JSON'u production ACL değiştiremesin; permission changes ayrı denetlenmiş komut olsun.
- [ ] **FORM-ACL-19:** Export/download politikası ile görüntüleme ayrı olsun; görüntülenmiş/indirilmiş verinin geri alınabilir olduğu iddia edilmesin.
- [ ] **FORM-ACL-20:** Kurumsal rollout/migration mevcut izinleri sessiz genişletmesin; form read-only/user access değişikliği impact raporuyla uygulansın.

**Kabul:** Merkezde tasarımcı formu hazırlar. Saha personeli atanmış kaydı ve izinli alanları girer; form tasarımını değiştiremez. Tablo/Python/AI/ham API denemesiyle başka kayıt veya yasak field'a yazamaz.

## 11. FORM-PUB — Merkezi form yayınlama, sürüm ve dağıtım — P0/P1

Eklenti release onayı ile form tasarımı yayını farklı kaynak lifecycle'larıdır. Form, eklentiyle gelebilir fakat bağımsız olarak merkezi tanım ve deployment olarak yayınlanır; her saha kullanıcıya Python eklentisi kurdurmak gerekmez.

Önerilen akış: `draft → validation/preview/tests → review → approved → published deployment → deprecated/withdrawn`. Yayınlanan release immutable'dır; değişiklik yeni draft/release oluşturur.

- [ ] **FORM-PUB-01:** Form ID, release revision/digest, owner, tenant/project scope, locale ve compatible schema refs saklansın.
- [ ] **FORM-PUB-02:** Tasarımcı kendi taslağını düzenlesin; shared editing varsa draft revision conflict ile korunmalı.
- [ ] **FORM-PUB-03:** Publish öncesi form compiler, source binding, permission/capability preflight ve kayıtlı form testleri çalışsın.
- [ ] **FORM-PUB-04:** Merkezden yayınlayacak yetkili rol ve gerekirse reviewer policy tanımlansın; data collector publish yetkisi almasın.
- [ ] **FORM-PUB-05:** Release onayı form definition + schema/domain/relation/action/source version digests'ine bağlansın; breaking kaynak değişimi yeniden değerlendirme gerektirsin.
- [ ] **FORM-PUB-06:** Deployment; published version, hedef dataset mapping, effective validation/write policy, audience ve task paketini bağlasın.
- [ ] **FORM-PUB-07:** Personal/local form ile kurum formu ayrı listelensin; local designer kişisel kopyasını değiştirse bile kurum yayını değişmesin.
- [ ] **FORM-PUB-08:** “Edit published” yeni version draft açsın; sahadaki aktif formu anlık bozmasın.
- [ ] **FORM-PUB-09:** Rollout; pilot/ekip/genel audience, başlangıç zamanı, previous/current ve rollback referanslarıyla çalışsın.
- [ ] **FORM-PUB-10:** Aktif session form version'a bağlı kalsın; yeni versiyon geldiğinde draft/value migration veya kullanıcıya seçim sunulsun.
- [ ] **FORM-PUB-11:** Kabul edilen eski form/schema sürümleri ve grace/offline window açık olsun; eski client yeni zorunlu field/rule'ı atlayarak commit yapamasın.
- [ ] **FORM-PUB-12:** Breaking domain/code/field/relation değişikliği data migration planı ve server validation profile güncellemesiyle birlikte yayınlansın.
- [ ] **FORM-PUB-13:** Withdraw/revoke yeni session/submit politikasını uygulasın; eski saha taslağı silinmeden pending/conflict olarak tutulabilsin.
- [ ] **FORM-PUB-14:** Form template export/import ve project duplicate; resource kimliklerini remap etsin, user grants/credentials'i kopyalamasın.
- [ ] **FORM-PUB-15:** `.kcad` local/portable form resources taşırken cloud authority claims server'dan doğrulansın; dosya içi claim ile yeni grant yaratılmasın.
- [ ] **FORM-PUB-16:** Version diff; layout, binding, domain/rule, role/task ve action dependency etkilerini anlaşılır göstersin.
- [ ] **FORM-PUB-17:** Kullanıcı bir formu “gör/doldur/read-only” modunda açarken effective capability ve permission kaynaklarını görebilsin.
- [ ] **FORM-PUB-18:** Form/deployment/schema server endpoint'leri Python/MCP/AI çağrılarında aynı lifecycle ve yetkiyi kullansın.

**Kabul:** Merkezde v1 yayınlanır; saha v1 ile çalışırken tasarımcı v2 taslağını hazırlar. v2 kontrollü yayınlanır; eski taslaklar veri kaybetmeden ve eski mandatory rule'ı atlayamadan senkronize edilir.

## 12. FORM-MOBILE — Mobil/web runtime ve çevrimdışı saha akışı — P1/P2

Mobil hedef bu planda **form/veri girişi istemcisidir**; bütün masaüstü CAD uygulamasını telefona taşımak değildir. Form tanımı ortak olsa da mobil istemcinin auth, query, offline, attachment ve UI adapter'ları ayrıca uygulanmalıdır.

### 12.1 Çalışma matrisi

| Yetenek | Masaüstü | Web | Mobil |
|---|---|---|---|
| Portable form/domain/rules | Native renderer/çekirdek | Web renderer + WASM | Mobil renderer + uyumlu rule çekirdeği |
| Python plugin GUI | Yerel izole Python + KentOS UI | Uyumlu worker veya seçilmiş yetkili server runtime | Portable UI + varsa seçilmiş server action |
| uv ortam/paket yönetimi | Kullanıcı-local | Browser'da desktop uv işlemi varsayılmaz | Desktop native wheel/uv kurma şartı yok |
| Processing plugin | Desktop executor | Capability'ye göre browser Python veya server | Capability'ye göre portable/local destek veya server |
| Offline form veri girişi | Local replica | İzinli browser store | Görev bazlı local replica |
| Camera/GNSS/barcode | Cihaz capability'ye göre | Browser izin/capability | Mobil cihaz izin/capability |

Bir masaüstü Python paketinin web/mobile'da çalıştığı yalnız manifest iddiasıyla kabul edilmemelidir. Formun portable kısmı Python'suz çalışmalıdır; server-dependent action offline iken unavailable/pending görünmelidir. Veri upload gerektiren alternatif kullanıcı/kurumun seçilmiş politikasına bağlı olmalıdır.

- [ ] **FORM-MOBILE-01:** Ortak FormSession/draft/validation ve renderer interface'i üç platform için tanımla.
- [ ] **FORM-MOBILE-02:** Device capability discovery; camera, gallery, location/GNSS, barcode/QR, file/asset, offline ve required widget'ları bildirsin.
- [ ] **FORM-MOBILE-03:** Cihaz izinleri ve veri erişim izinleri ayrı olsun; kamera izni tüm kaynak dataset'lere erişim vermesin.
- [ ] **FORM-MOBILE-04:** Map feature seçimi, geometry capture/edit ve coordinate/CRS/accuracy metadata form kayıtlarına kontrollü bağlansın.
- [ ] **FORM-MOBILE-05:** Touch/keyboard focus, hata alanına gitme, küçük ekran relation editor ve erişilebilir target size testleri yap.
- [ ] **FORM-MOBILE-06:** Offline task package; immutable form release, schema/domain/relations, authorized record/reference data subset, expiry ve policy receipt taşısın.
- [ ] **FORM-MOBILE-07:** Paket yalnız görev için gerekli alan/satır/attachment verisini içersin; bütün proje default offline download olmasın.
- [ ] **FORM-MOBILE-08:** Offline create/update/delete/link işlemleri local UUID + source revision + operation queue ile saklansın.
- [ ] **FORM-MOBILE-09:** Parent/child yeni kayıtlar ve attachment outbox aynı logical submission'a bağlansın; FK eşlemesi retry'da değişmesin.
- [ ] **FORM-MOBILE-10:** Local validation server-independent kuralları uygulasın; source/server gerektiren kural pending/needs-online olarak görülsün.
- [ ] **FORM-MOBILE-11:** Sync; current actor/assignment/ACL/schema/domain/version tekrar doğrulasın, idempotent commit ve attachment finalize yapsın.
- [ ] **FORM-MOBILE-12:** Conflict resolution; field diff, geometry, related rows, deleted parent ve schema migration etkisini göstersin; last-write-wins sessiz varsayılan olmasın.
- [ ] **FORM-MOBILE-13:** Yetki/schema değişiminde local taslak korunmalı; server rejected sonuç düzeltilebilir conflict/blocked olarak kalmalı.
- [ ] **FORM-MOBILE-14:** Offline cihazdan önceden indirilmiş veri anında geri alınamaz. Expiry/minimal projection/encrypted store politikası ve sync commit reddi gerçek sınırlarıyla uygulanmalı.
- [ ] **FORM-MOBILE-15:** Device account switch/logout ve shared-device cache izolasyonu; başka kullanıcı öncekinin draft/record/secret verisini görmesin.
- [ ] **FORM-MOBILE-16:** Attachment/GNSS timestamps, timezone ve accuracy provenance korunsun; file path yerine taşınabilir asset handle kullan.
- [ ] **FORM-MOBILE-17:** Browser quota/eviction, mobile low-memory/connection loss ve app termination için draft/outbox recovery sınansın.
- [ ] **FORM-MOBILE-18:** Sync durumları kullanıcıya ayırt edilebilir olsun: local draft, pending upload, validation blocked, conflict, committed, submitted, approved.
- [ ] **FORM-MOBILE-19:** Aynı formda role/read-only/missing-source/server-action davranışı platformlar arasında aynı result codes kullansın.
- [ ] **FORM-MOBILE-20:** Web/mobile Python executor ve package support ayrıca capability/test matrisiyle yayımlansın; form kullanmak native plugin kurmayı zorunlu kılmasın.

**Kabul:** Saha elemanı formu mobilde offline doldurur, yeni parent/child ve fotoğraf kaydeder; bağlantı gelince izin ve sürüm doğrulanarak aynı server record graph'ına senkronize olur. UV veya NumPy mobil veri girişi için gerekmemektedir.

## 13. FORM-WORK — Taslak, veri onayı, ekler ve işlem akışları — P1

- [ ] **FORM-WORK-01:** Veri workflow state graph'ı draft/submitted/in_review/approved/rejected/returned gibi açık sürümlü tanım taşısın; form publication state ile karışmasın. — *17 Ekim, kısmen: yerel iş akışı (ADR 0428: durumlar, adımlar, kilitli durumlar; yayından ayrı); rollerle bulutta açık.*
- [ ] **FORM-WORK-02:** Her transition actor/task/record izinleri, required fields ve domain/relation validation ile denetlensin. — *17 Ekim, kısmen: adım kaydın durumundan, geçerlilik ve gerekçeyle denetlenir (ADR 0428); aktör ve görev yetkileri bulutta açık.*
- [ ] **FORM-WORK-03:** Saha personeli submitted kaydı kendisi approved yapamasın; reviewer kararı audit/history ve beklenen record revision taşısın. — *17 Ekim, kısmen: adımlar kaydın geçmiş alanına an, kişi, adım ve gerekçeyle yazılır (ADR 0429); kendi kaydını onaylamanın yasağı rollerle bulutta açık.*
- [x] **FORM-WORK-04:** Onaylanan kayıt read-only veya correction workflow'da düzenlenebilir olsun; user seçimi sunucu politikasını değiştiremesin. — *17 Ekim: onaylı kayıt kilitli durumda salt okunur, düzeltme adımıyla açılır (ADR 0428).*
- [ ] **FORM-WORK-05:** Return/reject; field/path bazlı geri bildirim ve düzeltilebilir taslak üretsin; eski submission snapshot'ı kaybolmasın. — *17 Ekim, kısmen: geri gönderme gerekçe alanıyla (ADR 0428); alan bazlı geri bildirim ve eski anlık görüntü açık.*
- [ ] **FORM-WORK-06:** Parent/child/attachment ve calculated fields aynı submission revision'a bağlansın; reviewer hangi snapshot'ı onayladığını bilsin.
- [ ] **FORM-WORK-07:** Fotoğraf/dosya/audio gibi ekler MIME/size/hash, owner, record relation, capture timestamp ve erişim policy taşısın.
- [ ] **FORM-WORK-08:** Attachment upload staging/resumable/check/finalize ve orphan cleanup akışı olsun; kayıt commit edilmeden uzun upload DB transaction tutmasın.
- [ ] **FORM-WORK-09:** Unsaved parent ve offline child ekleri local UUID/job IDs ile bağlansın; retry duplicate attachment veya child oluşturmasın.
- [ ] **FORM-WORK-10:** Geçersiz medya/eksik upload/permission failure sonucu submission incomplete olarak kalsın; kullanıcıya başarılı final kayıt denmesin.
- [ ] **FORM-WORK-11:** Form action; calculate/lookup/processing/report/submit/assign gibi açık descriptor tipi taşısın, side-effect ve required grant belirlensin.
- [ ] **FORM-WORK-12:** Python-dependent action release/env/runtime capability ile pinlensin; mobile/web kullanımı server executor'a açık ve izinli bağlansın.
- [ ] **FORM-WORK-13:** Action output field mapping authoritative write policy ile doğrulansın; hesap onaylı kaydı veya forbidden field'ı sessiz değiştirmesin.
- [ ] **FORM-WORK-14:** Görev listesi, assigned record queue, progress ve review feedback aynı task/deployment modelinden gelsin.
- [ ] **FORM-WORK-15:** Workflow event, audit ve kullanıcı bildirimleri veri yetkisine göre filtrelensin; private kaydı yanlış audience'a duyurmasın.

**Kabul:** Veri giriş elemanı formu submitted yapar, kontrolör aynı kaynak snapshot'ı onaylar veya düzeltmeye döndürür. Form tasarımcı rolü veya AI çağrısı veri onay rolünün yerine geçmez.

## 14. FORM-API — UI, komut, Python ve AI eşdeğerliği — P0/P1

Önerilen komut aileleri aşağıdadır; public SDK isimleri mevcut `kentos.cad`/katalog uyumu gözetilerek ADR'de kesinleştirilir. Form runtime'ı Python'suz çalışır; Python facade bu aynı komutların otomasyon istemcisidir. Tam plugin environment/uv/.kek lifecycle'i [python.md](python.md) kapsamındadır.

| Aile | Kapsam |
|---|---|
| `datasets.schemas.*` | Schema/field/type/default/constraint/design/migration ve mevcut field adapter |
| `domains.*` / `subtypes.*` | Domain/subtype/contingent create/edit/version/publish |
| `relations.*` / `sources.*` | Link/query/join/lookup ve scope'lu source connection |
| `forms.*` | Create/draft/edit/compile/preview/validate/version/review/publish/import/export |
| `forms.deployments.*` / `assignments.*` | Audience/data mapping/task/version ve effective grant |
| `forms.sessions.*` | Open/read/draft/patch/validate/save/submit/close ve offline queue |
| `records.workflow.*` / `attachments.*` | Review transition, immutable submission snapshot, staged upload/finalize |
| `ui.sessions.*` / `jobs.*` | Renderer session/capability, action/job progress/cancel/result |

- [x] **FORM-API-01:** Kaynak sözleşmeleri mevcut Rust contracts/authoritative katmanda tut; JSON Schema, TS ve Python tiplerini aynı kaynaktan üret. — *Yapıldı (16 Ekim; ADR 0401 §9).*
- [x] **FORM-API-02:** Her UI özelliği için semantic command/query ve argüman/default/validation/result/error/permission/undo ya da audit eşlemesi çıkar; coverage manifest olsun. — *Yapıldı (16 Ekim; ADR 0244'ün `catalog/v1/operations.json`'ı).*
- [x] **FORM-API-03:** UI, CLI/headless, Python ve MCP aynı handler/state machine/validator/access resolver'ı kullansın; alternate business rule oluşturma. — *Yapıldı (16 Ekim; ADR 0401 §9).*
- [x] **FORM-API-04:** `describe/compile/preview/validate/plan` production no-write olsun; mock preview, production save ve publication ayrı komut olsun. — *Yapıldı (16 Ekim; ADR 0401 §9 (`cad.forms.validate`)).*
- [ ] **FORM-API-05:** Mutation; actor context, immutable form/source revisions, expected record revision, idempotency ve typed patch taşısın.
- [ ] **FORM-API-06:** Sonuç; session/submission/job/resource IDs, validation paths, warnings, conflict, actual changeset/artifacts ve audit/undo sınırını bildirsin.
- [x] **FORM-API-07:** Python wrapper/stub/help/examples ve CLI/MCP schema drift CI'da hata olsun; üretilen dosyaları elle düzeltme. — *Yapıldı (16 Ekim; `scripts/python/sdk.py --check`, katalog testleri).*
- [ ] **FORM-API-08:** Form/schema/domain/relation keşfi pagination/projection/permission filtreli olsun; tüm özel şema ve veriyi her AI context'ine doldurma.
- [ ] **FORM-API-09:** AI form tasarımı draft/diff ve preview üretir; tasarım/yayın/data review/ACL değişimini doğru actor/grant ile ayrı çağırır.
- [ ] **FORM-API-10:** Form yardım metni, lookup label, schema açıklaması ve source verisi AI açısından veri olsun; içeriği endpoint/actor/grant/approval kapsamını değiştirmesin.
- [ ] **FORM-API-11:** Python/AI query, result, report, aggregate ve export effective field/row/task politikasına uysun; data masking renderer'a bırakılmasın.
- [x] **FORM-API-12:** Generic command veya doğrudan table/attribute API üzerinden gönderilen patch aynı authoritative kuralları uygulasın; form/version ID seçmek mandatory constraint'i kaldıramasın. — *Yapıldı (16 Ekim; ADR 0401 §8, ADR 0402 §4).*
- [ ] **FORM-API-13:** İsteğe bağlı plugin/form action için descriptor/release/executor/permission digest pinle; missing/offline executor sonucu açık pending/unavailable olsun.
- [ ] **FORM-API-14:** Semantic recorder izinli tasarım/veri girişi adımlarından yeniden çalıştırılabilir Python/command örneği üretebilsin; secrets ve yasak field değerlerini kaydetmesin.
- [x] **FORM-API-15:** Geliştirici örnekleri: core bina formu, domain/subtype, 1:N master–detail, authorized lookup ve offline queue; her örneğin capability kapsamı belli olsun. — *Yapıldı (17 Ekim; `python/examples/forms`: bina, alt tür ve bileşim, 1:N malikler, il-ilçe listesi; çevrimdışı kuyruk bulut işiyle açık).*
- [ ] **FORM-API-16:** SDK/command version negotiation ve deprecation/migration süreci dokümante edilsin; form v1 profili yeni extension runtime kurulunca sessizce değişmesin.

**Kabul:** Aynı tasarım ve veri girişi UI, Python ve MCP ile yürütülür; aynı kaynak/sürüm/izin altında aynı validation ve semantic result oluşur. Formu kullanan normal kullanıcıya Python/uv kurulumu gerekmez.

## 15. FORM-QA — İlk dilim ve tam sistem kabulü — P0/P1

### 15.1 Doğrulama işleri

- [ ] **FORM-QA-01:** JSON Schema/codegen/version/profile, duplicate ID, broken source ref ve unsupported required capability testleri kur.
- [x] **FORM-QA-02:** Mevcut beş field türünün adapter testleri; legacy/unknown attrs, rename/drop, absent/empty/null ve exact integer/decimal kaybını bağımsız beklentilerle doğrula. — *Yapıldı (16 Ekim; `fixtures/forms/v1`, `fixtures/document-ops/v1`).*
- [ ] **FORM-QA-03:** İlk core tanımı desktop/web/server'da aynı values/rules/validation fixtures ile çalışsın; mobile adapter hazır olduğunda aynı fixture korpusuna katılsın.
- [ ] **FORM-QA-04:** Gerçek input testleri tab/focus/dirty draft preservation, save/cancel, validation navigation, responsive ve a11y akışlarını kapsasın.
- [x] **FORM-QA-05:** Local persist/save/reopen, eski `.kcad`, undo/redo ve cloud revision conflict testleri yap; form taslağı veri mutation'ına karışmasın. — *Yapıldı (16 Ekim; ADR 0401 §3, `.kcad` örnekleri ve masaüstü testleri).*
- [ ] **FORM-QA-06:** Form design/publish/data edit/schema manage ayrımını role/grant matrix ile doğrula; UI gizlemek tek test kanıtı olmasın.
- [ ] **FORM-QA-07:** ACL testleri ham HTTP/generic command/Python/plugin/AI/table/import/lookup/export/event yollarını, cross-tenant ve forbidden row/field negatiflerini kapsasın.
- [ ] **FORM-QA-08:** Form/domain/schema release ve deployment digests için tampering, eski source binding, unpublished visibility ve accepted-version negatifleri çalışsın.
- [ ] **FORM-QA-09:** Domain/subtype/contingent ve rule fixtures bütün veri girişlerinde aynı kodlarla geçsin; locale yalnız display'i değiştirsin.
- [ ] **FORM-QA-10:** Parent-child/link writes, FK/cardinality, duplicate retry ve attachment staging/cleanup fault injection ile atomicity/idempotency sınansın.
- [ ] **FORM-QA-11:** Join/source fixtures duplicate key, revoked source, permission-filtered count ve stale async response durumlarını kapsasın.
- [ ] **FORM-QA-12:** Mobile/offline; revoked permission, expired task, changed schema/domain, deleted parent ve stale snapshot sync sonuçlarını doğrula.
- [ ] **FORM-QA-13:** Account switch, logout, browser cache recovery/eviction ve mobile termination; özel draft/lookup/attachment izolasyonunu sınasın.
- [ ] **FORM-QA-14:** Veri onayının exact submission revision'a bağlılığını, changed snapshot/reviewer grant ve approved record correction akışlarını doğrula.
- [x] **FORM-QA-15:** Büyük form, domain ve relation için açılış/rule recompute/paging/submission/memory ölç; gerçek referans cihaz ve veri korpusu üzerinden eşik belirle. — *Yapıldı (17 Ekim; ADR 0424: `forms_timing`, 100 000 kayıt, bütçeler; denetim 3 kat hızlandı). Gerçek saha cihazı ve web ölçümü açık.*
- [ ] **FORM-QA-16:** Generator çıktısını tek doğruluk kanıtı sayma; bağımsız veri/policy beklentileri ve gerçek merkez/saha kullanım kabulü oluştur.
- [ ] **FORM-QA-17:** AI eval; geçerli binding/rule/diff, permission projection, onay/yayın ayrımı ve untrusted help/source metnini ayrı ölçsün.
- [ ] **FORM-QA-18:** Her profilin bağımsız teslim kapısı olsun; ilk core/central kabulü ileri mobile/plugin özelliklerine bağlanmasın. Temiz Python/uv bulunmayan ortamda ilk form senaryosu mutlaka çalışsın.

### 15.2 İlk özellik kabul korpusu

| ID | Senaryo | Beklenen sonuç |
|---|---|---|
| I01 | Python/uv/plugin manager bulunmayan cihazda core form açılır | Desktop/web alan görüntüleme, saf kurallar ve yetkili edit çalışır; paket/interpreter indirme yok |
| I02 | Mevcut katmandan form oluşturulur; alanlar iki sekmeye taşınır | Stable field binding korunur; küçük ekranda aynı alan/kural anlamı |
| I03 | Form taslağı ve local release `.kcad` ile kaydedilip açılır | Layout/rule/value metadata korunur; mevcut/unknown attrs kaybolmaz |
| I04 | “Durum ek inceleme ise not zorunlu” kuralı eklenir | UI hata alanını gösterir; ham API de authoritative write profile ile geçersiz final kaydı reddeder |
| I05 | Tasarımcı v1'i belirli personele yayınlar; viewer açar | Personel sadece atanmış row/field'ı yazar; viewer yazamaz; form design/publish farklı grant |
| I06 | Personel form JSON/schema veya yasak alan/kaydı değiştirir | HTTP/table/Python/AI bütün yollarda ret; hiçbir kısmi mutation yok |
| I07 | İki kullanıcı aynı record revision'ını kaydeder; biri retry yapar | Conflict görünür, dirty draft korunur; idempotent retry duplicate write üretmez |
| I08 | Aynı form UI/komut/Python/MCP ile tasarlanır ve kaydedilir | Argüman/default/validation/policy/result eşdeğer; aktif belge/actor/version bağlamı doğru |

### 15.3 Tam hedefin kabul korpusu

| ID | Senaryo | Beklenen sonuç |
|---|---|---|
| F01 | Aynı FormBundle desktop/web/mobile renderer'a verilir | Aynı değer/domain/rule/relation anlamı; responsive görünüş |
| F02 | Domain typed code/label, subtype ve contingent seçim | İzinli kombinasyonlar; locale değişse de saklanan kod aynı |
| F03 | Conditional required alana UI/API geçersiz veri girilir | Ortak server validation code; final commit yok |
| F04 | Parent + iki child + attachment offline oluşturulur | Stable UUID/FK; sync retry duplicate üretmez |
| F05 | N:M link/unlink ve association/composition delete planı | İzin/cardinality/cascade sonuçları doğru; orphan row yok |
| F06 | Join duplicate key veya unauthorized source içerir | Ambiguous write reddedilir; private count/lookup sızmaz |
| F07 | Veri giriş personeli form JSON/domain/schema değiştirir | Server permission denial; authorized data entry etkilenmez |
| F08 | Read-only kullanıcı forbidden field/başkasının row'una ham patch yollar | HTTP/Python/AI/table bütün yollarında aynı ret |
| F09 | Designer v2 taslağı hazırlarken saha v1 kullanır | Published v1 değişmez; draft/version/conflict yönetimi doğru |
| F10 | Yetki/domain/schema offline sırasında değişir | Taslak korunur; sync current policy ile blocked/conflict döner |
| F11 | Server/Python-dependent mandatory action mobilde offline | Açık pending/unavailable; başarı veya approved sonucu uydurulmaz |
| F12 | Reviewer değişmiş record snapshot'ı onaylar | Version conflict; eski onay yeni veriye uygulanmaz |
| F13 | Account değişimi veya browser/mobile cache recovery | Eski hesabın özel draft/lookup/attachment verisi sızmaz |
| F14 | Lookup veya calculate cevabı geç gelir | Yeni değer/session/version ezilmez; pending state kapanır |
| A01 | AI form/domain/relation taslağı ve rule önerisi oluşturur | İncelenebilir JSON/diff; source refs valid; tasarım/yayın/veri onayı atlanmaz |
| A02 | Form help/lookup/source içinde araç talimatı vardır | Data olarak işlenir; actor/grant/endpoint/audience değişmez |
| A03 | Aynı form işi UI, command, Python ve MCP ile çalışır | Aynı source/scope altında eşdeğer semantic result, validation ve audit |

### 15.4 Tamamlandı kapıları

İlk ürün teslimi için I01–I08 ve `core/central` kapsamının aşağıdaki şartları zorunludur. F01–F14 ve mobil/offline/ileri relation kapıları ilgili sonraki fazın tamamlanma şartıdır; “form özelliği ilk eklensin” hedefini bütün ekosistemin bitmesine bağlamaz.

- [ ] Kaynak contract, schema/codegen ve kullanılan field adapter/migration tamamlandı.
- [ ] UI/command/Python/AI ve teslim edilen renderer'lar aynı semantik korpusu geçti.
- [ ] Form/data/field/row/task yetkisi bütün yetkili server girişlerinde ve commit'te uygulandı.
- [ ] No-write preview/validate, rollback/conflict/idempotency ve local persistence doğrulandı.
- [ ] Published snapshot/deployment/version binding ve source remapping test edildi.
- [ ] Teslim edilen platformlarda accessibility ve unsupported capability davranışı doğrulandı.
- [ ] İleri fazda relation/attachments/workflow atomicity ve offline current-policy sync geçti.
- [ ] Gerçek tasarımcı/veri giriş personeli/viewer rolleriyle örnek merkez/saha kabulü yapıldı.
- [ ] Kalan platform/profile/executor sınırları dokümante edildi; eksik yeteneğe yanlış hazır/tam durumu verilmedi.

## 16. Uygulama sırası, bağımlılıklar ve ilk iş listesi

P0 ilk çalışan ve doğru yetkilendirilmiş ürün temeli, P1 gelişmiş veri/form iş akışları, P2 ileri mobil/çevrimdışı/eklenti kapsamıdır. Bölümlerin genel P0/P1 etiketleri tam standardı gösterir; aşağıdaki teslim fazları ve `FORM-FIRST` listesi gerçek uygulama sırasını belirler. Takvim/ekip kapasitesi verilmediği için süre tahmini yapılmaz.

| Faz | Somut teslim | Bağımlılık / çıkış kapısı |
|---|---|---|
| FM0 — İlk core sözleşme | Mevcut fields/UUID/attrs/ACL envanteri, core profile, field mapping, value adapter ve semantic commands | Global typed storage göçü veya Python eklenti manager şart değil |
| FM1 — İlk çalışan yerel form | Temel designer + generated form + desktop/web renderer + saf kurallar/session + `.kcad` persist | I01–I04, I07–I08'in local tarafı; mevcut komut/SDK/AI temeli |
| FM2 — İlk merkezi kontrollü form | Server form kaynakları, immutable release/deployment, design/publish/use ve data/row/field/task guard | I01–I08 bütünü; önerilen ilk kurum/saha yayını |
| FM3 — Gelişmiş şema ve ilişkiler | Domain/subtype/contingent, typed/null genişlemesi, table records, 1:N/N:M, master–detail | Çalışan core/central ve source migration; F02–F03, F05–F08 |
| FM4 — Gelişmiş tasarım/iş akışı | Dış lookup/join, reuse/rule builder, actions, ekler ve exact-revision veri review | Yetkili source adapters, relation ve staged submission; F06, F09, F12, F14 |
| FM5 — Gerçek mobil ve offline | Mobil renderer/auth, task package/replica/outbox, media/GNSS, conflict/current-policy sync | Versioned sources/releases, ACL, relations ve submissions; F01, F04, F10–F13 |
| FM6 — İsteğe bağlı ekosistem ve kabul | Hazır/eklenti Processing action, custom widgets, `.kek` resources, performans ve AI eval | Yalnız bu action'lar için Python planının runtime/executor aşamaları; tüm ilgili F/A senaryoları |

FM1 ve FM2 ilk yeni özellik olarak önceliklendirilir. Mobil destek için ortak tanım FM0'da tasarlanır; gerçek mobil ürün FM5'te kabul edilir. Native hazır Processing action'ları yeterli command/capability varsa Python kurulmadan daha erken eklenebilir. FM6 yalnız opsiyonel eklenti entegrasyonunun yeridir.

Bağımlılık omurgası:

1. `Mevcut fields/UUID → kararlı binding/value adapter → core form/compiler/session → desktop/web designer/renderer`.
2. `Mevcut server access resolver → form/data grant ayrımı → projection/write guard → immutable release/deployment → kontrollü merkezi veri girişi`.
3. `Genişletilmiş schema/domain → table/relation/lookup → master–detail ve workflow`.
4. `Immutable kaynak/yayın + görev/satır/alan policy → mobile replica/outbox → current-policy sync`.
5. `Çalışan form/widget/action contracts → isteğe bağlı Python builder/Processing/custom-widget/.kek`.

Python/AI/komut eşdeğerliği her fazın tamamlanma koşuludur; sonunda açılacak ikinci bir uygulama yolu değildir. Form çekirdeği Python'dan çağrılabilir olmak için plugin runtime'a bağımlı hale getirilmemelidir.

### 16.1 İlk 12 uygulama işi

1. FM0 için mevcut attribute edit, fields, UUID, persist ve ProjectAccess akışlarını tekrar incele.
2. Core FormDefinition/Session/ValidationResult ve semantic command/result şemalarını kesinleştir.
3. Stable field ID backfill ve canonical attrs adapter'ını lossless değer fixture'larıyla kur.
4. Mevcut katmandan generated form ve snapshot'a form resources kaydını ekle.
5. KentOS bileşenleriyle temel sekme/grup/field designer ve desktop renderer'ı çıkar.
6. Aynı tanımı web renderer'da çalıştır; draft/focus/error navigation uyumunu geçir.
7. Saf rule profile, compiler/preflight ve ortak authoritative constraint binding'ini tamamla.
8. Record edit/save/cancel, revision conflict, idempotency ve uygun undo/audit akışını doğrula.
9. UI komutlarını mevcut SDK/codegen/headless/MCP'de aynı seçeneklerle erişilebilir kıl.
10. Cloud form draft/release/deployment kaynakları ve design/publish/use izinlerini ekle.
11. Dataset/row/field/task projection/write guard'ı tablo/import/ham API dahil tüm yollara bağla.
12. Tasarımcı → yetkili personel → viewer örneğini yayınla; I01–I08 geçtikten sonra FM3 ve devamındaki işleri aç.

## 17. FORM-DEC — Kararlar ve geliştiren ajana uygulama sınırları

- [x] **FORM-DEC-01:** FormDefinition/core profile/version/capability negotiation ve ilgili sözleşmeler için ADR yaz; v1'i tek platformun UI stack'ine bağlama. — *Yapıldı (16 Ekim; ADR 0401).*
- [x] **FORM-DEC-02:** İlk field ID backfill/storage mapping ve rename/drop politikası seçilsin; mevcut ID semantiği kaynakta doğrulansın. — *Yapıldı (16 Ekim; ADR 0401 §2).*
- [x] **FORM-DEC-03:** Canonical string attrs adapter'ının null/absent/empty/exact numeric sınırı ile sonraki typed storage migration kararı ayrı kaydedilsin. — *Yapıldı (16 Ekim; ADR 0401 §5).*
- [ ] **FORM-DEC-04:** Form draft/release/deployment/session sahipliği, `.kcad` transport, cloud authority ve local replica persist formatı seçilsin.
- [ ] **FORM-DEC-05:** Form design undo ile veri edit undo ve merkezi publication/audit/revert sınırları açık seçilsin; offline/cloud onayı local undo gibi gösterilmesin.
- [ ] **FORM-DEC-06:** Shared widget subset, responsive layout, custom widget profili ve renderer compatibility/fallback politikası yazılsın.
- [ ] **FORM-DEC-07:** Form/data/row/field/task/deployment policy intersection ve mevcut ProjectAccess/RLS göçü güvenli varsayılanlarla belirlensin.
- [ ] **FORM-DEC-08:** Local personal form ile kurum yayınının kaynak kimliği/sahipliği; import/remap/credential/grant politikası kesinleştirilsin.
- [ ] **FORM-DEC-09:** Form/schema/domain/rule version pin, accepted old submission ve breaking migration politikası seçilip test edilsin.
- [ ] **FORM-DEC-10:** Dataset/table/relation CRUD, transaction sınırı ve FK/cardinality enforcement otoritesi seçilsin.
- [ ] **FORM-DEC-11:** Cross-service source için read-only/explicit transaction/workflow-saga capability politikası yazılsın; otomatik dağıtık atomiklik varsayılmasın.
- [ ] **FORM-DEC-12:** Form publication/review ile record data approval rolleri ve kurumun gerekiyorsa reviewer separation politikası seçilsin; plugin release onayı ayrı kalsın.
- [ ] **FORM-DEC-13:** Mobil renderer/host stack ve shared rule engine, auth, local store, asset/sync adapter'ı seçilsin; portable standard stack bağımsız kalsın.
- [ ] **FORM-DEC-14:** Offline policy receipt/expiry/revocation, account cache izolasyonu ve conflict çözümünün gerçek sınırları belirlensin.
- [ ] **FORM-DEC-15:** Optional Python/Processing action için descriptor/executor/grant/runtime pin ve offline unavailable davranışı Python planıyla ortak sözleşmede tanımlansın.
- [ ] **FORM-DEC-16:** İlk core/central ürün kabulü ve desteklenen capability listesi sabitlensin; ileri şema/mobile/eklenti kapsamı ilk teslimin önkoşulu yapılmasın.

Uygulama sınırları:

- Mevcut Rust/TS domain, contracts, expression, KentOS UI, SDK/codegen ve server access resolver genişletilir; ayrı ve farklı kurallı bir form veritabanı/API kurulmaz.
- İlk çalışan dilim bütün typed/null/domain/relation göçünü beklemez. Bunun yerine mevcut kaynakla açık, testli adapter kullanılır; bilinmeyen eski değerler korunur.
- Form JSON'u yayın tanımıdır, yetki kaynağı değildir. Designer preview test verisini production'a yazamaz; role preview impersonation izni vermez.
- Form UI'sini saklamak devre dışı bırakılmış veri endpoint'i anlamına gelmez; authorization kaynak mutation/query katmanında uygulanır.
- Aynı field/domain/constraint forms, table, import, processing, command, Python ve AI'da aynı authoritative tanıma bağlıdır.
- Bir mobil JSON preview, gerçek mobil auth/offline/sync kabulü yerine geçmez; destek durumu gerçek test sonucu ile raporlanır.
- `.kek` form resources dağıtabilir; temel form yayınlama ve kullanım için extension runtime kurulması gerekmez. Plugin release review ile kurumun form yayını farklı kaynak lifecycle'larıdır.
- Başka geliştiricinin devam eden kaynak değişikliklerine bu belge çalışması adına müdahale edilmez. Uygulamaya başlarken kaynak sahipliği ve devam eden işlerle kesişen dilim netleştirilir.
- Generated katalog/schema/SDK dosyaları elle düzeltilmez; kaynak ve generator birlikte güncellenir.
- Her teslimde incelenen kaynak, gerçekten çalıştırılan test, geçilen profil, kalan yetenek ve başarısız/atlanan kabul açık raporlanır.

## 18. Resmi araştırma kaynakları

Bu kaynaklar 12 Ekim 2026 araştırmasında incelendi. QGIS kıyası 3.44 resmi dokümanını, ArcGIS kıyası ilgili Pro/Field Maps belgelerini esas alır. Uygulamaya geçerken sürüm/platform desteği tekrar doğrulanmalıdır. Bu belge KentOS'a ait önerilen standarttır; kaynak yazılımın kodunu veya UI dosyalarını kopyalama talebi değildir.

| Kaynak | Kullanıldığı konu |
|---|---|
| [QGIS 3.44 — Attributes Form Properties](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html#attributes-form-properties) | Tab/group/widgets, koşullar, defaults ve form tasarımı |
| [QGIS 3.44 — Joins and Relations](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/joins_relations.html) | Parent/child, N:M, relation reference/embedded form |
| [ArcGIS Field Maps — Build the form](https://doc.arcgis.com/en/field-maps/latest/prepare-maps/configure-the-form.htm) | Visible/required/editable/calculated expression ve saha formu |
| [ArcGIS Pro — Create and manage domains](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/create-modify-and-delete-domains.html) | Coded/range domain, owner, subtype ve split/merge policy |
| [ArcGIS Pro — Geodatabase terminology](https://pro.arcgis.com/en/pro-app/3.5/help/data/geodatabases/overview/geodatabase-terminology.htm) | Domains, subtypes, contingent values ve attribute rules |
| [ArcGIS Pro — Edit feature attributes](https://pro.arcgis.com/en/pro-app/3.6/help/editing/edit-feature-attributes.htm) | Related records ve contingent choices |
| [ArcGIS Pro — Data relationship options](https://doc.esri.com/en/arcgis-pro/latest/help/data/relationships/data-relationship-options.html) | Join/relate/relationship class ayrımı |
| [ArcGIS Pro — Prepare a relationship class](https://doc.esri.com/en/arcgis-pro/latest/help/data/relationships/prepare-to-create-a-relationship-class.html) | Key types ve cardinality |
| [ArcGIS Pro — Relationship class types](https://doc.esri.com/en/arcgis-pro/latest/help/data/relationships/geodatabase-relationship-class-types.html) | Association/composition deletion davranışı |
| [ArcGIS Pro — Attributed relationship classes](https://doc.esri.com/en/arcgis-pro/latest/help/data/relationships/understanding-attributed-relationship-classes.html) | N:M link table ve ilişki satırı attributes |

**Ürün kabulü:** İlk teslimde kullanıcı mevcut katmandan masaüstü/web formu tasarlayıp doldurur; yetkili merkez form sürümünü seçilen personele yayınlar ve sunucu onların yalnız izinli veriye yazmasını sağlar. Aynı standardın sonraki profilleri master–detail/domain/join, workflow ve mobil offline yeteneklerini ekler. Bütün teslim edilen UI işlemleri aynı komut/Python/AI yüzeyinden erişilebilirdir; temel form kullanıcıları uv/Python eklentisi kurmak zorunda kalmaz.
