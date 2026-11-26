# fonts — programın okuduğu yazı tipi dosyaları

Kurulumda `<kaynak>/fonts/` olur (CLAUDE.md §26). Program açılışta Arial ve Times New Roman'ı sırayla burada,
kullanıcının `kentos-cad/fonts` klasöründe ve işletim sisteminin yazı tipi klasörlerinde arar (ADR 9311): Mac'te ve
Windows'ta ikisi de sistemle gelir.

Arial ve Times New Roman'ın dosyaları bu depoya ve pakete konmaz: lisansları (Monotype) dağıtmaya izin vermez. Kurum
Monotype'tan dağıtım lisansı alırsa lisanslı dosyalar buraya konur (`Arial.ttf`, `Arial Bold.ttf`, `Times New
Roman.ttf`, `Times New Roman Bold.ttf`, `Times New Roman Italic.ttf`, `Times New Roman Bold Italic.ttf` ya da
Windows adlarıyla `arial.ttf`, `arialbd.ttf`, `times.ttf`, `timesbd.ttf`, `timesi.ttf`, `timesbi.ttf`); program onları
sistemdekilerden önce okur.

İkisinin de bulunmadığı makinede (Linux sunucusu gibi) program aynı ölçülü ikizleriyle çizer: Arial'ı Arimo, Times New
Roman'ı Liberation Serif (ikisi de SIL OFL 1.1, programın içinde gömülü). Harflerin genişlikleri aynıdır, yazının
yerleşimi değişmez.
