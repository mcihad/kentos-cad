# ADR 0439: Kaynaklar, başvurular ve göç

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AV`; `plan/form.md` FORM-SRC-03, -04, -06, -07, -08, FORM-REL-12,
  -20, FORM-DATA-05, -16, -22.
- **Bağlam belgesi:** ADR 0404 (başka katmandan liste), 0405 (ilişkiler), 0216 (Veri aktar), 0200 (Anahtarla birleştir), 0430
  (değer listeleri), 0431 (aranır liste).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** kurum servisleri ve dış kaynaklar (FORM-SRC-01, -02, -09–12), bulutun yetkili araması ve önbelleği, sunucunun
  bekçisinde mekânsal daraltma, değer etiketlerinin başka dilleri.

## Bağlam

Başka katmandan liste (ADR 0404) yalnız anahtar, etiket, zincir ve süzgeç taşıyordu:

- liste hep etiketin doğal sırasıyla, bütünüyle forma iniyordu; seçilen kaydın başka alanları görünmüyordu;
- zincir metinleri harfi harfine karşılaştırıyordu (`06` ile `6`, `İNÖNÜ` ile `inönü` ayrı), alanların türü denetlenmiyordu;
- aynı anahtarı tutan ikinci kayıt sessizce atlanıyordu (ilk eşleşme doğru sayılıyordu);
- değerin kopyası ile canlı görünüşü ayrılmamıştı; kopyalanan değerin nereden geldiği bilinmiyordu;
- binanın parseli bütün parsellerden seçiliyordu, binanın durduğu parselden değil.

QGIS'in Value Relation'ı sıralama ve süzgeç verir, kopyalamayı ve mekânsal seçimi ayrı eklentilere bırakır; ArcGIS Field Maps'te
liste Arcade ile hesaplanır, yazımı ayrıca denetlenmez. Eski verinin metin öznitelikleri (içe aktarmadan `1,5`, `17.10.2026`, `evet`)
alanlar tiplenince uymayan değer olarak kalıyordu; bunları kayıpsız çevirecek bir yol yoktu. Değer listeleri projeler arasında
taşınamıyordu.

## Karar

### 1. Listenin sırası ve sayfası

`FormLookup`'a:

- `sort` (`field`, `descending`): listenin katmanının bir alanına göre sıra; sayılar değerleriyle, tarih ve saatler yazıldıkları
  gibi, metin doğal sırayla; boş değerler her iki yönde sonda; eşitlikte etiketin doğal sırası. Yoksa etiketin doğal sırası.
- `limit` (1–1000): formun durumu (`FormItemState.options`) listenin ilk sayfasını, `more` kalanın sayısını taşır. Python, MCP
  ve sunucu bütün listeyi indirmez; denetim bütün listeyle yapılır, arama hepsine ulaşır.

### 2. Sütunlar ve canlı önizleme

`columns` (en çok 4): listenin katmanının alanları (ya da `$uid`), değerin etiketinin yanında okunuşlarıyla (tarih `01.03.2024`,
evet/hayır, liste etiketi). Durumda seçeneğin `description`'ı sütunların ` · ` ile birleşimidir; seçilen kaydın sütunları
`preview`'dadır (etiketiyle). Önizleme canlıdır: kaynak değişince değişir, kayda yazılmaz.

### 3. Kopya ve kaynağı

`copy` (`from`, `to`, en çok 20): değer seçilince listenin kaydının alanları kaydın alanlarına kopyalanır; `stamp` kaydın bir metin
alanına kopyanın kaynağını yazar: `‹katman› · ‹kod› · #‹parmak izi› · ‹gün›`.

- **Ne zaman:** değer değişince (yeni kayıtta değer verilince) ya da damga başka bir kodu anıyorsa. Değer aynı kalınca kopya
  yeniden yapılmaz: kopya canlı görünüş değildir.
- **Parmak izi:** kopyalanan değerlerin FNV-1a'sı (32 bit; her değer UTF-8 baytları ve U+001F ile, boş değer hiçlik), sekiz küçük
  onaltılık hane. Gün kuralların `@tarih`'idir (ev sahibi verirse).
- **Kaynak değişince:** kaydın değeri damganın kodunu anıyor ama kaynağın şimdiki parmak izi farklıysa `form_lookup_stale`
  (uyarı): değeri yeniden seçmek kopyayı yeniler.
