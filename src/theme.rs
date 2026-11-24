//! Couleurs, polices et style général.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Stroke, Theme,
};

use crate::model::TaskColor;

pub const BG: Color32 = Color32::from_rgb(0xF4, 0xF6, 0xFA);
pub const SURFACE: Color32 = Color32::WHITE;
pub const BORDER: Color32 = Color32::from_rgb(0xE4, 0xE7, 0xED);
pub const LINE: Color32 = Color32::from_rgb(0xEE, 0xF0, 0xF4);
pub const TEXT: Color32 = Color32::from_rgb(0x1A, 0x1F, 0x2B);
pub const MUTED: Color32 = Color32::from_rgb(0x6B, 0x72, 0x80);
pub const FAINT: Color32 = Color32::from_rgb(0x9A, 0xA0, 0xAB);
pub const ACCENT: Color32 = Color32::from_rgb(0x2F, 0x6B, 0xF2);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xE3, 0xEB, 0xFE);
pub const DONE_BG: Color32 = Color32::from_rgb(0xEE, 0xF0, 0xF3);
pub const DONE_FG: Color32 = Color32::from_rgb(0x8B, 0x91, 0x9C);
pub const DANGER: Color32 = Color32::from_rgb(0xC6, 0x34, 0x34);

/// (fond de la carte, couleur d'accent : rond, heure, icône)
pub fn task_colors(color: TaskColor) -> (Color32, Color32) {
    match color {
        TaskColor::Violet => (Color32::from_rgb(0xED, 0xE9, 0xFD), Color32::from_rgb(0x5B, 0x4B, 0xC4)),
        TaskColor::Rose => (Color32::from_rgb(0xFC, 0xE6, 0xEF), Color32::from_rgb(0xB0, 0x30, 0x5F)),
        TaskColor::Bleu => (Color32::from_rgb(0xE3, 0xEA, 0xFD), Color32::from_rgb(0x2D, 0x55, 0xC8)),
        TaskColor::Orange => (Color32::from_rgb(0xFD, 0xEC, 0xD9), Color32::from_rgb(0xA8, 0x56, 0x1A)),
        TaskColor::Vert => (Color32::from_rgb(0xE1, 0xF3, 0xE6), Color32::from_rgb(0x2A, 0x7D, 0x47)),
    }
}

/// Assombrit légèrement une couleur (survol).
pub fn darken(c: Color32, amount: f32) -> Color32 {
    let f = 1.0 - amount;
    Color32::from_rgb(
        (c.r() as f32 * f) as u8,
        (c.g() as f32 * f) as u8,
        (c.b() as f32 * f) as u8,
    )
}

const BOLD: &str = "bold";

pub fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

/// Segoe UI (police de Windows) si elle est présente, sinon la police d'egui.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let dir = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:\\Windows".into())
        .join("Fonts");
    let read = |name: &str| std::fs::read(dir.join(name)).ok();

    if let Some(bytes) = read("segoeui.ttf") {
        fonts
            .font_data
            .insert("segoe".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".into());
    }
    let mut bold_family = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(bytes) = read("seguisb.ttf").or_else(|| read("segoeuib.ttf")) {
        fonts
            .font_data
            .insert("segoe-semibold".into(), Arc::new(FontData::from_owned(bytes)));
        bold_family.insert(0, "segoe-semibold".into());
    }
    fonts.families.insert(FontFamily::Name(BOLD.into()), bold_family);
    ctx.set_fonts(fonts);
}

pub fn install_style(ctx: &egui::Context) {
    ctx.set_theme(Theme::Light);
    // Barre de titre claire même quand Windows est en mode sombre.
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Light));
    ctx.style_mut_of(Theme::Light, |style| {
        use egui::TextStyle;
        style.text_styles.insert(TextStyle::Body, regular(15.0));
        style.text_styles.insert(TextStyle::Button, regular(15.0));
        style.text_styles.insert(TextStyle::Small, regular(12.5));
        style.text_styles.insert(TextStyle::Heading, bold(20.0));
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 7.0);
        style.spacing.interact_size.y = 32.0;

        let v = &mut style.visuals;
        v.panel_fill = BG;
        v.window_fill = SURFACE;
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.window_corner_radius = 14.into();
        v.selection.bg_fill = ACCENT_SOFT;
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.override_text_color = None;
        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
            &mut v.widgets.noninteractive,
        ] {
            w.corner_radius = 8.into();
        }
        v.widgets.inactive.weak_bg_fill = SURFACE;
        v.widgets.inactive.bg_fill = SURFACE;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xF5, 0xF7, 0xFB);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xC9, 0xCF, 0xD9));
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.active.weak_bg_fill = Color32::from_rgb(0xEC, 0xEF, 0xF5);
        v.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        v.extreme_bg_color = SURFACE;
        v.text_edit_bg_color = Some(SURFACE);
    });
}
