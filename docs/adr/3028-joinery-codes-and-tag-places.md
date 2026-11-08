# 3028 — Doğrama kodları, 90/210 ve etiketin yeri

Tarih: 2026-11-08. Durum: kabul (M28 birinci bölüm, ADV-16 ve ADV-03; plan S19, G10).

## Sorun

- **Kod yoktu.** Kapı ve pencere numaraları yerleştirirken K1, K2 …, P1 … diye her açıklığa ayrı veriliyordu (ADR 0714).
  Türkiye'de planda doğrama kodu yazılır: aynı tür ve ölçüdeki bütün kapılar aynı kodu taşır, doğrama listesi kod
  başına bir satırdır. ADR 0714 bunu "Numara tür kodu değildir" diye sınır olarak yazmıştı. Projenin kapı tipinin adı
  (ADR 0772) "K01" diye belgelenmişti, ama kabul dairesindeki tipler "İç kapı 90", "Giriş kapısı" gibi açıklayıcı adlar
  taşıyordu; kodun ayrı bir yeri yoktu.
- **Ölçü yazmıyordu.** Etikette ya da başka bir yerde "90/210" yoktu (ADV-16'nın kabulü: "Kapı ölçüsünde 90/210
  yazar").
- **Etiket bir şeyin üstüne biniyordu (S19, G10).** `cad.tags.create` açıklığın etiketini 4 yazı yüksekliği kuzeye
  koyuyordu (ADR 0344). Gerçek pencerede, kabul dairesinde Hepsini etiketle'nin 10 etiketi oda etiketlerine, kapı
  yaylarına ve düşey duvarlarda duvar çizgisine bindi. Kapıların numarası olmadığı için altıgenlerin içi de boştu
  (`.run/gui-evidence/mimari-m28/once/`).

## Araştırma (8 Kasım 2026)

- **İTÜ MIM 244 ders notu** (Dr. Ecem Edis, *Teknik Çizim İlkeleri*, 2013, s. 3; `referans/itu-dograma-kodlari.png`):
  - "Binada kullanılan, aynı türde olup farklı boyut ve özelliklere sahip yapı elemanlarına … özellik ve boyutlarını
    ayırt edici şekilde referans numaraları verilir." Kod tür ve ölçü başınadır, açıklık başına değil.
  - Kodlar: merdiven M, kapı K, camlı kapı CK, camekân CMK, pencere P, giriş kapısı GK, gömme dolap GD.
  - Aynı not ölçünün yerini "çizgi üzerine yükseklik, çizgi altına genişlik" diye yazar.
- **İçmimarlar Odası, İçmimari Proje Çizim ve Sunuş Standartları Şartnamesi** (2.3.7; s. 7, 12;
  `referans/icmo-2-3-7-genislik-yukseklik.png`):
  - Aynı tanım; pencere P1, P2, merdiven M1 … "mimari projelerde yapılmış olan kodlamaları ile kullanılır".
  - "Boyutları belirtilecek yapı elemanları ve donatıların çizgi üzerinde genişlik, çizgi altında yükseklik yazılır."
    İTÜ notunun tersi. Yaygın "90/210" yazımıyla bu kaynak uyuşuyor (genişlik önce).
  - 2.4 j: "pencere orta aksı üzerinde en ve yükseklik gösterilir."
- **Mimarlar Odası İstanbul Büyükkent Şubesi standardı** (2015; W4 §2.19): doğrama kodları K, P, GK, CK.
- **Revit** (*Example: Numbering Doors by Rooms*, 2027 yardımı): kapının Mark değeri oda numarasından ve bir harften
  verilir (101A, 101B). Odaya göre numara bu maddenin ikinci bölümüdür.

## Karar

- **Kod tipin üstündedir** (`OpeningType.code`; Revit'in Type Mark'ı gibi). Bir tipin bütün açıklıkları aynı kodu okur.
  - Kural: harfle başlar; harf, rakam, "-" ya da "." ile en çok 16 karakter. Projede tektir, büyük küçük harf ayırmadan.
  - Açıklığın numarası (K1, ADR 0714) aynen kalır. Yinelenen numara denetimi kodları saymaz.
  - Tipin kopyası kodu almaz; kitaplıktan gelen ve kitaplığa giden tip kodunu taşır.
- **`cad.openingtypes.edit`'e iki işlem eklendi:**
  - `code`: bir tipe kod ver ya da kodunu kaldır (null).
  - `codes`: kodu olmayan her tipe kendi türünün sıradaki kodu verilir, 01'den başlayarak, tipin çizimde ilk görüldüğü
    sırayla. Daha önce verilmiş kod kalır. Ayrıntılar `opening_codes`'ta:
    - Türler: kapı K; bir yanında oda olan, öbür yanında olmayan kapı GK (dış duvardaki giriş kapısı); pencere P;
      boşluk B (KentOS'un ADR 0714'teki harfi). Niş ve bacaya kod verilmez.
    - Bir tipin kapılarının hepsi dış duvardaysa tip GK olur, yoksa K.
    - Proje tipi izlemeyen açıklıklara önce her biçim ve ölçü için bir tip açılır. Tipin adı biçim ve ölçüdür (Kapı
      90/210); o yapıdaki açıklıklar bu tipi izler. Kilitli katmandaki açıklık atlanır.
- **Etiket değerleri `{code}` ve `{size}`** (`TAG_PLACES` 40). `{size}` santimetre olarak genişlik/yüksekliktir
  (90/210). Tam santimetre küsuratsız, değilse tek ondalıkla yazılır; lentosu olmayan açıklıkta yalnız genişlik.
  Hazır içerik "Doğrama kodu ve ölçü" = `{code}\n{size}`. Açıklığın varsayılan şablonu (`{number}`) değişmedi.
- **Doğrama kodları aracı** (`tool.openingCodes`; Mimari › Belgele › Etiket ▾'nün üçüncü satırı; DOGRAMAKODU,
  KAPIKODU):
  - Önizlemede her açıklığın yanında kodu, kartta tipler ("K01  İç kapı 90 · 4").
  - Enter ya da tık: kodlar ve Etiketle açıksa (E) etiketler tek geri alma adımında yazılır. Etiketsiz açıklıklar yeni
    etiket alır; etiketliler yerinde kalır, içerikleri koda ve ölçüye çevrilir.
  - Kart imleci izler (ADR 3018); fare kenara çekilince plan görünür.
  - İkon `openingCodes`: kapı (kanat ve yay) ve iki satırlı bir altıgen etiket.
- **Etiketin yeri** (`tags::opening_places`):
  - Önce kapının açılmadığı yüzde (pencerede odanın olduğu yüzde), açıklığın tam karşısında, duvar yüzünden yarım yazı
    boşluğu ötede. Sonra öbür yüz; kapının açıldığı yüzde yayın ötesi.
  - Ardından duvar boyunca çeyrek adımlarla en çok iki adım, sonra bir çerçeve dışarıda; her yüz sırayla.
  - Her yerin bir ağırlığı var:
    - duvar, kolon, donatı ya da kapı yayı 1;
    - ölçü çizgisi ½, ölçünün dik bağlama çizgisi 0,05;
    - etiket ya da ölçü yazısıyla örtüşme alanının on katı;
    - duvar boyunca her çeyrek adım 0,0375, her dış çerçeve 0,3;
    - etiketin kendi duvarından başka bir duvarın ardında görünmesi 2. Başka odaya düşen etiket o odanınki sanılır.
  - En hafif yer seçilir. Açıklığına yakın bir yer, uzaktaki boş bir yere tercih edilir.
  - **Komutun kendi yeri değişmedi.** Python ve ajan için `cad.tags.create` ADR 0344'teki gibi 4 yazı yüksekliği kuzeye
    koyar. Araçlar (Doğrama kodları, Hepsini etiketle) her etiketin yerini kendileri hesaplayıp `at` olarak verir.
    İlk denemede kural komutun içindeydi; ortak örneklerden 33'ü (komutların Python başvurusu) eski yerle yazılmıştı.
    Kuralın Python'a taşınması sonraki adımlardadır.
  - Hepsini etiketle'de etiketsiz açıklıkların yeri de böyle bulunur (en çok 500; üstünde komutun yeri).
- **KCAD şema 208:** Kodlu bir tip ya da `{code}`, `{size}` adlı bir etiket şablonu şema 208 ister; eski sürüm dosyayı
  `UnknownField` ile reddeder. Python başvurusu (`tools/kcad`, `kcad_v2_reference.py`) aynı kuralı yazar.

## Ölçüm

- **Testler:**
  - `opening_codes_tests::the_apartment_s_doors_and_windows_get_their_joinery_codes_and_tags_in_one_step` (kabul
    dairesi):
    - Kart: "4 tip · 10 açıklık · 4 yeni kod", "K01  İç kapı 90 · 4", "GK01  Giriş kapısı · 1".
    - Enter: İç kapı 90 K01, Giriş kapısı GK01, Pencere 120 P01, Pencere 60 P02.
    - 10 etiket: "K01\n90/210", "GK01\n100/210", "P01\n120/130", "P02\n60/70".
    - Her etiket duvar yüzünün ötesinde ve açıklığına 2,5 m'den yakın. Hiçbiri başka bir etikete (oda etiketleri dahil,
      gerçek çerçeve ölçüleriyle) binmiyor.
    - Tek geri alma etiketleri ve kodları götürüyor.
  - `opening_codes_tests::an_untyped_door_in_the_outer_wall_gets_a_type_and_gk01`: dış duvardaki tipsiz kapı "Kapı
    90/…" tipini ve GK01'i alıyor, numarası (K1) kalıyor, etiketi "GK01\n90/…".
  - `opening_codes::tests`: sıradaki kod (K03'ten K04, GK02, P1'den P02, B01); ölçü yazısı 90/210, 120/130, 90.5/130,
    lentosuz 90.5.
  - Ortak örnekler: `cad.openingtypes.edit`'e üç durum eklendi (kod verme, rakamla başlayan kod reddi, kodu olmayan
    tip). `TAG_PLACES` iletisi ve 19 277 durumun hepsi geçiyor.
  - KCAD: `opening-codes.kcad`; kırık `opening-codes-in-schema-207`, `-twice`, `-digit-first`. 239 geçerli dosya.
