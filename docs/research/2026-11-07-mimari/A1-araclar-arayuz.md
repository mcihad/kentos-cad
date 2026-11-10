# A1 — KentOS Mimari: araçların ve arayüzün kullanım denetimi

Tarih: 7 Kasım 2026 · Depo: `/home/cihad/Projects/kentos-cad` · Dal: `arch-adv-ileri` (`e40e6dae0`).
Kapsam: kullanıcının gördüğü ve kullandığı taraf (şerit, araçların akışı, istemler, ayar paneli, pencereler, kısayol,
arama, ikon). Veri modeli ve plan maddelerinin durumu bu raporun dışında.

**Yöntem.** Derleme, test ve ekran görüntüsü çalıştırılmadı. Kod, katalog (`catalog/v1/*.json`,
`docs/inventory/web.json`) ve belgeler okundu; katalog üzerinde küçük sayım/arama benzetimi betikleri çalıştırıldı
(`_betikler/`); ikon SVG'leri katalogdan bir tabloya çizildi (`arch-icons.png`). Diskte bulunan yerel görüntülere
bakıldı.

**Kanıt etiketleri** (her iddianın yanında):

| Etiket | Anlamı |
|---|---|
| **[K]** | Koddan/katalogdan okundu; çalıştırılarak doğrulanmadı |
| **[G]** | Bugünkü yerel görüntü (7 Kasım 2026): `.run/gui-evidence/mimari-durum/01…05` ve `.run/shots/kullanim/*` |
| **[EG]** | Eski yerel görüntü (19 Ekim 2026, `.run/shots/arac-*`); güncel derlemeyi kanıtlamaz |
| **[S]** | Katalog üzerinde benim yaptığım sayım/benzetim (`_betikler/arch_cmds.py`, `search_sim.py`, `icon_sheet.py`) |
| **[D]** | Belge: DESIGN.md, plan, ADR, kabul kaydı |
| **[T]** | Tahmin; kod okumasıyla çıkarıldı ama davranış görülmedi |

Yan dosyalar (aynı dizin): `arch_cmds.txt` (Mimari sekmesi ve bağlamsal sekmelerin 82 komutunun katalog kaydı),
`ribbon_cad_dump.txt` (bütün CAD sekmeleri), `arch-icons.png/.svg` (ikon tablosu), `_betikler/`.

---

## 0. Kısa sonuç

1. **Mimari sekmesi 46 komutla tasarlandı, şimdi 78 komut taşıyor; 42'si (%54) açılır liste ya da ▾ arkasında.**
   1440×900'de bütün düğmeler küçük, büyük ana araç yok. Sık işler (Duvarı kaydır/uzat/buda, Hepsini etiketle,
   Mahal numarala) ▾ içinde. Seyrek işler (Aksları/Açıklıkları numarala, İç nokta seç, Mahal kaynakları) görünür.
2. **78 Mimari komutunun hiçbirinde klavye kısayolu yok.** En kısa takma ad 3 harf. F1 listesinde mimari araç
   görünmüyor. DESIGN.md §1.4 ("her aracın kısayolu vardır") ihlal ediliyor.
3. **"pencere" yazınca Pencere yakınlaştır çalışıyor.** `PENCERE` takma adı yakınlaştırmanın, pencere aracınınki
   `PENCEREKOY`. Komut ara'da da ilk sonuç yakınlaştırma.
4. **Esc, Enter'lanmamış çok parçalı duvarı sessizce siliyor.** Duvar aracı bütün zinciri Enter'da yazıyor, Esc
   taslağı mesajsız atıyor.
5. **Modal pencereler kaydedilmemiş düzenlemeyi sormadan atıyor.** 12 mimari pencere modal: Bina programı, Duvar
   stilleri, Kapı-pencere tipleri, Malzemeler, Oda modülleri, Tasarım seçenekleri… Karartılmış zemine bir tık
   onları sormadan kapatıyor. Seçimle çalışan pencereler (modül, seçenek, program) modal olduğu için seçimi
   değiştirmek için kapatıp yeniden açmak gerekiyor.
6. **Terimler çakışıyor.** Duvar kalınlığı istemde "Genişlik", ayar panelinde "Kalınlık", Öznitelikler'de
   "Genişlik". Aynı paneldeki "Kalınlık" çizgi kalınlığı demek. "Döşeme" düğmesi bir yerde döşeme (slab), bir
   yerde döşeme deseni. "Mahal" düğmesi bağlamsal sekmede mahal çizelgesi. "Boşluk" hem açıklık hem şaft.
   "Duvar birleşimi" ile "Duvarları birleştir" ayrı araçlar.
7. **Seçenek harfleri araçtan araca değişiyor.** Genişlik duvarda E, merdivende G; K: Kapat, Kesme, Kalınlık,
   Kullanım, Etiket. Çalışan aracın seçeneği olmayan harf genel kısayolu başlatıp aracı bırakıyor; örneğin
   Merdiven'de E, Esnet'i başlatır [T].
8. **Duvar çizerken imleç etiketi CAD projesinde de "Semt" yazıyor.** Kuzeyden saat yönünde açı gösteriyor;
   doğrusu "Açı" ve doğudan saat yönünün tersi.
9. **Mimari nesneler hep etkin katmana yazılıyor.** Ofis standardının A-DUVAR, A-KAPI… katmanları
   kendiliğinden kullanılmıyor. Mimari sekmesinde katman seçici yok. Bugünkü örnekte 54 nesnenin hepsi
   "Çizim" katmanında [G].
10. **Başlangıç zayıf.** KentOS Mimari Giriş sekmesinde açılıyor; Mimari 6. sekme. Yeni projede kat yok, bu yüzden
    Kat hücresi de yok. Şablon elle uygulanıyor. Duvar stili "Yok" olduğu için plan poşesiz, ince çift çizgi.
11. **Pratik eksikler.** Tek adımda "oda" (duvar + mahal + etiket) aracı yok. Odayı sürükleyerek büyütmenin
    aracı (Duvarı kaydır) ▾ içinde ve bağlamsal Duvar sekmesinde yok. Mobilya/vitrifiye kitaplığı Mimari
    sekmesinde yok; "mobilya, lavabo, klozet" aramaları sonuç vermiyor. Sağ tık menüsü ve üzerine gelme kartı
    mimariye özgü değil. Yeni nesnenin evresi seçilemiyor.
12. **İkonlar.** Üç duvar aracı aynı ikonu (`wall`), beş mahal aracı aynı ikonu (`space`), Etiket ile Mahal
    numarala aynı ikonu (`tag`) kullanıyor. Duvar ölçüleri, Açıklık dağıt ve Duvarı kaydır genel CAD ikonlarını
    ödünç alıyor. Yedi çizelge ikonu birbirine çok benziyor. Sahibin ikon turunda (`icon_tour.rs`) Mimari'den
    tek sahne yok.

---

## 1. Envanter

### 1.1 Şeridin yapısı ve sayıları

- **KentOS Mimari'nin sekmeleri** [G 03]: Dosya, Giriş, Ekle, Açıklama, Değiştir, **Mimari**, Görünüm, Yönet,
  Çıktı. Mimari 6. sırada.
  - Program açılışta 2. sekmeyi (Giriş) ya da düzende son kalan sekmeyi açar [K `app.rs:1720-1724`,
    `modes.rs:140-144`, `layout.rs:250-262`]. Bugünkü görüntüde iş seçildikten sonra Giriş açık [G 02].
- **Mimari sekmesi** [K `catalog/v1/ribbon.json:1930-2275`; S]:

  | Panel | Görünen düğme | Bölünmüş düğme listesinde | Menüde | Panel ▾ (taşma) | Toplam komut |
  |---|---|---|---|---|---|
  | Çiz (`:1934`) | 8 | 0 | 0 | 0 | 8 |
  | Yerleştir (`:1972`) | 8 | 4 | 0 | 0 | 12 |
  | Düzenle (`:2051`) | 9 | 0 | 0 | 11 (`:2091`) | 20 |
  | Belgele (`:2106`) | 6 | 10 | 0 | 5 (`:2212`) | 21 |
  | Proje (`:2221`) | 6 | 0 | 4 (Evre, `:2245`) | 8 (`:2260`) | 17 |
  | **Toplam** | **37** | **14** | **4** | **24** | **78 (benzersiz)** |

  - 24 Ekim kabulünde (ARCH-UX-02) sekme "46 mimari komut" ile tasarlanmıştı [D
    `docs/baseline/2026-10-24-architecture-ribbon/README.md:44`]. O günden beri +32 komut eklendi (%70).
  - ADV-07 "Mevcut beş panelli Mimari şerit yeniden kalabalıklaştırılmasın" diyor [D
    `plan/architecture-advanced.md:340`].
- **"Etiketsiz öğeler"** [K]:
  - Kaynak JSON'da `command` alanı olmayan öğeler bölünmüş düğmelerdir: `{"key":…, "split":[…]}`. Düğmenin yüzünde
    listedeki `title` yazar ve son seçilenin ikonu görünür [K `ribbon_plan.rs:233-242`].
  - Yerleştir'de üç tane var:
    - `archOpenings` "Boşluk ▾": Boşluk, Niş, Baca.
    - `archGrids` "Aks ▾": Aks, Aks ağı.
    - `archStairs` "Merdiven ▾": Merdiven, Döner merdiven.
  - Belgele'de dört tane var:
    - `archDimensions` "Ölçü ▾": Duvar ölçüleri, İç ölçü zinciri.
    - `archSchedules` "Çizelge ▾": Kapı ve pencere, Mahal, Mahal kaplama, Donatı, Bölüm çizelgesi ve Malzeme lejandı.
    - `archMarks` "İşaret ▾": Kesit, Görünüş, Detay, Kuzey oku.
    - `archRevisions` "Revizyon ▾": Revizyon işaretle, Revizyon listesi.
  - Proje'deki `{"menu":"Evre"}` dört radyo seçeneği taşır: Tümü, Mevcut, Yıkım, Yeni. Menünün bloğunun etiketi boş.
