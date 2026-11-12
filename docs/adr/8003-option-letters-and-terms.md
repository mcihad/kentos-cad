# 8003 — Mimari'de tek ad, tek harf: seçenek harfi sözleşmesi, terimler ve Açı

Tarih: 2026-11-07. Durum: kabul (M4, ADV-08; plan S5, S6, S9, S10, G2).

## Sorun

- **Aynı şeyin birden çok adı vardı.** Duvar kalınlığı istemde "Genişlik (E)", araç panelinde "Kalınlık", Öznitelikler'de
  yine "Genişlik" diye geçiyordu. Aynı paneldeki Genel › "Kalınlık" ise çizgi kalınlığıydı.
  - Kapının "Uzaklık (U)"su panelde "Uçtan uzaklık", "Mesafe referansı (R)"sı "Ölçü noktası" idi.
  - Mimari araçlarının istem ve panelleri başta ve iki tıktan sonra dökülünce, aynı harfe istemde ve panelde farklı ad
    veren **49** yer çıktı.
- **Aynı anlam farklı harfteydi.** 12 ad birden çok harfte duruyordu (panel satırları dahil): Genişlik kapıda E, merdiven ve rampada G;
  Kalınlık duvarda E, döşemede K; Yöntem B, M ve Y; Sınır S ve Ç; Kat L ve V …
- **Yanlış harf aracı bırakıyordu.** Çalışan aracın seçeneği olmayan bir harf genel kısayolu başlatıyordu.
  - Merdivende genişlik için basılan E, Esnet'i başlatıp merdiveni düşürüyordu.
  - Shift+E duvarın E'si sayılmıyordu, Uzat'ı başlatıyordu (G2).
- **CAD projesinde imleç "Semt" yazıyordu.** Duvar, çoklu çizgi, nokta ve istasyon-öteleme araçlarında ad da değer de
  CBS'ninkiydi: doğuya çizilen duvar "Semt 90°" gösteriyordu.
- **Kelimesi kelimesine çeviriler.** Sahip 7 Kasım'da Öznitelikler'de "Ortak duvar: Benimse"yi görüp ne demek olduğunu sordu
  ve şunu söyledi: "Çok mantıksız Türkçe kelimeler var her zaman bunları da düzelt." "Benimse", "adopt"un çevirisiydi.

## Araştırma (7 Kasım 2026)

- **AutoCAD 2024 — "About Entering Commands in the Command Window"** (help.autodesk.com):
  - Seçenekler büyük ya da küçük harfle aynı çalışır: "If you prefer to use the keyboard, specify the option by
    entering the capitalized, colored letter. You can enter uppercase or lowercase letters. For example, to choose the
    Width option, type w and press Enter."
  - Komutun içinde başka komut ancak şeffaf (apostrofla) çalışır ve bitince asıl komut sürer: "After you complete the
    transparent command, the original command resumes."
  - Aynı sayfanın komut satırı resmi: PLINE `[Arc Halfwidth Length Undo Width]`, açıklaması "to respond to prompt, click
    option or enter highlighted letter" (`.run/gui-evidence/mimari-m4/referans/`).
- **AutoCAD Architecture 2024 — WALLADD:** seçenekler Style, Group, Width, Height, Offset, Flip, Justify, Match, Arc.
  Width'in tanımı: "Specifies the thickness of the wall." İngilizce ad Width olsa da anlam kalınlıktır; Türkçede duvar
  için "kalınlık" denir.
- **Türk mevzuatı:**
  - **Sığınak Yönetmeliği md. 8 ve 14:** mimari projede dış duvar ve tavan **kalınlıkları** gösterilir.
  - **PAİY md. 4(ıı) ve (rr):** iç yükseklik "**döşeme kaplaması** üstünden tavan kaplamasına".
  - **PAİY md. 4(ğ/1) ve 20(8)(e):** "**şaft**, ışıklık, hava bacası ve **galeri boşlukları**", "temele inen **asansör
    boşluğu** … şaft". Şaft bir boşluk türüdür; galeri ve asansör boşluğu da öyle.
  - **2026 Dijital Hazırlama Yönetmeliği ekleri** (RG 05.08.2026/33331): 241 sayfalık taranmış PDF indirilip OCR ile okundu.
    - **EK-2 Tablo 2.3** mimari öğe adları: Aks, Döşeme, Duvar, Duvar Kaplama, Kapı, Kolon, Korkuluk, Merdiven, Pencere,
      Rampa …
    - **EK-3** mahal işlevleri: Galeri (GAL) ile Şaft (SFT) ayrı işlevlerdir.
    - **EK-4** CAD katman ve sembol esasları; katman adları EK-2'nin öğe kodlarından kurulur.
- **TDK:** "lejant, -dı" (sozluk.gov.tr). "Malzeme lejandı" doğrudur, değiştirilmedi.

## Karar

### 1. Harfler

