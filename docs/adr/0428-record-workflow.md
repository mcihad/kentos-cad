# ADR 0428: Kayıt iş akışı

- **Durum:** kabul edildi (2026-10-17). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle getirene kadar
  sürdürmek. TODOS.md §16.6 `FORM-AK`; `plan/form.md` FORM-WORK-01, 02, 04, 05 (yerel kural; roller ve yetki FORM-DEC-12 ile bulutta).
- **Bağlam belgesi:** ADR 0401 (form, ön denetim), 0402 (kurallar), 0403 (formla düzenle).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Saha verisi çoğu zaman bir akıştan geçer: sahada taslak, merkeze gönderilir, onaylanır ya da gerekçesiyle geri gönderilir; onaylı
kayıt kendiliğinden değişmez. QGIS'in formunda böyle bir akış yoktur; Survey123 ve ArcGIS'te ayrı uygulamalarla kurulur.

## Karar

### 1. Model

Formun gövdesinin `workflow`'u (`kentos_contracts::FormWorkflow`): durum alanı (`field`, değer listeli ve tek değerli), ilk durum
(`initial`), kilitli durumlar (`locked`) ve adımlar (`steps`, en çok 20): adın (`label`, akışta bir kez), atıldığı durumlar (`from`;
boşsa her durumdan), götürdüğü durum (`to`), geçerli kayıt isteyip istemediği (`valid`) ve gerekçe alanı (`reason`, serbest metin
alanı). Kuralı `workflow_problem`; ön denetimin `form_workflow_invalid` hatası.

### 2. Kayıtlar

- Kaydın durumu durum alanındadır; boşsa ilk durum. Yeni kayıt ilk durumla başlar.
- Durum alanı formda salt okunurdur: yalnız adımlar değiştirir. Kilitli durumdaki kaydın bütün öğeleri salt okunurdur; o durumdan
  atılan adımların gerekçe alanları açık kalır. Kuralları uygulayan form başka yollardan yazmayı da aynı kuralla reddeder.
- `cad.forms.transition` v1 (“İş akışı adımı”): `form`, `draft`, `uid`, `step` (adın), `values` (adımla yazılan değerler; kilitli kayıtta
  yalnız gerekçe), `moment`, `user`, `expectedRevision`. Adım kaydın durumundan atılmalı; verilen değerler formun kurallarıyla yazılır
  (verilen değerin hatası adımı durdurur, kaydın öbür sorunları yalnız geçerlilik isteyen adımı); durum alanı adımın durumunu alır;
  kayıt yeniden tamamlanır (hesaplar); `valid` adımda formun hatası olan kayıt, `reason` adımda boş gerekçe reddedilir. Tek geri alma
  adımı “Form: <adım>”. Retler: `invalid_uid`, `invalid_moment`, `invalid_user`, `invalid_revision`, `revision_conflict`,
  `unknown_form`, `form_unpublished`, `form_preflight`, `no_workflow`, `entity_not_found`, `form_layer_mismatch`, `layer_locked`,
  `unknown_step`, `step_not_allowed`, `record_locked`, `form_invalid`, `reason_required`. Python `kentos.cad.forms.transition`, MCP'de
  araç (yıkıcı listede).

### 3. Formla düzenle

Altta, Kaydet'in yanında kaydın durumundan atılan adımların düğmeleri; basılınca yazılan değerlerle adım atılır, pencere kapanır
ve ““Onayla”: kayıt “Onaylandı” durumunda.” denir. Kilitli kayıtta “Kayıt “Onaylandı” durumunda; değiştirilemez” şeridi, Kaydet
kapalı.

### 4. Tasarımcıda

Üçüncü sütunun **İş akışı** bölümü: İş akışı ekle (katmanın ilk liste alanı ve iki durumuyla), durum alanı, ilk durum, kilitli
durumların çipleri; her adım bir kutu: adı, şu durumlardan (çipler), şu duruma, Geçerli kayıt ister, gerekçe alanı; Adım ekle,
İş akışını kaldır.

### 5. Dosya

`.kcad` şema 60'ın formunda `workflow`; bağımsız okuyucu `kcad.py` yapıyı denetler (katmana uyumu ön denetimindir).

## Denetimler

- `scripts/fixtures/forms_transition_cases.py --check`: 32 durum (`fixtures/commands/v1/cad.forms.transition.json`), formun bağımsız
  Python ikiziyle.
- Python: `test_a_record_moves_through_its_workflow`.
- Masaüstü: `a_step_of_the_workflow_writes_the_state_and_closes_the_editor`; resimler `form-is-akisi-tasarim`, `form-is-akisi-adim`,
  `form-is-akisi-kilitli`.

## Sonuçlar

- Kayıtlar onay akışından formun kendi kuralıyla geçer; aynı akış Python ve ajanla yürür.
- Açık: rollere göre adımlar ve gözden geçirenin ayrılığı (FORM-DEC-12, bulut), adımların geçmişi (kim, ne zaman, hangi anlık
  görüntü; FORM-WORK-03, 06), görev listesi (FORM-WORK-14).
