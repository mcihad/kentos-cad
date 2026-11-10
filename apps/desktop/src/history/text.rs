//! What the command history writes of a call (docs/adr/0284): the command's
//! Turkish title, its input in plain words, what it wrote, its warnings and
//! its error with their codes. Pure: the layers' names come from the caller.

use kentos_commands::{CallKind, CallRecord, CallStatus, CommandWarning, ProjectPart};
use kentos_domain::{RefusedAs, RefusedCall};
use serde_json::Value;

/// The longest input text before it is cut with “…”.
const INPUT_WIDTH: usize = 240;

/// The product commands' Turkish titles, the catalog's (`commandCatalog.json`;
/// a test holds them to it).
pub(crate) const TITLES: [(&str, &str); 443] = [
    ("cad.polygon.create", "Kapalı alan oluştur"),
    ("cad.line.create", "Çizgi oluştur"),
    ("cad.polyline.create", "Çoklu çizgi oluştur"),
    ("cad.entities.delete", "Nesneleri sil"),
    ("cad.point.create", "Nokta oluştur"),
    ("cad.circle.create", "Daire oluştur"),
    ("cad.arc.create", "Yay oluştur"),
    ("cad.entities.transform", "Nesneleri dönüştür"),
    ("cad.entities.edit", "Nesneleri düzenle"),
    ("cad.entities.array", "Nesneleri diziye kopyala"),
    ("cad.drawing.inspect", "CAD çizim denetimi"),
    ("cad.arrays.find", "Blok dizi örüntülerini bul"),
    ("cad.arrays.adopt", "Mevcut blokları diziye bağla"),
    ("cad.arrays.edit", "Diziyi düzenle"),
    ("cad.constraints.edit", "Kısıtlar"),
    ("cad.parameters.edit", "Parametreler"),
    ("cad.constraints.query", "Kısıtları oku"),
    ("cad.entities.explode", "Nesneleri patlat"),
    ("cad.curves.edit", "Eğriyi düzenle"),
    ("cad.entities.match", "Özellik kopyala"),
    ("cad.entities.cleanup", "Çizimi temizle"),
    ("cad.entities.copy", "Panoya kopyala"),
    ("cad.entities.paste", "Yapıştır"),
    ("cad.drawing.audit", "Denetle"),
    ("cad.drawing.compare", "Revizyonları karşılaştır"),
    ("cad.drawing.repair", "Çizimi onar"),
    ("cad.drawing.take", "Seçilen farkları al"),
    ("cad.review.notes.edit", "İnceleme notunu düzenle"),
    ("cad.review.notes.list", "İnceleme notlarını listele"),
    ("cad.entities.offset", "Nesneleri ötele"),
    ("cad.entities.flatten", "2B düzleme indir"),
    ("cad.entities.order", "Çizim sırasını değiştir"),
    ("cad.annotations.scales", "Açıklamanın ölçekleri"),
    ("cad.nurbs.create", "NURBS eğrisi oluştur"),
    ("cad.walls.create", "Duvar oluştur"),
    ("cad.walls.rectangle", "Dikdörtgen duvar"),
    ("cad.walls.partition", "Ölçülü bölme duvar"),
    ("cad.walls.convert", "Çizgiden duvar"),
    ("cad.walls.edit", "Duvarı düzenle"),
    ("cad.walls.joins", "Duvar birleşimleri"),
    ("cad.walls.distances", "Duvarın mesafeleri"),
    ("cad.columns.distances", "Kolonun mesafeleri"),
    ("cad.spaces.number", "Mahalleri numarala"),
    ("cad.spaces.separators", "Mahal ayırıcı"),
    ("cad.spaces.discover", "Toplu mahal bul"),
    ("cad.spaces.create", "Mahal oluştur"),
    ("cad.spaces.edit", "Mahali düzenle"),
    ("cad.slabs.create", "Döşeme oluştur"),
    ("cad.slabs.edit", "Döşemeyi düzenle"),
    ("cad.spaces.quantities", "Metraj"),
    ("cad.grids.create", "Aks"),
    ("cad.columns.create", "Kolon"),
    ("cad.stairs.create", "Merdiven"),
    ("cad.ramps.create", "Rampa"),
    ("cad.beams.create", "Kiriş"),
    ("cad.arch.edit", "Mimari nesneyi düzenle"),
    ("cad.tags.create", "Etiket"),
    ("cad.tags.edit", "Etiketi düzenle"),
    ("cad.tags.check", "Etiketleri denetle"),
    ("cad.viewmarks.create", "Görünüm işareti"),
    ("cad.viewmarks.edit", "Görünüm işaretini düzenle"),
    ("cad.viewmarks.references", "Başvurular"),
    ("cad.spaces.finish", "Döşeme ve tavan"),
    ("cad.templates.apply", "Mimari şablonu"),
    ("cad.profiles.apply", "Veri profilini uygula"),
    ("cad.profiles.check", "Veri profilini denetle"),
    ("cad.profiles.import", "Veri profili içe aktar"),
    ("cad.profiles.list", "Veri profillerini listele"),
    ("cad.profiles.migrate", "Veri profili göçü"),
    ("cad.profiles.remove", "Veri profilini çizimden kaldır"),
    ("cad.sheets.set", "Pafta seti"),
    ("cad.navigator.read", "Proje gezgini"),
    ("cad.xrefs.edit", "Dış başvuru"),
    ("cad.xrefs.list", "Dış başvurular"),
    ("cad.rasters.georef", "Raster oturt"),
    ("cad.rasters.mosaic", "Sanal mozaik"),
    ("cad.rasters.reproject", "Rasteri yeniden izdüşür"),
    ("cad.rasters.gpkgread", "GeoPackage karolarını oku"),
    ("cad.rasters.gpkgwrite", "Rasteri GeoPackage karosu olarak yaz"),
    ("cad.time.query", "Zamanda sorgu"),
    ("cad.sensors.ingest", "Sensör gözlemlerini al"),
    ("cad.sensors.latest", "Sensörlerin son gözlemi"),
    ("cad.pointclouds.describe", "Nokta bulutunun bilgisi"),
    ("cad.pointclouds.query", "XYZ sor"),
    ("cad.pointclouds.region", "Nokta bulutu alan istatistiği"),
    ("cad.pointclouds.profile", "Nokta bulutu profili"),
    ("cad.meshes.describe", "Meshin bilgisi"),
    ("cad.meshes.query", "Mesh sor"),
    ("cad.meshes.region", "Mesh alan istatistiği"),
    ("cad.meshes.profile", "Mesh profili"),
    ("cad.stac.search", "STAC'ta ara"),
    ("cad.stac.prepare", "STAC verisini hazırla"),
    ("cad.images.edit", "Resmin kaynağı"),
    ("cad.inserts.clip", "Kırp"),
    ("cad.assets.check", "Bağlı dosyalar"),
    ("cad.schedules.create", "Çizelge"),
    ("cad.extract.run", "Veri çıkarımı"),
    ("cad.extract.table", "Veri çıkarımı tablosu"),
    ("cad.schedules.edit", "Çizelgeyi düzenle"),
    ("cad.schedules.query", "Çizelgeyi oku"),
    ("cad.openings.create", "Açıklık ekle"),
    ("cad.openings.edit", "Açıklığı düzenle"),
    ("cad.openingtypes.edit", "Kapı ve pencere tipleri"),
    ("cad.materials.edit", "Malzemeler"),
    ("cad.modules.edit", "Oda modülü"),
    ("cad.options.edit", "Tasarım seçeneği"),
    ("cad.options.query", "Tasarım seçeneklerini oku"),
    ("cad.elevations.edit", "Oda iç görünüşü"),
    ("cad.elevations.query", "İç görünüşleri oku"),
    ("cad.modules.query", "Oda modüllerini oku"),
    ("cad.openings.query", "Açıklıkları oku"),
    ("cad.openings.distances", "Açıklığın mesafeleri"),
    ("cad.arch.audit", "Mimari denetim"),
    ("cad.arch.exceptions", "Denetim istisnası"),
    ("cad.program.edit", "Bina programı"),
    ("cad.program.check", "Bina programını denetle"),
    ("cad.levels.assign", "Kat ver"),
    ("cad.north.create", "Kuzey oku"),
    ("cad.revisions.mark", "Revizyon işaretle"),
    ("cad.legends.materials", "Malzeme lejandı"),
    ("cad.casework.run", "Mutfak dizisi"),
    ("cad.casework.edit", "Mutfak dizisini düzenle"),
    ("cad.casework.query", "Mutfak dizilerini sorgula"),
    ("cad.dimensions.reassociate", "Ölçüyü yeniden bağla"),
    ("cad.dimensions.create", "Ölçü ekle"),
    ("cad.dimensions.interior", "İç ölçü zinciri"),
    ("cad.dimensions.edit", "Ölçüyü düzenle"),
    ("cad.dimensions.query", "Ölçüleri oku"),
    ("cad.fonts.report", "Yazı tiplerini raporla"),
    ("cad.hatches.edit", "Taramaları düzenle"),
    ("cad.leaders.edit", "Kılavuzları düzenle"),
    ("cad.tables.edit", "Tabloları düzenle"),
    ("cad.fields.edit", "Yazının alanlarını düzenle"),
    ("cad.texts.edit", "Yazıları düzenle"),
    ("cad.texts.check", "Yazıları denetle"),
    ("cad.entities.trim", "Nesneleri buda"),
    ("cad.entities.extend", "Nesneleri uzat"),
    ("cad.entities.fillet", "Köşe yuvarla"),
    ("cad.entities.chamfer", "Pah kır"),
    ("cad.entities.stretch", "Nesneleri esnet"),
    ("cad.entities.break", "Nesneyi kır"),
    ("cad.entities.join", "Nesneleri birleştir"),
    ("cad.entities.reverse", "Yönü çevir"),
    ("cad.entities.lengthen", "Uzat-kısalt"),
    ("cad.entities.divide", "Nesneyi böl"),
    ("cad.entities.boundary", "İçine tıklayarak alan"),
    ("cad.entities.hatch", "Tara"),
    ("cad.entities.create", "Nesneleri ekle"),
    ("cad.entities.set", "Nesnelerin özelliklerini değiştir"),
    ("cad.blocks.find", "Tekrarlanan bileşenleri bul"),
    ("cad.blocks.convert", "Tekrarları bloğa dönüştür"),
    ("cad.blocks.define", "Blok tanımla"),
    ("cad.blocks.edit", "Blok tanımını değiştir"),
    ("file.import.block", "Dosyadan blok al"),
    ("cad.blocks.replace", "Bloğu başkasıyla değiştir"),
    ("cad.blocks.query", "Blokları say, bul ve çizelgele"),
    ("cad.blocks.proxy", "Bloğun vekil görünüşü"),
    ("cad.blocks.freeze", "Sabit bloğa çevir"),
    ("cad.blocks.parameters", "Blok parametreleri"),
    ("cad.blocks.insert", "Blok ekle"),
    ("cad.layers.service", "Servis katmanı"),
    ("cad.network.define", "Ağ tanımı"),
    ("cad.subnetworks.find", "Alt ağları bul"),
    ("cad.subnetworks.update", "Alt ağları güncelle"),
    ("cad.linearref.define", "Olay bağı"),
    ("road.terrains.create", "Arazi yüzeyi oluştur"),
    ("road.terrains.edit", "Arazi yüzeyini düzenle"),
    ("road.terrains.query", "Arazi yüzeyini sorgula"),
    ("road.terrains.contours", "Eş yükselti eğrileri"),
    ("road.terrains.zeroline", "Sıfır hattı"),
    ("file.import.landxml", "LandXML al"),
    ("road.alignments.create", "Aks oluştur"),
    ("road.alignments.edit", "Aksı düzenle"),
    ("road.alignments.label", "Aks km yazısı"),
    ("road.profiles.create", "Boykesit oluştur"),
    ("road.profiles.edit", "Boykesiti düzenle"),
    ("road.alignments.check", "Aksı denetle"),
    ("road.profiles.check", "Boykesiti denetle"),
    ("road.profiles.lengths", "Düşey kurp boyu öner"),
    ("road.profiles.sight", "Görüş mesafesi"),
    ("road.profiles.propose", "Kırmızı kot önerisi"),
    ("road.profiles.draw", "Boykesit çiz"),
    ("road.profiles.query", "Boykesiti sorgula"),
    ("road.templates.edit", "Tip kesiti düzenle"),
    ("road.sections.edit", "Kesit takımını düzenle"),
    ("road.sections.query", "Enkesitleri sorgula"),
    ("road.sections.draw", "Enkesit çiz"),
    ("road.superelevations.edit", "Deveri düzenle"),
    ("road.superelevations.query", "Deveri sorgula"),
    ("road.superelevations.check", "Deveri denetle"),
    ("road.superelevations.widen", "Genişletme öner"),
    ("road.superelevations.transitions", "Geçiş öner"),
    ("road.corridors.edit", "Koridoru düzenle"),
    ("road.corridors.water", "Platform suyu"),
    ("road.corridors.daylights", "Şev uçları"),
    ("road.corridors.preflight", "Koridor ön denetimi"),
    ("road.corridors.protected", "Korunan alanlar"),
    ("road.corridors.draw", "Şevli plan çiz"),
    ("road.corridors.query", "Koridoru sorgula"),
    ("road.volumes.edit", "Kübaj hesabını düzenle"),
    ("road.volumes.query", "Kübajı sorgula"),
    ("road.volumes.approve", "Kübajı onayla"),
    ("road.volumes.uncertainty", "Kübajın belirsizliği"),
    ("road.volumes.progress", "Dönem metrajı"),
    ("road.volumes.compare", "Alternatifleri karşılaştır"),
    ("road.alignments.impact", "Aks değişikliğinin etkisi"),
    ("road.profiles.impact", "Boykesit değişikliğinin etkisi"),
    ("road.volumes.states", "Hacim halleri"),
    ("road.volumes.ledger", "Kübaj defteri"),
    ("road.volumes.history", "Onay geçmişi"),
    ("road.hauls.edit", "Brükneri düzenle"),
    ("road.hauls.plan", "Taşıma planı"),
    ("road.hauls.approve", "Taşıma planını onayla"),
    ("road.hauls.network", "Taşıma ağı"),
    ("road.hauls.trucks", "Taşıma filosu"),
    ("road.hauls.query", "Brükneri sorgula"),
    ("road.guardrails.edit", "Otokorkuluğu düzenle"),
    ("road.guardrails.query", "Otokorkuluğu sorgula"),
    ("road.guardrails.draw", "Otokorkuluk çiz"),
    ("road.culverts.edit", "Menfezi düzenle"),
    ("road.culverts.query", "Menfezi sorgula"),
    ("road.culverts.draw", "Menfez çiz"),
    ("road.culverts.hydraulics", "Menfez hidroliği"),
    ("road.culverts.closure", "Taşkında yol kapanması"),
    ("road.drainage.query", "Drenajı sorgula"),
    ("road.drainage.draw", "Drenajı çiz"),
    ("road.swept.query", "Araç izini sorgula"),
    ("road.swept.draw", "Araç izini çiz"),
    ("road.terrains.analysis", "Yüzeyi analiz et"),
    ("road.terrains.compare", "Yüzeyleri karşılaştır"),
    ("road.terrains.quality", "Ölçme kalitesini denetle"),
    ("road.terrains.repair", "Yüzeyi onar"),
    ("road.terrains.coverage", "Yüzeyin kapsamı"),
    ("road.terrains.lowconfidence", "Düşük güvenli alanları çiz"),
    ("road.terrains.density", "Nokta yoğunluğu"),
    ("road.terrains.sparse", "Seyrek alanları çiz"),
    ("road.terrains.cloud", "Buluttan arazi"),
    ("road.platforms.edit", "Platformu düzenle"),
    ("road.platforms.query", "Platformu sorgula"),
    ("road.platforms.draw", "Platform çiz"),
    ("road.walls.edit", "İstinat duvarını düzenle"),
    ("road.walls.query", "İstinat duvarını sorgula"),
    ("road.walls.draw", "İstinat duvarı çiz"),
    ("road.pay.edit", "Poz defterini düzenle"),
    ("road.pay.query", "Poz metrajını sorgula"),
    ("road.routes.find", "Ön güzergâh bul"),
    ("road.routes.draw", "Ön güzergâh çiz"),
    ("road.junctions.edit", "Kavşağı düzenle"),
    ("road.junctions.query", "Kavşağı sorgula"),
    ("road.junctions.draw", "Kavşak çiz"),
    ("road.junctions.clearance", "Kavşağın düşey açıklığı"),
    ("road.junctions.tie", "Kavşağa bağla"),
    ("road.junctions.overlap", "Kavşakta koridor örtüşmesi"),
    ("road.project.query", "Yol projesini sorgula"),
    ("road.project.rebuild", "Yol çizimlerini yeniden üret"),
    ("road.project.edit", "Kendiliğinden yeniden üret"),
    ("cad.entities.grade", "Eğimle kot ver"),
    ("cad.pointclouds.overlap", "Tarama uyumu denetimi"),
    ("cad.orthophoto.check", "Ortofoto doğruluk denetimi"),
    ("cad.orthophoto.compare", "Ortofoto karşılaştırması"),
    ("cad.readjustment.distribute", "Düzenleme dağıtım taslağı"),
    ("cad.scenarios.impact", "Alternatif etkisi"),
    ("cad.zoning.check", "İmar planı denetimi"),
    ("cad.asbuilt.check", "Uygulama kontrolü"),
    ("cad.pipes.check", "Boru hattı denetimi"),
    ("cad.parcels.check", "Parsel alan denetimi"),
    ("cad.topology.clean", "Topolojik temizlik (taşınma raporu)"),
    ("cad.survey.baselines", "GNSS baz ağı dengelemesi"),
    ("map.features.code", "Kodla çiz"),
    ("cad.crs.factors", "Ölçek çarpanları"),
    ("map.work.query", "Harita işini sorgula"),
    ("map.work.edit", "Harita işini düzenle"),
    ("map.survey.query", "GNSS arşivini sorgula"),
    ("map.survey.edit", "GNSS arşivini düzenle"),
    ("map.features.reconcile", "Adayları uzlaştır"),
    ("map.survey.network", "GNSS ağını dengele"),
    ("file.export.landxml", "LandXML dışa aktar"),
    ("file.export.gltf", "glTF dışa aktar"),
    ("file.export.ifc", "IFC dışa aktar"),
    ("road.alignments.import", "LandXML'den aks al"),
    ("road.stakeout.query", "Aplikasyon cetveli"),
    ("road.stakeout.send", "Aplikasyonu alete gönder"),
    ("road.sheets.plan", "Yol paftaları"),
    ("road.sheets.sections", "Enkesit paftaları"),
    ("road.alignments.fit", "Çizgiden aks"),
    ("road.profiles.fit", "Çizimden boykesit"),
    ("road.alignments.query", "Aksı sorgula"),
    ("cad.topology.check", "Topolojiyi denetle"),
    ("cad.topology.exceptions", "Topoloji istisnaları"),
    ("cad.topology.repair", "Topolojiyi onar (tarif)"),
    ("cad.addresses.check", "Adres verisini denetle"),
    ("cad.addresses.compare", "Adres kaydıyla karşılaştır"),
    ("cad.addresses.normalize", "Adres yazılışlarını düzelt"),
    ("cad.addresses.define", "Adres verisi"),
    ("cad.layers.time", "Katmanın zamanı"),
    ("cad.scenarios.edit", "Senaryo"),
    ("cad.layers.filter", "Katman süzgeci"),
    ("cad.layers.crs", "Sistem ata"),
    ("cad.layers.reproject", "Yeniden izdüşür"),
    ("cad.layers.table", "Koşullu biçim"),
    ("cad.layers.rules", "Kayıt kuralları"),
    ("cad.layers.indexes", "Dizinler"),
    ("cad.layers.contingents", "Değer bileşimleri"),
    ("cad.layers.subtypes", "Alt türler"),
    ("cad.layers.compose", "Katman çizimi"),
    ("cad.layers.info", "Bilgi kartı ve eylemler"),
    ("cad.layers.labels", "Etiketler"),
    ("cad.labels.pin", "Etiketi sabitle"),
    ("cad.labels.report", "Etiketleri denetle"),
    ("cad.layers.renderer", "Katman stili"),
    ("cad.layers.edit", "Katman düzenle"),
    ("cad.layers.set", "Katman ağacı"),
    ("cad.layers.states", "Katman durumları"),
    ("cad.fields.usage", "Alanın kullanıldığı yerler"),
    ("cad.layers.find", "Katmanları bul"),
    ("cad.layers.translate", "Katmanları çevir"),
    ("cad.linetypes.edit", "Çizgi tipleri"),
    ("cad.hatchpatterns.edit", "Tarama desenleri"),
    ("cad.library.edit", "Stil kitaplığı"),
    ("cad.styles.impact", "Stil değişikliğinin etkisi"),
    ("cad.standards.resolve", "Proje standartlarını birleştir"),
    ("cad.standards.palettes", "Ofis araç paletleri"),
    ("cad.standards.install", "Standart kaynaklarını kur"),
    ("cad.standards.exceptions", "Standart bulgusunu kabul et"),
    ("cad.standards.conflicts", "Bağlı standartlardaki farklı tanımları göster"),
    ("cad.standards.check", "Standardı denetle"),
    ("cad.standards.apply", "Standardı uygula"),
    ("cad.styles.edit", "Yazı ve ölçü stilleri"),
    ("cad.settings.set", "Proje ayarları"),
    ("cad.ucs.set", "Kullanıcı koordinat sistemi"),
    ("cad.views.edit", "Adlı görünümler"),
    ("cad.forms.edit", "Formlar"),
    ("cad.forms.validate", "Formu denetle"),
    ("cad.forms.submit", "Formla kaydet"),
    ("cad.forms.audit", "Kayıtları formla denetle"),
    ("cad.forms.recompute", "Hesapları yenile"),
    ("cad.domains.edit", "Değer listeleri"),
    ("cad.domains.export", "Değer listelerini dışa aktar"),
    ("cad.fields.migrate", "Değerleri biçime çevir"),
    ("cad.attachments.inspect", "Ekleri incele"),
    ("cad.schema.describe", "Şemayı keşfet"),
    ("cad.forms.export", "Formu dışa aktar"),
    ("cad.forms.import", "Formu içe aktar"),
    ("cad.forms.transition", "İş akışı adımı"),
    ("cad.forms.compare", "Sürümleri karşılaştır"),
    ("cad.forms.sources", "Formun kaynakları"),
    ("cad.forms.test", "Form senaryolarını çalıştır"),
    ("cad.records.create", "Kayıt ekle"),
    ("cad.records.duplicate", "Kaydı çoğalt"),
    ("cad.relations.edit", "İlişkiler"),
    ("cad.joins.edit", "Birleşik görünümler"),
    ("cad.joins.query", "Birleşik görünümü oku"),
    ("cad.datasets.edit", "Veri kümesi bilgisi"),
    ("cad.datasets.describe", "Veri kümesini tanımla"),
    ("cad.forms.fill", "Formla toplu yaz"),
    ("cad.units.change", "Birimi değiştir"),
    ("cad.gis.derivation", "GIS türevini denetle"),
    ("cad.gis.takeback", "GIS'ten geri al"),
    ("cad.gis.detach", "GIS türevinin bağını kopar"),
    ("cad.sources.describe", "Kaynakları tanı"),
    ("cad.sources.edit", "Kaynağa bağla"),
    ("cad.fields.write", "Alanlara toplu yaz"),
    ("cad.processing.tools", "İşlem araçları"),
    ("cad.processing.run", "İşlem aracını çalıştır"),
    ("cad.processing.batch", "İşlem aracını toplu çalıştır"),
    ("cad.sources.sync", "Kaynakla eşitle"),
    ("cad.sources.reconcile", "Kaynakla uzlaştır"),
    ("cad.sources.relate", "İlişkiyi dosyaya eşle"),
    ("cad.records.page", "Kayıtları sayfa sayfa oku"),
    ("cad.sources.query", "Kaynağı sorgula"),
    ("file.import.pdf", "PDF içe aktar"),
    ("file.import.gpkg", "GeoPackage içe aktar"),
    ("file.import.dxf", "DXF içe aktar"),
    ("file.import.dwg", "DWG içe aktar"),
    ("cad.sheets.create", "Şablondan pafta oluştur"),
    ("cad.sheets.fit", "Görünüm penceresini nesnelere sığdır"),
    ("cad.sheets.levels", "Kat planı paftaları"),
    ("cad.sheets.details", "Detay paftaları"),
    ("cad.plotstyles.import", "Baskı stili tablosu içe aktar"),
    ("cad.sheets.edit", "Pafta kitabı"),
    ("cad.dimensionsets.create", "Duvar ölçüleri oluştur"),
    ("cad.dimensionsets.edit", "Duvar ölçülerini düzenle"),
    ("cad.dimensionsets.query", "Duvar ölçü gruplarını sorgula"),
    ("cad.changeset.apply", "Değişiklik kümesi"),
    ("cad.donut.create", "Halka oluştur"),
    ("cad.rectangle.create", "Dikdörtgen oluştur"),
    ("cad.polygon.regular", "Düzgün çokgen oluştur"),
    ("cad.sector.create", "Daire dilimi oluştur"),
    ("cad.revcloud.create", "Revizyon bulutu oluştur"),
    ("cad.entities.adjoin", "Bitişik alan"),
    ("cad.parcels.create", "Parsel oluştur"),
    ("cad.shapes.edit", "Biçim işlemleri"),
    ("cad.survey.points", "Ölçme hesabı"),
    ("cad.survey.adjust", "Ağ dengelemesi"),
    ("cad.images.clip", "Resmi kırp"),
    ("cad.time.version", "Yeni sürüm oluştur"),
    ("cad.roads.edit", "Plan yolu çizimi"),
    ("cad.objects.derive", "Nesneden türet"),
    ("cad.annotations.write", "Açıklama yaz"),
    ("cad.areas.edit", "Alan işlemleri"),
    ("cad.constructions.create", "Yapım yardımcıları"),
    ("cad.entities.vertex", "Köşe ve kenar"),
    ("cad.entities.pick", "Tıklanan nesneler"),
    ("cad.crs.transform", "Koordinat dönüştür"),
    ("cad.entities.relate", "Standart ilişkiler"),
    ("cad.entities.isolate", "Geçici nesne yalıtımı"),
    ("cad.entities.select", "Nesne seç"),
    ("cad.snap.find", "Kenet bul"),
    ("cad.points.construct", "Nokta yapımı"),
    ("cad.curves.construct", "Eğri yapımı"),
    ("file.package", "Paketle"),
    ("file.export.gpkg", "GeoPackage dışa aktar"),
    ("file.export.shp", "Shapefile dışa aktar"),
    ("file.export.csv", "CSV dışa aktar"),
    ("file.export.fgb", "FlatGeobuf dışa aktar"),
    ("file.import.fgb", "FlatGeobuf içe aktar"),
    ("file.export.parquet", "GeoParquet dışa aktar"),
    ("file.import.parquet", "GeoParquet içe aktar"),
    ("file.export.gml", "GML dışa aktar"),
    ("file.import.gml", "GML içe aktar"),
    ("file.import.shp", "Shapefile içe aktar"),
    ("file.import.geojson", "GeoJSON içe aktar"),
    ("file.import.kml", "KML/KMZ içe aktar"),
    ("file.export.xyz", "XYZ dışa aktar"),
    ("file.export.table", "Tabloyu dosyaya yaz"),
    ("file.export.atlas", "Atlası PDF yap"),
    ("file.import.csv", "CSV içe aktar"),
    ("file.export.gis", "GIS türevi dışa aktar"),
    ("file.export.sld", "Görünüşü SLD olarak dışa aktar"),
    ("file.import.sld", "SLD'den görünüş al"),
    ("file.export.qml", "Görünüşü QML olarak dışa aktar"),
    ("file.import.qml", "QML'den görünüş al"),
    ("file.export.maplibre", "Görünüşü MapLibre stili olarak dışa aktar"),
    ("file.import.maplibre", "MapLibre stilinden görünüş al"),
    ("file.export.dxf", "DXF dışa aktar"),
    ("file.export.dwg", "DWG dışa aktar"),
    ("file.exchange.check", "Dışa aktarma kaybı denetimi"),
    ("file.recover", "Bozuk çizimi kurtar"),
    ("file.export.block", "Bloğu dosyaya yaz"),
    ("file.export.pdf", "PDF olarak yazdır"),
    ("file.preflight.sheets", "Baskı öncesi tam denetim"),
    ("file.publish.sheets", "Paftaları toplu yayınla"),
    ("file.publish.status", "Yayın durumu"),
];

