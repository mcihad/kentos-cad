# ADR 0513: Servisten veri almanın bitişi ve OGC API'nin süzgeç profili

- **Durum:** kabul edildi (2026-10-18). `plan/gis.md` CBS-43'ün ilk dilimi. Bu dilim iki şeyi sağlar:
  - sınırlı ya da sayfalı servisten gelen eksik sonuç "tam veri" sayılmaz;
  - OGC API Features servisinin desteklediği bölümler ve süzgeç dili açıkça okunur ve gösterilir.
- **Bağlam belgesi:** ADR 0208 §10 (Servisten veri al), 0228 (web'e ara).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - canlı sağlayıcı: servisin üstünde sorgu ve düzenleme, kopya almadan ayrı;
  - expires/ETag ile yenileme;
  - belirtecin yenilenmesi ve kesilen ağın sürdürülmesi;
  - CQL2 JSON ve queryables'tan alan listesi;
  - WFS'nin kendi sınırı: WFS 1.x eşleşen sayıyı bildirmez, sayfalamaz; sayfadan kısa yanıtın eksik olup olmadığı bilinemez;
  - açık OGC API yayını (P2).

## Bağlam

Servisten veri al'ın alma döngüsü (`feed::Taking`) şu durumlarda durup sonucu tam veri sayıyordu:

- **WFS 2.0:** sunucu sayfayı kendi sınırında kestiğinde, eşleşen sayıyı bildirse bile döngü bitiyordu. Örneğin sunucu 2000
  istenene 500 veriyorsa 1800 nesneden yalnız 500'ü geliyordu.
- **OGC API:** sonraki sayfa bağlantısı olmayan yanıtta, eşleşen sayı daha büyük olsa da sonuç sessizce kısalıyordu.
- **Sayfa sınırı:** ev sahibinin 1000 sayfalık sınırına gelindiğinde de aynısı oluyordu.

Süzgeçte ayrı bir sorun vardı. OGC API Features'ta süzgeç Bölüm 3 ve CQL2'dir. Bunları bilmeyen sunucu bilinmeyen
parametreyi yok sayar ve bütün nesneleri verir. KentOS süzgeci her servise gönderiyordu.

## Karar

### 1. Bitiş

Alma işi nasıl bittiğini söyler (`feed::Ending`):

- **`complete`:** servis başka nesnesi olmadığını söyledi.
- **`capped`:** en çok nesne sınırına gelindi ve servisin daha fazlası vardı ya da sayfa kesildi.
- **`short`:** servis, eşleştiğini söylediği sayıya varmadan sayfaları bitirdi.
- **`pages`:** 1000 sayfa sınırına gelindi (`MAX_PAGES`; sınır artık alma işinin kendisinde).

Sayfaların sürme kuralı servis türüne göre değişir:

- **WFS:**
  - eşleşen sayı biliniyorsa alınan o sayıya varana dek sürer; sunucunun kestiği sayfa da sürer;
  - bilinmiyorsa WFS 2.0 boş sayfaya dek sorulur (sonda bir istek fazla);
  - WFS 1.x sayfalamaz, kısa sayfada durur.
- **OGC API:** `next` bağlantısı varken sürer.
- **ArcGIS:** `exceededTransferLimit` varken sürer.

`taken_words` bitişi söyler. Masaüstü tam olmayan almayı uyarı olarak duyurur. Python `Taken.ending`'i verir, `capped`
korunur.

### 2. Uygunluk profili

- OGC API servisine bağlanınca teklif hazır olduktan sonra uygunluk sayfası sorulur. Adresi şöyle bulunur:
  - açılış sayfasının `conformance` bağlantısı;
  - yoksa `collections` yolunun yanı;
  - adresin kendi sorgusu (ör. bir anahtar) korunur.
- İstek isteğe bağlıdır. Başarısız olursa profil "okunamadı" kalır (`FeedConnecting::conformance_request`,
  `read_conformance`).
- Sınıflar ailesiyle (`/spec/` sonrası) ve adıyla (`/conf/` sonrası) okunur; sürüm karşılaştırılmaz (1.0 ve taslakları
  aynı sınıftır) (`profile::FeedProfile`). Okunan bölümler:
  - Bölüm 1: core, GeoJSON;
  - Bölüm 2: crs;
  - Bölüm 3: filter, features-filter, queryables;
  - CQL2: metin, JSON, basic, uzamsal, zamansal;
  - Bölüm 4: create-replace-delete. Taslaktır; okunur ama kullanılmaz, profilde öyle yazılır.
- CQL2 metin süzgeci (`filter`, `filter-lang=cql2-text`) yalnız Bölüm 3'ün `features-filter`'ını ve CQL2'nin
  `cql2-text`'ini bildiren servise gider.
- Başka servise süzgeçli veri kaynağı kurulmaz (`feed`'in reddi). Ret nedenini söyler: hangi bölüm eksik, ya da sayfa
  okunamadı.
- Masaüstünün penceresinde "Uygunluk" satırı profili gösterir. Süzgeç alanının altında desteğin olmadığı söylenir.
- Python'da teklifin `profile`'ı vardır.

### 3. Pencere

Servisten veri al'ın gövdesi kayar. Alçak pencerede özet ve düğmeler görünür kalır; önceden 1100×650'de düğmeler
pencerenin dışına düşüyordu.

## Doğrulama

- `scripts/fixtures/feed_paging_cases.py` kuralı KentOS kodu olmadan yazar ve `fixtures/services/v1/paging.json`'ı üretir:
  - 12 uygunluk sayfası: 1.0 ve taslak sınıflar, Bölüm 3'süz, CQL2 metinsiz, Bölüm 4, başka belgeler, boş, sayfa olmayan,
    JSON olmayan, başarısız istek;
  - 20 alma, sahte bir sunucuyla: WFS 2.0 ve 1.1, OGC API, ArcGIS; sunucunun sayfa sınırı, eşleşen sayı, `next`'siz
    yanıt, en çok nesne, sayfa sınırı.
- `crates/shared/services/tests/all/paging.rs` aynı sunucuyu KentOS'un istemcisine karşı oynatır: istek sayısı, alınan ve
  gelen nesneler, eşleşen sayı ve bitiş başvuruyla aynıdır.
- `python/tests/test_services.py` yerel bir HTTP sunucusunda uçtan uca dener:
  - süzgeçsiz OGC API'de profil ve süzgecin reddi;
  - eşleşenden az veren servisin `short`'u;
  - süzgeçli serviste `filter` ve `filter-lang=cql2-text`;
  - uygunluk sayfası olmayan serviste süzgecin reddi.
- Masaüstü resmi: `servis-ogc-uygunluk`, çevrimdışı hazır yanıtlarla (`service_scenes.rs`).
