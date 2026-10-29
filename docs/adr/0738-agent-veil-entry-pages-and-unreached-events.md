# ADR 0738: Ajanın okuduğu, Veri girişinin sayfaları ve hiç erişilmemiş kaydın olayları

- **Durum:** kabul edildi (2026-10-29). `plan/gis.md` CBS-38'in dördüncü dilimi. Kapsamı ben belirledim.
- **Kalan işler:**
  - planın tamamlanma ölçütü: "Formda gizli alan tablo export'u veya **MCP sorgusundan okunamamalı**; erişilemeyen kaydın
    varlığı autocomplete/sayımda açığa çıkmamalı";
  - planın "… **AI bağlamı** da aynı projection'a tabi olsun";
  - [ADR 0550](0550-one-reach-resolver.md) §6'nın iki bilinen sınırı: olay günlüğünde kapsam dışı kaydın kimliği ve zamanı,
    sayfa imlecinin kapsam dışı kayıt olabilmesi;
  - [ADR 0649](0649-form-scope-in-sql-and-pooled-connections.md)'un "sayfaların SQL'de süzülmesi".
- **Bağlam belgesi:**
  - ADR 0443 §2 (`redact`: örneklerde gizle, komut geçmişinin kuralı), 0638 §5 (`withheld`);
  - ADR 0550 (tek erişim çözücüsü `kentos_forms::reach`), 0539 ve 0649 (Veri girişinin kapsamı, SQL'de);
  - ADR 0133 (MCP sunucusu), 0137 (masaüstünün ajan bağlantısı).
- **Web:** web'in bulut istemcisi aynı cevapları okur: sayfalar dolu gelir, imleç verildiği gibi kullanılır, olaylar azalır.
  Web'de değişmesi gereken yok (TODOS.md §0.2'ye satır yazıldı).
- **`.kcad`:** şema değişmez. **Sunucu:** göç yok; olay kaydının JSON'una yalnız sunucunun okuduğu `earlier` eklenir.
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - ajanın gizli alana göre **süzmesi ve araması**: cevapta değer yoktur, ama bir sorgunun ya da seçimin koşulu gizli
    alanı okuyabilir (kâhin). `redact` bir erişim denetimi değildir (ADR 0550 §5); sahibinin haklarıyla yazan ajan formun
    işaretini de kaldırabilir;
  - tırnaksız anılan değer (bir etiket metninin içinde) ve kuralların bilmediği biçimdeki cevaplar;
  - dosya dışa aktarmalarında (DXF, GeoPackage …) gizli alanlar: planın dışa aktarma yetkisi;
  - kaynak, dataset, medya ve onay yetkileri; önbellek anahtarları; saklı kopyaların kesilebilmesi; sayı ve tarih
    alanlarının SQL'e itilmesi (ADR 0649).

## Bağlam

ADR 0550, formun "örneklerde gizle" dediği alanın (`redact`) değerinin "hiçbir dosyaya, kopyaya, sayıya, istatistiğe,
grafiğe ve **ajanın cevabına** girmez" kuralını koydu. Sunucu ve masaüstünün dışarı giden yolları bu kuralla kesiyordu.
MCP ve masaüstünün ajan bağlantısı ise kesmiyordu: `drawing.entities`, `drawing.entity`, kayıt sayfaları, kaynak
sorguları, retlerin iletileri ve kayıt (`drawing.journal`) gizli değerleri olduğu gibi veriyordu. Planın tamamlanma
ölçütü tam bunu ister: gizli alan MCP sorgusundan okunamamalı.

Sunucuda Veri girişinin iki sızıntısı ADR 0550 §6'da bilinen sınır olarak kalmıştı:

- **Olaylar.** Kapsamın dışındaki kaydın her değişikliği Veri girişine silme olarak, kimliğiyle gidiyordu. Hiç erişmediği
  bir kaydın oluşturulduğunu, değiştiğini ve silindiğini, zamanıyla öğreniyordu.
- **Sayfalar.** Sayfa kimlik sırasıyla `limit + 1` satır okunup kapsamla süzülüyordu. Kapsamı katmanın %1'i olan Veri
  girişine 500'lük sayfalar beşer kayıtla, çoğu boş geliyordu. İmleç sayfanın son okunan satırıydı: kapsam dışı bir
  kaydın kimliği.

## Karar

### §1 Ajana giden her cevap kesilir

`kentos_headless::Veil` bir çizimin ajandan sakladığı alanları ve tuttukları değerleri bilir:

- Formların `redact` dediği ve katmanların `withheld` dediği alanların adları (`python_call::redacted_names`, tek
  çözücünün listesi). Gizli alan yoksa perde de yoktur, cevap olduğu gibi gider.
- Çizimde bu alanların tuttuğu boş olmayan değerler.

Cevabın değer taşıyan biçimleri kesilir; kesilen yere `‹gizli›` yazılır:

