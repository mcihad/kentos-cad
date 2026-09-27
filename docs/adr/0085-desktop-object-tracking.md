# ADR 0085: Masaüstünde Nesne izleme

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.7; TODOS.md `UX-07`; ADR 0018 (izler), 0021 (masaüstü araç oturumu), 0029 (kenet), 0083 (izlerin `prompt` ve `logged` anahtarları).
- **Kaynak:** web'in `viewport/ViewportController.ts` izleme bölümü, `viewport/objectTracking.ts`, `viewport/overlay.ts` `drawObjectTracking`, `tools/tracking.ts` `constrainPoint` ve `pointFromText`. Hesap ortak çekirdekte: `crates/shared/geometry-core/src/tools/object_tracking.rs`. Web ajanının `c624e09`'u (`object-tracking` izi, `rest` adımı, `track` ve `trackPoints` beklentileri) `ddc8bab`'da main'e geldi; masaüstü onu bekleyenler listesinde tutuyordu.

## Bağlam

- **Web'de Nesne izleme:**
  - Bir kenet noktasının üstünde 350 ms durmak o noktayı alır; alınmış noktada yeniden durmak bırakır.
  - Noktalardan yatay ve dikey hizalar çıkar; kutupsal izleme açıksa her açı adımında da.
  - Kenet yokken imleç en yakın hizaya ya da iki noktanın hizalarının kesişimine kilitlenir.
  - Hizadayken yazılan sayı noktadan hiza boyunca mesafedir.
  - En çok üç nokta tutulur; noktalar komutundur.
  - Shift+F3 izlemeyi kapatır, noktaları tutar.
- **Masaüstünde:** yoktu. Durum çubuğundaki "İzleme" ve Shift+F3 "web'de var" diyordu.

## Karar

### Araç oturumu (`kentos-interaction`)

- **`object_tracking::ObjectTracking`:** web'in görünümde tuttuğu durum.
  - alınan noktalar (en çok 3, en eskisi önce gider);
  - imlecin kilidi (`TrackHit`);
  - süren bekleme, numarasıyla (`dwell`, `dwell_due`).
  - İzleme noktası olabilen kenet türleri web'inkilerdir: uç, orta, merkez, nokta, çeyrek, kesişim.
  - Hizalar ve kesişim çekirdektendir (`track_point`, `track_angles`, `along_track`).
- **Oturum:** `Session::tracks` izlemenin ne zaman çalıştığını söyler (kenetlenen bir komut sürerken). `Session::snap_from` komutun son noktasını verir (yalnız kesişimlerde).
- **İşaretçi:** kenet yoksa kilitlenen noktadadır (`Pointer::tracked`). Kenetlenen nokta gibi o da tamdır: orto ve kutupsal izleme onu kaydırmaz (web'in `constrainPoint`'i).
- **Yazılan mesafe:** `Context::track_along`. Yazılan tek sayı, hizadayken, hiza boyunca mesafedir. Bu yirmi üç aracın hepsinde geçerlidir, web'in `pointFromText`'i gibi.
- **Ayar:** `Draft::tracking` (`drafting.tracking`). Ayar artık masaüstünde de barınır; oturum ayarıdır, varsayılanı açıktır.

### Masaüstü

- **Bekleme:** kenet noktasında durmak 350 ms'lik bir bekleme başlatır; kendi iş parçacığında sayılır, bilgi kartının gecikmesi gibi. Süre dolduğunda imleç hâlâ oradaysa nokta alınır ya da bırakılır.
- **Komut değişince** noktalar gider.
- **İşaretler** (`marks.rs`), web'in `drawObjectTracking`'i gibi kenet renginde:
  - her alınmış noktada 10 px artı (1,5 px);
  - kilidin hizaları kesikli 3/4, %85;
  - imlecin yanında "İzleme 12.063 m < 0°" ya da "İzleme: kesişim", alanın renginde haleyle.
  - Kenet varken yalnız noktalar görünür.
- **Aç ve kapat:** `draft.tracking` (Shift+F3). Durum çubuğundaki "İzleme" durumunu gösterir. Kapalıyken noktalar kalır, açılınca yine hiza verirler.

### İz oynatıcısı

- **`rest` adımı:** imleci noktaya getirir. Beklemeyi oynatıcının saati geçirir: web 500 ms bekler, oynatıcı saatini 500 ms ilerletir ve beklemenin mesajını kendisi verir.
- **Zamanlayıcı yok:** oynatıcıda bekleme zamanlayıcısı kurulmaz. Böylece bir kenetten geçen `move` web'deki gibi (30 ms) nokta almaz.
- **Beklentiler:** `trackPoints` ve `track` web'in kuralıyla karşılaştırılır: noktalar `clickTolerance` içinde, açılar tam.
- **Bekleyenler listesi:** `object-tracking` bekleyenler listesinden çıktı; liste boş.

## Doğrulama

- **İz:** `object-tracking` masaüstünde üç varyantta (US, Türkçe Q, 2× ekran) geçiyor:
  - dwell ile nokta alma ve bırakma;
  - yatay hizaya kilitlenme;
  - hizada yazılan 10;
  - iki hizanın kesişimi;
  - Shift+F3;
  - üç nokta sınırı;
  - araçtan çıkınca temizlik.
  Öbür bütün izler de geçiyor.
- **`kentos-interaction` testleri:**
  - durmak alır, yeniden durmak bırakır, uzun durmak geri çevirmez;
  - üç nokta, en eskisi gider;
  - geç kalan bekleme, en yakın nokta kenedi ve kapalı izleme nokta almaz.
- **Görüntüler:** `kentos-cad snapshot … --iz object-tracking --adim 3|9`, koyu ve açık:
  - yatay hizaya kilitlenmiş imleç ve etiketi;
  - iki hizanın kesişimi.
