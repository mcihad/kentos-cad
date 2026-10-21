# ADR 0579: Adres verisini denetle

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-34'ün kalanından: “adres normalizasyonu ve yinelenen adres kodunun
  gösterilmesi”.
- **Bağlam belgesi:** ADR 0229 (adres arama, Denetle §13).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - Yazılışları düzelten bir komut (en çok yazılanı bütün kapılara yazmak): bugün yalnız söylenir.
  - Mahalle ve ilçe adlarının farklı yazılışları.
  - Adres kodunun UAVT gibi bir dış kayıtla karşılaştırılması.

## Bağlam

Adres verisi penceresinin Denetle'si (ADR 0229 §13) dizinin tuttuklarını, dışarıda kalanları ve aynı yolda yinelenen kapı numaralarını
söylüyordu. İki şeyi söylemiyordu:

- birden çok kapıda aynı adres kodu (UAVT gibi, tek olması gereken bir kimlik);
- bir yolun kapılarının onun adını farklı yazması. “Atatürk Bulvarı”, “ATATÜRK BULVARI” ve “Atatürk Blv.” arama için bir yoldur (adın Türkçe
  katlanmış biçimi bir), ama kayıtta üç yazılıştır. Adresin normalleştirilmesinin ilk bulgusu budur.

Denetle yalnız masaüstündeydi; Python ve yapay zekâ ajanı adres verisini denetleyemiyordu.

## Karar

### 1. Çekirdek

`ops::geocode::check` iki liste daha verir:

- **Yinelenen adres kodu:** birden çok kapıda aynı (kırpılmış) kod; kapıları çizimin sırasıyla.
- **Farklı yazılış:** dizinin bir yolunda (adın katlanmış biçimi, mahalle ve ilçe) kapıların yolun adını birden çok biçimde yazması.
  - Her yazılış kırpılmış metnidir; her biri onu yazan kapı sayısıyla söylenir.
  - Sıra: en çok yazılan önce, eşitlikte önce yazılan.
  - Baştaki ve sondaki boşluk başka yazılış değildir.

Dışarıda kalan kapı (yol adı ya da kapı numarası okunmayan) listelere girmez. Her listenin sayısı bütün listedir; listenin kendisi ilk
kapılarına göre ilk 100 öğedir.

### 2. `cad.addresses.check` v1

Salt okunur komut:

- **Girdi:** `locator` (adres verisinin kimliği; verilmezse projenin ilki) ve `limit` (1–100, varsayılan 100).
- **Çıktı:** sayılar (kapılar, yollar, yol çizgileri, mahalleler, bölgeler), dışarıda kalanlar ve üç liste: `repeats`, `codes`,
  `spellings`. Kapılar nesnelerin kalıcı kimlikleriyle verilir.
- **Retler, sırasıyla:** `invalid_limit`, `no_address_data` (yeni: projede adres verisi yok), `unknown_locator`.

Python (`kentos.cad.addresses.check`) ve MCP komutu katalogdan alır. Masaüstünün Adres verisi penceresi Denetle'de iki satır daha yazar:
“Birden çok kapıda aynı adres kodu” ve “Adı farklı yazılan yol”, her biri ilk sekiz öğesiyle.

## Doğrulama

- **Bağımsız başvuru: `scripts/fixtures/address_check_command_cases.py --check`.**
  - Dizin adres başvurusunun kendi kurucusuyla kurulur (`geocode_cases.py`'nin `build`'i, KentOS kodu yok). Denetimin kuralı bu ADR'dendir.
  - **21 durum:** ilk ve kimliğiyle verilen adres verisi, sınır, plan, üç ret, başka alan adlı ikinci adres verisi, temiz veri, iki mahallede
    aynı ad, eşit ve en çok yazılış, üç kapı bir numarada (5/A ayrı), bir kod üç yolda, kodların sırası, boşluklar, dışarıda kalan kapılar,
    boş çizim, sınırda kesilen liste.
- **Masaüstü:** `KENTOS_CASES_ONLY=cad.addresses.check cargo test -p kentos-native-application --test all every_command_case`.
- **Python:** `python.tests.test_command_cases`.
