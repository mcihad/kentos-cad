# ADR 0436: Tarih ve saat aralığı

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AS`; `plan/form.md` FORM-DATA-12'nin tarih aralığı.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0412 (tarih ve saat, saat), 0430 (değer listeleri).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Alanın aralığı (en az, en çok) yalnız sayı alanındaydı; tescil tarihi, ölçüm saati gibi sınırlar ifadeyle yazılıyordu.
ArcGIS'in aralık domain'i tarihi de alır.

## Karar

- Tarih, tarih ve saat, saat alanında da `min` ve `max` (ikisi de dahil), alanın tek biçiminde (`YYYY-AA-GG`,
  `YYYY-AA-GGTSS:DD:SS`, `SS:DD:SS`); bu biçimler yazıldıkları gibi sıralanır, karşılaştırma metindir (`range_order`). Aralık
  dışı değer `range` ile reddedilir; ileti tarihi arayüzün gösterdiği gibi söyler: “… 01.01.2000 ile 31.12.2026 arasında olmalı;
  01.01.2027 verildi.”
- Alanlar penceresinde tarih ve saat alanlarının En az ve En çok'u açık; 7.10.2026 gibi yazılan uç kaydederken tek biçime
  çevrilir. Değer listeleri penceresinde **Tarih** türü (yalnız aralık).
- Bağımsız başvurular: `layer_field_cases.py` (kuralı ve değerler), `forms_cases.py` ve `kcad.py`.

## Denetimler

- `layer_field_cases.py --check`: dokuz yeni değer durumu (tarih, tarih ve saat, saat aralıkları), dört yeni tanım durumu; eski
  “tarihte aralık reddedilir” durumları artık geçerli.
- Masaüstü: resim `form-tarih-araligi`.

## Sonuçlar

- Tarih sınırları kuralla yazılır; form, tablo ve komutlar aynı iletiyle reddeder.
- Açık: açık uçlu (hariç) aralık ve birimli aralık (FORM-DATA-12'nin kalanı).
