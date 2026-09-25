//! Model ve düzen sekmeleri: model alanı ve kâğıt paftaları.
//!
//! İlk sekme her zaman model alanıdır; kapanmaz, yeri değişmez. Düzenler
//! (paftalar) haritanın kendi görünümüyle bir kâğıda yerleştirilmiş
//! hâlidir; her düzen kendi görünümünü saklar.

use kentos_rc::spatial::Viewport;

/// Bir düzen: kâğıt paftası.
#[derive(Debug, Clone)]
pub struct Sheet {
    pub name: String,
    /// Paftadaki harita çerçevesinin görünümü.
    pub viewport: Viewport,
    /// Çerçeve ilk boyutunu aldığında görünür katmanlara sığdırılır.
    pub fitted: bool,
}

/// Model ve düzen sekmeleri. Sekme sırası: 0 model alanı, ardından
/// düzenler.
#[derive(Debug, Clone)]
pub struct Sheets {
    sheets: Vec<Sheet>,
    current: usize,
    /// Yeni düzenin adındaki sayı.
    next: usize,
}

impl Sheets {
    /// Model alanı ve iki düzen; model alanı açık.
    pub fn new(viewport: Viewport) -> Self {
        let mut sheets = Self {
            sheets: Vec::new(),
            current: 0,
            next: 1,
        };

        sheets.add(viewport);
        sheets.add(viewport);
        sheets.current = 0;
        sheets
    }

    /// Açık sekmenin sırası.
    pub fn current(&self) -> usize {
        self.current
    }

    /// Açık düzen; model alanı açıksa yok.
    pub fn sheet(&self) -> Option<&Sheet> {
        self.current
            .checked_sub(1)
            .and_then(|index| self.sheets.get(index))
    }

    pub fn sheet_mut(&mut self) -> Option<&mut Sheet> {
        self.current
            .checked_sub(1)
            .and_then(|index| self.sheets.get_mut(index))
    }

    /// Düzenler, sekme sırasıyla.
    pub fn iter(&self) -> impl Iterator<Item = &Sheet> {
        self.sheets.iter()
    }

    pub fn select(&mut self, tab: usize) {
        if tab <= self.sheets.len() {
            self.current = tab;
        }
    }

    /// Yeni düzen ekler ve açar; görünümü verilen görünümdür.
    pub fn add(&mut self, viewport: Viewport) {
        self.sheets.push(Sheet {
            name: format!("Düzen {}", self.next),
            viewport,
            fitted: false,
        });
        self.next += 1;
        self.current = self.sheets.len();
    }

    /// Düzeni kapatır; açık düzen kapanırsa solundaki sekme açılır.
    pub fn close(&mut self, tab: usize) {
        let Some(index) = tab
            .checked_sub(1)
            .filter(|index| *index < self.sheets.len())
        else {
            return;
        };

        self.sheets.remove(index);

        if self.current >= tab && self.current > 0 {
            self.current -= 1;
        }
    }

    /// Sekmeyi `from` sırasından `to` sırasına taşır; model alanı ilk
    /// sekmede kalır.
    pub fn reorder(&mut self, from: usize, to: usize) {
        let count = self.sheets.len();

        if from == 0 || from > count {
            return;
        }

        let to = to.clamp(1, count);
        let sheet = self.sheets.remove(from - 1);
        self.sheets.insert(to - 1, sheet);

        self.current = match self.current {
            current if current == from => to,
            current if from < current && current <= to => current - 1,
            current if to <= current && current < from => current + 1,
            current => current,
        };
    }
}

#[cfg(test)]
mod tests {
    use iced::Size;
    use kentos_rc::spatial::LonLat;

    use super::*;

    fn names(sheets: &Sheets) -> Vec<&str> {
        sheets.iter().map(|sheet| sheet.name.as_str()).collect()
    }

    fn sheets() -> Sheets {
        let mut sheets = Sheets::new(Viewport::new(
            LonLat::new(35.0, 39.0),
            5.0,
            Size::new(800.0, 600.0),
        ));
        sheets.add(sheets.sheets[0].viewport);
        sheets
    }

    #[test]
    fn the_model_tab_stays_first() {
        let mut sheets = sheets();
        assert_eq!(names(&sheets), ["Düzen 1", "Düzen 2", "Düzen 3"]);
        assert_eq!(sheets.current(), 3);

        // Model alanı taşınmaz, önüne de sekme konmaz.
        sheets.reorder(0, 2);
        assert_eq!(names(&sheets), ["Düzen 1", "Düzen 2", "Düzen 3"]);

        sheets.reorder(3, 0);
        assert_eq!(names(&sheets), ["Düzen 3", "Düzen 1", "Düzen 2"]);
        assert_eq!(sheets.current(), 1);

        // Açık sekme yerinden oynayan sekmelerle birlikte kayar.
        sheets.select(2);
        sheets.reorder(1, 3);
        assert_eq!(names(&sheets), ["Düzen 1", "Düzen 2", "Düzen 3"]);
        assert_eq!(sheets.current(), 1);
        assert_eq!(
            sheets.sheet().map(|sheet| sheet.name.as_str()),
            Some("Düzen 1")
        );
    }

    #[test]
    fn closing_the_open_sheet_opens_its_left_neighbour() {
        let mut sheets = sheets();

        sheets.select(2);
        sheets.close(2);
        assert_eq!(names(&sheets), ["Düzen 1", "Düzen 3"]);
        assert_eq!(sheets.current(), 1);

        // Solundaki sekme kapanınca açık sekme aynı kalır.
        sheets.select(2);
        sheets.close(1);
        assert_eq!(
            sheets.sheet().map(|sheet| sheet.name.as_str()),
            Some("Düzen 3")
        );

        // Model alanı kapanmaz; son düzen kapanınca model alanı açılır.
        sheets.close(0);
        sheets.close(1);
        assert_eq!(sheets.current(), 0);
        assert!(sheets.sheet().is_none());

        // Yeni düzen sayıyı sürdürür.
        sheets.add(Viewport::new(LonLat::new(0.0, 0.0), 1.0, Size::ZERO));
        assert_eq!(names(&sheets), ["Düzen 4"]);
    }
}
