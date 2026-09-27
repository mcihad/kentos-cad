# ADR 0092: Masaüstünde Stil yöneticisi

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** docs/STYLE.md §5, §7; CLAUDE.md §4.4, §4.8, §4.10; DESIGN.md §7.9.1; ADR 0090 (stilli çizim), 0091 (Katman stili)
- **Sahibin yönü (27 Eylül):**
  - web'in stil pencereleri masaüstüne en az bire bir gelir, olabildiğince iyileşerek;
  - iyileşmeler web'e de gelir;
  - görsellik en önemli işlerdendir.

## Bağlam

- **Web:** `StyleManager` (`ui/style/StyleManager.ts`, `managerDetails.ts`, `styleFiles.ts`):
  - solda kaynaklar ve kategoriler, ortada kartlar, sağda seçilenin ayrıntıları;
  - arama, türler, kopyalama, silme;
  - `.kstil` dosyası ve pano ile içe ve dışa aktarma; PNG ve JPEG görüntü alma;
  - seçme kipi: Katman stili'nin yuvası ya da seçili nesneler için.
- **Kitaplığım** web'de tarayıcıdadır (`kentos.styles.v1`).
- **Masaüstü:** `style.manager`, `style.assign` ve `style.clearSymbol` web'e yönlendiriyordu. Katman stili yuvasının “Kitaplıktan seç…” ve “Kitaplığıma kaydet” öğeleri sönüktü (ADR 0091).
- **Web penceresinin kusurları:**
  - yeniden adlandırılan kategoride liste eski adda kalıyordu: “Bu kategoride sembol yok”;
  - kopya seçiliyordu ama liste yerinde kaldığından kartı görünmüyordu;
  - “Üzerine yaz” notu sistem öğelerinin kopya olarak alındığını söylüyordu. `importStyles` onları atlar (`kstil.json`);
  - Kaynak ve Açıklama boşaltılınca eski metin kalıyordu: `library.update` `undefined`'ı JSON kopyasında düşürür. Bu `style/library.ts`'tedir; web ajanına bildirildi.

## Karar

### Hesap: `kentos-native-style`

- **`library`**: `style/library.ts`'in karşılığı.
  - Kaynaklar eklenme sırasını korur.
  - Ağaç: kardeşler kategorinin `order`'ı, sonra Türkçe ada göre sıralanır.
  - Arama Türkçe harfleri katlar. Ad, yol, etiket ve sembolün açıklamasında arar; arama boş kategorileri bırakır.
  - Ekle, güncelle (`null` alanı siler), yeniden adlandır (boşsa “Adsız”), taşı, sil.
  - Kopyala: yeni kimlik (`u…`, `p…`) ve “(kopya)” adı. Projeye kopyalanan sembolün kullandığı Kitaplığım çizimleri de gelir.
  - Kategori ekle ve yeniden adlandır.
  - `version` her değişiklikte, `assets_version` yalnız çizim ve görüntü değişince artar.
- **`file`**: `.kstil` (`style/file.ts`; biçim `kentos-style`, sürüm 1).
  - Dışa aktarma sembollerin kullandığı çizimleri de yazar.
  - Okurken sürüm, öğe, sembol ve kategori denetlenir; bir kusur varsa dosyadan hiçbir şey alınmaz.
  - SVG temizliği `sanitizeSvg`'nin düzenli ifadelerinin elle yazılmış karşılığıdır. Web'deki gibi yalnız `url(`'den hemen sonra `#` gelen iç başvuru kalır; öbürleri `none` olur.
  - İçe aktarma: çizimler önce gelir, yeni kimlik alan çizime sembollerin başvurusu da taşınır. Çakışma kipleri:
    - Kopya olarak al;
    - Üzerine yaz (yalnız hedef kitaplıktakilerin; sistemdekiler ve öbür kitaplıktakiler atlanır);
    - Atla.
  - İki platform `fixtures/style/v1/kstil.json`'u geçer (`tests/kstil.rs`).

### Kitaplığım dosyada

