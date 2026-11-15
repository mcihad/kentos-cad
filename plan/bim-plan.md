# KentOS Mimari — BIM geliştirme ve yönetmelik uyum planı

İnceleme: **15 Kasım 2026** (ikinci tur; ilki aynı gün, `8bc6bba77`). Kod tabanı: `8bc6bba77`. Dal: `codex/bim-ifc-uyum`.

**Eş belge:** [IFC uyum ve teslim planı](ifc-uyum.md) · **Dayanak:**
[araştırma notu](../docs/research/2026-11-15-bim-ifc-yonetmelik.md) · **Önceki işler:** [Mimari](architecture.md),
[İleri mimari](architecture-advanced.md) (M1…M51), [mevzuat araştırması](../docs/research/2026-11-07-mimari/W4-mevzuat-standart.md).

Bu tur **araştırma ve plandır**; uygulama kodu değişmedi. İlk taslağa göre değişenler: iki yönetmeliğin ana metni madde madde
yeniden okundu; eklerin bağımsız bir dökümü depodaki sayfa okumalarıyla karşılaştırıldı; bugünkü kod yönetmeliğin sınıf
ve alan listesine karşı ölçüldü (§3); mimari modülün zayıf yanları kanıtıyla sayıldı (§3.2); iş paketleri yasal takvime ve
bağımlılığa göre yeniden kuruldu (§5, §6). Paket numaraları [TODOS.md §18](../TODOS.md) ile aynıdır.

## 0. Özet

