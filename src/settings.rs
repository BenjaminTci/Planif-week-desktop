//! Fenêtre des paramètres.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Id, Layout, Margin, Response, RichText, Sense,
    Stroke, Ui, pos2, vec2,
};

use crate::autostart;
use crate::theme;

pub struct Settings {
    /// État réel du lancement automatique, relu dans Windows à chaque changement.
    autostart: Result<bool, String>,
    error: Option<String>,
}

impl Settings {
    pub fn open() -> Self {
        Self {
            autostart: autostart::is_enabled(),
            error: None,
        }
    }

    /// Affiche la fenêtre ; renvoie `false` quand elle doit se fermer.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut keep_open = true;
        let frame = Frame::new()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(16)
            .inner_margin(Margin::same(24));
        let response = egui::Modal::new(Id::unique("parametres"))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(70))
            .show(ctx, |ui| {
                ui.set_width(440.0);
                ui.label(RichText::new("Paramètres").font(theme::bold(20.0)).color(theme::TEXT));
                ui.add_space(14.0);
                self.autostart_row(ui);
                ui.add_space(18.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Fermer").clicked() {
                        keep_open = false;
                    }
                });
            });
        keep_open && !response.should_close()
    }

    fn autostart_row(&mut self, ui: &mut Ui) {
        let available = self.autostart.is_ok();
        let on = matches!(self.autostart, Ok(true));
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(360.0);
                ui.label(
                    RichText::new("Lancer au démarrage de Windows")
                        .font(theme::bold(15.0))
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new("Planif'Week s'ouvre tout seul quand vous ouvrez votre session.")
                        .color(theme::MUTED)
                        .size(13.5),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let resp = toggle(ui, on, available);
                if resp.clicked() && available {
                    self.error = autostart::set_enabled(!on).err();
                    self.autostart = autostart::is_enabled();
                }
            });
        });
        let problem = self.error.as_ref().or(self.autostart.as_ref().err());
        if let Some(message) = problem {
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("Impossible de modifier ce réglage : {message}"))
                    .color(theme::DANGER)
                    .size(13.0),
            );
        }
    }
}

/// Interrupteur coulissant.
fn toggle(ui: &mut Ui, on: bool, enabled: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(46.0, 26.0), Sense::click());
    let t = ui.ctx().animate_bool_responsive(resp.id, on);
    let off_color = Color32::from_rgb(0xD3, 0xD8, 0xE1);
    let mut track = lerp_color(off_color, theme::ACCENT, t);
    if !enabled {
        track = track.gamma_multiply(0.5);
    } else if resp.hovered() {
        track = theme::darken(track, 0.06);
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(13), track);
    let x = rect.left() + 13.0 + t * (rect.width() - 26.0);
    painter.circle_filled(pos2(x, rect.center().y + 0.5), 10.0, Color32::from_black_alpha(25));
    painter.circle_filled(pos2(x, rect.center().y), 10.0, Color32::WHITE);
    resp
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}