/// A command's title, or its id when the table has none.
pub(crate) fn title_of(command: &str) -> &str {
    TITLES
        .iter()
        .find(|(id, _)| *id == command)
        .map_or(command, |(_, title)| title)
}

/// An input field's Turkish name; none: the field's own.
fn field_name(key: &str) -> Option<&'static str> {
    Some(match key {
        "a" | "from" => "Başlangıç",
        "anchor" => "Sabit uç",
        "b" | "to" => "Bitiş",
        "a0" => "Başlangıç açısı",
        "a1" => "Bitiş açısı",
        "action" => "Eylem",
        "actions" => "Eylemler",
        "angle" => "Açı",
        "asBlock" => "Blok olarak",
        "at" | "place" => "Yer",
        "attrs" | "attributes" => "Öznitelikler",
        "base" => "Taban noktası",
        "block" => "Blok",
        "blocks" => "Bloklar",
        "host" => "Duvar",
        "openings" => "Açıklıklar",
        "spaces" => "Mahaller",
        "seed" => "İç nokta",
        "grids" => "Akslar",
        "columns" => "Kolonlar",
        "stairs" => "Merdivenler",
        "ramps" => "Rampalar",
        "series" => "Seri",
        "spacings" => "Aralıklar",
        "crossings" => "Aks kesişimleri",
        "tagStart" => "İlk etiket",
        "risers" => "Rıht sayısı",
        "riserHeight" => "Rıht",
        "tread" => "Basamak",
        "landing" => "Sahanlık",
        "rise" => "Yükseklik farkı",
        "slope" => "Eğim",
        "tags" => "Etiketler",
        "of" => "Nesne",
        "marks" => "İşaretler",
        "form" => "Biçim",
        "target" => "Hedef",
        "mark" => "Numara",
        "flip" => "Çevir",
        "surface" => "Yüzey",
        "grid" => "Izgara",
        "size" => "Yazı yüksekliği",
        "origin" => "Başlangıç",
        "content" => "Şablon",
        "leader" => "Kol",
        "all" => "Hepsi",
        "change" => "Değişiklik",
        "sort" => "Sıra",
        "group" => "Grup",
        "total" => "Toplam",
        "row" => "Satır",
        "column" => "Sütun",
        "parts" => "Parçalar",
        "usage" | "usages" => "Kullanım",
        "levels" => "Katlar",
        "phases" => "Tadilat durumları",
        "bothSides" => "İki yana",
        "boundaries" => "Sınırlar",
        "boundaryLayer" => "Sınır katmanı",
        "bulges" => "Bükümler",
        "by" => "Yöntem",
        "c" | "center" => "Merkez",
        "chain" => "Zincir",
        "clip" => "Pano içeriği",
        "changes" => "Değişiklikler",
        "closed" => "Kapalı",
        "color" => "Renk",
        "copy" => "Kopya",
        "corner" => "Köşe",
        "count" => "Sayı",
        "deleteSource" => "Kaynağı sil",
        "description" => "Açıklama",
        "dimension" | "measure" => "Ölçü",
        "dimensionStyles" => "Ölçü stilleri",
        "distance" => "Uzaklık",
        "drawingUnit" => "Çizim birimi",
        "dx" => "Doğu farkı",
        "dy" => "Kuzey farkı",
        "fields" => "Alanlar",
        "filter" => "Süzgeç",
        "geometry" => "Geometri",
        "height" => "Yükseklik",
        "holes" => "Delikler",
        "id" => "Kimlik",
        "index" => "Sıra",
        "islands" => "Adalar",
        "kind" => "Tür",
        "kinds" => "Türler",
        "label" => "Etiket",
        "layer" | "layerId" => "Katman",
        "layers" => "Katmanlar",
        "library" => "Kitaplık",
        "lineType" => "Çizgi tipi",
        "lineTypeScale" => "Çizgi tipi ölçeği",
        "lineWeight" => "Çizgi kalınlığı",
        "locked" => "Kilitli",
        "name" => "Ad",
        "note" => "Not",
        "objects" => "Nesneler",
        "offset" => "Öteleme",
        "opacity" => "Opaklık",
        "operation" | "op" => "İşlem",
        "ops" => "İşlemler",
        "p" | "point" => "Nokta",
        "parent" => "Üst grup",
        "path" => "Yol",
        "pattern" => "Desen",
        "picks" => "Tıklananlar",
        "plotScale" => "Çizim ölçeği",
        "points" | "pts" => "Noktalar",
        "r" | "radius" => "Yarıçap",
        "reference" => "Başvuru",
        "region" => "Bölge",
        "renderer" => "Stil",
        "scale" => "Ölçek",
        "side" => "Yan",
        "srid" => "Koordinat sistemi",
        "steps" => "Adımlar",
        "style" => "Stil",
        "symbol" => "Sembol",
        "text" => "Yazı",
        "textStyles" => "Yazı stilleri",
        "time" => "Zaman",
        "tolerance" => "Tolerans",
        "topology" => "Topoloji",
        "transform" => "Dönüşüm",
        "transparency" => "Saydamlık",
        "trim" => "Kırp",
        "uid" => "Nesne",
        "uids" => "Nesneler",
        "unit" => "Birim",
        "value" => "Değer",
        "visible" => "Görünür",
        "widths" => "Genişlikler",
        "window" => "Pencere",
        "wipeout" => "Maske",
        "z" => "Kot",
        // The fields Mimari's commands send, which the row said in English (docs/adr/3004).
        "adopt" => "Ortak duvar",
        "arch" => "Mimari",
        "bubbleSize" => "Balon boyu",
        "bubbles" => "Balonlar",
        "caps" => "Uçlar",
        "cooktop" => "Ocak",
        "depth" => "Derinlik",
        "dishwasher" => "Bulaşık makinesi",
        "distanceReference" => "Ölçü noktası",
        "edge" => "Kenar",
        "extent" => "Kapsam",
        "face" => "Yüz",
        "first" => "İlk numara",
        "fit" => "Uyum",
        "frame" => "Çerçeve",
        "justify" => "Hiza",
        "keepSources" => "Kaynak çizgiler korunur",
        "length" => "Boy",
        "level" => "Kat",
        "material" => "Malzeme",
        "method" => "Alan yöntemi",
        "middle" => "Ortada",
        "nodes" => "Düğümler",
        "number" => "Numara",
        "order" => "Sıra",
        "oven" => "Fırın",
        "pairs" => "Çiftler",
        "radial" => "Eğrisel aks sistemi",
        "rings" => "Halkalar",
        "role" => "Döşeme türü",
        "rotation" => "Açı",
        "sink" => "Evye",
        "slabs" => "Döşemeler",
        "source" => "Kaynak",
        "span" => "Aralık",
        "system" => "Aks sistemi",
        "thickness" => "Kalınlık",
        "type" => "Tür",
        "void" => "Döşeme boşluğu",
        "wall" => "Duvar",
        "walls" => "Duvarlar",
        "width" => "Genişlik",
        _ => return None,
    })
}

