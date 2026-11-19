# Claude test kaydı

KentOS'u gerçek pencerede (macOS, gerçek fare ve klavye) kullanırken bulunan ve düzeltilen hatalar. Her kayıt şunları
yazar:

- ne görüldü;
- nedeni;
- ne değişti;
- nasıl doğrulandı.

## 1. Sahneden seç, başka katmandaki alanı almıyordu (19 Kasım 2026)

**Bildiren:** sahip. Sözü: “sahneden araç seçip processing tool ile kenar uzunluklarını yazdırma aracı çalışmıyor, sahnedeki
poligonu seçemiyoruz, seçmeye uygun”.

**Gerçek pencerede yeniden üretildi:**

1. KentOS CAD › Yeni proje › CBS (Ankara, TUREF / TM33).
2. Düzenle › Kapalı alan ile dört köşeli bir alan çizildi.
3. Öznitelikler'de alanın katmanı Kadastro / Parsel sınırı yapıldı. Etkin katman Taslak kaldı. Oto. seçim kapalıydı; bu,
   CBS projesinin kendi ayarı.
4. İşlemler › Kenar uzunluklarını yaz › Sahneden seç ile alanın içine ve kenarına tıklandı:
   - hiçbir şey alınmadı, satır “(0 seçili)” kaldı;
   - hiçbir ileti çıkmadı;
   - imleç alanın üstüne gelince vurgu da olmadı.

**Neden:** `PickObjects::takes`, Otomatik seçim kapalıyken seçen adımlar gibi yalnız etkin katmanın nesnelerini alıyordu
(ADR 0622 §5). Oysa pencerenin alanı nesneyi düzenlemez, okur. Alanın kendi kapsamları (Görünen, Tümü, Katman) da her
katmandan alır.

**Düzeltme:**

- `crates/native/interaction/src/viewer_tools/pick_objects.rs`:
  - Alanın nesneleri her görünen katmandan alınır; seçim süzgeci geçerli kalır.
  - Pencerenin üzerinde çalışacağı seçim için ayrı bir kip var: `PickObjects::selection` (Çizimden seç). Bu kip etkin
    katmanla sınırlı kalır.
  - Hiçbir şey almayan tıklama, imlecin altında bir nesne varsa nedenini söyler. Nedenler: seçim süzgeci, alanın almadığı
    tür, formun katmanı, Otomatik seçim'in katmanı.
- `crates/native/interaction/src/viewer_tools/selectable.rs`:
  - kutu için `in_box_read`;
  - başka katman iletisi tek yerde (`other_layer_words`).
- `apps/desktop/src/shell/window_pick.rs`: Çizimden seç `PickObjects::selection` kullanır.
- Belgeler:
  - ADR 0088 güncellemesi;
  - ADR 0622 §5'teki okuyan adımlar listesi;
  - kılavuz §0;
  - TODOS.md §0.2 (web).

**Doğrulama:**

- `cargo test -p kentos-interaction --test all`: 994 test geçti, 3'ü yeni.
- `cargo test -p kentos-desktop --lib -- input_objects room_modules the_start_vertex_is_picked`: 13 test geçti, 1'i
  yeni.
- Gerçek pencerede:
  - Parsel sınırı'ndaki alan, imleç üstüne gelince vurgulandı; tıklayınca alındı (“1 seçili”).
  - Pencere “1 kapalı alan; seçili nesneler” dedi.
  - Çalıştır “1 nesneye 4 kenar uzunluğu yazıldı.” dedi; üst kenarda “306.505” okundu.
- Resimler `.run/gui-evidence/sahneden-sec-katman/` altında:
  - `once-0-secili.png`: düzeltmeden önce;
  - `sonra-vurgu.png`, `sonra-1-secili.png`, `sonra-calistir.png`, `sonra-yazi.png`: düzeltmeden sonra.

## Bulunan, henüz düzeltilmeyen

**Türkçe Mac klavyesinde “@” yazılamıyor; göreli koordinat girilemiyor.**

- Türkçe Q (PC) düzeninde “@” Option+Q ile yazılır. KentOS Option+Q'yu Alt+Q, yani Menülerde ara sayıyor.
- Mimari projede Duvar aracındayken “@12,0” yazıldı. Menü araması açıldı ve “12,0” arama kutusuna gitti.
- Mimari kullanım denetimi sahibin isteğiyle durdurulduğu için bu düzeltilmedi.
