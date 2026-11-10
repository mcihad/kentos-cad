# 3000 — Mimari kabul gerçek pencerede, kullanıcı gibi

Tarih: 2026-11-07. Durum: kabul (M1, ADV-26; [plan/architecture-advanced.md](../../plan/architecture-advanced.md)).
Mimari ileri planın ADR'leri 3000'den başlar (plan §6).

## Sorun

Sahibin sözü (7 Kasım 2026): “Sen kullanımı nasıl test ediyorsun bilmiyorum ama mouse ile kullanmak ile kod testleri
arasında fark oluşuyor.”

Mimari kabulü bugüne kadar kod içi izlerle yapıldı (`kentos-cad kullan`, `snapshot`, `cargo test … traces`). Bu izler:

- Iced olaylarını uygulamaya doğrudan verir; işletim sisteminin imleci ve gerçek pencere yoktur.
- Aracı kimliğiyle başlatır (`run: tool.wallRectangle`), şeride hiç tıklamaz.
- Kenetlemeyi, ızgarayı ve yardımları kapatır (`draft: snap false …`).
- Komut satırına odağı programla verir (`focus: commandLine`).
- Benim varsayılan ayarlarımla, koyu temada ve tam `kentos-cad`'de oynar; sahip açık temada, Kenet ve Izgara açıkken
  `kentos-mimari` kullanıyor.

27 Ekim'den beri gerçek pencerede mimari kabul yapılmadı; ADV ADR'lerinin çoğu bunu kendisi söylüyor.

## Araştırma (7 Kasım 2026)

