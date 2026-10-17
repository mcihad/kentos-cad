# ADR 0402: Formlar: hesaplanan değerler, süzülen seçenekler, uyarılar ve kaydın oluşumu

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): “Formları en iyi hâle getirene kadar, hatta QGIS'in form
  yönetimini de iyice araştır, ondan daha iyi hâle gelene kadar devam et. Bu piyasadaki en iyi coğrafi bilgi sistemleri
  yazılımı olacak.”
- **Bağlam belgesi:** ADR 0401 (form çekirdeği); `plan/form.md` FORM-FIRST-10, FORM-RULE (§8), FORM-DATA-06, -20; TODOS.md
  §16.6 `FORM-E`.
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** ilişkiler ve ana–ayrıntı, başka katmandan seçenek listesi (ADR 0403'e), merkezi yayın ve izinler
  (`FORM-F`), birden çok değer seçilen alan (katman alanının biçimi değişmeli), barkod ve ek öğesi.

## Bağlam: QGIS ve ArcGIS Field Maps ne yapıyor

Karşılaştırma için QGIS'in “Attributes Form” belgesi (3.40, sürükle bırak tasarımcısı, düzenleme öğeleri, alan
seçenekleri) ve ArcGIS Field Maps Designer'ın form belgesi okundu. QGIS'in hata kayıtlarından ikisi de doğrudan konumuz:

- **QGIS:**
  - Üç yerleşim: kendiliğinden, sürükle bırak, Qt Designer `.ui`.
  - Kaplar: sekme, grup kutusu, satır. Sütun sayısı, görünürlük ifadesi, daraltılmış başlama (ifadeyle de), renk.
  - Ek öğeler: HTML, QML, metin (ifadeli), boşluk, ilişki, eylem.
  - Alan: takma ad, düzenlenebilir (veriye bağlı), son değeri yeniden kullan, etiketi üstte.
  - Kısıtlar: boş olmaz, tekil, ifade. Zorlanan ya da yumuşak (yumuşak sarı uyarıdır, kaydı durdurmaz).
  - Varsayılan değer ifadesi ve “güncellemede de uygula”.
  - Bölme, kopyalama ve birleştirme politikaları.
  - Düzenleme öğeleri: Değer haritası, Değer ilişkisi (başka katmandan, `current_value()` ile süzme, çoklu seçim),
    İlişki başvurusu, Benzersiz değerler, UUID, tarih, aralık, renk, ek.
  - Çoklu düzenleme kipi.
