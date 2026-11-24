# ADR 9215: DWG açılışında köprü metni 2,3 kat hızlı yazılıyor

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-21 (ön adım).
- **İstek:** sahibin 23 Kasım isteği: “NCZ ve DWG/DXF içe aktarımını kusursuz hâle getir”. Toplu ölçümde (ADR 9212 §6)
  SİVAS AVM DWG'si (32 MB, 601 619 nesne) 24 saniyede açılıyordu.
- **Sözleşmeler:** değişmedi. Köprü metni bayt bayt aynı.

## §1 Ölçüm

**Açılışın aşamaları.** Kodek sürümde, simgeli bir sınama ikilisiyle ayrı ayrı zamanlandı. Geçici sonda commit'lenmedi.

| Aşama | Süre |
|---|---|
| DWG'nin çözülmesi (acadrust) | 3,9 s |
| Okunmayan nesnelerin atılması | 0,18 s, 486 531 nesne |
| Köprünün DXF metni olarak yazılması (75 MB) | **11,3 s** |
| DXF'in okunması (601 619 nesne) | 1,7 s; açılış iki kez okur: önce birim, sonra ölçek |

**Profil** (`perf`, kodeğin sınama ikilisi): zamanın %68'i `CadDocument::extension_dictionary_handle` içinde.

- Kodek bir sahibin uzantı sözlüğünü kendi dizininde bulamayınca bütün nesneleri tek tek geziyor. Her arama yaklaşık
  3 ms sürüyor.
- Yazıcı bunu her tablo kaydında soruyor (`write_common_table_data`), blok kayıtları dahil. Çizimde 16 027 blok kaydı
  var.
- Atılan nesneler haritadan çıkıyor, ama harita kapasitesini koruyor: 21 280 nesne için 904 536 yer. Harita gezintisi
  boş yerleri de dolaşıyor.

## §2 Karar

Okunmayan nesneler atıldıktan sonra nesne haritası küçültülür (`objects.shrink_to_fit()`, `kentos_dwg::bridge`).

- **Sonuç:** kapasite 904 536 → 28 672, yazma 11,3 → **4,9 s**. Küçültme 0,08 s sürüyor.
- **Doğruluk:** küçültmeli ve küçültmesiz köprü metni bayt bayt aynı (78 939 653 bayt, `cmp`). Kodek nesneleri tanıtıcı
  sırasıyla yazıyor, harita sırasıyla değil.
- **Beklenen açılış kazancı** yaklaşık 6,4 s (ölçülen 22,9 s'lik iş parçacığından). Masaüstündeki uçtan uca süre bir
  sonraki release ölçümünde yazılacak.

## §3 Kalan ve sahibe sorulacak

- **Kalan 4,9 s'nin çoğu aynı aramada.** 21 bin nesne hâlâ her blok kaydında geziliyor.
  - Asıl düzeltme kodekte: sahip → uzantı sözlüğü dizini bir kez kurulmalı.
  - Bağımlılık sahibin seçimiyle yazarının deposundan, sabit bir sürümde. Bu yüzden iki yol var: yazara bir düzeltme
    önermek ya da çatallı bir kopyaya geçmek. Karar sahibin; bu işte kodeğe dokunulmadı.
  - **Sahibin kararı (24 Kasım): yazara öner.** Yazarın kurallarıyla açıldı: issue hakanaktt/acadrust#651, PR #652.
    - Önce kırmızı sınama commit'i: süre oranı 14,2.
    - Sonra düzeltme: yazıcı sözlüklerin sahiplerini yazma başında bir kez topluyor. Sınamanın oranı 4,1.
    - Aynı DWG yazarın ana dalında, okunmayanlar atılmadan (507 811 nesne, 447 MB DXF): 112,3 s → 2,95 s.
    - Kodeğin bütün sınamaları geçti: 3 271.
    - Birleşince sabit sürüm yükseltilecek. Ana dal bizim sürümümüzden 552 commit ileride; yükseltme ayrı bir iş.
- **Köprünün kendisi** (çöz → DXF metni yaz → oku) RM-21'in doğrudan eşlemesiyle kalkacak.
- **İkinci DXF okuması** (1,7 s) çizim ölçeği 1'den farklıyken gerekli: stillerin kâğıt boyları ölçekle okunuyor.

## Uygulama

`crates/native/dwg/src/lib.rs` (`bridge`).
