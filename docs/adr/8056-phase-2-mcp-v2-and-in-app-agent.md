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