- **Yönetmelik ne istiyor:** ruhsat projesinin 2B paftaları pafta başına PDF/A (1.9.2027'den kademeli), mimari modeli IFC 4.3
  (1.9.2029'dan kademeli; Sivas'ta 1.9.2032), teslimi e-PYS üzerinden (1.9.2028). Çizimler **modelden üretilir**; model ile
  pafta uyumlu olmak zorundadır; her teslimde 21 soruluk model kalite kontrol formu düzenlenir.
- **KentOS nerede:** 2B teslim büyük ölçüde hazır (Ruhsat takımı, PDF/A-2b, EK-2 adları, EK-3 numarası, EK-4 katmanları).
  IFC dışa aktarma çalışıyor ve şema kurallarından hatasız geçiyor, ama mimari modelde karşılaşılan 29 zorunlu sınıfın
  15'ini yazıyor; yangın dayanımı ve acil çıkış gibi zorunlu bilgiler çizimde tutulmuyor; emsal alanları modelde nesne
  değil; form yazılmış dosyadan değil bellekteki modelden cevaplanıyor.
- **Mimari modülün zayıflığı:** binanın 3B görünümü yok; eleman sınıfları dar; tek bina; eğri geometri yaklaşık; dış model
  okunamıyor; paftanın modelden çıktığı kanıtlanamıyor (§3.2, Z1…Z14).
- **Yol:** önce kaynak (ekler sayfa sayfa kataloğa), sonra 2B teslimin kalanı, sonra ruhsat düzeyinde model (3B görünüm,
  yangın bilgisi, zonlar, eksik elemanlar, geometri, çok bina), sonra dosyadan doğrulama ve tek teslim paketi, sonra dış
  disiplinler ve belediyenin inceleme tarafı (§6).

## 1. Hedef ve sınır

- **Ürün hedefi.** Mimar, bugünkü hızlı 2B araçlarla çizerken duvar, açıklık, mahal, döşeme ve öbür elemanları kimlikli yapı
  nesneleri olarak üretir. Aynı model revizyonu kat planını, kesiti, görünüşü, vaziyet planını, çizelgeleri, mimari IFC'yi,
  model kalite kontrol formunu ve PDF/A paftaları besler. Bir duvar yer değiştirdiğinde hangi mahalin, ölçünün, kaçış
  yolunun, paftanın ve teslim dosyasının etkilendiği görülür.
- **İki kullanıcı.** Proje müellifi hazırlar ve yükler; idare ve yapı denetim kuruluşu inceler (e-PYS Md. 5). Müellifin ön
  denetimi ile idarenin incelemesi aynı kural kataloğunu kullanır.
- **İlk üretim dilimi.** Konut ve ticaret+konut karma kullanımlı yeni binalar: yönetmelik önce onlara uygulanır, öbür
  binalar 1.9.2033'te gelir. Yönetmeliğin kapsamı "yeni yapılacak binalar"dır (Md. 1/2); tadilat projeleri aynı modelle
  sürer (ADR 8039).
- **Disiplin sınırı.** KentOS Mimari yalnız mimari disiplinin modelini (MM) yazar. Statik, elektrik ve tesisat modellerini
  kendi müellifleri kendi yazılımlarıyla verir; KentOS onları değiştirmez, altlık alır, hizalar ve karşılaştırır.
- **Beş ayrı kabul.** "Uygun" sözü bunlardan hangisinin sağlandığı söylenerek kurulur:
  1. dosya okunuyor (STEP);
  2. şema geçerli (IFC 4.3'ün kuralları);
  3. Türkiye teslim profili (EK-1…EK-7);
  4. model ile pafta uyumu (Md. 9/1);
  5. idare kabulü.

  Tasarımın hukuki ve mesleki uygunluğuna müellif ve idare karar verir; KentOS ölçer ve kanıt gösterir.
- **Kapsam dışı.** Statik ve tesisat tasarımı; e-imzanın üretimi (yöntemini Bakanlık belirleyecek, e-PYS Md. 5/4); resmî
  onay iddiası; web (ADR 0228; gerekenler TODOS.md §0.2'ye).

## 2. Yönetmelik ve takvim

Metinler ve ekler [araştırma notunda](../docs/research/2026-11-15-bim-ifc-yonetmelik.md) §1–§3'tedir.

### 2.1 Takvim

| Tarih | Ne başlar | Kimde | KentOS'ta hazır olması gereken |
|---|---|---|---|
| 1.9.2027 | PDF/A paftalar (Hazırlama Md. 4/2–3) | İl > 2.000.000, belediye > 600.000, toplam yapı inşaat alanı > 5.000 m² | Kapı 1: BIM-01…05 |
| 1.9.2028 | e-PYS Yönetmeliği | Kanal, biçim ve tarih Bakanlığın kararıyla (e-PYS Md. 4/4) | BIM-14, BIM-15; idare için BIM-18 |
| 1.3.2029 | PDF/A paftalar | Kapsamdaki öbür bütün binalar (Sivas dahil) | Kapı 1 |
| 1.9.2029 | IFC modeli (Md. 4/5), 1. grup | İl > 2.000.000, belediye > 600.000, alan > 10.000 m² | Kapı 2 ve 3 |
| 1.9.2030 | IFC, 2. grup | İl > 2.000.000, belediye > 400.000, alan > 10.000 m² | — |
| 1.9.2031 | IFC, 3. grup | İl > 1.000.000, belediye > 300.000, alan > 10.000 m² | — |
| 1.9.2032 | IFC | Kalan bütün binalar (Sivas dahil) | — |
| 1.9.2033 | Yönetmeliğin tamamı | Konut ve ticaret+konut dışındaki binalar | Kapı 5 |

Sivas satırları bir çıkarımdır (il nüfusu 1 milyonun altında; TÜİK'ten doğrulanmadı). Yasal tarihler ürün hedefinin en geç
sınırıdır; süre tahmini verilmez (TODOS.md §22), sıra bağımlılıkla kurulur (§6).

### 2.2 Yönetmeliğin yazılımdan istedikleri

✓ var · ◐ kısmen · ✗ yok.

| # | İstenen | Dayanak | KentOS'ta bugün | Paket |
|---|---|---|---|---|
| R1 | Her pafta ayrı PDF/A dosyası; A serisi kâğıt | Md. 4/2, 8 | ✓ Ruhsat takımı, PDF/A-2b (ADR 8037); bağımsız doğrulama yok | BIM-05 |
| R2 | Çizim dışı belgeler PDF/A | Md. 4/3 | ◐ hesap ve liste paftaları var; dış belge pakete girmiyor | BIM-05 |
| R3 | Proje kodu ve dosya adları | Md. 6; EK-1, EK-2 | ✓ pafta ve model adı (ADR 8037, 8047 §7) | BIM-01 |
| R4 | CAD katmanı, sembol, çizgi; mahal numarası ve adı | Md. 7; EK-3, EK-4 | ◐ katman anahtarı (ADR 8008) ve EK-3 numarası (ADR 8027) var; denetimi yok | BIM-03 |
| R5 | Çizimler modelden üretilir; model ile PDF/A uyumlu | Md. 9/1; EK-9/21 | ◐ plan modelin kendisi, kesit ve görünüş türev (ADR 8042–8043); kanıtı yok | BIM-04, BIM-14 |
| R6 | IFC teslimi | Md. 4/5; EK-5 §5.4 | ◐ IFC 4.3 dışa aktarma (ADR 8047) | IFC-02…06 |
| R7 | Zorunlu sınıflar ve alanları; vekil sınıf kullanılmaz | Md. 12; EK-5, EK-6 | ◐ 29 sınıftan 15'i; yangın ve dayanım alanları yok | BIM-07…09; IFC-02, IFC-03 |
| R8 | Proje bilgileri, kişiler, koordinat sistemi | Md. 13; EK-7 | ◐ künye, mimar, büro, TUREF dilimi var; proje tipi ve açıklaması yok | IFC-02, IFC-05 |
| R9 | Model, mahal ve varlık adları | Md. 10, 11; EK-5 §5.5 | ◐ kalıp var; iki yorum açık | BIM-01, IFC-02 |
| R10 | Gelişim seviyesi | Md. 14; EK-8 | ✗ ölçülmüyor, görülemiyor | BIM-06, BIM-10, IFC-06 |
| R11 | Model kalite kontrol formu | Md. 15; EK-9 | ◐ 21 soru bellekteki modelden | IFC-08, BIM-14 |
| R12 | Bilgi yönetimi | Md. 4/4; TS EN ISO 19650-1/2 | ✗ | BIM-15 |
| R13 | Tarihe ve koşula bağlı kapsam | Md. 1; Geçici 1–2 | ✗ | BIM-02 |
| R14 | e-PYS: müellif yükler, idare inceler; e-imza ya da basılı; taranmış PDF kullanılmaz | e-PYS Md. 4–7 | ◐ vektör PDF/A var; paket, kayıt ve inceleme yok | BIM-14, 15, 18 |

## 3. Bugünkü durum (15 Kasım 2026)

### 3.1 Güçlü taban

- **Model nesneleri:** duvar (katmanlı, birleşimli), kapı/pencere/boşluk (tip ve örnek), mahal, döşeme, aks, kolon, kiriş,
  çatı, korkuluk, merdiven, rampa; katlar ve düşey model (ADR 8040–8041, 8044–8045).
- **Türevler:** kesit ve görünüş modelden, eskime işaretiyle (ADR 8042–8043); çatı planı; çizelgeler ve metraj.
- **Türkiye'ye uygun proje:** PAİY alanları, emsal ve TAKS, kat irtifakı, vaziyet planı, otopark ve sığınak hesapları (ADR
  8029–8032, 8036); mevzuat denetimi ve kaçış yolları (ADR 8033–8035); Ruhsat takımı, Türk anteti, PDF/A-2b (ADR 8037–8038).
- **Dijital proje:** EK-4 katman anahtarı (ADR 8008), EK-3 mahal numarası (ADR 8027), EK-2 dosya adları, EK-9 formu, IFC 4.3
  bina modeli ayrı pakette (`kentos-ifc`, ADR 8047), eski plandan tanıma (ADR 8048).
- **Ortak altyapı:** komut kataloğu, Python ve MCP; bağımsız başvurular; gerçek pencere kabulü (ADR 8000); proje bulutu ve
  dosya revizyonları (ADR 0031); CBS tarafı (parsel, plan, koordinat sistemi); yolun 3B sahnesi ve çizicisi (ADR 0910).

### 3.2 Mimari modülün zayıf yanları

Önem: K uyum için şart, Y yüksek, O orta.

| # | Zayıflık | Kanıt | Önem | Paket |
|---|---|---|---|---|
| Z1 | **Binanın 3B görünümü yok.** Model yalnız planda, kesitte ve dışa aktarılan dosyada görülüyor; gövde hatası, mahal örtüşmesi, yanlış kot karşı programda fark ediliyor | 3B pencere yalnız yol sahnesini açıyor (`apps/desktop/src/shell/scene3d_window.rs`); bina gövdelerini kesit modeli kuruyor, gösteren yok | K | BIM-06 |
| Z2 | **Yangın ve performans bilgisi tutulmuyor:** yangın dayanım sınıfı, acil çıkış, malzeme dayanımı, yapım yöntemi, yangın koruma sınıfı | Sözleşmelerde alan yok (`crates/shared/contracts/src/arch`); EK-9'un 9. sorusu "hayır" (ADR 8047 §9) | K | BIM-07 |
| Z3 | **Emsal alanları modelde nesne değil;** bağımsız bölüm modelde grup olarak durmuyor | EK-9/10 ve EK-6 6.60 mekânsal zon istiyor; yazılmıyor | K | BIM-08 |
| Z4 | **Paftanın modelden üretildiği kanıtlanamıyor:** plan paftasındaki serbest çizim ayırt edilmiyor; IFC ile PDF/A'nın aynı revizyondan çıktığı kayıtlı değil | EK-9/21 "modele girmeyen nesne yoksa evet" (`quality.rs`); Ruhsat takımı ile `cad.arch.ifc` ayrı komutlar | K | BIM-04, BIM-14 |
| Z5 | **Eleman sınıfları dar:** giydirme cephe, kaplama ve asma tavan nesnesi, baca, dikme yok; asansör, vitrifiye ve mobilya sınıfsız blok; rampanın gövdesi yok | `Entity` listesi (`contracts/src/core/entity.rs`); `arch.rs` `Why::NotYet`; ADR 8045 | Y | BIM-09 |
| Z6 | **Geometri her yerde gerçek değil:** eğri duvar ve merdiven düz parçalarla; mahalin hacmi düz prizma; açıklık gövdeden kesilmiyor; içe dönen saçak ve çatı katmanları yok | ADR 8047 §6; `arch.rs` (mahal `bottom + height`); ADR 8044 "sonraya" | Y | BIM-10 |
| Z7 | **Tek bina:** program bina ve kanat tanıyor; model, künye ve dışa aktarma tek binalı; ortak bodrum yok | `building.rs` tek saha, tek bina; `program.rs` | Y | BIM-11 |
| Z8 | **2B kuralların denetimi yarım:** EK-4 katmanı ve çizgi tipi, EK-3 numara biçimi ve mahal adı sözlüğü denetlenmiyor | ADR 8047 §9 "yalnız varlığı"; M9 "profil ve blok katmanı kaldı" | Y | BIM-03 |
| Z9 | **Mevzuat denetimi yarım:** başvuru tarihine göre kural seti, plan notu katmanı, yangın dayanımları, erişilebilirlik, emsalin 22. madde sınırları | ADV-22 ve ADV-23 "Kalanlar" | Y | BIM-13 |
| Z10 | **Dış model okunamıyor:** statik ve tesisat modeli altlık olamıyor, çakışma bulunamıyor, iki revizyon karşılaştırılamıyor; belediye gelen IFC'yi KentOS'ta açamıyor | Depoda IFC okuyucu yok | Y | IFC-07, IFC-09; BIM-17, BIM-18 |
| Z11 | **Tip sistemi yarım:** duvar stili ve kapı/pencere tipi var; döşeme, kolon, kiriş, çatı, merdiven, korkuluk tipsiz | W2 §2.1; IFC'de tip nesnesi ve katmanlı malzeme takımı yok | O | BIM-12 |
| Z12 | **Teslim kaydı yok:** hangi revizyon, hangi dosyalarla, kime verildi, ne cevap geldi | Dosya revizyonu var (ADR 0031); teslim ve iade kaydı yok | O | BIM-15 |
| Z13 | **Hız hedefi tutmadı:** 2+1 daire gerçek pencerede 82 etkileşim ve 121,3 s (hedef ≤ 80 ve ≤ 100 s) | `architecture-advanced.md` §7; ADR 8052 | O | BIM-20 |
| Z14 | **Kalanlar ve kabul:** ADV paketlerinin çoğu `[~]`; gerçek ofis DWG'siyle tadilat kabulü sahibin dosyasını bekliyor; örnek proje, çokgen oda, kot etiketi, 1:20 kasa profili açık | `architecture-advanced.md` §3–§5 | O | BIM-20 |

### 3.3 Ölçülen başlangıç değerleri

| Ölçü | Bugün | Kaynak |
|---|---|---|
| IFC'ye yazılan zorunlu sınıf (mimari modelde karşılaşılan 29'dan) | 15 | `crates/shared/ifc/src/arch.rs`; araştırma notu §5 |
| EK-9'un sorularından yazılmış dosyadan kanıtlanan | 0 / 21 | `quality.rs` bellekteki modeli okur |
| EK-9'da bugün "evet" olamayanlar | 9 "hayır" (zorunlu özellikler); 10 elle (emsal zonu yazılmıyor); 19 "evet" deniyor ama çatı ve merdivenin tipi `NOTDEFINED` | ADR 8047 §9; araştırma notu §4–§5 |
| Şema kuralları, bağımsız okuyucu | bölüm binasında 0 bulgu | ADR 8047 |
| Türkiye profilini sınayan bağımsız okuyucu | yok | — |
| 3B'de görülebilen bina gövdesi | 0 (IFC'de 68) | Z1 |
| Yangın dayanımı taşıyabilen nesne türü | 0 | Z2 |
| Ruhsat takımı | tek komut, PDF/A-2b, pafta başına EK-2 adlı dosya | ADR 8037 |
| 2+1 daire, gerçek pencere | 82 etkileşim, 121,3 s | ADR 8052 |

## 4. Kararlar

- **K1 — Model tek kaynaktır.** KentOS'ta model, kimlikli yapı nesneleri ve katlardır; plan onun kendi görünüşü, kesit ve
  görünüş türevi, IFC aynı kesit modelinin gövdeleridir. Yeni olan bunun kanıtıdır (BIM-04) ve tek teslim girdisidir
  (BIM-14).
- **K2 — Kural veridir.** Sınıf, alan, ad kalıbı, sözlük, eşik ve tarih kodda sabit durmaz; kaynak sayfasıyla sürümlü
  katalogdadır. İdarenin ek tanımları (Md. 11/3, 12/2, 13) ve Bakanlığın sonraki kararları (e-PYS Md. 4/4) veri
  değişikliğidir. IFC'ye ait katalog `kentos-ifc`'tedir; tarih, kapsam ve mevzuat kuralları mevcut kural motorundadır (ADR
  8033).
- **K3 — Bilinmeyen, yanlış değildir.** Bilinmiyor, uygulanmaz, hayır, sıfır ve eksik ayrı durumlardır. Yangın dayanımı
  gibi bir değer varsayılanla doldurulmaz. Her performans değerinin kaynağı görünür: tip, örnek, türetim, müellif beyanı,
  belge.
- **K4 — Doğrulama yazılmış dosyadan yapılır.** Formun cevabı bellekteki modelden değil, diske yazılan IFC ve PDF/A'dan
  okunur. `kentos-ifc` bunun için bir okuyucu kazanır; aynı okuyucu dış disiplin modeline ve idare incelemesine hizmet eder.
- **K5 — Gerçek ruhsat modeli dış servise gönderilmez.** Doğrulama çevrimdışıdır; yalnız açık ve sentetik örnekler dış
  doğrulayıcıda sınanır.
- **K6 — Kimlik kalıcıdır.** Nesnenin kimliği, tipinin kimliği ve pafta ya da mevzuat etiketi ayrı alanlardır. Taşıma ve tip
  değişikliği kimliği korur; kopya ve modül örneği yeni kimlik alır; bölme ve birleştirmede eski–yeni kaydı tutulur; silip
  geri almak nesneyi ilişkileriyle geri getirir.
- **K7 — Genel bir katı modelleme çekirdeği şimdi alınmaz.** Ruhsat modelinin gövdeleri bugünkü kesit modeliyle (prizma, kol,
  eğik yüz) ve ona eklenecek yay ve kesilmiş gövdeyle kurulur. Çekirdek kararı (TODOS.md §23) ölçülen bir ihtiyaç çıkınca
  ADR ile yeniden açılır.
- **K8 — Bir madde, bütün yüzleriyle.** Her paket `architecture-advanced.md` §6'nın talimatıyla yapılır: araştırma, ADR,
  çekirdek ve sözleşme, komut (katalog, ≥ 20 bağımsız durum, Python, MCP), arayüz kapıları, gerçek pencere kabulü, resimler.

## 5. İş paketleri

Her paket bir bütündür; alt maddeleri aynı dilimde biter. `[ ]` başlanmadı, `[~]` sürüyor, `[x]` bütün kabulü sağlandı.

### 5.1 Kaynak ve kapsam

#### BIM-01 — Birincil kaynak ve kural kataloğu [ ]

**Kullanıcı sonucu:** Her denetim bulgusu ve dışa aktarılan her alan, yönetmeliğin hangi ekinin hangi sayfasından geldiğini
söyler. İki okumanın ayrıştığı yerler kapanmıştır. [IFC-01](ifc-uyum.md#10-iş-paketleri) ile aynı dilimdir.

- Eklerin PDF'si bir kez indirilir (`.run/` altında, Git'e girmez) ve EK-1…EK-9 sayfa görüntülerinden okunur. Okunan her
  satır kataloğa sayfası, tablosu ve satırıyla girer: EK-2'nin sözlükleri (2.2–2.14), EK-3 (3.1 ve 3.2'nin yaklaşık 150
  adı), EK-4'ün katman ve çizgi kuralları, EK-5 (5.1, 5.2, 5.3), EK-6'nın mimariye düşen tabloları, EK-7, EK-8'in nesne
  tablosu, EK-9.
