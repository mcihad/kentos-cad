# ADR 0336: Parametreler, tek işlem ve kısıtların taşınması

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §13 `CAD-PARAM-04`, `10`, `12` (sahibin sözü: “cad param
  ile devam et ve bitene kadar durma”; beş grubun üçüncüsü). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0334 (model ve çözücü), 0335 (kısıt komutları ve aracı), 0253 (komut protokolü, damgalar), 0300 (ilişkili
  dizilerin veritabanı projesinde taşınması), 0251 (pafta kitabının projede taşınması).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0334 parametre tablosunu ve ifade dilini kurdu; ADR 0335 ölçü kısıtının parametresini adlandırdı ve değerini yazdı. Ama
kullanıcının kendi parametresi (bir uzunluk, bir oran) eklenemiyor, bir parametrenin ifadesi değiştirilip çizim onunla yeniden
çözülemiyor, parametre yeniden adlandırılamıyordu (AutoCAD'in parametre yöneticisi). Kısıtlar `.kcad`'de taşınıyordu ama veritabanı
projesinde, bulut eşitlemesinde, protokolün durum damgalarında yoktu: bir veritabanı projesine kaydedilen kısıtlar kaybolurdu.

## Karar

### 1. Parametre komutu (`CAD-PARAM-04`)

**`cad.parameters.edit` v1** (anlamsal; tek geri alma adımı “Parametreler”): `edits` sırasıyla uygulanır, sonra çizim yeni
değerlerle çözülür ve parametreler, oynayan nesnelerle birlikte yazılır (`CAD-PARAM-10`).

- `add`: kullanıcı parametresi (ad, tür, ifade, isteğe bağlı aralık ve açıklama).
- `set`: ifade, alt ve üst sınır (`clear` kaldırır), açıklama; verilmeyenler kalır. Yalnız ölçen ölçünün parametresine ifade
  verilmez (`parameter_measured`).
- `rename`: yeni ad; onu okuyan ifadeler ve ölçü kısıtının `parameter`'ı aynı adımda izler (ifadenin sözcükleri yeniden yazılır,
  sayılar, birimler ve öbür adlar dokunulmaz).
- `remove`: başka bir parametrenin okumadığı kullanıcı parametresi; ölçünün parametresi kısıtıyla gider (`parameter_read`).
- Değerler parametrenin kendi biriminde: uzunluk çizim biriminde, açı projenin açı biriminde, sayı birimsiz. Bağımlılık grafı ve
  döngü ADR 0334 §1'inki (`param_order`): döngü adlarıyla söylenir. Aralığın dışındaki değer `parameter_range`.
- Çıktı her parametrenin değerini (`values`), oynayan nesneleri ve çözücünün teşhisini verir.

### 2. Tek işlem (`CAD-PARAM-10`)

Kısıt ekleme ve kaldırma (`cad.constraints.edit`), parametre değişikliği (`cad.parameters.edit`) ve çözümün oynattığı bütün
nesneler her biri tek geri alma adımıdır: belgenin `transact`'ı içinde nesneler ve kısıt kümesi birlikte yazılır, ret hiçbir şey
bırakmaz. İzleme kuralları (silinen nesnenin kısıtları, ADR 0334) aynı adımdadır.

### 3. Protokol ve taşınma

- **Damga:** `ProjectPart::Constraints` ve `StateStamps.constraints` (kısıt kümesinin kanonik JSON'unun SHA-256'sı). Kısıtları değiştiren
  çağrının özeti `project`'te `constraints`'i sayar; beklenen damgası tutmayan çağrı `stale_state` olur (ADR 0253).
- **Veritabanı projesi:** `kentos.project.constraints` (jsonb; göç 0022): kısıt kümesi sözleşmenin JSON'u, boşsa null. Projenin
  meta verisidir (pafta kitabı ve ilişkili diziler gibi): değişince meta sürümü artar; kopyada (`duplicate_project`), kontrol
  noktasında ve geri yüklemede taşınır; `ProjectPatch.constraints` (yok: dokunulmaz; boş: kaldırılır) uygulamanın kuralıyla
  (`constraints_problem`, nesnelerin kimlikleri olmadan) denetlenir; görüntü (`.kcad`) projenin nesnelerinin tuttuğu kısıtları
  yazar (bir nesnesi olmayan kısıt düşer).
- **Bulut eşitlemesi:** masaüstünün eşitlemesi kısıtları meta olarak taşır (`ExternalMeta.constraints`), dosya projesi `.kcad`
  revizyonuyla.

### 4. Python ve MCP (`CAD-PARAM-12`)

`kentos.cad.constraints.edit`, `.query` ve `kentos.cad.parameters.edit` katalogdan üretilir; `Document.constraints()` tabloyu okur.
Oluşturma, listeleme, değiştirme, kaldırma, çözme (`solve`) ve teşhis (`query`) aynı çözücüden geçer; MCP'de araçlar, yazan ikisi
yıkıcı listede.

### 5. Sözleşmenin ekleri

- `constraints_kept(set, uids)`: bir kümenin, var olan nesnelerin tutabildiği kadarı (görüntü, açılış, başka düzenleyicinin kümesi);
  sonra kural bozuluyorsa kusurlu parametre ya da kısıt, onu anan kısıtlarla, sırayla düşer: kalan küme her zaman kurallarını tutar.
- `rename_in_expression(text, from, to)`: yalnız tam adlar; sayının birimi, işlevler ve öbür adlar dokunulmaz.
- `BoundChange`: `{ "kind": "clear" }` ya da `{ "kind": "to", "value": … }`.
- Kodlar: `parameter_exists`, `parameter_not_found`, `parameter_measured`, `parameter_read`.

## Doğrulama

- `python3 scripts/fixtures/parameter_command_cases.py --check`: `fixtures/commands/v1/cad.parameters.edit.json`'un 24 durumu, KentOS
  kodu olmadan (çözümler kısıt başvurusundan); masaüstünün ve Python'un oynatıcıları geçer.
- `KENTOS_TEST_DB=required cargo test -p kentos-application --test project_constraints`: veritabanı projesinde kısıtlar (tutulur,
  görüntüde silinen nesneninkiler düşer, bozuk küme yerinde reddedilir, kopya alır, kaldırılır).
- `.run/py/bin/python -m unittest python/tests/test_constraints.py`: Python'dan oda, dikdörtgen kısıtları, sorgu, parametreyle
  genişleme, dosya, geri alma ve çelişki.
- Protokol: `python3 scripts/fixtures/protocol_cases.py --check`; kısıtları değiştiren çağrının özeti `constraints` damgasını taşır.
