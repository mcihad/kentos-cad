# ADR 9160: Bilgi kartının başlığı kaydı görüntüleme alanıyla adlandırır

- **Durum:** kabul edildi (2026-11-23); adım VY-59. Plan: [plan/veri.md](../../plan/veri.md) VY-59.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 13. maddesinin
  kalanı (ADR 9145 §4). Madde şöyle istiyor: “Bilgi kartı başlığına katman takma adı, görüntüleme alanı ve anahtar
  gelsin.” Katman ve anahtar ADR 9145'te başlığın altına geldi; görüntüleme alanı yoktu.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Kartın başlığı katmanın kart şablonundan gelir (Bilgi kartı ve eylemler, ADR 0225 §5). Şablon yoksa:
  - çizim nesnesinde nesnenin türü ve etiketi yazar (“Çoklu çizgi”);
  - veri satırında katmanın adı yazar (“parseller”).
- Kaydın kendi adı (“Gül Sokak”) başlıkta görünmüyordu. Değerlendirmede kart “Çoklu çizgi · durum planlı” diyordu ve
  kaydın hangisi olduğunu söylemiyordu.
- **Araştırma.** Masaüstü CBS'lerde katmanın bir görüntüleme adı vardır: bir alan ya da ifade. Bilgi sonucunun başında,
  form görünümünde kaydın adı olarak ve yer bulucuda kullanılır. Varsayılan, adında “name” bileşeni olan alandır; böyle
  bir alan yoksa katmanın ilk alanıdır. Kaynak:
  [QGIS vektör katman özellikleri, “Display Properties”](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html).

## §2 Karar

1. **Görüntüleme alanı** (`kentos_commands::display_field`, kartın sözleşmesinde: `layer_info.rs`):
   - önce adı kendisi bir ad sözü olan alan seçilir: ad, adı, isim, ismi, name, unvan, unvanı, başlık, title;
   - yoksa sözcüklerinden biri ad sözü olan ilk alan seçilir: “yol_adi”, “Mahalle adı”;
   - karşılaştırmada Türkçe harfler sadeleşir (ı, i, İ, I aynı; ş, ğ, ü, ö, ç düz harfleriyle), büyük-küçük harf fark
     etmez;
   - başka bir sözün içindeki “ad” sayılmaz: “ada”, “adres”, “adana_kodu”.
   - **İlk alana düşülmez.** Kadastroda ilk alan çoğu kez “ada” ya da bir koddur; adı olmayan sayıyı başlığa koymak
     kaydı adlandırmaz. Ad alanı yoksa başlık eskisi gibidir.
2. **Alanların sırası:** katmanın bildirilmiş alanları. Bildirilmiş alanı olmayan çizim katmanında nesnenin öznitelikleri
   kullanılır.
3. **Rol:** çağıranın rolünün okumadığı alan (ADR 9150) görüntüleme alanı olamaz; başlıktan sızmaz.
4. **Başlık** (kartın kendi başlık şablonu yoksa):
   - çizim nesnesinde tür ve etiketten sonra “ · ad” gelir: “Çoklu çizgi · Gül Sokak”. Etiket aynı adı söylüyorsa
     ad tekrar edilmez (“Nokta P1”);
   - veri satırında katmanın adından sonra satırın adı gelir: “mahalleler · Çınar”;
   - değer boşsa başlık eskisi gibidir.
5. **Seçim yeri:** başlık şablonu (“{yol_adi}”) görüntüleme alanını açıkça seçmenin yoludur. Bilgi kartı ve eylemler
   penceresinde Başlık kutusunun altı, şablon boş bırakılırsa hangi alanın kullanılacağını söyler: “… ve görüntüleme
   alanı “Ad”.” Alan yoksa bunu söyler.

## §3 Kanıt

- **Sözleşme** (`kentos-contracts`, `layer_info` sınamaları):
  - ad sözünün kendisi bileşik adlardan önce gelir;
  - “ADI”, “İSİM”, “Unvanı” bulunur;
  - “ada”, “adres”, “adana_kodu” ve “adim” bulunmaz.
- **Masaüstü** (`cad/info_card_tests.rs`):
  - “Ad: Kent Kitaplığı” verilen bina nesnesinin kartı “… · Kent Kitaplığı” diyor;
  - P1 noktasında etiket adı söylediği için ad tekrarlanmıyor;
  - rolün okumadığı “Ad” görüntüleme alanı olmuyor;
  - mahalleler tablosunun satır kartı “mahalleler · Çınar” diyor.
- `tests/attachments_tests.rs`: şablonsuz yol kartının beklenen başlığı “Çoklu çizgi · Gül Sokak” oldu.
- `gpkg_tests/status.rs`: kaynağa bağlı olmayan katmanın nesnesi ADR 9145'ten beri katman ve kimlik satırını veriyor;
  sınamanın eski “satır yok” beklentisi buna göre düzeltildi.
- İz, ek, GeoPackage, kart ve ortak masaüstü sınamaları geçiyor.
- **Resimler:** `.run/gui-evidence/kullanim-9160/`.
  - `01-bilgi-gorunen-ad`: kart “Yay · Kent Kitaplığı”, altında katman ve kimlik.
  - `02-kart-ayari-gorunen-alan`: Başlık kutusunun altında görüntüleme alanı “Ad”, önizlemede “Daire · Kent Kitaplığı”.

## §4 Kalan

- ADR 9145 §4'ün öbür kalanları: nesne ile veri satırının aynı tıklamada birlikte sayılması, etiket önizlemesinin kendi
  biçimiyle çizimi, veri katmanında etiket.
- Görüntüleme alanının yer bulucuda ve tablonun form görünümünde de kaydın adı olması.
- Web (TODOS.md §0.2).
