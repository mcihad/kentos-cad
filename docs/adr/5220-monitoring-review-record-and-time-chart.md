# 5220 — İzleme incelemesinin kaydı; eşiklerin kaynağı; hareketin zaman grafiği

Tarih: 2026-11-11. plan/map-cad.md MAP-19, onuncu tur. Durum: taslak. ADR 4840'ın “Sonra”sındaki üç iş: yetkili inceleme
kaydı (`.kcad` bölümü), eşiklerin kaynağıyla saklanması, hareketin zaman grafiği.

İlgili:
- [ADR 4840](4840-monitoring-trend-and-thresholds.md): dönemlerden doğru, iki eşik;
- [ADR 4420](4420-monitoring-series-of-epochs.md): dönem dizisi;
- [ADR 3620](3620-asbuilt-job-kinds-and-monitoring.md): iş türleri, izleme;
- [ADR 5200](5200-kept-survey-adjustments.md): dengelemenin sonucunun projede saklanması (aynı kayıt yolu).

## Dayanak

- **Mevzuat** (dokuzuncu turun notu `.run/research/map-cad/mevzuat/MAP-19.9.md`, 10 Kasım 2026'da okundu):
  - TKGM Genelge 2025/4:
    - m. 3: mühendis, kontrol mühendisi ve kontrol raporu tanımları;
    - m. 12(16)–(20): belgelerde tarih, düzenleyenlerin ve kontrole yetkili personelin unvanı, adı soyadı ve imzası;
      kontrol mühendisi “Görülmüştür” ya da “Kontrol edilmiştir” yazıp isim ve tarihle imzalar.
  - BÖHHBÜY m. 76 (kontrol mühendisi imzalar, idarenin yetkilisi onaylar), m. 82(1)(c) (kontrolün sonucu teknik raporda)
    ve m. 98(1)(ı) (yapım, kontrol ve onay raporları arşivlenir).
  - 5070 sayılı Kanun m. 5: güvenli elektronik imza elle atılan imzayla aynı hukukî sonucu doğurur.
  - İzleme eşiği koyan resmî metin bulunamadı (DSİ, TBDY 2018, AFAD, KGM). Eşikler projenin ya da idarenin değeridir; kaynağı
    kayda yazılmalıdır.
- **Trimble Business Center, Monitor Point Movement** ([belge](https://help.fieldsystems.trimble.com/tbc/25014.htm), 11 Kasım
  2026'da okundu; resimler `.run/research/map-cad/uygulama/MAP-19.10-resim/`, not `MAP-19.10.md`):
  - Monitoring Point Chart, tek noktanın yer değiştirmesini kaydırılabilir bir zaman ekseninde gösterir;
  - bileşenler (Δ2D, Δ3D, ΔEasting, ΔNorthing, ΔElevation) grafiğin altında seçilir;
  - yalnız Δ2D, Δ3D ya da ΔElevation seçiliyken grafik uyarı eşiğini aşan bölgede sarı, alarm eşiğini aşanda kırmızı
    taranır (resim `tbc-monitoring-25260_421x419.png`: iki yatay çizgi ve iki bant);
  - noktalara uyarı ve alarm eşikleri Properties panelinde ya da Monitoring Points Spreadsheet'te yazılır (resim
    `tbc-monitoring-25251_470x251.png`).
- **Trimble 4D Control ve Bentley iTwin IoT** (dokuzuncu turun notu): uyarı onaylanana dek yeniden tetiklenir; onay
  olay günlüğünde kişisi ve zamanıyla kayıtlıdır.
- **ArcGIS Data Reviewer** (dokuzuncu turun notu): her geçişte kimin ve ne zaman incelediği yazılır.
- **KentOS bugün** (ADR 4840): eşikler formda kalır, kaynağı yazılmaz; inceleme kaydı ve grafik yoktur.

## Karar

1. **`cad.asbuilt.review` v1, İzleme incelemesi** (yeni, belgeyi değiştirir, tek geri alma adımı “İzleme incelemesi”).
   - **Girdi:** `name`; saklarken `check` (izleme işi: `kind: monitoring`), `reviewer` ve eşik verildiyse
     `thresholdsSource`; kaldırırken yalnız `remove`. `expectedRevision` isteğe bağlı.
   - **Saklanan** (`settings.monitoringReviews`, aynı ad yenisiyle değişir):
     - iş: ölçülen ve önceki dönemin katmanı, dönem dizisi ve günleri, bu dönemin günü, ad özniteliği, kontrol
       noktaları, hareket sınırları, doğrunun güveni, eşikler ve kaynağı;
     - noktalar: son yatay ve kot hareketi, yatay hız (doğrununki, yoksa ortalama), durum, aşılan sınırlar. Dizisiz işte
       önceki döneme göre hareket; durum ve hız yok;
     - **özet** (`digest`, SHA-256): işin okuduğu noktalar. Önce önceki dönemler, sonra `design`, sonra `measured`.
       Her katman `k<TAB>katman`, noktaları `p<TAB>ad<TAB>x<TAB>y<TAB>z` (altı ondalık, kot yoksa `-`), çizim sırasıyla.
       Tasarım özetinin (ADR 1700) yazımı;
     - inceleyenin beyanı:
       - rol: `responsible` sorumlu mühendis, `control` kontrol mühendisi, `staff` kontrolü yapan teknik personel,
         `approver` onaylayan;
       - unvan, ad soyad, gün;
       - ibare: `checked` “Kontrol edilmiştir”, `seen` “Görülmüştür”;
       - imza: `wet` ıslak, `electronic` güvenli elektronik;
       - not.
   - **Cevap:** saklanan kayıt, `replaced` ya da `removed` ve projenin bütün incelemeleri: ad, inceleyen, gün, ibare,
     alarm ve uyarıdaki nokta sayısı, `current`. `current` işin bugün okuduğu noktaların özeti saklananla aynı mı
     demektir; değilse inceleme eski sonuca aittir. Sürüme değil özete bakılır, çünkü kaydın yazılması sürümü
     değiştirir.
   - **Retler:**
     - `invalid_input`: ad; denetim ya da inceleyen eksik; kaldırırken başka alan; izleme işi olmayan denetim; kaynaksız
       eşik ya da eşiksiz kaynak; kaynağın, unvanın, adın, günün ya da notun biçimi;
     - `unknown_monitoring_review` (yeni kod): kaldırılacak inceleme yok;
     - denetimin kendi retleri, kendi yollarıyla (`invalid_asbuilt_check`, `layer_not_found`);
     - `invalid_revision`, `revision_conflict`.
   - **KentOS yetkiyi ve imzayı doğrulamaz.** İnceleyenin beyanını yazar; unvan, lisans ya da e-imza denetlenmez.
     Kayıt “onaylı” ya da “yetkili” diye damgalanmaz.
2. **`.kcad` şema 245:** `settings.monitoringReviews`, JSON olarak.
   - Okuyucu ve yazıcı kuralları sözleşmenin `monitoring_reviews_problem`'ıdır:
     - ad tekil kısa etiket; katmanlar, ad özniteliği ve kontrol noktaları kısa etiket;
     - günler takvim günü; sınırlar sonlu ve eksi değil; güven 0 ile 1 arasında;
     - eşik sınırları sıfırdan büyük, kaynaklı (en çok 500 karakter);
     - özet 64 küçük onaltılık hane;
     - noktalar tekil, hareket ve hız sonlu ve eksi değil; aşılan sınırlar yalnız durumla ve dört türden;
     - inceleyenin unvanı, adı ve günü geçerli.
   - Anahtar 17 harflidir; `surveyAdjustments`'tan önce yazılır (baytça). Şema 244 bu alanı reddeder.
   - Bağımsız okuyucu (`tools/kcad/kcad.py`) aynı kuralları kendisi denetler. Bilinmeyen sürüm örneği
     `broken/schema-version-246.kcad`.
3. **Masaüstü** (Uygulama kontrolü › İzleme):
   - **Dönem dizisi:** eşiklerin altına **Eşiklerin kaynağı** satırı.
   - **İnceleme bölümü** (denetim cevap verince):
     - İncelemenin adı;
     - İnceleyen (rol, unvan, ad soyad);
     - Gün, ibare, imza;
     - Not;
     - **Projede sakla** ve **İncelemeyi kaldır** düğmeleri.
   - **Projedeki izleme incelemeleri** tablosu: inceleme, inceleyen, gün, ibare, alarm, uyarı ve noktalar (“Güncel” ya da
     “Eski sonuca ait”). Özet, noktaları değişen incelemeleri söyler.
   - **Grafik**, dizinin üçüncü görünümü (Hareketler | Doğru ve eşikler | Grafik):
     - Nokta seçimi (seçilmezse ilk alarmdaki, yoksa ilk nokta) ve Yatay | Doğu | Kuzey | Kot;
     - ölçüler dönemlerin günlerine göre (gün eksikse dönem sırasıyla), mm;
     - Doğu, Kuzey ve Kot'ta uydurulan doğru kesikli. Hız cevabın, kesişim aynı dönemlerin ortalamasından (en küçük
       kareler);
     - Yatay'da uyarı ve alarm eşikleri çizgi ve bantla (sarı, kırmızı), Kot'ta iki yönlü;
     - eşiği aşan ölçü o düzeyin rengiyle;
     - seçim yeniden denetlemez.
4. **Kapsam işareti:** ADR 3620 gibi **Harita + Yol + CBS**.

## Sınırlar

- Özet noktaların altı ondalığına bakar. Nokta sırasının, adının ya da yerinin değişmesi incelemeyi eskitir. Bir
  özniteliğin (σ) değişmesi eskitmez.
- Aynı adda iki ölçü varsa kayıt ilkini tutar (iş noktaları adla eşler).
- Onay akışı (inceleme, düzeltme, doğrulama evreleri), uyarının onaylanması ve bildirim yok. Kayıt bir beyandır.
- Kayıttaki eşikler forma geri alınmaz; form boşsa eşikler yeniden yazılır.
- Grafikte Δ3D yok. Kot eşikleri iki yönlü, yatay eşikler tek yönlüdür.
- Gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/asbuilt_review_cases.py`; KentOS kodu yok), `cad.asbuilt.review` için 30 durum:
  - iş, noktalar ve durumları ADR 4840'ın başvurusundan (elle);
  - özet betikte `hashlib`'le, kurulumun kendi noktalarından;
  - dört dönem, doğru ve hız eşikleri; dizisiz iki dönem; eşiksiz doğru;
  - onaylayan ve e-imza, iki satırlı not; aynı adın değişmesi, ikinci inceleme;
  - incelemeden sonra yeri değişen nokta (eski inceleme `current: false`);
  - Yol ve CBS projeleri; kaldırma (biri kalır, hepsi gider); plan ve doğrulama yazmaz;
  - 14 ret ve sürüm çatışması.

  Ayarlar tam karşılaştırılır (sayıların yeri tutulur, değeri `$any`), cevap 1e-6 içinde. Bozma denemesi: bir özet ve
  bir `current` değiştirilince koşucu tam o ikisini yakalar (30'da 2).
- **`.kcad`:** `monitoring-reviews.kcad` ve beş bozuk örnek:
  - şema 244'te;
  - büyük harfli özet;
  - takvimde olmayan gün;
  - kaynaksız eşik;
  - iki kez ad.

  Bağımsız okuyucu ve Rust okuyucu aynı sonucu verir (1825 dosya, kcad testleri 244).
- **Python** (`python/tests/test_monitoring_review.py`), testin kendi hesabıyla:
  - özet testin kendi yazımıyla;
  - noktaların durumu denetimin izleriyle aynı;
  - bağımsız okuyucu kaydı ve kaynağı geri okur;
  - noktası değişen çizimde eski inceleme eski, yenisi güncel.

  Python ortak durumları (`cad.asbuilt`) geçer.
- **MCP** (`crates/native/mcp/tests/asbuilt_review.rs`): durumun özeti, işi, inceleyeni ve noktaların durumu; kaynaksız
  eşiğin ret iletisi.
- **Masaüstü** (`asbuilt_review_tests`, 2 test ve resim):
  - pencereden saklanan inceleme durumunkiyle aynı;
  - kaynaksız eşik saklanmaz;
  - nokta taşınınca inceleme eski sonuca ait olur;
  - inceleme kaldırılır;
  - grafiğin noktaları günlere göre (0, 90, 181 gün), eşikler yalnız Yatay ve Kot'ta;
  - seçim yeniden denetlemez.

  Çapraz denetimler 154 test. Resimler `.run/gui-evidence/izleme-inceleme-5220/` (inceleme, grafik, grafik-kot; açık
  geniş, koyu dar).

## Sonra

- Kayıttaki eşiklerin forma alınması; incelemenin raporu (PDF) ve paftaya yazılması.
- Onay evreleri ve uyarının onaylanması; e-imzanın doğrulanması (kurum altyapısı).
- Grafikte Δ3D ve birden çok nokta; ağırlıklı doğru.
- Web (TODOS §0.2).