/// A number as people read it: at most four decimals, no trailing zeros.
pub(crate) fn number(n: f64) -> String {
    let text = format!("{n:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// A point `{x, y}` (east, north: Y, X), or none.
fn point(value: &Value) -> Option<String> {
    let map = value.as_object()?;
    if map.len() != 2 {
        return None;
    }
    Some(format!(
        "{}, {}",
        number(map.get("x")?.as_f64()?),
        number(map.get("y")?.as_f64()?)
    ))
}

/// The tag of a tagged object (`kind`, `op`, `operation`, `type`).
fn tag(map: &serde_json::Map<String, Value>) -> Option<(&'static str, &str)> {
    ["kind", "op", "operation", "type"]
        .into_iter()
        .find_map(|k| map.get(k).and_then(Value::as_str).map(|v| (k, v)))
}

/// A value in words; `deep` objects are said by their tag and fields once.
fn value(key: &str, v: &Value, layer: &dyn Fn(&str) -> Option<String>, deep: bool) -> String {
    match v {
        Value::Null => "yok".to_owned(),
        Value::Bool(b) => if *b { "evet" } else { "hayır" }.to_owned(),
        Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), number),
        Value::String(s) => {
            if matches!(key, "layer" | "layerId" | "boundaryLayer" | "parent")
                && let Some(name) = layer(s)
            {
                return format!("«{name}»");
            }
            if key == "uid" || is_id(s) {
                return "1 nesne".to_owned();
            }
            if let Some(word) = enum_word(key, s) {
                return word.to_owned();
            }
            // A list of fields names them in Turkish (`fields: ["frame"]`).
            if key == "fields"
                && let Some(name) = field_name(s)
            {
                return name.to_lowercase();
            }
            // A temporary distance's reference and a length's fixed end (docs/adr/0728).
            match (key, s.as_str()) {
                ("to", "grid") => return "aks".to_owned(),
                ("to", "face") => return "yüz".to_owned(),
                ("anchor", "start") => return "baş".to_owned(),
                ("anchor", "end") => return "son".to_owned(),
                ("anchor", "middle") => return "orta".to_owned(),
                _ => {}
            }
            let short: String = s.chars().take(40).collect();
            if short.chars().count() < s.chars().count() {
                format!("{short}…")
            } else {
                short
            }
        }
        Value::Array(items) => {
            if key == "uids" || key == "objects" {
                return format!("{} nesne", items.len());
            }
            if items.iter().all(|i| point(i).is_some()) && !items.is_empty() {
                return format!("{} nokta", items.len());
            }
            if items.len() <= 4 && items.iter().all(|i| !i.is_object() && !i.is_array()) {
                return items
                    .iter()
                    .map(|i| value(key, i, layer, false))
                    .collect::<Vec<_>>()
                    .join(", ");
            }
            format!("{} öğe", items.len())
        }
        Value::Object(map) => {
            if let Some(p) = point(v) {
                return p;
            }
            // An object's tag in Turkish too (`distance` is uzaklık, docs/adr/3004).
            let head = tag(map).map(|(k, t)| (k, enum_word(k, t).unwrap_or(t)));
            if !deep {
                return head.map_or_else(|| format!("{} alan", map.len()), |(_, t)| t.to_owned());
            }
            let fields: Vec<String> = map
                .iter()
                .filter(|(k, _)| head.is_none_or(|(t, _)| t != k.as_str()))
                .take(4)
                .map(|(k, v)| format!("{} {}", label_of(k, v).to_lowercase(), value(k, v, layer, false)))
                .collect();
            match head {
                Some((_, t)) if fields.is_empty() => t.to_owned(),
                Some((_, t)) => format!("{t} ({})", fields.join(", ")),
                None => fields.join(", "),
            }
        }
    }
}

