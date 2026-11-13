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

