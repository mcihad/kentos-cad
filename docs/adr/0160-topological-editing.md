# ADR 0160: Topolojik düzenleme

- **Durum:** kabul edildi (2026-10-02). `HYB-06`. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** TODOS.md `HYB-06` (ilgili: `HYB-08`, `NUM-10`), ADR 0068 ve 0074 (tutamaçlar ve tutamaç menüsü), ADR 0047 (`cad.entities.edit`, Esnet), ADR 0142 (köşe kotu), ADR 0143 (çok parçalı alan), ADR 0148 (Topolojik temizlik), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); QGIS Topological editing, ArcGIS Pro Map Topology, Netcad Düzenle › Dokunan, Noktaları Dokunan.

## Bağlam

Komşu parseller, yol ve bina çizgileri ortak köşe ve kenar paylaşır. KentOS'ta tutamaç, tutamaç menüsü ve Esnet yalnız düzenlenen nesneyi değiştirir. Bir parselin köşesini taşımak onu komşusundan ayırır: araya boşluk ya da bindirme girer, alanlar ve toplu alan (ADR 0151) bozulur.

Topolojik temizlik (ADR 0148) ve Kenar eşleme (ADR 0159) ayrılmış sınırı sonradan onarır; düzenlerken korumazlar.

Öbür programlar:

- **QGIS:** Topological editing açıkken ortak köşe ve kenarlar taşınınca komşu nesnelerde de taşınır. Görünen ve düzenlenen her katmanda çalışır; komşuya eklenen köşenin kotu kenardan aradeğerlenir.
- **ArcGIS Pro:** Map Topology ortak sınırların bağını korur; ortak kenara köşe ekleme ve silme vardır.
- **Netcad:** Düzenle'nin Dokunan seçeneği dokunan nesneleri, Noktaları Dokunan noktaları da birlikte düzenler.

## Karar

### 1. Kip

**Topolojik düzenleme** bir çizim yardımcısıdır: Kenet, Orto ve Kutupsal gibi durum çubuğunda açılıp kapanır. Onlar gibi oturum ayarıdır (`drafting.topology`, ADR 0023), ilk değeri kapalıdır. Açıkken aşağıdaki düzenlemeler ortak köşe ve kenarları birlikte değiştirir.

Seçeneği:

- **Noktalar da** (`drafting.topologyPoints`): açıkken nokta nesneleri de ortak köşe sayılır (Netcad'in Noktaları Dokunan'ı). İlk değeri kapalıdır; kapalıyken ölçü noktası yerinde kalır, çizgiler ondan ayrılabilir. Durum çubuğu hücresinin sağ tık menüsündedir.

### 2. Ortak köşe ve kenar

- **Ortak köşe:** düzenlenen köşeyle 1 µm içindeki başka bir nesnenin köşesi:
  - çizginin ucu;
  - çoklu çizginin ve alanın köşesi (delikleri ve parçaları dahil);
  - “Noktalar da” açıksa nokta.

  Yalnız görünen ve kilitsiz katmanlardaki nesneler katılır.
- **Ortak kenar:** iki ucu da ortak köşe olan, iki nesnenin art arda iki köşesi arasındaki kenar (iki yönde de). Yaylı kenarda kabarıklık, yön aynıysa aynı, tersse ters işaretli olmalıdır (1e-9 içinde).
- **Kilitli komşu:** ortak köşesi kilitli katmandaki bir nesnede olan düzenleme yine yapılır. Kilitli nesne değişmez, ayrıldığı sayısıyla söylenir.

### 3. İşlemler

| İşlem | Komşulara ne olur |
|---|---|
| Köşe taşıma (tutamaç) | Ortak köşeler aynı yere taşınır. |
| Kenar ortasından yeni köşe | Ortak kenarlara aynı köşe eklenir. |
| Yaylı kenarı biçimleme (kenar ortası) | Ortak yaylı kenarlar aynı yaydan geçer. |
| Köşe sil (tutamaç menüsü) | Ortak köşe, iki yanındaki kenarlar da ortak olan nesnelerden silinir; sınır zinciri birlikte kısalır. |
| Düz kenar yap, Yaya dönüştür | Ortak kenar da aynı biçimi alır. |
| Esnet | Pencereye giren köşelerin ortak köşeleri de taşınır; seçili olmayan komşularda da. |

Bir nesnenin geçersiz kalacağı değişiklik (açık yolda 2'den, halkada 3'ten az köşe) o nesneye yapılmaz, söylenir.

### 4. Kot

