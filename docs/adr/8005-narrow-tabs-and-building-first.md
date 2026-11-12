# 8005 — Dar şeritte ⋯ menüsü; Öznitelikler'de yapı elemanının kendi alanları önce

Tarih: 2026-11-07. Durum: kabul (M6, ADV-09; plan S17, S18).

## Sorun

- **Sekme satırı dar pencerede gizli taşıyordu (S17).**
  - Mimari'de bir sekme açıkken satır, açık sekmeyi gösterecek kadar sola kayıyordu.
  - 1100 × 650'de, 18 px yazıyla bir duvar seçilince Dosya, Giriş, Ekle ve Açıklama ekranın dışında kalıyordu.
    Ulaşmanın tek yolu 4 px'lik bir kaydırma çubuğuydu ve ekranda bunu söyleyen bir şey yoktu
    (`.run/gui-evidence/mimari-m6/once-18/`).
  - Mimari dışı bir sekme açıkken satır hiç sınırlanmıyordu. Araştırmada kentos-cad'in 14 sekmesi 1100 px'te pencereden
    taşmış, seçimin "Duvar" sekmesi görünmez olmuştu.
- **Öznitelikler genel alanlarla başlıyordu (S18).**
  - Bir duvar seçilince önce her nesnede olan sekiz satır geliyordu: Tür, Katman, Renk, Çizgi kalınlığı, Çizgi tipi,
    ölçeği, Saydamlık, Sembol.
  - Duvarın kendi bölümünde de Kalınlık'tan sonra birleşim ve düğüm satırları geliyordu; Stil, Yükseklik, Kat ve Evre
    altta kalıyordu.
  - 1100 × 650 ve 18 px'te panelde yalnız "Tür" görünüyordu.

## Araştırma (7 Kasım 2026)

- **Microsoft Fluent 2 — Tablist** (fluent2.microsoft.design, Tablist usage):
  - "Tabs in a horizontal tablist won't scroll or wrap to the next line. This means they are limited by the width of their
    container. If you need to show more tabs, include an overflow menu button."
  - "Tablists don't wrap; if there isn't enough room, additional tabs go into an overflow menu."
  - Dar düzen için açılır listeyi gösteren görsel `.run/gui-evidence/mimari-m6/referans/fluent-tablist-small.png`'de.
- **Revit ve AutoCAD Architecture:** özellik paletinin başında nesnenin kendi bilgisi durur.
  - Revit'te paletin tepesinde duvar tipi seçicisi, ardından Constraints (taban ve üst kat) ve Dimensions gelir.
  - ACA'da Design sekmesinin Dimensions bölümü duvarın genişliğini ve yüksekliğini taşır (ADR 8003'ün kaynağı: "specify
    the thickness of the wall — enter a value for Width").
- **Mevzuat:** doğrudan hüküm yok. Öznitelikler'in başa aldığı alanlar 2026 Dijital Hazırlama Yönetmeliği EK-2 Tablo
  2.3'ün öğelerinin (Duvar, Kapı, Pencere, Döşeme, Merdiven, Rampa, Kolon, Aks) taşıdığı ana bilgilerdir.

## Karar

- **Dar şerit** (ortak bileşen `kentos_ui` ribbon, `tab_fit.rs`):
  - Sekmeler sığmayınca açık sekme, bağlam sekmeleri (seçimin: Duvar, Açıklık …) ve programın kendi sekmesi (Mimari'nin
    Mimari'si, `pinned_tab`) her zaman görünür.
  - Öbür sekmeler soldan sığdığı kadar gösterilir. Gerisi bağlam sekmelerinden önce duran bir ⋯ menüsüne girer; ipucu
    "Diğer sekmeler" ve adlarını söyler.
  - Menü ilk sığmayan sekmeden başlayan kesintisiz bir sıradır.
  - Açık sekme ile ⋯ bile sığmıyorsa satır eskisi gibi kayar.
  - Sekme genişliği yazının ölçülen genişliğinden hesaplanır.
  - Bu kural artık yalnız mimari sekmelerde değil, her sekmede ve 1200 px'in altında geçerli.
- **Öznitelikler** (`properties/rows.rs`, `building_first`):
  - Duvar, açıklık, mahal, döşeme, merdiven, rampa, kolon ve aksta nesnenin kendi bölümü en üste çıkar ve nesnenin adını
    taşır ("Duvar").
  - Bölümün başında ana satırlar şu sırayla durur: Numara, Ad, Tür, Kalınlık, Genişlik, Stil, Yükseklik, Hiza, Kat, Evre.
    Öbür satırlar kendi sıralarını korur; not satırı kendi satırının altında kalır.
  - "Genel" (Tür, Katman, Renk …) ondan sonra gelir.

## Ölçüm

- **Testler:**
  - `tab_fit` 4 durum: hepsi sığar; açık ve bağlam sekmesi kalır, sondakiler menüye girer; menü kesintisiz sıradır;
    çok dar satır kayar.
  - `properties::tests::a_walls_own_section_comes_first_with_its_main_rows_on_top`.
  - `architecture_ribbon` testi artık 1100 px'te bir kaydırmanın bağlam sekmesini görüş dışına çıkarmadığını denetliyor;
    eskiden kaymayı ön koşul sayıyordu.
  - Arayüz kitaplığı 353 test, masaüstü 1968 geçti, 1 düştü. Düşen `module_surface_tests::each_program_docks_its_own_panels`,
    `main`'den gelen eski hata.
- **Gerçek pencere** (ADR 8000; `KENTOS_REAL_SIZE=1100x650` ile `fixtures/interaction/real/dar-ekran-duvar{,-18}.txt`):
  - **18 px, önce:** satır "Değiştir · Mimari · Görünüm · Yönet · Çıktı · Duvar (1)"; ilk dört sekme görünmüyordu.
    Öznitelikler'de yalnız "Genel › Tür" vardı.
  - **18 px, sonra:** satır "Dosya · Giriş · Ekle · Mimari · ⋯ · Duvar (1)". ⋯ menüsü Açıklama, Değiştir, Görünüm,
    Yönet ve Çıktı'yı listeledi. Görünüm seçilince satır "Dosya · Giriş · Mimari · Görünüm · ⋯ · Duvar (1)" oldu. Proje
    adı ve sağdaki simgeler geri geldi. Öznitelikler "Duvar › Kalınlık 0.200 m"yle açıldı.
  - **13 px:** bütün sekmeler sığdı (⋯ yok). Öznitelikler: Duvar › Kalınlık, Stil, Yükseklik, Hiza.
- **Düzenek:** `scripts/usage/real/session.sh` artık `KENTOS_REAL_SIZE` alır; 18 px için sahibin ayarlarının kopyası
  kullanıldı, sahibin dosyasına yazılmadı.
