# ADR 0268: Çizim yardımlarının önceliği ve geçici geçersiz kılma

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-04`: “Ortho/polar tracking ve geçici override
  önceliğini tanımla; geçici kapatma aracın diğer ayarlarını kaybettirmesin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0029 (kenet, Shift ile orto), 0085 (nesne izleme), 0166 (sayısallaştırma kilitleri, Dik açı), 0261 (UCS),
  0267 (koordinat kilitleri).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Bir sonraki noktanın yerini birden çok yardım değiştirebilir: kilitler, Dik açı, kenet ve nesne izleme, Orto ve kutupsal izleme. Kural
kodda vardı (`points::constrain`) ama yazılı değildi, dışarıdan okunamıyordu ve iki eksik vardı:

- Shift Orto'yu basılıyken ters çevirir; ama önizleme ancak fare kımıldayınca yenileniyordu. F8 ve F10 de öyle.
- Orto ve kutupsal izleme ikisi açıkken Orto önce gelir, ama durum çubuğu ikisini de “açık” gösteriyordu; kullanıcı kutupsalın neden
  çalışmadığını göremiyordu. Shift'in geçici hâli de hiçbir yerde görünmüyordu.

AutoCAD Orto ve kutupsal izlemeyi birbirini kapatan anahtarlar yapar (F8 F10'u kapatır): bu, kullanıcının öbür ayarını kaybettirir.
Planın sözü (“geçici kapatma aracın diğer ayarlarını kaybettirmesin”) bunun tersini ister.

## Karar

### 1. Öncelik: tek kural

Bir sonraki noktanın yerini, ilkinden sonuncusuna, şunlardan **ilk uyan** belirler (`kentos_interaction::Aid`, `constrain_ruled`):

1. **Koordinat kilidi** (ADR 0267): doğu ya da kuzey kilitli.
2. **Doğrultu kilidi** (Açı, Sapma, Paralel, Dik; ADR 0166): kenetin noktasını da doğrultuya izdüşürür.
3. **Dik açı** (ADR 0166 §4): ikinci kenardan sonra.
4. **Kenet** ya da **nesne izleme** noktası: kesindir, Orto ve kutupsal onu oynatmaz.
5. **Orto** (Shift basılıyken ters).
6. **Kutupsal izleme**: imleç ışına yakalama payı (10 px) içindeyse.
7. Hiçbiri: imlecin kendisi.

Uzunluk kilidi bunların her biriyle birlikte tutulur. Orto ve kutupsal izleme **hiçbir zaman birlikte** uygulanmaz: Orto önce gelir.
İkisi de açık kalabilir: Orto kapanınca (ya da Shift onu geçici kapatınca) kutupsal izleme kendiliğinden sürer. Ayar kaybolmaz.

Araçlar da, durum çubuğu da, izler de aynı işlevi okur: `Session::ruling_aid` aracın ölçtüğü noktadan, imlecin yerinde hangi
yardımın kural koyduğunu söyler.

### 2. Geçici geçersiz kılma ve görünürlüğü

- **Shift** basılıyken Orto ters döner (kapalıysa açık, açıksa kapalı); bırakınca eski hâline döner. Ayar değişmez.
- Shift'e basmak ya da bırakmak önizlemeyi **hemen** yeniler (fare kımıldamadan): kabuk imlecin yerini yeniden araca verir
  (`repoint`).
- Durum çubuğunun **Orto** hücresi, nokta alan bir komut çalışırken Orto'nun **şu anki** hâlini gösterir; ipucu “Shift basılı: geçici
  olarak açık.” (ya da kapalı) der.
- **Kutupsal** hücresi, kutupsal açıkken Orto kural koyuyorsa “açık ama etkisiz” görünür (Kenet'in ölçek aralığı dışındaki hâli gibi:
  içi boş işaret) ve ipucu “Orto açıkken kutupsal izleme uygulanmaz; Orto kapanınca kendiliğinden sürer.” der.

### 3. Geçişler aracın durumunu korur

F3, F8, F10, Shift+F3, Dik açı ve Topoloji geçişleri yalnız oturumun ayarını değiştirir: çalışan komutun noktaları, seçenekleri,
kilitleri, Kalıcı'sı ve referans noktası kalır; önizleme yeni yardımla hemen yenilenir.

## Sonuçlar

- Öncelik yazılı ve tek yerde; sınanabilir (`aid` beklentisi).
- Kullanıcı Shift'in geçici etkisini ve kutupsalın neden çalışmadığını durum çubuğunda görür.
- Orto ile kutupsal izleme birbirini kapatmaz; ayarlar korunur (AutoCAD'den bilerek ayrılır).

## Doğrulama

- `cargo test -p kentos-interaction --test all aids` (öncelik sırası, Shift'in iki yönü, Orto'nun kutupsala önceliği, kesin nokta,
  kilitler, Dik açı).
- `cargo test -p kentos-desktop aids::` (Shift'te önizlemenin fare kımıldamadan yenilenmesi, Orto hücresinin geçici hâli, kutupsalın
  notu, geçişlerde komutun noktaları).
- Etkileşim izi `fixtures/interaction/v1/aid-priority.json` masaüstünde üç çeşitte; izlerin yeni `hold` adımı ve `aid` beklentisi.
- Resim: `KENTOS_SHOTS_ONLY=giris-oncelik cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-giris-oncelik-*`: Shift basılı, Orto geçici açık, Kutupsal askıda).
