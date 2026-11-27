# ADR 9334: Ağ analizleri işlem geçmişine girer, Yeniden aç aynı ayar ve noktalarla açar, açtıkları katman neyden üretildiğini taşır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  18, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:**
  - [ADR 0209](0209-network-analysis.md) (ağ analizi);
  - [ADR 0529](0529-processing-batch-and-run-record.md) (çalışmanın kaydı);
  - [ADR 0633](0633-dataset-metadata-and-lineage.md) §2 (türetim kaydı);
  - [ADR 9179](9179-buffer-cases-and-the-run-in-words.md) (işin sözleri);
  - [ADR 9333](9333-service-area-reach-and-buffer-side.md) (hizmet alanının çıktıları).

## Bağlam

- **Gözlem:**
  - Tampon araç kutusundan çalıştı; Nesneler girdisi Tampon'a bağlanan yeni model `qa_noktalar` ile tek adımda üç çıktı
    üretti. İşlem geçmişi ikisini gösterdi, modelde Yeniden aç vardı.
  - En kısa yol, Hizmet alanı ve Şebeke izleme sonuçları Komut geçmişinde göründü; işlem geçmişinin kart sayısı
    artmadı.
- **İncelemenin kabulü:**
  - Etkileşimli analizler de aynı işlem kaydı sözleşmesine girsin. Girdi katmanı ve ağ kimliği, eşik ve birim, çıktı
    katmanı ya da seçilen nesneler ve işlem durumu tutulsun.
  - Yeniden aç girdileri ve ayarları doldursun.
  - Tarih, parametre, çıktı ve kaynak kimliği proje yeniden açılınca doğrulansın.
- **Kod:**
  - Ağ araçları sonuçlarını kendi geri alma adımlarıyla yazıyordu (`write_results`); işlem çalıştırıcısının kaydına
    uğramıyorlardı.
  - İşlem geçmişi oturumluktur (ADR 0529'un açığı). Kalıcı olan, işlem aracının açtığı katmanın türetim kaydıdır (ADR
    0633 §2).

## Araştırma

