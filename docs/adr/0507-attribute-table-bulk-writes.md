# ADR 0507: Öznitelik tablosunda toplu yazma: Alan hesapla ve Yapıştır

- **Durum:** kabul edildi (2026-10-18). `plan/gis.md` CBS-20'nin ilk dilimi:
  - toplu yapıştırma ve alan hesabı;
  - önizleme ve hedefin tür denetimi;
  - bütün ya da hiç yazma, satır satır sorun raporu;
  - kapsamın açık sayıları.
- **Bağlam belgesi:** ADR 0199 (katman alanları, öznitelik tablosu), 0066 (`cad.entities.set`), 0422, 0423, 0438
  (alt türler, değer bileşimleri, kayıt kuralları, tekil alan), 0084 (Öznitelik hesapla aracı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - sıralamanın, süzgecin ve toplamların kaynağa itilmesi ve imleçli sayfalar (CBS-04'ün sorguyu kaynağa itmesiyle);
  - gizli sütunun kopyalama ve dışa aktarmada süzülmesi (CBS-38'in alan yetkisiyle);
  - grafik ve istatistiğin tablonun süzgecini kullanması;
  - tablonun hücresinden Ctrl+V ve sütun başlığının menüsü.

## Bağlam

Öznitelik tablosu hücre hücre düzenleniyordu. Elektronik tablodan yapıştırma ve “bu alana şu ifadeyi yaz” yoktu.
İşlemler'in Öznitelik hesapla aracı vardı, ama alanın tür kurallarını yazmadan önce göstermiyor ve sorunu satır satır
söylemiyordu.

Plan ister:

- bin hücrelik yapıştırmada tek yanlış değer ya bütün işlemi durdurmalı ya da açık satır raporu vermeli;
- sessiz kısmi başarı olmamalı;
- kapsam (seçili, gösterilen, bütün) sayılarıyla ayrılmalı.

## Karar

### 1. `cad.fields.write`

- Anlamsal ürün komutu, tek geri alma adımı. Adım “Alan hesapla: alan” ya da “Yapıştır”.
- **`calculate`:** `expression`'ın her nesnedeki değeri `field`'a.
  - Dil İfadeyle seç'inki: öznitelikler adlarıyla, geometri değerleri projenin çizim biriminde, `$sıra` nesnelerin
    sırası.
  - `uids` boşsa katmanın bütün nesneleri.
  - Boş sonuçta eski değer kalır (`empty: keep`) ya da kalkar (`clear`).
  - `$ölçek` reddedilir: değer çizimin ölçeğine göre değişmemeli.
- **`values`:** her satırın değerleri kendi nesnesine; null değeri kaldırır.
- **Denetim:** her değer `cad.entities.set`'in yoluyla, nesne nesne:
  - alanın türü ve kuralları, kanonik metne çevrilir (`checks::field_values`);
  - zorunlu alan;
  - alt tür, değer bileşimi, kayıt kuralları;
  - tekil alanlar, yazılanlar birlikte. İki kaydın değerleri yer değiştirebilir; çakışan değer onu taşıyan
    nesneleri dışarı alır.
- Alanı olan katmanda yalnız alanların adlarıyla yazılır (`field_not_found`); alansız katmanda öznitelik adıyla.
- **`policy`:**
  - `all` (varsayılan): uymayan bir değer bütün yazmayı reddeder (`invalid_values`, ilk sorunla);
  - `valid`: uyanlar yazılır, öbürleri `values_skipped` ile söylenir.
- Bir değeri gösteren etiket yeni değeri gösterir (Öznitelik hesapla gibi).
- **`plan`:** değişecekleri (nesne, alan, eski ve yeni değer) ve sorunları (nesne, alan, değer, kod, ileti) ilk 200
  kadar listeler, hepsini sayar. `policy: all`'da da reddetmez; pencerenin önizlemesi budur.
- Python (`kentos.cad.fields.write`) ve MCP (yıkıcı listede) katalogdan gelir.

### 2. Masaüstü

- Öznitelik tablosunun çubuğunda **Alan hesapla…** ve **Yapıştır**.
- **Alan hesapla penceresi:**
  - Alan ve İfade (ε: İfade oluşturucu, katmanın alanları ve nesneleriyle);
  - Nesneler: Seçililer (n), Tabloda gösterilenler (n), Katmanın bütün nesneleri (n);
  - “Sonuç boşsa alanın değerini kaldır”;
  - **Önizle** komutun planını gösterir: kaç nesnede değişecek, aynı kalacak, boş kalacak, uymayacak; ilk değişiklikler
    ve sorunlar satır numarasıyla;
  - **Yaz** yalnız uymayan yokken açıktır. **Uyanları yaz** uymayan ve değişen varken görünür (`policy: valid`).
- **Yapıştır:**
  - panonun hücrelerini (satırlar ve sekmeler) ilk seçili satırdan başlayarak tablonun gösterilen satırlarına,
    alan sütunlarının sırasıyla yazar;
  - ilk satır alan adlarıysa başlık sayılır ve sütunlar adlarıyla eşlenir;
  - boş hücre değeri kaldırır; tablonun sonunu aşan satırlar söylenir;
  - pencere önizlemeyle açılır.
- Yazınca pencere kapanır, sonuç bildirimle söylenir.

## Doğrulama

- `fixtures/commands/v1/cad.fields.write.json`: 32 ortak durum, KentOS kodu olmadan `fields_write_cases.py`'den.
  - İfadenin değeri dilin bağımsız başvurusundan (`expression_language.py`), alanın denetimi ve kanonik metni alanların
    bağımsız başvurusundan (`layer_field_cases.py`'nin `check`'i).
  - Kapsananlar: hesap, boş sonuç, etiket, ölçek dışı ondalık, zorunlu alan, uzunluk, alansız katman, yapıştırma,
    kanonik metin, tekil alanın yer değiştirmesi ve çakışması, `policy: valid`, planlar, geri alma, retler.
- Yetenek testi: her ret ve uyarı kodu bir durumda.
- Masaüstü (`features::tests`): Alan hesapla önizler, tek adımda yazar, geri alınır; başlıklı yapıştırmada uymayan satır
  söylenir ve Uyanları yaz uyanı yazar. Resimler `.run/shots/alan-hesapla-*`.
