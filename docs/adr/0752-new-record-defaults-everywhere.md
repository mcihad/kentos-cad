# ADR 0752: Yeni kaydın varsayılanları her yolda aynı sırayla

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözü “formlarımız çok zayıf”; ADR 0750'nin kalanı. Sahibin PostGIS tablosundaki `tur` sütununun
  varsayılanı `'İlçe'`.
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** değişmez. **Komut sözleşmesi:** değişmez; aynı
  komutlar yeni kurala göre yazar.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Yeni bir kayda üç yerden varsayılan gelebilir:

- **katman alanının kendi varsayılanı** (ADR 0199 §2; bağlı tablonun sabit varsayılanı ADR 0750'de alana geçer);
- **alt türünün varsayılanı** (ADR 0422 §2);
- **formun varsayılanı:** düz bir değer ya da ifade (ADR 0402 §3).

Yeni kaydın yapıldığı iki yol bu üçünü farklı sırayla veriyordu:

- **Çizim komutları** (`cad.polygon.create` ve diğerleri, `cad.records.create`) önce alt türün ve alanın
  varsayılanını yazıyordu. Katmanın kuralları uygulanan formu aynı adımda kaydı tamamlarken yalnız boş kalan alanları
  doldurduğundan formun kendi varsayılanı hiç gelmiyordu. Formun alt türü seçen varsayılanı da diğer alanlara alt
  türün varsayılanını getiremiyordu.
- **Formun yeni kaydı** (Formlar penceresinin Önizleme'si, `cad.forms.validate` ve `cad.forms.submit`'in `defaults`'u,
  ilişkili kayıtlarda Yeni) formun varsayılanını ve alt türünkini veriyordu, katman alanınınkini hiç vermiyordu.

Sonuç:

- Sahibin ilçe formunda `tur` boş görünüyordu; komut aynı kaydı `İlçe` diye yazıyordu.
- Form, değer bileşimi uymayan bir yeni kayda “geçerli” diyordu. Oysa komut, alanın varsayılanıyla dolan aynı kaydı
  reddediyordu (ADR 0423'ün ortak durumları).

Başvurulan yollar (30 Ekim 2026'da okundu):

- QGIS'in kullanıcı kılavuzu, öznitelik formunun alan ayarları: varsayılan değer yalnız yeni nesnede alana konur;
  sabit ya da ifade olabilir. QGIS'in varsayılanı oluşturulan nesnenin diğer alanlarının değerlerini bilmez.
  <https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html>
- ArcGIS Pro, alt türlere genel bakış: her alt türün alanlarına, nesne oluşturulurken kendiliğinden konan
  varsayılanlar verilir (yerel yol 25, ana yol 35 mil/saat).
  <https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/an-overview-of-subtypes.html>
- ArcGIS Field Maps'te formun öğesine yazılan varsayılan uygulamaya geçmez, şablonunki geçer; Esri'nin destek kaydı
  varsayılanın şablonda verilmesini söyler. Daha özel olan (şablon, alt tür) alanınkinin önündedir.

## Karar

### 1. Tek sıra: formunki, alt türünkü, alanınki

Yeni kaydın boş alanları her yolda bu sırayla dolar:

1. **Verilen değer:** hiçbir varsayılan onu ezmez. Boş verilen değer verilmemiş sayılır.
2. **Formun düz varsayılanı.** Kayıt bir formla yapılıyorsa o formunki; çizim komutunda katmanın kuralları uygulanan
   formununki (birden çoksa ilkininki).
3. **Alt türün ve alanın varsayılanı:** önce alt tür alanının kendi varsayılanı, sonra her alanın alt türündeki
   varsayılanı, yoksa alanın kendi varsayılanı. Alt türün sunmadığı varsayılan konmaz.
   - Formun varsayılan verdiği alan atlanır.
   - Formda olmayan alanlar da dolar: kayıt, çizim komutunun yazacağıyla aynıdır.
   - Alt tür, formun düz varsayılanından sonra bulunur: formun seçtiği alt türün varsayılanları gelir.
4. **Formun ifadesi** (`defaultExpr`) adımın içinde hesaplanır. İfade, kendinden önce dolan varsayılanları okur:
   `Tür || ' kaydı'` → `İlçe kaydı`.
5. **İfade boş verirse** ya da yalnız ev sahibinin verdiği bir değeri okuyup bekliyorsa (an, kullanıcı; çizimde
   verilmez), alan alt türün ya da alanın varsayılanını alır.

Kural tek yerde, sözleşmenin `kentos_contracts::new_record_defaults`'ındadır (`subtypes.rs`). Formun oturumu
(`Prepared::complete`) ve çizim komutlarının yazacağı öznitelikler (`checks::new_defaults`) onu kullanır. Kuralları uygulanan formun
varsayılanlarını belge söyler (`Document::form_defaults`, `enforcing_forms`).

### 2. Çizim komutları formun düz varsayılanını kendileri yazar

- Kuralları uygulanan formun düz varsayılanı, çizim komutunun yazdığı özniteliklere alanın kuralıyla girer. Böylece
  değer bileşimi, alt tür, kayıt kuralları ve tekil alan denetimleri yazılacak kaydı sorar.
- Formun ifadeyle verdiği alanı komut boş bırakır. Form aynı adımda doldurur (ADR 0402 §4); ifade bir şey vermezse
  alanın varsayılanı gelir.
- Kuralları uygulanan formun katmanına başka yoldan giren yeni nesne de (içe aktarma, yapıştırma, Python, MCP) aynı
  adımda bu sırayla tamamlanır: formun belge düzeyindeki izlemesi artık alt türün ve alanın varsayılanını da verir.
  Formu olmayan katmana içe aktarılan nesne eskisi gibi dosyadaki değerleriyle gelir.

### 3. Formun yeni kaydı ne yazılacaksa onu gösterir

- Önizleme ve Formla düzenle'nin yeni kaydı katmanın varsayılanlarını gösterir: sahibin ilçe formunda `tur` `İlçe`,
  `ek_veri` `{}`, `aktif` işaretli.
- `cad.forms.validate`'in `defaults`'lu cevabı kaydı komutun yazacağı gibi verir; değer bileşiminin sorunu artık
  formda da görünür.

## Doğrulama

- `python3 scripts/fixtures/form_defaults_cases.py --check`: KentOS kodu olmadan, formun ikiziyle (forms_cases.py,
  subtypes_cases.py) yazılmış iki dosya:
  - `cad.forms.validate.defaults.json`: 25 durum. Formda olmayan alanlar, verilen ve boş verilen değer, var olan
    kayıt, formun düz, evet/hayır ve ifade varsayılanı, ifadenin alanın varsayılanını okuması, boş ifade, anı okuyan
    ifade (an verilerek ve verilmeden), salt okunur ve hesaplanan alan, zorunlu alan, gizlenen alan (silinerek ve
    silinmeden), taslak ve yayın, alt türler.
  - `cad.polygon.create.formdefaults.json`: 22 durum. Formsuz katman, kuralları uygulanmayan form, formun düz (metin,
    evet/hayır, tarih, ondalık) ve ifade varsayılanları, anı okuyan ifade çizimde, verilen ve boş değer, yalnız
    taslakta olan varsayılan, gizlenenleri silen form, alt türün varsayılanı, formun seçtiği alt tür, başka katman.
  - Rust'ta `KENTOS_CASES_ONLY=defaults,formdefaults cargo test -p kentos-native-application --test all
    every_command_case`; Python'da `test_command_cases`. Koşucunun dosyayı oynattığı, bir beklenti bozularak
    denendi (1/22 düştü).
- Değişen eski durumlar ikizlerle birlikte yeniden yazıldı, farkları okundu:
  - `cad.forms.validate.subtypes.json`'da üç durumda `Malzeme` alanının varsayılanı `ahsap` artık kayıtta. Durumun adı
    “türsüz: varsayılan yok” iken “türsüz: alanın kendi varsayılanı (ahşap)” oldu.
  - `cad.forms.validate.contingents.json`'da iki durumda `ahsap` kayıtta. “yeni kayıt, varsayılanlarla” artık
    geçersiz: komut aynı kaydı zaten `invalid_combination` ile reddediyordu.
- Formların, belgenin, uygulama katmanının ve masaüstünün bütün testleri; `clippy -D warnings`.
- `cargo test -p kentos-desktop --bin kentos-cad district_form_screens -- --ignored --test-threads=1`:
  `form-ilce-2-onizleme-*` ve `form-ilce-2b-onizleme-asagi-*`. Önizlemenin yeni kaydında `ek_veri` `{}`, `tur`
  `İlçe`, `aktif` işaretli.

## Kalan

- Alt tür alanını formun ifadesi veriyorsa çizim komutu, ifade adımda hesaplanmadan diğer alanlara alt türün
  değil alanın kendi varsayılanını yazar.
- Veritabanının hesapladığı sütunun (`now()`, `uuidv7()`) formda “veritabanı verir” diye söylenmesi (ADR 0750).
- Yabancı anahtarın başka katmandan liste olarak önerilmesi (ADR 0750).
