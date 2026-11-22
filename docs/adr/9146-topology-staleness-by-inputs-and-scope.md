# ADR 9146: Topoloji yalnız okuduğu değişince eskir; tolerans karşılaştırmasıyla, denetim kapsamıyla söylenir

- **Durum:** kabul edildi (2026-11-22); adım VY-45. Plan: [plan/veri.md](../../plan/veri.md) VY-45.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  14. maddesi (P1):
  - etiket ayarı kaydedilince topoloji “sonuçlar eski olabilir” dedi;
  - tolerans, depolama hassasiyeti ve kabul eşiği ayrı girişler; hangi birimde, neyle karşılaştırıldıkları
    söylenmiyordu;
  - istenen: geometri, ilişki ya da kural değişimini kartografik değişiklikten ayırmak; kapsamı, sürümü ve sağlayıcısına
    erişilemeyen katmanı raporlamak; sabit veride bulgulara gidip istisna, düzeltme, geri alma ve yeniden denetimi
    sınamak.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Denetimin eskidiği çizimin `generation` ya da `revision` sayacı değişince söyleniyordu (ADR 0202 §5, 0524). Katmanın
  stili ve etiketi de bu sayaçları değiştirir; kural ise yalnız geometriye bakar.
- Belgenin değişen nesne günlüğü (`change_mark`, `changes_since`) okuyucuya bir işaretten beri eklenen, değişen ya da
  silinen nesneleri verir. Katman ağacının değişimi bu günlükte yoktur.
- Denetimin okuduğu (`Gathered`):
  - kuralların katmanlarındaki nesnelerin şekli ve katmanı;
  - kuralların, toleransın, istisnaların ve profilin özeti (`stamp.rules`);
  - bağlı katmanların kaynak sürümü (`stamp.sources`);
  - veri katmanlarının tabloları (ADR 9124).
- Kural penceresi toleransı “bu uzaklıktaki iki yer aynı yerdir” diye anlatıyordu; profilin iki uzunluğunu genel bir
  cümleyle anıyordu.
- Kuralların kendi karşılaştırmaları (`geometry-core` `topology_rules`):
  - sarkan uç: uç başka çizgiye `d ≤ tolerans` ise değer;
  - çakışma: örtüşen parçanın genişliği `2 × alan ÷ çevre > tolerans` ise bulgudur.
- Profilin uzunlukları (ADR 1260 §4):
  - depolama hassasiyeti: koordinat yuvarlanmaz; altında taşınan köşe de taşınmış sayılır;
  - kabul eşiği: kaynak köşeyi daha uzağa taşıyan onarım adımı reddedilir;
  - tolerans hassasiyetin en az iki katıdır.
- **Araştırma.** Başvuru alınan CBS uygulamasının belgeleri (22 Kasım 2026'da okundu):
  - çözünürlük benzersiz koordinat değerlerini ayıran en küçük uzaklıktır (saklama adımı);
  - tolerans koordinatlar arasındaki en küçük uzaklıktır: biri öbürünün toleransı içindeyse aynı yerdedir; ilişki ve
    topoloji işlemleri bunu kullanır;
  - kirli alanlar nesnelerin eklendiği, değiştiği ya da silindiği yerlerdir; doğrulama yalnız onlarda çalışır, önceki
    doğrulamadan beri düzenleme yoksa denetlenecek bir şey yoktur.

  Kaynaklar:
  - [uzamsal başvuru özellikleri](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/the-properties-of-a-spatial-reference.html);
  - [topoloji: kirli alanlar ve doğrulama](https://doc.esri.com/en/arcgis-pro/latest/help/data/topologies/topology-in-arcgis.html).

## §2 Karar

1. **Okuduğu değişince eskir.** Denetim günlüğün o anki işaretini saklar (`Checked.mark`). Sonuç şu durumlarda eskidir:
   - kuralların, toleransın, istisnaların ya da profilin özeti değişti;
   - bağlı bir katmanın kaynağı başka sürümde;
   - günlükteki bir nesne kuralların katmanlarında eklendi, silindi, şekli değişti ya da katman değiştirdi;
   - günlük işarete kadar uzanmıyor: temkinli olarak eski sayılır.

   Stil, etiket ve öznitelik değişimi sonucu eskitmez. Yanıt denetimin ve çizimin sayaçlarıyla saklanır; görünüm onu her
   karede yeniden hesaplamaz.
2. **Uzunlukların karşılaştırması.** Kural penceresi yazılan değerlerle söyler:
   - “Tolerans 0.001 m: bir çizgi ucu başka bir çizgiye 0.001 m ya da daha yakınsa değer, daha uzaksa sarkan uçtur; iki
     alanın örtüşen parçası genişliği (2 × alan ÷ çevre) toleransı aşarsa bulgudur.”
   - Profil varsa hassasiyetin ve kabul eşiğinin işi, toleransın en az iki kat kuralı ve gerekçe zorunluluğu.
   - Profil yoksa bunların uygulanmadığı.
3. **Kapsam ve sürüm.** Sekmenin çubuğunun altında son denetimin kapsamı durur: “Kapsam: Parsel, Bina, Yol · 6 kural ·
   18 nesne · bütün çizim · çizim sürümü 0 (şimdi 1) · tolerans 0.001 m”.
   - Değişenler denetlendiyse “değişenler (N nesne)” yazar.
   - Okunamayan kaynak (“denetlenemedi”, ADR 9124) ve kuralı olup katmanı olmayan katmanlar eskisi gibi uyarı
     satırlarındadır.

## §3 Kanıt

- **Masaüstü** (`gis/topology/municipal_tests.rs`, paylaşılan `topology-rules.kcad`):
  - 19 bulgu; her satıra basınca görünüm bulgunun kutusunu içeriyor;
  - #16'nın pafta kenarındaki ucu “Pafta kenarı: …” gerekçesiyle istisna, projede saklanıyor;
  - #2/#3 çakışması `subtractFirst` ile düzeltiliyor, denetim yineleniyor;
  - geri alınca eski, Değişenleri denetle ile çakışma geri geliyor, istisna duruyor;
  - kapsam “değişenler (…)”, Denetle'den sonra “bütün çizim”.
  - İkinci sınama: parselin etiketi değişince eskimiyor, kapsam “(şimdi …)” diyor; yolun taşınması ve toleransın
    değişmesi eskitiyor.
- `tolerance_words`: değerler, birim ve karşılaştırma; profilsiz cümle.
- Topoloji (durdurma, değişenleri denetleme, kaynak, nesne, tarif dahil), iz, şerit, modül ve geçmiş sınamaları (84)
  geçiyor.
  - `traces::ek_rules_tests` toplu koşuda bir kez kaydedip açmada takıldı; tek başına geçiyor. Bu, iki 2+1 daire izinin
    bilinen yarışıdır.
- **Resimler** (`.run/gui-evidence/kullanim-9146/`, Git'e girmez):
  - kural penceresi profil değerleriyle;
  - etiket değişiminden sonra eskime uyarısı olmayan sekme ve kapsam satırı.

## §4 Kalan

- **Veri katmanı kaynağı.** Veri katmanının kaynağında dışarıdan yapılan değişiklik ve katmanın başka tabloya
  bağlanması eskimeyi tetiklemiyor; bağlı katmanların kaynak sürümü tetikliyor.
- **Gerçek veritabanı.** PostGIS kaynaklı katmanda eskime ve “denetlenemedi” sınaması koşturulmadı (Docker).
- **Kurum profili örneği.** Belediyenin gerçek eşik değerleri yazılmadı: değer uydurulmaz, kurumun profili beklenir.
- Web (TODOS.md §0.2).
