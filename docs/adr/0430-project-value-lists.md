# ADR 0430: Projenin değer listeleri

- **Durum:** kabul edildi (2026-10-17). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle getirene kadar
  sürdürmek. TODOS.md §16.6 `FORM-AM`; `plan/form.md` FORM-DATA-10, 11, 12, 13, 14, 22'nin yerel kısmı.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0418 (kullanımdan kalkan değer), 0422 (alt türler), 0423 (değer bileşimleri).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir değer listesi (yapı malzemesi, kat sayısının aralığı) çoğu zaman birçok katmanda aynıdır. Alanın kendi listesiyle her
katmanda ayrı yazılır ve zamanla ayrışır. ArcGIS'te bu iş “domain”dir; QGIS'te yoktur (her katmanın Value Map'i ayrıdır).

## Karar

### 1. Model

- Projenin ayarının `domains`'i (`kentos_contracts::FieldDomain`): kimlik, ad, açıklama, uyduğu alan türü (`kind`) ve ya kod
  listesi (`values`: kod, etiket, kullanımdan kalktı) ya da aralık (`min`, `max`, ikisi de dahil), ikisi birden değil. En çok 200
  liste, adlar büyük küçük harf gözetmeden tek. Kuralı `domains_problem`; listenin kodları ve aralığı o türdeki bir alanın
  kurallarıyla denetlenir.
- Alanın `domain`'i bağlandığı listenin kimliğidir. Bağlı alanın `values`'u (ya da `min` ve `max`'ı) listenin kopyasıdır: alanı
  okuyan her kural (değerin denetimi, form, tablo, alt türler, bileşimler, içe ve dışa aktarma) değişmeden onu okur. Liste
  değişince belge bağlı bütün alanların kopyasını aynı adımda yeniden yazar (`Document::set_domains`, `follow_domains`). Listesi
  olmayan bağ (başka çizimden gelen alan) kopyasını tutar; yeni bir bağ projede olmayan listeye kurulamaz.
- Liste geri alma adımındadır (`Op::Domains`): ayar olduğu hâlde, bağlı alanları değiştirdiği için geri alma ikisini birlikte
  geri verir (birim değişikliğinin kuralı, ADR 0256 §1).

### 2. Uyum

Alan yalnız kendi türündeki listeye bağlanır; kopya alanın kurallarına uymalıdır (ondalık kodlar alanın basamağına sığar, çok
değerli alanın kodlarında “;” olmaz). Uymayan bağ `domain_mismatch` ile reddedilir (`link_problem`).

### 3. Kodların göçü

Etiketin değişmesi kodu ve kayıtları değiştirmez. Kayıtların tuttuğu bir kod listeden yalnız çıkamaz (`domain_code_in_use`):
`recode` onu tutan bağlı alanların kayıtlarını yeni listenin bir koduna aynı adımda taşır (çok değerli alanda yinelenen kod bir
kez kalır) ya da kod kullanımdan kaldırılır (ADR 0418). Kilitli katmandaki kayıt taşınmaz (`layer_locked`). Aralık daralınca
dışarıda kalan kayıtlar `domain_values_outside` uyarısıyla söylenir.

### 4. Komut

`cad.domains.edit` v1 (“Değer listeleri”): `create` (kimlik verilmezse adından), `fromField` (alanın kendi listesinden liste yapıp
alanı bağlar), `update` (türü değişmez; `recode`), `remove` (bağlı alanı olan liste `detach` olmadan silinmez; `detach` alanları
kopyalarıyla çözer), `link` (alanı listeye bağlar ya da bağı çözer, kopya alanın kendi listesi olur). Cevap: liste, tanımı değişen
alanlar, taşınan kayıt sayısı, değişip değişmediği. Retler sırasıyla: `invalid_revision`, `revision_conflict`, `unknown_domain`,
`layer_not_found`, `not_a_layer`, `field_not_found`, `domain_exists`, `invalid_domain`, `domain_mismatch`, `invalid_recode`,
`domain_code_in_use`, `domain_in_use`, `layer_locked`. Python `kentos.cad.domains.edit`, MCP'de araç (yıkıcı listede).

### 5. Masaüstü

- **Değer listeleri** penceresi (`data.domains`, CBS › Veri › Form paneli): listeler, Yeni liste, Alanın listesinden…, düzenleyici
  (ad, tür, açıklama, Kod listesi | Aralık, kodlar ve Kullanımdan kalktı), **Çıkan kodlar** (taşınacakları kod), **Kullanan
  alanlar**. Kaydet bütün değişiklikleri tek geri alma adımında yazar ve kapanır; ret pencereyi açık bırakır ve nedenini söyler.
- **Alanlar** penceresinin değer listesinde **Projenin listesi**: alan bir listeye bağlanır, kodları salt okunur görünür; listenin
  düğmesi “↗ Liste adı”, aralığa bağlı alanın En az ve En çok'u kapalı.

### 6. Dosya

`.kcad` şema 72'nin ayarlarında `domains` (JSON olarak, `domains_problem` ile), alanda `domain`; bağımsız okuyucu `kcad.py` ve
yazıcı `kcad_v2_reference.py`, örnek `domains.kcad` ve beş bozuk dosya.

## Denetimler

- `scripts/fixtures/domains_cases.py --check`: 41 durum (`fixtures/commands/v1/cad.domains.edit.json`), KentOS kodu olmadan.
- `scripts/fixtures/kcad_v2_reference.py --check`; `cargo test -p kentos-kcad`.
- Python: `test_a_project_list_is_shared_by_fields_and_its_codes_move`.
- Masaüstü: `a_list_dropping_a_code_moves_its_records_in_one_undo_step`, `a_code_records_hold_keeps_the_window_open_until_it_is_moved`;
  resimler `deger-listeleri`, `deger-listesi-alan`.

## Sonuçlar

- Bir liste bir kez yazılır, birçok katman izler; kodun göçü kayıtlarla aynı adımdadır.
- Açık: kurumun ve kişinin listeleri (kapsam, sahip, yayın durumu; FORM-DATA-10'un bulut kısmı), etiketlerin dilleri ve geçerlilik
  aralığı (FORM-DATA-11), büyük listelerin sayfalanması (FORM-DATA-16).
