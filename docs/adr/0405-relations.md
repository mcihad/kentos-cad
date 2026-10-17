# ADR 0405: İlişkiler: kalıcı tanım, silme politikası, bağlama ve koparma

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek.
- **Bağlam belgesi:** ADR 0404 (başka katmandan liste, `$uid` ile başvuru, ilişkili kayıtlar öğesi); `plan/form.md`
  FORM-REL-01, -03, -04, -05, -11; TODOS.md §16.6 `FORM-I`.
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** N:M bağlantı tablosu, geometrisiz tablo katmanı, formun içinden yeni alt kayıt oluşturma (alt kaydın
  geometrisi çizilmeli).

## Bağlam

ADR 0404 ile bir kayıt başka nesneye kalıcı kimliğiyle bağlanabiliyor, ilişkili kayıtlar formda listeleniyor. Ama bağın
kendisi bir yerde tanımlı değil: üst nesne silinince alt kayıtlar ne olacak, kim bağlayabilir, bağ hangi alanda — her form
kendi başına söylüyor.

- **QGIS:** proje ilişkileri (Project Properties › Relations): ad, üst ve alt katman, alanlar, güç (Association,
  Composition). Composition'da üst silinince alt silinir, kopyalanınca alt da kopyalanır. Kısıt veri kaynağına bırakılır
  (PostgreSQL'de ON DELETE). İlişki düzenleyicisinde alt nesneyi bağlama ve koparma.
- **ArcGIS:** ilişki sınıfı (relationship class): basit ya da bileşik, mesajlaşma yönü, kardinalite; bileşikte üst silinince
  alt silinir. Kurallar geodatabase'dedir.

## Karar

### 1. Proje ilişkisi (`relations`, proje ayarı, `.kcad` şema 72 içinde)

```json
{ "id": "bina-sayac", "name": "Binanın sayaçları", "parent": "bina", "child": "sayac", "field": "bina",
  "onDelete": "unlink" }
```

- `parent`, `child`: katman kimlikleri; `field`: alt katmanın, üst nesnenin kalıcı kimliğini tutan alanının anahtarı.
- `onDelete`: üst nesne silinince alt kayıtlar:
  - `keep`: bağ olduğu gibi kalır (silinmiş nesneye başvuru; form `form_lookup_missing` gösterir);
  - `unlink`: alt kaydın alanı boşalır (varsayılan);
  - `cascade`: alt kayıtlar silinir (bileşik ilişki);
  - `restrict`: alt kaydı olan üst nesne silinmez (`relation_restricted`).
- En çok 100 ilişki; kimlik ve ad formların kuralıyla. Komut katmanların var olduğunu ve `field`'in alt katmanda
  bulunduğunu denetler; dosya biçimi yalnız şeklini.

### 2. Belge politikayı izler

Bir adımda üst katmandan nesne silinince belge, ilişkili ölçü ve taramalar gibi, aynı adımda alt kayıtları izler (`unlink`:
alan boşalır, `cascade`: alt kayıt silinir, alt kaydın kendi ilişkileri de izlenir; `relation_follow.rs`). Kilitli
katmandaki alt kayıt değişmez, bağı kalır. Bütün silme yolları (Sil, Kes, Python,
MCP, katmanı silme) aynıdır. `restrict` silmeyi reddeden bir kuraldır: `cad.entities.delete` alt kaydı olan üst nesneyi
`relation_restricted` ile reddeder; başka yolda silinen üst nesnenin alt kaydı `keep` gibi kalır ve Denetle (ADR 0306)
bozuk bağı söyler.

### 3. Komut: `cad.relations.edit`

`create`, `update`, `remove`. Proje ayarıdır (formlar gibi geri alma adımı değildir). Retler sırasıyla: `invalid_relation`
(kimlik, ad, katmanlar ve alan yazılmalı), sürüm, `unknown_relation`, `relation_exists`, `too_many_relations`,
`layer_not_found`, `not_a_layer`, `relation_field_unknown`.
Python ve MCP katalogdan.

### 4. Formda bağlama ve koparma

İlişkili kayıtlar öğesi (ADR 0404 §4) **Bağla** ile alt katmanın bağsız nesnelerinden birini seçtirir, alanına bu kaydın
kimliğini yazar; satırın **Bağı kopar**'ı alanı boşaltır. İkisi de `cad.entities.set` ile, alt kaydın kendi formunun yazma
profilinden geçerek yazar. Bağlandıktan sonra pencere kaydı yeniden hazırlar, yazılan değerler kalır.

## Denetimler

- Ortak durumlar KentOS kodu olmadan yazılmış başvurudan: `cad.relations.edit` (22 durum), `cad.entities.delete`'in
  ilişkilerle durumları (`cad.entities.delete.relations.json`, 21 durum: unlink, cascade zinciri, restrict ve girdideki yeri,
  keep, kilitli alt katman, geri alma). Bağlar `$uid:` ile, koşucular onu öznitelik beklentilerinde de çözer.
- Masaüstünde İlişkiler penceresi (`data.relations`, Veri › Tablo ve Yönet › Form), ilişkili kayıtlar kutusunda Bağla ve
  Bağı kopar; akış testi ve resimler `form_scenes.rs`'te.

## Sonuçlar

- İlişkinin kuralı formdan bağımsız, projededir; bütün silme yollarında aynı izlenir.
- Alt kaydın üst nesnesi silindiğinde ne olacağı bir kez, açıkça seçilir.
