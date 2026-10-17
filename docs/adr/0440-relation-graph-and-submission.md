# ADR 0440: İlişkilerin grafı ve kaydın alt kayıtlarıyla gönderilmesi

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AW`; `plan/form.md` FORM-REL-04, -06, -09, -14, -15, -16, -17,
  FORM-DESIGN-05, FORM-WORK-06, FORM-API-06.
- **Bağlam belgesi:** ADR 0404 (ilişkili kayıtlar), 0405 (ilişkiler ve silme politikası), 0409 (çoktan çoğa, Yeni), 0425
  (ilişkili tablo), 0428–0429 (iş akışı ve geçmişi), 0253 (komut protokolü).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** bulutta izinlere göre süzülen alt kayıt sayıları ve sayfalar, rollerle doğrulanan gözden geçiren, sunucunun
  ilişkiler arası işlem sınırı.

## Bağlam

İlişkiler (ADR 0405) silmede izleniyordu ama:

- silmenin planı alt kayıtlara ne olacağını söylemiyordu; kilitli katmandaki alt kayıt sessizce olduğu gibi kalıyordu;
- formda Bağla ve Bağı kopar hemen ayrı adımlar olarak yazılıyordu; kayıt ile alt kayıtları tek gönderim değildi;
- alt kaydı açmak, üst kaydın kaydedilmemiş değeri varken reddediliyordu (taslak kaybolmasın diye);
- ilişkinin alanı, bağları tutan kayıtlar varken değiştirilebiliyordu; bileşik ilişkiler döngü kurabiliyordu;
- iş akışı adımı kaydın hangi hâli üzerinde atıldığını söylemiyordu; gönderimin cevabı neyin yazıldığını ayrıntısıyla vermiyordu.

QGIS'in ilişki düzenleyicisi alt kaydı ayrı bir düzenleme oturumunda yazar; ArcGIS'te bileşik ilişkinin silmesi geodatabase'in
işidir, sayısı önceden görülmez.

## Karar

### 1. Silmenin ilişkiler üzerindeki etkisi

- `cad.entities.delete`'in planı ve cevabı `followed`'ı (ilişki başına bağı koparılan ve silinen alt kayıt sayısı, ilişkilerin
  sırasıyla) ve `attachments`'ı (silinen nesnelerin, izlenen alt kayıtlar dahil, ekleri) taşır. Hesap belgenin `relation_effects`'idir:
  `follow_relations` ile aynı yürüyüş, hiçbir şey yazmadan.
- Kilitli katmandaki alt kaydı değiştirecek (unlink) ya da silecek (cascade) silme reddedilir (`relation_locked`); kilit kaydı
  arkadan değiştirmeye izin değildir. Belgenin kendi izlemesi (komutsuz yollar) kilitli alt kaydı eskisi gibi bırakır.
- Bir adımda en çok 10 000 alt kayıt izlenir (`relation_too_large`).

### 2. Kayıt alt kayıtlarıyla tek gönderimde

`cad.forms.submit`'in `children`'ı: her biri bir proje ilişkisiyle (`relation`, üst katmanı formun katmanı olmalı,
`relation_mismatch`), alt kaydın kimliği (`uid`), değerleri (`values`) ve `unlink` (Bağı kopar).

- Alt kayıt çizimde varsa ilişkinin alt katmanında olmalıdır (`form_layer_mismatch`); bağ alanına kaydın kimliği yazılır (çoktan
  çoğada listeye eklenir), `unlink` onu çıkarır.
- Yoksa çağıranın verdiği kimlikle geometrisiz kayıt oluşur: kimlik çağıranındır, aynı kimlikle yeniden gönderim ikinci kayıt
  yapmaz, aynı kaydı yazar (yinelemeye dayanıklı, istemcide üretilebilir). Olmayan nesnenin bağı koparılmaz (`entity_not_found`).
- Değerler alt katmanın yayımlanmış formu varsa onunla (varsayılanlar, kurallar, zorunluluk), yoksa alanların kuralıyla yazılır;
  hata yolu `children[i].values.<alan>`'dır.
- Kayıt ve bütün alt kayıtlar tek geri alma adımıdır (“Form: <ad>”); biri reddedilirse hiçbiri yazılmaz.

### 3. İlişkinin kullanımı ve grafı

- `cad.relations.edit` bağları sayar (`links`: ilişkinin alanında değer tutan alt kayıtlar; değiştirilen ve silinen ilişkinin
  öncesi).
- Bağları tutan kayıtlar varken ilişkinin alt katmanı ya da alanı değişmez (`relation_in_use`): bağlar kaybolur ya da yanlış
  okunur. Değerler önce Veri aktar ya da Öznitelik hesapla ile taşınır.
- Bileşik ilişkiler başka katmanlar üzerinden döngü kuramaz, zinciri en çok 8 katman derinliğindedir (`relation_cycle`). Katmanın
  kendisiyle bileşik ilişkisi (kayıtların ağacı) döngü sayılmaz; belge aynı kaydı bir kez izler.

### 4. Anlık görüntü

Kaydın anlık görüntüsü (`snapshot`): öznitelikleri (iş akışının durum ve geçmiş alanları dışında), ekleri ve alt kayıtları
(üst katmanı kaydın katmanı olan her ilişkinin, kaydın kimliğini tutan alt kayıtları, kimlikleriyle sıralı, öznitelikleriyle)
kanonik JSON'unun SHA-256'sının ilk 16 onaltılık hanesi.

- `cad.forms.submit` yazdıktan sonraki anlık görüntüyü verir; `cad.forms.transition` adımın atıldığı anlık görüntüyü verir ve
  geçmiş satırının sonuna `· #…` olarak yazar.
