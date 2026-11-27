# ADR 8175: 3B görünüm paftada: aksonometri, perspektif, kesit perspektifi

- **Durum:** kabul edildi ve uygulandı (27 Kasım 2026).
- **Plan:** [plan/mimari-bim.md](../../plan/mimari-bim.md) MB-13. Karar: [ADR 8171](8171-mimari-3d-section-quality-and-missing-tools.md)
  §2.6.
- **İstek** (sahip, 27 Kasım 2026): "mimari proje çizimlerinde birçok eksik aracımız var". Araştırma raporunun 5.
  bölümü: 3B görünümü paftaya koymak piyasadaki bütün araçlarda var.
- **Sözleşmeler:** değişmedi. Pafta kitabı resmi var olan Resim öğesi ve varlık olarak tutar (ADR 0251); `.kcad` ve
  `.kpafta` aynı.

## §1 Sorun

- **Pafta 3B görünüm taşıyamıyordu.** Pafta yalnız planın görünüm penceresini, yazıyı ve resmi alıyordu. 3B
  görünümün bir resmini almanın da yolu yoktu: 3B penceresi yalnız glTF dışa aktarıyordu.
- **3B görüntüleyici yalnız perspektifti.** Mimarın sunum paftalarında en çok kullandığı aksonometri (paralel
  izdüşüm) yoktu.
- **Kesit perspektifi** görüntüleyicide vardı (kesit ve kapakları, ADR 8131 ve 8171 §2.2), ama paftaya
  taşınamıyordu.

## §2 Araştırma

### Piyasa

- **Revit:** 3B görünüm (perspektif ya da izometrik) bir görünüm olarak paftaya konur ve canlıdır; kesit kutusuyla
  kesit perspektifi olur.
- **Archicad:** 3B belgesi ve aksonometri yerleşime görünüm olarak konur; model değişince güncellenir.
- **SketchUp LayOut:** model görünümü paftada canlıdır; raster, vektör ya da karma çizilir.
- **Ortak nokta:** paftadaki görünüm modelin kendisidir. Görünümün izdüşümü, yönü ve kesiti onunla saklanır.

### Mevzuat

- PAİY 57'nin saydığı mimari proje çizimleri (vaziyet, kat planları, kesitler, görünüşler) arasında 3B görünüm yok
  (`docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`). EK-4'te 3B görünüm için bir kural da yok.
- 3B görünüm avan ve sunum paftalarının işi. Ruhsat takımının bir şartı değil; bu yüzden ön denetime girmez.

### KentOS'ta olan

- **Ortak kamera eksenölçeri biliyor:** `kentos_scene3d::Projection::Orthographic`. İzdüşüm matrisi, görüş hacmi ve
  tıklama ışını hazır. Görüntüleyicinin yörünge kamerası yalnız perspektif kuruyordu.
- **Ekran dışı çizici:** `kentos_ui::snapshot::Snapshot`, iced'in ekran dışı GPU çizicisi. 3B gölgelendirici
  öğesini de çiziyor (mimari probları bunu kullanıyor). Masaüstü onu zaten "snapshot" özelliğiyle bağlıyor.
- **Pafta resmi:** Resim öğesi bir varlığın SHA-256'sını tutar; baytları kitapla saklanır (ADR 0251). Bir resim en
  çok 4 MB olabilir (`ASSET_MAX_BYTES`).

## §3 Karar

### §3.1 Aksonometri

