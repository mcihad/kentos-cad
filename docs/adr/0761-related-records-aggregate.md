# ADR 0761: Formda ilişkili kayıtların toplamı

- **Durum:** kabul edildi (31 Ekim 2026).
- **İş:** plan/form.md `FORM-RULE-09`'un kalanı: “ilişkili toplamı okuyan hesap gelince bağımlılık grafı”.
- **Dal:** `formlar-surer` (sahibin isteği: “Form ile ilgili işlere devam et ama ayrı bir dal açarak devam et”).
- **Web:** web dururken (ADR 0228) yalnız masaüstü ve bulut; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez (§5).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Formun hesabı kaydın kendi alanlarını, ölçülerini ve `@` değerlerini okuyordu (ADR 0402, 0421). Bir binanın malik
sayısı, bağımsız bölümlerinin toplam alanı ya da hisselerin toplamı gibi değerler ise kaydın *alt kayıtlarından*
gelir. İlişkili kayıtlar öğesi (ADR 0404, 0425) onları listeleyip tablo altında toplayabiliyordu, ama bu toplam ne
alana yazılıyor ne bir kurala giriyordu. Alt kayıt eklenip silinince üst kaydın hesabı da kendiliğinden yenilenmiyordu.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **QGIS, `relation_aggregate`:** “Returns an aggregate value calculated using all matching child features from a layer
  relation.” Söz dizimi `relation_aggregate(relation, aggregate, expression, [concatenator:=''], [order_by])`, ilişki
  “a string, representing a relation ID”, ve “Fields and geometry are from the features on the joined layer.”
  <https://docs.qgis.org/latest/en/docs/user_manual/expressions/functions_list.html>
- **ArcGIS Arcade, `FeatureSetByRelationshipName`:** “Returns the related records for a given feature as a FeatureSet.”
  İmza `FeatureSetByRelationshipName(inputFeature, relationshipName, fieldNames?, includeGeometry?) -> FeatureSet`;
  sayma ve toplama bu kümenin üstünde `Count`, `Sum` ile yapılır.
  <https://developers.arcgis.com/arcade/function-reference/featureset_functions/>

İkisi de ilişkiyi kendi adıyla ya da kimliğiyle anar, iç ifadeyi alt kaydın alanlarıyla hesaplar. KentOS da bunu
QGIS'in tek işlevli biçimiyle yapar. Bağımlılık izleme işi ise belgenin adımına düşer.

## Karar

### 1. İfade dilinde `ilişkili_toplam`

```text
ilişkili_toplam(ilişki, işlem, ifade, koşul)        İngilizce adı relation_aggregate
ilişkili_toplam('Malikler', 'say')                  → binanın malik sayısı
ilişkili_toplam('Malikler', 'topla', Hisse, Durum = 'Etkin')
```

- **`ilişki`:** projedeki ilişkinin (ADR 0405) kimliği ya da adı, sabit metin. Önce kimlik, sonra ad aranır. Türkçe
  harfler ve büyük küçük harf ayrılmaz.
- **`işlem`:** `katman_toplamı`'nın toplamalarından biri (ADR 0214 §2.6, 0602), sabit metin. Sayılar
  `kentos.statistics/1` kuralıyla hesaplanır: kesin toplam, sayıların ortalaması, sayılar yoksa metinlerin en küçüğü.
