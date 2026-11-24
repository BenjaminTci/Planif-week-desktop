//! Dessin d'une carte de tâche.

use std::f32::consts::TAU;
use std::sync::Arc;

use chrono::NaiveTime;
use eframe::egui::{
    Color32, CornerRadius, Galley, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, pos2,
    text::{LayoutJob, TextFormat},
    vec2,
};

use crate::dates::format_time;
use crate::model::{Task, TaskColor};
use crate::theme;

pub const MIN_HEIGHT: f32 = 46.0;
pub const RADIUS: u8 = 10;
const PAD_Y: f32 = 11.0;
const TEXT_LEFT: f32 = 40.0;
const PAD_RIGHT: f32 = 12.0;
const CHECK_CENTER_X: f32 = 21.0;
const CHECK_R: f32 = 9.0;
const ICON_W: f32 = 14.0;

/// Ce qu'il faut pour dessiner une carte (copié depuis la tâche).
#[derive(Clone, Debug)]
pub struct CardView {
    pub title: String,
    pub time: Option<NaiveTime>,
    pub color: TaskColor,
    pub done: bool,
    pub recurring: bool,
}

impl CardView {
    pub fn new(task: &Task, done: bool) -> Self {
        Self {
            title: task.title.clone(),
            time: task.time,
            color: task.color,
            done,
            recurring: task.is_recurring(),
        }
    }
}

pub struct CardLayout {
    title: Arc<Galley>,
    time: Option<Arc<Galley>>,
    pub height: f32,
}

fn fade(c: Color32, alpha: f32) -> Color32 {
    if alpha >= 1.0 { c } else { c.gamma_multiply(alpha) }
}

pub fn layout(painter: &Painter, view: &CardView, width: f32, alpha: f32) -> CardLayout {
    let (_, accent) = theme::task_colors(view.color);
    let time = view.time.map(|t| {
        let color = if view.done { theme::DONE_FG } else { accent };
        painter.layout_no_wrap(format_time(t), theme::bold(13.0), fade(color, alpha))
    });
    let mut right = PAD_RIGHT;
    if let Some(g) = &time {
        right += g.size().x + 8.0;
    }
    if view.recurring {
        right += ICON_W + 6.0;
    }
    let wrap = (width - TEXT_LEFT - right).max(40.0);

    let text_color = if view.done { theme::DONE_FG } else { theme::TEXT };
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    job.append(
        &view.title,
        0.0,
        TextFormat {
            font_id: theme::regular(15.0),
            color: fade(text_color, alpha),
            strikethrough: if view.done {
                Stroke::new(1.2, fade(theme::DONE_FG, alpha))
            } else {
                Stroke::NONE
            },
            ..Default::default()
        },
    );
    let title = painter.layout_job(job);
    let height = (title.size().y + 2.0 * PAD_Y).max(MIN_HEIGHT);
    CardLayout { title, time, height }
}

/// Zone cliquable du rond « terminée ».
pub fn check_rect(rect: Rect) -> Rect {
    Rect::from_center_size(
        pos2(rect.left() + CHECK_CENTER_X, rect.center().y),
        Vec2::splat(CHECK_R * 2.0 + 8.0),
    )
}

pub struct PaintState {
    pub hovered: bool,
    pub check_hovered: bool,
    pub alpha: f32,
}

pub fn paint(painter: &Painter, rect: Rect, view: &CardView, layout: &CardLayout, state: PaintState) {
    let (bg, accent) = theme::task_colors(view.color);
    let (mut fill, ring) = if view.done {
        (theme::DONE_BG, theme::DONE_FG)
    } else {
        (bg, accent)
    };
    if state.hovered {
        fill = theme::darken(fill, 0.03);
    }
    let alpha = state.alpha;
    painter.rect_filled(rect, CornerRadius::same(RADIUS), fade(fill, alpha));

    // Rond à cocher.
    let c = pos2(rect.left() + CHECK_CENTER_X, rect.center().y);
    if view.done {
        painter.circle_filled(c, CHECK_R, fade(ring, alpha));
        paint_check_mark(painter, c, fade(Color32::WHITE, alpha), 1.8);
    } else {
        if state.check_hovered {
            painter.circle_filled(c, CHECK_R, fade(theme::darken(bg, 0.06), alpha));
            paint_check_mark(painter, c, fade(ring, alpha * 0.8), 1.6);
        }
        painter.circle_stroke(c, CHECK_R, Stroke::new(1.8, fade(ring, alpha)));
    }

    // Titre, centré verticalement.
    let title_pos = pos2(
        rect.left() + TEXT_LEFT,
        rect.center().y - layout.title.size().y / 2.0,
    );
    painter.galley(title_pos, layout.title.clone(), theme::TEXT);

    // Heure et icône de récurrence, à droite.
    let mut x = rect.right() - PAD_RIGHT;
    if let Some(time) = &layout.time {
        let pos = pos2(x - time.size().x, rect.center().y - time.size().y / 2.0);
        painter.galley(pos, time.clone(), accent);
        x -= time.size().x + 8.0;
    }
    if view.recurring {
        let center = pos2(x - ICON_W / 2.0, rect.center().y);
        let color = if view.done { theme::DONE_FG } else { accent };
        paint_repeat_icon(painter, center, fade(color, alpha * 0.85));
    }
}

fn paint_check_mark(painter: &Painter, c: Pos2, color: Color32, width: f32) {
    painter.add(Shape::line(
        vec![c + vec2(-4.0, 0.2), c + vec2(-1.2, 3.0), c + vec2(4.2, -3.0)],
        Stroke::new(width, color),
    ));
}

/// Petite flèche circulaire : « se répète chaque semaine ».
pub fn paint_repeat_icon(painter: &Painter, center: Pos2, color: Color32) {
    let r = 5.0;
    let start = 0.35 * TAU;
    let sweep = 0.78 * TAU;
    let points: Vec<Pos2> = (0..=20)
        .map(|i| {
            let a = start + sweep * i as f32 / 20.0;
            center + r * vec2(a.cos(), a.sin())
        })
        .collect();
    let end = *points.last().unwrap();
    painter.add(Shape::line(points, Stroke::new(1.5, color)));
    // Pointe de flèche dans le sens du tracé.
    let a = start + sweep;
    let tangent = vec2(-a.sin(), a.cos());
    let normal = vec2(a.cos(), a.sin());
    painter.add(Shape::convex_polygon(
        vec![end + tangent * 3.2, end + normal * 2.8, end - normal * 2.8],
        color,
        Stroke::NONE,
    ));
}

/// Contour en pointillés de l'emplacement d'une carte en cours de déplacement.
pub fn paint_placeholder(painter: &Painter, rect: Rect) {
    painter.rect(
        rect,
        CornerRadius::same(RADIUS),
        Color32::from_rgb(0xF7, 0xF8, 0xFB),
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );
}
