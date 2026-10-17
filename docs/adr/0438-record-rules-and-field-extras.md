# ADR 0438: Kayıt kuralları, kontrol zamanı, ekranın yayını ve kaydı, alanın tekil, hariç uç, birim ve değerlerin günleri

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AU`; `plan/form.md` FORM-RULE-01, 05, 07, 09, 10, 15, 17 ve
  FORM-DATA-03, 11, 12. Sahibin isteği (17 Ekim): yalnız masaüstü maddeleri, ilişkili maddeler birlikte, tek test turuyla.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0401 (form), 0418 (kullanımdan kalkan değer), 0423 (değer bileşimleri),
  0430 (değer listeleri), 0253 (komut protokolü).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Karar

1. **Kayıt kuralları** (katmanın `rules`'u, `kentos_contracts::layer_rules`): veri kümesinin kendi kısıtları; her kural bir
   ifade (alanlar adlarıyla, `$alan`, `@` değerleri), ileti, isteğe bağlı uyarı ve gösterildiği alanlar. Kaynağı ne olursa olsun
   katmanın her yazması onları sorar: okuduğu alanı değişen kural (yeni kayıtta her kural) bozulursa `rule_broken`, uyarı kuralı
   `rule_unmet`. Kural ancak cevabı yanlış olunca bozulur; boş cevap tutar (SQL CHECK gibi); dil iki değerli olduğundan boş alan
   “boş” ile karşılanır. Form kuralları yinelemez: katmanın kuralını gösterildiği alanın altında söyler (`form_layer_rule`),
   formun kendi denetimleri formundur (RULE-01). Çizim komutları sert kuralı reddeder; uyarı kuralını Öznitelikler, komutlar ve
   form söyler. Geometri değeri okuyan kural nesnenin şekli bilinen yazmada (Öznitelikler, form) sorulur; çizim komutunun yeni
   kaydında ölçüler boş okunur ve kural tutar. `cad.layers.rules` (tek adım “Kayıt kuralları”, `invalid_rules`: biçim ya da
   derlenmeyen ifade), Python, MCP; Kayıt kuralları penceresi (`data.layerRules`): kural başına derleniyor mu ve var olan kaç
   kaydın bozduğu; Kaydet komutla yazar ve kapanır. Alanın yeniden adlandırılması ifadeyi izler (`field_refs`).
2. **Kontrol zamanı** (`FormCheck.on`: `save`, `submit`; yazılmazsa değer değişince): ekran değişince sorulanları yazarken, Kaydet'e
   basılınca kaydederken sorulanları da gösterir; iş akışında ilerlerken (adımın `valid`'i) hepsi sorulur, iş akışsız formda
   ilerlerken sorulan kaydederken sorulur. Komutlar (`cad.forms.submit`, `.validate`, `.fill`) kaydetme zamanında sorar, zaman
   yalnız ekranın gösterdiğini değiştirir (RULE-07). Tasarımcıda denetim satırında “Ne zaman”.
3. **Tekil alan** (`LayerField.unique`): katmanda boş olmayan bir değer bir kez; her yazmada (`not_unique`), form da bilir.
4. **Hariç uç ve birim** (`minExclusive`, `maxExclusive`, `unit`): aralığın ucu hariç tutulabilir (iletiler “0 hariç” der), sayının
   birimi değerin yanında gösterilir; değer sayıdır. Değer listesi (aralık) de taşır ve bağlı alanlara kopyalar. Sayılar kesin
   ondalıkla karşılaştırılır (DATA-12).
5. **Değerin açıklaması ve günleri** (`FieldChoice.description`, `from`, `until`): açıklama formda değerin altında; günlerinin
   dışındaki değer (kuralların `@tarih`'ine göre) kullanımdan kalkmış gibi yeni seçimde sunulmaz, formla yeniden yazılmaz,
   tutan kayıtta kalır (DATA-11).
6. **Ekranın yayını** (`FormsSubmit.release`): form açıldıktan sonra yeniden yayınlandıysa `form_release_changed` (çakışma), eski
   ekran yeni formun kurallarını atlayarak yazamaz (RULE-15; tekrar ve sürüm damgaları protokolün, ADR 0253).
7. **Ekranın kaydı** (`FormsSubmit.opened`): kayıt form açıldıktan sonra değiştiyse (başka yazma, geri alma) `record_changed`
   (çakışma) ve değişen alanlar; Formla düzenle'de çubuk: Yeniden yükle (yazılanlar kalır) ya da Üzerine yaz. Hesaplar yazmada
   yeniden yapılır, eski hesap yazılmaz (RULE-17). Ekran artık bütün çizimin sürümünü değil yalnız kaydını bekler.

## Denetimler

- `layer_rules_cases.py --check`: `cad.layers.rules` (24), `cad.entities.set` (22), `cad.polygon.create` (21),
  `cad.forms.validate` (21), `cad.forms.submit` (20); kurallar formların bağımsız ikiziyle (forms_cases.py), değerler alanların
  ikiziyle (layer_field_cases.py: hariç uçlar, birim, tekil, değer günleri).
- `.kcad` şema 72: `layer-rules.kcad` ve yedi bozuk dosya, bağımsız okuyucu ve yazıcı.
- Masaüstü: `a_rule_is_shown_on_its_field_and_a_check_kept_for_saving_waits_for_kaydet`,
  `a_record_changed_meanwhile_is_reloaded_with_what_was_typed_or_written_over`,
  `the_rules_window_writes_through_the_command_in_one_step`; resimler `form-kayit-kurallari`, `form-kural-formda`, `form-cakisma`,
  `form-alan-ekleri`, `form-deger-gunleri`, `form-kural-baglari`.

## Sonuçlar ve açıklar

- RULE-07: masaüstünde odaktan çıkınca ayrı bir zaman yok (Iced olayı vermez); değer değişince sorulan onun işini görür.
- RULE-09: hesaplar kaynak alan, geometri, alt tür ve liste değişince yazmada yenilenir, toplu olarak `cad.forms.recompute`; ilişkili
  kayıtların toplamını okuyan hesap dilde yok, gelince bağımlılık grafıyla izlenecek.
- RULE-10: masaüstünde kaynaklar yerel ve eşzamanlı, bekleyen sonuç yok; uzak kaynakta bekleme ve zaman aşımı bulutla.
- DATA-03: birden çok alanlı tekil anahtar ve dizin açık. DATA-11: değer etiketlerinin başka dillerdeki karşılığı açık.