- **QGIS, işlem geçmişi:** her çalıştırmanın tarihi ve parametreleri tutulur. Kayıt komut olarak saklanır, çift tıkla
  araç penceresi aynı parametrelerle açılır
  ([The history manager](https://docs.qgis.org/latest/en/docs/user_manual/processing/history.html)).
- **ArcGIS Pro, geoprocessing history:**
  - Çalıştırılan araçlar projeyle saklanır. Proje sonra açılınca geçmiş yerindedir; kayıt aynı ayarlarla yeniden
    açılır.
  - İşlemler isteğe göre veri kümesinin üst verisine de yazılır
    ([Geoprocessing history](https://pro.arcgis.com/en/pro-app/latest/help/analysis/geoprocessing/basics/geoprocessing-history.htm)).
- **KentOS kararı:**
  - Ağ analizleri, işlem araçlarının çalışma kaydını (`RunRecord`) ve türetim kaydını (`LayerDerivation`) kullanır;
    ayrı bir kayıt biçimi kurulmaz.
  - Kalıcılık, üretilen katmanın türetim kaydıyla sağlanır (ArcGIS'in üst veriye yazmasının karşılığı). Oturum geçmişinin
    proje dosyasında saklanması ADR 0529'daki gibi açık kalır.

## Karar

### 1. Ağ analizinin çalışması

En kısa yol ve Hizmet alanı sonucu yazınca, Şebeke izleme seçince bir çalışma bırakır (`AnalysisRun`, araçtan
`ViewChange::Ran` ile). Çalışmada şunlar vardır:

- aracın komutu (`tool.netRoute`, `tool.netServiceArea`, `tool.netTrace`) ve adı;
- değerleri: ağın kimliği, maliyet (izlemede yok), duraklar ya da tesisler ya da başlangıçlar (`[doğu, kuzey]`),
  engeller, saat, araç profili ve gün. Araca göre ek değerler:
  - Hizmet alanı: aralıklar (maliyetin biriminde, uzunlukta metre), kenar payı, biçim, yön, birleşik, çizgiler;
  - En kısa yol: Sıra;
  - Şebeke izleme: Tür;
- sözleri, işlem araçlarınınki gibi. Örnek: “Ağ: qa_yollar ağı (ag-1)”, “Maliyet: Uzunluk, metre”, “Tesisler: 1”,
  “Aralıklar: 35 m”, “Kenar payı: 50.000 m”, “Çıktı katmanı: “Yaklaşık hizmet alanı” (yeni katman)”;
- yazdığı nesneler (`added`) ya da seçtikleri (`touched`), ağın okunan nesne sayısı ve zamanı.

Masaüstü çalışmayı işlem geçmişine işlem araçlarının kaydıyla ekler (`RunRecord`, durum Tamam). Kartta süre yerine
“Etkileşimli” yazar; “N nesneyi seç” yazılanları ya da seçilenleri seçer.

### 2. Yeniden aç

- Ağ analizinin kartında Yeniden aç aracı başlatır ve değerleri geri verir (`Tool::reopen`):
  - ağ, maliyet, saat, profil, gün ve aracın ayarları belleğe döner;
  - engeller, sonra duraklar sırasıyla, tıklanmış gibi ağa oturtulur;
  - sonuç önizlemede olduğu gibi gelir.
- Aralıklar, yazılanlar gibi denetlenir: artan, sıfırdan büyük, en çok 10. Olmayan ağ ya da maliyet aracın öntanımına
  düşer.
- İpucu: “Aracı aynı ağ, ayarlar ve noktalarla başlatır”.

### 3. Açılan katman neyden üretildiğini taşır

- En kısa yol ve Hizmet alanı'nın açtığı katmanlar (Rota, Yaklaşık hizmet alanı, Ulaşılan yollar) türetim kaydı
  taşır:
  - araç ve adı, sürüm 1, değerleri;
  - ağın çizgi ve düğüm katmanları, yazmadan önce alınan içerik özetleriyle (`sources::layer_digest`);
  - çizim sürümü, motor ve zaman.
- Kayıt aynı geri alma adımında yazılır; geri alma onu katmanla götürür. Var olan katmana yazan analiz kayıt bırakmaz
  (ADR 0633 §2'deki kural).
- Proje yazılıp açılınca kayıt yerindedir. Veri kümesi bilgisi › Soy aracı, zamanı, değerleri ve girdinin durumunu
  gösterir. `cad.datasets.describe` girdiyi `current` der; ağın çizgileri değişirse türev eskir.
- Motorun adı ve zaman biçimi işlem araçlarıyla tek yerdedir (`kentos_native_application::lineage::{engine, utc_at}`).

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Ağ analizinin çalışması, işlem geçmişinde kartı ve Yeniden aç | yok: Mimari'de ağ analizi yoktur | yok: ağ araçları yalnız CBS'dedir | yok: aynı gerekçe | var |
| Analizin açtığı katmanın türetim kaydı | yok: aynı gerekçe | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Aynı analizler işlem araçlarıyla yapılır (`network.shortestPath`, `network.serviceAreas` …, `cad.processing.run`); onların kaydı ve türetim kaydı ADR 0529 ve 0633'teki gibidir. Etkileşimli araç pencerenin işidir. `cad.datasets.describe` analizin türetim kaydını okur. |
| MCP | var | Aynı gerekçe. |
| Yapay zekâ ajanı | var | Aynı gerekçe. |
| Eklenti | var | Aynı gerekçe. |

## Açık

- **Geçmişin kendisi:** işlem geçmişi oturumluktur. Proje açılınca kalıcı olan, katmanların türetim kaydıdır (ADR
  0529'un açığı).
- **Şebeke izleme** katman açmaz; seçimi oturumun kaydındadır, projede kalmaz.
- **Var olan katman:** var olan katmana yazan analiz türetim kaydı bırakmaz.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **CBS paketi:** `cargo test -p kentos-desktop-cbs -- analysis_history_tests`: 2 geçti.
  - `a_service_area_is_a_run_reopened_with_its_values_and_its_layer_keeps_what_made_it`:
    - Hizmet alanı'nın kaydı `tool.netServiceArea`, durum Tamam; değerleri ağın kimliği, Uzunluk, `[35]`, 50, çizgiler
      açık, tesis A;
    - sözleri ağ, maliyet ve birimi, tesis sayısı, aralıklar, kenar payı, iki katman “(yeni katman)”;
    - 3 → 4 kayıt;
    - Yaklaşık hizmet alanı ve Ulaşılan yollar'ın türetim kaydı geçerli (`derivation_problem` yok), girdisi qa_yollar;
    - proje yazılıp okununca kayıt aynı; `cad.datasets.describe` girdiyi `current` diyor, eski türev değil;
    - Yeniden aç Hizmet alanı'nı 35 m, çizgiler açık ve A'daki tesisle başlatıyor: “1 tesis; 3 çizgi, 1 alan”.
  - `a_route_and_a_trace_are_runs_too_and_reopen_with_their_points`:
    - geçmiş yeniden eskiye Şebeke izleme, En kısa yol, Hizmet alanı;
    - En kısa yol'da iki durak ve “Rota” (yeni katman), Rota'nın türetim kaydı;
    - Şebeke izleme'de “Tür: bağlı”, bir başlangıç, üç seçilen nesne, katman yok;
    - Yeniden aç En kısa yol'u A ve B ile (Uzunluk 50.000 m), Şebeke izleme'yi A ile (120.000 m) başlatıyor.
- **Etkileşim katmanı:** `cargo test -p kentos-interaction`: hepsi geçti.
- **Masaüstü:** `cargo test -p kentos-desktop --lib -- processing::`: 134 geçti.
- **İşlem çekirdeği:** `cargo test -p kentos-processing --test cases`: 38 geçti (motor ve zaman yardımcıları ortak yere
  taşındı).
- **İzler:** `cargo test -p kentos-cad`: 37 geçti (ağ araçlarının üç izi dahil).
- **Dosya boyu:** `tool.rs` sınırın altına indi (`Name` ve `Values` `core/kept.rs`'e); `session.rs` küçüldü (aracın
  kancaları `core/session_hooks.rs`'e). Motor ve zaman `lineage.rs`'tedir; `sources/mod.rs` büyümedi.
- **Resim** (`.run/gui-evidence/kullanim-9334/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-gecmiste-analizler`: İşlemler › Geçmiş (3), Şebeke izleme'nin kartı ağın kimliği, türü, seçilenler, “3 nesneyi
    seç” ve Yeniden aç ile;
  - `02-yeniden-ac`: Hizmet alanı kayıttan yeniden açılmış, tesis A'da, ayarlarda 35 m ve “yazılır”;
  - `03-soy`: proje yeniden açıldıktan sonra Yaklaşık hizmet alanı'nın Soy'u: Güncel, araç, zaman, değerler,
    “qa_yollar: aynı”.
