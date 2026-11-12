//! Gevşeterek düzenle (docs/adr/0337 §1): the next write that changes constrained objects relaxes their constraints
//! instead of keeping them (AutoCAD's Shift while editing); pressed again, it does not.

use kentos_domain::Relax;

use crate::app::App;

impl App {
    /// Sets the next write to relax the constraints, or takes that back, and says which.
    pub(crate) fn relax_next(&mut self) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let armed = doc.model.relax() == Relax::Next;
        doc.model
            .set_relax(if armed { Relax::Off } else { Relax::Next });
        self.output(if armed {
            "Gevşetme kaldırıldı: düzenlemeler kısıtları korur.".to_owned()
        } else {
            "Bir sonraki düzenleme kısıtları gevşetir: bozduğu kısıtlar kaldırılır.".to_owned()
        });
    }
}