- **`ifade` ve `koşul`:** her alt kayıtta, alt kaydın alanlarıyla hesaplanır (QGIS'teki gibi). İfadesiz yalnız `say`
  olur ve alt kayıtları sayar.
- **Alt kayıt:** ilişkinin alt katmanında, ilişki alanı kaydın kalıcı kimliğini tutan kayıttır. Çoktan çoğa alan
  `;` ile birden çok kimlik tutar (ADR 0409 §1); her kimlik için sayılır.
- **Hesaplanmayan durumlar:** yeni kayıt kimliksizdir, alt kaydı olmayan kayıt gibi sayılır (`say` 0, `topla` boş).
- **Derleme kuralları:**
  - İlişki ve işlem sabit metin olmalıdır.
  - İşlem bilinen bir toplama olmalıdır.
  - İç ifadede ikinci bir `ilişkili_toplam` olamaz.
  - Kuralı bozan ifade derleme hatasıdır.
- **Nerede derlenir:** yalnız formun kurallarında. İfade şemasının yeni `relations` bayrağı bunu söyler; formun şeması
  `kentos_forms::form_schema_in`'dir. Başka her yerde reddedilir: İşlemler'de, etiketlerde, katman süzgecinde,
  stilde, katmanın kayıt kurallarında. Ret iletisi “ilişkili_toplam yalnız formun hesaplarında ve kurallarında
  kullanılır” (`RELATIONS_REFUSED`). İfade oluşturucu işlevi yalnız form kuralında önerir.
- **İç ifadenin adları:** dış ifadenin alanları sayılmaz. Bu yüzden ön denetim “katmanda olmayan alan” uyarısı
  vermez, hesap sırası da onları beklemez.

### 2. Formun çekirdeği

- **Hazırlama:**
  - Form hazırlanırken (`Prepared::new_with`) kurallarının andığı ilişkiler bulunur (`named_relations`).
  - Dünyadaki alt katmanların kayıtları da bulunur (`OtherLayer.relations`, `WorldRelation`).
  - Kayıtlar bir kez, üst kaydın kimliğine göre dizinlenir (`relatives::Relatives`).
  - Bir çağrı yalnız kaydına bağlı alt kayıtları okur.
- **Hesapta kullanım:**
  - Dizin kurallara nesneyle gelir (`Object.related`).
  - Formun `states`, `complete`, `validate_left`, `assess_left`, `enforce` ve benzeri yöntemleri nesneyi bununla
    görür.
  - `RuleScope` çağrıyı ifade dilinin `WorldCalls` yoluyla cevaplar
    (`kentos_expression::world::related::related_value`, `Expr::item`).
- **Kullanıldığı yerler:** hesap, varsayılan ifadesi, görünme, zorunluluk ve salt okunurluk koşulları, denetimler
  ve seçenek süzgeci aynı yoldan okur. Örnek kurallar:
  - “hisseler toplamı 1 olmalı”;
  - “iki bölümlü binada not zorunlu”.
- **Bilinmeyen ilişki:** projede olmayan ilişki ön denetimde uyarıdır (`form_relation_unknown`); değeri boştur.
  Uyarı olmasının nedeni, dünyasız hazırlanan formun da açılmasıdır.
- **Geometri:** alt kaydın ölçüleri (`$alan`, `$uzunluk`) yalnız bir çağrının iç ifadesi onları okuyorsa verilir
  (`related_needs`). Okumayan formda alt katmanın şekilleri çevrilmez.

### 3. Belge aynı adımda izler

Belge, formu uygulanan katmanların yazmalarını zaten aynı adımda tamamlıyordu (ADR 0402 §4). Buna şu eklendi:

- **Turlar** (`follow_forms`):
  - Bir adım bir ilişkinin alt kaydını ekler, değiştirir, bağlar, bağını koparır ya da silerse, alt kaydın ilişki
    alanının *önce ve sonra* andığı üst kayıtlar yeniden tamamlanır.
  - Bu tamamlamalar başka bir ilişkinin alt kayıtlarını değiştirdiyse sonraki turda onların üst kayıtları
    tamamlanır. Örnek: binanın malik sayısını okuyan adanın toplamı.
- **Tur sınırı:** en çok 8 tur yapılır (`MOST_RELATED_ROUNDS`); ilişkilerin halka kurduğu projede orada durulur.
- **Tek adım:** hepsi aynı geri alma adımındadır. Geri al üst kayıtların hesabını da geri getirir.
- **Hangi formlar kurulur:**
  - Her tur yalnız tamamlanacak kayıtların katmanlarının formlarını kurar.
  - Dünya, son tamamlamaların bıraktığı hâldir: üst kayıt alt kayıtların kendi hesaplarını okur.
  - Formların okumadığı katmana dokunan adım hiçbir şey kurmaz (`related_reads`).
  - Bir yan kazanç: yeni kaydın tamamlanması da artık yalnız o kaydın katmanının formlarını kurar; önceden bütün
    uygulanan formlar kuruluyordu.
- **Formla düzenle:** başka bir nesne değişince pencere formu yeniden kurar (`refresh_entry`). Kaydın taban
  değerleri de belgenin yeni hâlini alır. Böylece belgenin yazdığı hesap kullanıcının değişikliği sanılmaz ve
  başlıkta kaydedilmemiş noktası çıkmaz. Yazılan değerler yine üstte kalır. Toplu kipte taban ortak değerler
  olduğundan değişmez.

### 4. Sunucunun bekçisi

Sunucu formu uygulanan katmana yazmayı formun hesabıyla denetler (ADR 0411 §4). Hesaplanan alanın yazılan değeri
formun hesabından gelmelidir. Bunun için:

- **Dünya:** bekçinin dünyası, formların andığı ilişkilerin alt katmanlarını da okur (`relation_layers`).
- **Bindirme:** dünyaya değişiklik kümesinin kendi yazmaları bindirilir (`overlay_children`).
  - Oluşturulan ya da güncellenen alt kayıt yerini alır.
  - Silinen alt kayıt gider.
  - Katmanı değişen alt kayıt da gider.
- **Neden:** masaüstü alt kaydı ve üst kaydın yeni sayısını aynı kümede gönderir. Sunucu üstün hesabını yeni alt
  kayıtlarla yapar ve aynı sayıyı bulur. Başka bir sayı reddedilir.
- **Sınır:** sunucu üst kaydı kendiliğinden yeniden hesaplamaz. Bu, eşitleyen istemcinin işidir: masaüstünün belgesi
  bunu aynı adımda yapar.

### 5. Dosya ve şema

- **Şema numarası:** formun gövdesi `.kcad`'te bütün olarak okunur (şema 72, ADR 0401). İşlev ifadenin metnindedir;
  şema değişmez.
- **Bedeli:** bu işlevi kullanan formu daha eski bir KentOS'ta ön denetim “kural derlenmez” diye reddeder. Yayımlanmış
  form orada uygulanmaz.
- **Dışarıdaki tanımlar:** formun JSON Schema'sı, komut kataloğu ve Python SDK'sı değişmez. Yeni şey yalnız bir ön
  denetim kodudur (`form_relation_unknown`).

## Doğrulama

- **Ortak durumlar:** `fixtures/forms/v1/cases.json`'un `relatives` grubunda 29 durum.
  - Kaynak: `scripts/fixtures/forms_cases.py`'nin Python ikizi (`resolve_related`, `related_value`), KentOS kodu
    olmadan. Toplamalar `spatial_query_cases.py`'nin ondalık okuyucusuyla, kesirlerle hesaplanır.
  - Toplamalar: say, koşullu say, ifadeli say, topla, koşullu topla, ifadenin hesabıyla topla, ortalama, en yüksek,
    en düşük, değerleri birleştir, say benzersiz.
  - İlişkinin anılışı: başka ilişki, kimlikle, harf farkıyla, İngilizce adla.
  - Bağlar: çoktan çoğa bağ, boş bağ, alt kaydı olmayan kayıt, yeni kayıt.
  - Bilinmeyen ilişki ve dünyasız form.
  - Kurallarda: denetim ve zorunluluk.
  - Derlenmeyenler: bilinmeyen toplama, ifadesiz `topla`, sabit olmayan ilişki, iç içe çağrı.
  - Rust tarafı: `related_records_aggregate_as_the_cases_say`.
- **İfade dili:** `a_related_aggregate_compiles_in_a_form_only`: yalnız formda derlenir, retlerin yeri ve iletisi
  doğrudur, `katman_toplamı` formda yine reddedilir. İşlevin örnekleri yardımdakiyle aynı sonucu verir.
- **Belge** (`kentos-domain`'in `related_sums` testleri):
  - Malik eklenince binanın sayısı ve hisse toplamı, sonraki turda adanın toplamı aynı adımda değişir.
  - Tek geri al hepsini geri getirir; yinele yeniden yapar.
  - Hisse değişince toplam izler.
  - Bağ koparılınca ve alt kayıt silinince sayı düşer.
  - Başka binaya taşınan malik iki binayı da değiştirir; çoktan çoğa alan ikisine de sayılır.
- **Sunucu:**
  - `a_count_of_related_records_is_judged_over_the_relation`: ilişkiden gelen doğru sayı geçer, yanlışı reddedilir,
    satırsız alt katman da ilişkiyi taşır.
  - `the_commit_s_children_lie_over_the_stored_rows`: değişiklik kümesi dünyaya bindirilir, başka katmanın satırı
    kalır.
- **Resimler** (`tools_screens`):
  - `form-iliskili-toplam`: bina formu bağlı 4 bölümün sayısını, toplam alanını, maliklerini ve üst kattakileri
    hesaplar. Komut geçmişinde bağlama tek adımda “5 nesne değişti” der: 4 bölüm ve bina.
  - `form-iliskili-eklendi`: pencere açıkken 5. bölüm bağlanınca değerler 5, 453.75 ve 2 olur, başlıkta kaydedilmemiş
    noktası yoktur. Resimlerde bulunan bu kusur §3'teki taban düzeltmesiyle giderildi.
  - `form-iliskili-tasarim`: tasarımcıda hesabın ifadesi, “Ön denetim: sorun yok”.
- **Ölçüm** (release, `related_sums::timing`): 1 000 bina, 10 000 malik.
  - Bir malik eklemek, binayı ve adayı aynı adımda yeniden hesaplamakla birlikte, ortancada 13,1 ms (en çok
    13,9 ms). İlk uygulama her turda bütün formları kuruyordu ve 40,3 ms sürüyordu.
  - Formların okumadığı bir kaydı eklemek 1,8 ms. Önceden 13,3 ms sürüyordu, çünkü bütün uygulanan formlar kuruluyordu.
  - 10 000 maliki birden eklemek 124 ms.
  - Kalan pay, alt katmanın kayıtlarının her adımda dünyaya kopyalanmasıdır. Daha büyük katmanlarda belgenin kalıcı
    bir alt kayıt dizini sonraki iştir.