- Araştırma notu §4'teki 17 nokta tek tek karara bağlanır: sayfadan okunan, şemadan çözülen, idareye sorulan.
- Katalog bağımsız dökümle satır satır karşılaştırılır; fark listesi ADR'ye yazılır.
- **Başlangıç ölçümü:** bölüm binasının ve 2+1 örneğinin bugünkü çıktıları katalog satırlarına karşı dosyadan ölçülür
  (satır başına var ya da yok).

**İlk kabul:** katalogdaki her satırın sayfası var; rastgele seçilen 20 satır sayfa görüntüsüyle birebir; ölçüm raporu ADR'de.

#### BIM-02 — Tarihli kapsam kararı ve proje bağlamı [ ]

**Kullanıcı sonucu:** Ruhsat takımı penceresi, bu proje için hangi yükümlülüğün hangi tarihten başladığını maddesiyle
söyler (PDF/A, IFC, e-PYS). Bilgi eksikse "teyit gerekli" der, tahmin etmez.

- **Girdiler (künyede):** başvuru tarihi ya da kamuda ihale kararı/tarihi; il ve işlemi yapan belediye; ikisinin nüfusu,
  nüfusun kaynağı ve yılı; kullanım (konut, ticaret+konut, öbürü); toplam yapı inşaat alanı ve kat adedi (çizimden);
  müstakil olup olmadığı; tarım/hayvancılık ve 3194'ün 4, 26 ve 27. maddeleri.
- **Kural:** Hazırlama Md. 1/2, Geçici 1–2; e-PYS Md. 1/2, 8. Sonuç dört durumdur: uygulanır, uygulanmaz (maddesiyle),
  henüz başlamadı (tarihiyle), teyit gerekli.
- Geçici 1/1-b'nin son cümlesi (IFC başlayınca 4/2–3 uygulanmaz) ile Md. 9/1 (modelden üretilen çizimler PDF/A) ayrı
  satırlardır; tek bir bayrak PDF/A'yı kapatmaz.
- Nüfus yönetmelikte bir yıla bağlanmamıştır: kullanıcı girer, kaynağı kaydolur.
- Karar salt okuyan bir komuttur; Python ve MCP'de de vardır.

**İlk kabul:** ≥ 20 bağımsız durum (her eşiğin tam eşiti, altı ve üstü; her tarihin önceki günü ve kendisi; müstakil 200 m²
ve iki kat; karma kullanım; kamu ihalesi). Sivas'ta konut örneği PDF/A için 1.3.2029, IFC için 1.9.2032 verir; nüfus
girilmemişse "teyit gerekli".

### 5.2 2B teslim

#### BIM-03 — 2B kuralların denetimi: EK-3 ve EK-4 [ ]

**Kullanıcı sonucu:** Denetim, paftaya girecek çizimde EK-4'e uymayan katmanı, çizgi tipini ve yazıyı, EK-3'e uymayan mahal
numarasını ve adını, nesnesini seçerek söyler. Dayanak Md. 7; Z8.

- **EK-3:** numara kalıbı; kat kodunun mahalin katıyla uyuşması; kat ve bölüm içinde sıranın tekliği ve saat yönü; adın
  Tablo 3.2'nin sözlüğünden ya da kısaltmasından geldiği. Mahal adı seçicisi sözlükten beslenir.
