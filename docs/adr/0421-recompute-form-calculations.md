# ADR 0421: Hesapları yenile

- **Durum:** kabul edildi (2026-10-17). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle getirene kadar
  sürdürmek. TODOS.md §16.6 `FORM-AC`.
- **Bağlam belgesi:** ADR 0402 (kurallar ve hesaplar), 0403 §5 (formla toplu yazma), 0406 (Kayıtları denetle), 0417 (değiştirilebilir
  hesap).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Formun hesapları kayıt formla yazılırken yapılır (ADR 0402). Form sonradan değişince (yeni hesap, değişen ifade, gizlenenleri silme
açılınca) önceden yazılmış kayıtlar eski hâlinde kalır. QGIS'te bunun yolu Alan hesaplayıcısıdır: her alan için ifadeyi yeniden yazmak
gerekir, formun kuralı tekrar kullanılmaz. Kayıtları denetle (ADR 0406) sorunları gösterir ama düzeltmez.

## Karar

`cad.forms.recompute` v1 (“Hesapları yenile”): formun katmanındaki her kayıt formun bugünkü kurallarıyla tamamlanır
(`Prepared::complete`, var olan kayıt olarak: varsayılanlar dolmaz): hesaplar yeniden yapılır, değiştirilebilir hesabın elle
yazılmış değeri kalır, form gizlenenleri siliyorsa gizlenen alanlar boşalır. Değişen kayıtlar tek geri alma adımında yazılır
(“Hesapları yenile: <ad>”). Hesabı alanına sığmayan ya da formu durulmayan kayıt olduğu gibi kalır, `failed`'da sayılır ve
`form_calc_failed` uyarısı verilir.

- Girdi: `form`, `draft` (yoksa son yayın), `moment` ve `user` (hesaplar `@tarih`, `@simdi`, `@kullanici` okursa; verilmeyince o
  hesaplar yapılmaz), `expectedRevision`.
- Çıktı: `checked` (katmandaki kayıt sayısı), `changed` ve `failed` (katmanın sırasıyla), `version`, `revision`.
- Retler: `invalid_moment`, `invalid_user`, `invalid_revision`, `revision_conflict`, `unknown_form`, `form_unpublished`,
  `form_preflight`, `layer_locked`.
- Python `kentos.cad.forms.recompute`, MCP'de araç (yıkıcı listede).

Masaüstünde Formlar penceresinin altında **Hesapları yenile**: kaydedilmemiş taslakta önce Kaydet ister; yayını olan formda son
yayınla, olmayanda taslakla çalışır; sonucu durum çubuğunda söyler.

## Denetimler

- `scripts/fixtures/forms_recompute_cases.py --check`: 21 durum (`fixtures/commands/v1/cad.forms.recompute.json`), formun bağımsız
  Python ikiziyle (`forms_cases.py`).
- Masaüstü: `recomputing_writes_the_calculation_on_every_record_as_one_step`; resim `form-hesap-yenile`.

## Sonuçlar

- Form değişince eski kayıtlar tek düğmeyle, geri alınabilir biçimde yeni kurala getirilir; aynı iş Python ve MCP'den yapılır.
- Açık: yalnız seçili kayıtlar, büyük katmanda arka planda çalışma ve ilerleme çubuğu.
