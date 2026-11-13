# ADR 8056: Faz 2 — MCP v2 ve uygulama içi yapay zekâ ajanı

- **Durum:** uygulanıyor (13 Kasım 2026). Kaynak: `plan/argon-tavsiye.md` FAZ 2 (`ADIM 13` – `ADIM 15`) ve sahibin aynı gün
  eklediği `ADIM 15A` ("uygulama ayarlarına kişi kendi yapay zeka ajanını ayarlayabilmeli api key ile … varsayılan ajanıyla
  chat yapabilmeli … agentic olacak").
- **Sahibin kararı (13 Kasım, MCP koruyucusu):** çizimi değiştiren her MCP çağrısı önce `plan` ile çalışır (ne değişecek,
  kaç nesne); `execute` yalnız o plan görülmüşse ve çizim o andan beri değişmemişse kabul edilir.
- **Bağlam belgesi:** ADR 0133, 0134 (MCP sunucusu), ADR 0137 (masaüstünün ajan bağlantısı), ADR 0802 (modüllerin
  programları), ADR 0253 (komut kaydı, plan ve beklenen durum), ADR 8054 (Faz 1).

## §1. KentOS Yol'un bağlantısı ve modülün sunduğu komutlar (`ADIM 13`)

**İddianın denetimi.** Doğru: MCP'nin `PROGRAMS`'ı üç modülü biliyordu (`mimari`, `harita`, `cbs`); masaüstü KentOS
Yol'un bağlantısını (`masaustu-yol.sock`) zaten açıyordu, ajan ona adıyla bağlanamıyordu. İkinci iddia da doğru ve
ölçüldü: bir modülün programına bağlanan ajan, o modülün arayüzünde olmayan bir komutu da çalıştırabiliyordu
(`agents_answer` sunulma kuralına hiç bakmıyordu); Komut ara, komut satırı ve kısayollar bakıyor (`not_offered`).

**Karar.**
- MCP: `PROGRAMS` dört modül (`yol` → "KentOS Yol'da"); aracın açıklaması ve reddi dördünü sayar.
- Ajan bağlantısı: bir modülün programı, ajanın çalıştırdığı ürün komutunu (`run`, `call`, `start`'ın komutu,
  `replay`'in her satırı) yalnız sunuyorsa çalıştırır; sunmuyorsa `not_offered` ve hangi programlarda olduğu
  ("“cad.walls.create” KentOS Harita'da yok; KentOS Mimari'de var."). Bir günlük, satırlarından biri sunulmuyorsa
  hiçbir satırı çalışmadan reddedilir. kentos-cad (modülsüz) her komutu çalıştırır.
- **Kural iki kaynağın birleşimi:**
  1. Kaydın bölümü (`registry::discipline_of`, ADR 8054 §9'un beş dağıtıcısı): çekirdeğin komutu her modülün; bir
     disiplinin komutu kendi modülünün (mimari → KentOS Mimari, harita → Harita, CBS → GIS, yol → Yol).
  2. Arayüzün yolu: `catalog/v1/operations.json`'da bir masaüstü komutu ya da aracı `via` ile bir ürün komutundan
     geçiyorsa ve o modülün arayüzü o komutu sunuyorsa (`Catalog::module_has`), başka disiplinin komutu da o modülündür
     (Harita'nın Katman koordinat sistemi, CBS'nin `cad.layers.crs`'i).
- Neden yalnız `via` değil: o dosya düzeyleri için elle tutuluyor ve yollar eksik; `cad.polygon.create` yalnız Maske'nin
  yolu olarak geçiyor. Yalnız `via`'ya bakan bir kural temel çizim komutlarını reddedebilirdi. Bölüm tam (masaüstünün
  çalıştırdığı her komutun bir bölümü var, testle), `via` onu genişletir.
- Aks, kolon, merdiven, rampa, kiriş, korkuluk ve kolon mesafeleri kaydın çekirdek bölümünde duruyordu (yalnız
  derleme bayrağı mimarinindi); mimarinin bölümüne taşındı.

**Doğrulama:** `disciplines` (uygulama): her masaüstü komutunun bir bölümü var, örnek komutlar doğru bölümde;
`an_agent_on_a_module_program_runs_what_the_program_offers` (masaüstü): Harita'nın bağlantısında duvar reddedilir ve
gerekçesi Mimari'yi söyler, satırında duvar olan günlük bütünüyle reddedilir, çokgen, parsel ve katman koordinat
sistemi reddedilmez, kentos-cad duvarı reddetmez. MCP'nin testleri (`desktop.rs`, `tests/mcp.rs`) güncellendi; kural
gereği koşulmadı, derlemesi denetlendi.

## §2. MCP v2: kompakt yüzey, kademeli keşif ve "önce plan" koruyucusu (`ADIM 14`)

**İddianın denetimi.** Doğru ve ölçüldü: `tools/list` her katalog komutunu kendi aracı olarak veriyordu: **534 araç,
7 864 902 bayt** şema. Bir ajan bu kadarını her oturumda taşıyamaz, yanlış aracı seçer.

**Karar.**
- **Ortak araç katmanı `kentos_headless::agent_tools`.** Bir `Host` (çizimin adlı istekleri: `summary`, `layers`,
  `entities`, `entity`, `measure`, `selection`, `call`; ve modülün sunduğu, §1) üzerine kurulu araçlar: `drawing.summary`,
  `scene.layers|entities|entity|measure|selection`, `catalog.categories|search|describe`, kapılar `cad.run`, `arch.run`,
  `map.run`, `gis.run`, `road.run`, `command.batch` (`cad.changeset.apply`), `audit.list_providers|run|apply_fix`,
  `processing.list|describe|run|batch`. Aynı kod MCP'nin kendi çizimlerinde (`Session`), bağlı masaüstünde (bağlantı,
  `Desktop`) ve uygulama içi ajanda (§4) çalışır; MCP'nin testleri kural gereği koşulmadığı için kod test edilebilir
  yerde, headless'ta durur.
- **Sahibin koruyucusu.** Yazan komut önce `mode: "plan"` ile çağrılır: cevap ne ekleneceğini, değişeceğini,
  silineceğini ve `planId`'yi söyler, hiçbir şey yazmaz. `mode: "execute"` aynı komut ve girdiyle (anahtar sırasından
  bağımsız karşılaştırılır) ve o `planId` ile yazar; protokole `expected.plan` ve `expected.revision` gider: çizim
  plandan sonra değiştiyse `revision_conflict`, plan tutmuyorsa `plan_changed`, hiçbir şey yazılmaz. Plan bir kez
  kullanılır, en çok 64 plan tutulur. Plansız yazma `plan_required`. Yalnız okuyan komut (katalogda `effect` query ya da
  view, yıkıcı listede değil, dosya yazmıyor) hemen çalışır.
- **Kapılar.** Komut kendi bölümünün kapısından çalışır (`registry::discipline_of`); yanlış kapı `wrong_tool` ve doğru
  kapıyı söyler ("“cad.walls.create” mimarinin komutu: arch.run ile çağırın.").
- **Kademeli keşif.** `catalog.categories` bölümleri, aileleri ve sayıları; `catalog.search` sözcüklerden en çoğunu
  tutanı önce verir (Türkçe sözcük İngilizce kimlikli komutu bulur: "çokgen alan" → Kapalı alan oluştur); `catalog.describe`
  tek komutun tam şemasını ve üç örneğini. Modülün sunmadığı komut aramada ve kategorilerde görünmez.
- **Denetim ve İşlemler.** `*.check`/`*.audit` komutları (20) denetim; ailesinin `*.repair`'i onarımı
  (`cad.topology.check` → `cad.topology.repair`); `audit.apply_fix` yalnız onarım komutu alır ve koruyucudan geçer.
  `processing.*` İşlemler'in araçlarını listeler, tanımlar, çalıştırır (yazma, koruyucudan geçer).
- **Profiller.** Program (`kentos-mcp`) varsayılan **kompakt** profille açılır; `--profile full` ya da
  `KENTOS_MCP_PROFILE=full` eski düz listeyi verir. `Server::new()` tam profildir: ~200 test dosyası katalog komutlarını
  doğrudan ve plansız çağırıyor; kural gereği koşulamıyorlar, kırılmasınlar diye. Kompakt listede eski
  `drawing.layers|entities|entity|measure|selection` (yerlerini `scene.*` aldı), `drawing.replay` ve `drawing.job.*`
  (koruyucuyu atlatırdı) yok; `session.status` ve `desktop.detach` eklendi. Çizim kapanınca koruyucusu da gider.
- **`extension.*`** Faz 3'ün `.kek` sistemi olmadan anlamsız; o faza kaldı.
- **§1'in açığı kapandı:** masaüstü bağlantısı bir değişiklik kümesinin adımlarını da modül kuralından geçirir; küme
  çekirdeğin olsa da içine konan duvar Harita'da reddedilir. MCP'nin yıkıcı komut listesi ortak katmana taşındı.

| Ölçüm | Tam profil | Kompakt |
|---|---|---|
| Araç sayısı | 534 | 44 |
| `tools/list` şeması | 7 864 902 bayt | 39 787 bayt (~198 kat küçük) |

**Doğrulama:** `agent_tools` testleri (headless, 8): kategoriler ve arama (Türkçe sözcük, bölüm süzgeci), plansız
yazmanın reddi, planla yazma ve planın tek kullanımı, başka girdinin planı kullanamaması, değişmiş çizimde
`revision_conflict` ve hiçbir şey yazılmaması, yanlış kapı, okuyan komutun plansız çalışması, iki adımlı kümenin tek
geri alma adımı, mimariyi sunmayan evsahibinde aramanın ve kapının gizlemesi ve kümeye gizlenen komutun reddi,
denetim/onarım ve İşlemler kapıları. Masaüstü: değişiklik kümesi adımının reddi. MCP programı elle sürüldü (JSON-RPC):
kompakt liste, `drawing.new`, `catalog.search`, plansız yazma reddi, plan (1 eklenecek), yazma, aynı planla ikinci
yazmanın reddi, yanlış kapı, `session.status`, `audit.list_providers`.

## §4. Uygulama içi yapay zekâ ajanı (`ADIM 15A`)

**İstek.** Kullanıcı kendi ajanını (sağlayıcı, model, kendi API anahtarı) ayarlardan tanımlar, birini varsayılan yapar,
onunla sohbet ederek çizdirir; Anthropic, OpenAI, DeepSeek, Gemini, Kimi, GLM ve öbürleri. Araştırma:
`docs/research/2026-11-13-yapay-zeka-saglayicilari.md`.

**Karar.**
- **Sağlayıcı katmanı ayrı crate, `kentos-agent`** (`crates/native/agent`): arayüz, GPU, veritabanı ya da başka bir
  KentOS crate'i yok (`scripts/arch/deps.mjs`, `agent` grubu). Kendi tokio çalışma zamanı (iki iş parçacığı,
  `kentos-ajan`); sonuçlar kanalla gelir, Iced'in yürütücüsü bekler. 13 sağlayıcı, dört tel biçimi (araştırmanın
  "Soyutlama önerisi"): Anthropic Messages; OpenAI Responses (OpenAI, xAI; `store: false`, şifreli düşünce öğeleri geri
  gider); Chat Completions, her sağlayıcıya bir tuhaflık profiliyle (DeepSeek, Kimi, GLM, Qwen `reasoning_content`; GLM
  `tool_stream`; Mistral çağrı kimliği; Ollama ve LM Studio `tool_choice`'suz); Gemini'nin `streamGenerateContent`'i
  (düşünce imzaları). Araç adları geri dönüşlü eşlenir (`cad.run` ↔ `cad__run`); `tool_choice` yalnız auto, örnekleme
  parametresi yok, en çok 25 araç turu.
- **Sağlayıcının kaydı aynen geri gider** (araştırmanın 1. tuzağı): bir turun düşünce öğeleri ve imzaları, sağlayıcı ve
  modelle etiketli, olduğu gibi saklanır ve yalnız ona gönderilir. Sohbetin ortasında ajan değişirse başkasınınki
  gönderilmez. Başarısız ya da durdurulan turdan sonra sohbet son bütün adımda kalır: çizimde yapılanlar unutulmaz.
- **Ajanlar ve anahtarlar ayarlarda değil**, yalnız kullanıcının okuyabildiği `yapay-zeka-ajanlari.json`'da (0600):
  Ayar dosyası dışa aktarılıp paylaşılıyor, anahtar onunla gitmemeli. Anahtarın yeri üç: bu bilgisayarda (o dosyada),
  yalnız bu oturum (bellekte), sağlayıcının ortam değişkeni (`DEEPSEEK_API_KEY` …). Anahtar `Debug`'da, hata
  iletisinde ve günlükte görünmez, sağlayıcının hata metninden de ayıklanır. Okunamayan dosya kenara alınır, üstüne
  yazılmaz, açılışta söylenir. Testler ve görüntü üreticileri kullanıcının dosyasına dokunmaz (`App::boot`: bellekte).
- **Uygulama ayarları › Yapay zekâ:** ajanlar (Varsayılan / Varsayılan yap, Düzenle, Sil) ve Ajan ekle. Ajan, ayarlar
  penceresinin üstünde açılan kendi penceresinde, kendi Vazgeç ve Kaydet'iyle düzenlenir (formun ve pencerenin iki
  Kaydet satırı hangisinin ajanı sakladığını sorduruyordu): sağlayıcı, ad, model (öneri alanı: sağlayıcının önerileri
  ve "Listeyi getir"le sağlayıcının listesi), adres (Çin uç noktası), API anahtarı ve yeri, varsayılan, gizlilik notu;
  alt çubukta "Bağlantıyı dene" en ucuz çağrıyla anahtarı sınar ve sonucu söyler.