| Biçim | Ne |
|---|---|
| öznitelik haritası (`attrs`, `values`) | gizli alanın değeri (null dışında her değer) |
| birleşik görünüm (`columns`'un `field`'ı) | gizli alanın sütunundaki her hücre (`cells[i]`), toplamın `sum`, `count`, `skipped`'i |
| uzlaştırmanın alanı (`name`, `layer`, `table`) | iki yanın değeri |
| istatistik (`field`, `read` …) | `read`, `skipped`, `sum`, `mean`, `min`, `max`, `std` |
| her metin | tırnak içinde (“…”) anılan gizli değer |

- Yapının kendi anahtarları alan değildir: gizli alanla aynı adlı bir katman, alanın tanımı ya da değer listesi olduğu
  gibi kalır.
- Kayıt (`drawing.journal`) önce komut geçmişinin kuralıyla yazılır (ADR 0443 §2), sonra kesilir.
- Özet (`drawing.summary`, `drawing.open`, `drawing.new`, masaüstünde `summary`) gizli alanları `hidden` ile adlandırır.
  Ajan değerin neden `‹gizli›` olduğunu bilir.
- Retler de kesilir. Örneğin tekil alanın reddi: “TC” katmanda tekil olmalı; “‹gizli›” başka bir nesnede var.

Nerede:

- **MCP'nin yerel çizimleri** (`kentos-mcp`). Nesne sayfaları, tek nesne, komutların cevabı ve reddi, işler ve kaydın
  yeniden oynatılması kesilir.
- **Masaüstünün ajan bağlantısı.** `kentos_headless::rpc::call_for_agent` her isteği aynı kuralla yanıtlar.
- **Masaüstünün Python konsolu** çizimi bütünüyle okur (`rpc::call_from`). Çizim sahibinindir (ADR 0550 §3), konsol onun
  kendi kodudur.
- **Sunucudaki proje.** MCP `project.open` ile sunucunun kesilmiş kopyasını alır (ADR 0550 §2). Kopyayı bütün alan form
  tasarımcısının çiziminde de perde yerelde iner.

Ajan, sahibin haklarıyla yazar: kendi yazdığı değer cevapta değil, çizimde vardır. Gizli bir değeri açık bir alana
kopyalayan bir yazma onu açık yapar, sahibin kendisinin yapması gibi. Perde bir erişim denetimi değil, ajanın bağlamına
kazara değer girmemesidir.

### §2 Veri girişinin sayfaları dolu, imleci kendi kaydı

Formları kayıtlarını daraltan Veri girişinin sayfası (`projects::reached_page`):

- Kimlik sırasıyla okunan satırlar: daraltılmayan katmanlarınki ve daraltılan katmanlarda ADR 0649'un koşulunun
  tuttukları. Koşul gönderilemiyorsa katmanın bütün satırları okunur.
- Her satır kapsamla yeniden yargılanır. Okuma, `limit` kayıt kapsama girene ya da satır kalmayana kadar sürer.
- Sayfa sonuncusu dışında doludur.
- İmleç verilen son kayıttır, hiçbir zaman kapsam dışı bir kayıt değildir. Ancak arkasından kapsamdaki bir kayıt
  geliyorsa verilir.

Kapsam katmanı daraltmıyorsa ya da çağıran Veri girişi değilse sayfa eskisi gibidir.

### §3 Hiç erişilmemiş kaydın olayı gitmez

Sunucu (`changes::commit_in`) bir kaydı değiştirirken ya da silerken önceki kopyasından Veri girişinin kapsamına
gerekeni olay kaydına yazar (`events::Earlier`, kaydın kimliğiyle `earlier`):

- kaydın katmanı;
- katmanın kapsamlarının okuduğu alanlar (`Scopes::reads`, `Reach::scope_reads`) ve değerleri.

Yalnız formları kayıtlarını daraltan katmanlarda ve kapsamlar yalnız öznitelik okuyorsa yazılır. Şekli, etiketi, türü
ya da başka katmanı okuyan kapsam için yazılmaz (`ScopeReads::More`). `earlier` istemciye gitmez: olayın sözleşmesinde
alanı yoktur.

Veri girişi olayları okurken her değişiklik şöyle yargılanır (`events::hide_out_of_reach`):

| Değişiklik | Kayıt şimdi | Önceki kopya | Giden |
|---|---|---|---|
| oluşturma | kapsamda | — | oluşturma |
| oluşturma | kapsam dışı ya da silinmiş | — | hiçbir şey |
| değişiklik | kapsamda | — | değişiklik |
| değişiklik | kapsam dışı ya da silinmiş | kapsamdaydı ya da bilinmiyor | silme, sürümsüz |
| değişiklik | kapsam dışı ya da silinmiş | kapsam dışındaydı | hiçbir şey |
| silme | — | kapsamdaydı ya da bilinmiyor | silme |
| silme | — | kapsam dışındaydı | hiçbir şey |

- **Önceki kopya** Veri girişinin bugünkü kapsamıyla yargılanır (`Reach::reached`).
- **"Bilinmiyor"** şu durumlardır: kopya yazılmamış (katman o an daraltılmamıştı ya da kapsam şekli okuyordu), ya da
  bugünkü kapsam kopyanın tutmadığı bir alanı okuyor. Bunlarda eski davranış sürer: silme olarak gider, çünkü Veri
  girişi kaydı tutuyor olabilir.
