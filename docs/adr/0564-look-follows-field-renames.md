# ADR 0564: Görünüş alanın adını izler

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-17'nin öbür yarısının ilk dilimi (“alan rename olduğunda stil bağı
  bozulmamalı”). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0414 (alanın kullanıldığı yerler ve yeniden adlandırmada izlenmesi, `kentos_forms::field_refs`), 0213
  (`cad.layers.renderer`, işleyicilerin JSON'u), 0090–0094 (stil sistemi, kitaplık), 0199 (katman alanları).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - bağlı kaynağın (GeoPackage, PostGIS) sütunu kaynakta yeniden adlandırılınca görünüşün izlemesi;
  - kitaplığın simgelerindeki veriye bağlı değerler: simge katmanlar arasında paylaşılır, bir katmanın alanıyla yeniden yazılamaz.
    Bugün kullanılan yer olarak sayılmaz;
  - aynı stilin yerel ve PostGIS katmanında aynı sınıfları ve lejantı verdiğinin kabulü (CBS-17'nin “Bağımlılık ve tamamlanma
    ölçütü”).

## Bağlam

Alanın adı ifadelerde geçer. ADR 0414 alanın kullanıldığı yerleri sayar (`cad.fields.usage`) ve yeniden adlandırmada
(`cad.layers.edit`'in `fields`'ının `renames`'i) aynı adımda yeni adı yazar. Sayılan yerler şunlar:
- formlar, listeler, ilişkiler;
- katmanın koşullu biçimi, süzgeci, etiketleri, zamanı, bilgi kartı ve eylemleri.

Katmanın görünüşü bu yerler arasında değildi. Kategorili görünüşün `[ada]`'sı, aralıklının `[alan]`'ı, kurallı görünüşün
süzgeçleri, tematik işleyicilerin ifadeleri (Sürekli renk, Orantılı sembol, İki değişkenli renk, Nokta yoğunluğu, Grafik …) ve
simge katmanlarının veriye bağlı değerleri (`{"expr": …}`) eski adda kalıyordu. Alanı yeniden adlandıran kullanıcı görünüşünün
sessizce bozulduğunu, sınıfların “Diğerleri”ne düştüğünü görüyordu.

## Karar

### 1. Görünüşün yerleri

`field_refs`'in katman yürüyüşü süzgeçten sonra, etiketten önce katmanın işleyicisini (`style.renderer`) de yürür. JSON'un her
nesnesinde şunlar ifade sayılır:
- `expr`, `exprX`, `exprY` anahtarlı metinler: işleyicinin kendi ifadeleri, tematik işleyicilerin alanları (`fields[].expr`),
  simge katmanlarının veriye bağlı değerleri;
- bir kuralın (bir `rules` ya da `children` dizisinin öğesinin) `filter`'ı.

Yürüyüş şu kurallarla işler:
- İç içe işleyiciler (Kümeleme'nin iç işleyicisi) aynı kuralla yürünür.
- Kitaplığın simgesi (`{"ref": …}`) yürünmez: projenin simgesidir, katmanlar paylaşır.
- Anahtarlar serde_json'un haritasının sırasıyla (sıralı) yürünür; bağımsız başvuru aynı sırayı kullanır.

Yerin Türkçe adı ““Bina” katmanı › Görünüş”tür. Kuralın içindeyse kuralın etiketi eklenir: “… › Görünüş › “Yüksek” kuralı”.
Yol `layers/<kimlik>/style/renderer/…`'dir.

### 2. Yeniden adlandırma

Yeniden adlandırma bu yerlerde de ADR 0414 §2'nin kuralıyla yazar:
- ifade sözcük sözcük yeniden yazılır;
- yeni ad tek sözcük değilse ya da anahtar sözcükse köşeli parantezle yazılır;
- metin içindeki aynı harfler değişmez.

Adım aynıdır (“Alanlar”), geri alma görünüşü de geri verir. Görünüş değişince çizim yeniden kurulur (katmanın görünüşü değişmiş gibi).

### 3. Komutlar

Komutların kendisi değişmez, yalnız yerleri genişler:
- `cad.fields.usage`'ın cevabı görünüşün yerlerini de verir;
- `cad.layers.edit`'in yeniden adlandırması onları da yazar.

Katalogdaki açıklamalar bunu söyler. Sözleşme, `.kcad` ve şema değişmez.

## Doğrulama

- `python3 scripts/fixtures/field_usage_cases.py --check`:
  - bağımsız başvuruya görünüşün yürüyüşü eklendi;
  - ortak çizimin Bina katmanının görünüşü kurallı: iki ifadeli süzgeç, iç içe kural, veriye bağlı kalınlık ve kitaplığın simgesi;
  - Malik katmanının görünüşü `[Malik adı]` ile kategorili;
  - `cad.fields.usage`'ın 20 ve `cad.layers.edit.renames`'in 20 durumu buna göre yeniden üretildi.
- `cargo test -p kentos-native-application --test all fixtures`: iki dosyanın durumları masaüstünün işleyicileriyle.
- `cargo test -p kentos-forms --lib field_refs`: çekirdeğin birim testleri.
