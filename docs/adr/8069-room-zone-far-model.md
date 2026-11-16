# ADR 8069: Mahal, zon ve emsal modeli — balkonların %20 sınırı (ilk adım)

- **Durum:** sürüyor (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-08, [ifc-uyum.md](../../plan/ifc-uyum.md)
  IFC-03; uygulama sırasında B8. Bu ilk adım ilk kabulün balkon ölçütünü sağlar; emsal zonları, gruplar, mahal hacmi ve
  IFC zonları sonraki adımlardır (§4).
- **Bağlam:** ADR [8030](8030-emsal-taks-zoning-badge-and-table.md) (emsal ve TAKS: mahal mahal sınıflama, %30 sınırı,
  tablo), [8029](8029-paiy-area-definitions.md) (yönetmeliğin alanları), [8031](8031-condominium-units-annexes-and-land-shares.md)
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

## §3. Sınamalar

- **Masaüstü** (`zones_tests`): paiy_area_tests'in elle denetlenen iki bölümlü binası (net 25,7125 m²); 1. bölümün 7,5 m²'lik
  balkonundan 0,20 × 25,7125 = 5,1425 aşan 2,3575 m² emsale girer, katlar alanı 90 + 2,3575; 2. bölümün 4 m²'si sınır
  içinde; 1. bölüm balkonu 5 m² olunca hiçbiri girmez. Mevcut emsal ve TAKS sınamaları (`paiy_area`) değişmeden geçer.
- **`kentos-domain`:** 35 sınama geçti.

## §4. Sıradaki adımlar (BIM-08, IFC-03)

- Emsal zonları: kat kat kapalı çokgen, durumu (dahil, hariç, öbürü) ve maddesi; `IfcSpatialZone` ve
  `TREpys_EmsalOzellikSeti.EmsalDurumu`; kat ve sınıf bazında tabloyla karşılaştıran Python kuralı (genel toplam eşitken
  katlar arasında yer değiştiren alan ya da yanlış sınıf).
- Gruplar: bağımsız bölüm, eklenti, ortak alan `IfcZone`; mahalin türü (iç, dış, otopark).
- Mahalin hacmi: eğik çatı altında kesilmiş, kapalı hacim.
- Çizimde denetim: kapanmayan mahal, katının kot aralığında olmayan mahal; bölümü verilmemiş balkon.
- Tablo satırı: Emsal ve TAKS tablosunda balkonların %20'yi aşan kısmı.