- Kaynağın boş alanı hedefi boşaltır; değer boşalınca kopyalar kalır.
- Bütün yazma yollarında aynıdır: formla yazma (`assess`), belgenin izlemesi (`form_follow.rs`, adımın öncesindeki değerle), yazma
  profili (`enforce`): formun göstermediği kopya hedefi kopyanın değeriyle yazılabilir, başka değerle `form_not_on_form`.

### 4. Türler, karşılaştırma ve yinelenen anahtar

- **Ön denetim:** saklanan alan formun alanına yazılabilmeli (aynı aile: tam ve ondalık sayı, tarih, tarih ve saat, saat,
  evet/hayır; ondalık tam sayıya yazılmaz; her tür metne yazılır); zincirin iki alanı aynı aileden; kopyanın kaynağı hedefe
  yazılabilmeli; damga metin alanı; kopyanın hedefi listenin değeri ya da damga olamaz. Uymayan `form_lookup_type`; sütun, sıra ve
  kopya alanı yoksa `form_lookup_field`.
- **Karşılaştırma:** zincirde iki sayı alanı değerleriyle karşılaştırılır (`6` `06`'yı bulur); metin harfi harfine, `fold` ile büyük
  küçük harf ve Türkçe harfler katlanarak (`İ`/`i`, `I`/`ı`), uçlar kırpılarak. Değer ifade metnine konmaz: tırnak, yüzde ve özel
  karakterler değer olarak kalır.
- **Boş:** anahtarı boş kayıt listede yoktur; kaydın boş zincir alanı hiçbir şey sunmaz.
- **Yinelenen anahtar** (`duplicates`): bir kodu birden çok kayıt tutuyorsa ilkinin etiketi ve alanları alınır;
  - `warn` (varsayılan): o kodu tutan kayda `form_lookup_duplicate` uyarısı;
  - `first`: bilerek seçildi, sessiz;
  - `refuse`: `form_lookup_duplicate` hata.
- İşlem araçları da aynı soruyu sorar: Anahtarla birleştir ve Veri aktar'a **Yinelenen anahtar** parametresi: İlk satırı al
  (varsayılan, söylenir), Aktarma (bu anahtarlı hedeflere bir şey aktarılmaz, Veri aktar'da satırları eklenmez), Durdur (araç
  çalışmaz, ilk yinelenen anahtarı söyler).

### 5. Mekânsal liste

`spatial` (`relation`, `tolerance`): listenin yalnız kaydın geometrisine göre şöyle duran kayıtları sunulur:

- `contains`: kaydı içeren (binanın parseli);
- `within`: kaydın içinde kalan;
- `intersects`: kesişen; toleransla, aradaki uzaklık tolerans kadarsa;
- `nearest`: en yakın tek kayıt; toleransla, tolerans içindeyse; eşit uzaklıkta listenin sırasıyla ilki.

İlişkiler Konuma göre seç'inkilerdir (`ops::spatial_query`, ADR 0200 §1; yaylar kesin). Tolerans çizim biriminde, metreye çevrilir;
koordinat sistemi projenindir (iki katman aynı sistemdedir). Geometrisi olmayan kayda liste bir şey sunmaz. Sunucunun bekçisi
şekilleri okumaz: dünyası şekil taşımayan liste mekânsal daraltılmaz (bulutta açık).

### 6. Aranır liste

- Alanın `minSearch`'ü (1–10): arama bu kadar harf yazılınca başlar; altında “En az n harf yazın”.
- Yazma duraksayınca (150 ms) aranır; her arama artan bir sırayla bekler, sırası geçen ya da başka alana ait cevap düşer
  (eski sonuç yeni alana uygulanmaz).
- Öneriler sayfa sayfa (12'şer), **Daha fazla göster** bir sayfa ekler; önce etiketinde, sonra sütunlarında geçen değerler.
- Sayfalı listenin araması bütün listededir (`Prepared::options`); 200'den uzun ya da sayfalı açılır liste aranır listeye döner.

### 7. Değer listelerinin paketi

- `cad.domains.export` v1 (sorgu): verilen kimliklerin listeleri verildiği sırayla (her biri bir kez), kimlik yoksa hepsi
  projenin sırasıyla; paket `kentos.domains` v1 (`.kliste`). Kayıt, bağ ya da yetki taşımaz. Olmayan kimlik `unknown_domain`.
- `cad.domains.edit`'in `import` işlemi: paketin listeleri eklenir; projede aynı kimlikli liste aynıysa olduğu gibi kalır, farklıysa
  `domain_exists`, `replace` ile Kaydet gibi değiştirilir (türü değişmez `domain_mismatch`, bağlı alanlara uymalı, kayıtların
  tuttuğu kod çıkamaz `domain_code_in_use`, aralık dışında kalan kayıt `domain_values_outside` uyarısı). Boş paket ya da iki kez
  geçen kimlik `invalid_domain`. Bir liste reddedilince hiçbiri eklenmez. Cevabın `imported`'ı eklenen ya da değişen listeler.
- Değer listeleri penceresinde **İçe aktar…**, **Dışa aktar…** ve **Aynı kimlikli listeleri değiştir**. Alt türler ve bileşimler
  kendi komutlarıyla tanımlarının bütünüyle yazılır (ADR 0422, 0423); onların dosyaya alınıp verilmesi açık.

### 8. Değerleri biçime çevir

`cad.fields.migrate` v1: katmanın değerleri alanlarının tek biçimine (ADR 0199) yazılır: `1,5` `1.5`, `17.10.2026` `2026-10-17`,
`17.10.2026 14:30` `2026-10-17T14:30:00`, `8:05` `08:05:00`, `evet` `true`, listedeki değerin etiketi kodu (çok değerlide her
biri). Alan verilmezse katmanın listesiz metin olmayan bütün alanları, katmanın sırasıyla.

- **Kayıpsız:** alana uymayan değer yazıldığı gibi kalır ve sayılır (`values_unfit` uyarısı, ilk beş değerle); yalnız boşluk olan
  değer boş kalır (`blank`), özniteliği olmayan nesne sayılır (`absent`); hiçbir alanın adlandırmadığı öznitelik olduğu gibi kalır
  (`unknown`, adları). Geçmiş geri alma adımıdır.
- Retler: `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`, `field_not_found`, değişecek değeri olan
  kilitli katman `layer_locked`. Tek geri alma adımı “Değerleri biçime çevir”.
- Alanlar penceresinde **Değerleri biçime çevir**: kaydedilmiş alanlarla çalışır, sonucu satırda ve bildirimde söyler.

## Denetimler

- `scripts/fixtures/forms_cases.py --check`: kaynak durumları (6 ön denetim, 20 kayıt, 6 yazma, 2 yazma profili; mekânsal olanlar
  `spatial_query_cases.py`'nin ilişkileriyle, sütunlar `layer_field_cases.py`'nin okunuşuyla), KentOS kodu olmadan.
- `scripts/fixtures/sources_cases.py --check`: `cad.domains.export` 21, `cad.domains.edit`'in içe aktarması 22,
  `cad.fields.migrate` 22 durum (içe aktarma `domains_cases.py`'nin kuralıyla, göç `layer_field_cases.py`'nin tek biçimleriyle).
- `scripts/fixtures/spatial_query_cases.py --check` ve `transfer_cases.py --check`: Yinelenen anahtar'ın üçer durumu.
- Masaüstü: `a_chosen_parcel_is_copied_with_its_stamp_and_shown_with_its_columns`,
  `a_searched_list_takes_the_newest_answer_and_drops_a_late_one`, `the_value_lists_go_out_as_a_package_and_come_back`,
  `values_written_before_the_fields_are_turned_into_their_canonical_texts`; resimler `form-kaynak-tasarim`, `form-kaynak-giris`,
  `form-kaynak-arama`, `form-deger-paketi`, `form-degerleri-bicime`.

## Sonuçlar

- Binanın parseli binanın durduğu parsellerden, alanı ve maliki yanında seçilir; kopyası nereden ve ne zaman geldiğini söyler,
  kaynak değişince uyarır.
- Zincir ve anahtar türüyle karşılaştırılır; yinelenen anahtar hiçbir yerde sessizce doğru sayılmaz.
- Uzun liste sayfa sayfa iner ve aranır; geç gelen arama cevabı yeni yazılanın üstüne yazılmaz.
- Değer listeleri projeler arasında dosyayla taşınır; eski metin değerleri kayıpsız ve geri alınabilir biçimde tiplenir.