- **Gerçek pencere** (`fixtures/interaction/real/dograma-kodlari.txt`; kabul dairesi, 1440×900, sahibin ayarlarının
  kopyası):
  - Etiket ▾'de yeni satır ve ikon görünüyor.
  - Önizlemede kodlar açıklıklarda; Enter ile "Doğrama kodları verildi: K01 İç kapı 90, GK01 Giriş kapısı, …;
    açıklıklar etiketlendi".
  - 10 etiketin hiçbiri duvar çizgisine, kapı yayına, oda etiketine ya da ölçü yazısına binmiyor.
  - Hepsini etiketle aynı dairede de çakışmasız.
  - Resim: `.run/gui-evidence/mimari-m28/m28-once-sonra.png`, ikon `ikon/ikonlar.png`.

## Sonraki adımlar

- **Odaya göre numara** (bu maddenin ikinci bölümü): Açıklıkları numarala'da kapı, açıldığı odanın numarasını alır,
  odada birden çok kapı varsa harf eklenir.
- Ölçü zincirinde açıklığın yüksekliği (İçmimarlar Odası: "çizgi üzerinde genişlik, çizgi altında yükseklik").
- Çizelgede Kod sütunu; doğrama listesinin kod başına satırı (M38).
- CK (camlı kapı) ve CMK (camekân) türlerinin kendiliğinden ayrılması; bugün kod elle verilir.
- Pencerenin parapet yüksekliğinin etikette yazılması.
- `cad.tags.create`'te yerleştirme kuralının seçenek olması ve Python başvurusuna taşınması.
