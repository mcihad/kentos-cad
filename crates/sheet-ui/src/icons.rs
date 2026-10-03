//! The sheet layouts' icons: the web's own (apps/web/src/ui/sheet/icons.ts),
//! drawn from the same SVG text by the KentOS UI icon set's rule (20×20,
//! `currentColor`, 1.4 px round strokes), so both platforms show the same
//! pictures. The existing icons (select, pan, text, legend, table, export …)
//! are the set's own.

use kentos_ui::icon::Icon;

pub const LAYOUT: Icon = Icon::Svg(
    r#"<rect x="3.5" y="2.5" width="13" height="15" rx="1"/><rect x="5.5" y="4.5" width="9" height="6.5"/><path d="M5.5 13h9v2.5h-9zM10.5 13v2.5"/>"#,
);
pub const NEW: Icon = Icon::Svg(
    r#"<path d="M11 2.5H4.5a1 1 0 0 0-1 1v13a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1V9"/><path d="M14.5 1.8v5.4M11.8 4.5h5.4"/>"#,
);
pub const TEMPLATE: Icon = Icon::Svg(
    r#"<rect x="6.5" y="2.5" width="10" height="12" rx="1"/><path d="M4 5.5v11a1 1 0 0 0 1 1h8.5"/><path d="M8.7 5.5h5.6M8.7 8h5.6M8.7 10.5h3.4"/>"#,
);
pub const SAVE_TEMPLATE: Icon = Icon::Svg(
    r#"<path d="M11.5 2.5h-7a1 1 0 0 0-1 1v13a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1v-6.5"/><path d="m14.4 1.8 1 2 2.2.3-1.6 1.6.4 2.2-2-1-2 1 .4-2.2-1.6-1.6 2.2-.3z"/>"#,
);
pub const PAGE: Icon = Icon::Svg(
    r#"<rect x="6" y="5" width="10.5" height="12.5" rx=".6"/><path d="M6 2.5h10.5M3.5 5v12.5" stroke-width="1.1"/><path d="m7.2 1.6-1.2.9 1.2.9M15.3 1.6l1.2.9-1.2.9M2.6 6.2l.9-1.2.9 1.2M2.6 16.3l.9 1.2.9-1.2" stroke-width="1.1"/>"#,
);
pub const PREFLIGHT: Icon = Icon::Svg(
    r#"<path d="M7 3.5H5.5a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1v-12a1 1 0 0 0-1-1H13"/><rect x="7" y="2.3" width="6" height="2.6" rx=".6"/><path d="m7 10.3 1.7 1.7 3.6-3.8M7.5 14.6h5"/>"#,
);
pub const ZOOM_PAGE: Icon = Icon::Svg(
    r#"<rect x="6" y="5" width="8" height="10" rx=".5"/><path d="M2.5 6V2.5H6M14 2.5h3.5V6M17.5 14v3.5H14M6 17.5H2.5V14"/>"#,
);
pub const ZOOM_REAL: Icon = Icon::Svg(
    r#"<path d="M4.3 6.4 6 5v10M13.7 6.4 15.4 5v10"/><path d="M10 8.2v.1M10 11.8v.1" stroke-width="2"/>"#,
);
/// The web's `zoomSelection`.
pub const ZOOM_SELECTION: Icon = Icon::Svg(
    r#"<path d="M10 2.5v3M10 14.5v3M2.5 10h3M14.5 10h3"/><circle cx="10" cy="10" r="4.5"/><circle cx="10" cy="10" r="1" fill="currentColor"/>"#,
);
/// The web's `print`.
pub const PRINT: Icon = Icon::Svg(
    r#"<path d="M5.5 7.5v-4h9v4"/><rect x="2.5" y="7.5" width="15" height="6.5" rx="1"/><path d="M5.5 12h9v5h-9z"/>"#,
);
pub const MAP: Icon = Icon::Svg(
    r#"<rect x="2.5" y="3.5" width="15" height="13" rx=".6"/><path d="m2.5 13.3 4-3.4 3 2.1 4.3-4.8 3.7 2.7"/><path d="M9.3 3.5 7.6 9.4" stroke-dasharray="1.6 1.4"/>"#,
);
pub const SCALE_BAR: Icon = Icon::Svg(
    r#"<rect x="2.5" y="8.5" width="15" height="3"/><path d="M6.25 8.5v3M10 8.5v3M13.75 8.5v3"/><path d="M2.5 8.5h3.75v3H2.5zM10 8.5h3.75v3H10z" fill="currentColor" fill-opacity=".35" stroke="none"/><path d="M2.5 14v1.6M10 14v1.6M17.5 14v1.6"/>"#,
);
pub const NORTH: Icon = Icon::Svg(
    r#"<path d="M10 7.2 13.6 17.3 10 15 6.4 17.3z"/><path d="M10 7.2V15l-3.6 2.3z" fill="currentColor" fill-opacity=".35" stroke="none"/><path d="M8.6 1.8v3.8M8.6 3.9l2.6-2.1M9.4 3.3l1.8 2.3"/>"#,
);
pub const COORDINATES: Icon = Icon::Svg(
    r#"<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><path d="M2.5 7h15M7.5 3.5v13M12.5 3.5v13"/><path d="M4 10h2M4 13h2M9 10h2M9 13h2M14 10h2M14 13h2" stroke-width="1.2"/>"#,
);
pub const TITLE_BLOCK: Icon = Icon::Svg(
    r#"<rect x="2.5" y="5" width="15" height="10.5" rx=".5"/><path d="M2.5 8.8h15M2.5 12.2h15M9.5 8.8v6.7M13.5 12.2v3.3"/><path d="M4.5 6.9h6" stroke-width="1.2"/>"#,
);
pub const PICTURE: Icon = Icon::Svg(
    r#"<rect x="2.5" y="3.5" width="15" height="13" rx="1"/><circle cx="7" cy="8" r="1.6"/><path d="m2.5 14.5 4.5-4 3 2.5 3-3 4.5 4"/>"#,
);
pub const SHAPE: Icon = Icon::Svg(
    r#"<rect x="2.5" y="7.5" width="9" height="9" rx=".5"/><circle cx="12.5" cy="7.5" r="4.6"/>"#,
);
pub const RECT: Icon = Icon::Svg(r#"<rect x="3" y="5" width="14" height="10" rx="1"/>"#);
pub const ELLIPSE: Icon = Icon::Svg(r#"<ellipse cx="10" cy="10" rx="7" ry="5"/>"#);
pub const TRIANGLE: Icon = Icon::Svg(r#"<path d="M10 3.5 17 16H3z"/>"#);
pub const POLYGON: Icon = Icon::Svg(r#"<path d="m10 3 6.6 4.8-2.5 7.7H5.9L3.4 7.8z"/>"#);
pub const LINE: Icon =
    Icon::Svg(r#"<path d="M3.5 15.5 16.5 4.5"/><path d="m12.4 4.6 4.1-.1-.1 4.1"/>"#);
pub const ARROW: Icon = Icon::Svg(r#"<path d="M3 10h13"/><path d="m12 6 4.5 4-4.5 4"/>"#);
pub const BORDER: Icon = Icon::Svg(
    r#"<rect x="2.5" y="3.5" width="15" height="13" rx=".4"/><rect x="5" y="6" width="10" height="8" rx=".3"/><path d="M10 3.5V6M10 14v2.5M2.5 10H5M15 10h2.5" stroke-width="1.1"/>"#,
);
pub const GRID: Icon = Icon::Svg(
    r#"<rect x="2.5" y="2.5" width="15" height="15" rx=".6"/><path d="M7.5 6v3M6 7.5h3M12.5 6v3M11 7.5h3M7.5 11v3M6 12.5h3M12.5 11v3M11 12.5h3" stroke-width="1.2"/>"#,
);
pub const ATLAS: Icon = Icon::Svg(
    r#"<rect x="6" y="2.5" width="11.5" height="12.5" rx=".8"/><path d="M4 5v11.5a1 1 0 0 0 1 1h9.5"/><path d="m8 12 2.5-2.5 2 1.5 3-3" stroke-width="1.2"/>"#,
);
pub const OVERVIEW: Icon = Icon::Svg(
    r#"<rect x="2.5" y="3.5" width="15" height="13" rx=".6"/><rect x="9" y="7" width="5" height="4" fill="currentColor" fill-opacity=".3"/><path d="m2.5 14 4-3.5 3 2"/>"#,
);
pub const VARIABLES: Icon = Icon::Svg(
    r#"<rect x="2.5" y="3.5" width="15" height="13" rx="1.5"/><circle cx="10" cy="10" r="2.2"/><path d="M12.2 10v1.2a1.5 1.5 0 0 0 3 0V10a5.2 5.2 0 1 0-2 4.1"/>"#,
);
pub const GROUP: Icon = Icon::Svg(
    r#"<rect x="2.5" y="2.5" width="15" height="15" rx="1" stroke-dasharray="2 1.6"/><rect x="5" y="5" width="5.5" height="5.5" rx=".5"/><rect x="9.5" y="9.5" width="5.5" height="5.5" rx=".5"/>"#,
);
pub const UNGROUP: Icon = Icon::Svg(
    r#"<rect x="2.5" y="2.5" width="6.5" height="6.5" rx=".5"/><rect x="11" y="11" width="6.5" height="6.5" rx=".5"/><path d="M11.5 5.5h3v3M8.5 14.5h-3v-3" stroke-dasharray="1.6 1.4"/>"#,
);
pub const ALIGN_LEFT: Icon = Icon::Svg(
    r#"<path d="M3.5 2.5v15"/><rect x="5.5" y="5" width="10" height="3.5" rx=".5"/><rect x="5.5" y="11.5" width="6" height="3.5" rx=".5"/>"#,
);
pub const ALIGN_CENTER: Icon = Icon::Svg(
    r#"<path d="M10 2.5v15"/><rect x="4" y="5" width="12" height="3.5" rx=".5"/><rect x="6.5" y="11.5" width="7" height="3.5" rx=".5"/>"#,
);
pub const ALIGN_RIGHT: Icon = Icon::Svg(
    r#"<path d="M16.5 2.5v15"/><rect x="4.5" y="5" width="10" height="3.5" rx=".5"/><rect x="8.5" y="11.5" width="6" height="3.5" rx=".5"/>"#,
);
pub const ALIGN_TOP: Icon = Icon::Svg(
    r#"<path d="M2.5 3.5h15"/><rect x="5" y="5.5" width="3.5" height="10" rx=".5"/><rect x="11.5" y="5.5" width="3.5" height="6" rx=".5"/>"#,
);
pub const ALIGN_MIDDLE: Icon = Icon::Svg(
    r#"<path d="M2.5 10h15"/><rect x="5" y="4" width="3.5" height="12" rx=".5"/><rect x="11.5" y="6.5" width="3.5" height="7" rx=".5"/>"#,
);
pub const ALIGN_BOTTOM: Icon = Icon::Svg(
    r#"<path d="M2.5 16.5h15"/><rect x="5" y="4.5" width="3.5" height="10" rx=".5"/><rect x="11.5" y="8.5" width="3.5" height="6" rx=".5"/>"#,
);
pub const DISTRIBUTE_H: Icon = Icon::Svg(
    r#"<rect x="2.5" y="6" width="3.5" height="8" rx=".5"/><rect x="8.25" y="6" width="3.5" height="8" rx=".5"/><rect x="14" y="6" width="3.5" height="8" rx=".5"/><path d="M2.5 3h15" stroke-dasharray="1.6 1.4"/>"#,
);
pub const DISTRIBUTE_V: Icon = Icon::Svg(
    r#"<rect x="6" y="2.5" width="8" height="3.5" rx=".5"/><rect x="6" y="8.25" width="8" height="3.5" rx=".5"/><rect x="6" y="14" width="8" height="3.5" rx=".5"/><path d="M3 2.5v15" stroke-dasharray="1.6 1.4"/>"#,
);
pub const MOVE_LEFT: Icon = Icon::Svg(r#"<path d="M16 10H4.5M8.5 6l-4 4 4 4"/>"#);
pub const MOVE_RIGHT: Icon = Icon::Svg(r#"<path d="M4 10h11.5M11.5 6l4 4-4 4"/>"#);
pub const FRONT: Icon = Icon::Svg(
    r#"<rect x="3" y="3" width="9" height="9" rx=".5"/><rect x="8" y="8" width="9" height="9" rx=".5" fill="currentColor" fill-opacity=".3"/>"#,
);
pub const BACK: Icon = Icon::Svg(
    r#"<rect x="8" y="8" width="9" height="9" rx=".5" fill="currentColor" fill-opacity=".3" stroke-dasharray="2 1.5"/><rect x="3" y="3" width="9" height="9" rx=".5"/>"#,
);
pub const FORWARD: Icon = Icon::Svg(
    r#"<rect x="3.5" y="8.5" width="9" height="8" rx=".5"/><path d="M15 11.5V3.5m-2.5 2.5L15 3.5l2.5 2.5"/>"#,
);
pub const BACKWARD: Icon = Icon::Svg(
    r#"<rect x="3.5" y="3.5" width="9" height="8" rx=".5"/><path d="M15 8.5v8m-2.5-2.5L15 16.5l2.5-2.5"/>"#,
);

// The template gallery's actions and sections (apps/web/src/ui/icons.ts).
pub const CLOUD: Icon = Icon::Svg(
    r#"<path d="M6 15.5a3.5 3.5 0 0 1-.4-7A4.8 4.8 0 0 1 14.8 7a3.3 3.3 0 0 1-.3 8.5z"/><path d="M10 9v4.6M7.9 11.6 10 13.7l2.1-2.1"/>"#,
);
pub const SHARE: Icon = Icon::Svg(
    r#"<circle cx="7.5" cy="7" r="2.6"/><path d="M2.8 16.3c.4-2.9 2.3-4.5 4.7-4.5s4.3 1.6 4.7 4.5"/><path d="M12.6 4.6a2.5 2.5 0 0 1 0 4.8M14 11.9c1.8.4 3 1.9 3.3 4.4"/>"#,
);
pub const TRASH: Icon = Icon::Svg(
    r#"<path d="M4 5.5h12M8 5.5v-2h4v2M5.6 5.5l.8 11h7.2l.8-11"/><path d="M8.6 8.5v5M11.4 8.5v5"/>"#,
);
pub const EDIT: Icon = Icon::Svg(
    r#"<path d="M4 16l.9-3.6 8.4-8.4a1.8 1.8 0 0 1 2.7 2.7l-8.4 8.4z"/><path d="M12 5.3l2.7 2.7"/>"#,
);
pub const COPY: Icon = Icon::Svg(
    r#"<rect x="3.5" y="7.5" width="9" height="9" rx="1"/><path d="M7.5 7.5v-4h9v9h-4"/>"#,
);
pub const HISTORY: Icon =
    Icon::Svg(r#"<path d="M3.5 10a6.5 6.5 0 1 0 2-4.7"/><path d="M3 3.3v3h3M10 6.5V10l2.5 1.8"/>"#);
pub const STYLES: Icon = Icon::Svg(
    r#"<path d="M10 3a7 7 0 1 0 0 14c1 0 1.5-.8 1.2-1.6-.4-1 .2-2 1.3-2H14a3.5 3.5 0 0 0 3.5-3.5C17.5 6 14.1 3 10 3z"/><circle cx="6.6" cy="9.2" r=".9"/><circle cx="8.9" cy="6.2" r=".9"/><circle cx="12.6" cy="6.6" r=".9"/>"#,
);
