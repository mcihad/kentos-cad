# ADR 8069: Mahal, zon ve emsal modeli — balkonların %20 sınırı, emsal zonları, bölüm grupları

- **Durum:** kısmen uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-08,
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03; uygulama sırasında B8. İlk kabulün iki ölçütü sağlandı (balkon sınırı;
  zonların emsale dahil alanı tablonunkine eşit); katlar arasında yer değiştiren alanın denetimi ve eğik çatı altındaki
  mahal hacmi kaldı (§4).
- **Bağlam:** ADR [8030](8030-emsal-taks-zoning-badge-and-table.md) (emsal ve TAKS: mahal mahal sınıflama, %30 sınırı,
  tablo), [8029](8029-regulation-areas-of-independent-units.md) (yönetmeliğin alanları), [8031](8031-condominium-units-annexes-and-land-shares.md)
  (bağımsız bölüm, eklenti), [0773](0773-building-program-units-and-design-brief.md) (bina programı).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 22/1-l (değişik RG 25/7/2019-30842; değişik ibare RG 11/7/2021-31538), 4/1-ğ-3
  (bağımsız bölüm net alanı), 5/8 (emsal harici %30).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** değişmedi.

## §1. Araştırma

Resmî Gazete'nin ve mevzuat.gov.tr'nin sitesi bu bilgisayarda sertifika hatası veriyor; metin yönetmeliğin 1 Temmuz 2026
tarihli derlemesinden (alomaliye.com, `planli-alanlar-01-07-2026.pdf`, pdftotext ile) okundu:

- **22/1-l:** bağımsız bölüm net alanının %20'sini geçmemek kaydıyla açık çıkma şartlarını taşıyan balkonlar (kat ve ara
  sahanlıklar dahil merdiven evi ve açık havuzlarla birlikte) katlar alanına dâhil edilmez.
- **4/1-ğ-3:** bağımsız bölüm net alanı duvarlar arasındaki temiz alandır; açık çıkmalar, balkonlar, teraslar, eklenti
  mekânlar ve ortak alanlar bu alanda değerlendirilmez.