Komşuların kotu, düzenlenen nesneninki gibi `cad.entities.edit`'in kot kuralıyla taşınır (ADR 0142, `ops::elevation`):

- taşınan köşe kotunu korur;
- eklenen köşe kenarın üstündeyse kenarın kotunu uzunluğa göre doğrusal alır, değilse kotsuz kalır;
- kenarın bir ucu kotsuzsa eklenen köşe de kotsuz kalır.

Böylece düzenlenen nesne ile komşuları aynı köşede aynı kotu taşır.

### 5. Önizleme

- **Sürükleme:** düzenlenen nesneyle birlikte komşuların değişen kenarları da çizilir.
- **Tutamaç:** üzerine gelinen ortak köşenin kartı kaç nesnenin köşesi olduğunu söyler (“3 nesnenin köşesi”).
- **Durum çubuğu:** kip açıkken hücresi vurgulanır.

### 6. Komut

Düzenleme `cad.entities.edit`'in var olan işlemleriyle (`grip`, `vertexRemove`, `straightEdge`, `arcEdge`, Esnet'in işlemi) yazılır. Her değişen nesne bir `update` olur ve hepsi tek geri alma adımında yazılır. Komşular verilen geometriyle değişir; komut bir şey hesaplamaz.

### 7. Ortak çekirdek

