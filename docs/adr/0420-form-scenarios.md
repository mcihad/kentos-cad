# ADR 0420: Form senaryoları

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-AB`; `plan/form.md` FORM-DESIGN-24.
- **Bağlam belgesi:** ADR 0401 (form, ön denetim), 0402 (kurallar ve hesaplar), 0403 §3 (tasarımcının önizlemesi).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir formun kuralları (görünme, zorunluluk, hesap, varsayılan, denetim) büyüdükçe bir değişikliğin başka bir durumu bozması kolaylaşır.
QGIS'te ve ArcGIS Field Maps'te formu sınamanın yolu elle önizlemektir; kaydedilen, yeniden çalışan bir sınama yoktur.

## Karar

### 1. Senaryo

Formun gövdesinin `tests`'i (en çok 50): her senaryonun adı (formda bir kez, 1–64 karakter), boş bir kayda yazılan değerleri
(alanların anahtarlarıyla), yeni kayıt olup olmadığı (`new`: varsayılanlar dolar) ve beklenen: kaydın değerleri, gösterilen,
zorunlu ve salt okunur öğeler, sorunlar (`kod@öğe`). Senaryo anı ve kişiyi okumaz (`@tarih`, `@simdi`, `@kullanici` boştur): sonra
da aynı sonucu verir. Senaryo gövdeyle kaydedilir, yayın onu da dondurur.

Çalıştırmak formun bugünkü hâlinin sonucunu beklenenle karşılaştırmaktır (`kentos_forms::scenario::run`); farklar sırayla, birer
cümle: kaydın değerleri (alan adına göre), gösterilen, zorunlu, salt okunur öğeler, sorunlar.

### 2. Komut

`cad.forms.test` v1 (salt okunur, “Form senaryolarını çalıştır”): `form`, `draft` (taslağın senaryoları taslakla; yoksa son yayınınkiler
o yayınla), `names` (yalnız verilenler sırasıyla). Çıktı her senaryonun sonucu, geçen ve toplam sayı. Retler: `unknown_form`,
`form_unpublished`, `form_preflight`, `unknown_test`. Python `kentos.cad.forms.test`, MCP'de araç: bir CI işi formun bozulmadığını
denetler.

### 3. Tasarımcıda

Önizleme'nin altında **Senaryolar**: her senaryo geçiyor (✓) ya da düşüyor (uyarı ve farkları); “n / m senaryo geçiyor”; ad ve
**Senaryo olarak kaydet** önizlemedeki değerleri ve formun şimdiki sonucunu senaryo yapar; × siler. Değişiklikler taslağın geri
alınabilen adımlarıdır.

## Denetimler

- `scripts/fixtures/forms_test_cases.py --check`: 20 durum (`fixtures/commands/v1/cad.forms.test.json`), formun bağımsız Python
  ikiziyle (`forms_cases.py`'nin `Form`'u).
- `kentos-kcad`, `kcad_v2_reference.py --check` (örnek formun taslağında bir senaryo).
- Masaüstü: `a_scenario_is_kept_and_fails_when_the_form_changes`; resim `form-senaryolar`.

## Sonuçlar

- Form değiştikçe önceki durumların bozulup bozulmadığı bir bakışta görülür; aynı sınama Python ve MCP'den çalışır.
- Açık: senaryonun bir nesnenin geometrisiyle çalışması (`$alan`), başka katmandaki kayıtlara bağlı senaryolar.
