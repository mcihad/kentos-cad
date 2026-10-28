# ADR 0724: Merdiven katın yüksekliğine bağlı, yönetmelik profiliyle denetim, üst katta gösterim, merkez direği ve metraj

- **Durum:** kabul edildi (28 Ekim 2026). plan/architecture.md ARCH-STAIR-01'in kalan alt maddelerini ve kabulünü kapatır.
- **Bağlam:** ADR 0343 (merdiven), ADR 0704 (yükseklikten merdiven), ADR 0705 (korkuluk), ADR 0709 (döner merdiven),
  ADR 0713/0715/0716/0720 (etkin kat, kat görünümü, paftada kat), ADR 0721 (kata bağlı kot ve izleyici), ADR 0723
  (rampa: kata bağlı başlangıç, erişilebilirlik profili), ADR 0703 (Mimari denetim), ADR 0718 (kat planı paftaları),
  ADR 0265 (yeni şemanın kuralı), ADR 0622 §9 (araç = komut).
- **Web:** ertelendi (ADR 0228). TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0704 merdiveni yükseklikten tasarladı; ADR 0709 döner merdiveni ekledi. Beş iş açık kalmıştı:

- **Kat yüksekliği izlenmiyordu.** Araç etkin katın yüksekliğini bir kez okuyordu. Kat yükselince rıhtlar eski
  kalıyor, yalnız `stair_rise_level` uyarısı çıkıyordu.
- **Yönetmelikle denetim yoktu.** Blondel oranı yalnız bilgiydi; kurum ya da yönetmelik profili yoktu.
- **Üst katta görünmüyordu.** Üst katın planında alttaki merdiven yoktu. Yalnız etkin kat onu gizliyordu, Etkin kat ve
  alt kat ise bütün merdiveni soluk çiziyordu.
- **Merkez direği yoktu.** Döner merdivende iç basamak yalnız bilgiydi.
- **Metraj ve pafta kabulü yoktu.** Metraj merdivenleri saymıyordu.

### Araştırma

Kaynaklar yerelde doğrulanmadı.