- **EK-4:** katman adı kalıbı ve sözlüğü; nesne türünün doğru katmanda durduğu; çizgi tipi, ölçü ve yazı kuralları; blok
  katmanı ve profil (M9'un kalanı).
- DXF ve DWG aynı katmanlarla gider (ADR 8046).
- Bulgular Mimari denetim › Mevzuat'ın "Dijital Proje Yön." grubundadır; düzeltmesi tek adımdır.

**İlk kabul:** 2+1 örneğinde her kural için bir geçen, bir kalan durum; bağımsız başvuru KentOS kodu olmadan sınar.

#### BIM-04 — Paftanın modelden üretildiğinin kanıtı [ ]

**Kullanıcı sonucu:** Her ruhsat paftası hangi model revizyonundan üretildiğini taşır. Modelden gelmeyen çizim paftada ayrı
listelenir. Eski kesit, görünüş ya da çizelgeyle ruhsat yayını yapılamaz. Dayanak Md. 9/1; EK-9/21; Z4.

- **Türev kaydı:** kesit ve görünüş blokları, çizelgeler, hesap tabloları ve kontrol formu üretildikleri belge revizyonunu
  taşır. Kesit ve görünüşün "eski" işareti var; ötekilere genişler.
- **Pafta içeriği sınıflanır:** model nesnesi; modele bağlı açıklama (ölçü, etiket, kot); detay çizimi (kaynağı ve amacı
  kayıtlı); serbest çizim. Yapı katmanlarında duran serbest çizim bulgudur ("duvar katmanında 14 serbest çizgi").
- **Yayın kapısı:** eski türev, kopmuş ölçü ya da etiket, yapı katmanında serbest çizim varsa ruhsat paketi durur.
  Gerekçeli istisna kayıtlıdır (mevcut istisna düzeni, ADR 0732).
- Formun 21. sorusu bu kayıttan cevaplanır.

**İlk kabul:** bir kapı değişince kesit, görünüş, doğrama listesi ve mahal listesi "eski" olur ve paket durur; yeniden
üretimden sonra geçer. Duvar katmanına çizilen serbest çizgi raporda nesnesiyle görünür.

#### BIM-05 — 2B ruhsat paketi (PDF/A) [ ]

**Kullanıcı sonucu:** Ruhsat takımı tek adımda, bağımsız denetimden geçmiş, adları EK-2'ye uyan PDF/A dosyaları ve bir teslim
listesi üretir. IFC zorunluluğu başlamamış proje için teslim budur. Dayanak Md. 4/2–3, 8; e-PYS Md. 6.

- **Bugün var:** PDF/A-2b, pafta başına dosya, EK-2 adı, künye, kendi denetimi (ADR 8037).
- **Eklenen:**
  - üretilen baytlarda bağımsız PDF/A doğrulaması;
  - vektör içerik ve gömülü yazı tipi denetimi (taranmış görüntüden pafta reddedilir, e-PYS Md. 6/4);
  - A serisi kâğıt ve "bir dosya, bir pafta" denetimi (Md. 8);
  - teslim listesi: dosya adı, özeti, pafta numarası, revizyonu, üretildiği model revizyonu;
  - atomik yayın: yarım paket kalmaz, önceki paket durur;
  - çizim dışı belgeler (rapor, hesap) pakete PDF/A denetimiyle eklenir.
- PDF/A'nın istenen bölümü ve düzeyi kurum profilinin bir satırıdır (yönetmelik yalnız "PDF/A" diyor).

**İlk kabul:** 2+1'in ve bölüm binasının paketi bağımsız doğrulayıcıdan hatasız geçer. Bilerek bozulan dosya (gömülmemiş
yazı tipi, yanlış ad, iki paftalı dosya) paketi durdurur ve sebebini söyler.

### 5.3 Ruhsat düzeyinde model

#### BIM-06 — Binanın 3B görünümü [ ]

**Kullanıcı sonucu:** Mimar, modelin gövdelerini (dışa aktarılacak olanların aynısını) 3B'de görür. Bir nesneye tıklayınca
planda seçilir. Katları ayırır, kesit kutusuyla içine bakar. Denetim bulguları 3B'de boyanır. Dayanak Md. 14; EK-9/13–15; Z1.

- **Kaynak:** kesit modelinin gövdeleri, yolun 3B görünümünün kullandığı ortak sahne çekirdeği ve çizicisiyle
  (`kentos_scene3d`, `kentos_render_wgpu::scene3d`). İkinci bir 3B motoru kurulmaz.
- **Görünüş:** sınıfa, kata, tadilat durumuna ve bulguya göre renk; mahaller yarı saydam hacim.
- **Etkileşim:** yörünge, kaydırma, yaklaşma; hazır bakışlar; kat süzgeci; kesit kutusu; 3B'de tıkla → planda seç, planda
  seç → 3B'de vurgula.
- Model değişince sahne yenilenir. 2B proje açılışı 3B'yi yüklemez (kılavuz §20).
- Çizimi değiştirmez: görünümdür, komut değildir.

**İlk kabul:** bölüm binasının 68 gövdesi 3B'de; gövde sayısı dışa aktarılan dosyadakiyle eşit; 3B'de tıklanan duvar planda
seçili; örtüşen iki mahal kırmızı; gerçek pencerede resim.

#### BIM-07 — Yangın, performans ve malzeme bilgisi [ ]

**Kullanıcı sonucu:** Duvar stili, kapı ve pencere tipi, döşeme, kolon, kiriş, çatı ve merdiven yangın dayanım sınıfını;
kapı ve merdiven acil çıkış rolünü; taşıyıcı malzeme dayanım sınıfını taşır. Eksik olan, Denetim'de nesnesiyle listelenir;
hiçbiri varsayılanla dolmaz. Dayanak EK-6 6.3, 6.7, 6.16, 6.20, 6.25, 6.52, 6.56, 6.58, 6.61, 6.64, 6.66; EK-9/9; Z2.

- **Alanlar:** yangın dayanım sınıfı (Binaların Yangından Korunması Hakkında Yönetmeliğe göre); acil çıkış (evet, hayır,
  bilinmiyor); malzemenin dayanımı ("Beton_C30" biçiminde yazılır); binanın yapım yöntemi ve yangın koruma sınıfı;
  planlanan net alan; kaplama kalınlıkları; asansörün kişi ve yük kapasitesi.
- **Kaynağı:** tipte ya da örnekte. Her değerin dayanağı saklanır: müellif beyanı, belgenin adı ve tarihi, türetim.
- **Acil çıkış önerisi:** kaçış yolları motorunun (ADR 8035) kullandığı kapılar ve merdivenler aday gösterilir; müellif
  onaylar. Beyan ile rota uyuşmazsa bulgu.
- Türetilen değerler (dışarıda, taşıyıcı) açıklanır ("katın dış çizgisinde olduğu için dışarıda") ve örnekte değiştirilir.
- Yeni `.kcad` alanları; düzenleme komutlarında alanlar; Öznitelikler ve tip pencereleri; çizelge sütunları.

**İlk kabul:** bölüm binasında bütün zorunlu yangın alanları dolunca formun 9. sorusu evet. Bir duvar stilinin sınıfı
silinince o stilin duvarları bulguda. Girilmemiş değer dosyaya boş metin olarak yazılmaz; eksik sayılır.

#### BIM-08 — Mahal, zon ve emsal modeli [ ]

**Kullanıcı sonucu:** Emsal ve TAKS tablosu ile modeldeki alanlar aynı çokgenlerden gelir. Her emsal alanı dahil, hariç ya
da öbürü diye maddesiyle işaretlidir. Bağımsız bölüm ve ortak alan modelde gruptur. Mahalin hacmi kapalıdır ve üstü doğru
yerde biter. Dayanak EK-6 6.58, 6.60, 6.68; EK-9/10, 12–14; PAİY 4, 5/8, 20–22; Z3.

- **Emsal alanı:** kat kat kapalı çokgen; emsal durumu (dahil, hariç, öbürü) ve dayandığı madde. Bugünkü mahal
  sınıflamasından (ADR 8030) türer, elle düzeltilir. Tablo bu çokgenlerden kurulur.
- **Mahalin türü:** iç, dış (balkon, teras, açık alan), otopark; kullanımından.
- **Mahalin hacmi:** tabanı döşemenin üstü; üstü tavan, döşeme altı ya da çatı yüzü. Eğik çatı altında kesilir. Galeri ve
  dubleks tek mahaldir.
- **Çizimde denetim (dışa aktarmadan önce):** aynı katta örtüşen mahal (gerçek çokgenle), kapanmayan mahal, katının kot
  aralığında olmayan mahal, numarası yinelenen mahal.
- **Gruplar:** bağımsız bölüm, eklenti, ortak alan (bina programı, ADR 0773 ve 8031); yangın kompartımanı (BIM-13).

**İlk kabul:** iki bağımsız bölümlü örnekte emsale dahil alanların toplamı tablonun emsale esas alanına eşit (bağımsız
başvuru). Balkonun %20 sınırını aşan kısmı dahil alana geçer. Kırma çatı altındaki mahalin hacmi prizmadan küçük ve kapalı.

#### BIM-09 — Eksik yapı elemanları [ ]

**Kullanıcı sonucu:** Konut ve karma kullanımlı bir binada karşılaşılan her eleman modelde kendi nesnesidir ve kendi
sınıfıyla dışa gider. "Modele girmeyen nesne" listesi boşalır. Dayanak EK-5 Tablo 5.1; EK-6 §6.1.3 ve 6.14, 6.20, 6.21,
6.35, 6.40, 6.49, 6.53, 6.64; Z5.