- **Keystroke-Level Model:** Kieras, “Using the Keystroke-Level Model to Estimate Execution Times” (Card, Moran & Newell
  1983'ün modelinden).
  - İşlemler ve süreler:

    | İşlem | Süre |
    |---|---|
    | K (tuş; ortalama, sekreter olmayan yazıcı) | 0,28 s (0,12–1,2) |
    | P (işaretleme) | 1,1 s (0,8–1,5) |
    | B (düğmeye basma ya da bırakma) | 0,1 s; tık BB = 0,2 s |
    | H (eli klavye ile fare arasında götürme) | 0,4 s |
    | M (düşünme) | 1,2 s (0,6–1,35) |
  - M'nin yeri için önerilenler: işe başlarken, bir parametreyi düşünürken. Tasarımlar karşılaştırılırken aynı kurallar
    kullanılır; tutarlılık yerden önemlidir.
- **Başka uygulamalar kullanımı nasıl kaydediyor:**
  - **AutoCAD Action Recorder** (help.autodesk.com, AutoCAD 2025 Customization: “To Work with the Action Recorder”,
    “About Action Macros”). Komutları ve girdileri (noktalar, metinler, sayılar, seçenekler) bir eylem ağacına yazar,
    sonra oynatır. Bir iletişim kutusu çıkınca oynatma durur, kapanınca sürer.
  - **AutoCAD 2026 Activity Insights.** “What's Changed” paneli son kayıttan beri neyin değiştiğini okunur cümlelerle
    yazar. Referans resim `.run/gui-evidence/mimari-m1/referans/`.
  - **Revit günlük (journal) dosyaları.** Autodesk'in geliştirici blogu, günlüklerin resmî olarak yalnız sınama ve hata
    bildirimi için yeniden oynatıldığını yazıyor.

  Üçü de gerçek uygulamanın kendi olay yolundan geçer.
- **Bu makinede gerçek girdinin yolları:**
  - KDE Plasma 6.6 Wayland oturumu var; xdotool, ydotool ve Xvfb yoktu; `/dev/uinput` yalnız root'a açık.
  - Görünmez iç içe KWin (`dbus-run-session -- kwin_wayland --virtual --xwayland …`) gerçek pencereyi gösterdi ve
    `spectacle -b -n -f` resmini aldı. Ama XTEST olayları XWayland'den uygulamaya geçmedi: imleç (720,450)'de kaldı.
  - Sahibin izniyle `xvfb` kuruldu (21.1.22). Xvfb'de XTEST doğrudan çalışır.
  - Uygulama ilk denemede çöktü: Intel Vulkan sürücüsü ekrana basmak için DRI3 istiyor, Xvfb'de DRI3 yok. Yazılım Vulkan'ı
    (lavapipe) `WGPU_BACKEND=vulkan` ile seçilince açıldı.

## Karar

- **`scripts/usage/real/`:**
  - `session.sh`: Xvfb 1440×900; lavapipe; her koşu yeni geçici ev dizini ve sahibin ayarlarının kopyası; uygulama; sürücü;
    sayaç.
  - `driver.py`: XTEST ile tık, sürükleme, tekerlek, tuş ve yazı; resimler X ekranından.
  - `count.py`: etkileşimler ve KLM süresi.
  - `scenario_2p1.py`: 2+1 dairenin gerçek senaryosunu üretir.
- **Senaryo:** `fixtures/interaction/real/daire-2p1.txt`, bir kullanıcının yapacağı gibi yazıldı:
  - Başlangıç'ta Yeni proje;
  - Mimari sekmesi ve şeritteki düğmeler (gerekirse menüleri);
  - değerler aracın panel alanlarında;
  - noktalar fareyle; kenetleme ve ızgara sahibin ayarlarındaki gibi açık.
- **Sayaç kuralları:** dosyanın başında.
  - Tık P + BB, hareket P, tuş K, yazı harf başına K.
  - Klavye ile fare arası her geçişte H.
  - Her araç başlatmada ve her yazılan değerde bir M.
- **Kabul:** her mimari madde gerçek pencerede de kabul edilir; sahibe giden resimler buradan alınır. Kod içi izler hızlı
  ve ek güvence olarak kalır.
- **İz denetimi:** kod içi izlerde kullanıcının yapamayacağı adımlar ayıklanır. İlk örnek `focus: commandLine`; araç
  çalışırken komut satırında yazı alanı yok, tık seçenek çipine düşüyor.
- **Sahibin verisi:** sahibin ayar ve kurtarma dosyalarına yazılmaz; düzenek bunu kendi geçici ev dizininde sağlar.

## Ölçüm

- **2+1 daire, gerçek pencerede:** 147 etkileşim.
  - Dökümü: 70 tık, 57 tuş, 14 değer (71 harf), 3 hareket, 3 tekerlek; 11 araç.
  - KLM: K 128, P 73, B 143, H 43, M 25 → **177,6 s** (2 dk 58 s).
  - Bir koşu yaklaşık 3,5 dakika (yazılım çizimi).
- **Sonuç kod içi izle aynı:**
  - Mahaller: Salon 36,00; Yatak odası 1 15,75 → değişiklikten sonra 16,20; Yatak odası 2 12,15 → 11,70; Koridor 10,72;
    Banyo 10,05 m².
  - 54 nesne; iki çizelge, toplam 84,67 m².
- **Aynı KLM kurallarıyla kod içi iz** (`acceptance-flat.json`): 158 etkileşim, 207,7 s. Ama içinde fareyle yapılamayan
  `focus` adımları ve komut satırına yazılan seçenek harfleri var.
- **Gerçek pencerede görülen, kod içi izin görmediği farklar:**
  1. Araç çalışırken komut satırını istem ve seçenek çipleri dolduruyor; yazı alanı görünmüyor. "Komut satırına tıkla"
     çipe düşüp menüsünü açıyor. Seçenek harfi çizim alanında küçük harfle yazılınca çalışıyor ("e" → "duvar
     kalınlığını yazın"), ama bunu ekranda hiçbir şey söylemiyor.
  2. Büyük harf (Shift+E) aracın seçeneği sayılmıyor (`keys::option_letter` Shift'i dışarıda bırakıyor): Dikdörtgen
     duvarı bırakıp **Uzat**'ı başlatıyor. Küçük "e" doğru çalışıyor. İlk denemede sürücü büyük "E"yi Shift'le
     yazmıştı ve bu "E bozuk" diye kaydedilmişti; ikisi ayrı ayrı denenip düzeltildi
     (`.run/gui-evidence/mimari-m3/harf/harf-ozet.png`).
  3. Bir tablo yerleşince şerit kendiliğinden bağlamsal **Tablo** sekmesine geçiyor. Kullanıcının bir sonraki Çizelge ▾
     tıkı başka düğmeye gidiyor; ikinci çizelge oluşmadı, Mimari sekmesine dönmek gerekti.
  4. Duvar ölçüleri "Ölçü katmanı: Çizim", Hepsini etiketle "Etiket katmanı: Çizim (sabit)" ile çalışıyor. Yeni projede
     Ölçü ve Yazı katmanları hazır olduğu hâlde 54 nesnenin hepsi Çizim'de.
  5. Dikdörtgen duvarın varsayılanı 0,20 m ve Stil: Yok (poşe yok).
  6. Kenetleme ve ızgara açıkken fareyle konan pencereler 1–2 cm kayıyor. Dış zincir gerçekte 1.900 · 1.200 · 2.952 ·
     1.200 · 2.095 · 1.200 · 0.953, kod içi izde 1.910 · 1.200 · 2.940 · 1.200 · 2.100 · 1.200 · 0.950.
  7. Görünen alanda çizelgeye yer yok; kullanıcı önce uzaklaştırıyor. İkinci çizelgenin noktası birincinin üstüne
     düşünce tık tabloyu seçti, sessizce.
  8. Gerçek Mimari modülünün şeridi (büyük Duvar, Dikdörtgen duvar, Kapı, Pencere düğmeleri, katlanmış Proje ▾)
     `kentos-cad`'in kod içi resimlerinden farklı; önceki resimler sahibin gördüğünü göstermiyordu.
  9. Yeni proje Giriş sekmesinde açılıyor.
  10. Geçici ölçünün yazısı mahal etiketinin, kapı ve pencere etiketleri duvar çizgilerinin üstüne biniyor.
- **Açık kalan (M1):** `kentos-cad kullan --modul`, `snapshot --sekme`'nin CAD sekmeleri ve sayaç kurallarının kod
  içi izlere uygulanması.