fn label(key: &str) -> String {
    field_name(key).map_or_else(|| key.to_owned(), str::to_owned)
}

/// A field's name for its value: a wall's `by` is how far it moves, elsewhere `by` is a way (docs/adr/3004).
fn label_of(key: &str, v: &Value) -> String {
    if key == "by" && v.is_number() {
        return "Miktar".to_owned();
    }
    label(key)
}

/// An object's id, as the journal keeps it (`0192f5a1-6000-…`, `$uid:source`): never shown (docs/adr/3004).
fn is_id(s: &str) -> bool {
    if s.starts_with("$uid") {
        return true;
    }
    let parts: Vec<&str> = s.split('-').collect();
    s.len() == 36
        && parts.iter().map(|p| p.len()).eq([8, 4, 4, 4, 12])
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// A tag or a choice said in Turkish: `left` is sol, `clear` net (docs/adr/3004).
fn enum_word(key: &str, s: &str) -> Option<&'static str> {
    let keyed = matches!(
        key,
        "kind" | "op" | "operation" | "side" | "measure" | "reference" | "justify" | "method" | "role" | "face"
            | "from" | "distanceReference" | "of"
    );
    if !keyed {
        return None;
    }
    Some(match s {
        "left" => "sol",
        "right" => "sağ",
        "center" | "middle" => "orta",
        "start" => "baş",
        "end" => "son",
        "corners" => "köşeler",
        "size" => "boyut",
        "area" => "alan",
        "edge" => "kenar",
        "clear" => "net",
        "axis" => "eksen",
        "inner" => "iç net",
        "outer" => "dış ölçü",
        "net" => "net",
        "gross" => "brüt",
        "centerline" => "duvar ekseni",
        "shift" => "kaydır",
        "set" => "değiştir",
        "distance" => "uzaklık",
        "length" => "boy",
        "flip" => "çevir",
        "move" => "taşı",
        "source" => "kaynak boyunca",
        "balcony" => "balkon",
        "terrace" => "teras",
        "floor" => "döşeme",
        "fixtures" => "donatı",
        "openings" => "açıklıklar",
        "spaces" => "mahaller",
        "units" => "bölümler",
        "finishes" => "kaplamalar",
        "direction" => "doğrultu",
        "rehost" => "başka duvara",
        "replaceType" => "türü değiştir",
        "detach" => "tipten ayır",
        "push" => "tipe uygula",
        "refresh" => "tipi yeniden uygula",
        "reset" => "tipe döndür",
        _ => return None,
    })
}

