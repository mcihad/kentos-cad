# ADR 0651: Nesnenin kaynağı bilgi kartında, süzgeç, sağlayıcının hatası ve bulut taslağı Kaynak durumu'nda

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-40'ın üçüncü dilimi. Kapsamı ben belirledim.
- **Kalan işler:** ADR 0543 ve 0546'nın “Kalan”ı: bilgi kartında kaynak, süzgeç ve bulut taslağı, sağlayıcı hataları.
- **Bağlam belgesi:**
  - ADR 0503 (bağlı katman), 0538 ve 0553 (bağın tabanı), 0543 (Kaynak durumu ve `next`), 0546 (Öznitelik tablosunun kaynak
    satırı);
  - ADR 0211 (katman süzgeci), 0225 (bilgi kartı), 0505 (PostGIS sağlayıcısı), 0043 (masaüstünün bulut taslağı).
- **Web:** ertelendi (ADR 0228). TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Bağımlılık:** yeni kitaplık yok.
- **Sonraya kalan:**
  - iki referans kullanıcı grubuyla (CBS uzmanı ve form kullanan belediye personeli) kabul: tamamlama oranı, süre, yanlış hedef
    ve kurtarma ihtiyacı kaydedilerek; bu geliştiricinin değil sahibin ve kullanıcıların işidir, resim kabul sayılmaz;
  - Kaynağın tümü tablosunda okunamayan sayfanın yeniden denenmesi;
  - bilgi kartında nesnenin tablodaki satır numarası (`fid`).

## Bağlam

Bağlı katmanın durumu (ADR 0543) katman başınadır: kaç nesne yazılmamış, dosya değişmiş mi, sıradaki iş ne. Bir nesneye bakan
kullanıcı o nesnenin kaynağa yazılıp yazılmadığını görmüyordu. Kaynak okunamadığında yalnız “Yerinde yok” ya da “Tablo
okunamadı” yazıyordu; sağlayıcının nedeni (izin yok, tablo yok, sunucu cevap vermedi) atılıyordu. Katmanın süzgeci olduğunda
tabloda görünenle katmanın nesne sayısı uyuşmuyordu ve bunun nedeni söylenmiyordu. Bulut projesinde cihazın taslağında bekleyen
değişiklik yalnız durum çubuğunun kayıt hücresindeydi.

## Karar

### §1 Nesnelerin kaynağı: `cad.sources.describe`'ın `uids`'i

`uids` verilince cevap, bağlı katmanların nesneleri için `objects`'tir (sorulan sırayla): bağın kimliği, tablo, bağın anahtarı
ve nesnenin o anahtardaki değeri, bağın yazılabilirliği ve durum:

- `matched`: tabloyla son eşleştiği gibi;
- `edited`: o zamandan beri katmanda değişti, sıradaki yazma tabloya koyar;
- `added`: katmana sonradan eklendi, tabloda satırı yok;
- `unknown`: katman değişti ve bağın tabanı yok (katmanda `MAX_BASE_ROWS`'tan çok nesne var ya da taban eski bir bağdan kaldı).

Tablo okunmaz: katmanın özeti bağınkiyle aynıysa her nesne `matched`, değilse bağın tabanı (ADR 0538) nesnenin özetini söyler; taban yoksa ve katman değişmişse `unknown`.
Bağsız katmanın nesnesi cevapta yoktur. Olmayan kimlik `entity_not_found`, `path` ile birlikte `invalid_input`'tur.

Masaüstünde Bilgi kartı bağlı katmanın nesnesinde başlığın altında bir satır gösterir: “Kaynak: tablo · anahtar değer —
kaynakla aynı / değişti, kaynağa yazılmadı / yeni, kaynakta yok”, salt okunurda “(salt okunur)”. Yazılmayı bekleyen satır uyarı
renginde. Satır çizimin sürümüyle önbellektedir: taban her değişiklikte bir kez okunur, kartın her karesinde değil.

### §2 Süzgeç ve bulut taslağı

Bağın durumu `filtered`'ı katmanın süzgecinden geçen nesne sayısıyla verir (süzgeç yoksa yok). Kaynak durumu'nun Kapsam
sütununda “· süzgeç n”, ayrıntısında “Katmanın süzgeci n nesneden m tanesini gösteriyor; yazma ve okuma bütün nesnelerle
yapılır.” Bulut projesinde cihazın taslağında gönderilmemiş değişiklik varsa pencerenin özetinde sayısıyla söylenir.

### §3 Sağlayıcının hatası

Bağın durumu `error`'ı tablonun neden okunmadığını sağlayıcının sözüyle verir: dosya yerinde değilse “… yerinde değil: taşınmış
ya da silinmiş olabilir; Kaynak ▸ Yeniden bağla ile yerini gösterin.”, dosyada tablo yoksa okuyucunun sözü, veritabanı cevap
vermediyse ya da reddettiyse sağlayıcının Türkçe iletisi. Kaynak durumu ayrıntıda “Neden: …” der. Cevap vermeyen veritabanının
satırında düğme “Yeniden dene”dir: durumları yeniden sorar; dosyanın düğmesi Yeniden bağla… olarak kalır.

## Doğrulama

- `cad.sources.describe`'ın ortak durumları 29'dan 50'ye (`scripts/fixtures/bound_sources_cases.py`, KentOS kodu olmadan): bağdan
  hemen sonra hepsi aynı, sorulan sıra, değişen, geri alınan, eski değerine dönen, taşınan nokta, eklenen nokta, bağsız katman,
  iki bağ, anahtarlı bağ ve anahtarı değişen, tabanı olmayan değişmiş salt okunur katman (`unknown`), silinen nesne, kimlik olmayan
  metin, `path` ile birlikte, plan ve validate; üç süzgeç (koşul, liste, hiçbirini geçirmeyen); yerinde olmayan dosyanın ve
  dosyada olmayan tablonun nedeni. Durumun kuralı adımlardan çıkar: bağdan sonra hepsi aynı, sonraki adımın değiştirdiği değişmiş,
  eklediği yeni; geri alma ya da eski değer katmanı eski hâline döndürür.
- Masaüstü (`cargo test -p kentos-desktop --bin kentos-cad the_status_and_the_card_say_the_source`): süzgeçli katmanın sayısı,
  taşınmış dosyanın nedeni, değişen noktanın kart satırı, bağsız katmanın nesnesinde satır yok.
- Resimler `source_status_screens`: `.run/shots/gpkg-kaynak-durumu-*-suzgec-*`, `-bilgi-karti-*`.

## Ölçümler

Release, bu makinede (`cargo test --release -p kentos-native-application --test all object_source_timing -- --ignored
--nocapture`): `.run/perf/atlas-100k.gpkg`'nin 102 400 parseli bağlı bir katmanda, bir nesnenin kaynağı (beş denemenin en iyisi):

| Durum | İlk yazım | Bugünkü |
|---|---|---|
| değişmemiş katman | 203 ms | 1,3 ms |
| bir nesnesi değişmiş katman | 202 ms | 1,0 ms |

İlk yazım katmanın bütün özetini (`layer_digest`) ve tabanın bütün satırlarını her soruda hesaplıyordu. Tabanın satırları
kalıcı kimliğe göre sıralı olduğundan nesnenin satırı metnin içinde ikili aramayla bulunur ve yalnız bakılan satırlar çözülür
(`kentos_commands::base_row_of`, 17 satır); katmanın özeti yalnız tabanı olmayan bağda hesaplanır. Bilgi kartının satırı ayrıca
çizimin sürümüyle önbellektedir: aynı sürümde kartın her karesi önbellekten okur.
