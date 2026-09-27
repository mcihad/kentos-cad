# ADR 0072: Katman silme: iki platformda tek adım, bulutta sunucu koruması

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.8, §7, §13.1, §21.3; ADR 0003 (işlem ve geri alma), 0020 (ortak belge fixture'ları), 0026 (bulutta nesne kimliği), 0040 (masaüstü eşitlemesi)
- **Web ajanının 12. görevi (27 Eylül):** web tarafı `1fd4120`; ortak durumlar `fixtures/document-ops/v1/layer-remove.json` (on senaryo).

## Bağlam

- Web'in Katmanlar panelinde katman ya da grup silinemiyordu; masaüstünde de.
- Katman ağacı bulutta projenin bilgisidir (`@project`). Nesneler ayrı satırlardır.
- `project.changes` bir nesnenin güncel katmanını komutun yeni ağacında arıyordu. Silinen katmandaki nesnenin silinmesi bu yüzden "‘…’ katmanı projede yok." diye reddediliyordu.
- Sunucu, içinde nesne kalan bir katmanın ağaçtan düşmesini de engellemiyordu. Başkasının o arada eklediği nesne, ağaçta olmayan bir katmanda kalabilirdi.

## Karar

### Belge (web `CadDocument.removeLayer`, masaüstü `kentos_domain::Document::remove_layer`)

- **Tek adım:** katman ya da grup, altındaki her şey ve üzerlerindeki nesnelerle birlikte silinir. Adım "Katman sil"dir; açık bir işlem ya da grup varsa ona katılır.
- **Geri alma:** düğümü yerine koyar (üst grubu ve sırası; alt düğümleri, bayrakları ve stiliyle). Sonra nesneleri yuvalarına ve kalıcı kimlikleriyle getirir.
- **Geçmiş işlemleri:** `layerRemove` ve tersi `layerAdd`. Masaüstünde `Op::LayerRemove` ve `Op::LayerAdd`; düğüm ve yeri `LayerPlace`'te tutulur.
- **Retler:** hiçbir şey değişmez; bu sırayla denetlenir:
  1. çizimin son katmanı, ya da bütün katmanları içeren grup;
  2. etkin katman, ya da onu içeren grup;
  3. kilitli düğüm (kendisi ya da üstündeki grup);
  4. kilitli bir katman içeren grup.
- **Ret metinleri** web ajanının raporundaki gibidir ("“X” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin." …). `layer_removal_refused` / `layerRemovalRefused` aynı metni sorudan önce verir.
- **Başka düzenleyicinin ağacı:** gelen ağaç, ağaç işlemi tutan geri al/yinele adımlarını düşürür. Silinen düğümün kayıtlı yeri yeni ağaca uymayabilir. Masaüstünde bu `apply_external`'dadır.
- **Ortak durumlar:** `removeLayer` işlemi, `catch` alanıyla ret. İki belge de on senaryoyu geçer.

### Arayüz (web LayersPanel, masaüstü `layering.rs`)

- **Menü:** "Sil" satır menüsünün son öğesidir; ayraçtan sonra gelir, çöp kutusu simgesi vardır. Her zaman açıktır: silinemeyen düğüm nedenini uyarı olarak söyler.
- **Soru:** nesneli katman ya da grup için sorulur.
  - Başlık: "Katmanı sil" ya da "Grubu sil".
  - İleti: "“X” katmanı üzerindeki N nesneyle birlikte silinsin mi?" / "“X” grubu, içindeki K katman ve N nesneyle birlikte silinsin mi?"
  - Açıklama: "Geri al (Ctrl+Z) katmanı nesneleriyle geri getirir."
  - Düğmeler: Vazgeç ve kırmızı Sil. Yanıt geç gelirse ret yeniden denetlenir; düğüm gitmişse hiçbir şey olmaz.
- **İleti:** "“X” katmanı ve üzerindeki N nesne silindi." ve öbür dört biçim.
- **Delete tuşu:** web'de odaktaki ağaç satırında da çalışır. Masaüstünde ağacın klavyesi yok, bu yüzden Delete çizimdeki seçimi silmeye devam eder (TODOS.md `UI-11`: ağacın klavyesi).
- **Çizim:** web silinen katmanın GPU tamponlarını bırakır. Masaüstü sahneyi her düzenlemede belgenin güncel ağacından kurar; kalan tampon olmaz.

### Bulut

- **Gönderim sırası** (web `leavingObjects`, masaüstü `ProjectSync::leaving`):
  - Ağaç bir düğümü düşürdüğünde önce silmeler gider. Sunucunun düşen katmanda tuttuğu nesnelerin değişiklikleri de önce gider.
  - Ağaç bunların sonuncusuyla aynı komutta gider. Yeni nesneler ağaçla ya da sonra gider.
  - En çok 2000 değişiklikte hepsi tek komuttur. Daha çoğunda önceki komutlar ağaçsız ve `@project`'siz gider.
- **Sunucu** (`changes.rs`):
  - Silinen ya da değişen nesnenin güncel katmanı yeni ağaçta yoksa ağacın eski hâlinde aranır: orada var olmalı ve kilitli olmamalıdır.
  - Yeni ağaç katman düşürüyorsa, komutun nesneleri yazıldıktan sonra aynı işlemde bakılır. Düşen katmanda hâlâ nesne varsa `@project` çakışması döner ve komutun hiçbir şeyi yazılmaz. İleti: "“X” katmanında hâlâ nesne var (başka biri eklemiş ya da taşımış olabilir); katman silinmedi. Sunucudaki hâli ile sizinkini karşılaştırın."
  - Aynı komutta düşen katmandan başka katmana taşınan nesne katmanın gitmesine izin verir.
- **`project.edit` izni olmayan:** silmeler gider, ağaç cihazda kalır (var olan bildirimle). Sunucuda katman boş kalır.

## Bilinen sınırlar

- **@project çakışmasında "benimkini koru":** diğer kullanıcının nesnesi katmanda kaldıkça ağacı yeniden gönderir ve aynı çakışmayı alır. "Sunucudakini al" katmanı geri getirir; silmeler yine de gider.
- **Kaldırılan katmana gelen nesne:** başka istemcinin buradan silinen bir katmana koyduğu nesne gelişte okunamaz. Koruma katmanı tutarsa o nesne proje yeniden açılana dek burada görünmez.
- **2000'den çok değişiklik:** aynı ağacın eklediği katmana taşıma, ağaçtan önce giderse reddedilebilir.

## Doğrulama

- `fixtures/document-ops/v1/layer-remove.json`: web (`documentOps.test.ts`) ve masaüstü (`kentos-domain` fixtures).
- Masaüstü:
  - `layering` testi: ret metinleri, soru, Vazgeç, silme, geri alma, soru sorulmadan silinen boş katman;
  - `kentos-cloud`'da `a_removed_layer_goes_after_its_objects`: 2002 + örnek nesneli katman ve yeni katmanın nesnesi. İlk komutta 2000 silme var, ağaç yok; ikincisinde kalan silmeler, yeni nesne ve ağaç.
- Sunucu: `changes::a_layer_goes_with_its_objects_or_not_at_all`, gerçek PostGIS'te (`KENTOS_TEST_DB=required`). Senaryolar:
  - "Bina" iki nesnesiyle tek komutta silinir;
  - başkasının nesnesi kalan "Çizim"in silinmesi `@project` çakışması olur ve hiçbir şey yazılmaz;
  - aynı komutta nesnesi taşınınca "Çizim" gider.
- Web: `pnpm test` (eşitleme testleri, sahte sunucu korumayı taklit eder), smoke (Sil ve sorusu, geri alma, boş satırda Delete, etkin katmanın reddi).
