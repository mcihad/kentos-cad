# fonts — bütün yazı tipleri

KentOS'un bütün yazı tipi dosyaları bu klasördedir. Kurulumda `<kaynak>/fonts/` olur (CLAUDE.md §26, ADR 9313).
Her yüzün lisansı yanındadır.

| Klasör | Ne |
|---|---|
| `cizim/` | Çizimin yüzleri: Barlow, Arimo, Overpass, Quicksand, Architects Daughter, Courier Prime, IBM Plex Mono ve Times New Roman'ın ikizi Liberation Serif |
| `arayuz/` | Arayüzün yüzleri: Inter, IBM Plex Sans ve Mono, Noto Sans, Roboto, Source Sans 3, Plus Jakarta Sans, JetBrains Mono (kaynakları `arayuz/README.md`'de) |
| `yedek/` | Yazı tiplerinde olmayan harfler için yedek yüzler: Noto Sans, Noto Sans Arabic, Hebrew ve Math |

Programlar bu yüzleri derlenirken içlerine alır. Sınamalar, başsız çalışma ve WASM bu yüzden klasöre bakmadan aynı
harfleri çizer.

## Listeler buradan

Her program açılırken bu klasörü ve kullanıcının `kentos-cad/fonts` klasörünü okur (alt klasörler de). Okuyanlar:
masaüstü, `kentos`, `kentos-mcp` ve Python modülü. Yazı tipi listeleri bu okumadan kurulur:

- Ayarlar'ın “Çizim yazı tipi”;
- Proje ayarları;
- yazı stili ve ölçü stili.

Listede önce KentOS'un dokuz yüzü, sonra klasörlerdeki aileler adlarına göre gelir. Yazı tipleri penceresi
(`text.fonts`) her dosyayı ve KentOS'un onunla ne yaptığını gösterir. Python ve MCP aynı listeyi `cad.fonts.report`'un
`files` ve `choices` alanlarından alır.

Kendi yazı tipinizi (`.ttf`, `.otf`) kullanıcının `kentos-cad/fonts` klasörüne koyup programı yeniden başlatın. Yeri:

- macOS'ta `~/Library/Application Support/kentos-cad/fonts`;
- Windows'ta `%APPDATA%\kentos-cad\fonts`;
- Linux'ta `$XDG_DATA_HOME/kentos-cad/fonts`.

Şu dosyalar listelenir ama kullanılmaz; nedeni pencerede yazar:

- koleksiyonlar (`.ttc`, `.otc`);
- lisansı belgeye gömülmesini yasaklayan yüzler (paftanın PDF'i kullandığı harfleri gömer);
- aynı yüzün ikinci kopyası.

## Arial ve Times New Roman

Program Arial ve Times New Roman'ı sırayla bu klasörde, kullanıcının klasöründe ve işletim sisteminin yazı tipi
klasörlerinde arar (ADR 9311). Mac'te ve Windows'ta ikisi de sistemle gelir.

Arial ve Times New Roman'ın dosyaları bu depoya ve pakete konmaz: lisansları (Monotype) dağıtmaya izin vermez. Kurum
dağıtım lisansı alırsa lisanslı dosyalar buraya ya da kullanıcının klasörüne konur. Program onları sistemdekilerden
önce okur.

İkisinin de bulunmadığı makinede (Linux sunucusu gibi) program aynı ölçülü ikizleriyle çizer: Arial'ı Arimo, Times New
Roman'ı Liberation Serif (ikisi de SIL OFL 1.1). Harflerin genişlikleri aynıdır, yazının yerleşimi değişmez.