- Görüntüleyicinin şeridine **Aksonometri** anahtarı (Saydam'ın ardından). Açıkken kamera ortografik, kapalıyken
  perspektif. Kesit, yön ve dönüş aynı kalır.
- **Görüntü boyu aynı kalır:** ortografik görünümün yüksekliği, perspektifin hedefte gösterdiği yüksekliktir
  (`2 · uzaklık · tan(FOV/2)`). Anahtar açılıp kapandığında hedefin çevresindeki nesneler aynı boyda görünür;
  tekerlek yine yaklaştırır (uzaklık yüksekliği değiştirir).
- **Göz modelin önünde:** ortografik görüş hacmi gözde başlar. Göz hedeften `uzaklık + erişim + 1 m` geride durur;
  erişim, modelin kutusunun hedefe en uzak köşesidir. Böylece yaklaşınca modelin arkası kesilmez. Derinlik
  `geri + erişim + 1 m`.
- **Yer:** `kentos_model_view::OrbitCamera::axonometric`. Görüntüleyici (`kentos_model_view_ui::Viewer`) anahtarı
  tutar; tıklama, ölçme ve kesit tutamaçları aynı kameradan okur. IFC görüntüleyicisi de aynı görüntüleyiciyi
  kullandığı için Aksonometri onda da var.

### §3.2 Paftaya koy

- 3B penceresinin şeridinin sonunda, Katlar'ın yanında **Paftaya koy**.
- **Görünüm olduğu gibi çizilir:** izdüşüm, yön, kesit ve kapakları, gizlenen ve saydam olanlar. Yalnız ekranın
  ihtiyacı olanlar çıkar: işaretli nokta, ölçüler, kesit tutamaçları.
- **Seçim önce bırakılır:** seçili nesnenin vurgusu sahnenin içine işlenmiş. Kâğıtta mavi bir duvar istenmez.
- **Çözünürlük:** pencerenin biçiminde, uzun kenarı 2400 piksel (20 cm'de 300 dpi), beyaz zemin üstüne.
  - Pencerenin mantıksal boyutu korunup ölçek büyütülür. Görüntünün çerçevesi ekrandakinin aynısıdır.
  - PNG 4 MB'ı aşarsa önce 1800, sonra 1350 piksel denenir. Sonunda da sığmazsa söylenir.
- **Sahne parça parça GPU'ya yüklenir:** "bekleyen" işareti düşene kadar en çok 40 kez yeniden çizilir.
- **Arka planda:** çizim kendi iş parçacığında yapılır, arayüz donmaz; sınamada eşzamanlı koşar (3B modelin kuruluşuyla
  aynı kalıp).
- **Adı görünümü söyler:** "3B görünüm, aksonometri, kuzeydoğudan" ya da "3B görünüm, perspektif, kuzeydoğudan,
  kesit". Ad hem varlığın hem öğenin adıdır; ikinci kez aynı ad "… 2" olur.

### §3.3 Paftada yeri

- Resim **öndeki paftaya** konur. Önde pafta yoksa araç bunu söyler ve hiçbir şey yapmaz. Şablondan yeni pafta
  kurmak (Kat planı paftaları'nın yolu) soruları, kimlikleri ve şablonu gerektiriyordu; ilk sürümde yok.
- **Yer:** paftanın basılabilir alanının sol yarısında. Orada bir resim varsa sağ yarıda, ikisi de doluysa
  ortada. Sunum paftasında iki görünüm yan yana durur (aksonometri ve kesit perspektifi), üst üste binmez.
- **Boyut:** yarının genişliğinin 9/10'u ve alanın yüksekliğinin 6/10'u içinde, biçimi korunarak. Sığdırma
  "İçine sığdır", kırpma açık.
- Tek geri alma adımı ("3B görünüm"); yeni öğe seçili gelir, Öznitelikler'de taşınır ve boyutlanır.
- **Yeri:** `kentos_sheet_ui::Designer::place_picture` (`crates/sheet-ui/src/placed_picture.rs`).

### §3.4 Neden resim, neden canlı değil

- **Vektör aksonometri** gizli çizgi kaldırma ister: 3B yüzleri izdüşürüp birbirini örtenleri ayırmak. Kesit ve
  görünüş çizicisi bunu yalnız yatay bakışlar için yapıyor. Eğik bakış ayrı ve büyük bir iştir.
- **Canlı görünüm** paftanın çizicisinin 3B çiziciyi çağırmasını ister. Pafta çekirdeği (`kentos-sheet`) ortak ve
  web derlemesine de giriyor; GPU çizicisine bağlanamaz. Masaüstü yeniden çizebilirdi, ama bunun için görünümün
  tanımı (kamera, izdüşüm, kesit) öğeyle saklanmalı. Bu sözleşme değişikliğidir (Kalanlar).
- Resim bugün işe yarar: sunum paftası, avan, rapor. Model değişince yeniden konur.

### §3.5 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | 3B görünüm ve pafta Mimari'nin. |
| IFC görüntüleyicisi | Aksonometri var, Paftaya koy yok | Aynı görüntüleyici; IFC dosyasının paftası yok. |
| Harita, Yol | yok | Yol'un 3B görünümü ayrı (arazi ve koridor); bina görüntüleyicisi değil. |
| MCP, Python, eklenti, yapay zekâ ajanı | yok | Ekran dışı GPU çizimi masaüstünün; pafta komutları resmi zaten alıyor (varlık + öğe). |

## §4 Kanıt

- **Kamera** (`kentos-model-view`, `the_axonometric_frames_the_target_as_the_perspective_does_and_sees_the_whole_model`):
  - Hedef iki izdüşümde de ortada.
  - Bakışa dik, hedefin derinliğindeki bir nokta ikisinde de aynı yerde: hedefte ölçek aynı.
  - Birbirinden uzak iki eşit dikey eşit boyda; perspektifte değil.
  - Modelin hedefin arkasındaki en uzak noktası gözün önünde.
- **Görüntüleyici** (`aksonometri_draws_with_parallel_lines_still_picks_and_its_picture_is_the_window_s`):
  - Anahtar kamerayı ortografik yapar; tıklama yine ikinci duvarı seçer.
  - Resim ekranın boyunda; ölçü, tutamaç ve işaret taşımaz.
  - Ad izdüşümü, yönü ve kesiti söyler.
- **Pafta** (`placed_picture::tests`):
  - Önde pafta yokken reddedilir.
  - Birinci resim sol yarının ortasında, biçimi korunur. İkincisi aynı varlıkla sağ yarıda, üst üste binmez.
    Üçüncüsü ortada.
  - Tek geri alma adımı "3B görünüm"; öğe seçili.
- **Masaüstü** (`scene3d_building_tests::aksonometri_turns_the_camera_and_paftaya_koy_asks_for_a_sheet_in_front`):
  3B penceresinde Aksonometri kamerayı ortografik yapar; önde pafta yokken Paftaya koy hiçbir şey koymaz, ne
  yapılacağını söyler.
- **Gerçek pencere** (`.run/gui-evidence/mimari-bim/{once,sonra}/mb13-*`; Xvfb, lavapipe):
  - Kesit binası ve çerçevesi; KATPLANLARI ile A-101 paftası açıldı.
  - 3B penceresinde Aksonometri ile Paftaya koy'a basıldı, sonra doğu–batı kesitiyle perspektif koyuldu.
  - A-101'de iki resim yan yana: "3B görünüm, aksonometri, kuzeydoğudan", "3B görünüm, perspektif, kuzeydoğudan,
    kesit". %57 yakınlıkta keskin.
- **Sürücü:** `scripts/usage/real/driver.py` `fit` artık pencereyi öne de alır (`stack_mode=X.Above`). Pencere
  yöneticisi olmayan ekranda ikinci pencere (3B görünüm) birincinin üstünde kalıyordu; ana pencereye dönen senaryo
  onu göremiyordu.

## Kalanlar

- **Canlı görünüm:** görünümün tanımı resim öğesiyle saklanır, model değişince "Yenile" yeniden çizer (§3.4).
- **Vektör aksonometri:** gizli çizgi kaldırmayla çizgi çizim; PDF'te çizgi olarak basılır.
- **Önde pafta yokken** varsayılan şablondan "3B görünüm" paftası kurulur.
- **Boşluk kırpma:** resim pencerenin çerçevesiyle gelir; modelin çevresindeki boşluk kırpılmaz. Önce Sığdır
  önerilir.
- **Saydam zemin:** resim beyaz zemin taşır; altındaki çizimi örter.

## Uygulama

- `crates/shared/model-view/src/camera.rs`: `OrbitCamera::axonometric`.
- `crates/model-view-ui/src/viewer.rs`, `viewer_view.rs`: Aksonometri anahtarı, `camera`, `reach`, `picture`,
  `picture_name`.
- `crates/sheet-ui/src/placed_picture.rs` (yeni): `Designer::place_picture`.
- `apps/desktop/src/shell/scene3d_sheet.rs` (yeni): Paftaya koy, ekran dışı çizim, yerleştirme;
  `scene3d_window.rs` (olaylar), `scene3d_view.rs` (düğme).
- `scripts/usage/real/driver.py`: `fit` pencereyi öne alır.
