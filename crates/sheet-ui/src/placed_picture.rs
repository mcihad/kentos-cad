//! A picture the host made, placed on the sheet in front (docs/adr/8175 §3.3): 3B görünüm's picture of the model. In
//! the left half of the sheet's printable area, else the right half when a picture is there, else its middle; at most
//! nine tenths of the half across and six tenths of the area up, its shape kept. Its bytes kept with the book as a
//! chosen picture's are (docs/adr/0251); one undo step; the new item chosen.

use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
use kentos_sheet::model::{Item, Owner};
use kentos_sheet::ops::{AddAssets, AddItems, Op};
use kentos_sheet::units::RectUm;

use crate::designer::Designer;

/// The share of a half's width and of the printable area's height a placed picture takes at most.
const SHARE_ACROSS: f64 = 0.9;
const SHARE_UP: f64 = 0.6;

/// Whether `a` and `b` share some of their area.
fn overlap(a: &RectUm, b: &RectUm) -> bool {
    a.left < b.left + b.width && b.left < a.left + a.width && a.top < b.top + b.height && b.top < a.top + a.height
}

impl Designer {
    /// `bytes`, a picture of `size` pixels named `name`, placed on the sheet in front in the undo step `step`: the
    /// item's name (`name`, else `name 2` …), or why nothing was placed (no sheet in front, an empty picture, one the
    /// book cannot hold).
    pub fn place_picture(&mut self, name: &str, bytes: Vec<u8>, size: (u32, u32), step: &str) -> Result<String, String> {
        let Some(sheet) = self.open_sheet() else {
            return Err("Önde pafta yok: önce paftayı açın, resim açık paftaya konur.".to_owned());
        };
        let (width, height) = size;
        if width == 0 || height == 0 {
            return Err("Resim boş: konacak bir şey yok.".to_owned());
        }
        let area = sheet.page.margin_rect();
        let half = f64::from(area.width) / 2.0;
        let (aw, ah) = (half * SHARE_ACROSS, f64::from(area.height) * SHARE_UP);
        let ratio = f64::from(height) / f64::from(width);
        let (w, h) = if aw * ratio <= ah { (aw, aw * ratio) } else { (ah / ratio, ah) };
        let at = |middle: f64| {
            RectUm::new(
                area.left + (middle - w / 2.0).round() as i32,
                area.top + ((f64::from(area.height) - h) / 2.0).round() as i32,
                w.round() as i32,
                h.round() as i32,
            )
        };
        // The halves' middles in turn, the first no picture lies on; else the area's middle.
        let pictures: Vec<RectUm> =
            sheet.items.iter().filter(|i| matches!(i.kind, ItemKind::Picture(_))).map(|i| i.frame).collect();
        let free = |r: &RectUm| !pictures.iter().any(|p| overlap(p, r));
        let frame = [half / 2.0, half * 1.5].into_iter().map(at).find(free).unwrap_or_else(|| at(half));
        let sheet_id = sheet.id.clone();
        let taken: Vec<&str> = sheet.items.iter().map(|i| i.name.as_str()).collect();
        let item_name = (1..)
            .map(|n| if n == 1 { name.to_owned() } else { format!("{name} {n}") })
            .find(|n| !taken.contains(&n.as_str()))
            .unwrap_or_else(|| name.to_owned());
        let meta = crate::pictures::asset_of(name, &bytes)?;
        let sha = meta.sha256.clone();
        let mut ops = Vec::new();
        if !self.book.assets.iter().any(|a| a.sha256 == sha) {
            ops.push(Op::AddAssets(AddAssets { assets: vec![meta] }));
        }
        let id = self.new_id("o");
        let picture = PictureItem { asset: Some(sha.clone()), fit: PictureFit::Contain, clip: true };
        let item = Item::new(&id, &item_name, frame, ItemKind::Picture(picture));
        ops.push(Op::AddItems(AddItems { to: Owner::sheet(&sheet_id), items: vec![item], index: None }));
        self.add_asset_bytes(&sha, bytes);
        if !self.commit_as(ops, Some(step)) {
            return Err(self.error.clone().unwrap_or_else(|| "Resim paftaya konamadı.".to_owned()));
        }
        self.selection = vec![id];
        self.refresh();
        Ok(item_name)
    }
}

