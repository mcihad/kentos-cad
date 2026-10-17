# ADR 0412: Tarih ve saat, saat alanları

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-S`; `plan/form.md` FORM-DATA-04.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0401 (formlar), 0210 (zaman), 0214 (ifade dilinin tarih ve saati).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Katman alanının türleri metin, tam sayı, ondalık sayı, tarih ve evet/hayırdı. Hasar tespitinin anı, vardiyanın başlangıcı, sayaç
okumasının saati tarih alanına sığmıyordu; metin alanına yazılınca ne denetleniyor ne sıralanıyordu. QGIS'te `QDateTime` ve
`QTime` alanları takvim ve saat seçiciyle (DateTime düzenleyicisi) düzenlenir; ArcGIS'te `Date`, `DateOnly` ve `TimeOnly`
ayrıdır.

## Karar

### 1. İki tür

| Tür | Arayüzde | Tek biçim | Okunanlar | Gösterim |
|---|---|---|---|---|
| `datetime` | Tarih ve saat | `YYYY-AA-GGTSS:DD:SS` | tarihin kuralı (`YYYY-AA-GG`, `G.A.YYYY`), ilk `T` ya da boşluktan sonra saatin kuralı; yalnız tarih gece yarısıdır | `GG.AA.YYYY SS:DD`, saniyesi varsa `SS:DD:SS` |
| `time` | Saat | `SS:DD:SS` | `S:DD`, `SS:DD`, `SS:DD:SS` (ASCII rakamlar, saat 0–23, dakika ve saniye 0–59) | `SS:DD`, saniyesi varsa `SS:DD:SS` |

Değer yerel duvar saatidir; saat dilimi yazılmaz (`Z` ya da `+03:00` reddedilir). Aralık ve değer listesi tarihte olduğu gibi bu
türlerde de yoktur. Verilerden al bütün değerleri iki noktalı ve tarih-saat olarak okunan sütunu `datetime`, bütünü saat olan
sütunu `time` yapar (tarihten sonra, evet/hayırdan önce).

### 2. Formda

Formun Tarih öğesi alanın türüne göre açılır: tarih takvim, tarih ve saat takvim ile saat ızgarası (`DatePicker::date_time`), saat
saat paneli (`TimePicker`). Kutuda kaydın değeri insanın yazdığı biçimde durur (`16.10.2026 14:30`, `08:00`), yazılan olduğu gibi
kalır ve formun kuralıyla tek biçime çevrilir. Metin öğesi de bu türlere uyar.

### 3. Başka yerlerde

İfade dilinde tarih ve saat tarih türündendir (ISO metni zaman sırasıyla karşılaştırılır), saat metindir. Öznitelik tablosu iki türü
gösterim biçimiyle gösterir ve zaman sırasıyla sıralar; dışa aktarmada metin hücresidir. Grafikler iki türü metin olarak sayar.

### 4. Dosya

İki tür `.kcad` şema 72'dedir (formlarla aynı şema); şema 71 okuyucusu alanın türünü `bad_value` ile reddeder
(`broken/field-datetime-in-schema-71.kcad`). Örnek `forms.json`'un `Tespit` ve `Vardiya` alanları; bağımsız `tools/kcad/kcad.py`
ve `kcad_v2_reference.py` aynı kuralla.

## Denetimler

- `scripts/fixtures/layer_field_cases.py --check`: iki türün 20 değer durumu, dört alan sorunu, iki tür tahmini ve altı gösterim
  KentOS kodu olmadan; sözleşmenin testi aynı dosyayı geçer.
- Masaüstü: `form_scenes`'in `a_date_and_time_and_a_time_are_written_through_the_form` (yazılan tek biçime çevrilir, gün dışı saat
  reddedilir), resim `form-tarih-saat`.

## Sonuçlar

- An ve saat kendi türleriyle denetlenir, sıralanır ve seçicilerle girilir.
- Açık: saat dilimli an, süre (`interval`) türü.
