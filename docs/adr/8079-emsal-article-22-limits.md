# ADR 8079: Emsalin 22. madde sınırları — kapıcı dairesi, depo eklentisi, kütüphane ve merdiven evi

- **Durum:** kısmen uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (B15'in "emsalin 22.
  madde sınırları" kalanı); [architecture-advanced.md](../../plan/architecture-advanced.md) ADV-22'nin "22. maddenin
  %20/%50/70 m² sınırları" ve "kat ara toplamı" kalanları. 22/1-j, 22/1-m ve 5/8'in bodrum listesi kaldı (§6).
- **Bağlam:** ADR [8030](8030-emsal-taks-zoning-badge-and-table.md) (emsal ve TAKS, mahal mahal sınıflama, %30),
  [8069](8069-room-zone-far-model.md) (balkonların %20'si, emsal zonları, K40, K41),
  [8031](8031-condominium-units-annexes-and-land-shares.md) (eklenti), [8072](8072-buildings-shared-basement-location.md)
  (binanın ±0,00'ı), [8078](8078-regulation-rule-set-by-application-date.md) (başvuru gününe göre kural seti, K60–K65).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 22/1-ç, -f, -k, -l (RG 25.7.2019/30842; 23.1.2021/31373; 11.7.2021/31538),
  44/1 (RG 3.7.2017/30113), 4/1-ııı (normal kat), 5/8 (%30).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building` bölümüne
  isteğe bağlı iki alan (`limits`, `uses`); başka sözleşme değişmedi. Metrekare cetveli aracının öntanımı değişti (K69).

## §1. Araştırma

Resmî Gazete'nin sitesi bu bilgisayarda sertifika hatası veriyor (ADR 8078 §1); metinler yönetmeliğin 1 Temmuz 2026
derlemesinden (`planli-alanlar-01-07-2026.pdf`, pdftotext ile) ve değişiklik yönetmeliklerinin yayımlanmış metinlerinden
okundu:

| Kaynak | Okunan | KentOS'ta |
|---|---|---|
| PAİY 22/1-ç (Değişik: RG 25.7.2019/30842, 7 nci madde) | "Bu Yönetmelik uyarınca yapılması zorunlu olan kapıcı dairelerinin toplam 70 m2'si" | Kapıcı dairelerinin 70 m²'si birlikte dışarıda (K66) |
| PAİY'nin ilk metni (RG 3.7.2017/30113), 22/1-ç | "Bu Yönetmelikte öngörülen asgari sayıda her bir kapıcı dairesinin 75 m2'si" | 2019 değişikliği "her bir … 75" yerine "toplam 70" dedi: sınır dairelerin toplamınındır |
| PAİY 44/1-a, -b (RG 3.7.2017/30113, değişmedi; 1.10.2017'den yürürlükte) | Katı yakıtlı binada konut bölümü 40'tan fazlaysa, öbür ısıtmada konut kullanımlı binanın bağımsız bölümü 60'tan fazlaysa kapıcı dairesi zorunlu | Zorunluluk paketin kuralı (K67) |
| PAİY 22/1-k (Değişik: RG 25.7.2019/30842) | "Bütün cepheleri tamamen toprağın altında kalan bodrum katlarda yer alan, tek başına bağımsız bölüm oluşturmayan, bağımsız bölüm net alanının %50'sini geçmeyen depo amaçlı eklentiler" | Toprak altındaki katta bölümün depo eklentisi bölüm net alanının %50'sine kadar dışarıda (K66) |
| PAİY'nin ilk metni, 22/1-k | Konut bölümünün brüt alanının %10'unu aşmayan depo eklentileri | Kural seti 25.7.2019'dan başlar |
| PAİY 22/1-f (Değişik: RG 23.1.2021/31373, 1 inci madde; 6 ncı madde: yayımı günü yürürlükte) | Alışveriş merkezlerinde ve 40'tan fazla bağımsız bölümlü binaların normal katlarında, ortak merdivenle bağlantılı, dışarıdan erişimi olmayan, toplamda 50 m²'yi geçmeyen, ticari kullanıma konu edilemeyen, ortak alan niteliğinde kütüphane odası | 50 m² sınırı tabloda; koşullar paketin kuralı (K66, K67) |
| PAİY 22/1-l (Değişik: RG 25.7.2019/30842; %20 ve açık havuzlar RG 11.7.2021/31538) | "… balkonlar, kat ve ara sahanlıkları dâhil açık veya kapalı merdiven evi ile açık havuzlar" | Merdiven evi %30 içinde dışarıda (K68); balkon ADR 8069'da |
| PAİY 4/1-ııı | "Normal kat: Bodrum, zemin, tesisat katı ve çatı arası dışında kalan kat veya katları" | 22/1-f'nin "normal katlarında"sı |
| PAİY 5/8 | 22/1'in emsal harici alanları toplam emsale esas alanın %30'unu aşamaz; bodrumda merdivenlerin asgari alanı bu hesaba girmez | %30 ADR 8030'da; bodrum merdiveninin asgari alanı kaldı (§6) |
| RG 25.7.2019/30842, 15 inci madde | Yayımı günü yürürlüğe girer; geçiş hükmü yok | 22/1-ç ve -k'nin hâli 25.7.2019'dan |

- **Okuma (ikinci kaynak):** bir hukuk bürosunun 22/1-ç rehberi sınırı dairelerin toplamına uygular: iki zorunlu 50 m²'lik
  dairede 70 m² dışarıda, 30 m² emsale girer; zorunlu olmayan daire emsal harici sayılmaz.
- **Bakanlık görüşleri** sitesi (yapı yönetmelikleri görüşleri) sertifika hatası veriyor; okunamadı. 2018 tarihli bir
  görüşün özeti (bodrumdaki kapıcı dairesi) 2019 değişikliğinden önce olduğu için kural seti onu okumaz.
- **22/1-l'nin 11.7.2021'den önceki payı:** 2019 değişikliğinin bir özeti %10, 2021 değişikliğinin bir karşılaştırma
  tablosu "%15 iken %20" diyor. Çelişkili: kural seti 11.7.2021'den başlar (K61'in uzman incelemesi).
- **Benzer araçlar:** aranan genel emsal hesaplayıcıları 22/1'in kullanım sınırlarını tabloda ayrı göstermiyor (17
  Kasım 2026). KentOS kararı, ADR 8069'un balkon satırı gibi, sınırı aşan kısmın kendi satırıdır.
- **KentOS'un zemini:** model tabii zemini tutmaz; kesitler tesviye edilmiş zemini ±0,00'da çizer (EK-4 4.30,
  ADR 8042 §4).

## §2. Karar

- **K66 — Sınırlar alan hesabının içinde** (`kentos_domain::zoning`; ADR 8069 K40'ın okumasıyla). Emsal ve TAKS tablosu,
  IFC'nin emsal zonları ve olgular aynı katlar alanını okur:
  - **22/1-ç:** adı ya da kullanımı Kapıcı olan, ya da programdaki grubunun adı ya da niteliği Kapıcı olan kapalı mahal
    kapıcı dairesidir. Kapıcı dairelerinin 70 m²'si birlikte dışarıda, aşan kısım emsale girer (`CARETAKER_AREA`).
  - **22/1-k:** bir bağımsız bölümün eklenti grubunun (`annexOf`) mahali, adı, kullanımı ya da grubunun niteliği Depo ise
    ve kat toprak altındaysa dışarıdadır. Sınır bölümün net alanının %50'si
    (`STORAGE_SHARE`); net alan 22/1-l'nin net alanıdır (bölümün kendi, kapalı, emsale dahil mahalleri). Toprak altında
    olmayan kattaki depo emsale dahil kalır.
  - **Toprak altındaki kat:** katın üstü (kotu ve yüksekliği; yüksekliği yoksa üstteki katın kotu) binasının ±0,00'ının
    (ADR 8072) altında ya da onda. Modelin tesviye edilmiş zemini ±0,00'dadır; eğimli arsa modelde yoktur. Üstü
    bilinmeyen kat toprak altında sayılmaz.
  - **22/1-f:** adı ya da kullanımı Kütüphane olan mahaller 50 m²'ye kadar birlikte dışarıda (`LIBRARY_AREA`).
  - **Grubun sözü:** grup bir mahale ancak kendi adının emsale dahil saydığı mahalde rol verir. Depo eklentisindeki
    kömürlük kömürlüktür (22/1-i), sınırsız dışarıda; su deposu ve teknik hacim depo sayılmaz.
  - **Aşan kısım** her sınırın mahallerine alanlarıyla orantılı paylaştırılır (artık son mahale verilmez). Bir mahal onu
    okuyan ilk sınırın altındadır (balkon, depo, kapıcı, kütüphane). Kullanım listesinin emsal sınıfı her zaman önce gelir.
  - **Tablo** sınırı aşan her kısmı kendi satırında söyler (aşan kısım yoksa satır da yoktur): "Depo eklentilerinin
    %50'yi aşan kısmı, emsale eklenen", "Kapıcı dairelerinin 70 m²'yi aşan kısmı, emsale eklenen", "Kütüphanenin 50
    m²'yi aşan kısmı, emsale eklenen".
  - **IFC:** sınırı aşan kısım katın emsale dahil zonunun açıklamasında yazılır ("22/1-k: … aşan 17.14 m² depo
    eklentisi de dahil"). Katta emsale dahil mahal yoksa mahalin kendi emsal harici zonunda "emsale dahil" diye yazılır.
    Zonların alanı ve açıklamalardaki paylar birlikte tablonun emsale esas alanıdır (K41). Farklı sayılan iki komşunun
    ortak duvarının cephe köşesi bunun dışındadır (§6).
- **K67 — Binanın bütününe bağlı koşullar paketin kuralı** (`tr.kentos.mimari-mevzuat` 0.3.0, `emsal`; K10).
  - Kural olguların `building.limits`'ini okur: sınır, bölüm, mahaller, alan, izin verilen, aşan, hepsi ortak alan mı.
    Ayrıca binaların kullanımını (`building.uses`) okur.
  - **22/1-ç'nin zorunluluğu (44/1):** konut bölümü D, bağımsız bölüm U ile:

    | Durum | Sonuç |
    |---|---|
    | D > 60, ya da D > 40 ve U > 60 | İki ısıtmada da zorunlu: geçti |
    | D = 0, ya da D ≤ 40 ve U ≤ 60 | Zorunlu değil: `paiy_caretaker_not_required`, kaldı |
    | 40 < D ≤ 60 ve U ≤ 60 | Katı yakıtlıysa zorunlu: eksik veri (çizim ısıtmayı söylemez) |
    | D ≤ 40 ve U > 60 | Katı yakıtlı değilse zorunlu: eksik veri |

  - "Konut kullanımlı bina" en az bir konut bölümü olan binadır; idarenin teyidi beklenir.
  - **22/1-f'nin koşulları:** bina alışveriş merkezi (binanın kullanımı) ya da 40'tan fazla bağımsız bölümlü olmalı,
    kütüphane normal katta (bodrum, zemin, adında tesisat ya da çatı geçen kat dışında; zemin, adında Zemin geçen ya da
    kotu ±0,00'a en yakın kattır) ve ortak alan grubunda olmalı. Biri tutmuyorsa `paiy_library_not_exempt`, kaldı.
    Hepsi tutuyorsa aynı kod uzman incelemesidir: ortak merdivenle bağlantı, dışarıdan erişim ve ticari kullanım
    çizimden okunmaz.
  - **22/1-k ve 22/1-l** özet satırında tablonun sayımını söyler; bulgusu yoktur.
  - **Kural seti:** 22/1-ç ve -k 25.7.2019, 22/1-f 23.1.2021, 22/1-l 11.7.2021 hâliyle. Kullanımı olan maddede daha
    eski başvuru günü `regulation_text_unknown`'dur (uzman incelemesi, K61): "emsal tablosu bugünkü hâliyle sayar".
- **K68 — Merdiven evi emsal harici** (22/1-l; %30 içinde). Adı ya da kullanımı merdiven olan, bir bağımsız bölümün
  kendi mahali olmayan kapalı mahaldir (merdiven altı ve merdiven altı deposu değil; yangın merdiveni 5/8'in %30
  dışındaki sınıfında kalır). Kural kaçış yollarının merdiven okumasıyla aynıdır (`room_names::stair_name`).
- **K69 — Kat ara toplamı:** Metrekare cetveli aracı (Metraj ▾ › Metrekare cetveli) her katı ara toplamıyla kapatır:
  "Zemin kat toplamı", "1. kat toplamı"; katlar kotlarına göre, en alttan (ADR 8038). Çizelgenin Ara toplam seçeneği bu
  hazırda öntanımlı açıktır; kapatılabilir. ADR 8029'un "kat adının yazı sırası katların sırası değil" engeli ADR
  8038'in kat sıralamasıyla kalkmıştı.
- **Sınırlar:**
  - Zorunlu dairelerden fazla çizilen kapıcı dairesi ayrılmaz: 70 m² toplam sınırı onu da sayar.
  - Birden çok binalı parselde 44/1-a parselin toplam konut bölümünü okur; binanın kendi 60'ı okunmaz.
  - Bunlar idarenin teyidini bekler.

## §3. Uygulama

- **Alan hesabı** (`crates/native/domain/src/zoning.rs`; sınırlar kendi dosyasında, `zoning/limits.rs`):
  - `ZoningLimit` (Balcony, Storage, Caretaker, Library) ve `ZoningLimitUse`; `ZoningReport::{storage_over,
    caretaker_over, library_over, limits}`; `ZoningRoom::limit`.
  - `Document::zoning_limits` (imar durumu olsun olmasın), `buried_levels`.
  - Sabitler `STORAGE_SHARE`, `CARETAKER_AREA`, `LIBRARY_AREA`; tablonun üç satırı.
- **IFC** (`kentos_ifc::arch::EmsalArea`): `storage`, `caretaker`, `library` payları ve açıklamanın cümleleri
  (`limited_parts`). `arch_ifc::emsal_areas` payı emsale dahil zona, yoksa katın emsal harici zonuna verir.
- **Olgular:** `kentos_extensions::audit_facts::building::EmsalLimit`; `BuildingFacts::{limits, uses}`.
  `kentos_processing::audit_facts::building::measure` doldurur. SDK'nın `factsSchema`'sı yeniden yazıldı;
  `fixtures/audit-facts/v1/mimari.json`'a planın balkon sınırı eklendi.
- **Metrekare cetveli:** `ScheduleTool::of_preset` hazır `AreaTable` için `subtotals: true`
  (`crates/native/interaction/src/arch/tag_tools.rs`).
- **Paket:** `catalog/standards/mimari-mevzuat` 0.3.0:
  - `emsal.py` (`kapici`, `kutuphane`, `depo`, `balkon`, `kural`, `kontrol`);
  - `data/surumler.json`'a dört madde; `mimari.KURALLAR`'a `emsal`;
  - manifestte `emsal` denetimi (`denetim:emsal_denetimi`).

## §4. Sınamalar

- **Bağımsız durumlar** (`fixtures/standards/v1/emsal.json`, 23): beklenenler 22/1 ve 44/1'in metninden elle.
  `catalog/standards/tests/mimari-mevzuat/test_emsal.py` (3 sınama) geçti.
  - Kapıcı: 70 konut (geçti); 45 konut ve 62 bölüm; 61 konut (sınırın üstü); 80 m²'lik daire.
  - Kapıcı, kaldı: 30 konut; tam 40 (sınır); konutsuz büro.
  - Kapıcı, eksik veri: 50 konut; 41 konut ve 60 bölüm; 35 konut ve 70 bölüm.
  - Kapıcı, tarih: tarihsiz; 1.6.2018 başvurusu (uzman incelemesi).
  - Kütüphane: 45 bölüm (uzman); alışveriş merkezi; 20 bölüm; zemin; bodrum; ortak alan değil; iki koşul birden.
  - Depo ve balkon: sınır içinde, aşan; hiçbiri; 1.6.2020 başvurusunda balkon (uzman incelemesi).
  - `test_asansor.py` ve `test_kapilar.py` de geçti.
- **Masaüstü** (`emsal_limits_tests`, zones_tests'in 15 × 6 m planı iki katta; bodrum −3,00, 3,00 m; elle):
  - Toprak altındaki 30 m² depo: 0,50 × 25,7125 = 12,85625 dışarıda, 17,14375 içeride. Katlar alanı 60 + 17,14375;
    harici 12,85625 + iki merdiven evi + kazan dairesi. Tablonun satırı "17.14".
  - Bodrumun üstü ±0,00'ın 1 m üstünde: depo bütünüyle dahil, satır yok.
  - Kapıcı dairesinin üç mahali (90 m²): 20 m² içeride, mahal başına 20/3. Kütüphane 60 m²: 10 m² içeride.
  - IFC: bodrumda emsale dahil mahal yok; bodrumun emsal harici zonu "17.14 m² depo eklentisi emsale dahil" der.
    Zeminin emsale dahil zonu iki dairenin konturudur: 60 + 4 × 0,075 × 0,125 = 60,0375 m² (her daire iki cephe
    ucunda ara duvarın öbür yüzüne uzanır; tablo bu köşeyi merdiven eviyle yarı yarıya böler). Köşe payı düşülünce zon
    ve bodrumdaki pay tablonun 77,14375'idir.
  - Metrekare cetveli aracı (`paiy_area_tests`): iki katlı planda Zemin kat toplamı 90.00, 1. kat toplamı 90.00, toplam
    180.00.
  - Denetim: 2 konut bölümünde kapıcı dairesi `paiy_caretaker_not_required` (kaldı, üç mahal). Kural setinde 22/1-ç
    kaldı, öbür üçü kapsam dışı.
- **Olgular** (`audit_facts::building::tests`): bodrumdaki iki depo eklentisi 16 m², sınırları 16 m² (32 m²'lik dairenin
  yarısı).

## §5. Arayüz

- Yeni pencere ya da düğme yok. Emsal ve TAKS tablosu (Mimari › Belgele › Metraj ▾) yeni satırları kendiliğinden
  yazar.
- Denetim'in kural seti satırları 22/1-ç, -f, -k, -l'yi de söyler.
- Öznitelikler › İmar durumu oranları tablonun gösterim kuralıyla yazar (gürültü önce atılır). Önceden doğrudan iki
  basamağa yuvarlıyordu: 89,999… m²'lik taban alanında TAKS tabloda 0.23, burada 0,22 görünüyordu.
- Resimler `.run/gui-evidence/emsal-22-8079/`: Emsal ve TAKS tablosu ve Denetim'in kapıcı dairesi bulgusu, açık ve koyu.

## §6. Sıradaki adımlar

- **22/1-j:** bodrumdaki yönetim, toplantı, spor ve sosyal mekânlar. "Emsal hesabına konu alanın %5'i"nin çizimin
  katlar alanı mı, planın izin verdiği alan mı olduğu belirsiz; yeni kullanım sözcükleri ister — yapıldı: ADR
  [8147](8147-basement-common-rooms-22-1-j.md) (taban 21/2–3'ün kullanımıyla katlar alanı).
- **22/1-m:** kesme taş yığma geleneksel yapının balkon ve taşlıkları — yapıldı: ADR [8094](8094-structural-system-traditional-cut-stone.md) (taşıyıcı sistem beyanı; kalın taş duvarların payı orada §6).
- **5/8'in bodrum listesi:** bodrumda merdivenlerin, sığınağın ve tesisat alanlarının asgari alanı %30'un dışındadır.
  KentOS onları %30 içinde sayar; asgari alanın hesabı kaldı.
- **Emsal zonlarının cephe köşesi:** farklı sayılan iki komşunun (daire ve merdiven evi gibi) ortak duvarının cephe
  köşesi iki zonun çokgeninde de var. Tablo bu alanı yarı yarıya böler, zonlar bölmez: her köşe ara duvar kalınlığı ×
  dış duvarın yarısı kadardır (deneme planında 0,01875 m²). Köşeyi ara duvarın ekseninden bölmek — yapıldı: ADR
  [8097](8097-emsal-zone-facade-corner.md).
- **Isıtma beyanı:** 44/1'in ara durumunu kapatmak için künyede ya da binada ısıtma sistemi.
- **Eğimli arsa:** cephe cephe tesviye kotu (22/1-k'nin "bütün cepheleri").