/// The input in words: “Katman: «Yapı» · Merkez: 487020, 4420015 · Yarıçap: 7.5”;
/// the nesting said once, absent and empty fields left out, cut at its width.
pub(crate) fn inputs_for(
    command: Option<&str>,
    input: &Value,
    layer: &dyn Fn(&str) -> Option<String>,
) -> String {
    if command == Some(kentos_commands::CAD_ARRAYS_ADOPT)
        && let Ok(i) = serde_json::from_value::<kentos_commands::ArraysAdopt>(input.clone())
    {
        let missing = i.items.iter().filter(|i| i.uid.is_none()).count();
        let kept = i.items.iter().filter(|i| i.kept).count();
        let kind = match i.layout {
            kentos_commands::ArrayLayout::Grid { .. } => "Izgara",
            kentos_commands::ArrayLayout::Lattice { .. } => "Dönük/eğik ızgara",
            kentos_commands::ArrayLayout::Line { .. } => "Sıra",
            kentos_commands::ArrayLayout::Polar { .. } => "Kutupsal",
            kentos_commands::ArrayLayout::Path { .. } => "Yol boyunca",
        };
        let mm = (i.tolerance.unwrap_or(1e-6) * 1000.).to_string().replace('.', ",");
        return format!("{kind} · {} yer · {} mevcut blok · {missing} eksik · {kept} korunan · Tolerans: {mm} mm",
            i.items.len(), i.items.len()-missing);
    }
    if command == Some(kentos_commands::CAD_BLOCKS_CONVERT)
        && let Ok(i) = serde_json::from_value::<kentos_commands::BlocksConvert>(input.clone())
    {
        // This command's tolerance is metres, independent of the project's display unit.
        // Keep significant digits: the generic four-decimal summary made 0.001 mm say 0.
        let mm = (i.tolerance.unwrap_or(kentos_commands::RECOGNITION_TOLERANCE) * 1000.)
            .to_string()
            .replace('.', ",");
        let name = value("name", &Value::String(i.name), layer, false);
        let on = |b| if b { "evet" } else { "hayır" };
        return format!(
            "Ad: {name} · {} aday · Kaynak: {} parça · Tolerans: {mm} mm · Döndürme: {} · Aynalama: {} · Farklı boyut: {}",
            i.groups.len(), i.source.len(), on(i.rotations.unwrap_or(true)),
            on(i.mirrors.unwrap_or(false)), on(i.scales.unwrap_or(false))
        );
    }
    let words = inputs(input, layer);
    // A wall's width is its Kalınlık (docs/adr/3003, 3004).
    if command.is_some_and(|c| c.starts_with("cad.walls.")) {
        return words.replace("Genişlik:", "Kalınlık:").replace("genişlik ", "kalınlık ");
    }
    words
}