- **`ops::topology_edit`** (WASM `topologyEdit`):
  - **Girdi:** düzenlenen nesnenin yakınındaki komşular (biçim ve kilitli mi) ve düzenlemenin ortak yerlerdeki değişiklikleri. Komşuları platform, deponun dizininden düzenlenen yerlerin 1 µm çevresinde bulur; düzenlenen nesne aralarında yoktur.
  - **Değişiklikler:**
    - `move`: `at`'teki köşe `to`'ya;
    - `insert`: `a`–`b` düz kenarına `p`;
    - `bulge`: `a`'dan `b`'ye kenarın kabarıklığı `from` iken `to`;
    - `remove`: `prev` ile `next` arasındaki `at` köşesi.

    Taşımalar birlikte, hepsi eski yerlere göre yapılır (Esnet'in köşeleri zincirlenmez); öbürleri sırayla.
  - **Sonuç:** değişen komşuların biçimleri; kilitli ve geçersiz kalacak komşuların sayıları.
  - **Hesap yok:** köşeler değişikliklerin noktalarını, kenarlar kabarıklıklarını kopyalar; iki platform aynı bitleri verir.
- **Bağımsız başvuru:** `scripts/fixtures/topology_edit_cases.py`, kesin kesirlerle. Durumlar:
  - iki ve üç parselin ortak köşesi;
  - ortak kenara köşe ekleme (iki yönde);
  - yaylı ortak kenar;
  - delik ve parça köşesi;
  - noktalar açık ve kapalı;
  - kilitli komşu;
  - köşe silmede sınır zinciri;
  - geçersiz kalacak nesne;
  - kot aradeğerlemesi.

### 8. İş sırası

1. Çekirdek: `ops::topology_edit`, WASM, bağımsız başvuru ve ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Çekirdek:** `ops::topology_edit`:
     - `apply`: komşular ve değişiklikler; değişen komşular, kilitli ve geçersiz sayıları;
     - `changes`: düzenlenen nesnenin öncesi ve sonrası. Tutamaç, Köşe ekle ve sil, Düz kenar yap, Yaya dönüştür ve Esnet bu dört değişikliğe iner; platformların her işlem için ayrı eşlemesi gerekmez.

     WASM `topologyEdit`, `topologyChanges`; web cephesi `model/ops/topologyEdit.ts`.
   - **Başvuru:** `scripts/fixtures/topology_edit_cases.py` kesin kesirlerle, KentOS kodu olmadan (`fixtures/topology/v1/edit.json`):
     - 11 düzenleme durumu (§7'nin listesi, Yaya dönüştür, Düz kenar yap ve Esnet'in zincirlenmeyen taşımaları dahil);
     - 7 fark durumu (köşe taşıma, kapanış kenarına köşe, çizginin çoklu çizgi olması, yaylı kenar, köşe silme, Esnet, açık yolun ucu).
   - **Sonuç:** çekirdek (yerli) ve web (WASM) bit bit aynı. Ters yöndeki kenarın kabarıklığını ters çevirmeyi bozan bir deneme testi düşürür.
2. Kip ve tutamaçlar iki platformda: durum çubuğu, ayar, köşe taşıma, kenar ortası, önizleme; ortak iz `topology-edit.json`.

   *(2 Ekim: tamam.)*
   - **Ayar ve komutlar:** `drafting.topology`, `drafting.topologyPoints` (oturum ayarı, iki platform). Komutlar kendi simgeleriyle: `draft.topology` (Topolojik düzenleme) ve `draft.topologyPoints` (Topolojik düzenlemede noktalar da; şeritte “Noktalar da”). İkisi de Araçlar › Çizim yardımcıları'nda; Topolojik düzenleme, komut sürerken çizim alanının sağ tık menüsünde de.
   - **Durum çubuğu:** Topoloji hücresi. Noktalar da, hücrenin sağ tık menüsündedir.
   - **Tutamaç:** köşe taşıma, kenar ortasından yeni köşe ve yaylı kenarın biçimlenmesi komşuları aynı `cad.entities.edit` (`grip`) adımında yazar.
     - Komşular deponun dizininden, değişen yerlerin 1 µm çevresinden bulunur (web `tools/neighbours.ts`, masaüstü `kentos_interaction::neighbours`).
     - Yazıdan sonra iletiler: kaç komşunun da değiştiği, kilitli kaç komşunun değişmediği. Geçersiz kalacak komşu, köşe silmeyle (3. adım) olabilir.
   - **Nokta:** Noktalar da açıkken tutamaçla taşınan nokta da bir köşedir; yerindeki köşeler onunla gelir. Çekirdeğin `changes`'i noktanın taşınmasını `move` sayar; başvuruya iki fark durumu eklendi.
   - **Önizleme:** sürüklerken komşuların değişen biçimleri de kesikli çizilir.
   - **İz:** `topology-edit.json` (`topology-edit.kcad`) iki platformda. İz biçimine `bulges` ve `zs` beklentileri (1e-9 içinde) eklendi; yaylı ortak kenar ve kenar üstüne eklenen köşenin kotu böyle denetlenir.
3. Tutamaç menüsü ve Esnet iki platformda; resimler.

   *(2 Ekim: tamam.)*
   - **Tutamaç menüsü:** Köşeyi sil, Ortasına köşe ekle, Düz kenar yap ve Yaya dönüştür komşuları aynı adımda yazar; ortak yay ters işaretle, eklenen köşenin kotu kenardan.
   - **Esnet:** pencerenin taşıdığı köşelerin komşuları, seçili olmasalar da gelir.
     - Önizleme komşuları bir kez bulur (pencereyi bir metre doğuya esneten çekirdek taşımalarıyla); her imleç hareketinde yalnız çekirdeği çağırır.
     - Yazarken her şey çizimin o anki hâlinden yeniden hesaplanır.
     - İleti esnetilen nesneleri sayar; komşular ayrı iletide söylenir.
   - **Kart:** kip açıkken tutamacın kartı ortak köşeyi kaç nesnenin paylaştığını söyler (“4 nesnenin köşesi”; kilitli ve, Noktalar da açıkken, noktalar dahil). Sayıyı çekirdek, yerinde kalan bir taşımayla bulur.
   - **Yardımcılar:** platformların yardımcıları birden çok düzenlenen nesneyi alır (web `neighboursOf`, `neighboursAt`, `putRight`, `cornerCount`; masaüstü `neighbours_of`, `around`, `put_right`, `corner_count`).
   - **Sınama:** web `selectGripTopology.test.ts` ve masaüstü `crates/native/interaction/tests/all/topology_edit.rs` aynı elle hesaplanmış beklentilerle. Ortak ize Esnet adımları eklendi; kullanım senaryosuna kart ve Esnet resimleri.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

### 9. Kapsam dışı

- **Kalıcı topoloji modeli:** düğüm ve kenar kimlikleri (`NUM-10`). Ortaklık her düzenlemede geometriden bulunur.
- **Bitişik alan çizimi:** yeni alanı komşuya yaslama (`HYB-08`). Kendi ADR'sindedir.
- **Taşı ve Döndür:** bütün nesneyi taşıyan değiştirme araçları. Nesne bütün olarak taşınınca ortak sınır zaten ayrılır; bu, kullanıcının isteğidir.

## Sonuçlar

- Komşu parsellerin ortak köşe ve kenarları düzenlenirken birlikte kalır; boşluk ya da bindirme oluşmaz.
- Kip kapalıyken bugünkü düzenleme aynen sürer.
- Ortaklık geometriden bulunur; ayrı bir topoloji verisi tutulmaz.

## Doğrulama

- **Çekirdek:** bağımsız başvuruya göre iki platformda ortak köşe ve kenarlar ve her işlemin sonucu.
- **Arayüz:** ortak izle iki platformda, resimler iki temada, 1440×900 ve 1100×650.