- Kayıtlarının hepsi düşen bir değişiklik olayı (proje ve blok değişikliği yoksa) hiç gitmez. İmleç yine ilerler.
- Düzenleyici ve öteki roller her olayı eskisi gibi duyar.

## Doğrulama

- **Perdenin kuralı.** `scripts/fixtures/agent_veil_cases.py` ADR'den, KentOS kodu olmadan 23 durum yazar
  (`fixtures/agent-veil/v1/cases.json`): öznitelik ve değer haritaları, iç içe kayıtlar, aynı adlı yapısal anahtarlar,
  tırnaklar, birleşik görünüm, uzlaştırma, istatistik, sayılar. `kentos_headless::veil::tests::the_shared_cases_are_cut_alike`
  aynılarını verir.
- **MCP.** `cargo test -p kentos-mcp --test agent_veil`:
  - formun gizlediği bir alan ve "dışarı verilmez" tekil bir alanı olan çizim kurulur;
  - özet, nesne sayfası, tek nesne, kayıt sayfası, tekil alanın reddi ve kayıt okunur; hiçbirinin cevabında gizli dört
    değerden biri yoktur;
  - açık alanın değeri ve `‹gizli›` vardır.
- **Masaüstü.** `python::tests::an_agent_reads_no_value_of_a_field_the_drawing_withholds`: ajan bağlantısının sayfasında
  değer yoktur, özet `hidden: ["TC"]` der, konsolun Python'u değeri okur.
- **Sunucu** (geçici PostGIS'te, `KENTOS_TEST_DB=required cargo test -p kentos-application --test project_form_scope`):
  - `a_veri_girisi_pages_are_full_and_lead_on_from_its_own_records`: 12 kayıt kimlik sırasıyla, her ikincisi Ali'nin.
    1, 2, 5, 6 ve 100'lük sayfalarla, SQL'e itilen ve sunucuda yargılanan kapsamla okunur. Her imleç Ali'nin bir kaydıdır,
    imleçli her sayfa doludur, sayfaların birleşimi Ali'nin kayıtlarıdır, sırasıyla.
  - `a_veri_girisi_hears_nothing_of_records_it_never_reached`: her commit'ten sonra Ali'nin duydukları okunur.
    Ali'nin kaydının oluşturulması gider. Ayşe'ninki ve onun değişikliği gitmez (olay hiç gitmez, imleç ilerler).
    Kapsamdan çıkan kayıt sürümsüz silme olarak gider, kapsama giren değişiklik olarak. Ayşe'nin kaydının oluşturulup
    silinmesi gitmez, Ali'nin kaydının silinmesi gider. Ali okumadan önce kapsamından çıkıp silinen kaydı, değişikliğin
    olayı silme olarak söyler. Silmenin kendisi söylemez: önceki kopyası kapsam dışındaydı. Düzenleyici hepsini duyar.
    Şekli okuyan kapsamda kapsam dışı değişiklik eskisi gibi silme olarak gider.
  - ADR 0550'nin testi (`project_reach`'in `a_veri_girisi_is_told_nothing_of_the_records_out_of_its_reach`) yeni kurala
    göre: Ayşe'nin kaydının oluşturulması Ali'ye artık hiç gitmez.
- **Çözücü.** `kentos-forms`'un `scope::tests::a_scope_says_what_it_reads` ve
  `reach::tests::an_earlier_copy_tells_whether_a_veri_girisi_reached_it`.

## Ölçümler

Release, bu makinede, geçici PostGIS'te (`cargo test --release -p kentos-application --test project_form_scope
scope_timing -- --ignored --nocapture`). Ölçülen, 100 000 kayıtlı katmanda kapsamı 1 000'ini veren Veri girişidir (beş
denemenin en iyisi).

| Kapsam | Sayı | İlk 500'lük sayfa | Bütün sayfalar |
|---|---|---|---|
| SQL'e itilen (`Sorumlu = @kullanici_eposta`) | 46 ms | 32 ms | 55 ms |
| sunucuda yargılanan (`uzunluk(Sorumlu) = 17 ve …`) | 543 ms | 340 ms | 691 ms |

Önceki kuralda her sayfa 501 satır okuyup Ali'nin yaklaşık beş kaydını veriyordu. 1 000 kayıt için 200 istek gerekiyordu.

Düzenleyicinin daraltılmış katmanda 1 000 değişikliği (üç denemenin en iyisi):

| | Commit | Olayın kaydı |
|---|---|---|
| önceki kopyalarla (kapsam öznitelik okur) | 278 ms | 72 219 bayt |
| kopyasız (kapsam şekli okur) | 264 ms | 37 206 bayt |

Önceki kopyalar commit'e bir sorgu ekler: değişen kayıtların öznitelikleri, yalnız daraltılmış katmanlar için. Bu
yaklaşık %5 süre ve olay kaydında kayıt başına alanların değeri demektir.

Perde yalnız gizli alanı olan çizimde kurulur. Kurulurken çizimin nesneleri bir kez gezilir.
