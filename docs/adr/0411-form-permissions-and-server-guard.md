# ADR 0411: Formların izinleri, Veri girişi rolü ve sunucunun yazma bekçisi

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-F`; `plan/form.md` FORM-FIRST-14–17.
- **Bağlam belgesi:** ADR 0015 (proje erişimi), 0035 (davet ve misafir), 0401 (form, yayın, Kuralları uygula), 0402
  (kurallar ve hesaplar), 0404 (başka katmandan liste), 0410 (geometrisiz kayıt).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Formlar projenin ayarıdır (ADR 0401): bulutta veritabanı projesinin ayarlarıyla gider ve gelir. Ama sunucu onları tanımıyordu:

- proje ayarını değiştirebilen herkes (`project.edit`) formu da tasarlıyor ve yayınlıyordu; tasarım ile yayın ayrı yetki değildi;
- nesne yazabilen herkes (`feature.write`) formun alanlarını formdan geçmeden yazabiliyordu: Kuralları uygula yalnız istemcinin
  kendi belgesinde zorlanıyordu; Python, MCP ya da başka bir istemci `project.changes` ile kuralı aşabiliyordu;
- saha ekibine “yalnız formla kayıt düzenle, çizim değiştirme” denecek bir rol yoktu.

QGIS'te formun düzeni projededir; yetkiyi veri kaynağı (PostgreSQL rolleri, QGIS Server'ın erişim denetimi) verir; alan
kısıtları (not null, unique, ifade kısıtı) kaynakta ya da istemcide zorlanır. ArcGIS'te özellik hizmetinin “yalnız öznitelik
güncelle” yeteneği ve Field Maps'in form şablonu ayrı ayrı yetkilendirilir.

## Karar

### 1. İzinler

`ProjectPermission`'a üç izin:

| İzin | Anlamı |
|---|---|
| `forms.design` | formları ve ilişkileri oluşturmak, taslağı değiştirmek, silmek, Kuralları uygula'yı açıp kapamak |
| `forms.publish` | formun yeni sürümünü yayınlamak (`releases`) |
| `forms.use` | yayımlanmış formla kayıt düzenlemek |

### 2. Veri girişi rolü (`entry`)

Rollerin sırası Görüntüleyici < Yorumcu < **Veri girişi** < Düzenleyici < Yönetici < Sahip:

| Rol | İzinler |
|---|---|
| Görüntüleyici | okuma, indirme, geçmiş |
| Yorumcu | + yorum |
| Veri girişi | + `forms.use` |
| Düzenleyici | + `feature.write`, işler, `forms.use` |
| Yönetici, Sahip | + `project.edit`, paylaşma, `forms.design`, `forms.publish` (Sahip ayrıca silme, devretme) |

Veri girişi `feature.write` almaz: nesne oluşturmaz, silmez, şekil değiştirmez. `project.changes`'te yalnız var olan bir
nesnenin özniteliklerini değiştirir; nesnenin katmanında yayımlanmış bir form olmalı ve değişiklik o formun yazma
kuralından geçmeli (`Prepared::assess`: formun göstermediği, gizlediği ya da kilitlediği alan, hesaplanan alana başka değer,
alanın kuralını bozan değer ve formun bütün denetimleri). Öznitelik dışındaki bir alan (geometri, katman, renk, etiket, ekler)
değişirse yazma reddedilir. Misafir de Veri girişi olabilir (davetin rolleri: Görüntüleyici, Yorumcu, Veri girişi,
Düzenleyici). Göç 0021 rolün adını veritabanının denetimlerine ve davetin rol sırasına ekler.

### 3. Ayarların bekçisi

`project.changes`'in ayar yaması kayıtlı ayarla karşılaştırılır:

- formların sürümleri (`releases`) değiştiyse `forms.publish`;
- formların başka bir şeyi (taslak, ad, katman, Kuralları uygula, eklenen ya da silinen form) ya da ilişkiler değiştiyse
  `forms.design`;
- yeni formlar dosya biçiminin kuralını korur (`forms_problem`, `relations_problem`): bozuk form yazılmaz;
- yayımlanmış sürüm değişmez: numarasıyla kalan sürüm aynen kalır, yeni sürüm sonuncudan sonra gelir, yalnız en eskiler
  düşebilir (form en çok `MOST_FORM_RELEASES` sürüm tutar), son sürüm silinmez (`releases_problem`).

`project.edit` ayrıca gerekir (ayar yaması zaten onu ister); böylece formlara yalnız ikisini birden tutan dokunur.

### 4. Yazma bekçisi bütün yollarda

Her nesne yazmasında (oluşturma ve güncelleme), nesnenin katmanında **Kuralları uygula** açık ve yayımlanmış bir form varsa
sunucu, istemcinin belgesinin yaptığını yapar (`Prepared::enforce`): değişen alan formda görünmeli ve açık olmalı, hesaplanan
alan hesabının değerini almalı, kayıt formun bütün kurallarını tutmalı. Hesap ortak çekirdekten (`kentos-forms`), dünya (başka
katmandan listeler, ilişkili kayıtlar) projenin kayıtlı nesnelerinden, değişkenler projenin ayarından masaüstündeki kuralla
(`kentos_project::variables`; bağımlılık yönünde sunucu, saf proje modelini kullanabilir: `scripts/arch/deps.mjs`) gelir; `@tarih`,
`@simdi` istemcideki gibi denetimde yoktur. Kural bozulursa bütün değişiklik reddedilir (`form_rules`, alanın yolu
`features[i].entity`). Masaüstü, Python, MCP ve web aynı bekçiden geçer.

### 5. Masaüstünde Veri girişi

Veri girişi rolüyle açılan veritabanı projesinde belgenin nesne kuralı `AttributesOnly`'dir (`kentos_domain::ObjectPolicy`):
her komut adımının bütünlük denetimi (`faults`, ADR 0253 §8) yeni nesneyi, silmeyi ve öznitelik dışındaki her değişikliği
nedeniyle reddeder, adım bütünüyle geri alınır. Formla düzenle, Öznitelikler ve Öznitelik tablosu öznitelikleri yazar.
Eşitleme `forms.use` sahibine de nesne değişikliklerini gönderir (`OpenedProject::can_send_objects`); dosya projesinin
revizyonu yalnız yazanın. Rol açıkken değişince kural da değişir.

## Denetimler

- Bekçinin kuralları saf bir modülde (`form_guard.rs`) birim testleriyle: ayarların izinleri, Veri girişinin yazabildikleri
  ve yazamadıkları, Kuralları uygula'nın reddi.
- Veritabanı testi (`tests/project_forms.rs`, geçici veritabanında): tasarım ve yayın izinleri, Veri girişi ile öznitelik
  güncellemesi ve şekil değişikliğinin reddi, düzenleyicinin kuralı bozan yazmasının reddi.
- Sunucunun bütün veritabanı testleri (133) kendi geçici PostGIS kapsayıcısında (127.0.0.1:55432; geliştiricinin 5432'si
  değil) göç 0020 ve 0021 dahil geçti.
- `kentos-native-application`'ın `object_policy` testi: kural açıkken nokta oluşturma, taşıma ve silme reddedilir, öznitelik
  yazılır.
- Masaüstü: paylaşma penceresinde Veri girişi rolü (`fixtures/cloud/v1/share.json`, resim `bulut-paylas-kisiler-*`);
  Veri girişi rolündeki kullanıcıya proje açılınca ne yapabileceği söylenir.

## Sonuçlar

- Formun tasarımı, yayını ve kullanımı ayrı yetkilerdir; saha ekibi çizimi değiştirmeden formla veri girer.
- Kuralları uygula bir istemci tercihi olmaktan çıkar, sunucunun kuralı olur.
- Açık: alan düzeyinde okuma yetkisi (formun göstermediği alanın Veri girişine gönderilmemesi, TODOS.md `FORM-R`), form başına
  kullanıcı ataması, onay akışı (FORM-ACL, FORM-WORK).
