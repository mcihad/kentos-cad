//! The view the walls are drawn at: the detail the project, the scale or a view asks for, the cut height and
//! the pens (docs/adr/0339 §4, 0696, 0781 §1). Read-only; the wall commands stay in the application.

/// The view the project's walls are drawn at (docs/adr/0339 §4): the detail Proje ayarları › Mimari says, else the
/// drawing scale's (`WallDetail::at_scale`), and that scale for the hatches.
pub fn view_of(
    settings: &kentos_commands::ProjectSettings,
) -> kentos_geometry_core::geom::wall::View {
    view_at(settings, settings.plot_scale)
}

/// The view a sheet's map at 1:`scale` draws the walls at: the settings' detail, else that scale's.
pub fn view_at(
    settings: &kentos_commands::ProjectSettings,
    scale: f64,
) -> kentos_geometry_core::geom::wall::View {
    let detail = match &settings.arch {
        Some(arch) => arch.detail_at(scale),
        None => kentos_commands::WallDetail::at_scale(scale),
    };
    kentos_geometry_core::geom::wall::View {
        detail: kentos_formats::walls::detail(detail),
        scale,
        // The project's plan cut height (docs/adr/8012).
        cut: settings.plan_cut(),
        // The model's lines at the scale's line group (docs/adr/8013).
        pens: settings.pens_at(scale),
    }
}

/// [`view_at`] with a view's own detail when its recipe gives one (docs/adr/0781 §1): that detail outranks the
/// project's and the scale's, as a view's Detail Level does in Revit.
pub fn view_with(
    settings: &kentos_commands::ProjectSettings,
    scale: f64,
    detail: Option<kentos_commands::WallDetail>,
) -> kentos_geometry_core::geom::wall::View {
    let mut view = view_at(settings, scale);
    if let Some(d) = detail {
        view.detail = kentos_formats::walls::detail(d);
    }
    view
}

/// Model-space navigation changes only detail. Patterns retain their project paper scale;
/// sheet maps and exports continue to use `view_at` (ADR 0696).
/// `previous` gives automatic detail a 10% dead band at its two existing thresholds.
pub fn screen_view(
    settings: &kentos_commands::ProjectSettings,
    screen_scale: f64,
    previous: Option<kentos_geometry_core::geom::wall::Detail>,
) -> kentos_geometry_core::geom::wall::View {
    use kentos_geometry_core::geom::wall::Detail;
    let mut view = view_of(settings);
    if settings
        .arch
        .as_ref()
        .and_then(|arch| arch.detail)
        .is_some()
        || !screen_scale.is_finite()
        || screen_scale <= 0.0
    {
        return view;
    }
    let target = kentos_formats::walls::detail(kentos_commands::WallDetail::at_scale(screen_scale));
    let high = kentos_commands::DETAILED_UP_TO;
    let medium = kentos_commands::STANDARD_UP_TO;
    view.detail = match (previous, target) {
        (Some(Detail::Detailed), _) if screen_scale <= high * 1.1 => Detail::Detailed,
        (Some(Detail::Standard), Detail::Detailed) if screen_scale >= high * 0.9 => {
            Detail::Standard
        }
        (Some(Detail::Standard), Detail::Schematic) if screen_scale <= medium * 1.1 => {
            Detail::Standard
        }
        (Some(Detail::Schematic), _) if screen_scale >= medium * 0.9 => Detail::Schematic,
        _ => target,
    };
    view
}
