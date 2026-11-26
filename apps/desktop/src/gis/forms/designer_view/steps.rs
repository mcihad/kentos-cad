//! The designer's way to a release in one line (the CBS review's 6th item, docs/adr/9325): Alanları yerleştir, Örnek
//! kayıtla dene, Taslağı kaydet, Formu yayınla, each said done or not; the first not done is the next to take.

use super::*;

impl App {
    /// The four steps for `form` as the window holds it.
    pub(super) fn design_steps<'a>(&self, w: &Window, form: &ProjectForm) -> Element<'a, Message> {
        let placed = w
            .body
            .as_ref()
            .is_some_and(|b| b.walk().iter().any(|(_, i)| matches!(i, FormItem::Field(_))));
        let saved = !w.dirty;
        let published = saved
            && form
                .releases
                .last()
                .is_some_and(|r| crate::forms::designer::same_form(&r.body, &form.draft));
        let steps = [
            ("Alanları yerleştir", placed),
            ("Örnek kayıtla dene", w.tried),
            (SAVE, saved),
            (PUBLISH, published),
        ];
        let next = steps.iter().position(|(_, done)| !done);
        let mut line = Row::new().spacing(8).align_y(Center);
        for (i, (words, done)) in steps.iter().enumerate() {
            if i > 0 {
                line = line.push(icon(Icon::ChevronRight).size(11.0).tone(Tone::Muted));
            }
            let mark: Element<'_, Message> = if *done {
                icon(Icon::Check).size(12.0).tone(Tone::Success).into()
            } else {
                label::caption(format!("{}", i + 1)).style(muted).into()
            };
            let text: Element<'_, Message> = if next == Some(i) {
                label::caption(*words).font(typography::ui_strong()).into()
            } else {
                label::caption(*words).style(muted).into()
            };
            line = line.push(row![mark, text].spacing(4).align_y(Center));
        }
        container(line).padding([2, 4]).into()
    }
}
