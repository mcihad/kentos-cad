# 0964 — Ters çevrilen aksın dever çizelgesi

Tarih: 2026-11-05. Durum: kabul (Y58). Kapsam ROAD-DEVER-03.

## Sorun

ADR 0904 §1 işareti tek bir sözleşmeye bağlar:

- enine eğim her yanda eksenden dışa ölçülür, yüzde ile;
- dışa düşen eksidir;
- sol ve sağ aksın yönüne göredir.

Aynı madde "aks ters çevrilince sol ve sağ yer değiştirir, km'ler eşlemeyle taşınır" diyordu, ama kod bunu yapmıyordu.
`road.alignments.edit reverse` aksı çeviriyor, dever çizelgesi olduğu gibi kalıyordu. Çizelge eski revizyona bağlı
kaldığı için "eski" sayılıyordu; kullanıcı yeniden bağlarsa satırların km'leri ve yanları yanlış yere düşüyordu:

- eski km 100'deki satır yeni km 100'e;
- sol yanın değeri yeni sol yana, yani eski sağ kenara.

ROAD-DEVER-03: "Sol/sağ ve forward/reverse sign convention tek sözleşmede olsun; alignment reverse planı schedule'ı
doğru remap etsin."

## Araştırma

- **Civil 3D, "To Reverse Alignment Direction":**
  - Aks ters çevrilince ona bağlı nesneler (profiller, kesit hatları, kesitler, koridor, dever) yeniden işlenir.
  - API belgesine göre km eşitlikleri ve tasarım hızları silinir.
  - Kullanıcılar profilin kendiliğinden aynalanmadığını, yeniden kurulması gerektiğini bildiriyor.
  - Kaynaklar: Autodesk Civil 3D Kullanıcı Kılavuzu "To Reverse Alignment Direction" (2015–2024), Civil 3D API
    `Alignment.Reverse`, Autodesk forumu "Reverse Alignment Direction with Attachments".
- **KentOS'ta var olanlar:**
  - Ters çevirme km eşitliklerini kaldırıp uyarıyordu (`equations_dropped`).
  - Paralel aksın ötelemesinin işaretini çeviriyordu.
  - Klotoidlerin giriş ve çıkış boylarını değiştiriyordu.
- **Kurala dayanak:** Yeni bir hesap gerekmez. Dönüşüm tanımlardan çıkar: tersine okunan aynı fiziksel yol.

## Karar

### Dönüşüm

Ters çevirme, aksa bağlı dever çizelgelerini **aynı geri alma adımında** dönüştürür. L aksın boyudur.

| Alan | Dönüşüm | Neden |
|---|---|---|
| km'li satırlar | s → L − s, sıra ters, sol ↔ sağ, değer aynı | Yeni sol yan eski sağ kenardır. Her yan kendi dışına doğru yazıldığı için değer değişmez, yalnız sütun değişir. |
| Normal eğim | sol ↔ sağ | Aynı neden |
| Kurba bağlı kural | aynen kalır | Kural PI kimliğini izler; kurp öbür yöne döner ama dış kenarı fiziksel olarak aynı kenardır. Kuralın iki ucu (oran, geçiş, teğet payı, genişleme) eşittir. |
| Dönme ekseni | sol kenar ↔ sağ kenar; eksen, iç ve dış kenar aynı | Aynı fiziksel kenar |
| Bağlılık | aksın yeni revizyonu, çizelgenin revizyonu +1 | Çizelge eskimez |

İşaret hiçbir zaman eksi ile çarpılmaz. Bu, ADR 0904 §1'in sözleşmesinin doğrudan sonucudur.

### Yazma ve rapor

- Aks ve dever çizelgeleri tek işlemde yazılır: tek adım "Aksı düzenle"; geri almak ikisini birlikte geri getirir.
- Komutun yanıtı çevrilen çizelgelerin kimliklerini `superelevations` alanında verir.
- Güzergâh penceresi çevrilenleri adlarıyla söyler.
- Plan (`plan`) da aynı dönüşümü gösterir, hiçbir şey yazmaz.

## Ölçüm

**Kurba bağlı kuralın değişmezliği, bağımsız referansta.** `scripts/fixtures/road_alignment_command_cases.py`, dever
referansıyla (`road_superelevation_cases.py`, KentOS kodu yok) iki çizelge kurar:

- düz akstaki çizelge;
- ters PI'larla, giriş ve çıkış klotoidleri değiştirilmiş ve normal eğimleri yer değiştirmiş çizelge.

Sonra 401 km'de "eski sol(s) = yeni sağ(L − s)" ve genişlemelerin eşitliğini sınar; tutmazsa referans durur. R 400 m,
klotoidler 80/80, oran %6, geçiş 100 m, teğet payı 0,6, genişleme 0,8 m ile en büyük fark 1e-14'tür. Teğet payı 0 iken
de aynıdır.

**Rust'ta.** `crates/native/application/tests/all/superelevation_reverse.rs`, çekirdeğin kendi çizelgeleriyle
(`made_superelevations`) satırlı ve kurba bağlı iki çizelgede 1001 km'de aynı eşitliği sınar. Her kenarın eğimi ve
genişlemesi 1e-9 içinde aynıdır; başka akstaki çizelgeye dokunulmaz.

**Paylaşılan durumlar.** `road.alignments.edit`'e 2 yeni durum eklendi:

- "Ters çevrilir: dever çizelgeleri de döner": yazma ve geri alma;
- "Ters çevirmenin planı devere de bakar".

Rust'ta ve Python'da geçer.

**Pencere testi.** Satırlı çizelgenin normal eğimleri −2,5/−3'ten −3/−2,5'e, dönme ekseni sol kenardan sağ kenara
geçer. Eski km 900'deki "sol %6, sağ −%6" satırı yeni km 389,370'te "sol −%6, sağ %6" olur. Tek geri alma ikisini
birlikte geri getirir.

## Açık

- **Diğer km'ler:** Boykesitler Y59'dan beri ters çevirmede onunla döner (ADR 0965). Kesit takımı ve koridor km'leri
  dönmez; tip kesitin yanları yüzünden eskir ve söylenir.
- **Km eşitlikleri:** Ters çevirmede eskisi gibi silinir; Civil 3D de siler.