- **Asistan paneli:** sağ yuvada bir sekme, her programda; Görünüm › Paneller'de ve Komut ara'da "Asistan"
  (`view.assistant`). `kentos-ui`'nin `AiConversation`'ı: akan yanıt, düşünce, araç kartları (araç, komut, plan ya da
  execute, girdi ve sonuç), Durdur, Yeniden dene, Kopyala; başlıkta Yeni sohbet ve Yapay zekâ ayarları; birden çok ajan
  varken seçici. Ajan yoksa panel nasıl ekleneceğini söyler ve "Ajan ekle" formu açar. Araç kartı komutun Türkçe adını
  ve çağrının ne yaptığını söyler ("Dikdörtgen duvar · plan", "Komut ara · “duvar”"; `agent_tools::call_words`),
  kimlikler kartın ayrıntısındadır. İletinin eylemleri dar panelde alt satıra akar, düğme yazıları kırılmaz ("Yararlı /
  değil" kırılıp belirteçleri görünmez yapıyordu); oylar ipuçlu simge, belirteç ve süre ayrı satırda.
- **Araçlar ADIM 14'ünkü** (`agent_tools`), öndeki çizimde (`AppHost`), ajan bağlantısıyla aynı çekirdekten
  (`App::agent_request`): programın sunduğu komutlar, çizimin gizlediği alanlar, her yazma bir geri alma adımı ve komut
  geçmişinde "Asistan: …". Programın hiçbir komutunu çalıştırmadığı bölümün kapısı sunulmaz
  (`agent_tools::offered_tools`; ölçüm: Mimari ve GIS'te `road.run` yok, Harita ve Yol arayüz yollarıyla beşini de
  açıyor, Görünümler'in `cad.views.edit`'i mimarinin). Koruyucu turlar arasında tutulur. 48 000 baytı aşan sonuç kesilir
  ve "daha dar sorun" denir: belirteçler kullanıcının parası.
- **Onay:** silen ya da var olanı değiştiren bir yazma (MCP'nin yıkıcı listesi, adımlarından biri öyle olan küme, onarım
  ve İşlemler) panelde "Onay gerekiyor" kartıyla bekler; reddedilirse çalışmaz ve ajana söylenir. Ekleyen yazma hemen
  çalışır.
- **Sistem talimatı** Türkçe: önce `drawing.summary`, katalogda arama ve şema, kapılar, önce plan sonra aynı girdiyle
  execute, birim ve eksenler, hatayı okumak, kısa yanıt; kullanıcıya iç kimlik, sürüm ya da plan numarası yazılmaz
  (ilk denemede yanıt "id 7, revision 2" diyordu).
- **Bileşen düzeltmeleri** (sahibin 13 Kasım uyarıları: "combobox her şey iç içe girmiş", model düğmeleri silik):
  `Select` satırı tek satırdır, ad önce sığar, ayrıntı kalan yere "…" ile (DESIGN.md §7.13, "Açılır liste satırı");
  elle yazılan ama önerisi olan değer `Suggest` ile seçilir, düğme sırasıyla değil (§7.13, "Öneri alanı").

**Doğrulama.**
- `kentos-agent`: 15 + 43 test. Dört tel biçimi sahte sunucuya karşı: metin; parçalı araç çağrısı → cevap → metin;
  düşüncenin ikinci istekte aynen gitmesi; 401 → `Auth` (sahte sunucu anahtarı yankılar, iletide yok); 429 ve
  `retry-after` → `RateLimit`; akış içinde hata → `Stream`; iptal → `Cancelled`. DeepSeek kaydının yeniden oynatılması;
  Mistral, Kimi, GLM, Ollama, LM Studio, OpenRouter tuhaflıkları; ajan dosyası geçici klasörde (0600, tek varsayılan,
  oturum anahtarı dosyaya yazılmaz, okunamayan dosya kenara alınır).
- Masaüstü `assistant::tests` (8): sahte sağlayıcıyla planla → uygula ve alanın çizilmesi, iki turun belirteçleri,
  talimat ve araç adları, tek geri alma; silen yazmanın onay beklemesi ve reddedilince çalışmaması; 401'in sohbette
  anahtarsız söylenmesi; ajansız sohbetin ayarlara yönlendirmesi; ajanların ve varsayılanın tutulması, ajan penceresinin
  Esc ile kapanıp ayarların kalması; model önerileri; onay kuralı; programların kapıları. Headless `agent_tools` (8),
  panel listeleri, katalog, ayarlar ve `kentos-ui` seçim kutusu testleri.
- Gerçek pencere (Xvfb, KentOS Mimari, sentetik çizim; DeepSeek `deepseek-flash`, anahtar yalnız oturumun ortam
  değişkeninde): ajan ayarlardan tanımlandı, "Bağlantı çalışıyor: DeepSeek 2 model listeledi."; "Çizimde boş bir yere
  6 m x 4 m bir odanın duvarlarını çiz; yeri sen seç, var olan nesnelerle çakışmasın." isteğinde ajan sahneyi okudu,
  katalogda aradı, `cad.walls.rectangle`'ın şemasını okudu, bir Duvar katmanı açıp odayı merdivenin üstündeki boş yere
  planlayıp yazdı (tur 25–55 sn). (20, 0) verildiğinde merdivenle çakışacağını görüp üç seçenek sordu.

## §5. MCP kaynakları: oturumun özeti ve kataloğun bölümleri (`ADIM 15`)

**İddianın denetimi.** Doğru ve ölçüldü: sunucunun iki kaynağı vardı, `kentos://catalog` ve her çizimin
`kentos://drawing/{tutamaç}`'ı. Bir ajanın ilk okuyacağı şey yoktu: açık çizimlerin ve bağlı masaüstlerinin hepsi tek
okumada, kataloğun bölümleri araç çağırmadan. Üstelik tek katalog kaynağı **43 897 695 bayt**tı ve listede bunu söylemiyordu.

**Karar.**
- `kentos://session/overview`: sunucudaki her çizim ve bağlı her masaüstü için özet (proje türü, birim, koordinat
  sistemi, etkin katman), katman ağacı ve açtığı kapılar; koruyucunun kuralı ve sıradaki adım. Ajanın göreceği gibi:
  araç katmanının evsahibinden okunur (`agent_tools::overview`; çizimin gizlediği alanlar gizli). Masaüstünün kapıları
  burada bilinmez (programı sunmadığını kendisi reddeder, §1), `null` yazılır; yanıt vermeyen masaüstü nedenini söyler.
- `kentos://catalog/categories`: bölümler, kapıları, aileleri, komut sayıları (`agent_tools::every_category`;
  `catalog.categories` aracının aynısı, çizim açılmadan).
- `kentos://catalog` kalır ama listede boyutu (`size`) ve "ajan için bölümler, arama ve tanım yeter" uyarısıyla.
- **Faz 3'e kalanlar** (`.kek` sistemiyle): `kentos://extensions/installed`, `kentos://sdk/ai-kek-pack` ve eklenti
  etkinleşince `notifications/tools/list_changed`. Özetteki `extensions` şimdilik boş liste.

| Kaynak | Bayt (bir çizim açık) |
|---|---|
| `kentos://session/overview` | 5 873 |
| `kentos://catalog/categories` | 6 791 |
| `kentos://catalog` | 43 897 695 |

**Doğrulama:** headless `an_agent_reads_a_drawing_and_the_catalog_in_one_read` (özet, katman ağacı, beş kapı; mimariyi
sunmayan evsahibinde dört kapı; bölümler sırasıyla beş kapı). MCP'nin kaynak testi güncellendi (kural gereği koşulmadı,
derlemesi denetlendi). Program elle sürüldü: `resources/list` dört kaynak ve kataloğun boyutu, özet çizimi ve beş kapıyı,
bölümler 171 / 65 / 43 / 94 / 108 komutu verdi.