#[cfg(test)]
mod tests {
    use kentos_sheet::kinds::{ItemKind, PictureFit};

    use crate::questions::QuestionsMessage;
    use crate::{Context, Designer, Effect, Message};

    /// A PNG of `w` × `h` grey pixels.
    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("a header");
        writer.write_image_data(&vec![200u8; (w * h * 3) as usize]).expect("the pixels");
        drop(writer);
        out
    }

    #[test]
    fn a_picture_goes_in_the_middle_of_the_sheet_in_front_its_shape_kept() {
        let mut d = Designer::new(Context::default());
        let none = d.place_picture("3B görünüm", png(300, 200), (300, 200), "3B görünüm");
        assert!(none.is_err_and(|why| why.contains("Önde pafta yok")), "no sheet in front");
        assert!(d.update(Message::NewSheet).is_empty(), "the template's questions first");
        assert_eq!(d.update(Message::Questions(QuestionsMessage::Make)), [Effect::ShowSheet]);
        let steps = d.undo.len();
        let name = d.place_picture("3B görünüm", png(300, 200), (300, 200), "3B görünüm").expect("placed");
        assert_eq!(name, "3B görünüm");
        let sheet = d.open_sheet().expect("a sheet");
        let area = sheet.page.margin_rect();
        let item = sheet.items.iter().find(|i| i.name == name).expect("its item");
        // Its shape, within the left half and six tenths of the area up, in the half's middle.
        assert!((f64::from(item.frame.width) / f64::from(item.frame.height) - 1.5).abs() < 1e-3, "{:?}", item.frame);
        assert!(item.frame.width <= area.width / 2 && item.frame.height <= area.height * 6 / 10 + 1);
        let middle = |l: i32, w: i32| l + w / 2;
        assert!((middle(item.frame.left, item.frame.width) - (area.left + area.width / 4)).abs() <= 1, "{:?}", item.frame);
        assert!((middle(item.frame.top, item.frame.height) - middle(area.top, area.height)).abs() <= 1);
        let first = item.frame;
        let ItemKind::Picture(p) = &item.kind else { panic!("a picture") };
        assert_eq!(p.fit, PictureFit::Contain);
        assert!(d.book.assets.iter().any(|a| Some(&a.sha256) == p.asset.as_ref() && a.width == 300 && a.height == 200));
        assert_eq!(d.selection(), std::slice::from_ref(&item.id));
        assert_eq!(d.undo.len(), steps + 1, "one undo step");
        assert_eq!(d.undo.last().map(|s| s.label.as_str()), Some("3B görünüm"));
        // A second of the same view takes the next name, the same picture and the right half; a third the middle.
        assert_eq!(d.place_picture("3B görünüm", png(300, 200), (300, 200), "3B görünüm").expect("placed"), "3B görünüm 2");
        assert_eq!(d.book.assets.len(), 1 + d.book.assets.iter().filter(|a| a.width != 300).count());
        let second = d.open_sheet().unwrap().items.iter().find(|i| i.name == "3B görünüm 2").unwrap().frame;
        assert!(!super::overlap(&first, &second), "{first:?} {second:?}");
        assert!((middle(second.left, second.width) - (area.left + area.width * 3 / 4)).abs() <= 1);
        d.place_picture("3B görünüm", png(300, 200), (300, 200), "3B görünüm").expect("placed");
        let third = d.open_sheet().unwrap().items.iter().find(|i| i.name == "3B görünüm 3").unwrap().frame;
        assert!((middle(third.left, third.width) - middle(area.left, area.width)).abs() <= 1);
    }
}