- **Düğmenin etiketi** katalogdaki `short` alanıdır, yoksa `title` [K `view.rs:595`]. ▾ listelerinde `title`
  kullanılır [K `view.rs:2205-2236`]. İpucu katalog açıklamasının tamamıdır [K `catalog.rs:964-979`].
- **Bağlamsal sekmeler** seçim değişince kendiliğinden öne gelir; ayar `drafting.contextualTabs` varsayılan
  açık [K `schema.rs:187`, `modes.rs:102-116`]. Sekme satırının sonunda "Duvar 1" gibi görünür [G change-06].
  Beş bağlamsal sekmenin içeriği için bkz. §1.4.
- **Mimari'ye ait ama başka sekmede duran işler** [K ribbon.json; G]:
  - Mobilya/vitrifiye kitaplığı: Ekle › Blok (`block.library` "KentOS blok kitaplığı", `tool.blockInsert`)
    ve Bloklar paneli.
  - Kat görünümü: Görünüm › Görünüş › Kat görünümü ▾ ve durum çubuğundaki Kat hücresi.
    - `active_level.rs:8-9`deki "Mimari › Proje'nin menüsü" sözü artık doğru değil; ribbon.json'da Proje
      panelinde yalnız Evre var.
  - Ölçme: Görünüm › Sorgu ▾ (`archQuery`: Mesafe, Alan, Koordinat sorgula, Açı, Tanımla).
  - Standardı denetle/uygula: Yönet › Standart.
  - Kat planı paftaları: Çıktı › Pafta.
  - İçine tıklayarak alan: Giriş › Çizim ▾.

### 1.2 Araç envanteri

Sütunlar:

- **Akış**: en kısa olağan kullanım (araç + tık/girdi).
- **Sayısal girdi**: komut satırı seçenek harfi, ayar paneli, imleç yanı.
- **Enter / Esc / Ctrl+Z**: aracın `confirm`, `cancel` ve `undo_step` kodu.
- **İkon**: katalogdaki ad ve durumu.

Ortak olanlar:

- Bütün araçlarda imleç yanı değer alanı var (`drafting.cursorInput` varsayılan açık [K `schema.rs:140`]).
  Rakam ya da `@` yazınca açılır, uzunluk ve açı kilitleri vardır [K `preview.rs:1-45`, `input.rs:950-956`].
- Ayarı olan araç başlayınca Öznitelikler dokunda "Etkin aracın ayarları" açılır [K `input.rs:398-401`,
  `architecture_settings.rs:47-133`].
- Sağ tık varsayılan olarak komut menüsünü açar (Onayla/İptal/seçenekler/kenet). Hızlı sağ tık = Enter
  davranışı `drafting.rightClickEnter` ile açılır ve varsayılanı kapalıdır [K `drawing_menus.rs:1-27`,
  `schema.rs:170`].

**Çiz**

| Araç (komut) | Kaynak | Akış | Sayısal girdi | Önizleme / geri bildirim | Enter / Esc / Ctrl+Z | İkon |
|---|---|---|---|---|---|---|
| Duvar (`tool.wall`) | `path.rs` (Shape::Wall), `wall.rs` | Araç, N tık, sonra Enter ya da K (Kapat). Zincirin tamamı Enter'da tek adımda yazılır | Değer alanı: uzunluk, X,Y, @dX,dY, @d<a. Seçenekler: U, E Genişlik, H Hiza (döngü), Ö Öteleme, S Stil adı. Ayar paneli: Stil, Kalınlık, Hiza, Öteleme [K `tool_settings.rs:142-154`] | Gövde %22 dolgulu, yüzler, bileşen sınırları kesikli; geçersizse nedeni yazar [K `wall.rs:282-366`]. Etikette uzunluk ve **"Semt"** [K `path.rs:2447`; EG `arac-duvar-ciz`: "Semt 300.0000 g"]. Mevcut duvarla birleşim önizlenmez [K `wall.rs:285-306`, `:368-371`] | Enter yazar, araç kalır [K `path.rs:2369-2388`]. **Esc taslağı sessizce atar** [K `input.rs:476-489`, `session.rs:962-967`, `:1009-1046`]. Ctrl+Z son noktayı geri alır [K `path.rs:2390-2394`] | `wall` (3 araçla aynı) |
| Dikdörtgen duvar (`tool.wallRectangle`) | `wall_rectangle.rs:299` | 2 köşe tıkı; ya da B, "4,3", yön tıkı | İstemde R Boy referansı, O Ortak duvar, E Kalınlık, S, B Boyutlar, D Döndür. Panelde aynıları "Boylar" ve "Açı (°)" adıyla [K `:355-373`, `:418-445`] | Planlanan duvar, referans halkası, benimsenen duvarlar işaretli [G 05] | Enter taslağı bırakıp çıkar [K `:507-511`]. Esc yalnız soru adımında geri, yoksa çıkar. Ctrl+Z köşe ya da son yapılan | `wall` |
| Ölçülü bölme (`tool.wallPartition`) | `wall_partition.rs:275` | Kaynak yüze tık (+ sınır, boy, yön), önizleme, tık ya da Enter | U, E **Kalınlık**, S, M, Y, B, C, L, A, D [K `:305-320`] | Yazmasız önizleme | Enter yazar; kaynak varsa kalır | `wall` |
| Çizgiden duvar (`tool.wallFromLines`) | `wall_from_lines.rs` | Çizgileri seç, ayarla, Enter | Panel: genişlik, hiza, stil, Çiftler, kaynak koru, referans | Yazmasız önizleme. İpucu metni 862 karakter ve "ADR 0668" geçiyor [S] | Enter tek adım [K `:548`] | `wallFromLines` |
| Mahal (`tool.space`) | `space_tools.rs:260` | Odanın içine bir tık; yazar, numara artar. Ç ile köşeleri çizme kipi | Y yöntem, L kat, N, A ad, K kullanım, V boşluk, Ç çiz. Panelde ad/kullanım listesi yalnız proje ayarında varsa çıkar [K `:300-345`, `:454-503`, `:202-203`] | Oda sınırı ve alan etiketi | Enter: çizim kipinde yazar, yoksa çıkar [K `:593-602`]. Ctrl+Z köşe | `space` (5 araçla aynı) |
| Döşeme (`tool.slab`) | `slab_tools.rs:468` | Odaya tık ya da köşe çizimi | S, T, K Kalınlık, F, Ö, E Kenar, L, W, M; panelde aynıları [K `:531-550`] | Önizleme, kırmızı uyarı | Enter köşe kipinde yazar | `slab` |
| Toplu mahal bul (`tool.spaceDiscover`) | `space_discover.rs:125` | Önizle, Enter (bölge için 2 köşe) | Panel | Yeni mahaller, çakışmalar | Enter tek adım | `space` |
| Mahal ayırıcı (`tool.spaceSeparator`) | `path.rs` (separator_level) | Çoklu çizgi, Enter | Yol aracının seçenekleri | Kesikli önizleme | Esc taslağı atar (Duvar gibi) | `space` |

**Yerleştir**

| Araç | Kaynak | Akış | Sayısal girdi | Önizleme | Enter / Esc / Ctrl+Z | İkon |
|---|---|---|---|---|---|---|
| Kapı, Pencere, Boşluk ▾ (Niş, Baca) | `opening_tools.rs:461` | İmleç duvar yanında gezer, bir tık yazar, araç kalır | B Yöntem, T Tür, E Genişlik, Y Yan, M Menteşe, Ç Açılış, V, Ş, U Uzaklık, F Duvar ucu, R Mesafe referansı, O Orta; niş/baca için D, Y, Z, L, Ü [K `:558-608`]. Panelde "Uçtan uzaklık", "Alt kot" gibi başka adlarla [K `opening_tool_placement.rs:284-345`] | "Kapı · Kapı 90" etiketi ve uç uzaklıkları [K `:907-918`; EG `arac-acik-kapi`]. Uzun adım metni komut satırında kesiliyor ("açıklığın konac…") [EG] | Enter çıkar [K `:904`]. Esc adım adım geri: pervaz, soru, uzaklık, sonra çıkış [K `:889-902`]. Ctrl+Z ilk pervaz | `door`, `window`, `wallOpening`, `wallNiche`, `wallFlue` |
| Kolon | `arch_tools.rs:666` | Bir tık; A ile bütün aks kesişimlerine | B, E, Y, D, L, H, K [K `:689-713`] | Ana hat | Enter çıkar. Ctrl+Z belgeyi geri alır | `column` |
| Aks ▾ (Aks, Aks ağı) | `arch_tools.rs:279`, `grid_system.rs:572` | Aks: 2 tık, S ile seri. Ağ: ayarlar, sonra tık | E, S, C; ağın aralıkları panelde | Önizleme | Enter çıkar; ağda yalnız iki nokta kipinde yazar | `archGrid`, `gridSystem` |
| Merdiven ▾ (Merdiven, Döner merdiven), Rampa | `arch_tools.rs:1660` | 2–4 tık, Enter | Merdivende **12 seçenek**: B, Y, R, S, D, **G Genişlik**, H, K Kesme, N, O, U, Enter. Rampada Y, **E Eğim**, G, H, L, Z, O, U [K `:1746-1787`] | Uyarı (yönetmelik denetimi değil) | Enter yazar | `stair`, `stairCurved`, `ramp` |
| Mutfak dizisi ("Mutfak") | `casework.rs:506` | Baş, son, taraf: 3 tık | M, D, E Evye, O, L, F, U, Y, B | Önizleme | Enter yazar ve kalır | `casework` |