- Formla düzenle son adımın anlık görüntüsünü kaydın şimdiki hâliyle karşılaştırır: aynıysa “Son adım bu kayıt üzerinde atıldı”,
  değilse kaydın, alt kayıtlarının ya da eklerinin adımdan sonra değiştiğini söyler. Gözden geçiren neyi onayladığını bilir.

### 5. Gönderimin cevabı

`FormsSubmitted`'a:

- `children`: alt kayıtlar yazıldıkları gibi (kimlik, ilişki, `created`, öznitelikler, değişti mi);
- `changes`: yazılan kayıtlar (kimlik, katman, `created`, değişen alanların adları), önce kayıt sonra alt kayıtlar;
- `snapshot`;
- `undo`: yazıldıysa geri alma adımının adı.

Protokolün zarfı (istek kimliği, plan kimliği, beklenen sürüm, uyarılar, çakışma; ADR 0253) aynen geçerlidir.

### 6. Tasarımcı ve görünümler

- İlişkili kayıtlar öğesinin **İlişki** seçimi: projenin üst katmanı formun katmanı olan ilişkileri; seçilen ilişki katmanı ve
  alanı doldurur.
- **Başvuru**: formun katmanını alt katman sayan ilişkiden, üst kayda kimliğiyle bağlanan alan öğesi (başka katmandan liste,
  `$uid`).
- İlişkili kayıtların **Kartlar** görünümü (`view: cards`): her alt kayıt bir kart, sütunların adları ve değerleri, Aç, Çizimde
  göster ve Bağı kopar. Tablo ve kartlar 50'şer sayfalanır; tablonun toplamları bütün alt kayıtlarındır.
- Gizli öğenin alt kayıtları aranmaz (durumunda `related` yoktur); sekmenin içi açılınca okunur.

### 7. Formla düzenle'de alt kayıtlar

- Proje ilişkisiyle kurulan öğede Bağla ve Bağı kopar kaydın taslağına girer, Kaydet ile kayıtla aynı gönderimde yazılır, Vazgeç
  bırakır; liste bekleyen bağları gösterir.
- Alt kaydın formu üst kaydın formunun üstünde açılır: üstteki form taslağıyla bekler, alt kayıt kaydedilince ya da vazgeçilince
  yenilenip geri gelir; başlık “altta … formu bekliyor” der. En çok 4 form iç içe açılır.

## Denetimler

- `scripts/fixtures/forms_cases.py --check`: `cad.forms.submit.children.json` 21 durum; `cad.entities.delete.relations.json`'a kilitli
  alt katmanın reddi, kilide dokunmayan ilişki ve planın sayıları; `cad.relations.edit.json`'a döngü, kendi kendine bileşik ilişki,
  bağlar varken alan değişimi ve bağların sayısı; gönderimin bütün cevaplarında anlık görüntü, değişiklikler ve geri alma adı.
- `scripts/fixtures/forms_transition_cases.py --check`: geçmiş satırının anlık görüntüsü ve cevabın `snapshot`'ı.
- Alt kaydı olan anlık görüntü gerçek kimliklere bağlıdır; ortak durumlarda onlar `$any`'dir, alt kaydı olmayanlar hesaplanır.
- Masaüstü: `a_link_waits_for_kaydet_and_goes_in_with_the_record`,
  `a_child_form_opens_over_its_parent_and_the_parent_comes_back_with_its_draft`,
  `a_step_s_snapshot_is_logged_and_a_later_change_is_said`; resimler `form-iliski-kartlar`, `form-iliski-bekleyen`,
  `form-iliski-yigini`, `form-iliski-tasarim`, `form-onay-goruntusu`.

## Sonuçlar

- Silmeden önce alt kayıtlara ne olacağı görülür; kilitli katman sessizce atlanmaz.
- Kayıt ve alt kayıtları tek adımda, ya hep ya hiç yazılır; istemcinin ürettiği kimlik yeniden gönderimde çoğalmaz.
- Alt kayda geçmek üst kaydın taslağını kaybettirmez.
- Gözden geçiren onayladığı hâli parmak iziyle bilir; sonradan değişen kayıt söylenir.