- **Shift ve Caps Lock seçeneği değiştirmez** (`keys::option_letter`): Shift+E duvarın Kalınlık'ıdır, AutoCAD'deki gibi.
- **Seçenek olmayan harf çalışan komutu bitirmez** (`App::letter_keeps_command`):
  - Bir komut çalışırken, adımın seçeneği olmayan bir harfin kısayolu o komutu bitirecek bir araçsa araç başlamaz, uyarı
    çıkar: "Duvar sürüyor: “L” bu adımın seçeneği değil. Çizgi için önce Esc'ye basın."
  - Komutun üstünde çalışan araçlar (Kaydır, Pencere yakınlaştır, ölçüler) ve araç başlatmayan komutlar eskisi gibi
    çalışır.
  - Komut yokken harf aracını başlatır, eskisi gibi.
- **Sözleşme:** bu anlamlar Mimari'nin her aracında aynı harftedir:

  | Harf | Anlam |
  |---|---|
  | E | Kalınlık, Genişlik, En |
  | G | Geri |
  | K | Kapat |
  | Y | Yükseklik |
  | S | Stil |
  | T | Tür |
  | L | Kat |
  | Ö | Öteleme |
  | D | Açı |

  Bir ad, Mimari araçlarının istemlerinde tek harftedir.
- **Değişen harfler:**
  - Merdiven, Döner merdiven, Rampa: Genişlik G → E. Rampa: Eğim E → M.
  - Döşeme: Kalınlık K → E, Kenar E → N.
  - Çizgiden duvar: Yöntem M → B.
  - Mahal: Sınır Ç → S (Ç de okunur).
  - İç görünüş: Aralık R → A (KentOS'un öbür araçları gibi).
  - Merdiven ve rampanın değer sorduğu adımda G, Geri olarak kalır.

### 2. Açı

Duvar ve Çoklu çizgi'nin (`path.rs`), Nokta'nın ve İstasyon-öteleme'nin imleç yazısı CAD projesinde "Açı" yazar: doğudan,
saat yönünün tersine (ADR 0165 §4, `Format::direction`). CBS projesinde "Semt" kalır. Çizgi ve Paralel bunu zaten
yapıyordu.

### 3. Terimler: istem = panel = Öznitelikler

- **Duvar** "Genişlik" → **Kalınlık** (istem, soru, Öznitelikler).
  - Çizginin ağırlığı Öznitelikler'de, "Bu değer nereden geliyor?"da, geçmiş yazısında ve Özellik eşle'de **Çizgi
    kalınlığı**.
  - Durum çubuğundaki "Kalınlık" düğmesi ve katman sütunu yalnız çizgi kalınlığını anlatıyor, yer dar; kaldı.
- **Kapı, pencere, boşluk, niş, baca:** Uzaklık → Uçtan uzaklık; Mesafe referansı → Ölçü noktası; Orta → Yerleştirme
  (Ortala); Yan → Açılma yanı; Alt/Üst → Alt kot/Üst kot; Yüzden → Yüzden uzaklık. İmleç kartı "Duvar · En 0.900 m" →
  "Genişlik 0.900 m" (0,90 kapının genişliğiydi, duvarın değil).
- **Dikdörtgen duvar:** Döndür → Açı; panelde Boylar → Boyutlar.
- **Ölçülü bölme:** Ölçme → Uzaklık türü; Yan → Kaynağın yanı.
- **Kolon:** Döndür/Dönüş → Açı; Etiket → İlk etiket.
- **Aks:** Seri → Seri aralıkları; Kesik → Kesik uçlar.
- **Aks sistemi:** Biçim → Düzen (Dik/Eğrisel); Kolonlar → Kesişimlere kolon.
- **Merdiven:** Kesme → Kesme yüksekliği; Numaralar → Basamak numaraları; Döner merdivenin Yay'ı → Yay boyu (Yay, çoklu
  çizginin yay kipidir).
- **Rampa:** Yükseklik → Yükseklik farkı; Başlangıç → Başlangıç kotu.
- **Mahal:**
  - Yöntem → Alan yöntemi (Öznitelikler de).
  - Boşluk → **Döşeme boşluğu**. ADR 0708'in boşluğu şaft, asansör ve galeri boşluğunu birlikte kapsar; PAİY ve EK-3
    şaftı ayrı bir tür sayar, bu yüzden plandaki "Şaft" adı alınmadı.
  - Çiz → Sınır (Duvarlardan bul / Köşeleri çiz).