- **Rampa:** gövdesi (eğik döşeme, sahanlıklar) kesit modelinde; kesitte, görünüşte ve 3B'de.
- **Giydirme cephe:** çizgisi, yüksekliği, dikme ve kayıt aralıkları, panelleri; planda, görünüşte, kesitte; doğrama
  listesinde.
- **Kaplama ve asma tavan:** mahalin kaplamalarından ve asma tavanından (ADR 8038, 8040) türeyen nesne; elle de çizilir.
- **Baca:** çatı bacası (ADR 8044) ve duvardaki baca tek yapı elemanı.
- **Asansör, vitrifiye, mobilya:** kitaplık bloğunun yapı sınıfı ve gövde ölçüleri. Sınıfı olmayan blok yapı elemanı
  sayılmaz.
- **Dikme ve benzeri** (pergola dikmesi, cephe kaydı).
- Temel ve öbür statik elemanlar yazılmaz; statik modelden altlık gelir (BIM-17).
- Her araç: komut, katalog, ≥ 20 ortak durum, Python, MCP; kendi ikonu, şerit yeri, kısa adı, terim sözlüğü.

**İlk kabul:** rampalı, giydirme cepheli, asansörlü örnek binada dışa aktarmanın kayıp listesi boş. Her yeni eleman planda,
kesitte, 3B'de ve çizelgede.

#### BIM-10 — Geometri doğruluğu [ ]

**Kullanıcı sonucu:** Kesit, görünüş, 3B ve dışa aktarılan model aynı gövdeyi gösterir ve o gövde çizilen yapıyla aynıdır:
eğri duvar eğridir, açıklık duvardan kesilmiştir, döşemenin boşluğu boştur. Dayanak Md. 14; EK-8; EK-9/14; Z6.

