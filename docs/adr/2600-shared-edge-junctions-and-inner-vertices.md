# 2600 — Paralel kaydır'da ortak kenarın ortasındaki köşeler: üçlü birleşim, ara köşe ve delik

Tarih: 2026-11-08. plan/map-cad.md MAP-07, dördüncü tur. Durum: taslak. Plan satırı: “Ortak sınırın bir kez
düzenlenmesi bütün yetkili komşularda atomik uygulansın. Kilitli/yetkisiz komşu, yay, delik, üçlü birleşim ve çoklu
köşe durumlarında kısmi tutarsız veri yerine açıklanabilir ret/önizleme verilsin.”

İlgili:
- [ADR 0160](0160-topological-editing.md): Topolojik düzenleme (ortak köşe ve kenar birlikte);
- [ADR 1960](1960-parallel-shift-shared-boundary.md): Paralel kaydır'da ortak sınır (köşeler, kilitli komşu, yaylı
  ortak kenar);
- [ADR 1261](1261-topological-editing-stops-at-a-locked-neighbour.md): Kilitli komşuda durdur.

## Dayanak

- **ArcGIS Pro 3.4, “Move topological edges or features”**
  ([belge](https://pro.arcgis.com/en/pro-app/3.4/help/editing/move-topology-edges-and-nodes.htm), 8 Kasım 2026'da
  okundu): harita topolojisinde kenar taşınınca “Topologically connected edges stretch and remain connected”: kenara
  bağlı olan her şey bağlı kalır.
- **QGIS Kullanıcı Kılavuzu, Topological editing**
  ([belge](https://docs.qgis.org/3.10/en/docs/user_manual/working_with_vector/editing_geometry_attributes.html), 8 Kasım
  2026'da okundu): “When you move common vertices/segments, QGIS will also move them in the geometries of the
  neighboring features.” Kılavuz kenarın ortasındaki komşu köşeyi anlatmaz.
- **KentOS bugün (ADR 1960):** Paralel kaydır'ın ortak sınırı yalnız kenarın uçlarındaki köşeleri taşıyordu. Kenarın
  ortasında duran komşu köşe yerinde kalıyordu:
  - iki parselin buluştuğu üçlü birleşim (A'nın kenarına B1 ile B2'nin ortak köşesi değiyor): B1 ile B2'nin sınırı
    kırılıp kaymış kenara uzanıyordu, aralarında boşluk ya da bindirme kalıyordu;
  - komşunun kendi ara köşesi (kenar boyunca ölçülmüş bir sınır noktası, “çoklu köşe”): komşunun sınırı V biçimini
    alıyordu;
  - kenara T ile bağlanan çizginin ucu (yol, bölme çizgisi) kenardan kopuyordu; deliğin halkası da aynı.

## Karar

1. **Kenarın içindeki köşeler kenarla gider** (`topology_follow::along_edge`, çekirdekte `topology_edit::inside_segment`).
   Topolojik düzenleme açıkken düz kenar kayınca, görünen komşuların kenarın içinde (kenara 1 µm içinde, iki uçtan 1 µm'den
   uzak) duran her köşesi kenara dik, kenarla aynı uzaklıkta kayar ve kaymış kenarın üstünde kalır:
   - üçlü birleşimin köşesi, komşunun ara köşesi, deliğin köşesi, kenara bağlanan çizginin ucu, çoklu çizginin ara köşesi;
   - nokta yalnız Noktalar da açıkken;
   - kenarın uzantısındaki, kenardan 1 mm uzaktaki ve gizli katmandaki köşe kalır (yalnız görünen nesneler izler, ADR 0160).
   Köşelerin taşınması Topolojik düzenlemenin ortak kuralıyla (`topology_edit::apply`) köşelerinkiyle birlikte yazılır:
   tek adım, komşular `neighbours`'ta boylarıyla.
2. **Açıklanabilir ret** (`edge_shift_refused`, hiçbir şey yazılmaz):
   - kenar kısalırken içteki ortak köşe kaymış kenarın dışına ya da ucuna düşüyorsa: “Kenarın ortasındaki ortak köşe
     kaydırılan kenarın dışına düşüyor …” (`work.distance`); hedef alanla da;
   - yaylı kenarın ortasında komşu köşe varsa (yay kaydırılınca köşe yayın üstünde kalmaz): “Yaylı ortak kenarın ortasında
     komşu nesnenin köşesi var …” (`work.at`).
3. **Kilitli komşu:** kilitli katmandaki komşunun kenar içindeki köşesi de köşeleri gibi kalır: sınırın ayrıldığı
   söylenir (`neighbour_locked`) ya da Kilitli komşuda durdur açıkken hiçbir şey yazılmaz (`shared_boundary_locked`). Kenara
   yalnız kenar içindeki ucuyla değen kilitli çizgi artık söylenir (önce hiç sayılmıyordu).
4. **Cevap:** `ShapesEdited.carried`: kenarın içinde kilitsiz bir komşunun köşesi olan yer sayısı.
5. **Masaüstü:** önizleme komutun kuralıyla çizer (taşınan köşeler komşuların kesikli çizgisinde; ret etiketi tehlike
   renginde); yazınca “Topolojik düzenleme: 3 komşu nesne de değişti; kenarın ortasındaki 2 ortak köşe de kenarla kaydı.”
6. **Yan düzeltme:** Topolojik düzenlemenin yazdığı komşunun deliklerinde ve parçalarında da sıfır yaylar atılır (dış
   halkada atılıyordu; delik halkası `bulges: [0, 0, …]` ile yazılıyordu).
7. **Kapsam işareti:** ADR 1960'taki gibi **CAD (Harita + Yol) ve CBS'nin tipli katman düzenlemesi**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/edge_shift_junction_cases.py` → `fixtures/commands/v1/cad.shapes.edit.junction.json`,
  21 durum, KentOS kodu yok. Paralel kaydır ADR 0191'in kuralıyla, ortak köşeler ADR 1960'ınkiyle, kenar içindeki
  köşeler kenara dik aynı uzaklıkla, kesirlerle; harita koordinatında, mikrometre içinde (kısalan kenarın köşeleri tam
  metre değil; koordinatın son biti nanometreye yakın).
  - Üçlü birleşim dışarı ve içeri, hedef alanla; alanların toplamı korunur. Noktalar da ile kenardaki sınır taşı.
  - Çoklu köşe (komşunun ara köşesi ve oraya bağlanan çizgi) dışarı ve içeri; delik dışarı ve içeri.
  - İki T birleşimi ve çoklu çizginin ara köşesi birlikte; uzantıdaki, 1 mm uzaktaki ve gizli katmandaki uçlar yerinde.
  - Yolun çoklu çizgisinin ilk kenarı ve ona bağlanan kol.
  - Kilitli komşunun ara köşesi ve kenara bağlanan kilitli çizgi: ayrıldığı söylenir ya da durdurulur.
  - Kısalan kenar: içteki köşe dışarı düşer (uzaklıkla ve hedef alanla) ve ucuna düşer: ret; içeride kalır: kayar.
  - Yaylı kenarın ortasındaki komşu köşe: ret. Komşusuz kenar ve Topolojik düzenleme kapalı: yalnız kenar.
  - Rust ve Python geçer; ADR 1960'ın ortak sınır durumları ile köşe ve esnetme durumları değişmedi.
- **Çekirdek:** `topology_edit::tests`: kenar ve yay içindeki köşeler.
- **MCP** (`crates/native/mcp/tests/shared_edge_junction.rs`): çizim `.kcad` olarak açılır, üçlü birleşim taşınır
  (`carried` 1), kaydedilen dosyada köşe kaymış kenarda; kısalan kenar reddedilir.
- **Masaüstü** (`edge_shift_junction_scenes.rs`): önizleme komşuları çizer; yazınca üçlü birleşim ve kol ucu kaymış
  kenarda, ileti iki yeri söyler, tek geri alma hepsini alır. Çapraz denetimler geçer. Resimler
  `.run/shots/arac-paralel-kaydir-ucuncu-birlesim-*`.

## Sonra

- Tutamaç, Köşe tablosu ve Esnet'in köşe taşımasında da kenar içindeki komşu köşe (kenar döner; köşenin kenar üstündeki
  yeri oranla korunmalı). Yapıldı: [ADR 3400](3400-inner-vertices-follow-a-changed-edge.md).
- Yaylı ortak kenarda içteki köşenin eş merkezli taşınması (bugün ret).
- Topoloji denetiminin veri formu yolu; taşınmaların sahnede işaretlenmesi (MAP-07'nin öbür maddeleri). İşaret yapıldı:
  [ADR 3000](3000-shift-preview-marks-carried-vertices.md).
- Web (TODOS §0.2).
