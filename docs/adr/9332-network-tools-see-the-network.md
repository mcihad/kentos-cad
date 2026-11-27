# ADR 9332: Ağ araçları ağı baştan gösterir, kaydedilen ağı hemen görür, tek çizgi katmanından ağ oluşturur

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  16, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 0209](0209-network-analysis.md) (ağ analizi: En kısa yol, Hizmet alanı, Şebeke izleme).

## Bağlam

- **Gözlem:**
  - `qa_yollar` içe alındı, ağ eklenmeden Analiz › En kısa yol açıldı.
  - Alt çubuktaki Ağlar ile varsayılan yol ağı tanımlandı ve kaydedildi; bildirim “1 ağ” dedi.
  - Açık rota aracı hâlâ “projede ağ tanımlı değil” diyordu. Esc ve yeniden açınca A → B doğrudan kenarı 50.000 m ile
    Rota katmanına yazıldı.
- **Kod:** ağ araçları (En kısa yol, Hizmet alanı, Şebeke izleme) ağı yalnız başlarken ve bir seçenek değişince
  okuyordu. Belgenin değiştiğini söyleyen bildirime (`document_changed`) bakmıyorlardı.
- **İncelemenin kabulü:**
  - Ağ tanımı kaydından sonra etkin araç ağlarını yeniden okusun ve ilk nokta adımına dönsün.
  - Uygun tek çizgi katmanı varsa “Bu katmandan ağ oluştur” kısayolu olsun.
  - Ağın adı, maliyet ölçütü, birim ve bağlanma toleransı başlangıçtan itibaren görünsün.

## Araştırma

- **QGIS:** en kısa yol işleminde ağın çizgi katmanı, strateji (en kısa ya da en hızlı) ve başlangıç ile bitiş aynı
  formda seçilir; ağ ayrıca tanımlanmaz
  ([ağ analizi](https://docs.qgis.org/4.4/en/docs/user_manual/processing_algs/qgis/networkanalysis.html)).
- **KentOS kararı:**
  - Ağ proje ayarı olarak kalır (ADR 0209); araç onu her an güncel görür.
  - Tek çizgi katmanından ağ, Ağlar'ın Yeni ağ'ının öntanımıyla tek adımda yazılır.
  - Ağın kendisi aracın ayarlarında baştan görünür.

## Karar

### 1. Araç ağı her an güncel görür

- En kısa yol, Hizmet alanı ve Şebeke izleme belge değişince ağları yeniden okur (`document_changed`). Ağlar'ın Kaydet'i,
  bir betik ya da geri alma da buna dahildir.
- Görülen ağ değiştiyse (yokken tanımlandı, değişti ya da silindi) araç noktalarını bırakır ve ilk adıma döner: En kısa
  yol'da “ilk durağa tıklayın ya da Y,X yazın”.
- Aracın kendi yazdığı (Rota katmanı) ağı değiştirmez; adım sürer.

### 2. Katmandan ağ oluştur

- Projede ağ yokken çizimde kilitli olmayan tek bir çizgi katmanı varsa (çizgi ya da çoklu çizgi tutan) istem bunu
  söyler: “projede ağ tanımlı değil; Ağlar ile tanımlayın ya da “qa_yollar” katmanından ağ oluşturun [Ağlar (G) /
  Katmandan ağ oluştur (O)]”.
- **O:** o katmandan “qa_yollar ağı” adlı bir yol ağı `cad.network.define` ile yazılır. Ağlar'ın Yeni ağ'ı gibidir: iki
  yönlü, uçlarda bağlanır, öntanımlı tolerans, Süre 50 km/sa. Araç hemen o ağla ilk adıma geçer.
  - Söylenen: ““qa_yollar” katmanından “qa_yollar ağı” yol ağı oluşturuldu: iki yönlü, uçlarda 0.010 m toleransla
    bağlanır. Ağlar ile değiştirebilirsiniz.”
- Birden çok çizgi katmanı varsa öneri yoktur; Ağlar sorar.

### 3. Ağ baştan görünür

- Aracın ayarları (Öznitelikler'in başı) ağı başlarken gösterir:
  - **Ağ:** adı;
  - **Maliyet:** ölçütü ve birimi (Uzunluk, metre; Süre, dakika; alanın birimi);
  - **Tolerans:** bağlanma toleransı;
  - **Bağlanma:** uçlarda ya da köşelerde de.
- Ağ yokken tek satır nasıl kurulacağını söyler.
- İstem metni değişmez. Ağ ve maliyet seçenekleri istemde kalır.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Ağ araçlarının ağı güncel görmesi, katmandan ağ, ağın ayarlarda görünmesi | yok: ağ analizi yoktur | var | yok: Yol'da ağ analizi yoktur | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Ağ `cad.network.define` ile yazılır, analizler aynı ağı okur. Etkileşimli araç masaüstünün sunumudur. |
| MCP | var | Aynı komut. |
| Yapay zekâ ajanı | var | Aynı komut. |
| Eklenti | var | `document.write` izniyle aynı komut. |

## Açık

- **Katman seçimi:** birden çok çizgi katmanında hangisinden ağ kurulacağını soran bir kısayol yok; Ağlar kullanılır.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **CBS paketi:** `cargo test -p kentos-desktop-cbs`: 60 geçti.
  - `en_kisa_yol_offers_the_layer_and_shows_its_network_from_the_start` (A, C, B yolları, tek çizgi katmanı, ağ yok):
    - istem öneriyi söylüyor, ayarlarda not var;
    - O ile “qa_yollar ağı” yazılıyor, istem ilk durakta;
    - ayarlar: Ağ, Maliyet “Uzunluk, metre”, Tolerans, Bağlanma “uçlarda”;
    - A → B tıklanınca istemde “Uzunluk 50.000 m”.
  - `a_network_saved_while_the_tool_runs_brings_it_to_its_first_stop`: araç açıkken ağ komutla yazılınca araç sürüyor,
    istem ilk durakta.
- **İzler:** `cargo test -p kentos-cad`: 37 geçti; ağ izlerinin istemleri değişmedi.
- **Resim** (`.run/gui-evidence/kullanim-9332/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-agsiz-rota`: istemde öneri, Öznitelikler'de not;
  - `02-katmandan-ag`: ağın dört satırı, istem ilk durakta.