**Düzenle.** 9 görünen ve ▾ içinde 11 araç. Hepsi tıklayarak çalışır; `wall_edit_tools.rs:323`, `opening_tools.rs:1095`,
`space_tools.rs:746/923/1028`, `wall_shift.rs:207`, `space_number.rs:174`, `opening_array.rs:159`,
`viewmark_tools.rs:569`.

| Araç | Yer | Akış | Not | İkon |
|---|---|---|---|---|
| Duvar birleşimi (`tool.wallJoins`) | görünür | Duvar ucuna tık, seçenek: O, Y, G, A, E, T, R | Adı "Duvarları birleştir" ile karışıyor | `wallJoins` |
| Duvarı böl | görünür | 1 tık | | `wallSplit` |
| Açıklığı taşı | görünür | 2 tık | | `openingMove` |
| Mahali böl | görünür | 3 tık | | `spaceSplit` (Mahal'e çok benziyor) |
| **İç nokta seç** (`tool.spaceRebound`) | görünür | 1–2 tık | Adı bağlam olmadan anlaşılmıyor | `space` |
| Mahal kaynakları | görünür | Tıklarla ekle/çıkar, Enter | İleri düzey bir iş görünür yerde | `space` |
| Aksları numarala | görünür | B, sırayla tıklar, Enter | Seyrek kullanılan iş | `gridNumber` |
| Açıklıkları numarala | görünür | Ö, B, Enter | Seyrek kullanılan iş | `openingNumber` |
| Açıklık dağıt | görünür | Kaynak, ayarlar, Enter | | `array` (genel Dizi ile aynı) |
| **Duvarı kaydır** (`tool.wallShift`) | **▾** | Kenara tık, sürükle ya da mesafe yaz | Net mesafe, oda alanları ve izleyenler canlı [K `wall_shift.rs:1-6`]. "Pratik" düzenlemenin ana aracı | `edgeShift` (genel Paralel kaydır ile aynı) |
| Mahal numarala | ▾ | Tıklar ya da önseçim, iki yön noktası, Enter | Denetim akışında kullanılıyor | `tag` (Etiket ile aynı) |
| Duvarları birleştir, Duvarı uzat, Duvarı buda, Duvarı çevir, Stili değiştir | ▾ | Tık ya da önseçim | Uzat/buda AutoCAD alışkanlığıyla sık kullanılır | ayrı ikonlar |
| Açıklığı çevir | ▾ | Tık; Shift+tık menteşe | | `openingFlip` |
| Mahalleri birleştir (`arch.spaceMerge`) | ▾ | Önseçim, hemen | | `spaceMerge` |
| Döşeme deseni ("Döşeme"), Tavan ızgarası ("Tavan") | ▾ | Odalara tık | Kısa adı "Döşeme", slab ile çakışıyor | `spaceFloor`, `spaceCeiling` |

**Belgele**

| Araç | Yer | Akış | Not |
|---|---|---|---|
| Duvar ölçüleri, İç ölçü zinciri | Ölçü ▾ | Duvar tıkı ve yer; ya da iki noktalı kesen çizgi ve yer | İkon `dimension` genel Ölçülendirme ile aynı |
| Etiket | görünür | Nesneye tık, yere tık (ya da Enter) | |
| Metraj (`arch.quantities`) | görünür | Pencere (modal, 720/980 px) | [K `quantities.rs:1013,1213`] |
| Çizelgeler ve Malzeme lejandı | Çizelge ▾ | Tür seç, imleçle yerleştir | Lejant "çizelge" değil; Kapı-pencere ve Mahal kısa adlarıyla bağlamsal sekmede |
| Kesit, Görünüş, Detay, Kuzey oku | İşaret ▾ | Noktalar ve Enter; yer ve yön; merkez, yarıçap ve balon; tık | Kuzey oku bir görünüm işareti değil |
| Revizyon işaretle, Revizyon listesi | Revizyon ▾ | Bulut ve üçgen; tabloyu yerleştir | |
| Hepsini etiketle | ▾ | Türleri seç, Enter | Daire yürüyüşünde kullanılıyor [K `walkthrough_scenes.rs:134`] |
| Etiketleri denetle | ▾ | Modal pencere [K `tags_check.rs:193-198`] | İpucu "alt panelin Uyarılar sekmesi" diyor [K additions `arch.tagsCheck`] |
| Birleşimleri denetle | ▾ | Modal pencere [K `wall_joins.rs:198-199`] | |
| Başvurular | ▾ | Modal pencere [K `viewmark_refs.rs:376-377`] | |
| Oda iç görünüşü | ▾ | Mahale tık, yere tık | |

**Proje**

| Öğe | Yer | Ne açar | Not |
|---|---|---|---|
| Denetim (`arch.audit`) | görünür | Alt panelin Denetim sekmesi; modal değil | Satır nesneye gider, Düzelt [K `arch_audit.rs:1-10`] |
| Program | görünür | Modal, 1000 px [K `program_window.rs:671-672`] | |
| **Katlar** (`arch.settings`) | görünür | Proje ayarları › Mimari (modal) | Duvar çizim ayrıntısı, mahal yöntemi ve önekler de burada; düğmenin adı yalnız "Katlar" [K additions `arch.settings`] |
| Duvar stilleri | görünür | Modal, 860 px [K `wall_styles.rs:652-653`] | |
| Şablon | görünür | Modal, 650 px [K `architecture_template.rs:234-235`] | |
| Evre ▾ | görünür | 4 görünüm süzgeci | Yeni nesnenin evresini belirlemez |
| Pafta seti | ▾ | Modal, 760 px | |
| Gezgin | ▾ | Dok paneli | |
| Kırpma çerçevesi | ▾ | Araç | |
| Kapı ve pencere tipleri | ▾ | Modal, 900 px [K `opening_types_window.rs:849-850`] | Tip iş akışının merkezi ▾ içinde |
| Malzemeler | ▾ | Modal, 920 px | |
| Oda modülleri | ▾ | Modal, 860 px | |
| Modül yerleştir | ▾ | Araç | Bir yerleştirme aracı Proje ▾ içinde |
| Tasarım seçenekleri | ▾ | Modal, 860 px | |

### 1.3 Pencereler

| Pencere | Biçim | İşlemi yazan düğme | Kaydedilmemiş değişikliği sorar mı | Seçime bağlı mı |
|---|---|---|---|---|
| Bina programı | Modal 1000 | Kaydet | **Hayır**: zemine tık ya da × = Vazgeç, kopya atılır [K `program_window.rs:491-497`] | Evet ("Seçilenleri ver") |
| Duvar stilleri | Modal 860 | Kaydet | **Hayır** [K `wall_styles.rs:367-371`] | Hayır |
| Kapı ve pencere tipleri | Modal 900 | Uygula | **Hayır** [K `opening_types_window.rs:710-713`] | Kısmen |
| Malzemeler | Modal 920 | Uygula | **Hayır** [K `materials_window.rs:506-509`] | Hayır |
| Oda modülleri | Modal 860 | Her düğme kendi işini yazar | — | **Evet**: "Seçimden tanımla", "Seçimi örneğe ekle/çıkar" [K `room_modules_window.rs:39-45`] |
| Tasarım seçenekleri | Modal 860 | Her düğme kendi işini yazar | — | **Evet**: "Seçimden tanımla", "Seçimi ekle/çıkar" [K `design_options_window.rs:31-37`] |
| Metraj | Modal 720/980 | Dışa aktar | — | Kat süzgeci |
| Mimari şablonu | Modal 650 | Uygula | — | — |
| Pafta seti, Başvurular | Modal 760 | — | — | — |
| Etiketleri denetle, Birleşimleri denetle | Modal 720 | Göster, Öneriyi uygula | — | "Göster" karartılmış zeminin arkasında yakınlaştırır [K `tags_check.rs:78-97`, `ui/src/widget/dialog.rs:77-87`] |
| Proje ayarları › Mimari ("Katlar") | Modal | Kaydet | DESIGN §7.9 form kuralı | — |
| Gezgin | Dok paneli | — | — | Seçili nesneyi gösterir |
| Mimari denetim | Alt panel sekmesi | Düzelt | — | Satır seçer, kapsam seçimi var |

`Dialog::modal` karartılmış alana basmayı ve ×'i `on_dismiss` olarak gönderir [K `crates/ui/src/widget/dialog.rs:77-87`].
DESIGN §7.9'a göre "kendi kaydı olan düzenleyiciler" yalnız kaydedilmemiş bir değişiklik varken sorar
[D `DESIGN.md:419`]. Mimari'nin Kaydet/Uygula'lı düzenleyicileri bu kurala uymuyor.

### 1.4 Bağlamsal sekmeler [K ribbon.json]

- **Duvar (`ctx.wall`, `:6473`)**: Seçim; Düzenle (Taşı büyük, Kopyala, Döndür, Ölçekle, Aynala, Sil).
  - Duvar paneli: Duvar ölçüleri ve Özellikler büyük; küçükler Duvar birleşimi, Duvarı böl, Duvarları
    birleştir, Uzat, Buda, Çevir, Stili değiştir, Birleşimleri denetle.
  - Açıklık paneli: Kapı, Pencere, Boşluk ▾, Etiket.
  - **Duvarı kaydır yok.**
  - Görüntüde doğrulandı [G change-06].
- **Açıklık (`ctx.opening`, `:6629`)**: Seçim; Düzenle (yalnız Kopyala, Sil).
  - Açıklık paneli: Özellikler, Taşı, Çevir, Numarala, Dağıt, Etiket, Kapı-pencere çizelgesi.
  - Tip paneli: Kapı ve pencere tipleri, Aynı tiptekileri seç, Tipten ayır, Tipe uygula, Tipe döndür.
- **Mahal (`ctx.space`, `:6744`)**: Seçim; Düzenle (Taşı, Sil). Mahal panelinde 11 düğme:
  - Özellikler, Metraj, Mahali böl, İç nokta seç, Mahal kaynakları, Mahal numarala, Mahalleri birleştir.
  - Kısa adı "**Döşeme**" olan düğme döşeme desenidir (`:6823`); "Tavan" tavan ızgarasıdır.
  - Etiket.
  - Kısa adı "**Mahal**" olan düğme mahal çizelgesidir (`:6835`).
  - Aynı panelde `space` ikonu 2 kez, `tag` ikonu 2 kez geçiyor.
- **Döşeme (`ctx.slab`, `:6846`)**: Seçim; Düzenle; Döşeme paneli (Özellikler, Döşeme, Metraj, Mahal).
- **Mimari nesne (`ctx.arch`, `:6936`)**: aks, kolon, merdiven, rampa, etiket ve görünüm işareti seçilince açılır.
  - Seçim ve Düzenle panellerinden sonra Mimari paneli: yalnız Aksları numarala, "**Denetle**" (Etiketleri
    denetle'nin kısa adı) ve Başvurular.
  - **Özellikler düğmesi yok.** Merdiven, kolon ya da rampaya özgü hiçbir araç yok.

---

## 2. Kullanım sorunları (önem sırasıyla)

Önem ölçeği: **Kritik** = emek ya da veri kaybı ya da temel işin yanlış komutla yapılması; **Yüksek** = her
gün yaşanan sürtünme ya da yanlış anlama; **Orta** = belirli akışlarda; **Düşük** = cila.

### S1 — Esc, çizilen çok parçalı duvarı sessizce siliyor · Kritik
- **Kanıt** [K]:
  - Duvar yol aracıdır; `confirm` zinciri ancak Enter'da yazar [K `path.rs:2369-2388`]. Yol aracının
    `cancel`'ı yok (varsayılan `false`).
  - Esc'te uygulama `session.exit()` çağırır: araç düşer ve hiçbir mesaj verilmez [K `input.rs:476-489`,
    `session.rs:962-967`, `:1009-1046`]. Belge açıklaması da bunu söyler: "its draft is dropped: nothing reaches the drawing".
  - Mahal ayırıcı da aynı yolu kullanıyor.
- **Etki**: AutoCAD'de PLINE'dan Esc ile çıkınca o ana kadar çizilen parçalar kalır. Revit'te duvar zincirinde
  her parça tıklandığında yazılır. Bu ikisi genel bilgidir, bu oturumda doğrulanmadı. KentOS'ta 10 köşelik bir
  dış duvar tek Esc ile kaybolur.
- **Çözüm yönü**:
  - Esc'te noktalar ≥ 2 ise taslağı yazmak, ya da "Duvar yazılmadı (6 köşe); Ctrl+Shift+Z ile geri getir"
    benzeri geri getirilebilir bir taslak bırakmak.
  - En azından durum çubuğunda bir bildirim göstermek.
  - Yol aracı ailesinde (Çoklu çizgi dahil) tek bir kararla çözülmeli.

### S2 — "pencere" yazınca Pencere yakınlaştır çalışıyor · Kritik (Mimari için)
- **Kanıt**:
  - `tool.zoomWindow` takma adları: Z, ZOOM, **PENCERE**, PENCEREBUYUT [K `docs/inventory/web.json:17214-17224`].
  - `tool.window`'unki PENCEREKOY, WINDOW [K `catalog/v1/additions.json:2443-2449`].
  - Komut satırı önce tam takma ada bakar [K `typed_names.rs:36-44`]. Bu yüzden "pencere" + Enter,
    yakınlaştırmayı başlatır.
  - Komut ara'da da sıra aynı: önce takma ad eşitliği (0 puan), sonra başlık (2 puan) [K `ribbon_search.rs:51-79`].
    Benzetimde "pencere" için çıkan liste: Pencere yakınlaştır, Görünümü böl, **Pencere** (3. sırada) [S].
- **Çözüm yönü**:
  - Mimari programında (ve genelde) `PENCERE` takma adını pencere aracına vermek, yakınlaştırmaya `PY` ya da
    `PENCEREBUYUT` bırakmak.
  - Programa göre takma ad önceliği tanımlamak (`offers` zaten var).
  - Çakışmayı test eden bir "takma ad sözlüğü" testi eklemek.

### S3 — Modal pencereler düzenlemeyi sormadan atıyor; seçime bağlı işler modal · Kritik / Yüksek
- **Kanıt**: §1.3 tablosu.
  - Bina programı'nın Vazgeç'i ve zemine tık aynı mesajı gönderir; kopya atılır [K `program_window.rs:491-497`, `:672`].
  - Duvar stilleri [K `wall_styles.rs:367-371`, `:653`], Kapı-pencere tipleri [K `:710-713`, `:850`] ve
    Malzemeler [K `:506-509`, `:576`] de aynı.
  - Oda modülleri ve Tasarım seçenekleri "Seçimden tanımla/Seçimi ekle" diyor ama modal oldukları için açıkken
    seçim değiştirilemiyor.
  - Denetim pencerelerinde "Göster" karartılmış zeminin arkasında yakınlaştırıyor.
- **Çözüm yönü**:
  - Bu düzenleyicileri yüzer ya da dok paneline almak; Metraj ve denetimleri alt panele almak (Mimari
    denetim'in yaptığı gibi).
  - Kendi kaydı olan düzenleyicilere DESIGN §7.9'daki "kaydedilmemiş değişiklik" sorusunu eklemek.
  - Seçimle çalışan pencerelerde "Çizimden seç" düğmesi koymak: pencere geçici olarak çekilir, seçim yapılır,
    Enter ile geri gelir.

### S4 — Mimari araçlarda klavye kısayolu yok; takma adlar uzun · Yüksek
- **Kanıt**:
  - 78 Mimari komutunun 0'ında kısayol var; en kısa takma ad 3 harf (AKS, NIS, KAT…) [S].
  - "Mimari" kategorisinde yalnız Üst kat/Alt kat'ın kısayolu var (Ctrl+↑/↓) [K additions].
  - F1 penceresi kısayolu olmayan komutu listelemiyor [K `shortcuts.rs:164`], dolayısıyla F1'de mimari araç yok.
  - DESIGN.md §1.4: "Her araç ve komutun kısayolu vardır ve görünür durumdadır" [D `DESIGN.md:15`].
  - Genel çizimde tek harfler dolu: L, P, C, A, R, T, D, H, S, E, Z, B, G, J, N, V, X, Y, I, O [S]. Boş
    kalanlar F, K, M, Q, U, W.
  - AutoCAD takma ad profili (`catalog/v1/aliases-autocad.json`) mimari komut içermiyor (WALLADD, DOORADD vb. yok).
- **Çözüm yönü**:
  - İki harfli Türkçe takma adlar: DU duvar, DD dikdörtgen duvar, KP kapı, PN pencere, MH mahal, KL kolon,
    AK aks, MR merdiven, ET etiket, DÖ döşeme…
  - Tam takma ad eşitliği önceliği zaten var [K `typed_names.rs:36-44`].
  - En sık beş araca Shift ya da Alt harf kısayolu vermek.
  - F1'i takma adları da gösterecek biçimde genişletmek.
  - AutoCAD Architecture adlarını profile eklemek.

### S5 — Seçenek harfleri tutarsız; yanlış harf aracı değiştiriyor · Yüksek
- **Kanıt** [K]:
  - Aynı kavram farklı harfte:
    - Genişlik: duvar ve kapıda E [K `wall.rs:103`, `opening_tools.rs:563-568`], merdiven ve rampada G
      [K `arch_tools.rs:1753`, `:1781`].
    - Kalınlık: Ölçülü bölmede E [K `wall_partition.rs:306`], döşemede K [K `slab_tools.rs:533`].
  - Aynı harf farklı anlamda:
    - K: Kapat (duvar), Kesme (merdiven), Kalınlık (döşeme), Kullanım (mahal), Etiket (kolon).
    - G: Geri, Genişlik (merdiven), Grup (çizelge; `tag_tools.rs:1086`).
    - Y: Yay, Yan, Yüz, Yükseklik, Yöntem.
  - Çizim alanı odaktayken önce çalışan aracın seçenek harfi denenir, sonra genel kısayol
    [K `input.rs:928-947`].
  - Merdiven'in istemi E içermez [K `arch_tools.rs:1767-1787`]. E genel kısayolda Esnet'tir (`tool.stretch` 'E').
    Bu yüzden merdiven çizerken E'ye basmak Esnet'i başlatır ve merdiven taslağı düşer [T: kod okuması, GUI'de
    denenmedi].
  - Duvarda S'den sonra stil adını çizim odaktayken yazarken ilk harf G ise Geri, D ise Ölçülendirme gibi
    işlenebilir [T].
  - ARCH-UX-01 bu odak belirsizliğini "ayrı regresyonla ele alınmalı" diye açık bırakmıştı [D
    `plan/architecture.md:233-237`].
- **Çözüm yönü**:
  - Mimari için bir seçenek harfi sözleşmesi yazmak: E = genişlik/kalınlık, G = Geri, K = Kapat, Y = Yükseklik,
    S = Stil, T = Tür/Tip, L = Kat, Ö = Öteleme.
  - Araç çalışırken seçenek olmayan tek harf genel aracı başlatmasın: ya komut satırına yazsın ya da "Bu adımda
    E yok; Esnet için Shift+E" desin.
  - Sözcük isteyen adımlarda (`takes_words`) klavyeyi komut satırına vermek.

### S6 — Türkçe terim çakışmaları ve aynı şeyin üç adı · Yüksek
- **Kanıt**:
  - **Duvar kalınlığı** dört yerde farklı adlanıyor:
    - İstemde "Genişlik (E)" [K `wall.rs:103`].
    - Etkin aracın ayarlarında "Kalınlık" [K `tool_settings.rs:144`].
    - Öznitelikler'de "Genişlik" [K `properties/rows.rs:1264`].
    - Hemen altındaki satır "Kalınlıkta sabit" [K `:1271`].
    - Aynı paneldeki Genel › "Kalınlık" çizgi kalınlığıdır [K `:986`; G change-06].
  - Dikdörtgen duvarda istemdeki adlar panelde başka: "Boyutlar (B)" panelde "Boylar", "Döndür (D)" panelde
    "Açı (°)" [K `wall_rectangle.rs:367-373`, `:432-442`; G 05]. Kapıda "Uzaklık (U)" panelde "Uçtan uzaklık".
  - **Döşeme**: `tool.slab` "Döşeme"; `tool.spaceFloor` kısa adı "Döşeme" [K additions:3091]. Bağlamsal Döşeme
    sekmesi ve paneli, ayrıca seçim süzgecinin "Döşeme"si de var.
  - **Mahal**: araç (`tool.space`), bağlamsal sekme ve panel; ama bağlamsal Mahal panelindeki "Mahal" düğmesi
    çizelgedir [K additions:2881, ribbon.json:6835].
  - **Boşluk**: duvar açıklığı aracı ve yüz etiketi (`tool.opening`); mahalin "Boşluk (şaft)" seçeneği
    [K `space_tools.rs:339`]; açıklık numarası B1.
  - **Birleşim / birleştir**: "Duvar birleşimi" uç birleşimidir, "Duvarları birleştir" iki duvarı tek duvar yapar,
    "Birleşimleri denetle" ayrı bir iş.
  - **"İç nokta seç"**: bir mahal aracı ama adı mahal demiyor [K additions:2578].
  - **"Denetle"**: kısa adı hem `arch.tagsCheck` [K additions:2839] hem `standards.check` [:1021] taşıyor.
    Bağlamsal Mimari nesne sekmesinde yalnız "Denetle" yazıyor.
  - **"Katlar"** düğmesi duvarın çizim ayrıntısını ve mahal alan yöntemini de açıyor [K additions:2394].
  - **"Mutfak"** bir mahal adı gibi okunuyor; araç mutfak dolabı dizisi [K additions:2738].
  - **"Kesit"** işareti yalnız bir başvurudur, kesit üretmez; adı bunu söylemiyor.
- **Çözüm yönü**:
  - Tek bir terim sözlüğü kurmak: duvar için "Kalınlık" (çizgi için "Çizgi kalınlığı"); istem, ayar paneli ve
    Öznitelikler aynı adı kullansın. Katalogdan üretilen bir test yazılabilir.
  - Kısa adlar bağlamsız okunduğunda anlam taşısın: "Döşeme deseni", "Mahal çizelgesi", "Mahal sınırını yenile",
    "Uç birleşimi", "Katlar ve ayrıntı", "Mutfak dizisi", "Kesit işareti".

### S7 — Şerit kalabalık, öncelik ters, ana araçlar büyük değil · Yüksek
- **Kanıt**:
  - §1.1 sayıları [S]: 1440×900'de beş panelin bütün düğmeleri küçük [G 03, 04].
  - Katalogda `"size":"lead"` ve panel düzeyinde `"keep": true` var [K ribbon.json:304, 379, 1512;
    `catalog.rs:1111`] ama Mimari sekmesi kullanmıyor. DESIGN §7.3.1 boyutun anlamdan gelmesini ister
    [D `DESIGN.md:283`].
  - Projenin kendi "mimar yürüyüşü" (`walkthrough_scenes.rs`) Hepsini etiketle, Mahal numarala ve İç ölçü
    zinciri gibi ▾ ya da bölünmüş liste içindeki araçları kullanıyor [K `walkthrough_scenes.rs:119-285`].
  - Görünür yerdeki Aksları numarala, Açıklıkları numarala, İç nokta seç ve Mahal kaynakları seyrek kullanılan
    işler.
  - Panel ikonları: Çiz ile Düzenle aynı (`wall`) [K ribbon.json:1935, 2052]; katlanınca iki panel aynı ikonla
    görünür.
- **Çözüm yönü**:
  - Aileleri bölünmüş düğmeye toplamak: Duvar ▾ (Duvar, Dikdörtgen, Ölçülü bölme, Çizgiden); Mahal ▾ (Mahal,
    Toplu bul, Ayırıcı); Numarala ▾ (Aks, Açıklık, Mahal); Denetle ▾.
  - Duvarı kaydır, Uzat ve Buda'yı görünür yapmak.
  - Duvar, Kapı ve Mahal'e `lead`, Çiz ve Yerleştir'e `keep` vermek.
  - Görünen ≤ 30 düğme ve ▾ başına ≤ 6 öğe için bir test eklemek.

### S8 — Duvar çizerken imleç etiketi CAD projesinde "Semt" gösteriyor · Yüksek
- **Kanıt**:
  - Etiket satırı sabit: `format!("Semt {}", format.bearing(bearing_grad(last, end)))` [K `path.rs:2447`].
  - Projeye uygun ad ve dönüşüm için `Format::direction_name()` ("Açı") ve `direction()` (doğudan, saat yönünün
    tersi) zaten var [K `crates/native/domain/src/format.rs:355-388`]. CAD projesinin açı birimi derece
    [K `new_project.rs:165`].
  - Bu yüzden sağa çizilen duvar "Semt 90.0000°" gösterir.
  - Görüntüde: CAD projesinde "5.500 m / Semt 300.0000 g" [EG `arac-duvar-ciz`].
- **Çözüm yönü**: `direction_name()` ve `direction()` kullanmak. Mimari'de etikete duvarın net uzunluğunu ve
  kalınlığını eklemek.

### S9 — Katman: mimari nesneler hep etkin katmana yazılıyor · Yüksek (AutoCAD düzeyi için)
- **Kanıt**:
  - Etkin katmana yazan araçlar: duvar [K `wall.rs:252`], dikdörtgen duvar [K `wall_rectangle.rs:78`], açıklık
    [K `opening_tools.rs:289`], mahal [K `space_tools.rs:174`], döşeme [K `slab_tools.rs:219`], aks ve kolon
    [K `arch_tools.rs:202`, `:625`], etiket [K `tag_tools.rs:123`].
  - Ofis standardı `mimari-ofis` A-DUVAR, A-KAPI, A-PENCERE… tanımlıyor [K `catalog/v1/standards/mimari-ofis.json`]
    ama hiçbir araç bunlara kendiliğinden yazmıyor.
  - Mimari sekmesinde Katmanlar alanı yok; o alan Giriş'te.
  - Bugünkü örnekte 54 nesnenin hepsi "Çizim" katmanında [G 02].
- **Çözüm yönü**:
  - Nesne türüne göre katman anahtarları (AutoCAD Architecture'daki layer key gibi): şablon ya da standart
    "duvar → A-DUVAR" eşlemesini taşısın; araç ayarında "Katman: Otomatik (A-DUVAR) ▾" görünsün.
  - Mimari sekmesine ya da Etkin aracın ayarlarına katman satırı eklemek.

### S10 — Yeni kullanıcı başlangıcı · Yüksek / Orta
- **Kanıt**:
  - Program Giriş'te açılıyor; Mimari 6. sekme [G 02, 03; K `app.rs:1720-1724`].
  - Yeni Mimari projesi sihirbazsız açılıyor [K `project/mod.rs:80-86`]. `arch: None`, yani kat yok
    [K `new_project.rs:78-84`]. Kat yoksa durum çubuğunun Kat hücresi görünmüyor [K `active_level.rs:317-332`].
  - Şablon ayrı bir düğme (Proje › Şablon) ve yeni projeyle çelişen ayarlar taşıyor: cm ve 1:50 [K
    `catalog/v1/templates/mimari.json`]; yeni proje m ve 1:100.
  - Duvar aracının varsayılan stili "Yok" olduğu için plan poşesiz [G 05; G kullanim flat-01/03].
  - Başlangıç ekranında örnek proje yok [K `start.rs`'de "örnek" yok]. Depoda `samples/Konut100m2.kcad` ve
    `konut-3-1.kcad` var.
  - Mahal adları için hazır liste yok; liste yalnız proje ayarından geliyor [K `space_tools.rs:202-203`].
- **Çözüm yönü**:
  - KentOS Mimari'de ilk sekme Mimari olsun.
  - Yeni projede "Zemin kat" kendiliğinden kurulsun.
  - İlk açılışta "Ofis paketi uygulansın mı?" diye tek bir öneri gösterilsin.
  - Varsayılan stil "Dış duvar 25" ya da ilk duvar için seçim kartı olsun.
  - Başlangıç ekranına "Örnek daire" eklensin.
  - Varsayılan Türkçe mahal adı listesi (Salon, Yatak odası, Mutfak, Banyo, WC, Hol, Antre, Balkon…) şablona
    konsun.

### S11 — Bağlamsal sekme ve sağ tık mimariyi tanımıyor · Orta
- **Kanıt**:
  - Duvar sekmesinde Duvarı kaydır yok; Mimari nesne sekmesinde Özellikler yok; merdiven, kolon ve rampa için
    özgü araç yok (§1.4).
  - Seçim varken sağ tık menüsü genel: Taşı, Kopyala, Döndür, Ölçekle, Aynala, Sil, pano, sıra, Benzerlerini
    seç… [K `drawing_menus.rs:322-406`]. Duvarda "Kapı ekle, Böl, Stil" yok; kapıda "Çevir, Tip" yok; mahalde
    "Adlandır, Böl" yok. Özellikler bile menüde değil.
  - Üzerine gelme kartı parsel odaklı: Ada, Mahalle, Nitelik [K `hover_card.rs:1-21`, `:149`].
  - Seçimin yanında mini araç çubuğu yok; yalnız "1/3 ▾" seçim çipi var [K `selection_chip.rs`; G change-06].
- **Çözüm yönü**:
  - Nesne türüne göre sağ tık bölümü: katalogdaki `ctx.*` panelleri sağ tıkta da listelensin.
  - Mimari kart: duvar (stil, kalınlık, uzunluk, kat); mahal (no, ad, net alan); kapı (tip, en, no).
  - Seçimin yanına küçük bir eylem şeridi.

### S12 — Denetimler dağınık; ipucu ile davranış uyuşmuyor · Orta
- **Kanıt**:
  - Mimari denetim alt panelde [K `arch_audit.rs`].
  - Birleşimleri denetle ve Etiketleri denetle modal pencerelerde [K §1.3]. Etiketleri denetle'nin ipucu
    "bulgular alt panelin Uyarılar sekmesinde" diyor [K additions `arch.tagsCheck`].
  - Standardı denetle Yönet sekmesinde. Kapsamları örtüşüyor: Mimari denetim birleşmeyen duvarları ve yinelenen
    numaraları da buluyor [K additions `arch.audit`].
- **Çözüm yönü**: Tek "Mimari denetim" alt paneli; birleşim, etiket ve standart orada süzgeç olarak görünsün.
  Ötekiler o sekmeyi süzülmüş açsın.

### S13 — Arama ve adlandırma keşfi · Orta
- **Kanıt** [S, `search_sim.py`; benzetim program süzgecini uygulamıyor]:
  - "oda" Mahal'i bulmuyor (Oda modülleri, Oda iç görünüşü… çıkıyor).
  - "mobilya", "lavabo", "klozet" ve "tefriş" 0 sonuç veriyor.
  - "kalınlık" yalnız "Çizgi kalınlığını göster"i buluyor.
  - "pencere" için bkz. S2.
  - Arama yalnız takma ad ve başlığa bakıyor, açıklamaya bakmıyor [K `ribbon_search.rs:51-79`].
- **Çözüm yönü**:
  - Eş anlamlı takma adlar: ODA → Mahal; MOBİLYA, LAVABO, KLOZET → kitaplık.
  - Açıklama sözcüklerini düşük puanla aramaya katmak.

### S14 — Duvar aracında harita seçenekleri; seçenekler "Diğer"e düşüyor · Orta
- **Kanıt**:
  - Duvar istemine İzle (İ) ve Akış (A) ekleniyor [K `path.rs:2039-2047`].
  - Programa göre seçenek süzgeci (`ToolOption`) bunları kapsamıyor [K `tool.rs:332-346`].
  - Duvar istemi 11 çip; 1440'ta Stil, Öteleme, Kapat ve Bitir "Diğer"e düşüyor [EG `arac-duvar-ciz`].
  - Merdiven 12, kapı 11+ seçenek taşıyor [K].
  - Bugünkü Dikdörtgen duvar görüntüsünde yalnız iki çip görünüyor, gerisi "Diğer"de [G 05].
- **Çözüm yönü**: Mimari programında İzle ve Akış'ı kapatmak; çipleri önem sırasına göre sıralamak. Ayar paneli
  ana yüzey olsun; komut satırında yalnız 3–4 ana seçenek kalsın.

### S15 — Geri bildirim boşlukları · Orta
- Duvar çizerken mevcut duvarla birleşim önizlenmiyor; tek duvarın gövdesi çiziliyor [K `wall.rs:285-306`,
  `:368-371`].
- Geçici ölçü sayıları mahal etiketlerinin üstüne biniyor ("4.800" "38.40 m²"nin üstünde) [G change-06, flat-07].
- Komut geçmişi iç alan adlarını Türkçe-İngilizce karışık döküyor: "adopt: evet · arch: level zemin · frame:
  corners · Başvuru: inner · width: 0.25"; "İşlem: distance (uzaklık 4.8, edge 0, yan right)" [G kullanim flat-01,
  change-06].
- Uzun adım metni komut satırında kesiliyor ("açıklığın konac…") [EG `arac-acik-kapi`]. Güncel derlemede
  denenmedi [T].
- İpuçları açıklamanın tamamını gösteriyor [K `catalog.rs:964-979`]: ortalama 274, en uzun 862 karakter; 78
  ipucundan 10'u 400 karakteri aşıyor. Beşinde "docs/adr/0721" ya da "ADR 0668" geçiyor [S].
- **Çözüm yönü**: Birleşim önizlemesini mevcut çözücüyle göstermek (`add_joined` zaten var). Geçici ölçü yazısını
  etiketlerden kaçırmak. Komut geçmişini Türkçe özet + "ayrıntı ▸" biçimine çevirmek. İpucunu iki cümleye indirmek
  ve kalan metni F1/yardıma taşımak.

### S16 — Evre ve kat yönetimi dağınık · Orta
- Yeni nesnenin evresi yalnız Modül yerleştir'de seçilebiliyor [K `module_place.rs:39`, `:461`]. Duvar, açıklık
  ve mahal araçları yalnız katı yazıyor [K `tool.rs:1451-1456`]. ADV-07 "araçlarda Evre"yi açık sayıyor [D
  `plan/architecture-advanced.md:326`]. Tadilat örneğinde evre çizgi dili ayırt edilmiyor (M0 gözlemi).
- Kat işleri beş ayrı yerde:
  - Katlar düğmesi (modal).
  - Durum çubuğu Kat hücresi: iyi bir menü [K `active_level.rs:275-313`].
  - Görünüm › Kat görünümü.
  - Çıktı › Kat planı paftaları.
  - Gezgin.
- **Çözüm yönü**: Etkin aracın ayarlarına "Evre" satırı ve durum çubuğuna "Yeni nesne evresi" eklemek; kat ve
  evreyi Mimari sekmesinde tek bir "Kat/Evre" alanında toplamak.

### S17 — Belge ve yardım davranışla uyuşmuyor · Düşük / Orta
- F1'in fare tablosu "Sağ tık: Komutu onayla ya da bitir (Enter gibi)" diyor [K `shortcuts.rs:31`]. Varsayılan
  davranış ise menüyü açmak [K `schema.rs:170`].
- DESIGN §9 ("Enter, sağ tık: … bitir"), §7.4.1 ("basılı sağ tık menüsü henüz yok") ve §7.3.1 ("bağlamsal sekme
  kendiliğinden açılmaz") koddan geride [D `DESIGN.md:690`, `:323`, `:278`].
- **Çözüm yönü**: F1 metnini ayardan üretmek; DESIGN.md'yi güncellemek.

### S18 — Enter ve bitirme sözcükleri araçtan araca değişiyor · Düşük
- Enter: Duvar, Mutfak ve Etiket yazıp kalıyor; Kapı, Kolon ve Dikdörtgen duvar çıkıyor; Duvar ölçüleri yazıp
  çıkıyor [K §1.2].
- Çiplerde Enter'ın adı "Bitir", "Uygula", "Yaz", "Numarala"; geri adımın adı "Geri (G)" ya da "Geri al (G)".
- **Çözüm yönü**: Kural tek olsun: tıkla yazan araçta Enter çıkar, Enter'la yazan araçta Enter yazar ve kalır.
  Çip sözleri tek olsun.

### S19 — Sekme satırı taşması (iş seçilmemiş çizim) · Düşük
- 1100×650'de 14 sekme sığmıyor, "Çık…" kesiliyor [G kullanim `…-1100x650.png`]. M0 görüntüsünde etkin bağlamsal
  "Duvar" sekmesi ekran dışında kalıyor.
- Mimari işinde 9 sekme var; sorun kentos-cad'in iş seçilmemiş hâlinde. DESIGN "1100 px'te her sekme sığar" diyor.

---

## 3. Çekirdek CAD yardımcılarının Mimari'deki durumu

| Yardımcı | Durum | Kanıt | Mimari açısından eksik |
|---|---|---|---|
| Nesne kenetleme | 13 tür: uç, orta, kesişim, merkez, dik, teğet, düğüm, ekleme, en yakın, ağırlık merkezi, uzantı, paralel, karelaj; ayrıca çizilmekte olan nesne ve tek seferlik kenet (Shift+sağ tık) | [K `snap_menu.rs:20-35`, `drawing_menus.rs:39-56`] | Mimariye özgü tür ya da süzgeç yok |
| Duvara kenet | Duvar yolunun köşeleri uç nokta sayılır [K `store/snap.rs:479-484`]. Genişletilmiş parçalar yani yüz çizgileri genel türlerle kenetlenir [K `store/mod.rs:128-140`] | | Duvar ekseni ya da yolu boyunca en yakın/orta nokta yok [T]; "yalnız yüz / yalnız eksen / çekirdek" seçimi yok |
| Açıklığa kenet | Açıklığın dört pervaz noktası uç nokta sayılır; kesilen duvar yüzünün ortak köşesi açıklığa ait sayılır [K `store/snap.rs:411-438`, `entity.rs:921`] | | Açıklık merkezi ve kanat ucu ayrı kenet değil |
| Orto / Dik açı / Kutupsal / İzleme | F8, Dik açı, F10, Shift+F3; durum çubuğunda lamba [K `view.rs:2273-2287`] | Oturum düzeyinde, bütün araçlarda | Duvar yönüne göre kutupsal (duvara paralel ya da dik) yok [T] |
| Dinamik girdi | İmleç yanı değer alanı; uzunluk ve açı kilitleri, Tab ile geçiş [K `preview.rs:28-45`; D `DESIGN.md` §7.4.2] | Varsayılan açık [K `schema.rs:140`] | Etiket "Semt" (S8). Alan yalnız yazınca açılıyor; görüntülerde görünmedi (M0) |
| Doğrudan mesafe | Değer alanına yazılan sayı imleç yönünde uzunluk; U ile son doğrultuda uzunluk [K `path.rs:2010-2013`] | | Duvarın iç/dış yüzüne göre "net" uzunluk girme yok (Hiza ile dolaylı) [T] |
| Tutamaçlar | Yol köşeleri; tutamaç menüsü (Esnet, Taşı, Döndür, Ölçekle, Aynala); sürüklerken izleyenler (ölçü sıraları, açıklıklar, mahaller) önizlenir [K `select.rs:880-925`] | | Duvar yüzünü ya da odayı sürükleyen tutamaç yok; o iş Duvarı kaydır aracında [T] |
| Geçici ölçü | Seçili duvar, açıklık ve kolonda en yakın referansa net mesafe; sayıya basıp yaz [D plan ARCH-EDIT-01; G change-06] | | Yazılar etiketlere biniyor (S15) |
| Öznitelikler | Üstte "Etkin aracın ayarları", altta "Seçili nesnenin özellikleri" [K `architecture_settings.rs:122-133`; G 05] | | Duvar seçiliyken önce Genel (renk, çizgi), sonra Geometri; duvarın asıl alanları kaydırınca görünüyor (M0) |
| Katman seçimi | Etkin katman Giriş › Katmanlar ya da dok panelinde | | Mimari sekmesinde yok; otomatik katman yok (S9) |
| Kat seçimi | Durum çubuğu Kat hücresi, Ctrl+↑/↓, araç ayarının ilk satırı "Kat", Gezgin [K `active_level.rs`] | | Yeni projede kat yok, hücre görünmüyor (S10) |

---

## 4. Eksikler

### 4.1 Pratik (hızlı) kullanım

1. **Oda önce çizim**:
   - Var olanlar: dikdörtgen oda için Dikdörtgen duvar (İç net ve Ortak duvar: Benimse); serbest biçim için
     önce kontur çizip sonra Çizgiden duvar (Kapalı kontur).
   - Eksik: tek adımda duvar + mahal + etiket üreten bir **Oda** aracı; ADR 0656 bunu dışarıda bırakmıştı
     [D `docs/adr/0656-…md:89`]. Mahal için ikinci bir araç gerekiyor.
2. **Odayı sürükleyerek büyütme ve canlı alan**:
   - Duvarı kaydır canlı etkileri gösteriyor (net mesafe, oda alanları, izleyen duvarlar) [K `wall_shift.rs:1-6`]
     ama Düzenle ▾ içinde, bağlamsal Duvar sekmesinde değil.
   - Mahalin kendisinde sürüklenebilir kenar tutamacı yok [T].
3. **Donatı kataloğu**:
   - Kitaplık Ekle sekmesinde ve Bloklar panelinde: çift tık ya da Enter yerleştirir, sürükle-bırak yok
     [K `blocks_panel.rs:1-12`, `:263-273`].
   - Mimari sekmesinde yalnız Mutfak dizisi var. Aramada mobilya kelimeleri bulunmuyor (S13).
4. **Akıllı varsayılanlar**:
   - İyi olanlar: açıklık numarası (K1, P1), mahal numarası artışı, merdiven yüksekliğinin kattan gelmesi,
     benimsenen ortak duvar.
   - Eksik olanlar: stil "Yok", katman etkin katman, kat yok, mahal adı listesi yok (S9, S10).
5. **Hızlı stil seçimi**:
   - Stil ayar panelinde yalnız ad listesi; önizleme Duvar stilleri penceresinde (modal).
   - Öneri: şerit ya da ayar panelinde kesit küçük resimli stil galerisi; yerleşik üç stil var (Dış duvar 25,
     İç duvar 15, Gazbeton 20) [K `contracts/src/arch.rs:1611-1643`].
6. **Bağlam mini araç çubuğu**: yok (S11).
7. **Tadilat**: yeni nesnenin evresi seçilemiyor (S16).
8. **Tekrar**: Enter son aracı yineliyor ve boşta sağ tık menüsünde "Yinele: …" var [K `drawing_menus.rs:339-351`].
   Bu iyi.

### 4.2 "AutoCAD kadar ayrıntılı" çizim için arayüz eksikleri

1. Türüne göre katman anahtarları ve standart katmanlara otomatik yazma (S9).
2. Mimari kenet türleri ve süzgeci: duvar yüzü, ekseni, çekirdeği, açıklık merkezi, kanat ucu (§3).
3. Kısayol ve kısa komut adları; AutoCAD Architecture takma ad profili (S4).
4. Görünüm/ayrıntı seçiminin hızlı anahtarı. Duvar çizim ayrıntısı (ölçeğe göre, şematik, standart, ayrıntılı)
   "Katlar" düğmesinin arkasında, Proje ayarları modalinde [K additions `arch.settings`]. Durum çubuğunda hücre
   olarak yok [K `view.rs`'de bulunamadı].
5. Duvar seçiliyken mimari alanlar Öznitelikler'de aşağıda kalıyor; çoklu seçimde ortak alan düzenleme
   doğrulanmadı [T].
6. Seçenek harf sözleşmesi ve tek terim sözlüğü (S5, S6). Ayrıntılı çizimde komut satırıyla hızlı çalışan
   kullanıcı en çok bundan etkilenir.
7. Kesit ve görünüş işaretleri yalnız başvuru; adları bunu söylemiyor. Gerçek kesit üretimi bu raporun
   kapsamı dışında.
8. Komut geçmişinin okunur özeti ve geri alma adımlarının adları (S15).

---

## 5. İkonlar

- **Kaynak**: SVG yolları web envanterindeki (`docs/inventory/web.json` › `icons`, 853 ad) ve eklerdeki
  (`catalog/v1/additions.json` › `icons`, 419 ad) kayıtlardan gelir. 20×20 kutu ve `currentColor` ile kentos_ui
  çizer. Envanterde olmayan ad en yakın yerleşik ikona düşer [K `icons.rs:1-30`].
- **Bulunmayan ikon yok**: Mimari'deki 82 komutun ikon adının hepsi katalogda var. Birebir aynı SVG'ye sahip iki
  farklı ad da yok [S].
- **Aynı ikonu paylaşan komutlar** [S; `arch-icons.png`'de görsel olarak doğrulandı]:
  - `wall`: Duvar, Dikdörtgen duvar, Ölçülü bölme. Çiz panelinde yan yana.
  - `space`: Mahal, Toplu mahal bul, Mahal ayırıcı, İç nokta seç, Mahal kaynakları. Beşi aynı.
  - `tag`: Etiket ve Mahal numarala. Bağlamsal Mahal panelinde ikisi bir arada; aynı panelde `space` de iki kez.
    DESIGN §6'nın "aynı simge iki kez yok" kuralının ruhuna aykırı [D `DESIGN.md:234`].
- **Genel CAD ikonunu ödünç alanlar** [S]:
  - Duvar ölçüleri ile Ölçülendirme (`dimension`).
  - Açıklık dağıt ile Dizi (`array`).
  - Duvarı kaydır ile Paralel kaydır (`edgeShift`).
  - Özellikler ile Katman ve öznitelik paneli (`panelRight`).
- **Zayıf ya da birbirine benzeyen aileler** (`arch-icons.png`):
  - Metraj, Kapı-pencere, Mahal, Kaplama, Donatı, Bölüm çizelgesi ve Revizyon listesi: hepsi tablo, yalnız üstteki
    küçük işaret farklı. 16 px'te ayırt edilmesi zor.
  - Mahal ile Mahali böl çok benziyor.
  - Duvarları birleştir, Duvarı böl ve Duvarı uzat: üçü de yatay çizgi ve ok.
  - Duvar birleşimi ile Birleşimleri denetle.
  - Tipten ayır, Tipe uygula ve Tipe döndür: aynı kapı kasası, küçük oklar.
  - Döşeme deseni, Tavan ızgarası, Evre: Tümü: ızgara benzeri.
- **Panel ikonları**: Çiz ve Düzenle aynı (`wall`).
- **İnceleme eksiği**: Sahibin ikon turunda (`icon_tour.rs:61-102`, 37 sahne) Mimari sekmesinden, onun ▾
  listelerinden ve bölünmüş düğmelerinden hiçbiri yok.
- **Öneri**: Her yöntem kendi ikonunu taşısın.
  - Duvar: serbest yol, dikdörtgen, ölçü oklu bölme.
  - Mahal: tık noktalı oda, çoklu oda, kesikli ayırıcı, artı imli iç nokta, kaynak oklu oda.
  - Mahal numarala: "01" yazılı oda.
  - Ödünç ikonları mimari eşdeğerleriyle değiştir: duvara dayalı ölçü, duvar üstünde üç açıklık, duvar kenarı oku.
  - Çizelge ailesini ayırt et: kapı, oda, kaplama dokusu, mobilya, daire, malzeme taraması.
  - İkon turuna Mimari sahnelerini ekle.

---

## 6. Resimle görülen ve hiç görülmemiş senaryolar

- **Kaynak**: `apps/desktop/src/*_scenes.rs`.
  - Mimari'yle ilgili yaklaşık 39 dosyada 400'ü aşkın sahne var, örneğin `wall_scenes.rs` 54, `walkthrough_scenes.rs`
    34, `program_scenes.rs` 18 [K].
  - Diskteki mimari araç görüntülerinin çoğu 19 Ekim tarihli [EG]. Güncel olanlar `kullanim/*` ve
    `mimari-durum/*` [G].
  - Bunların bir kısmı (en az `kullanim/*`) tam kentos-cad arayüzünü (14 sekme) gösteriyor, KentOS Mimari'yi değil.
    Araçlar şeritten değil kimlikle başlatılıyor (M0).
- **Arayüzü (şerit ya da araç) resimde hiç kullanılmamış komutlar** (`"tool.x"` dizgesi hiçbir sahnede yok [S]):
  - Mahal ayırıcı, İç nokta seç, Mahal kaynakları.
  - Açıklıkları numarala, Açıklık dağıt, Açıklığı çevir.
  - Duvarları birleştir, Duvarı uzat, Duvarı buda, Duvarı çevir, Stili değiştir.
  - Mahal kaplama ve Donatı çizelgesi araçları. Çizelgeler komutla üretiliyor.
  - Görünüş işareti, Evre: Mevcut durum.
  - Tipten ayır, Tipe uygula, Tipe döndür.
  - Özellikler düğmesinin kendisi.
- **Şeridin hiç resimlenmemiş açılır yüzeyleri** [K `open_split` ve `open_folded` çağrıları]:
  - Düzenle ▾ (11), Belgele ▾ (5), Proje ▾ (8).
  - Boşluk ▾ ve Aks ▾ bölünmüş listeleri.
  - Bugün resimlenmiş olanlar: Merdiven ▾ [K `arch_scenes.rs:458-470`] ve İşaret/Revizyon/Çizelge ▾
    [K `symbols_scenes.rs:193-195`].
- **Hiç resimlenmemiş akışlar** [K, T]:
  - Esc ile duvar zincirinin kaybı.
  - Pencere aracını adıyla başlatma ("pencere").
  - Modal pencerenin zemine tıkla kapanması.
  - Seçime bağlı pencerede seçimi değiştirme.
  - Çalışan araçta yanlış harf.
  - 1100 px'te KentOS Mimari ana şeridi (güncel derleme).
  - Bağlamsal Mahal ve Mimari nesne sekmeleri (güncel derleme).
- **Araç notu (M0)**: `snapshot --sekme arch` "böyle bir şerit sekmesi yok" diyor [M0: `snapshot.rs:72-76`,
  `catalog.rs:1239`]. Mimari sekmesinin hızlı kanıtı bu yüzden alınamıyor.

---

## 7. Plana önerilen yeni maddeler (ADV-UX)

ADV-07 genel kabul çalışmasını tarif ediyor. Aşağıdakiler onu besleyen, ölçülebilir ve dar dilimler. Kabul
satırlarındaki sayılar öneri; önce mevcut durum ölçülmeli.

- **ADV-UX-01 · Taslağı koruyan Esc** (S1)
  - Duvar ve yol araçlarında Esc taslağı yazar ya da tek adımda geri getirilebilir bırakır; bildirim gösterir.
  - Kabul: 6 köşeli zincir, Esc, sonra Ctrl+Z ya da "Geri getir" ile aynı çizim; Python/MCP eşitliği.
- **ADV-UX-02 · Takma ad ve kısayol sözlüğü** (S2, S4, S13)
  - PENCERE pencere aracına; iki harfli Türkçe takma adlar; en sık 5 araca kısayol; F1 takma adları gösterir;
    eş anlamlılar (ODA, MOBİLYA…); AutoCAD Architecture adları.
  - Kabul: takma ad çakışma testi; "pencere", "oda" ve "lavabo" ilk sırada doğru komutu buluyor.
- **ADV-UX-03 · Terim sözlüğü ve seçenek harf sözleşmesi** (S5, S6)
  - İstem, ayar paneli ve Öznitelikler aynı adı kullanır. Harfler: E, G, K, Y, S, T, L, Ö.
  - Seçenek olmayan harf çalışan aracı bırakmaz.
  - Kabul: katalogdan üretilen ad ve harf tutarlılık testi; merdivende E ile araç kalır.
- **ADV-UX-04 · Mimari şeridin yeniden düzeni** (S7)
  - Aileler bölünmüş düğmede; sık işler görünür; `lead` ve `keep`; panel ikonları ayrı.
  - Kabul: görünen ≤ 30 düğme, ▾ başına ≤ 6 öğe; mimar yürüyüşünün bütün araçları ya görünür ya da tek
    bölünmüş listede; 1440 ve 1100 px, 13 ve 18 px görüntü.
- **ADV-UX-05 · Modalsız mimari düzenleyiciler** (S3, S12)
  - Program, stiller, tipler, malzemeler, modüller, seçenekler ve metraj yüzer ya da dok paneline geçer.
  - Denetimler tek alt panelde birleşir. Kaydedilmemiş değişiklik sorusu gelir. "Çizimden seç" düğmesi eklenir.
  - Kabul: pencere açıkken seçim değiştirip modüle eklemek; zemine tık veri kaybetmiyor.
- **ADV-UX-06 · Mimari başlangıç** (S10)
  - Mimari sekmesiyle açılış; Zemin kat; şablon önerisi; varsayılan stil; mahal adı listesi; örnek proje.
  - Kabul: boş projeden ilk duvarın stil poşesiyle ve katla yazılması ≤ 3 eylem.
- **ADV-UX-07 · Türe göre katman ve şeritte katman/kat/evre alanı** (S9, S16)
  - Şablon katman eşlemesini taşır; araç ayarında "Katman: Otomatik (A-DUVAR)"; yeni nesnenin evresi.
  - Kabul: ofis paketli projede duvar, kapı ve mahal kendi katmanında; DXF'te aynı katmanlar.
- **ADV-UX-08 · Duvar aracının geri bildirimi** (S8, S14, S15)
  - "Açı" ve CAD yönü; çizerken birleşim önizlemesi; İzle ve Akış Mimari'de kapalı; çip önceliği.
  - Kabul: sağa duvar "Açı 0°"; T birleşim önizlemesi, yazılanla aynı.
- **ADV-UX-09 · Oda aracı ve odayı sürükleme** (§4.1)
  - Tek adımda duvar + mahal + etiket (dikdörtgen ve çokgen); mahal kenarı tutamacı Duvarı kaydır'ın planını
    kullanır; Duvarı kaydır bağlamsal Duvar sekmesinde.
  - Kabul: ADV-01'in "3×3 oda ekle" örneği tek araçla.
- **ADV-UX-10 · Donatı galerisi Mimari'de** (§4.1)
  - Yerleştir'de "Donatı ▾" (ıslak hacim, mobilya, mutfak); kategori ve arama; sürükle-bırak; duvara hizala.
  - Kabul: "lavabo" aramasından duvara hizalı yerleşime ≤ 3 eylem.
- **ADV-UX-11 · Mimari bağlam: sağ tık, kart ve mini şerit** (S11)
  - Kabul: duvar, kapı ve mahalde sağ tıkın ilk bölümü bağlamsal sekmenin araçları; kartta mimari alanlar.
- **ADV-UX-12 · İkon seti ve ikon turu** (§5)
  - Kabul: Mimari sekmesinde aynı ikonu taşıyan iki komut yok; ikon turunda Mimari'nin bütün listeleri.
- **ADV-UX-13 · Mimari kenet süzgeci** (§3)
  - Duvar yüzü, ekseni ve çekirdeği; açıklık merkezi.
  - Kabul: kenet menüsünde ayrı türler; ortak durumlar.
- **ADV-UX-14 · Yardım ve ipucu metinleri** (S15, S17)
  - İpucu ≤ 2 cümle ve ADR numarası yok; F1 fare tablosu ayardan; DESIGN.md güncel; komut geçmişi Türkçe özetli.
- **Süreç önerisi**: Kılavuz §9.2'nin "araç bitmiş sayılmaz" listesine arayüz kapıları eklensin:
  - Kendi ikonu.
  - Kısa takma ad ya da kısayol.
  - Şeritteki yerin gerekçesi ve sayım testi.
  - Terim sözlüğüne uyum.
  - Seçenek harf sözleşmesine uyum.
  - İkon turu ve gerçek pencere görüntüsü.

  Bugünkü liste yalnız komut, Python, MCP ve manifest istiyor [D `docs/kilavuz/09-yeni-is-tarifleri.md:13-40`].

---

## 8. Sınırlar ve doğrulanmayanlar

- Uygulama bu oturumda çalıştırılmadı. [T] ile işaretli davranışlar kod okumasından çıkarıldı:
  - Merdivende E'nin Esnet'i başlatması.
  - Stil adı yazarken harflerin başka işe gitmesi.
  - Mahal kenarı tutamacının olmaması.
  - Komut satırında adım metninin güncel derlemede kesilmesi.
- [EG] görüntüleri 19 Ekim tarihli; arayüz o günden beri değişti ("Semt" satırı kodda hâlâ aynı).
- Arama benzetimi (`search_sim.py`) programın `offers` süzgecini uygulamıyor; Mimari'de bazı sonuçlar (yol ve
  harita komutları) düşer. "pencere" sonucu bundan etkilenmez: Pencere yakınlaştır Mimari'de de var.
- Üçüncü taraf davranışları (AutoCAD'de PLINE'dan Esc, Revit duvar zinciri, AutoCAD Architecture layer key) genel
  bilgidir; bu oturumda resmî belgeden doğrulanmadı. Plana girmeden önce kaynak aranmalı (CLAUDE.md "Araştırma önce").
- Adım ve tık sayıları kod ve açıklamalardan çıkarıldı; zamanlı ölçüm değil. Gerçek sayılar için ADV-07'nin acemi
  ve deneyimli akış ölçümü gerekiyor. M0'nun iz sayıları şeridi kullanmıyor.
