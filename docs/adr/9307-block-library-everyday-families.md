# ADR 9307: Blok kitaplığına günlük aileler: ofis, okul, sağlık, erişilebilir ıslak hacim, peyzaj

- **Durum:** kabul edildi (2026-11-24). todos-new.md D6.
- **Kural:** CLAUDE.md, “Modül ve otomasyon denetimi” ve §25. Değer uydurulmaz: kaynağı olmayan ölçü “tipik” diye açıkça yazılır.
- **Web:** iptal edildi; yok. **`.kcad`:** değişmedi (kitaplık `kentos.blocks` sürüm 5'te kaldı). **Komut:** yeni komut yok;
  bloklar mevcut `cad.blocks.edit` (içe aktarma, `name: kentos-temel`) ve `cad.blocks.insert` ile gelir.

## Bağlam

Temel kitaplıkta (`catalog/v1/blocks/kentos-temel.kblok`) 40 blok vardı: kapı, pencere, ev mobilyası, ıslak hacim, mutfak,
asansör, otopark yeri, işaretler. Bir mimarın ve bir ölçmecinin her gün çizdiği ofis, okul, sağlık donatısı, erişilebilir
ıslak hacim ve peyzaj yoktu. Kitaplık `scripts/blocks/library.py` ile yazılır (`--check` karşılaştırır) ve Rust'ta
`kentos_native_application::blocks_library::BUILTIN_LIBRARIES` içinde gömülüdür; panel, Python, MCP ve ajan aynı
kitaplığı bu yoldan alır.

## Karar

Yirmi bir blok eklendi (kitaplık 40'tan 61'e çıktı). Hepsi metre cinsinden, planda, gerçek boyda; sandalyeli olanlarda
Görünüş parametresi (Sandalyeli / Yalnız masa), boşluk zarfı olanlarda Görünüş (Sade / Boşluk zarfı), uzunluğu değişenlerde
doğrusal parametre var. Blok katmanı boş bırakılır: blok, eklendiği katmana yerleşir (mevcut bloklarla aynı). Taban noktası
her bloğun açıklamasında yazılıdır (duvara dayananlarda duvardaki orta ya da sol köşe; masalarda merkez ya da arka sol köşe).
Yapı elemanı sınıfı (IFC) olanlar `ELEMENTS` tablosuna eklendi (masa, sıra, yatak, lavabo); ağaç, araç, dönüş alanı ve
işaretler çizim yardımcısıdır, sınıfsızdır.

### Ölçüler ve kaynakları

Kaynaklar yalnız burada anılır (blok açıklamalarında kuruluşun adı ve madde vardır, ADR atfı yoktur).

| Blok | Ölçü (m) | Kaynak |
|---|---|---|
| Toplantı masası 4 / 6 / 8 kişilik | 1,80 × 0,90 / 2,40 × 1,00 / 3,00 × 1,10; sandalye 0,45 × 0,45 | Tipik; yazılı kaynağı yok |
| İş istasyonu | 1,60 × 1,60 L masa, bölme 0,05 | Tipik |
| Arşiv dolabı | 1,00 × 0,40; genişlik 0,60–2,40 (0,20 adım) | Tipik |
| Öğrenci sırası tek / çift | 0,60 × 0,45 / 1,20 × 0,45; sandalye 0,40 | Tipik. Aranan okul sırası ölçüsü kamuya açık Millî Eğitim Bakanlığı metinlerinde bulunamadı |
| Öğretmen masası | 1,20 × 0,60 (yükseklik 0,75) | Millî Eğitim Bakanlığı, Mesleki ve Teknik Eğitim Genel Müdürlüğü donatım listesi (“Mobilya ve yer mekân tasarımı analizi”, ookgm.meb.gov.tr, 2017): öğretmen masası 120 × 60 × 75 cm. Aynı listede teknik resim atölyesinin öğretmen masası en az 120 × 80 |
| Yazı tahtası | 3,00 × 0,04; uzunluk 1,20–6,00 (0,60 adım) | Tipik; aynı kaynakta tahta yalnız “etkileşimli tahta, en az 65 inç” diye geçer, planda ölçüsü yoktur |
| Hasta yatağı | 0,90 × 2,00 | Tipik |
| Muayene masası | 0,60 × 1,90 | Tipik |
| Lavabo (sağlık) | 0,60 × 0,50 | Tipik |
| Erişilebilir klozet | asma klozet 0,37 × 0,54; eksen–yan duvar 0,46; boş yanda net en az 0,90; bar Ø0,035, duvardan 0,04; sabit bar yan duvarda, hareketli bar boş yanda | Diyanet İşleri Başkanlığı, Erişilebilirlik Kılavuzu (engelsiz.diyanet.gov.tr, “Engelsiz Kuran Kursu Fiziki Şartlar Kılavuzu”), 6.2.2 Tip 2 ve 6.3.4: “klozetin en az bir tarafında en az 90 cm net boşluk, diğer tarafta klozet orta noktası ile duvar arası en az 46 cm”; bar çapı 32–38 mm, duvarla arası 4 cm. Bar boyları (0,80) ve asma klozetin 0,37 × 0,54'ü tipiktir; kılavuzun şekilleri okunamadı |
| Erişilebilir lavabo | 0,60 × 0,50, yuvarlak kenarlı, dolapsız; iki yanında bar; diz boşluğu ≥ 0,205 derin (alt 0,685); üst en çok 0,86 | Aynı kılavuz, 6.5: diz boşluğu yüksekliği en az 68,5 cm, derinliği en az 20,5 cm; lavabo üst yüzü en çok 86 cm; “iki yanında tutunma çubukları”. Lavabo eni, bar boyu (0,40) ve önündeki 0,80 × 0,90 yaklaşım boşluğu tipiktir |
| Dönüş alanı Ø150 | Ø1,50 kesikli daire | Aynı kılavuz, 6.1: “klozet, lavabo ve donatılar hariç en az 150 cm çapında manevra alanı”. Planlı Alanlar İmar Yönetmeliği 29/9–10 yalnızca TS 9111'e uyulmasını ister; TS 9111 metni ücretlidir ve bu işte okunmadı (todos-new.md D5) |
| Ağaç Ø3 / Ø5 / Ø8 m | taç çapı 3 / 5 / 8; gövde Ø0,20 / 0,30 / 0,40 | Tipik; yazılı kaynağı yok |
| Çalı | taç Ø1,20 | Tipik |
| Otomobil | 1,80 × 4,50 | Tipik. Park yeri 2,40 × 4,90'dır, engellide 3,50 (Otopark Yönetmeliği 5/1-h-8, 9; zaten `Otopark yeri` bloğunda) |
| Bisiklet park yeri | 0,70 × 2,00 | Bisiklet Yolları Yönetmeliği (RG 12.12.2019, 30976) 17/9-a: iki bisiklet arası en az 70 cm, bisikletin boyuna park yeri en az 200 cm |

Erişilebilirlik Kılavuzu'ndaki ölçüler Yapı Denetimi Uygulama Yönetmeliği'ndeki 800 m² ayrımına göre Tip 1 (klozetin iki yanı
boş) ve Tip 2 (tek yanı boş) tuvalet planlarıdır. Eklenen blok Tip 2'dir; Tip 1 için bloğu aynalayıp iki yan boşluğu
elle çizmek gerekir, ayrı blok yapılmadı (kaynağı çizimden okunmadı). Boşluk zarfı bir çizim yardımcısıdır, mevzuat denetimi
değildir (bkz. ADR 0702); erişilebilirlik denetimi D5'te bekler.

### Modüller

Kitaplık, projenin modülüne göre sunulmaz: Bloklar paneli ve “Blok ekle” aracı modülün profilindedir (`Panel::Blocks`,
`ToolOption::Blocks`). Matris sınaması (`apps/kentos-cad/tests/programs/module_surface_tests.rs`) zaten her hücreyi sınar.

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Mimari | var | ofis, okul, sağlık, erişilebilir WC ve araç/ağaç plan çizimi bu modülün işidir |
| Harita | var | ağaç, çalı, otomobil, bisiklet yeri vaziyet planında kullanılır; ofis ve okul donatısı da yerleşim planlarında çıkar (kitaplık tek, ayıklanmaz) |
| Yol | var | Bloklar paneli Yol'da vardır; ağaç, otomobil ve bisiklet yeri kesit ve plan çevresinde kullanılır |
| CBS | yok | CBS projesinde Bloklar paneli ve Blok ekle aracı yok (“Bloklar KentOS GIS'te yok”); CBS katman verisi veritabanındadır, proje dosyası çizim nesnesi taşımaz (§25 kural 5) |

### Kanallar

Yeni komut yok: yirmi bir blok, kitaplık dosyasının içinde olduğundan mevcut komutların kanallarının hepsi hazırdır.

| Kanal | Var / yok | Nasıl ulaşılır |
|---|---|---|
| Arayüz | var | Bloklar paneli (kartlar, kategori süzgeci), Blok ekle; kitaplık “Blokları al” ile ya da ilk yerleşimde alınır |
| Python | var | `kentos.cad.blocks.edit` (içe aktarma, `name: kentos-temel`), sonra `kentos.cad.blocks.insert` (`python/kentos/cad/blocks.py`; `at`, `layer_id`, blok adı) |
| MCP | var | aynı komutlar araç katalogundan: `cad.blocks.edit`, `cad.blocks.insert` (`cad.run`, Mimari'de `arch.run`; yazan komut önce `plan`, sonra `execute`) |
| Yapay zekâ ajanı | var | MCP ile aynı araç katmanı (`kentos_headless::agent_tools`); `catalog.search` bloğu bulur |
| Eklenti | var | `document.write` izniyle `run`/`call`; `cad.blocks.*` `not_for_extensions` listesinde değil (geri al, yinele ve dosya işleri kapalıdır) |

Eksik yok; hiçbir kanal için yeni bir şey yapılmadı.

## Doğrulama

- `python3 scripts/blocks/library.py --check`: “blok kitaplığı tutarlı: 61 blok”; `python3 scripts/blocks/office_library.py --check` güncel.
- Her yeni blok adıyla ve Mimari, Harita, Yol işlerinde `cad.blocks.insert` ile yerleşir:
  `cargo test -p kentos-headless --test headless block_families` (`crates/native/headless/tests/headless/block_families.rs`).
- Resimler (`.run/gui-evidence/`): `blok-aileleri` (bir planda yerleşmiş aileler) ve `blok-aileleri-kartlar` (Bloklar paneli, kategoriler):
  `KENTOS_SHOTS_ONLY=blok-aileleri,blok-aileleri-kartlar cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture`.
- Açık: TS 9111 metni (ücretli) okunmadı; okul sırası, hasta yatağı ve bitki ölçüleri kaynaksız, “tipik” diye işaretli.
