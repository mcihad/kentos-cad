# ADR 8049: Ölçülü bölmede kalıcı bağ ve bağların kapsamı

- **Durum:** 1. bölüm yapıldı (11 Kasım 2026); `plan/architecture-advanced.md` ADV-01, M49. Ölçülü bölme mesafesini
  "Bu ölçüyü koru" ile kalıcı bağ olarak yazar (§1). Açık: bağların yay kenar, kolon, aks ve döşeme kapsamı (ADR 0770'in
  sınırları; §2).
- **Bağlam belgesi:** ADR 0770 (bağlantılı odalar ve kalıcı mimari bağlar), ADR 0657 ve 0734 (ölçülü bölme, yay kenar),
  ADR 0710 (duvarın mesafesi, Kaydır), ADR 0780 ve 0790 (yoldaki bağlar), ADR 0334–0338 (kısıtlar).

## Bağlam

Plan ADV-01'in 7 Kasım notu: "ölçülü bölme oluştururken kalıcı bağ yazmıyor (`walls_partition.rs`; ARCH-DRAW-02'nin
`[ ]`'i) ve bağların yay kenar, kolon, aks ve döşeme kapsamı (ADR 0770 sınırları) → M49". İlk kabulü: "1,20 m koridoru
koruyan bağı etkinleştirip dış duvarı değiştir … alanlar, kapılar, etiketler ve ölçüler izlesin."

ADR 0770 "Bu ölçüyü koru"yu Duvarın mesafesi'ne (`cad.walls.edit` Mesafe'nin `keep`'i) koydu; ama aynı ölçüyü ilk
çizerken veren araç, ölçülü bölme, ölçüyü yalnız bir kez uyguluyordu: bölme yerleşiyor, kaynak duvar sonra kayınca
bölme yerinde kalıyordu. Kullanıcı ölçüyü iki kez vermek zorundaydı (önce bölme, sonra Mesafe ile koru).

Başka programlar: Revit'te ölçü konulup kilitlenir ("lock it in place"), Chief Architect'te ölçü uçlarından hangi duvarın
kayacağı seçilir, Archicad'in mesafe kılavuzları kalıcı ilişki tutmaz (ADR 0770'in araştırması). Mevzuat açısından
korunan mesafe PAİY 29'un en küçük net ölçülerini (Mimari denetim › Mevzuat, ADR 8033) dış duvar değişse de tutmanın
yoludur.

## Karar

### 1. Ölçülü bölmede "Bu ölçüyü koru"

- `cad.walls.partition` v1'in yeni `keep` alanı (öntanımlı `false`; eski çağrılar bayt bayt aynı). `true` iken bölme
  yazıldıktan sonra **aynı adımda** bir mesafe bağı ve parametresi yazılır (ADR 0770 §4, `constraints_edit::add_bond_given`):
  - **Net yüzler arası** (`clear`): kaynağın bölme tarafındaki yüzü (`side` sol ise sol yüz) ile bölmenin kaynağa bakan
    yüzü (yolunun kaynak yüzün bulunduğu yanındaki yüz);
  - **Eksenler arası** (`axis`): kaynağın ekseni ile bölmenin ekseni;
  - değer verilen `distance` (parametre `d1`, `d2` …; ölçülen değer değil, verilen), bağın yönü çizimden okunur.
- Bağ ADR 0770'in çözücüsüne girer: kaynak duvar Kaydır, Mesafe, Boy ya da tutamaçla kayınca bölme aynı mesafede izler;
  bölmenin kendisi kaydırılırsa kaynak izler (en az değişiklik). Tek geri alma adımı "Duvar" bölmeyi ve bağını birlikte
  geri alır.
- **Yay kenar** kaynağıyla `keep` reddedilir (`invalid_constraint`, `keep`): mimari bağ düz duvar kenarları arasında
  kurulur; yay kenarın bağı §2'de.
- `WallsCreated`'ın yeni `bonds` alanı: yazılan bağın kimliği (yalnız bölmede; boşken yazılmaz).
- **Araç:** Mimari › Duvar ▾ › Ölçülü bölme'de "Bu ölçüyü koru" (K) aç/kapa satırı Araç ayarlarında ve komut satırında;
  K tek basışta değiştirir.

### 2. Bağların kapsamı (açık)

ADR 0770 §4 mimari bağları düz duvar kenarlarının çizgileri ve açıklık pervazlarıyla sınırlıyor. Kalanlar: yay kenar
(eş merkezli yayların radyal mesafesi), kolon (kolon yüzü ya da merkezi ile duvar çizgisi), aks (aks çizgisiyle duvar
çizgisi; Mesafe'nin aksa ölçüsünün korunması) ve döşeme (döşeme kenarıyla duvar yüzü).

## Doğrulama

- Uygulama: 8 × 6 m odada 3 m net bölme `keep` ile: bir bağ, parametre `3`, bölmenin ekseni y = 3,075; tek geri alma
  ikisini alır; güney yüz Kaydır ile 0,5 m dışa: bölmenin ekseni 2,575 (3 m korunur).
- 92 ortak durum (`walls_partition_cases.py`): bağ ve `d1 = 3` aynı adımda; yay kaynaktan `keep` reddedilir, hiçbir şey
  yazılmaz; masaüstü işleyicisinde ve Python'da.
- Araç: K satırı Kapalı → Açık, komut satırında "Bu ölçüyü koru", yerleşince bağ ve parametre, geri alma ikisini alır.
