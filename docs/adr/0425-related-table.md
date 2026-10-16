# ADR 0425: İlişkili kayıtların tablosu

- **Durum:** kabul edildi (2026-10-17). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle getirene kadar
  sürdürmek. TODOS.md §16.6 `FORM-AH`; `plan/form.md` FORM-REL-10 (ve FORM-DESIGN-06'nın ilişkili toplamı).
- **Bağlam belgesi:** ADR 0404 §4 (ilişkili kayıtlar öğesi), 0405 (ilişkiler), 0410 (geometrisiz kayıtlar), 0414 (alanın kullanıldığı
  yerler).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

İlişkili kayıtlar öğesi alt kayıtları adlarıyla, birer satırda listeliyordu. Bir binanın bağımsız bölümleri ya da bir parselin
hisseleri gibi alt kayıtların birkaç alanı birlikte okunur ve toplanır. QGIS'in ilişki düzenleyicisinin tablo görünümü alt kayıtları
öznitelik tablosu olarak gösterir; toplamı yoktur.

## Karar

- Öğenin `view`'i: `list` (varsayılan, yazılmaz) ya da `table`; `columns`: alt katmanın alanlarının anahtarları (en çok 8; boşsa ilk
  dört alanı, bağ alanı hariç). Kuralı sözleşmede (`body_problem`: en çok 8, boş ve yinelenen yok); olmayan sütun ön denetimin
  `form_lookup_field` hatasıdır. Sütunlar alt katmanın alanlarını anahtarla anar: yeniden adlandırmada anahtar kalır, Alanın
  kullanıldığı yerler onları sayar.
- Formda tablo: başlıkta alanların adları, her alt kayıt bir satır (değerler alanın gösterimiyle, sayılar sağa yaslı), satırda Aç,
  Çizimde göster ve Bağı kopar; ondalık sayı sütunlarının toplamı tablonun altında (alan, pay gibi ölçüler; tam sayı çoğu zaman bir
  numara ya da kattır). Yeni ve Bağla tablonun altında kalır.
- Tasarımcıda ilişkili kayıtlar öğesinin Görünüm'ü (Liste, Tablo) ve Sütunlar'ın çipleri.
- `.kcad` şema 60'ta öğenin `view` ve `columns`'ı; bağımsız okuyucu `kcad.py` ve form ikizi (`forms_cases.py`) kuralı yazar.

## Denetimler

- Python: `test_a_related_table_names_its_columns`; `forms_cases.py --check`, `kcad_v2_reference.py --check`, `kentos-kcad`.
- Masaüstü: `a_related_table_lists_the_units`; resimler `form-iliskili-tablo`, `form-iliskili-tasarim`.

## Sonuçlar

- Alt kayıtlar formda okunur bir tabloda; ölçülerin toplamı elle hesaplanmaz.
- Açık: kart ve ağaç görünümleri, tabloda yerinde düzenleme, sayfalama (şimdilik ilk 200 satır).
