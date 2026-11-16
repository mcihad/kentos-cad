//! Model görüntüleyici örneği (galerinin Model görüntüleyici sayfası; docs/adr/8073): KentOS'un yazdığı kesit binasının
//! IFC dosyası, görüntüleyici bileşeninde (`kentos_model_view_ui::Viewer`). Dosya galerinin bu sayfası ilk açılınca
//! okunur: vitrinin açılışı onu beklemez. Seçilen nesnenin özellikleri, uygulamanın IFC modeli penceresindeki gibi,
//! okuma yüzünden gelir.

use kentos_ifc::read::{self as ifc, Limits, Model};
use kentos_model_view_ui::{Message, Out, Viewer};

/// KentOS'un kesit binasından yazdığı IFC 4.3 dosyası (fixtures/arch/v1/ifc/, ADR 8047).
const FILE: &[u8] = include_bytes!("../../../fixtures/arch/v1/ifc/kesit-binasi.ifc");
const NAME: &str = "kesit-binasi.ifc";

/// Örneğin durumu: görüntüleyici ve okunan dosya.
pub struct ModelDemo {
    pub viewer: Viewer,
    model: Option<Model>,
}

impl std::fmt::Debug for ModelDemo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelDemo").field("viewer", &self.viewer).finish_non_exhaustive()
    }
}

impl ModelDemo {
    /// Dosyayı okur ve görüntüleyiciye verir; okunamazsa görüntüleyici nedenini söyler.
    pub fn open() -> ModelDemo {
        let mut viewer = Viewer::new(NAME);
        let never = || false;
        let read = ifc::read(FILE, &Limits::default(), &never).and_then(|m| {
            let mut bodies = m.bodies(FILE, &never)?;
            let openings = m.opening_bodies(FILE, &never)?;
            bodies.bodies.extend(openings.bodies);
            bodies.not_shown.extend(openings.not_shown);
            Ok((m, bodies))
        });
        let model = match read {
            Ok((m, bodies)) => {
                let schema = m.summary.schema.clone();
                viewer.load(ifc::view::input(NAME, &m, &bodies), vec![(schema, "Dosyanın şeması, başlığında yazdığı gibi.".to_owned())]);
                Some(m)
            }
            Err(e) => {
                viewer.failed(e.to_string());
                None
            }
        };
        ModelDemo { viewer, model }
    }

    /// Görüntüleyicinin iletisi; seçilen nesnenin özellikleri dosyadan okunur. Komut satırına yazılacak satırı verir.
    pub fn update(&mut self, message: Message) -> Option<String> {
        match self.viewer.update(message)? {
            Out::Chosen(Some((object, _))) => {
                let model = self.model.as_ref()?;
                match model.details(FILE, object) {
                    Ok(d) => self.viewer.set_details(object, d),
                    Err(e) => return Some(format!("Galeri: nesnenin özellikleri okunamadı: {e}")),
                }
                let o = &model.objects[object];
                Some(format!("Galeri: model görüntüleyicide {} ({}) seçildi.", o.name.as_deref().unwrap_or("adsız"), o.class))
            }
            Out::Chosen(None) => None,
            Out::Cancel | Out::Close => Some("Galeri: örnekte görüntüleyici kapanmaz; uygulamada IFC modeli penceresi kapanır.".to_owned()),
        }
    }
}
