# 3002 — Mimari adları: PENCERE pencere, iki harfli adlar, AutoCAD Architecture adları

Tarih: 2026-11-07. Durum: kabul (M3, ADV-08; plan S2, S7, S13).

## Sorun

- **"pencere" yanlış komutu açıyor.** Komut satırına "pencere" yazınca Pencere yakınlaştır başlıyordu.
  - `PENCERE` yakınlaştırmanın takma adı (web envanterinden: Z, ZOOM, PENCERE, PENCEREBUYUT); pencere aracınınki
    PENCEREKOY ve WINDOW.
  - Komut ara'da da ilk sonuç yakınlaştırmaydı.
- **Kısa ad yok.** 78 mimari komutun hiçbirinde kısayol ya da kısa ad yoktu; en kısa takma ad 3 harfti.
- **Arama boş dönüyor.** "oda", "lavabo", "mobilya" aramaları hiçbir şey bulmuyordu.
- **AutoCAD Architecture adları tanınmıyordu** (WALLADD vb.).
- **Ad çözümü modüle bakmıyordu.** Komut satırındaki ad çözümü (`typed_names.rs`) bütün komutları tarıyor.
  - Takma adlar bütün KentOS'ta tek; "pencere"yi pencereye vermek yakınlaştırmanın adını her yerde almak demekti.
  - Harita'da Netcad alışkanlığıyla "PENCERE" yakınlaştırmadır.

## Araştırma (7 Kasım 2026)

- **AutoCAD Architecture 2026 toolset** — “Frequently Used Commands” (help.autodesk.com, ENU, GUID-72935B4D-…):
  - WALLADD, DOORADD, WINDOWADD, SPACEADD, SLABADD, STAIRADD, ROOFADD;
  - WALLCLEANUPI, WALLSTYLEEDIT, DIMADD (AEC ölçüsü), SCHEDULEADD, PROJECTNAVIGATORTOGGLE.

  AutoCAD 2027 bu toolset'i içeriyor (W1 araştırması).
- **Revit:**
  - Yardımı kısayolların en az iki tuşun sırayla basılmasından oluştuğunu söylüyor.
  - Resmî kısayol listesi (PDF, autodesk.com/shortcuts/revit) bu makineden 403 verdi. Harfleri (WA, DR …) üçüncü
    taraf listelerinden doğrulanamadı; kopyalanmadı. AutoCAD'de DR zaten DRAWORDER.
- **KentOS'ta adların nasıl çalıştığı:**
  - Tek tuş kısayolları çizim alanında hemen çalışır (L, P, C, A, R, T, D, H, S, E, Z, B, G, J, N, V, X, Y, I, O dolu).
  - Takma adlar komut satırına yazılıp Enter ya da Boşluk'la gönderilir.
  - AutoCAD profili (ADR 0282) kapalı başlar ve KentOS'un adlarına dokunmaz.
  - Modül programı yalnız kendi komutlarını çalıştırır (ADR 0802, `App::offers`).

## Karar

- **Modül adları:** `catalog/v1/names-mimari.json` (`kentos.moduleNames/1`) yalnız KentOS Mimari'de geçerlidir. Orada
  KentOS'un ortak adlarından ve AutoCAD profilinden önce gelir (`App::run_module_name`, `run_typed`'in başında).
  - **PENCERE → Pencere** (`overrides: tool.zoomWindow`, gerekçesiyle). Pencere yakınlaştır Z tuşunu ve PENCEREBUYUT adını
    tutar. kentos-cad'de ve Harita'da PENCERE yine yakınlaştırmadır.
  - **İki harfli adlar:** DU Duvar, DD Dikdörtgen duvar, OB Ölçülü bölme, KP Kapı, PN Pencere, MH Mahal, DS Döşeme,
    KL Kolon, AK Aks, MR Merdiven, RM Rampa, MD Mutfak dizisi, ET Etiket. Harfler KentOS'ta hiçbir anlamı olmayanlardan
    seçildi; BO (Sınır) ve MT (Çok satırlı yazı) dolu olduğu için alınmadı.
  - **Mimarın sözcükleri:** ODA → Mahal; DONATI, MOBİLYA, LAVABO, KLOZET, VİTRİFİYE → KentOS blok kitaplığı.
  - Bir ad, KentOS'un ortak bir adını yalnız dosya `overrides` ve `why` ile söylerse alır; testler bunu tutar.
- **Görünürlük:**
  - **Komut satırının listesi:** Mimari'de satırlar modül adlarıyla kurulur. Pencere satırının adı PENCERE olur, ardından
    PENCEREKOY, WINDOW, PN gelir. Pencere yakınlaştır satırında PENCERE görünmez. Satırlar her karede kurulduğu için
    değişen satır bir kez yapılıp saklanır.
  - **Komut ara:** aynı adlarla puanlar ("pencere"de Pencere, "lavabo"da blok kitaplığı ilk sırada).
  - **F1:** "Mimari adları (komut satırı)" kümesi; küme adı da aranır ("mimari" bütün kümeyi gösterir).
- **AutoCAD profili** (açıldığında): WALLADD, DOORADD, WINDOWADD, SPACEADD, SLABADD, STAIRADD, WALLCLEANUPI,
  WALLSTYLEEDIT, DIMADD, PROJECTNAVIGATORTOGGLE profilin sonuna, ayrı küme olarak eklendi. Mevcut sıra korundu; LA hâlâ
  Özellikler'in adı.
- **Tek tuş kısayolu verilmedi.** Mimari'nin en sık araçlarına tek tuş vermek (ör. D Duvar) ya dolu harflerin
  anlamını Mimari'de değiştirmeyi ya da yalnız boş harfleri (F, K, M, Q, U, W) kullanmayı gerektiriyor. Bu sahibin
  kararı; ADV-08'de açık.

## Ölçüm

- **Testler** (`apps/desktop/src/module_names_tests.rs`, 7 durum):
  - dosya okunuyor; her ad masaüstünde çalışan ve Mimari'nin sunduğu bir komuta gidiyor; adlar tek;
  - yalnız PENCERE ortak bir ad alıyor;
  - Mimari'de pencere/PENCERE/pn → window, du → wall, kp → door, mh/oda → space, pencerebuyut → zoomWindow;
  - kentos-cad'de pencere → zoomWindow, du hiçbir şey başlatmıyor;
  - liste satırı ve Komut ara sırası; F1 satırları.
- **AutoCAD profili testleri** (yeni 10 ad dahil) geçti.
- **Masaüstü kitaplık testleri:** 1952 geçti, 1 düştü. Düşen `module_surface_tests::each_program_docks_its_own_panels`
  (Harita'nın Yol paneli) bu değişiklikler geri alınınca da düşüyor; `main`'den gelen ayrı bir hata.
- **Gerçek pencere** (ADR 8000; `fixtures/interaction/real/mimari-adlari.txt`, `.run/gui-evidence/mimari-m3/`):
  - "pencere" yazınca listenin ilk satırı PENCERE · Pencere; Enter Pencere aracını açtı.
  - "du" Enter Duvar'ı açtı.
  - F1'de "mimari" Mimari adlarını listeledi.
  - Komut ara'da "pencere"nin ilk sonucu Pencere (Mimari › Yerleştir).
- **Aynı oturumda görülen ayrı bulgu:** F1 penceresi Esc'yle kapanmadı; Kapat düğmesi kapattı (plan §1.5, G11).
