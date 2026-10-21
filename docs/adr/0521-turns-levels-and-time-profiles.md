# ADR 0521: Ağ analizinde dönüşler, seviyeler ve zaman profili

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-26'nın ilk dilimi (“dönüş yasağı/cezası, köprü-tünel seviye
  ayrımı, zamana bağlı maliyet”; erişilemeyen durağın nedeni ve eski grafla rota üretmeme).
- **Bağlam belgesi:** ADR 0209 (ağ analizi: tanım, graf, aramalar, araçlar, İşlemler, Python), 0224 (doğrusal
  referans), 0100 (ifade dili), 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - araç profilleri (yaya, kamyon: ağırlık, yükseklik ve genişlik sınırları) ve zamana bağlı kapanmanın tarih aralığı (yapıldı:
    [ADR 0567](0567-vehicle-profiles-and-dated-closures.md))
    (bugün profil her günü aynıdır);
  - dönüş katmanının çizimde kurulması (Dönüş çiz aracı); bugün dönüş, gelinen kenardan düğümün üstünden gidilen kenara
    çizilen çizgidir;
  - bölgesel ölçekte dış yönlendirici (Valhalla türü) bağlayıcısı; ölçülmeden seçilmez;
  - bağlı katmanın (GeoPackage, PostGIS) revizyonuyla grafın yenilenmesi CBS-12'nin sağlayıcısına bağlanınca.

## Bağlam

ADR 0209'un ağı her kavşakta her yöne serbest dönüşe izin veriyor, aynı yerde buluşan her kenarı bağlıyor ve süreyi günün
her saatinde aynı hızla hesaplıyordu. Gerçek yol verisinde üç şey bunu yanlış yapar: yasak ya da gecikmeli dönüşler (sola
dönüş yasağı, U dönüşü), birbirinin üstünden geçen ama bağlı olmayan yollar (köprü, tünel, viyadük) ve trafiğin saate göre
değişen hızı (sabah ve akşam yoğunluğu, gece kapanan yol). Plan, yasak dönüşün sonucu değiştirmesini, köprünün altındaki
yolun yanlış düğüme bağlanmamasını ve kapalı yolun eski grafla sessiz rota üretmemesini ister.

## Karar

### 1. Tanım ve dosya

Ağın tanımına (`NetworkDef`) iki isteğe bağlı alan, hızla maliyete bir alan eklenir; `.kcad` şema 87 (§6.4.8), yazıcı onu
yalnız bir ağ birini taşırken yazar, başka her proje eskisiyle bayt bayt aynıdır:

- `turns`: geri dönüş kuralı `uTurns` (`allowed` her yerde, `deadEnds` yalnız çıkmazda, `never` hiçbir yerde), üç ve daha
  çok parçanın buluştuğu kavşakta dönüşün açısına göre gecikmeler (`straight` gelinen yönden 30° içinde, `right`, `left`) ve
  geri dönüşün gecikmesi `back`, saniye, 0–3600; en çok dört dönüş katmanı (`layers`: katman, süzgeç, gecikme alanı).
- `levels`: kenarın başındaki, sonundaki ve isteğe bağlı içindeki seviye alanları; düğüm noktasının seviyesi başlangıç
  alanından.
- hızla maliyetin `periods`'ı: günün sırasıyla, örtüşmeyen en çok 24 dönem, her biri başlangıç ve bitiş dakikası ve ya
  hızların çarpanı (0–10; 0 kapalı) ya o dönemin hız alanı. Bütün gün kapalı olamaz.

Kurallar sözleşmenindir (`NetworkDef::problem`, `periods_problem`); bağımsız KCAD okuyucusu (`tools/kcad/kcad.py`) aynısını
uygular, iki okuyucu `fixtures/network/v1/rules.json`'un 94 durumunda aynı hükmü verir.

### 2. Dönüşler

Dönüş, düğüme bir yönde varan kenardan düğümden çıkan kenaradır. Yasaklıysa (dönüş katmanının alanı boş ya da katmanın
alanı yok; ya da geri dönüş kuralı izin vermiyor) arama o geçişi yapmaz; değilse süre maliyetlerine dönüş çizgisinin kendi
gecikmesi, yoksa kavşakta açısının gecikmesi eklenir (uzunluğa ve alandan maliyete eklenmez). Dönüş çizgisi gelinen
kenarın üstünde başlar, gidilen kenarın üstünde biter; ikisinin ortak düğümü yoksa Denetle “Bağlanmayan dönüş” der.
Dönüşler düğüm başına bir kez kodlanır (16 parçaya kadar), arama her adımda tabloya bakar; aramanın düğümü yayın kendisidir
(kenar ve yön), böylece aynı düğüme iki yönden varmak ayrı tutulur.

