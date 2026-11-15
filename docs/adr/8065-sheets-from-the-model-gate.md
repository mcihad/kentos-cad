# ADR 8065: Paftalar modelden — türevin durumu olguda, kural Python'da, ruhsat yayınının kapısı

- **Durum:** uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-04; uygulama sırasında B4. İlk kabul
  sağlandı (§8). Eksik kalanlar §9'da.
- **Bağlam:** ADR [8042](8042-section-derived-from-a-section-mark.md) ve [8043](8043-elevation-derived-from-an-elevation-mark.md)
  (kesit ve görünüşün eskimesi), [8038](8038-room-list-and-graphic-joinery-list.md) (listeler), [8037](8037-turkish-title-block-sheet-sets-pdfa.md)
  (Ruhsat takımı ve PDF/A yayını), [8064](8064-ek3-ek4-2d-checks-python.md) (EK-3/EK-4 kuralı ve `cad` olguları),
  [8063](8063-dated-digital-delivery-scope.md) (masaüstünün kural çağırıcısı).
- **Dayanak:** Hazırlama Yönetmeliği Md. 9/1 ("CAD çizim dosyaları, BIM modelinden üretilir. BIM modeli ile CAD çizim
  dosyası PDF/A nüshasının birbiriyle uyumlu olması zorunludur"); EK-9 Tablo 9.1 satır 21 ("Teslim edilen PDF paftaları
  BIM modelinden üretildi mi?", PDF s. 239).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Bağımlılık:** dış bağımlılık yok; `kentos-processing`'e
  varsayılan açık `mod-arch` özelliği (uygulamanın `mod-arch`'ı): bulut işinin olguları masaüstününkiyle aynı olsun diye.

## §1. Karar

1. **Neyin eskiyebildiği ölçüldü.** Hesap tabloları, mahal listesi ve doğrama listesi canlıdır: çizim değiştiği adımda
   yenilenir (ADR 8038 §3, `calc_tables`), eskimez. Eskiyebilen türev kesit ve görünüştür (blokları, ADR 8042 §5); EK-9
   formunun cevapları takım kurulurken yazılır (§9).
2. **Olgular Rust'ta (`cad` bölümü):** her katmanın nesneleri sınıfıyla (model, açıklama, serbest çizim) ve ilk serbest
   nesneleri; kesit ve görünüşler durumlarıyla (`current`, `stale`, `notDrawn`, `gone`; `sections_query::rows`); nesnesi
   silinmiş etiketler ve bağı kopmuş ölçüler.
3. **Kural Python'da (`dijital_proje.turetim`, paket 0.3.0; bim-plan K10).** Bulgularının hepsi hatadır.
4. **Ruhsat yayınının kapısı:** Ruhsat takımından açılan Paftaları toplu yayınla her Denetle'de kuralı sorar; bulgu
   varken, kural çalışamazken ya da cevap çizimin bugünkü kuşağına ait değilken Yayınla açılmaz. Başka yayınlar eskisi
   gibidir (eski kesit uyarıdır, ADR 8042 §6).
5. **Aynı bulgular Mimari denetimde:** Mevzuat grubunda, "Dijital Proje Yön. 9" ve EK-9 satır 21 ile; eski kesitin
   Düzelt'i Yeniden çiz'dir (tek adım). Kopmuş etiket ve ölçü Denetim'in Etiketler ve Ölçüler grubunda zaten vardır:
   orada ikinci kez listelenmez, kapıda sayılır.
6. **Kaynağın revizyonu:** yayın klasörünün `yayin.json`'u çizimin revizyonunu ve SHA-256'sını, her dosyanın SHA-256'sını
   zaten taşır (ADR 1090); ruhsat paftası hangi revizyondan üretildiğini oradan söyler.

## §2. Kurallar

| Kural | Kod | Ne | Düzeltme |
|---|---|---|---|
| Eski kesit ya da görünüş | `digital_derived_stale` | Model, çıkarıldığından beri değişti | `cad.sections.edit` refresh |
| Kaynağı silinmiş kesit | `digital_derived_gone` | İşareti silinmiş, yerleşimi duran kesit ya da görünüş | — |
| Model katmanında serbest çizim | `digital_free_on_model_layer` | Model nesnesi taşıyan katmanda çizgi, yay, tarama … ("“MM-DUVA-TUGLA” katmanında 14 serbest çizim") | — |
| Kopmuş bağ | `digital_annotation_broken` | Nesnesi silinmiş etiket, bağı kopmuş ölçü | Denetim › Etiketler, Ölçüler |

- **Nesnenin sınıfı:** model (duvar, açıklık, mahal, döşeme, aks, kolon, kiriş, çatı, korkuluk, merdiven, rampa,
  yerleştirilmiş blok), açıklama (ölçü, etiket, yazı, kılavuz, tablo, işaret), serbest çizim (nokta, çizgi, çoklu çizgi,
  kapalı alan, daire, yay, elips, eğriler, sonsuz çizgi, ışın, tarama). Resim, nokta bulutu ve kayıt sayılmaz. Bir model
  nesnesinin kendi çizdiği çizgi modelindir: mutfak dizisinin tezgahı ve çizgisi dizinin üyeleridir (`doc.casework()`,
  ADR 0727 §5). Bu ayrım olmadan 2+1 dairenin mutfak katmanı (MM-TEFR-MUTFAK, 6 blok, bir kapalı alan, bir çoklu çizgi)
  KentOS'un kendi çıktısında bulgu veriyordu; EK-3/EK-4 izi bunu yakaladı.
- **Model katmanı** model nesnesi taşıyan katmandır; sözlük gerekmez. Yalnız serbest çizim taşıyan katman (detay
  katmanı) bulgu değildir.

## §3. Olgular

`kentos_extensions::audit_facts::cad`'e: katmanın `model`, `annotations`, `free`, `firstFree` (en çok 100); bölümün
`derived` (tür, ad, işaret, durum, yerleşim sayısı; ada göre) ve `broken` (tür, kimlik; kimliğe göre) listeleri. Mimari ve
izin bölümleri gibi isteğe bağlıdır. Altın dosyalara yalnız bu alanlar eklendi; `mimari.json`'un `main`'deki kaçış yolu
farkı korunarak bırakıldı (ADR 8064 §3).

## §4. Masaüstü

- **Kapı** (`apps/desktop/src/shell/exchange/permit_gate.rs`): Ruhsat takımı'nın Paftaları oluştur'u yayın penceresini
  kapıyla açar. Denetle kuralı kısa bir süreçte sorar (pencerenin iş parçacığı beklemez), cevap sorulduğu andaki
  kuşağı taşır. Pencerenin sonunda "Ruhsat kapısı: paftalar modelden (Md. 9/1)": bulgular kırmızı, onarımları altında;
  "Denetim'de göster" pencereyi kapatıp Mimari denetimi açar. Açıkken "Açık: kesitler ve görünüşler güncel, model
  katmanlarında serbest çizim ve kopmuş bağ yok."
- **Denetim:** `arch_audit/standards.rs` artık `dijital_proje.mimari.denetle`'yi (EK-3, EK-4 ve türev) tek süreçte sorar.

## §5. Crate ayrımı

Yeni crate yok. Kesitin durumu `kentos-native-application`'ın `sections_query`'sinden (mimari özelliği), olgunun biçimi
`kentos-extensions`'ta, ölçümü `kentos-processing`'de, kural Python paketinde.

## §6. Sınamalar ve kanıt

- **Paket** (`tests/test_turetim.py`, 5): her kural için bir geçen, bir kalan durum; düzeltmenin girdisi; `mimari`'nin iki
  kuralı birlikte vermesi.
- **`kentos-processing`, `kentos-extensions`:** bölümün yeni alanları, sırası, sınırı; SDK şeması; altın dosyalar.
- **Masaüstü** (`permit_gate_tests`, kesit binası `fixtures/arch/v1/section-building.kcad`, KentOS kodu olmadan yazılmış):
  B-B kesiti çıkarılıp takım kurulunca kapı açık; giriş kapısı GK01 10 cm yükselince "B-B kesiti eski" ile kapalı; Yeniden
  çiz ile açık; duvarın katmanına çizilen çizgi "1 serbest çizim var" ile kapatır, olgu çizginin kimliğini verir, silinince
  açık. Mimari denetimde aynı eski kesit bulgusu Dijital Proje Yön. 9 ve EK-9 satır 21 ile; Düzelt yeniden çizer.
- **Resimler:** `.run/gui-evidence/dijital-turetim-8065/` (açık ve koyu): kapı kapalı (eski kesit ve serbest çizgi),
  kapı açık.

## §7. Ölçüm

Kapının sorusu bütün çizimin `cad` olgusunu ve her kesitin yeniden çizimini ister (`state_of`). Kesit binasında (81 nesne,
2 kesit işareti) kural süreci dahil cevap testte bir saniyenin altında geldi; büyük çizimde ölçülmedi (§9).

## §8. Kabul (plan: BIM-04 ilk kabul)

| Ölçüt | Durum |
|---|---|
| Bir kapı değişince kesit "eski" olur ve paket durur; yeniden üretimden sonra geçer | var (kesit binası, GK01) |
| Görünüş | aynı kural ve olgu (`elevation`); bu işte ayrı bir görünüş durumu sınanmadı |
| Doğrama listesi ve mahal listesi "eski" olur | olmaz: canlıdırlar, değişiklik aynı adımda listeye girer (ADR 8038 §3) |
| Duvar katmanına çizilen serbest çizgi raporda nesnesiyle | var: bulgu sayısıyla, olgu ve Denetim nesnenin kimliğiyle |

## §9. Eksik kalanlar

- **İstisna:** gerekçeli istisna (ADR 0732) Python bulgusunu tanımıyor; kapıyı geçmenin yolu bulguyu düzeltmektir.
- **EK-9 formunun 21. sorusu** bugün `kentos_ifc::quality`'nin Rust cevabıdır; bu kayıttan cevaplanması formun Python'a
  taşınmasıyla (IFC-08) gelir. Formun cevaplarının takım kurulduktan sonra eskimesi de oradadır.
- **Pafta içeriğinin sınıfı** katman düzeyindedir; paftanın görünüm penceresinin gösterdiği nesneler ayrıca sayılmaz.
  "Detay çizimi (kaynağı ve amacı kayıtlı)" sınıfı yok: detay kendi katmanında serbest çizimdir.
- **Büyük çizimde ölçüm** ve kapının sorusunun kesit sayısıyla büyümesi ölçülmedi.
- **PDF'in kendisi** kaynağın revizyonunu taşımıyor; yayın klasörünün `yayin.json`'u taşıyor.