- Yay duvar ve döner merdiven gerçek yayla; parça sayısı görüntünün işidir, kaynağın değil.
- Açıklığı kesilmiş duvar gövdesi (IFC-06'nın iki çıktısından biri bunu kullanır).
- Döşemede boşluk ve şaft; eğimli döşeme; çatının katmanları ve içe dönen saçak (ADR 8044'ün kalanı).
- Her gövde kapalı ve kendini kesmeyen bir katıdır.
- Hacim ve alan kaynaktan hesaplanır; üçgenlenmiş görüntü ölçü kaynağı değildir (kılavuz §23.3).

**İlk kabul:** eğri cepheli ve farklı kotlu örnekte gövdelerin hacmi bağımsız başvuruya eşit (yay için kapalı formül).
Açıklıklı duvarın hacmi, dolu hacim eksi açıklıklardır. Bütün gövdeler kapalı.

#### BIM-11 — Çok bina, ortak bodrum ve vaziyet [ ]

**Kullanıcı sonucu:** Bir parselde birden çok blok ve ortak bodrum tek projede durur. Her bina kendi kodu, katları ve
künyesiyle ayrı model dosyası verir; saha ve parsel bilgileri hepsinde aynıdır. Dayanak EK-2 Tablo 2.1; EK-6 6.7, 6.55;
EK-7'nin "ortak öznitelikler aynı" notu; Z7.

- **Bina:** kodu, adı, kullanım amacı, kat takımı, ±0.00'ı. Her yapı nesnesinin binası bellidir.
- **Ortak bodrum:** hangi binanın katı sayılacağı açık bir seçimdir; alanı iki kez sayılmaz.
- Ruhsat takımı ve dışa aktarma bina seçer; pafta ve dosya adlarında bina kodu.
- **Saha:** parsel, çekmeler, ±0.00 ve röper (ADR 8032) bütün binalara ortaktır.
- **Onaylı plan:** parseli etkileyen plan kararları (fonksiyon, nizam, yaklaşma sınırı, emsal, TAKS, yükseklik) CBS
  tarafından kaynağı, onay tarihi ve plan notuyla gelir. Plan gösterimi ya da bir kesişim tek başına hukuki hesap kuralı
  olmaz; çelişen iki karar kullanıcıya gösterilir.

**İlk kabul:** iki bloklu, ortak bodrumlu örnekte iki model dosyası; aynı adı taşıyan iki "1. kat" karışmaz; proje ve saha
bilgileri iki dosyada birebir aynı; toplam alan tekrarsız.

#### BIM-12 — Tipler ve katmanlı yapı [ ]

**Kullanıcı sonucu:** Her eleman türünün tipi vardır. Bir değerin tipten mi örnekten mi geldiği görünür. Tip değişince
örnekler izler; örnekte ayrılan değer korunur. Dayanak Z11; W2 §2.1.

- Döşeme, kolon, kiriş, çatı, merdiven, korkuluk, kaplama ve giydirme cephe tipleri; duvar stili ve kapı/pencere tipiyle
  aynı düzende.
- "Tipten" ve "örnekten" işareti; önce çiz, beğenince tipe çevir.
- Katmanlı yapı: duvar, döşeme, çatı ve kaplamada sırası, kalınlığı, malzemesi ve çekirdeği belli katmanlar.
- Uyum için şart değildir (EK-6 tip istemez); kalite ve hız işidir, sırası sondadır.

**İlk kabul:** kolon tipinin ölçüsü değişince 16 kolon izler, örnekte ayrılan biri izlemez. Öznitelikler'de değerin kaynağı
yazılıdır.

### 5.4 Denetim

#### BIM-13 — Mevzuat denetimlerinin tamamlanması [ ]

**Kullanıcı sonucu:** Denetim, başvuru tarihinde yürürlükte olan kuralları uygular. Her bulgu geçti, kaldı, eksik veri,
uzman incelemesi ya da kapsam dışıdır. Yorum isteyen konu kendiliğinden "uygun" sayılmaz. Dayanak Z9; Ek A.

- **Kural seti** başvuru tarihiyle seçilir (PAİY'nin 1.7.2026 değişikliğinden önce ve sonra gibi); bölge türü; plan notu
  katmanı (ulusal → belediye → parsel).
- **Kalan kurallar:** iç yükseklik; kapı açılış yönü; emsalin 22. madde sınırları (%20, %50, 70 m² …) ve %30'u; kat ara
  toplamları; merdiven altı 1,80 m (ADV-22 ve ADV-23'ün kalanları).
- **Yangın:** kompartıman sınırı ve üstündeki açıklık; kullanım sınıfına göre kişi yükü; beyan edilen dayanımın (BIM-07)
  kuralın istediğiyle karşılaştırılması.
- **Erişilebilirlik:** parselden girişe, asansöre ve erişilebilir tuvalete kesintisiz yol; rampa, sahanlık, net geçiş. TS
  9111'in değerleri TSE metninden doğrulanmadan kural olmaz (W4 §2.6).
- **Bulgu durumu genişler:** "eksik veri" ve "uzman incelemesi". Uzman onayı kayıtlıdır; etkilediği nesne değişince eskir.
- **Konut dışı kullanımlar** (1.9.2033): kullanım sınıfına özel kural paketleri ayrı dilimlerde.

**İlk kabul:** her yeni kural için bir geçen, bir kalan durum, KentOS kodu olmadan yazılmış başvuruyla. Başvuru tarihi
30.6.2026 ve 2.7.2026 olan aynı çizim farklı kural setiyle denetlenir. Yağmurlama beyanı kaldırılınca kaçış uzaklığı
bulgusu değişir.

### 5.5 Teslim ve bilgi yönetimi

#### BIM-14 — BIM ruhsat paketi: IFC + PDF/A + kontrol formu [ ]

**Kullanıcı sonucu:** Tek komut, aynı model revizyonundan mimari modeli, bütün PDF/A paftaları, çizelgeleri ve kontrol
formunu üretir. Hepsi yazıldıktan sonra dosyadan denetlenir; biri kalırsa hiçbir şey yayımlanmaz. Dayanak Md. 9/1, 15; e-PYS
Md. 4/2, 6; Z4.

- **Teslim girdisi:** belge revizyonu, kural kataloğunun ve kurum profilinin sürümü, bina ve kat seçimi, altlık modellerin
  özetleri. Değişmez.
- **Akış:** hazırlık klasörü → dosyadan denetim ([IFC-08](ifc-uyum.md#10-iş-paketleri) ve BIM-05) → müellifin form
  incelemesi → yayımlanmış revizyon.
- **Form:** resmî Evet/Hayır ile KentOS'un kanıt durumu ayrı tutulur. Elle verilen cevap, etkilediği dosya ya da profil
  değişince eskir. Form kalıcı bir kaynak türü olur (bugün başlık satırından tanınıyor).
- **Teslim listesi:** her dosyanın adı, boyutu, SHA-256 özeti, üretildiği revizyon, denetim sonuçları, araç ve katalog
  sürümleri. İmzalanacak nüshalar ve özet listesi hazırdır; imzanın kendisi müellifin aracındadır.
- Komut, Python ve MCP aynı işi yapar. İptal ve hata kısmi paket bırakmaz; önceki paket erişilebilir kalır.

**İlk kabul:** bölüm binasında ve 2+1'de paket. Bir pafta eski revizyondan kaldığında paket durur ve sebebini söyler. İki
bağımsız model okuyucusu ve bağımsız PDF/A doğrulayıcısı hatasız.

#### BIM-15 — Bilgi yönetimi ve teslim kaydı [ ]

**Kullanıcı sonucu:** Projenin hangi revizyonu, hangi dosyalarla, ne zaman, kime teslim edildi ve ne cevap geldi sorusunun
cevabı projede kayıtlıdır. İade gerekçesi ilgili nesneye bağlıdır. Dayanak Md. 4/4; e-PYS Md. 4–7; Z12.

- **Durumlar:** çalışma, paylaşım, yayın, arşiv (TS EN ISO 19650'nin ortak veri ortamı); EK-2'nin revizyon kodu.
- **Teslim kaydı** bulut projesinde: paketin özeti, yayımlayan, tarih, kurum, cevap (kabul, iade), gerekçe, varsa sistemin
  makbuz numarası.
- **İade ve inceleme notu** nesneye bağlı bir sorun kaydıdır (mevcut gözden geçirme notları). Dış değişim için açık sorun
  biçimi IFC-10'dadır.
- **Sorumluluk ve gizlilik:** disiplin, müellif, sicil; kimlik bilgilerinde role göre görünürlük (6698 sayılı Kanun). Gerçek
  proje verisi açık test korpusuna girmez.
- **e-PYS:** resmî arayüz, yetki ve deneme ortamı yayımlanınca pilot. O güne kadar müellifin yetkili kanaldan yükleyeceği
  paket hazırlanır.

**İlk kabul:** bir paket yayımlanır; "iade" cevabı gerekçesiyle girilir; gerekçe iki nesneye bağlanır; düzeltilmiş ikinci
paket öncekiyle karşılaştırılır. Yetkisiz kullanıcı künyeyi göremez.

#### BIM-16 — Büro ve belediye pilotu [ ]

**Kullanıcı sonucu:** Bir konut ve bir ticaret+konut projesi gerçek bir büroda baştan sona KentOS'ta hazırlanır ve hedef
idarenin yazılı kabul ya da iade cevabını alır.

- Sıra: tek parsel konut → ticaret+konut ve ortak bodrum → çok bloklu parsel.
- Yetkili müellif incelemesi; gerçek pencerede akış (ADR 8000); aynı dosyayı yeniden üretebilen sürüm kaydı.
- İdarenin yorumları (ad kalıbı, mahal adı, görünüm tanımı, dosya sınırı) kurum profiline kaynağıyla girer.

**İlk kabul:** idarenin yazılı cevabı; cevaptaki her madde kataloğa ya da bir iş paketine bağlanmış.

### 5.6 Disiplinler ve idare

#### BIM-17 — Dış disiplin modeli ve çakışma [ ]

**Kullanıcı sonucu:** Statik ve tesisat modeli KentOS'ta altlık olarak açılır. Aks, kot ve koordinat uyuşmazlığı ile
çakışmalar nesnesiyle listelenir. Dayanak Md. 9/1 ("disiplinler tarafından ayrı ayrı"); EK-7; Z10.

- Altlık model salt okunur ([IFC-09](ifc-uyum.md#10-iş-paketleri)); revizyonu ve özeti kayıtlıdır.
- **Hizalama:** koordinat sistemi, harita dönüşümü, ±0.00, aks etiketleri, kat kotları; projenin ortak alanları.
- **Çakışma:** mimari duvar ve açıklık ile statik kolon, kiriş, perde; döşeme boşluğu ile şaft; geçişler. Kutu ön
  süzgeçtir; kararı gerçek gövde verir.
- Bulgu kaynağına döner (nesne, disiplin, revizyon) ve açık sorun biçimiyle dışa verilir (IFC-10).

**İlk kabul:** bölüm binasının bağımsız üretilmiş statik modeliyle hizalama. Bilerek kaydırılan bir kolon ve bir kat kotu
bulgu olur. Farklı düşey datum, geometri denetiminden önce durdurur.

#### BIM-18 — İdare inceleme çalışma alanı [ ]

**Kullanıcı sonucu:** Belediyenin proje inceleyeni gelen paketi (model + PDF/A + kontrol formu) KentOS'ta açar. Biçim
denetimi kendiliğinden koşar. İmar durumuyla karşılaştırma belediyenin kendi plan ve parsel verisiyle yapılır. Eksikler
gerekçesiyle ve nesnesiyle iade notuna dönüşür. Dayanak e-PYS Md. 4/3, 4/5, 5/2–3, 7/7; Z10.

- **Paket ön denetimi:** beş kabulün ilk dördü (IFC-08) ve PDF/A; müellifin formu ile KentOS'un ölçtüğü yan yana.
- **Görüntüleme:** 3B ve kat kat plan; mahal, bağımsız bölüm ve emsal alanı süzgeçleri; pafta ile model yan yana.
- **İmar denetimi:** parsel, çekme, TAKS, emsal, kat adedi ve yükseklik; belediyenin CBS'sindeki onaylı plan ve parselle.
  Kaynağı ve onay tarihi görünür.
- **İade notu:** madde, ölçülen ve istenen değer, nesne; yazıyla ve açık sorun biçimiyle.
- **e-PYS:** belediyenin yazılımında yapılan işlem e-PYS'ye aktarılır; arayüz yayımlanınca bağlanır.
- **Yetki ve gizlilik:** müellifin verisi üçüncü kişiye verilmez, çoğaltılmaz; erişim kaydı tutulur.

**İlk kabul:** başka bir yazılımın ürettiği açık örnek model ve KentOS'un kendi paketi açılır. Bilerek bozulan paket (yanlış
dilim, eksik yangın alanı, farklı revizyonlu pafta) doğru katmanda bulgu verir. İade notu nesneyi seçer.

### 5.7 Genişleme ve borç

#### BIM-19 — Kat irtifakı için 3B yapı modeli (araştırma önce) [ ]

Tapu ve Kadastro Genel Müdürlüğünün kat irtifakı ve kat mülkiyetinde istediği üç boyutlu sayısal yapı modeli aynı bina
modelinden üretilebilir mi? Genelgenin güncel metni, biçim şeması, ayrıntı düzeyi, bağımsız bölüm ve eklenti kimlikleri,
sorumlu meslek ve doğrulama servisi birincil kaynaktan okunur (araştırma notu §9). Karar ADR'ye yazılır: yapılır, ertelenir
ya da kapsam dışı. Kaynak okunmadan kod yazılmaz.

#### BIM-20 — Mimari kullanım borcu [~]

`architecture-advanced.md`'nin kalanları her fazda bir dilimle kapanır (Z13, Z14):

- hız: 2+1 dairede ≤ 80 etkileşim ve ≤ 100 s;
- çokgen oda ve odaya çift tık; kot etiketi ve çakışmasız yerleşim; 1:20 kasa profili ve perde taraması;
- tek tuş kısayol (sahibin kararı), yüzer pencere, örnek proje ve malzeme bağı, seçenekler arası eşleme;
- gerçek ofis DWG'siyle tadilat kabulü (sahibin dosyası); `kullan --modul` ve `snapshot --sekme`.

Yeni araçlar ADV-08…10'un kapılarından geçer: kendi ikonu, kısa adı, şerit yeri, terim sözlüğü, seçenek harfi.

## 6. Uygulama sırası (B1…B23)

Her satır bir ADR, bir commit, sahibe gönderilen resimler ve bir rapordur. IFC paketleri [eş planda](ifc-uyum.md#10-iş-paketleri).

### Faz 0 — Kaynak ve kapsam

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B1 | Ekler sayfa sayfa kataloğa; başlangıç ölçümü | BIM-01, IFC-01 | Her kural sayfasıyla; ölçüm raporu |
| B2 | Tarihli kapsam kararı | BIM-02 | Ruhsat takımında "bu projede ne, ne zaman" |

### Faz 1 — 2B teslim tam (yasal en geç 1.9.2027; Sivas 1.3.2029)

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B3 | EK-3 ve EK-4 denetimi | BIM-03 | Uymayan katman ve mahal adı bulguda |
| B4 | Paftanın modelden üretildiği kanıtı | BIM-04 | Eski türevle yayın durur |
| B5 | 2B ruhsat paketi | BIM-05 | Bağımsız denetimden geçmiş PDF/A takımı ve teslim listesi |

### Faz 2 — Ruhsat düzeyinde model

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B6 | Binanın 3B görünümü | BIM-06 | Bina 3B'de; tıkla, planda seç |
| B7 | Yangın, performans ve malzeme; modelin zorunlu alanları | BIM-07, IFC-02 | Formun 9. sorusu evet olabilir |
| B8 | Mahal, zon ve emsal; mekânsal zonlar ve gruplar | BIM-08, IFC-03 | Emsal tablosu ile model aynı alanlardan |
| B9 | Eksik yapı elemanları; eksik sınıflar | BIM-09, IFC-03 | Kayıp listesi boş |
| B10 | Geometri doğruluğu; geometri ve görünüm tanımı | BIM-10, IFC-06 | Eğri eğri, açıklık kesik |
| B11 | Çok bina; konum, kot ve çok bina | BIM-11, IFC-05 | İki blok, iki model dosyası |

### Faz 3 — Doğrulama ve teslim (yasal en geç 1.9.2028 e-PYS; 1.9.2029 ilk IFC)

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B12 | STEP okuyucu çekirdeği | IFC-07 | Yazılan dosya geri okunur |
| B13 | Yazılmış dosyadan doğrulama, IDS ve kontrol formu | IFC-08 | 21 sorunun kanıtı dosyadan |
| B14 | BIM ruhsat paketi | BIM-14 | Tek komut, tek revizyon, tek paket |
| B15 | Mevzuat denetimlerinin tamamlanması | BIM-13 | Başvuru tarihine göre kural seti |
| B16 | Bilgi yönetimi ve teslim kaydı | BIM-15 | Teslim ve iade kayıtlı |
| B17 | Büro ve belediye pilotu | BIM-16, IFC-11 | İdarenin yazılı cevabı |

### Faz 4 — Disiplinler ve idare (yasal en geç 1.9.2029)

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B18 | Altlık model | IFC-09 | Statik model planda ve 3B'de altlık |
| B19 | Dış disiplin ve çakışma; karşılaştırma ve sorun değişimi | BIM-17, IFC-10 | Çakışmalar nesnesiyle |
| B20 | İdare inceleme çalışma alanı | BIM-18 | Gelen paket açılır, denetlenir, iade notu çıkar |

### Faz 5 — Kalite ve genişleme

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| B21 | Tipler ve katmanlı yapı; tip nesneleri | BIM-12, IFC-04 | Değerin kaynağı görünür |
| B22 | Konut dışı kullanımlar (yasal en geç 1.9.2033) | BIM-13 | Kullanım sınıfına özel kurallar |
| B23 | Kat irtifakı için 3B yapı modeli araştırması | BIM-19 | Karar ADR'de |

- **Sürekli:** BIM-20 her fazda bir dilimle ilerler.
- **Bağımsız olanlar:** B15 hiçbir model paketine bağlı değildir, istenen fazda araya girer. B12 yalnız B1'e bağlıdır.
- **Sahibin seçimi:** öncelik belediyenin inceleme tarafıysa B12 → B18 → B20 öne alınır; müellif tarafının sırası
  değişmez.

## 7. Kabul ölçüleri ve kapılar

| Ölçü | Bugün | Hedef | Kapı |
|---|---|---|---|
| Kapsam kararı | yok | ≥ 20 durumda doğru | 1 |
| PDF/A, bağımsız doğrulayıcı | ölçülmedi (kendi denetimi var) | 0 hata | 1 |
| Model ile paftaların revizyonu | kayıtlı değil | aynı ve kayıtlı | 1, 3 |
| Yazılan zorunlu sınıf (29'dan; temel ve tabela dışında 27) | 15 | 27 | 2 |
| Modele girmeyen nesne (rampalı, giydirme cepheli, asansörlü örnek) | rampa ve sınıfsız bloklar | 0 | 2 |
| EK-6 ve EK-7'nin zorunlu satırlarından dosyada bulunan | B1'de ölçülecek | hepsi dolu ya da gerekçesiyle "eksik" | 2 |
| 3B'de görülen gövde / dosyadaki gövde | 0 / 68 | eşit | 2 |
| Şema kuralları, bağımsız okuyucu | 0 bulgu (bölüm binası) | 0 bulgu, her örnekte | 2, 3 |
| Türkiye profili, bağımsız ikinci okuyucu | ölçülmedi | 0 hata | 3 |
| Kontrol formunda dosyadan kanıtlanan soru | 0 / 21 | makinece denetlenebilenlerin hepsi | 3 |
| Büyük modelde (100 daire) dışa aktarma süresi ve belleği | ölçülmedi | B13'te taban, sonra bütçe | 3 |
| 2+1 daire, gerçek pencere | 82 etkileşim, 121,3 s | ≤ 80, ≤ 100 s | sürekli |

| Kapı | Yasal en geç | Koşul |
|---|---|---|
| 1 — 2B teslim | 1.9.2027 (Sivas 1.3.2029) | B1–B5; paket bağımsız doğrulayıcıdan hatasız |
| 2 — Ruhsat düzeyinde model | IFC'nin ön koşulu | B6–B11; kayıp listesi boş; zorunlu alanlar dolu ya da "eksik" diye listeli |
| 3 — BIM teslim | 1.9.2029 (Sivas 1.9.2032) | B12–B17; beş kabulün ilk dördü dosyadan; pilot idarenin yazılı cevabı |
| 4 — Disiplinler ve idare | 1.9.2028 – 1.9.2029 | B18–B20 |
| 5 — Bütün binalar | 1.9.2033 | B22 |

Her kapıda `architecture-advanced.md` §7'nin faz kapısı da koşulur: kabul izleri gerçek pencerede, resim matrisi sahibe,
sahibin kısa listesi.

## 8. Açık sorular ve riskler

| Soru | Kim cevaplar | Etkilediği |
|---|---|---|
| Mahalin modeldeki adı: EK-5 §5.5.4 mü, yalın EK-3 numarası mı | Ek sayfası (B1); gerekirse idare | IFC-02 |
| Görünüm tanımı; PDF/A'nın bölümü ve düzeyi; dosya boyutu sınırı | Bakanlığın kılavuzu ya da idare | IFC-06, BIM-05 |
| e-PYS'nin arayüzü, imza yöntemi, yükleme biçimi ve tarihleri | Bakanlık (e-PYS Md. 4/4, 5/4, 5/6) | BIM-15, BIM-18 |
| Ortak bodrum hangi binanın; bir parselde kaç model dosyası | İdare | BIM-11, IFC-05 |
| 2B sembol donatı (mobilya) modele girer mi | EK-8; idare | BIM-09 |
| Gruplama sınıfının (zon) neyi temsil edeceği | Ek sayfası | BIM-08 |
| Nüfus hangi yıla ve kaynağa göre | Bakanlık ya da idare | BIM-02 |
| TS 9111 ve öbür TSE metinlerindeki ölçüler | TSE (ücretli metin) | BIM-13 |
| Pilot büro ve idare | Sahip | BIM-16 |
| İdare tarafı öne alınsın mı | Sahip | §6 |

- **Ekler yanlış okunursa profil yanlış olur.** Önlem: B1 her şeyden önce; iki bağımsız okuma; her satırın sayfası.
- **Bakanlık biçimi ve tarihi değiştirebilir** (e-PYS Md. 4/4). Önlem: K2.
- **Başka yazılımların dosyaları profile uymayabilir** (Estonya'nın dersi; bağımsız aracın açık örneklerdeki ölçümü,
  araştırma notu §6 ve §8). Önlem: idare tarafında hoşgörülü okuyucu, açık bulgu.
- **"Modelden üretildi" sözü 2B öncelikli bir araçta sorgulanır.** Önlem: BIM-04'ün kaydı ve yayın kapısı.
- **Büyük model.** Önlem: süre ve bellek tabanı B13'te; bütçe ondan sonra (kılavuz §6).

## 9. Kurallar

- **Madde talimatı:** `architecture-advanced.md` §6. Araştırma her dilimde yenilenir; değer uydurulmaz, kaynak beklenir.
- **IFC:** IFC'ye ait her şey `crates/shared/ifc`'tedir; uygulamada yalnız bağdaştırıcı kalır (kılavuz §14).
- **Araç = komut:** çizimi değiştiren her iş komut, katalog, ≥ 20 bağımsız durum, Python, MCP ve manifestle biter.
- **Adlar:** arayüzde, kodda ve yorumlarda başka uygulama anılmaz; karşılaştırma yalnız ADR'de, planda ve araştırma notunda.
- **İddia:** "yönetmeliğe uygun" denmez; beş kabulden hangisinin, hangi profil sürümüyle, hangi örnekte sağlandığı söylenir.
  İdarenin kabulü yazılı cevaptır.
- **Veri:** gerçek ruhsat modeli ve müellif bilgisi dış servise ve açık test korpusuna gitmez.
- **Bu planın kabulü** uygulamanın üretim kabulü değildir.

## Ek A — Tasarımı etkileyen mevzuat

İlk taslaktan aktarıldı; bu turda yeniden açılmadı. Ayrıntı ve işaretler W4'tedir. Her kaynak uygulama anında proje türü,
yeri, geçiş hükümleri ve değişiklikleriyle yeniden değerlendirilir.

| Kaynak | Ürün davranışı | Kabul kanıtı |
|---|---|---|
| 3194 İmar Kanunu (4, 8, 21–23, 26–27) | Parsel, plan, ruhsat türü, yetkili idare, proje evrakı | Resmî imar durumu ve başvuru türüyle izlenebilir karar. [Kanun](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.3194.pdf) |
| Mekânsal Planlar Yapım Yönetmeliği | Onaylı uygulama imar planı, plan notları ve raporundan fonksiyon ve yapılaşma zarfı; planın numarası, onayı, kaynağı | Birden çok plan değişikliği olan parselde onay tarihi ve not maddesiyle yeniden üretilebilir sonuç. [Konsolide metin](https://www.lexpera.com.tr/mevzuat/yonetmelikler/YR801Y2014N29030P1/3) · [Plan gösterimleri](https://mpgm.csb.gov.tr/plan-gosterimleri-4926) |
| Planlı Alanlar İmar Yönetmeliği (4–5, 19–23, 26–42, 57–58) | Alan tanımları, TAKS ve emsal, çekmeler, kot, piyes, merdiven, asansör, rampa, çatı, çıkma, ışıklık, ruhsat çizimleri; 1.7.2026 değişikliği ayrı profil | Bağımsız alan hesabı; istisna kalemleri maddesiyle. [Konsolide metin](https://www.lexpera.com.tr/mevzuat/yonetmelikler/planli-alanlar-imar-yonetmeligi-4) · [1.7.2026 değişikliği](https://www.resmigazete.gov.tr/eskiler/2026/07/20260701-7.htm) · [Sivas](https://www.resmigazete.gov.tr/eskiler/2022/11/20221110-3.htm) |
| Binaların Yangından Korunması Hakkında Yönetmelik | Kullanım ve kullanıcı yükü, kompartıman, malzeme ve eleman dayanımı, kaçış yolu, korunumlu merdiven, çıkış kapısı | En uzak nokta, net geçiş, kapasite, rota ve istisnanın görsel kanıtı; yangın uzmanı örnekleri. [Bakanlık kılavuzu](https://webdosya.csb.gov.tr/v2/meslekihizmetler/2026/05/Binalar-n-Yang-n-Korunmas-Hakk-nda-Y-netmelik-K-lavuzu-20260507112134.pdf) |
| 5378 ve erişilebilirlik hükümleri; TS 9111 | Parselden erişilebilir girişe ve mahallere güzergâh, rampa, sahanlık, net kapı, dönüş alanı, asansör, tuvalet | Standardın baskısı ve maddesi; bağımsız boyut örnekleri. [5378](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.5378.pdf) |
| Otopark ve Sığınak Yönetmelikleri | İhtiyaç, erişilebilir park, geometri, araç rampası; sığınak alanı ve kapasitesi | Karma kullanım, bodrum ve istisnayı içeren elle hesap. [Otopark](https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=24408&MevzuatTertip=5) · [Sığınak 2025](https://www.resmigazete.gov.tr/eskiler/2025/11/20251107-2.htm) |
| Binalarda Enerji Performansı Yönetmeliği; TS 825 | Isıl zarf, koşullandırılmış mahal, katman ve beyanlı U değeri; enerji hesabı mühendisin | Aynı dış zarfın mimari ve enerji modelinde eşleşmesi. [BEP](https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=13594&MevzuatTertip=5) · [TS 825 tebliği](https://www.resmigazete.gov.tr/eskiler/2025/02/20250220-2.htm) |
| 634 Kat Mülkiyeti Kanunu (2–4, 12) | Bağımsız bölüm, eklenti, ortak yer, arsa payı, proje listesi | Dubleks, ortak merdiven ve eklenti örneğinde alan tekrarsız. [Kanun](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.634.pdf) |
| Türkiye Bina Deprem Yönetmeliği; 4708 ve Yapı Denetimi Uygulama Yönetmeliği | Aks, kot, taşıyıcı sınırı; kontrol formu ve müellif rolleri | Çakışma ve müellif kanıtı; deprem hesabı statik disiplinde. [4708](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.4708.pdf) |
| 5070 ve 6698 sayılı Kanunlar | Güvenli e-imza; kişisel veri | [5070](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.5070.pdf) · [6698](https://www.mevzuat.gov.tr/mevzuatmetin/1.5.6698.pdf) |

## Ek B — Model sözleşmesi

İlk taslaktan aktarıldı; hangi paketin kurduğu eklendi. Yeni `.kcad` alanları sürümlenir ve eski dosyanın okunması sınanır.
Kaynağı bilinmeyen sınıflandırma, performans ya da datum kullanıcıdan teyit bekler.

| Veri grubu | Asgari alanlar ve ilişkiler | Davranış | Paket |
|---|---|---|---|
| Proje, parsel, saha, bina, kat | Kalıcı kimlik; resmî ve yerel kod; yapı ve başvuru türü; plan profili; yatay ve düşey sistem; röper, ±0.00; bina ve kat sırası | Bir sahada birden çok bina, ortak bodrum, farklı kot. Her elemanın mekânsal sahibi belli | BIM-02, BIM-11 |
| Aks ve başvuru düzlemleri | Etiket, geometri, bina, kat ya da çatı düzlemi | Ölçülen değer ile hukuki sınır ayrı alanlar. Kot değişince bağlı elemanların raporu ve geri alma | BIM-11 |
| Duvar, kolon, kiriş, döşeme, çatı | Tip ve örnek; katman sırası; başvuru yüzü; alt ve üst kot; malzeme; taşıyıcı, dış ve zarf durumu; bağlantılar | Kesim ve birleşim tek geometri hesabından. Statik bilginin kaynağı belirsizse teyit istenir | BIM-07, BIM-10, BIM-12 |
| Kapı, pencere, açıklık | Duvarı; boşluğun profili; kasa, kanat ve yön; eşik; nominal ve net ölçü; parapet ve başlık; çıkış rolü; yangın, duman ve ısı performansı ile belgesi | Açıklık duvarı oyar, kapı ve pencere onu doldurur. Duvar taşınınca, silinince, bölününce, kopyalanınca bağlıları tek adımda izler | BIM-07 |
| Mahal, bağımsız bölüm, zon | Kapalı sınır ve hacim; alan türü; kot; kullanım; kaplamalar; komşuluk; kullanıcı yükü; ortak yer ve eklenti; yangın ve enerji zonu | Kendiliğinden bulunan sınır ile elle çizilen ayırıcı ayrı kökenlidir. Atrium, dubleks ve açık balkonda kat alanı tekrarlanmaz | BIM-08 |
| Merdiven, rampa, şaft, asansör, korkuluk | Alt ve üst kat; kol, sahanlık, eğim; net genişlik ve baş yüksekliği; boşluk ve korunum; kaçış ve erişilebilirlik rolü | Tek nesneden plan oku, kesit, 3B ve dolaşım bağlantısı | BIM-09, BIM-13 |
| Kaplama, donatı, tesisat rezervasyonu | Tip; ana eleman; işlev; hacim; disiplin sahibi; görünürlük; geçiş ve denetim durumu | Sembolik donatı, gerçek rezervasyon ve yetkili mühendislik nesnesi ayrı anlam taşır. Çakışma bulgusu kaynak nesneye döner | BIM-09, BIM-17 |

## Ek C — Olumsuz kabul örnekleri

İlk taslaktan aktarıldı. Her biri ilgili paketin bağımsız başvurusunda bir "kalan" durumdur.

| Konu | Örnek | Beklenen | Paket |
|---|---|---|---|
| İmar ve alan | Aynı balkon hem emsal harici hem ortak alan toplamında düşülmüş | Çift sayım gösterilir; etkilenen çokgen seçilir | BIM-08 |
| Yangın ve kaçış | Ayrı görünen iki çıkışın ortak dar boğazı; kapalı geçiş; yanlış yağmurlama beyanı | Üçü ayrı bulgu. Dayanım ve duman sızdırmazlık beyanı kaynak belgesinden | BIM-07, BIM-13 |
| Erişilebilirlik | Rampanın eğimi uygun, kapının açılışı sahanlığı kapatıyor | İkisi birlikte ölçülür | BIM-13 |
| Disiplinler | Aynı koordinat etiketi farklı yükseklik datumlarını gizliyor | Çakışma denetiminden önce dönüşüm doğrulaması istenir | BIM-17 |
| Bilgi bütünlüğü | PDF/A teknik doğrulamadan geçiyor, ama kat planı başka revizyondan | Paket durur | BIM-04, BIM-14 |
