// Pas de fenêtre de console derrière l'application en version finale.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod card;
mod dates;
mod editor;
mod model;
mod settings;
mod storage;
mod theme;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Planif'Week")
            .with_inner_size([1360.0, 900.0])
            .with_min_inner_size([980.0, 660.0])
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Planif'Week",
        options,
        Box::new(|cc| Ok(Box::new(app::PlanifApp::new(cc)))),
    )
}

/// Icône dessinée au lancement : un petit calendrier bleu.
fn app_icon() -> egui::IconData {
    const N: u32 = 64;
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    let rounded = |x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
        let cx = x.clamp(x0 + r, x1 - r);
        let cy = y.clamp(y0 + r, y1 - r);
        (x - cx).powi(2) + (y - cy).powi(2) <= r * r
    };
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let pixel = if !rounded(fx, fy, 2.0, 4.0, 62.0, 62.0, 12.0) {
                [0, 0, 0, 0]
            } else if fy < 20.0 {
                [0x2F, 0x6B, 0xF2, 255]
            } else if rounded(fx, fy, 12.0, 28.0, 52.0, 52.0, 3.0)
                && ((fx - 12.0) % 14.0 < 10.0)
                && ((fy - 28.0) % 13.0 < 9.0)
            {
                [0xE3, 0xEB, 0xFE, 255]
            } else {
                [255, 255, 255, 255]
            };
            rgba.extend_from_slice(&pixel);
        }
    }
    egui::IconData { rgba, width: N, height: N }
}
