# ADR 0437: Tasarımcı ekleri: yansıma, ağaç, adım adım, kopyala ve yapıştır, klavyeyle taşıma

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AT`; `plan/form.md` FORM-DESIGN-02, 03, 04, 07, 10, 12, 15, 22, 23 ve
  FORM-QA-01. Sahibin isteği (17 Ekim): ilişkili maddeler birlikte, tek test turuyla.
- **Bağlam belgesi:** ADR 0401 (form), 0403 (tasarımcı), 0431 (giriş biçimleri), 0434 (şema), 0435 (paket).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Karar

1. **Yansıma** (`mirror` alan öğesinde, yalnız `true`): aynı alan formda ikinci kez, salt okunur gösterilir (özet sekmesi gibi);
   değeri alanın kendi öğesi yazar. Yansımanın yalnız başlığı, yardımı, görünüşü ve görünme koşulu olur (`form_mirror_rules`);
   alanın kendi öğesi formda olmalı (`form_mirror_alone`); iki yansımasız öğe yine `form_field_twice`. Çekirdekte yansıma
   bağlanmaz: yazılmaz, denetlenmez, gizlenince silinmez. Tasarımcıda alanın özelliklerinde **Yansımasını ekle**, ağaçta
   “· yansıma”.
2. **Ağaç** (`tree` giriş biçimi): tek değerli, değer listeli alanın etiketlerindeki son “ / ”den öncesi dal olur (“Konut /
   Apartman”); kodlar listenindir.
3. **Adım adım** (`steps` gövdede): en az iki sekmeli formda sekmeler numaralı adımlardır; Geri ve İleri, hatası olan adımdan
   ileri geçilmez (kayıt o adımda kalır ve söylenir), Kaydet ve iş akışının adımları son adımdadır. Karşılaştırma “Form adım adım
   doldurulur.” der.
4. **Kopyala ve yapıştır**: tasarımcıda seçili öğe (içindekilerle) kopyalanır ve aynı ya da başka bir forma yapıştırılır; her
   kimlik yenilenir, alanlar hedef katmana anahtarla, yoksa adla bağlanır ve ifadeler yeniden yazılır (`rebind_body`); sekme yalnız
   sekmeli forma.
5. **Klavyeyle taşıma**: Alt+↑ ve Alt+↓ seçili öğeyi Yukarı ve Aşağı gibi taşır. Öğe araçları araç çubuğunun ikinci satırında
   (dar pencerede taşmıyor).
6. Var olanların kaydı: alan bağı kararlı anahtarla ve giriş biçimi alanın türüne göre süzülerek, değer listesi bağı türüyle
   denetlenerek (FORM-DESIGN-03); formun yapısı piksel değil sekme, grup ve sütun (FORM-DESIGN-12); tasarım taşınabilir JSON ve
   şeması (FORM-DESIGN-22, ADR 0434); ajan yalnız taslağı yazar, yayın ayrı bir adımdır ve fark `cad.forms.compare`'dedir
   (FORM-DESIGN-23).

## Denetimler

- `forms_cases.py --check`: yansımanın dört ön denetim ve iki kayıt durumu, ağacın üç durumu; `kcad.py` ve şema (`mirror`, `steps`,
  `tree`); `form_schema_cases.py`.
- Masaüstü: `a_step_form_keeps_a_step_with_an_error_and_its_mirror_writes_nothing`,
  `a_copied_group_is_pasted_with_fresh_ids_and_its_fields_rebound`; resimler `form-adim-adim`, `form-adim-ozet`, `form-adim-tasarim`.

## Sonuçlar

- Uzun formlar adım adım ve özetle doldurulur; bölümler formlar arasında taşınır.
- Açık: sürümlü ve parametreli bölüm şablonu kitaplığı, bölücü (splitter), ön denetimde mobil giriş biçimi uyumu.
