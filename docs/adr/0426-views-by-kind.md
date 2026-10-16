# ADR 0426: Alt türe göre görünüm

- **Durum:** kabul edildi (2026-10-17). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle getirene kadar
  sürdürmek. TODOS.md §16.6 `FORM-AI`; `plan/form.md` FORM-DESIGN-11.
- **Bağlam belgesi:** ADR 0401 (form), 0402 (kurallar), 0422 (alt türler).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Alt türü olan bir katmanın formu çoğu zaman türe göre değişir: sanayi binasının ayrı bir sekmesi, konutun başka bir grubu olur.
Gruplar ve alanlar görünürlük kuralı alıyordu, sekmeler almıyordu; kuralı elle yazmak da türlerin kodlarını bilmeyi gerektiriyordu.

## Karar

- Sekmenin `visibleWhen`'i: kural tutmayan kayıtta sekme ve içindekiler gösterilmez, sorulmaz (grubun kuralı gibi). Sözleşmenin,
  ön denetimin, alanın kullanıldığı yerlerin ve form çekirdeğinin kuralı aynıdır; `.kcad` şema 60'ta, bağımsız okuyucu ve form ikizi
  yazar.
- Tasarımcıda sekme, grup, alan, ilişkili kayıtlar ve ekler öğesinin özelliklerinde **Alt türlerde göster**: katmanın her alt türü
  bir çip; seçilenler görünürlük kuralını `[Tür] içinde ('sanayi', 'depo')` olarak yazar, hiçbiri seçilmezse kural kalkar. Kural
  başka biçimde yazılmışsa çipler kapalıdır ve “kuralı silin” der: elle yazılan kural ezilmez.

## Denetimler

- `subtypes_cases.py --check`: `cad.forms.validate.subtypes`'ın sekmeli formu (konutta gizli, sanayide görünen sekme ve zorunlu notu,
  türü değişince görünen sekme), form ikiziyle.
- Masaüstü: `a_tab_is_shown_in_the_kinds_chosen`; resim `form-alt-tur-sekme`.

## Sonuçlar

- Türe göre form, kod bilmeden tasarlanır; kural yine metindir, Python ve ajan aynı formu okur.
- Açık: kaydın durumuna göre salt okunur görünüm ve görev kipi (FORM-DESIGN-11'in kalanı, iş akışıyla).
