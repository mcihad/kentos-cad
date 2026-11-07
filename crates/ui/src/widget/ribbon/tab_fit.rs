//! Which ribbon tabs a narrow strip shows (docs/adr/3005). The open tab and the contextual tabs (the selection's: Duvar,
//! Açıklık …) always show; the other tabs show from the left while they fit, the rest go into the ⋯ menu that stands
//! before the contextual tabs. A strip that scrolled to its open tab hid the leading tabs with nothing to say so: at
//! 1100 px and 18 px text Dosya, Giriş, Ekle and Açıklama were off the screen behind a 4 px scroll bar. Fluent's rule:
//! "Tablists don't wrap; if there isn't enough room, additional tabs go into an overflow menu."

/// The tabs shown, by their place: all when they fit; else the kept ones and the leading others that fit beside the
/// ⋯ button of `more` pixels. None when even the kept tabs and the button do not fit (the strip then scrolls).
pub(super) fn plan(widths: &[f32], keep: &[bool], more: f32, available: f32) -> Option<Vec<bool>> {
    let total: f32 = widths.iter().sum();
    if total <= available {
        return Some(vec![true; widths.len()]);
    }
    let kept: f32 = widths.iter().zip(keep).filter(|(_, k)| **k).map(|(w, _)| w).sum();
    let mut room = available - kept - more;
    if room < 0.0 {
        return None;
    }
    let mut shown = keep.to_vec();
    // The leading tabs while they fit, in their order: the menu takes a run from the first that does not.
    for (i, w) in widths.iter().enumerate() {
        if keep[i] {
            continue;
        }
        if *w > room {
            break;
        }
        room -= w;
        shown[i] = true;
    }
    Some(shown)
}

#[cfg(test)]
mod tests {
    use super::plan;

    #[test]
    fn tabs_that_fit_all_show() {
        assert_eq!(plan(&[50.0, 60.0, 70.0], &[false, true, false], 30.0, 180.0), Some(vec![true; 3]));
    }

    #[test]
    fn the_open_and_contextual_tabs_stay_and_the_trailing_others_go_into_the_menu() {
        // Dosya Giriş Ekle Açıklama Değiştir Mimari(open) Görünüm Yönet Çıktı Duvar(contextual)
        let widths = [60.0, 55.0, 50.0, 85.0, 80.0, 70.0, 85.0, 60.0, 55.0, 90.0];
        let keep = [false, false, false, false, false, true, false, false, false, true];
        let shown = plan(&widths, &keep, 32.0, 560.0).expect("the kept tabs fit");
        assert!(shown[5] && shown[9], "the open tab and the contextual one stay");
        assert_eq!(&shown[..5], &[true, true, true, true, true], "the leading tabs fit: {shown:?}");
        assert_eq!(&shown[6..9], &[false, false, false], "Görünüm, Yönet and Çıktı go into ⋯");
        let used: f32 = widths.iter().zip(&shown).filter(|(_, s)| **s).map(|(w, _)| w).sum();
        assert!(used + 32.0 <= 560.0);
    }

    #[test]
    fn the_menu_takes_a_run_not_a_tab_here_and_there() {
        // The third does not fit: the fourth, small enough, goes into the menu too.
        let shown = plan(&[40.0, 40.0, 90.0, 20.0, 50.0], &[false, false, false, false, true], 20.0, 160.0).unwrap();
        assert_eq!(shown, vec![true, true, false, false, true]);
    }

    #[test]
    fn a_strip_too_narrow_for_its_kept_tabs_scrolls() {
        assert_eq!(plan(&[60.0, 90.0, 90.0], &[false, true, true], 30.0, 150.0), None);
    }
}