- **QGIS'in bilinen zayıflıkları:**
  - Formun açılışında `current_value()` kuralı ilk değeri görmez; bağlı liste, kullanıcı üst alanı yeniden yazana dek
    süzülmez (QGIS #21852, GitHub #29667).
  - Süzülen listede yanlış değer kaydedildi (#22062).
  - Kurallar yalnız formdadır. Öznitelik tablosu, alan hesaplayıcı ve Python formu atlar; “zorlanan kısıt” sağlayıcıya
    göre değişir.
  - Varsayılan ifadesi yeni kaydın öbür alanlarını okuyamaz.
- **Field Maps:**
  - Grup (açılış durumu, koşullu görünürlük) ve bilgi öğesi (alanlardan ve ifadelerden değerlerle, markdown).
  - Arcade ile görünürlük, zorunluluk ve düzenlenebilirlik ifadeleri, hesaplanan ifadeler. Hesaplanan alan
    düzenlenemez, form değiştikçe yeniden hesaplanır.
  - Koşullu değerler (contingent values). Gizli alanın değerini koruma seçeneği.
  - Çoklu seçim, barkod, ek öğeleri.

ADR 0401'den sonra bizde eksik olanlar, bu ADR'nin konusu:

- hesaplanan alan;
- ifadeli varsayılan;
- süzülen seçenekler;
- yumuşak denetim;
- tekil alan;
- ifadeli not;
- daraltılmış grup;
- gizlenen alanın değerinin silinmesi;
- kurallarda `$uzunluk` ve `$alan`;
- anın ve kullanıcının değerleri;
- yeni kaydın oluşumu: varsayılanların ve hesapların her yazma yolunda uygulanması;
- son değerin yeniden kullanılması.

## Karar

### 1. Sözleşmenin ekleri (`kentos.forms/1 core`, `.kcad` şema 72'nin içinde)

Şema 60 bu dalda henüz `main`'e girmedi; ekler aynı şemadadır. Bütün ekler isteğe bağlıdır. Yalnız `true` ya da dolu
yazılır, eski form aynı baytlarla okunur ve yazılır.

- **`FormField`:**
  - `defaultExpr`: yeni kaydın boş alanına konan değerin ifadesi. Yeni kaydın öbür alanlarını da okur: önce düz
    varsayılanlar, sonra ifadeler formun sırasıyla. `default` ile birlikte verilmez.
  - `calculate`: alanın değerini her yazmada hesaplayan ifade. Alan salt okunurdur. `default`, `defaultExpr` ve
    `readOnlyWhen` ile birlikte verilmez.
  - `choiceFilter`: değer listesinin her seçeneği için sorulan ifade. Seçeneğin kodu `@secenek`, etiketi
    `@secenek_etiketi`; kaydın alanları adlarıyla okunur. Doğru dönmeyen seçenek sunulmaz. Kayıttaki değer süzülmüşse
    `form_choice_filtered` hatasıdır.
  - `unique`: alanın değeri katmanda tekildir (boş değerler sayılmaz).
  - `reuseLast`: yeni kayıtta alan, bu oturumda formla yazılan son değerle açılır. Formun arayüzünün işidir, komut onu
    bilmez (CMD-07: Python değeri açıkça verir).
- **`FormCheck.warning`:** yumuşak denetim. Bozulunca `form_check_warning` uyarısı verir, kaydı durdurmaz.
- **`Group.collapsed`:** grup kapalı açılır. Durumu değiştirmez, yalnız görünüştür.
- **`FormBody.clearHidden`:** formla yazılan kayıtta gizlenen alanın değeri silinir (Survey123'ün `relevant`'ı, Field
  Maps'in “gizli değerleri koruma”nın tersi). Yoksa değer korunur (ADR 0401 §6).
- **`note` öğesinin metni** ADR 0225'in şablon dilidir: `{ifade}` kaydın değeriyle yazılır (QGIS'in metin öğesi,
  Field Maps'in bilgi öğesi). Süslü ayraç yoksa metin düz yazıdır.

### 2. Ön denetimin ekleri

- `defaultExpr`, `calculate`, `choiceFilter` ve notun şablonları derlenir; derlenmeyen `form_rule_invalid` hatasıdır.
- Birbirini dışlayan anahtarlar `form_rule_conflict` hatasıdır: `default` ile `defaultExpr`; `calculate` ile
  `default`, `defaultExpr` ya da `readOnlyWhen`.
- **Hesap döngüsü** `form_calc_cycle` hatasıdır: hesaplanan alanlar okudukları hesaplanan alanlardan sonra hesaplanır. A
  B'yi, B A'yı okursa form kullanılamaz.
- Değer listesi olmayan alanın süzgeci `form_choice_unlisted` hatasıdır.
- Katmanın zorunlu alanının hesabı da kabul edilir: değer hesaptan gelir.
- Notun okunmayan şablonu `form_rule_invalid` hatasıdır (kuralın yeri `text`).
- Anı ya da kullanıcıyı okuyan varsayılan ve hesap `form_moment_only` uyarısıdır; ileti alanın adını söyler (§5).

### 3. Kaydın oluşumu ve tamamlanması (`Prepared::complete`)

Bir kaydın formdan geçen her yazmada izlediği sıra:

1. **Yeni kayıt** (komutların var olan `defaults` girdisi, belgenin izlemesinde adımda eklenen nesne): verilen
   değerler önce yazılır; boş kalan alana düz varsayılan, sonra formun sırasıyla ifadeli varsayılan. Anı ya da
   kullanıcıyı okuyan ifade, bunlar verilmemişse atlanır, alan boş kalır.
2. **Hesaplar:** okudukları hesaplanan alanlara göre sıralanmış olarak (topolojik sıra, eşitlikte formun sırası).
   Sonuç alanın türüne çevrilir:
   - sayı en kısa yazımıyla;
   - evet/hayır `true` ya da `false`;
   - tarih `YYYY-AA-GG`;
   - boş değer alanı boşaltır.
   Alanın kuralına uymayan sonuç (aralığın dışı, listede olmayan kod) `form_calc_invalid` hatasıdır. Kayıt yazılmaz.
3. **Gizlenenler:** `clearHidden` açıksa, bu kayıtta gösterilmeyen alanların değeri silinir. Hesaplar silinen
   değerlerle yeniden yapılır, iki tur yeter. Kural derinliği 4 düzeyle sınırlı olduğundan, bu sınır sabit noktayı
   bulunur kılar. Üçüncü turda değişen kalırsa `form_unstable` hatasıdır.

Doğrulama tamamlanmış kayıtta yapılır. Formla yazmada (`assess`) kullanıcının hesaplanan alana verdiği değer, hesapla
aynıysa kabul, değilse `form_read_only` hatasıdır.

### 4. Yazma profili her yolda: belgenin izlemesi

ADR 0401 §8'de yalnız `cad.entities.set` formun kurallarını soruyordu. Artık **belge** kurallarını uygulayan formun
katmanında, bir adımda eklenen ya da değişen her nesneyi kayıttan önce tamamlar. Bu, ilişkili ölçünün ve taramanın
izlemesiyle aynı yerdedir (`follow_into`, ADR 0249 §3). Böylece çizim araçları, Python, MCP, içe aktarma ve
yapıştırma aynı varsayılanları ve hesapları alır. QGIS'te bunlar yalnız formun açıldığı yolda olur.

- Yeni nesne (adımda eklenen): §3'ün 1–3'ü.
- Öznitelikleri ya da geometrisi değişen nesne: §3'ün 2–3'ü. Geometri değişince `$alan` okuyan hesap da yenilenir
  (QGIS'in “güncellemede uygula”sı, her zaman açık).
- Tamamlama yalnız formun alanlarını yazar ve aynı geri alma adımına girer.
- **Yeni nesnenin eksikleri reddedilmez.** Zorunlu bir alan boşsa nesne yazılır; eksiği `cad.forms.validate` söyler,
  Öznitelikler'in Form satırı formu açar. Adımın sonucuna ayrıca uyarı eklenmez: izleme bütün yazma yollarının ortak
  adımıdır, her komutun sonuç sözleşmesini değiştirmez. Taslak ile tamamlanmış kayıt ayrımı plan FORM-DATA-06'dadır.
  Var olan nesneyi formun kuralını bozacak biçimde değiştiren yazma `form_rules` ile reddedilir (ADR 0401 §8);
  hesaplanan alana hesabından başka değer yazmak da buna girer.
- İzleme cihaza bağlı değerleri okuyan ifadeleri atlar (`HOST_ONLY`: `@tarih`, `@simdi`, `@kullanici`,
  `@koordinat_sistemi`); projenin değişkenleri, adı, EPSG'si ve ölçeği okunur. Böylece kayıt tekrar oynatılınca aynı
  sonucu verir.
- Hesap belge katmanında yapılır: `kentos_domain` formlar çekirdeğini kullanır. Çekirdek arayüz, ağ ve pencere bilmez
  (`pnpm arch:deps`).

### 5. Kuralların gördüğü değerler

- **`$uzunluk`, `$alan`, `$merkez_x`, `$merkez_y`, `$köşe_sayısı`:** nesnenin kendi şeklinden. Çizim biriminde
  okunur, Öznitelikler gibi (ADR 0165'in “ifadeler çizim biriminde” eki).
- **`@tarih`, `@simdi`, `@kullanici`:** komutun açık girdisinden (`moment`, `user`). Verilmezse bu değerler yoktur:
  onları okuyan kural boş değer görür. Masaüstü anı ve kullanıcının adını kendisi verir. Belgenin izlemesi ise
  tekrarlanabilir olmalıdır (ADR 0253'ün kaydı ve tekrar oynatması). Orada anı ve kullanıcıyı okuyan hesap yalnız formla
  yazmada (`cad.forms.submit`) yapılır, belgenin izlemesinde atlanır. Ön denetim bunu `form_moment_only` uyarısıyla
  söyler.

### 6. Komutlar

- `cad.forms.validate` ve `cad.forms.submit`'e `moment` (`2026-10-16` ya da `2026-10-16T14:30:00`, yerel saat) ve
  `user` (boş olmayan, başı ve sonu boşluksuz, en çok 64 karakter) eklendi. Okunmayan an `invalid_moment`, uymayan ad
  `invalid_user` ile, kimliğin biçiminden sonra, formdan önce reddedilir.
- Yeni kayıt ADR 0401'in `defaults` girdisidir (§3'ün 1'i); ayrı bir `new` eklenmedi.
- `cad.forms.submit`'in sonucunda yumuşak denetimler `form_check_warning` uyarısıdır (`path`: `values.<anahtar>`).
  Uyarı yazmayı durdurmaz.
- `cad.forms.validate` her öğenin durumuna sunulan seçenekleri (`choices`, süzgeci olan alanlarda) ve notların yazılmış
  metnini (`text`) ekler.
- **Tekil alan:** komut katmanın öbür nesnelerine bakar (`form_unique`, hata).

### 7. Masaüstü

- **Formla düzenle:**
  - Hesaplanan alanlar kilitli, değerleri her değişiklikte yenilenir.
  - Süzülen listeler yalnız sunulan seçenekleri gösterir. Açılışta da doğrudur, QGIS'in hatası burada yoktur.
  - Uyarılar sarı satırdır, kaydetmeyi durdurmaz.
  - Notlar kaydın değerleriyle yazılır. Kapalı gruplar başlığına basılınca açılır.
  - “Son değeri kullan” alanları oturumun son değeriyle açılır.
- **Tasarımcı:** denetçide Varsayılan (değer ya da ifade), Hesap, Seçenek süzgeci, Tekil, Son değeri kullan, denetimin
  Uyarı'sı, grubun Kapalı açılır'ı, formun Gizlenen değerleri sil'i. İfadeler İfade oluşturucuyla yazılır.
- An cihazın saatinden (saniyeye, yerel saat), kullanıcı oturum açan kişinin adından verilir; oturum yoksa kullanıcı
  verilmez.
- KentOS UI: kapalı (salt okunur) giriş soluk zeminli ve soluk yazılıdır; `Form` uyarıyı yardımın yerinde sarı yazar
  (`Form::warning`), hata onun yerini alır.
- **Ertelenen:** formu uygulanan katmanda çizilen nesnenin formunun kendiliğinden açılması (QGIS'in “öznitelik formunu
  bastırma” ayarı gibi kullanıcının tercihiyle), tasarımcıda geri alma, sürükleme ve canlı önizleme, çoklu düzenleme.
  Sonraki form gruplarındadır (TODOS.md §16.6).

## Denetimler

- Ortak durumlar KentOS kodu olmadan yazılmış başvurudan (`scripts/fixtures/forms_cases.py`). İfadeler
  `expression_language.py`'nin değerlendiricisiyle, şablonlar `template_cases.py`'nin kuralıyla:
  - çekirdeğin durumlarına hesaplar, döngü, ifadeli varsayılan, süzülen seçenekler, uyarılar, gizlenenin silinmesi,
    notların metni ve geometri değerleri eklenir;
  - komut durumlarına hesaplı form, `moment`, `user` ve retleri, tekil alan ve uyarı eklenir;
  - belgenin izlemesi komutların yolundan denetlenir: `cad.entities.set.forms.json`'da hesapların yenilenmesi ve
    hesaplanan alana yazmanın reddi, `cad.polygon.create.forms.json`'da (20 durum) çizilen alanın varsayılanları ve
    `$alan`'lı hesapları.
- Masaüstünün akış testleri ve resimleri `form_scenes.rs`'te.

## Sonuçlar

- Kural, varsayılan ve hesap bütün yazma yollarında aynıdır. Bu, QGIS'in yalnız formda geçerli kısıtlarından ve açılışta
  yanlış süzülen listesinden ayrıldığımız yerdir.
- Belge formlar çekirdeğine bağlanır. Bir adımın izlemesi, formu uygulanan katmanda adımın nesneleri kadar ifade
  değerlendirir.