- **Revit:** merdiven örneğinin özellikleri ([Stair Component Instance Properties](https://help.autodesk.com/cloudhelp/2020/ENU/Revit-Model/files/GUID-541EB756-8376-4522-BFA1-141C22346ED8.htm)).
  - Base Level ve Top Level merdivenin alt ve üst katlarıdır.
  - Rıht sayısı iki kat arasındaki yükseklikten hesaplanır.
  - Gerçek rıht yüksekliği salt okunurdur ve türün en büyük rıht yüksekliğinden büyük olamaz.
  - Topluluk notlarına göre kat aralığı değişince var olan kollar yeniden çizilmez. Rıht yüksekliği aşılırsa Revit
    reddeder ([forum](https://forums.autodesk.com/t5/revit-architecture-forum/how-can-i-manually-change-riser-heights-when-creating-stairs/td-p/2683156)).
- **Archicad:** merdivenin Floor Plan Display ayarı “Show on Stories: Home and One Story Up” der.
  - Kendi katında kırık çizgi “Visible – Hidden”, üst katta “Hidden – Visible” seçilir. Böylece üst kat merdivenin
    kesmeden sonrasını gösterir ([Graphisoft topluluğu](https://community.graphisoft.com/t5/Modeling/ArchiCAD-21-Stair-tool-break-lines/m-p/265341),
    [break mark](https://community.graphisoft.com/t5/Modeling/Issue-Stair-Break-Mark/m-p/334241)).
  - Resmî yardım sayfası bulunamadı; seçeneklerin adları Archicad API belgesinde de geçer.
- **Planlı Alanlar İmar Yönetmeliği md. 31** ([01.07.2026 derlemesi](https://www.alomaliye.com/wp-content/uploads/2017/07/planli-alanlar-01-07-2026.pdf)):
  - Ortak merdivenin kol ve sahanlık genişliği konutta 1.20 m, diğer yapılarda 1.50 m, bağımsız bölüm içinde 1.00 m'den
    az olamaz. Konutta yangın yönetmeliğinin kaçış genişliği de aranır.
  - Basamak yüksekliği asansörsüz binada 0.16 m'den, asansörlü binada 0.18 m'den fazla olamaz.
  - 2a+b 60–64 cm olur; basamak genişliği 0.27 m'den az olamaz.
  - Balansmanlı (dengelenmiş) merdivende basamak en dar kenarda 0.10 m'den, ortasında 0.27 m'den az olamaz.
  - Merdivenin iki yanında korkuluk ve küpeşte zorunludur.
  - md. 4'ün tanımlarına göre ara ve kat sahanlığı en az merdiven kolu genişliği kadardır.
  - Son kattaki çatı arası piyeslerine çıkan iç merdivenlerde (3. fıkra) bu şartlar aranmaz.

## Karar

### 1. Kata bağlı yükseklik (`heightFromLevel`)

- **Oluşturma.** `cad.stairs.create`'in `heightFromLevel: true`'su merdiveni katının yüksekliğiyle, `height` verilmiş
  gibi tasarlar. Rıht sayısı verilmezse en fazla rıhtla hesaplanır; rıht yüksekliği yükseklik ÷ rıht sayısıdır.
- **İzleme.** Belge, katın yüksekliği ya da merdivenin rıht sayısı değişince rıht yüksekliğini aynı adımda yeniden
  yazar. Bunu ADR 0721'in kat izleyicisi (`level_marks.rs`) yapar.
  - Ayar değişikliği, geri alma ve yinelemeden sonra da yazar; kendisi bir geri alma adımı değildir.
  - Rıht sayısı korunur (Revit'in kollarını yeniden çizmemesi gibi). Rıht profilin sınırını aşarsa bunu Denetim söyler.
- **Retler.** Aşağıdakiler `invalid_stair`, yeri `heightFromLevel`:
  - `height` ile birlikte verilmesi;
  - katı olmayan merdiven;
  - yüksekliği olmayan kat.

  Projede olmayan kat `level_not_found`'dur.
- **Değiştirme.** `cad.arch.edit`'te `heightFromLevel: true` bağlar; katın yüksekliği olmalıdır.
  - `false` ya da kendi `riserHeight`'ı bağı kaldırır; rıhtlar yerinde kalır.
  - `riserHeight` ile `heightFromLevel: true` birlikte verilmez.
  - Bağlı merdivenin katı kaldırılamaz.
- **KCAD.** Opt-in şema 125: yalnız bağlı merdiveni ya da merkez direği olan dosya yeni şemayı yazar. `false` yazılmaz.
  Katı olmayan bağlı merdiven ve düz merdivende direk okunurken reddedilir.
- **Araç ve Öznitelikler.** Yükseklik Etkin kat'ı izlerken (“3.060 m · katın”) araç komuta `heightFromLevel: true`
  gönderir; yazılan yükseklik merdivenin kendisinindir. Öznitelikler'de “Yükseklik: Kat yüksekliği / Kendi” var.

### 2. Yönetmelik profili ve Mimari denetim

- **Profiller.** `catalog/v1/arch-profiles/` altında, biçim `kentos.arch-profile` 1. Rampa profili ADR 0723'ten buraya
  taşındı. Dört merdiven profili var:

  | Profil | En yüksek rıht | En dar kol |
  |---|---|---|
  | `imar-merdiven-konut/1` | 0.18 m | 1.20 m |
  | `imar-merdiven-konut-asansorsuz/1` | 0.16 m | 1.20 m |
  | `imar-merdiven-diger/1` | 0.18 m | 1.50 m |
  | `imar-merdiven-daire-ici/1` | 0.18 m | 1.00 m |

  Hepsinde ortak kurallar: basamak ≥ 0.27 m, 2a+b 0.60–0.64 m, sahanlık ≥ kol genişliği, dönerin dar ucu ≥ 0.10 m, iki
  yanda korkuluk. Kaynak ve “mevzuat onayı değildir” notu dosyadadır.
- **Girdi.** `cad.arch.audit`'in `profile`'ı `profiles` listesi oldu.
  - En çok bir rampa ve bir merdiven profili verilir.
  - Katalogda olmayan ya da aynı türden ikinci profil `invalid_arch_audit`'tir, yeri `profiles[i]`. `validate` de reddeder.
  - `profile` bir gün yaşadı ve dosyaya yazılmıyordu; kaldırıldı.
- **Bulgular.** Yeni `stairs` grubunda yedi kod var:
  - `stair_riser_over_profile`;
  - `stair_tread_short`;
  - `stair_step_over_profile`;
  - `stair_width_narrow` (Düzelt: genişliği profilinki yapar);
  - `stair_landing_short` (sahanlık kol genişliğinden kısa; Düzelt: sahanlığı null, yani kol genişliği yapar);
  - `stair_winder_narrow` (iç basamak = basamak × (yarıçap − genişlik/2) ÷ yarıçap);
  - `stair_railing_missing` (Düzelt: iki yana korkuluk).

  Bulgular uyarıdır; kilitli katmandaki merdivenin düzeltmesi yoktur.
- **Masaüstü.** Denetim sekmesinde iki seçim var:
  - “Rampa: Yok / TS 9111 özeti”;
  - “Merdiven: Yok / Konut, asansörlü / Konut, asansörsüz / Diğer yapılar / Daire içi”.

  Öntanımlılar TS 9111 özeti ve Konut, asansörlü; ipucu profilin adını ve notunu söyler. API'de öntanımlı yok.

### 3. Merkez direği (`post`)

- Döner merdivenin `post`'u direğin çapıdır.
- Yalnız döner merdivende olur. Çapı sıfırdan büyük olmalı; yarıçapı merdivenin iç yarıçapından küçük olmalı.
- Planda yayın merkezinde çember olarak, okun ardından çizilir; üst katta da çizilir.
- Döner merdiven aracında “Merkez direği (M)” ayarı var; Öznitelikler'de satırı boş bırakılırsa direk kalkar.
  `cad.arch.edit`'te `post: null` kaldırır.

### 4. Üst katta gösterim

- **Ne çizilir.** Etkin katın hemen altındaki katın kesmesi olan merdiveni, etkin katın planında yukarıdan görüldüğü
  gibi çizilir. Bu Yalnız etkin kat ve Etkin kat ve alt kat görünümlerinde olur. Çizilenler:
  - kesmeden sonraki bant kenarları (dönüşteki kenar gönyesinden başlar) ya da döner merdivende yaylar;
  - kesmenin ötesindeki sahanlık kenarları ve bitiş çizgisi;
  - kesme çizgisi, kesmenin ötesindeki rıhtlar ve korkuluklar;
  - kesmeden bitişe ok ve merkez direği.

  Kesikli çizgi yoktur.
- **Ne çizilmez.** Kesmesi olmayan merdiven planın kesme düzlemine ulaşmaz, gizli kalır. Bütün katlar görünümünde
  her merdiven kendi katının planıyla çizilir.
- **Seçme.** Yukarıdan görülen merdiven seçilmez ama kenetlenir; soluk değildir.
- **Uygulama.**
  - Çekirdek: `geom::arch::stair_upper_pieces`, depoda `StairSpec.upper`.
  - Kat görünümü `LevelLooks.upper` listesini verir. Ekranın `Spatial`'ı bu merdivenleri yukarıdan koyar.
  - Pafta haritası, kendi katı yukarıdan bir merdiven gösteriyorsa, kendi kat görünümüyle ayrı bir depo kullanır. Bu
    pafta PDF'inde (`kentos-plot`) de masaüstü paftalarında da böyledir.
  - Ekranın kat görünümü paftayı değiştirmez: ekran yukarıdan bir şey çizerken (`Spatial::draws_otherwise`) paftalar
    çizimin kendi deposundan okur.
- Bu, Archicad'in “Home and One Story Up” ayarı ile kırık çizginin “Hidden – Visible” seçimine karşılık gelir.

### 5. Metraj

- `cad.spaces.quantities`'in merdiven satırı (`stairs`) şunları verir:
  - kat, genişlik, rıht sayısı ve yüksekliği, basamak;
  - yükseklik (rıht × rıht yüksekliği);
  - yürüme çizgisinin uzunluğu (sahanlıklar dahil) ve sahanlık sayısı;
  - plan alanı (düz merdivende gönyeli şeridin halkası; döner merdivende yarıçap × genişlik × dönüş);
  - korkuluk uzunluğu (kenarlardan 5 cm içeride);
  - döner merdivende yarıçap ve iç basamak;
  - kata bağlılık ve kural.
- Toplamlara `stairArea`, `stairRailingLength` ve `stairRisers` eklendi.
- Süzgeçler mahallerinkidir; kullanım süzgeci merdivenleri almaz.
- Metraj penceresinde Merdivenler sekmesi var. XLSX ve ODS yedi sayfa yazar.

## Doğrulama

- **Ortak durumlar.** KentOS kodu olmadan bağımsız başvurulardan üretildi:
  - `cad.stairs.create.levels.json`: 34 durum (`arch_command_cases.py`). Bağ, kat yükselince izleme, geri al/yinele,
    Değiştir'in bağlaması ve bağı kaldırması, merkez direği ve retler.
  - `cad.arch.audit.stairs.json`: 33 durum (`stair_access_cases.py`). Dört profil, yedi kod, Düzelt'ler, iki grup
    birlikte, retler.
  - `cad.spaces.quantities.stairs.json`: 21 durum (`stair_quantity_cases.py`).
  - Erişilebilirliğin 24 durumu `profiles` girdisiyle yeniden yazıldı.
  - Çekirdek başvurusu `arch_cases.py` 53 durum. Beş yeni durum var (merkez direği, üst kat) ve her merdiven durumuna
    üst kat parçaları eklendi.
  - Eski denetim (41), düzenleme (48), merdiven (30+33+26) ve rampa durumları değişmeden geçer. Koşucunun yanlış
    beklentiyi yakaladığı ayrıca denendi.
- **KCAD.**
  - Yeni örnekler: `stair-levels.kcad`, dört kural bozuk örneği ve önceki şema örneği.
  - Bilinmeyen şema örneği `schema-version-126` oldu.
  - Bağımsız Python okuyucusu 1295 dosyanın hepsini okudu; kodek 230 test.
- **Takımlar.**
  - uygulama 242 + 67, alan 121, etkileşim 785 + 66;
  - komutlar 920, sözleşmeler 1061 + 33 + 10;
  - geometri çekirdeği 534 + 284, başsız 64, pafta çıktısı 16 + 49;
  - masaüstü sahne testi `stair_scenes::the_stair_tool_binds_shows_above_is_measured_and_audited`;
  - etkileşimde `stair_above` (2 test).
- **Gerçek PDF** (`stair_levels.rs`). Zemin kata bağlı U merdiven 1,10 m kesmeyle çizildi; `cad.sheets.levels`'in iki
  paftası yazıldı.
  - poppler'ın `pdftotext`'i iki paftayı buldu.
  - `pdftoppm` 1. kat paftasını 50 dpi çizdi. Kesmeli merdivenle 22869, kesmesizle 22039 koyu piksel çıktı; fark
    yukarıdan görülen merdivendir.
  - Kat 3.40 m'ye yükselince rıhtlar izledi (18 × 0.189); pafta aynı kaldı.
- **Gerçek arayüz resimleri** (`.run/shots/arac-merdiven-*`, iki tema, 1440×900 ve 1100×650):
  - kat-arac, kat-yazildi, ust-kat, ust-kat-alt, kat-yukseldi, kat-oznitelik;
  - direk, metraj, profil-denetim, profil-duzeltildi.

## Açık kalanlar

- Kat yüksekliği değişince rıht sayısının yeniden hesaplanması; şimdi rıht sayısı korunur.
- Kat değişince kesme rıhtının yeniden bulunması; kesme bugün rıhtın sırası olarak saklanır.
- Birden çok kat geçen merdiven ve ara katlardaki gösterimi.
- Profilde tolerans, yangın yönetmeliğinin kaçış genişliği, kurumun kendi profil dosyası.
- Merdivenin kesitte ve 3B'de gösterimi; döner merdivende karışık kol.
