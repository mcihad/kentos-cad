# ADR 0415: Kaydı alt kayıtlarıyla çoğaltma

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-V`; `plan/form.md` FORM-REL-18.
- **Bağlam belgesi:** ADR 0405 (ilişkiler ve silme politikası), 0410 (geometrisiz kayıt), 0401 (formun tekil alanı), 0278
  (kilitli katman kuralı), 0047 (kilitli katmandaki nesnenin kopyası yapılmaz).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir binanın katları ve odaları ayrı katmanlardaki kayıtlardır; bina silinince bileşik ilişki (`cascade`) onları da siler. Benzer bir
bina girilirken QGIS'te “Duplicate Feature and Children” bileşik ilişkinin alt kayıtlarını da kopyalar; KentOS'ta yalnız Kopyala
vardı: alt kayıtlar kalıyor, formun tekil alanı kopyada tekrar ediyordu.

## Karar

### 1. Komut

`cad.records.duplicate` v1 (“Kaydı çoğalt”): `uids` (1–1000 nesne), `children` (alt kayıtlar da; varsayılan açık),
`expectedRevision`. Her nesne aynı yerde, aynı katmanda, bütün özellikleri ve ekleriyle yeni kalıcı kimlikle kopyalanır. Tek geri
alma adımıdır.

- **Alt kayıtlar:** kopyalanan nesnenin katmanı üst katmanı olan her **bileşik** (`cascade`) ilişkide, alanı nesnenin kimliğini
  tutan alt kayıtlar da kopyalanır ve alanları kopyanın kimliğini alır; alt kaydın kendi bileşik ilişkileri de izlenir. Her nesne bir
  kez kopyalanır: üst kaydıyla kopyalanan, girdide ayrıca verilmişse yeniden kopyalanmaz. `unlink`, `restrict` ve `keep`
  ilişkilerinin alt kayıtları kopyalanmaz (yalnız bileşik ilişki sahipliktir).
- **Tekil alanlar:** katmanın bir formunun tekil işaretlediği alan (taslakta ya da son sürümde) kopyada boş kalır; değer
  boşaltıldıysa `unique_cleared` uyarısı. Formun zorunlu tuttuğu alan boş kalırsa yazmayı formun kuralı reddeder (`form_rules`).
- **Kilitli katman:** kilitli katmandaki nesne atlanır, hepsi kilitliyse reddedilir (ADR 0278, `layer_locked`); kilitli katmandaki
  alt kayıt çoğaltılmaz ve sayısı uyarıda söylenir.
- **Sıra ve retler:** kimlikler (`no_entities`, `invalid_uid`), sayı (`invalid_count`), sürüm, nesneler (`entity_not_found`),
  kilit. Çıktı her kopyanın kaynağı, kimliği ve yuvası (`copies`, kaynağın üst kaydından sonra alt kayıtları sırayla) ve sürüm;
  planı kopya ve alt kayıt sayıları.

### 2. Masaüstünde

Öznitelik tablosunun çubuğunda ve Veri › Tablo'da **Çoğalt** (`data.duplicate`): seçili nesneleri alt kayıtlarıyla çoğaltır,
kopyalar seçilir, kaç kopya ve alt kayıt yapıldığı söylenir. Komut satırında `COGALT`. Öznitelik tablosu ve Öznitelikler bir
ilişkinin alt alanında üst nesnenin kimliği yerine etiketini gösterir (alan orada salt okunur; bağ formdan kurulur).

## Denetimler

- `scripts/fixtures/duplicate_records_cases.py --check`: 20+ ortak durum (`fixtures/commands/v1/cad.records.duplicate.json`) KentOS
  kodu olmadan.
- Masaüstü: `form_scenes`'in testi, resim `form-cogalt`.

## Sonuçlar

- Kayıt, sahip olduğu alt kayıtlarla tek adımda çoğaltılır; tekil değer kopyalanmaz.
- Açık: kopyayı başka yere koymak (öteleme), ilişki başına çoğaltma politikasının ayrı seçilmesi, N:M bağ tablosu.