pub(crate) fn inputs(input: &Value, layer: &dyn Fn(&str) -> Option<String>) -> String {
    let Some(map) = input.as_object() else {
        return String::new();
    };
    let first = ["uids", "uid", "layerId", "layer"];
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_by_key(|k| first.iter().position(|f| f == k).unwrap_or(first.len()));
    let parts: Vec<String> = keys
        .into_iter()
        .filter(|k| k.as_str() != "expectedRevision")
        .filter_map(|k| {
            let v = &map[k];
            let empty = v.is_null()
                || v.as_array().is_some_and(Vec::is_empty)
                || v.as_object().is_some_and(serde_json::Map::is_empty);
            (!empty).then(|| format!("{}: {}", label_of(k, v), value(k, v, layer, true)))
        })
        .collect();
    let text = parts.join(" · ");
    if text.chars().count() > INPUT_WIDTH {
        let cut: String = text.chars().take(INPUT_WIDTH).collect();
        format!("{cut}…")
    } else {
        text
    }
}

/// A project part's name.
fn part_name(part: ProjectPart) -> &'static str {
    match part {
        ProjectPart::Name => "çizimin adı",
        ProjectPart::Settings => "proje ayarları",
        ProjectPart::Styles => "stil kitaplığı",
        ProjectPart::Sheets => "pafta kitabı",
        ProjectPart::LayerFlags => "katmanların durumu",
        ProjectPart::Constraints => "kısıtlar",
        ProjectPart::DimensionSets => "otomatik ölçüler",
    }
}

