# 5000 — Köşe sil'de aynı dairenin iki yayı tek yay olur

Tarih: 2026-11-11. plan/map-cad.md MAP-07, onuncu tur. Durum: taslak. ADR 4200'ün “Sonra”sındaki ve MAP-07'nin
“Sıradaki”ndeki “yaylı kenarların birleşmesi”.

İlgili:
- [ADR 4200](4200-vertex-delete-carries-inner-vertices.md): Köşe sil'de birleşen kenarın içindeki komşu köşeler;
- [ADR 0279](0279-grip-menus-and-the-vertex-command.md): `cad.entities.vertex`;
- [ADR 0160](0160-topological-editing.md): Topolojik düzenleme.

## Dayanak

- **ArcMap, Merge lines and create line points**
  ([belge](https://desktop.arcgis.com/en/arcmap/latest/manage-data/editing-parcels/mergingparcellines.htm),
  11 Kasım 2026'da okundu; resimler `.run/research/map-cad/uygulama/MAP-07.10-resim/`). Parsel kumaşında Merge Courses
  penceresi:
  - düz çizgiler için açı kırığı toleransı;
  - yaylar için iki ölçütten biri: yarıçap özniteliği farkı yüzdesi ya da merkez noktalarının uzaklığı;
  - pencerenin resmindeki değerler 2 % ve 3.2808 ft (1 m). Belge bunları öntanımlı diye yazmıyor; KentOS değer almaz.
  - birleşen çizgide komşunun ucu “line point” olarak kalır, ortak sınır bozulmaz.
- **ArcGIS Pro, Simplify By Straight Lines And Circular Arcs**
  ([belge](https://pro.arcgis.com/en/pro-app/tool-reference/editing/simplifybystraightlinesandcirculararcs.htm),
  10 Kasım 2026'da okundu): var olan yayları birleştirmez.
- **Mevzuat** (`.run/research/map-cad/mevzuat/MAP-07.10.md`): yayların birleştirilmesine hüküm yok. UVDF §3.2.7.1.7'de
  yayın dönüş yönü (CCW/CW) anlamlıdır; birleşme yönü korumalı, ters yönlü yaylar birleşmemeli.
- **KentOS bugün:** Köşeyi sil iki kenarı her zaman düz bir kenar yapıyordu. Aynı dairenin iki yayı arasındaki köşe
  silinince yay kaybolup kiriş oluyordu (ADR 4200 §Sınırlar).

## Karar

1. **Komut:** `cad.entities.vertex`'e `joinArcs` eklenir, yalnız Köşeyi sil'de. Değeri çizimin uzunluğunda bir
   toleranstır: sıfır ya da büyük, sonlu.
   - Silinen köşenin iki kenarı da yay olmalı. İki yay aynı yöne dönmeli (büküm işaretleri aynı).
   - Merkezleri arasındaki uzaklık ve yarıçap farkı toleransı geçmemeli.
   - Bu koşullarda iki yay tek yay olur: iki ucu korunur, süpürme açıları toplanır. Büküm tan((θ₁ + θ₂)/4)'tür, yani
     tanjant toplamıyla (b₁ + b₂)/(1 − b₁b₂).
   - Birlikte tam daire ya da fazlası ediyorlarsa (b₁b₂ ≥ 1) birleşmezler.
   - Birleşmeyen iki yayda kenar eskisi gibi düz olur ve `arcs_not_joined` uyarısı nedeni söyler: merkezler ve
     yarıçaplar ne kadar ayrık, ters yön ya da tam daire.
   - `joinArcs` verilmezse eskisi gibi kenar düz olur. Tolerans büyütülmez, gizli birleştirme yok (CLAUDE.md §23.3).
2. **Çekirdek** (`ops::vertex::arc_join`, `remove_vertex_joining`): karar çekirdekte. `remove_vertex` aynı kalır;
   web'in `removeVertex` işlemi değişmez.
3. **Topolojik düzenleme** (`topology_edit::Change::Join`): birleşen kenar yay olursa değişiklik `join` olur.
   - Silinen köşeyi aynı iki komşu köşe arasında taşıyan komşuda köşe gider. Komşunun o iki kenarı da yaysa, komşu aynı
     yayı alır; ters yönde gidiyorsa işareti ters.
   - Düz kenarlı komşuda kenar eskisi gibi düz.
   - Böylece aynı yayları paylaşan iki parselin ortak sınırı bozulmaz.
   - ADR 4200'ün içteki komşu köşe kuralı yaylı kenara uygulanmaz. Aynı dairede kalan yayın üzerindeki komşu köşe yerinde
     kalır; tolerans içinde de yayın üstündedir.
4. **Masaüstü:** tutamaç menüsünün Köşeyi sil'i ve Köşe ekle/sil aracının silmesi `joinArcs`'ı Topolojik temizlik'in
   toleransıyla verir: kullanıcının yazdığı ya da ilk değer 1 cm. Yeni ayar ve yeni pencere yok.
5. **Geri alma:** tek adım, “Köşe sil”. `.kcad` şeması değişmez.
6. **Kapsam:** CAD (Harita + Yol) ve CBS'nin tipli katman düzenlemesi, ADR 4200'deki gibi.

## Sınırlar

- Köşe tablosundaki Kaldır (birden çok köşe birlikte, `vertex_table::removed`) yayları birleştirmez.
- Yay ile düz kenar birleşmez (yayın parçası düz kenar sayılmaz).
- Yüzde ile yarıçap ölçütü yok; tek uzunluk toleransı.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/vertex_command_cases.py`; KentOS kodu yok), `cad.entities.vertex.json`
  8 yeni durumla 93 durum:
  - Yaylar R = 7225'lik dairede, köşeler (1 + i·b)⁴ dönmeleriyle tam sayı.
  - Merkez, yayın uçları ve ortasından geçen çemberin merkezi olarak kesirle bulunur.
  - Birleşen büküm (1 + i·b₁)(1 + i·b₂)'nin Im/Re'si (dönmelerin bileşkesi). Yeni yayın merkezi eski merkezle aynı
    olmalı (betikte assert).
  - Durumlar: aynı daire (1/2 ve 1/4 → 6/7, geri alma); tolerans verilmezse düz; merkezleri ayrık (uyarıda 2479.1667
    ve 2054.1667); ters yön; tam daireyi aşan; Topoloji ile komşu (−6/7); iki ret (başka eylemde, eksi tolerans).
- **Çekirdek** (`ops::vertex::tests::two_arcs_of_one_circle_join_into_one`; `topology_edit`'in testleri geçer).
- **Python** (`python/tests/test_vertex_join_arcs.py`), testin kendi hesabıyla: büküm tan((4·atan ½ + 4·atan ¼)/4), yayın
  merkezi kiriş ve sehimden (20000, 0); komşu eksi; toleranssız düz; ayrık daireler düz.
- **MCP** (`crates/native/mcp/tests/vertex_join_arcs.rs`): büküm, komşunun eksi bükümü, `arcs_not_joined`.
- **Masaüstü** (`arc_join_scenes.rs`): A yarım daire, kenarı (20, 25)'te birleşen iki çeyrek yay; B üstte aynı yaylarla.
  - Tutamaçta sağ tık “Köşe 4 / Köşeyi sil”: A'nın bükümü 1, B'ninki −1; ileti doğru; tek geri alma ikisini de geri koyar.
  - Çapraz denetimler (module_tests, ribbon_tests, history::text, catalog, settings, topology_inner_scenes,
    drawing_menus) geçer: 152 test.
  - Resimler `.run/gui-evidence/yay-birlestir-5000/`.
- **Bozma denetimi:** durum dosyasında birleşen bükümün son basamağı ve uyarıdaki merkez uzaklığı değiştirildi. Koşucu
  tam bu iki durumu yakaladı, dosya geri kondu. Esnet, Paralel kaydır ve topoloji komutlarının durumları değişmedi.

## Sonra

- Köşe tablosunda Kaldır'da yayların birleşmesi.
- Köşenin gidişine ok; tutamaçla birden çok nesne; büyük veri ölçümü (MAP-07'nin kalanı).
- Web (TODOS §0.2): `joinArcs` ve `Change::Join` WASM'a.
