# ADR 9135: İlişki üst katmanın kendi kodunu anahtar alabilir

- **Durum:** kabul edildi (2026-11-22); adım VY-34. Plan: [plan/veri.md](../../plan/veri.md) VY-34.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  22. maddesi (P1):
  - Veri › İlişkiler › Yeni › üst `mahalleler`, alt `bakim`, alan `mahalle` › Kaydet'te üst kaydın anahtar alanı
    seçilemedi;
  - “İlişkiden kur” görünümü üst anahtarı “Nesnenin kimliği” yaptı;
  - kurumun M1/M2 kodları bu tanıma karşılık gelmedi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.
- **`.kcad`:** CBS verisi sürüm 6, ilişkinin `parentKey`'i. Belge şeması değişmedi.
- **Komut:** `cad.relations.edit` `parentKey`'i alır ve üst katmanda olmayan alanı reddeder. `cad.records.page`,
  `cad.forms.submit` ve silme izleme anahtarla yürür. Python türü yeniden üretildi.

## §1 Bağlam

- İlişki (ADR 0405) alt alanın üst nesnenin kalıcı kimliğini tuttuğunu varsayıyordu. Bu kural birçok yerde
  yineleniyordu:
  - silmenin izlenmesi (`relation_follow`);
  - ağ izlemenin ilişkili kayıtları (`related_of`);
  - formun ilişkili listesi, en az/en çok sayımı ve `ilişkili_toplam`'ı (`kentos-forms`);
  - alt kayıt değişince üstün yeniden hesabı (`form_follow`);
  - `cad.records.page`, formdan bağlama (`forms_submit`), kayıt çoğaltma, kaynakta eşleme.