/// What a row wrote: “1 nesne eklendi · 2 nesne değişti”, the parts and files.
pub(crate) fn outcome(row: &CallRecord) -> String {
    let mut said = Vec::new();
    for (n, what) in [
        (row.created.len(), "eklendi"),
        (row.changed.len(), "değişti"),
        (row.removed.len(), "silindi"),
    ] {
        if n > 0 {
            said.push(format!("{n} nesne {what}"));
        }
    }
    for part in &row.parts {
        said.push(format!("{} değişti", part_name(*part)));
    }
    for file in &row.effects {
        said.push(format!("dosya yazıldı: {}", file.path));
    }
    let text = if said.is_empty() && row.base != row.revision {
        // Block and other definition edits may have no object ids or named
        // project parts in this journal format. A changed revision still
        // proves a write; do not claim the drawing was unchanged.
        "Çizim güncellendi".to_owned()
    } else if said.is_empty() {
        "Çizimde değişiklik yok".to_owned()
    } else {
        let mut text = said.join(" · ");
        if let Some(first) = text.get(..1) {
            let upper = first.to_uppercase();
            text.replace_range(..1, &upper);
        }
        text
    };
    match row.status {
        CallStatus::Done => text,
        CallStatus::Undone => format!("{text} · geri alındı"),
    }
}

