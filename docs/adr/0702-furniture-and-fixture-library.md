# ADR 0702: Gerçek boyutlu mobilya, vitrifiye ve mutfak blokları

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-LIB-01'in ilk dilimi (kitaplığın içeriği).
- **Bağlam:** `plan/architecture.md` ARCH-LIB-01; ADR 0318 (dinamik bloklar ve KentOS blok kitaplığı), ADR 0315 (bloğun birimi),
  ADR 0317 (blok kartı).
- **Web:** ertelendi (ADR 0228); kitaplık dosyası ortak, web'in Bloklar paneli TODOS.md §0.2'de.

## Bağlam

KentOS blok kitaplığı (`catalog/v1/blocks/kentos-temel.kblok`) 16 blokla başlamıştı: iki kapı, pencere, beş mobilya, dört ıslak
hacim, üç işaret ve antet. Bir daire planı için gereken tek kişilik yatak, koltuk, yuvarlak masa, depolama, asma klozet, köşe
duş, beyaz eşya ve mutfak dolapları yoktu; vitrifiyelerin önündeki kullanım boşluğu da gösterilemiyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [AutoCAD 2026 — To Set Up a New Visibility State in a Dynamic Block](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-19EB1833-9F3D-4F6D-8FE7-F88CC10E377B.htm):
  bir bloğun nesneleri adlı görünürlük durumlarıyla gösterilir/gizlenir. KentOS'ta aynı kavram ADR 0318'in `visibility`
  parametresidir; boşluk zarfı bu yolla açılıp kapanır.
- AutoCAD'in arama tablosu (lookup) ve Revit'in aile tipleri boyut varyantlarını tek blokta tutar; KentOS'ta ADR 0318'in
  `lookup` parametresi aynı işi görür.

KentOS kararı: bloklar KentOS'un kendi ölçülerinden çizilir (başkasının hakkı yok), metre birimindedir (ADR 0315: mm projede
1000 kat büyür), gerçek boyutludur; boşluk zarfı mevzuat denetimi değil, çizim yardımıdır ve varsayılan olarak gizlidir.

## Karar

Kitaplığa 21 blok eklenir (kimlikler 17–37; ilk 16 blok bayt bayt aynıdır, eski projeler etkilenmez):

| Kategori | Bloklar | Dinamik |
|---|---|---|
| Mobilya/Yatak | Tek kişilik yatak 90 × 200, Komodin 50 × 40 | Yatak: En 80–140 cm, Boyut tablosu 90/100/120 |
| Mobilya/Oturma | Koltuk 85 × 85, İkili kanepe 150 × 85 | — |
| Mobilya/Masalar | Yuvarlak masa Ø100 ve dört sandalye, Çalışma masası 120 × 60 | Masa: Sandalyeli / Yalnız masa; çalışma masası 100–180 cm |
| Mobilya/Depolama | TV ünitesi 180 × 45, Kitaplık 90 × 35 | Uzunluk / genişlik |
| Islak hacim | Asma klozet, Pisuvar, Köşe duş 90 × 90, Dolaplı lavabo 80 × 46, Çamaşır ve kurutma makinesi 60 × 60 | Vitrifiyelerde Görünüş: Sade / Boşluk zarfı |
| Mutfak | Evye 80 × 50, Ocak 60 × 52, Buzdolabı 60 × 65, Bulaşık makinesi, Fırın, Alt dolap, Üst dolap | Evye: Süzgeç sağda (çevir); dolaplar 30–120 cm |

- **Taban noktası:** duvara dayanan bloklarda arka kenarın ortası ya da sol köşesi duvarda, önü +y yönünde (her bloğun
  açıklamasında yazılıdır); masa ve yuvarlak masa merkezde.
- **Boşluk zarfı:** kesikli dikdörtgen (nesnenin kendi çizgi tipi `dashed`), `Görünüş` parametresinin varsayılan `Sade`
  durumunda gizli. Ölçüleri (klozet önünde 80 × 60 cm, lavabo önünde 80 × 70 cm, duş girişinde 80 × 80 cm) yaygın plan
  alışkanlığıdır; KentOS bunun bir yönetmeliğe uyduğunu söylemez.
- **Üst dolap** planda kesikli çizilir (kesit düzleminin üstünde).
- Kitaplık `scripts/blocks/library.py`'den üretilir; mimari şablonu (`catalog/v1/templates/mimari.json`) kitaplığı aldığı için
  yeni bloklar şablonla da gelir (`cad.templates.apply` durumları yenilendi).
- **Duvara hizala (H):** Blok ekle'nin görünür seçeneği (oturum boyunca hatırlanır). Açıkken imlecin 40 piksel yakınındaki
  duvar yüzü bulunur; bloğun taban noktası yüzün üstüne, +y yönü odanın içine (yüzün imleç tarafına) bakacak biçimde
  döner. Önizleme ve yazma aynı noktayı ve dönüşü kullanır; yazma mevcut `cad.blocks.insert` komutudur (yeni komut yok),
  Python/MCP aynı girdiyi `at` ve `rotation` ile verir. Yakında duvar yoksa blok imleçte, verilen dönüşle durur.

## Kapsam dışı (sonraki dilimler)

DWG'den gelen blokların kategoriye alınması ve önizlemede fiziksel en/boy etiketi ARCH-LIB-01'in açık maddeleridir.

## Doğrulama

- `python3 scripts/blocks/library.py --check`; bloklar `cad.blocks.edit` `import name: kentos-temel` ile alınır ve her
  dinamik tanım ADR 0318'in kurallarından geçer (başsız sunucu testi).
- Masaüstü: Bloklar panelinin kartlarında yeni kategoriler, bir daire planına yerleştirme, boşluk zarfının açılması ve
  Duvara hizala ile yatağın yatak odası duvarına oturması (resimler).
