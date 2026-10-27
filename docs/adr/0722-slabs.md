# ADR 0722: Döşeme, balkon, teras ve şaft — duvarlardan döşeme, serbest kenarda korkuluk, süpürgelik ve kaplama geçişi

- **Durum:** kabul edildi (28 Ekim 2026); plan/architecture.md ARCH-ELEM-01'in kalan alt maddeleri ve kabulü.
- **Bağlam:** ADR 0708 (mahal boşluğu: şaft), ADR 0342 (mahal ve metraj), ADR 0641 (açık kat), ADR 0713 ve 0720
  (etkin kat), ADR 0339–0341 (duvar, birleşim, açıklık), ADR 0652 (Metraj CSV), ADR 0265 (yeni şemanın kuralı),
  ADR 0347 (mimari nesnelerin DXF'i), ADR 0622 §9 (araç = komut).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Mimari planda döşeme yoktu: katın döşemesi, balkon ve teras ya çizilmiş bir alanla (kalınlığı, kotu, korkuluğu yok)
ya da hiç gösterilmiyordu. Şaft ADR 0708 ile mahalin boşluk işaretiydi ama bir döşemeden düşülecek bir şey yoktu.
Mahal metrajı çevreyi veriyordu; süpürgelik boyu (kapı genişlikleri düşülmüş çevre) ve kaplama geçişleri (iki farklı
döşeme kaplamasının kapıdaki eşiği) elle hesaplanıyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Archicad 29 — Slab Tool Settings](https://help.graphisoft.com/AC/29/INT/_AC29_Help/150_UserInterfaceToolSettings/150_UserInterfaceToolSettings-6.htm):
  döşemenin kalınlığı ve “Offset to Home Story” (katına göre kot farkı) ayrı ayarlardır; kesit üst/alt referans
  düzlemi seçilir; döşeme konturu çizilir ya da Sihirli değnekle (Magic Wand) kapalı bir sınırdan alınır.
- [Revit — Create a Floor](https://help.autodesk.com/cloudhelp/2015/ENU/Revit-Model/files/GUID-A688CED0-E08C-4ED3-A8E8-14CBE78902BB.htm):
  “Pick Walls” duvarlardan sınır çizer; “Offset” kenarı duvardan kaydırır, “Extend into wall (to core)” sınırı
  duvarın içine taşır; konturun içindeki kapalı döngüler delik (şaft, merdiven boşluğu) olur.
- Graphisoft topluluk tartışmaları (belge değil): Archicad'in zone çevresi kapıları düşmez; süpürgelik için kullanıcılar
  kapı genişliğini ayrıca çıkarır. KentOS bunu metrajın kendi kuralı yapar (§5).

## Karar

1. **Model.** Yeni nesne türü `slab` (Döşeme): halkalar ve delikler (`parts`, mahalinkiyle aynı biçim), tür
   (`role`: Döşeme, Balkon, Teras), kalınlık (0 < t ≤ 5 m), katına göre kot farkı (`offset`, |f| ≤ 100 m), kat ve evre
   (`arch`), malzeme (en çok 120 karakter), kenar (`edge`: Korkuluk ya da Parapet, eni ≤ 2 m) ve metraj dışı işareti
   (`excluded`). İkinci alan motoru yoktur: çekirdek döşemeyi mahalin `Shape::Space` şekliyle çizer, seçer, kenetler,
   taşır, esnetir ve tutamaçlarını verir. Şaft ayrı tür değildir: döşemenin deliği (Kes) ya da ADR 0708'in boşluk
   mahali; boşluk mahali döşemeden metrajda düşülür.
2. **Sınır.** Üç yol, tek komut: çizilmiş halkalar; duvarların dış yüzleri (`outline`: imlecin içinde olduğu en dış
   yüz grubu, bina konturu); odanın iç yüzleri (`room`: mahal araması, net yöntem). Kat verilmişse yalnız o katın
   duvarları, verilmemişse bütün duvarlar okunur (ADR 0641'in kuralı). Kapanmayan sınır `slab_not_enclosed`.
3. **Kenar ötelemesi.** Her halkanın her kenarı kendine paralel kayar (pozitif dışa); delikler ters yöne. Halka
   ters dönerse ya da kaybolursa, bir kenar yön değiştirirse, kenarlar kesişirse ya da delik dış kenara veya başka
   deliğe değerse atomik ret `slab_offset_failed`; araç aynı hesabı önizlemede kırmızı uyarıyla gösterir ve yazmaz.
4. **Korkuluk ve parapet.** Kenar yalnız serbest kenarlar boyunca türer: halkanın, ortası aynı katın (kat yoksa
   bütün) duvarlarının bir yüzünde (1e-6 m) durmayan kenarları. Kenarın çizgileri (`runs`) saklanmaz, belge
   döşemeyi ve duvarları izleyerek aynı adımda yeniden yazar (`follow_slabs`); duvar taşınınca korkuluk izler.
5. **Metraj.** `cad.spaces.quantities` üç şey ekler, eski sütunlar ve sıralar aynıdır:
   - **Süpürgelik:** mahalin çevresi eksi, okuduğu geçişlerin (kapı; eşiği döşemeye inen Boşluk) ekseni boyunca
     halkasının düz kenarlarının duvar bandı içinde paralel giden kısmı. Yaylı kenar düşmez.
   - **Geçişler:** iki yanında kaplaması bilinen ve farklı iki mahal olan geçiş: genişliği ve iki kaplama.
   - **Döşemeler:** alan (delikler düşülür), aynı kattaki boşluk mahallerinin birleşiminin örttüğü alan, net alan,
     çevre, hacim (net × kalınlık), üst kot (katın kotu + kot farkı), kenar boyu. Metraj dışı döşeme listelenir,
     toplamlara katılmaz. Toplamlarda süpürgelik, döşeme alanı ve hacmi, korkuluk ve parapet boyu (sıfırsa yazılmaz).
6. **Araç.** Mimari › Döşeme (`tool.slab`): Tür (T), Sınır (S: Dış yüzler, Oda, Çiz), Kalınlık (K), Kot farkı (F),
   Kenar (E), Kenar eni (W), Kenar ötelemesi (Ö), Malzeme (M), Delik (D); çizerken Geri (G). İmleç bir sınırın içindeyken kontur, alanı ve
   serbest kenarların korkuluğu önizlenir; tek tık yazar, Esc vazgeçer, tek geri alma adımı. Delik açıkken tıklanan
   oda ya da çizilen alan, altındaki döşemeden kesilir. Seçim süzgecinde ve bağlamsal sekmede Döşeme; Öznitelikler'de
   Döşeme türü, Kalınlık, Kot farkı, Üst kot, Kenar, Kenar eni, Kenar uzunluğu, Malzeme, Kat, Evre, Metraj, Alan,
   Çevre, Hacim. Metraj penceresinde Döşemeler ve Geçişler sekmeleri, Mahaller'de Süpürgelik sütunu; CSV açık sekmeyi,
   XLSX/ODS beş sayfayı yazar.
7. **Komutlar.** `cad.slabs.create` (çizilmiş, `outline`, `room`; birden çok döşeme tek adımda) ve `cad.slabs.edit`
   (Değiştir, Kenarı ötele, Kes, Ekle; hepsi ya da hiç). `cad.levels.assign` döşemeye kat verir, `cad.points.construct`
   döşemenin düz kenarlarını kullanır. Python `kentos.cad.slabs`, MCP araçları (düzenleme yıkıcı listede), manifest.
8. **Dosyalar.** KCAD v2 şema 123 (opt-in: döşemesi olmayan dosya eski şemada ve aynı baytta kalır), tipli sütunlarda
   tür 30, `FORMATS_VERSION` 98; sunucu döşemeyi `cad_definition` olarak saklar, izdüşümü mahal gibi çokgen (göç
   0034). DXF'e mimari nesnelerin kuralıyla (ADR 0347) parçalarının adsız bloğu ve KENTOS verisiyle; geri okumada
   döşeme olarak gelir. KCAD okuyucusu blok tanımının içindeki döşemeyi reddeder (`bad_value`, mimari nesnelerin
   kuralı).

Yolda düzelen: sunucu, mahal ayırıcı çoklu çizginin `spaceSeparator`'ını (ADR 0650) geometriden saklıyor ve
kaybediyordu; artık tanımı saklanır (taşıma testi `every_sample_object_crosses_the_stored_feature_and_back`).

## Doğrulama

- Ortak durumlar KentOS kodu olmadan `scripts/fixtures/slab_cases.py` başvurusundan: `cad.slabs.create` 47,
  `cad.slabs.edit` 38 (L kontur, avlu, ıslak hacim, şaft boşluğu, iki parça, balkonun üç serbest kenarı, teras
  parapeti, kenar ötelemesi ve dört başarısızlığı, metraj dışı, gizli ve kilitli katman, retler). `cad.spaces.quantities`
  25 ve `cad.spaces.edit` 38 durumu süpürgelik ve geçişlerle `space_command_cases.py`'den yeniden yazıldı; kolon,
  kaynak, kapsam ve ayırıcı başvuruları (72, 63, 90, 105) aynı kuralla geçer. ADR 0651'in donmuş 20 metraj cevabı
  ADR 0722'nin eklediklerinden arındırılınca bit bit aynıdır.
- Takımlar: application 242 + 67, desktop 1355, interaction 783 + 66, commands 911, contracts 1061, domain 121,
  geometry-core 534 + 283, KCAD 226 ve bağımsız Python okuyucusuyla 1285 dosya, headless 64, MCP, Python 374;
  clippy `-D warnings`. DXF: `arch_dxf_cases.py` 11 mimari nesne. Sunucu DB testleri bu oturumda koşmadı
  (PostGIS yok); DB'siz testler geçti.
- Gerçek arayüz resimleri (`.run/shots/arac-doseme-*`, iki tema, 1440×900 ve 1100×650): dış yüzlerden önizleme,
  balkon ve korkuluk, ıslak hacim deliği, 5 cm aşağıdaki ıslak hacim döşemesi, yazılmış döşemeler ve Öznitelikler,
  ters çeviren öteleme uyarısı, Metraj'ın Döşemeler, Mahaller (Süpürgelik) ve Geçişler sekmeleri.
- Bilinen ve ilgisiz: `kentos-formats` `multidim::every_case_is_read_as_the_reference_reads_it` bu dilimden önce de
  düşüyordu.

## Açık kalanlar

- Döşemenin katmanlı yapısı (kaplama, şap, taşıyıcı katmanlar), kesitte gösterimi ve 3B.
- Katlar arası şaftın tek nesne olarak bütün katları kesmesi; şimdilik her katın boşluk mahali.
- Eğimli döşeme ve rampa birleşimi; döşeme altı (kiriş) kotu.