- **Çizgiden duvar:** Kaynağı koru → Kaynak çizgiler (Koru/Sil).
- **Açıklık dizisi:** Yöntem → Dağıtım; Aralık → Kopya aralığı. **İç ölçü zinciri:** Kat → Ölçülen kat.
- **Mutfak dizisi:** Modül → Modül eni; Derinlik → Tezgâh derinliği.
- **Etiketler:** İçerik → Hazır içerik; Biçim → Çerçeve; Mevcut → Mevcut etiket.
- **Modül yerleştir:** Ayna → Aynalı. **Malzeme lejandı:** Yükseklik → Yazı yüksekliği.
- **Komut adları:**
  - Mahal sekmesindeki "Döşeme" düğmesi (`tool.spaceFloor`) tam adıyla Döşeme deseni.
  - "Duvarları birleştir" → **Tek duvara çevir**: "Duvar birleşimi"nin yanında aynı araç gibi okunuyordu.

### 4. Düz Türkçe (sahibin kuralı)

| Önce | Sonra |
|---|---|
| Ortak duvar: Benimse / Çiz | Var olanı kullan / Ayrıca çiz |
| Kolon alanı: Düşme | Düşülmez (seçenekler: Yalnız netten düşülür, Her yöntemde düşülür) |
| Kolonlarda: Ölçme | Ölçülmez |
| Tür / Koru, Mevcut / Tür | Mevcut ya da türün |
| Kalınlıkta sabit: Yol | Sabit kalan çizgi: Taban çizgisi |
| Sol/Sağ bitmiş yüz | Sol/Sağ kaplama yüzü |

Eski sözcükler (Benimse, Çiz, Ç) yazılınca yine okunur.

### 5. Bilerek değişmeyenler

- **Metrajın kural yazısı** "kolon alanı: Yalnız net/Her yöntemde": dondurulmuş komut cevaplarında 609 yerde geçiyor.
  Yalnız hiçbir cevapta olmayan "Düşme", "Düşülmez" oldu.
- **"Kalınlık değiştir" geri alma adı:** web ile paylaşılan komut sözleşmesi.
- **Eş anlamlılar:** "Bütün katlar" (33 yer) ile "Tüm katlar" (15 yer); ikisi de doğru Türkçe, birleştirme ayrı iş.
- **Çoklu çizginin Genişlik (E)'si:** AutoCAD'in PLINE Width'i, doğru.
- **Tek tuş kısayolu:** sahibin kararı (ADR 8002).

## Ölçüm

- **Testler** (`apps/desktop/src/option_contract_tests.rs`, 10 durum):
  - Katalogdan üretilir: Mimari'nin sunup Harita'nın sunmadığı bütün araçlar başlangıçta ve iki tıktan sonra okunur.
  - Durumlar: istem ve panel aynı harfe aynı adı verir; bir ad tek harftedir; sözleşmenin harfleri; adımda harf tekliği;
    açıklamalardaki "Ad (H" harfleri.
  - Shift+E, merdivende e, seçeneği olmayan harf, Açı/Semt.
- **İhlaller:**
  - aynı harfe farklı ad 49 → 0;
  - istemlerde birden çok harfte ad 12 → 0.
  - Yalnız panelde kalan: aks sisteminin Yerleştirme'si P. Bu, kimsenin yazmadığı bir panel anahtarı.
- **Test takımları:**
  - etkileşim 896 + 75;
  - sözleşmeler 1354 + 34 + 10;
  - uygulama 146 + 525;
  - masaüstü 1961 geçti, 1 düştü. Düşen `module_surface_tests::each_program_docks_its_own_panels`, `main`'den gelen eski
    hata.
- **Gerçek pencere** (ADR 8000; `fixtures/interaction/real/harf-sozlesmesi.txt`, `.run/gui-evidence/mimari-m4/sonra/`;
  35 etkileşim, KLM 36,8 s):
  - imleç "Açı 90.0000°";
  - Shift+E "duvarın kalınlığını yazın";
  - l duvarı bıraktırmadı, Komut geçmişi uyarının tam metnini yazdı;
  - merdivende e "genişliği yazın";
  - mahal, kapı ve dikdörtgen duvarda istem ile panel aynı adları okuyor;
  - Öznitelikler'de duvarın Kalınlık'ı ve Çizgi kalınlığı.
- **Şerit yan etkisi** (gerçek pencerede görüldü, önlendi). Önce "Duvar birleşimi"nin adı değiştirilmek istendi:
  - Kısa "Uç birleşimi" ile Mimari şeridi 1440 px'te Proje ▾'yi açıp büyük düğmeleri küçültüyordu.
  - Uzun "Duvar ucu birleşimi" ile kentos-cad'in Mimari sekmesinde bir panel 1440 px'te ikona iniyordu (`ribbon_tests`).
  - Bu yüzden öbür araç yeniden adlandırıldı; şeridin yeniden düzeni M15'in işi.
- **Ayrı bulgular:**
  - `active_level::tests::a_drawing_opens_on_the_level_and_view_it_was_saved_with`, 2 MiB'lık test yığınını taşırıyor
    (64 MiB'la 0,16 s'de geçiyor; M4 olmadan da aynı).
  - Durum çubuğu uyarıyı kesiyor (G12).