- Yeri `$XDG_DATA_HOME/kentos-cad/kitaplik.kstil`'dir (yoksa `~/.local/share/kentos-cad/`; `style/user_library.rs`).
- Biçimi web'in `.kstil`'idir; dosya olduğu gibi paylaşılabilir.
- Her değişiklikte bütünü yazılır: önce geçici dosyaya, sonra yerine taşınarak.
- Okunamayan dosya `kitaplik-okunamadi-<zaman>.kstil` olarak ayrılır, üzerine yazılmaz; uygulama bir kez uyarır. Ayrılamazsa dosya yerinde kalır ve oturumdaki değişiklikler kaydedilmez.
- Projenin kitaplığı çizimdedir (`ProjectStyles`). Değişikliği `Document::set_styles` yazar: düzenlemedir, geri alma adımı değildir (kitaplığın geri alması yok, web'deki gibi).
- Açık çizim yokken proje kitaplığına yazılmaz; ilgili menü öğeleri sönüktür ve nedenini söyler.

### Pencere: `apps/desktop/src/style/manager/`

- Web'in penceresi, aynı sözler ve aynı düzenle:
  - `mod.rs` durum, `update.rs` olaylar, `assign.rs` sembol verme, `files.rs` dosyalar, `view.rs` ve `details.rs` görünüş;
  - çubuk: arama, türler, Yeni sembol, İçe aktar, Dışa aktar;
  - ağaç: Sistem, Kitaplığım, Proje, her düğümde öğe sayısı. Sağ tık menüsü: Yeni alt kategori, Yeniden adlandır (F2), Dışa aktar (n);
  - kartlar: yalnız görünen satırlar kurulur; aramada kaynak da yazar;
  - ayrıntılar: örnekli önizleme, alanlar, eylemler;
  - alt satır: durum, Kapat; seçme kipinde Seç.
- **Boyut:** web'in 1240 × 860'ı; gövde web'inki kadar geniştir.
  - Pencere, uygulama penceresinin yüksekliğinin en çok %92'si, kenarlarda 60 px pay kalır.
  - Dar pencerede ya da büyük yazıda ağaç ve ayrıntılar listeye yer bırakır.
  - Arama kutusu düğmelerden önce daralır; önizleme sütuna sığar.
  - İçe aktarmanın çakışma seçimi sığmazsa alt alta dizilir.
- **Tıklama:** web'in `clickToggles`'ı. Ağaçta bir düğüme basmak onu açar; seçili düğüme ikinci basış kapatır. Kartta çift tıklama seçme kipinde seçer, değilse öğeyi düzenleyicisinde açar.
- **Alanlar** web'deki gibi yazılırken değil, bırakılınca yazılır: Enter, başka öğe, arama, kategori, kapatma. Açıklama çok satırlıdır; Ctrl+Enter ya da Esc kaydeder.
- **Sembol ver:**
  - `style.assign` seçme kipinde açılır: tür seçili nesnelerin ilkinin türüdür, başlangıç onun sembolüdür.
  - Seç `cad.entities.set` ile tek adım yazar (“Sembol: ad”); kilitli katmandakiler atlanıp sayılır.
  - `style.clearSymbol` sembolü kaldırır.
  - Katman stili yuvasında:
    - “Kitaplıktan seç…” yöneticiyi yuvanın türüyle, yuvanın sembolünde açar;
    - Seç yuvaya `{ref}` yazar; Esc Katman stili'ne döner;
    - “Kitaplığıma kaydet” stildeki sembolü Sembollerim'e yazar; yuva artık onu gösterir.
- **Resimler** Katman stili'ninkilerle aynı yoldan çizilir (`thumbs.rs`). Anahtar kitaplığın yalnız çizim sürümünü içerir; ad değiştirmek resimleri yeniden kurdurmaz.
- **Henüz olmayanlar:**
  - Düzenle ve Yeni sembol sembol tasarımcısını, SVG çizimi SVG düzenleyicisini ister. Sönüktürler ve nedenini söylerler.
  - JPEG görüntüsü alınır ama masaüstünde çizilmez; bunu da söyler. JPEG çözücüsü yeni bir bağımlılıktır; sahibin onayını bekler.

### İyileşmeler

- **İki platformda:**
  - yeniden adlandırılan kategoriyi liste ve açık düğümler izler;
  - kopya yapılınca liste kopyanın yerine gider, kartı seçili görünür;
  - “Üzerine yaz” notu yapılanı söyler;
  - Delete seçili (Kitaplığım'daki ya da projedeki) öğeyi silmeyi sorar.
- **Masaüstünde:**
  - Kategori alanında Enter öğeyi yeni yerinde gösterir;
  - boşaltılan Kaynak ve Açıklama öğeden silinir;
  - F2 seçili kategoriyi adlandırır;
  - Enter silme sorusunu onaylar, seçme kipinde seçer;
  - bozuk PNG nedeniyle söylenir (web sessiz kalıyordu);
  - menülerde ipuçları açıklama satırıdır.

## Sonuçlar

- **Denetimler:**
  - `kstil.json` iki platformda geçer;
  - `library` birim sınamaları;
  - Kitaplığım'ın dosyası: yazılır, yeniden okunur, bozuk dosya ayrılır (`user_library.rs`);
  - belgenin `set_styles`'ı düzenlemedir, geri alma adımı değildir (`crates/native/domain/tests/document.rs`);
  - pencere kullanıcının sürdüğü gibi `manager/tests.rs`'te sınanır (13 durum). Kapsadıkları: arama ve türler, ağaç, kopya ve dosyası, alanlar, silme sorusu, sembol verme ve geri alma, Katman stili yuvası, projenin kitaplığı, içe aktarma ve çakışma, kategoriler, görüntüler, dosya adları ve pencerenin her durumunun kurulması.
- **Resimler:**
  - `cargo test -p kentos-desktop style::screens::manager_screens -- --ignored --nocapture` masaüstünü çeker (`.run/shots/smgr-*.png`): 15 durum; iki boy, iki tema ve büyük yazıyla bir boy daha;
  - `node scripts/e2e/shots.mjs stylemanager` (`apps/web`) web'i çeker;
  - web ajanının 15 sahnelik başvuru resimleri karşılaştırmada kullanıldı.
- **Başarım:**
  - ağaç ve liste kitaplığın sürümüyle önbellektedir;
  - ızgara yalnız görünen satırları kurar (776 sistem öğesi);
  - menünün sayısı menü açılınca hesaplanır.