- **İkinci kaynak:** İzmir Büyükşehir Belediyesinin bir görüşü aynı okumayı aktarır (bağımsız bölüm net alanının %20'si).
- **Düzeltme:** ADR 8030 §101 "açık balkon ve çıkmaların katlar alanının %20'si" diyordu; doğrusu bağımsız bölümün net
  alanının %20'sidir.

## §2. Karar

1. **Balkon:** açık hava programı (`SpaceBrief.outdoor`) olan ve adı ya da kullanımı "balkon" olan mahal; teras değil.
2. **Sınır:** her bağımsız bölümün (programda türü `unit`) balkonları, bölümün net alanının %20'sine kadar dışarıdadır.
   Net alan bölümün kendi, kapalı ve emsale dahil sayılan mahallerinin net alanlarının toplamıdır (4/1-ğ-3: balkon, teras,
   eklenti, ortak alan girmez).
3. **Aşan kısım** katlar alanına girer; bölümün balkonları arasında alanlarıyla orantılı paylaştırılır (artık rastgele son
   balkona verilmez). Rapor bunu `balcony_over` olarak ayrıca tutar.
4. **Balkonun ölçüsü** raporun mahal ölçüsüdür (dış kontur; ADR 8030'un 4/ğ-1 okuması). Bunun idarece teyidi beklenir
   (**K40**).
5. Bir bölüme bağlanmamış ya da ortak alana ait balkon bu sınıra bakılmadan dışarıda sayılmaya devam eder (§4).
6. **Emsal zonları** (EK-6 Tablo 6.60, `mevzuat/bim/ek-6-2-zorunlu-f-z.md`, PDF s. 102): her kat ve her sayılış biçimi
   için bir `IfcSpatialZone`; zemini o kattaki mahallerin konturlarının birleşimi (raporun 4/ğ-1 konturu), yüksekliği katın
   mahallerinin en yükseği. Ad EK-5'e göre `MZN` kodlu, uzun adı "Zemin kat emsale dahil alan"; `PredefinedType`
   USERDEFINED, `ObjectType` EMSAL; `TREpys_EmsalOzellikSeti.EmsalDurumu` DAHIL ya da HARIC; dayanağı Description'da
   (4/1-şş ve 21; 22/1 ve 5/8; 5/8'in kendi mahalleri ayrı HARIC zonu). Zon katına
   `IfcRelReferencedInSpatialStructure` ile bağlanır (mekânsal yapının parçası değildir). DIGER (idarenin özel
   düzenlemesiyle emsal artışı) KentOS'ta yok.
7. **Balkonun dahil kısmı (K41):** zon çokgendir, bir balkonun %20'yi aşan kısmı çokgenle bölünmez; balkon HARIC zonda
   kalır, dahil kısmının alanı DAHIL zonun açıklamasında yazılır ("… 2.36 m² balkon da dahil"). Zonların alanı ile
   açıklamadaki pay birlikte tablonun emsale esas alanıdır. İdarenin teyidi beklenir. %30 sınırını aşan emsal harici alan
   binanın bütününe ait bir sayıdır, zonlara bölünmez; tabloda yazılır.
8. **Bölüm grupları** (EK-6 Tablo 6.68): bina programının her bölümü bir `IfcZone`; adı `ZON` kodlu bölüm numarası, uzun
   adı bölümün adı, `ObjectType` türü (Bağımsız bölüm, Eklenti, Ortak alan, Dış alan); mahalleri (balkonları dahil)
   `IfcRelAssignsToGroup` ile.
9. **Zorunlu alanlar:** zonlar ve gruplar `kentos_ifc::required`'ın nesnelerindendir; EK-6'nın satırları onlarda da
   denetlenir.

## §3. Sınamalar

- **Masaüstü** (`zones_tests`, 3): paiy_area_tests'in elle denetlenen iki bölümlü binası (net 25,7125 m²); 1. bölümün 7,5
  m²'lik balkonundan 0,20 × 25,7125 = 5,1425 aşan 2,3575 m² emsale girer, katlar alanı 90 + 2,3575, tabloda kendi satırı
  "2.36"; 2. bölümün 4 m²'si sınır içinde; 1. bölüm balkonu 5 m² olunca hiçbiri girmez, satır da yok. Modelin zemin kat
  DAHIL zonu 90 m² (15 × 6), HARIC zonu 11,5 m² (7,5 + 4); DAHIL zon ile açıklamasındaki balkon payı tablonun emsale esas
  alanı 92,3575 m² (bağımsız: 15 × 6 + 7,5 − 0,20 × 4,675 × 5,5); bölüm 1 ve 2 odası ile balkonu, ortak alan holü; zon ve
  grupların zorunlu alanları tam. Mevcut emsal ve TAKS sınamaları (`paiy_area`, `site_plan`) değişmeden geçer.
- **`kentos-ifc`** (32): zon katına başvurulu, durumu `TREpys_EmsalOzellikSeti`'nde, grup mahalini `IfcRelAssignsToGroup` ile
  tutar.
- **`kentos-domain`:** 35; uygulamanın `arch_ifc` sınamaları; ortak süzgeçler 172 geçti.
- **Resim** `.run/gui-evidence/emsal-zon-8069/` (açık, koyu): iki bölümlü plan, balkonlar ve Emsal ve TAKS tablosu.

## §4. Sıradaki adımlar (BIM-08, IFC-03)

- Kat ve sınıf bazında karşılaştırma (genel toplam eşitken katlar arasında yer değiştiren alan ya da yanlış sınıf): KentOS'un
  kendi modelinde zonlar ve tablo aynı mahallerden gelir; anlamlı olduğu yer teslim edilen dosyanın okunmasıdır (IFC-08).
  Olgusu denetim olgularına girecek; eklenti sistemi başka bir ajanda değişirken bu adım onunla birlikte yapılacak.
- Mahalin türü (iç, dış, otopark) `IfcSpace.PredefinedType`'ta; elle düzeltilen emsal durumu (mahal başına).
- Mahalin hacmi: eğik çatı altında kesilmiş, kapalı hacim.
- Çizimde denetim: kapanmayan mahal, katının kot aralığında olmayan mahal; bölümü verilmemiş balkon.
- (Yapıldı) Tablo satırı: Emsal ve TAKS tablosunda "Balkonların %20'yi aşan kısmı, emsale eklenen", yalnız aşan kısım varken.