/// A row's title: the command's, or what an edit made without one was.
pub(crate) fn row_title(row: &CallRecord) -> String {
    match (row.kind, &row.command) {
        (CallKind::Command, Some(command)) => title_of(command).to_owned(),
        _ => format!(
            "Komutsuz düzenleme: {}",
            row.label.as_deref().unwrap_or("adsız")
        ),
    }
}

/// A warning as the history writes it: its code, then its message.
pub(crate) fn warning(w: &CommandWarning) -> String {
    format!("{}: {}", w.code, w.message)
}

/// How a refused call ended, its code and message.
pub(crate) fn refusal(r: &RefusedCall) -> String {
    let how = match r.refused {
        RefusedAs::Failed => "Reddedildi",
        RefusedAs::Conflict => "Çakışma",
        RefusedAs::NeedsInput => "Girdi eksik",
        RefusedAs::Cancelled => "Vazgeçildi",
    };
    match &r.error {
        Some(e) => format!("{how}, hiçbir şey yazılmadı · {}: {}", e.code, e.message),
        None => format!("{how}, hiçbir şey yazılmadı"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_titles_are_the_catalog_s() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../web/src/contracts/generated/commandCatalog.json");
        let text = std::fs::read_to_string(path).expect("the catalog");
        let catalog: Value = serde_json::from_str(&text).expect("the catalog reads");
        let local: Vec<(String, String)> = catalog["commands"]
            .as_array()
            .expect("its commands")
            .iter()
            .filter(|c| {
                c["hosts"]
                    .as_array()
                    .is_some_and(|h| h.iter().any(|h| h == "desktop"))
            })
            .map(|c| {
                (
                    c["id"].as_str().unwrap_or_default().to_owned(),
                    c["title"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let mine: Vec<(String, String)> = TITLES
            .iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect();
        assert_eq!(
            mine, local,
            "TITLES is the catalog's; write it again from it"
        );
        for (id, _) in kentos_native_application::DESKTOP_COMMANDS {
            assert_ne!(title_of(id), *id, "{id} has a title");
        }
    }

    #[test]
    fn the_input_is_said_in_words() {
        let layer = |id: &str| (id == "yapi").then(|| "Yapı".to_owned());
        assert_eq!(
            inputs(
                &json!({"c": {"x": 487020.0, "y": 4420015.5}, "expectedRevision": "3", "layerId": "yapi", "r": 7.5, "attrs": {}}),
                &layer
            ),
            "Katman: «Yapı» · Merkez: 487020, 4420015.5 · Yarıçap: 7.5"
        );
        assert_eq!(
            inputs(
                &json!({"uids": ["a", "b"], "by": {"kind": "distance", "distance": 0.5, "side": {"x": 1.0, "y": 2.0}}, "bothSides": false}),
                &layer
            ),
            "Nesneler: 2 nesne · İki yana: hayır · Yöntem: uzaklık (uzaklık 0.5, yan 1, 2)"
        );
        assert_eq!(
            inputs(
                &json!({"pts": [{"x": 0, "y": 0}, {"x": 1, "y": 0}, {"x": 1, "y": 1}], "closed": true}),
                &layer
            ),
            "Kapalı: evet · Noktalar: 3 nokta"
        );
    }

    #[test]
    fn repeat_conversion_history_names_millimetres_without_rounding_a_positive_tolerance_to_zero() {
        let mut input = json!({"name":"Yaylı tekrar", "source":["source"],
            "groups":[["source"],["other"]],"mirrors":true});
        let before = input.clone();
        let said = inputs_for(Some(kentos_commands::CAD_BLOCKS_CONVERT), &input, &|_| None);
        assert!(said.contains("2 aday · Kaynak: 1 parça · Tolerans: 0,001 mm"), "{said}");
        assert!(said.contains("Döndürme: evet · Aynalama: evet · Farklı boyut: hayır"));
        assert_eq!(input, before, "only the readable summary changes; replay keeps model metres");
        input["tolerance"] = json!(1e-12);
        let said = inputs_for(Some(kentos_commands::CAD_BLOCKS_CONVERT), &input, &|_| None);
        assert!(said.contains("Tolerans: 0,000000001 mm"), "{said}");
        assert_eq!(inputs_for(Some("cad.circle.create"), &input, &|_| None),
            inputs(&input, &|_| None), "other commands keep their existing summary");
    }

    #[test]
    fn adoption_history_says_missing_and_kept_places_with_model_tolerance_in_millimetres() {
        let input = json!({"source":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
            "layout":{"kind":"line","count":3,"dx":2.,"dy":0.,"fit":false},
            "items":[{"uid":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"},{},
                {"uid":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60","kept":true}]});
        let before = input.clone();
        assert_eq!(inputs_for(Some(kentos_commands::CAD_ARRAYS_ADOPT), &input, &|_| None),
            "Sıra · 3 yer · 2 mevcut blok · 1 eksik · 1 korunan · Tolerans: 0,001 mm");
        assert_eq!(input, before, "the replay input stays in model metres");
    }

    #[test]
    fn a_refusal_says_its_code() {
        let r = RefusedCall {
            seq: 1,
            command: "cad.circle.create".into(),
            version: 1,
            input: json!({}),
            refused: RefusedAs::Failed,
            error: Some(kentos_commands::CommandError {
                code: "layer_locked".into(),
                message: "Katman kilitli.".into(),
                ..Default::default()
            }),
        };
        assert_eq!(
            refusal(&r),
            "Reddedildi, hiçbir şey yazılmadı · layer_locked: Katman kilitli."
        );
    }
}