- Kurumların verisinde bağ çoğu zaman kendi kodlarıyladır: bakım kaydı mahallenin M1 kodunu tutar.
- **Araştırma.** Başvuru alınan CBS uygulamasında ilişki tanımı iki alan seçer: üst tablonun benzersiz anahtarı ve alt
  tablonun ona başvuran alanı. Bir üst kayıt silinince alt kayıtların ne olacağı ilişkinin kuralıdır.
  Kaynak: [birleşim ve ilişki kılavuzu](https://documentation.qgis.org/3.44/en/docs/user_manual/working_with_vector/joins_relations.html)
  (değerlendirmenin bağlantısı).

## §2 Karar

1. **Sözleşme.** `ProjectRelation.parent_key` (`parentKey`): alt alanın tuttuğu değerin üst katmanın hangi alanı
   olduğu. Yoksa eskisi gibi kalıcı kimliktir; var olan ilişkiler değişmez.
   - Boş olamaz, en çok 64 karakterdir.
   - `cad.relations.edit` üst katmanda olmayan anahtarı `relation_field_unknown` ile `edit.relation.parentKey`'de
     reddeder.
2. **Tek kural** (`kentos_domain::ParentKey`).
   - Bir üstün çocuklarına verdiği kod: anahtar alanının boşlukları atılmış değeri ya da kalıcı kimliği. Boş anahtarlı
     üstün çocuğu olmaz.
   - Bütün yollar bunu sorar: silmenin izlenmesi (kopar, sil, kısıtla), ilişkili kayıtlar, formun ilişkili listesi ve
     sayımı (`WorldRelation.by`), `ilişkili_toplam`, alt kayıt değişince üstün yeniden hesabı, `cad.records.page`.
   - Formdan bağlama: bağlarken kaydın o anki kodunu, koparırken alt kaydın tuttuğu eski kodu yazar. Anahtarı boş kayda
     alt kayıt bağlanmaz, nedeni söylenir.
3. **Benzersiz olmayan anahtar.** Silinen üstün kodunu başka bir üst hâlâ tutuyorsa çocukları yerinde kalır. Kısıtla
   kuralı da o üstü silmeye izin verir.
   - Kayıt çoğaltma anahtarla bağlı bileşik ilişkinin çocuklarını çoğaltmaz: kopya aynı kodu taşır, yeni kodu kişi
     verir.
   - Kaynakta eşleme (GeoPackage ilişkisi, bağlantı tablosu) kalıcı kimliklerledir; anahtarlı ilişki eşlenmez.
4. **İlişkiler penceresi.**
   - Üst katmandan sonra “Üst anahtar” seçimi gelir: “Nesnenin kimliği” ya da üst katmanın bir alanı. Üst katman
     değişince anahtar sıfırlanır.
   - Kaydet'ten önce altında denetimler görünür (`relation_checks.rs`):
     - anahtarın benzersizliği (aynı kodu tutan kayıtlar adlarıyla) ve boş anahtarlı üst kayıtlar;
     - iki alanın türü ayrıysa bu;
     - alt kayıtların kaçının bir üste bağlı olduğu;
     - hiçbir üste karşılık gelmeyen kod uyarı olarak adıyla (M9);
     - alanı boş (NULL) alt kayıtlar ayrı bir bilgi satırında.
   - Veri katmanının kayıtları kaynağından okunur, okunurken söylenir. Denetim kaydı durdurmaz.
5. **İlişkiden kur.** Birleşik görünüm üst anahtarı ilişkinin anahtarıyla kurar, yoksa kalıcı kimlikle.
6. **Dosya.**
   - CBS verisi sürüm 6: bir ilişkide `parentKey` varsa yazıcı `gis: 6` yazar.
   - Sürüm 5'in yükünde alan bilinmeyendir, boş anahtar bozuktur.
   - Rust ve Python okuyucuları aynı kuralla okur. Üç yeni örnek dosya var; 1883 dosyanın hepsi uyuşuyor.
   - GeoPackage profili ilişkinin belgesinde anahtarı taşır.

## §3 Kanıt

- **Çekirdek** (`kentos-domain` `relation_keys`): mahalle ve bakım kayıtlarıyla.
  - Formun `ilişkili_toplam`'ı M1'e 2, M2'ye 2, M3'e 0 sayıyor.
  - İlişkili kayıtlar koda göre; M9 ve boş olan hiçbir mahallenin değil.
  - Silme izleniyor: bileşik ilişkide iki kayıt gidiyor, kopar kuralında kodlar boşalıyor, kısıtla M1'i tutuyor, M3'ü
    tutmuyor.
  - Aynı kodlu ikinci mahalle çocukları koruyor.
- **Form motoru** (`kentos-forms`): ilişkili öğe koda göre listeliyor; boş kodun çocuğu yok; anahtarsız ilişkide kod
  sorulmuyor.
- **Uygulama** (`tests/all/relation_keys.rs`): olmayan anahtar yerinde reddediliyor; `cad.records.page` M1'in iki
  bakımını ve B03'ün mahallesini koda göre veriyor.
- **Ortak durumlar:** `cad.relations.edit` 31 durum (3 yeni: anahtarla oluşturma, olmayan anahtar, boş anahtar).
  Durumlar `scripts/fixtures/forms_cases.py`'ye yazıldı. Bu makinede `mpmath` olmadığı için üretici çalıştırılamadı;
  dosyaya üreticinin biçimiyle eklendi, Rust'ta geçiyor.
- **Dosya:** `kentos-kcad` 288 sınama ve Python okuyucusunun `--check`'i geçiyor.
- **Masaüstü** (`gis/forms/relation_keys_tests.rs`), raporun iki veri katmanıyla:
  - pencere benzersiz, 6 kayıttan 4'ü bağlı, “M9” uyarısı ve bir NULL diyor;
  - Kaydet anahtarı yazıyor; dosyaya yazılıp okununca ilişki aynı;
  - İlişkiden kur anahtarı `mahalle` olan görünüm yapıyor;
  - üst katman değişince anahtar sıfırlanıyor.
- Formlar, öznitelik tablosu, işlemler, veri katmanı, ağlar, izler, şerit, modül, katalog ve ayar sınamaları (376)
  geçiyor. Bir ağır iz sınaması paralel koşuda düştü, tek başına geçti.
- Resimler: `.run/gui-evidence/kullanim-9135/` (Git'e girmez).

## §4 Kalan

- **Veri katmanında formun ilişkili listesi.** Veri katmanı üzerindeki formda ilişkili listenin satırları hâlâ
  çizimdeki nesnelerden gelir. M1 formunda iki bakımı kaynaktan okuyup göstermek, formun dünyasına veri katmanının
  satırlarını katmak ister. Pencerenin denetimleri veri katmanında çalışıyor.
- Silme davranışının izole veride bir kez de gerçek pencerede denenmesi.
- `forms_cases.py`'nin `mpmath` olan bir makinede `--check` ile koşturulması.
- Web (TODOS.md §0.2).