### 3. Seviyeler

Seviye alanı tam sayıdır (±1000). Kenarın ucu yalnız aynı seviyedeki uçlara ve düğüm noktalarına bağlanır; seviyesi
olmayan düğüm noktası her seviyeye bağlanır (bağlayıcı). Köşelerde bağlanan ağda kenarın içi iç seviyesiyle, iç alanı
yoksa uçları aynıysa onların seviyesiyle bağlanır; uçları farklı, iç alanı olmayan kenar (rampa) içinden hiçbir şeye
bağlanmaz. Denetle başka seviyedeki içlerin kesişimini “Bağlanmayan kesişim” saymaz. Okunamayan seviye 0 sayılır ve
Denetle'de “Okunamayan değer”dir.

### 4. Zaman profili

Soru bir kalkış saatiyle sorulur (Saat). Kenarın hızı günün her anında dönemin hızıdır; bir parça dönemin sonunu geçerse
sonraki dönemin hızıyla sürer, hızı sıfır olan (kapalı) kenarda açılana kadar beklenir; gün yinelenir. Böylece geç varmak
hiçbir zaman erken çıkmak olmaz (ilk giren ilk çıkar) ve en erken varış etiketli aramayla tam bulunur. Saatsiz soru ve
profilsiz ağ eskisi gibi kenarın kendi hızıyla hesaplanır. Hizmet alanının tesise doğru sorusunda saat varış saatidir.

### 5. Arayüz, komut, Python

- **Ağlar penceresi:** Dönüşler (kural, kavşak gecikmeleri, dönüş katmanları süzgeç ve gecikme alanıyla), Seviyeler (üç
  alan) ve hızla maliyetin Zaman profili (`07:00-09:30 0,5; 17:00-19:00 hiz_aksam` biçiminde); form kuralları
  `kentos_interaction::network::form`, sözleşmenin kurallarıyla.
- **Araçlar:** En kısa yol ve Hizmet alanı'nda zaman profili olan ağda **Saat (Z)**: her saat, 07:30, 08:00, 12:00,
  17:30, 18:00, 22:00 ya da yazılan saat; oturumun belleğinde.
- **İşlemler:** En yakın tesis, Maliyet matrisi ve Hizmet alanları'nda **Kalkış saati** (gelişmiş); profilsiz ağda
  saatin maliyeti değiştirmediği söylenir.
- **Komut:** `cad.network.define` tanımın yeni alanlarını taşır (katalog şeması sözleşmeden); var olan komut, yeni komut
  yoktur.
- **Python:** `kentos.network.Network.time` (`"07:30"` ya da `None`); MCP katalogdan.
- **Graf ve eski veri:** graf her analizde çizimden kurulur; ağın katmanlarının (dönüş katmanları dahil) değişikliği
  grafı yeniden kurdurur (masaüstünde değişiklik damgası ve üye nesneler), eski grafla soru sorulmaz.

## Doğrulama

- `scripts/fixtures/network_turns_cases.py --check`: dört sahne (tek yönlü sokaklı ızgara, kıvrık çıkmaz, yasak ve
  gecikmeli dönüş üç geri dönüş kuralıyla; köprüyle kesişim seviyeli ve seviyesiz, bağlayıcıyla; gün boyu hızı değişen ana
  yol ve yan yol, beklenen kapanma); 39 soru (rotalar ve sıraları, hizmet alanının çizgileri, en yakın tesis, maliyet
  matrisi, Denetle). Başvuru KentOS kodu olmadan yazılmıştır: grafı sorunun yerleriyle bütün kurar, dönüşleri aramanın
  kenarları yapar, zamanları kesirlerle yürür ve her cevabı bütün yolları deneyerek tek en ucuz olduğunu denetler.
  Çekirdek `cargo test -p kentos-geometry-core --test all network_turns` aynısını verir.
- `scripts/fixtures/network_rules_cases.py --check` (94 durum) ve `cargo test -p kentos-contracts --test all
  network_rules`; formun durumları `network_form_cases.py`, `cargo test -p kentos-interaction --lib form`.
- `.kcad` şema 87: `fixtures/kcad/v2/network-turns.kcad` bağımsız Python yazıcısıyla, on dört bozuk örnek;
  `scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- İşlemler'in varsayılanları `network_processing_cases.py --check`; Python `python/tests/test_network.py` (07:00–09:00
  yarı hızda 08:00'de yola çıkan rota iki kat sürer).
