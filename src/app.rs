//! Fenêtre principale : en-tête, semaine, zone « Un jour », glisser-déposer.

use std::collections::BTreeSet;
use std::time::Duration as StdDuration;

use chrono::{Datelike, Duration, Local, NaiveDate};
use eframe::egui::{
    self, Align, Align2, Button, Color32, CornerRadius, CursorIcon, Frame, Id, Key, LayerId,
    Layout, Margin, Order, Painter, Pos2, Rect, RichText, ScrollArea, Sense, Shape, Stroke,
    StrokeKind, TextEdit, Ui, UiBuilder, Vec2, pos2, vec2,
};

use crate::card::{self, CardLayout, CardView, PaintState};
use crate::dates::{self, monday_of, weekday_index};
use crate::editor::{Edit, Editor, Kind, Outcome};
use crate::model::{OccKey, Planner, Schedule, Slot, TaskColor};
use crate::settings::Settings;
use crate::storage::Storage;
use crate::theme;

const MARGIN_X: f32 = 32.0;
const HEADER_H: f32 = 62.0;
const GAP: f32 = 16.0;
const ROUND: u8 = 14;
const DAY_HEADER_H: f32 = 58.0;
const ADD_ROW_H: f32 = 42.0;
const COL_PAD: f32 = 11.0;
const CARD_GAP: f32 = 8.0;
const LINE_STEP: f32 = 44.0;
const SOMEDAY_PAD: f32 = 20.0;
const SOMEDAY_HEAD: f32 = 24.0;
const SOMEDAY_MIN_CARD: f32 = 210.0;
const SOMEDAY_GAP: f32 = 10.0;
const HISTORY_LIMIT: usize = 100;
/// Temps de survol d'une flèche, pendant un glisser, avant de changer de semaine.
const NAV_HOVER_SECS: f64 = 0.7;

enum Action {
    Toggle(OccKey),
    Edit(OccKey),
    Delete(u64),
    Skip(u64, NaiveDate),
    Add(Slot, String),
    Move(OccKey, Slot, usize),
    Undo,
}

struct DragState {
    key: OccKey,
    view: CardView,
    grab: Vec2,
    size: Vec2,
}

/// Une colonne qui accepte les cartes déposées, mesurée pendant l'image en cours.
struct DropZone {
    slot: Slot,
    hit: Rect,
    list: Rect,
    items: Vec<(OccKey, Rect)>,
    horizontal: bool,
}

struct DropTarget {
    slot: Slot,
    index: usize,
    line: [Pos2; 2],
    clip: Rect,
}

struct Adding {
    slot: Slot,
    /// Saisie ouverte dans la liste (clic sur une ligne vide) plutôt qu'en bas de la colonne.
    in_list: bool,
    text: String,
    focus: bool,
}

struct Toast {
    text: String,
    error: bool,
    undo: bool,
    shown_at: Option<f64>,
}

impl Toast {
    fn info(text: impl Into<String>, undo: bool) -> Self {
        Self { text: text.into(), error: false, undo, shown_at: None }
    }

    fn error(text: impl Into<String>) -> Self {
        Self { text: text.into(), error: true, undo: false, shown_at: None }
    }
}

struct SomedayLayout {
    cards: Vec<(OccKey, CardView, CardLayout, Rect)>,
    add: Rect,
    grid_h: f32,
    total_h: f32,
}

pub struct PlanifApp {
    planner: Planner,
    storage: Storage,
    monday: NaiveDate,
    today: NaiveDate,
    dirty: bool,
    save_error: Option<String>,
    history: Vec<Planner>,
    actions: Vec<Action>,
    drag: Option<DragState>,
    zones: Vec<DropZone>,
    nav_targets: Vec<(Rect, i64)>,
    nav_hover: Option<(i64, f64)>,
    adding: Option<Adding>,
    editor: Option<Editor>,
    settings: Option<Settings>,
    toast: Option<Toast>,
}

impl PlanifApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        theme::install_style(&cc.egui_ctx);
        let storage = Storage::new(Storage::default_path());
        let (planner, warning) = storage.load();
        let today = Local::now().date_naive();
        Self {
            planner,
            storage,
            monday: monday_of(today),
            today,
            dirty: false,
            save_error: None,
            history: Vec::new(),
            actions: Vec::new(),
            drag: None,
            zones: Vec::new(),
            nav_targets: Vec::new(),
            nav_hover: None,
            adding: None,
            editor: None,
            settings: None,
            toast: warning.map(Toast::error),
        }
    }

    // ----- Modifications des données -------------------------------------------------

    /// Applique une modification, la rend annulable et demande l'enregistrement.
    fn commit(&mut self, change: impl FnOnce(&mut Planner)) {
        let before = self.planner.clone();
        change(&mut self.planner);
        if self.planner != before {
            self.history.push(before);
            if self.history.len() > HISTORY_LIMIT {
                self.history.remove(0);
            }
            self.dirty = true;
        }
    }

    fn apply_actions(&mut self) {
        for action in std::mem::take(&mut self.actions) {
            match action {
                Action::Toggle(key) => self.commit(|p| p.toggle_done(key)),
                Action::Edit(key) => {
                    if let Some(task) = self.planner.task(key.task_id) {
                        self.adding = None;
                        self.editor = Some(Editor::new(task, key, self.today));
                    }
                }
                Action::Delete(id) => {
                    self.commit(|p| p.remove(id));
                    self.toast = Some(Toast::info("Tâche supprimée", true));
                }
                Action::Skip(id, date) => {
                    self.commit(|p| p.skip_occurrence(id, date));
                    self.toast = Some(Toast::info("Occurrence retirée de ce jour", true));
                }
                Action::Add(slot, text) => {
                    let (title, time) = dates::split_trailing_time(&text);
                    let color = TaskColor::ALL[(self.planner.next_id as usize) % TaskColor::ALL.len()];
                    let schedule = match slot {
                        Slot::Day(date) => Schedule::Date { date },
                        Slot::Someday => Schedule::Someday,
                    };
                    self.commit(|p| {
                        p.add(title, time, color, schedule);
                    });
                }
                Action::Move(key, slot, index) => self.commit(|p| p.move_item(key, slot, index)),
                Action::Undo => {
                    if let Some(previous) = self.history.pop() {
                        self.planner = previous;
                        self.dirty = true;
                        self.toast = Some(Toast::info("Action annulée", false));
                    }
                }
            }
        }
    }

    fn save_if_dirty(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        match self.storage.save(&self.planner) {
            Ok(()) => self.save_error = None,
            Err(e) => {
                let message = format!(
                    "Enregistrement impossible dans {} : {e}",
                    self.storage.path().display()
                );
                if self.save_error.as_deref() != Some(message.as_str()) {
                    self.toast = Some(Toast::error(message.clone()));
                }
                self.save_error = Some(message);
            }
        }
    }

    fn go_to_week(&mut self, monday: NaiveDate) {
        self.monday = monday;
        self.adding = None;
    }

    // ----- En-tête -------------------------------------------------------------------

    fn header(&mut self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter().clone();
        let title_rect = painter.text(
            rect.left_top(),
            Align2::LEFT_TOP,
            dates::header_title(self.monday),
            theme::bold(29.0),
            theme::TEXT,
        );

        // Pastille « Semaine 48 ».
        let badge_text = painter.layout_no_wrap(
            format!("Semaine {}", dates::iso_week(self.monday)),
            theme::bold(13.5),
            theme::ACCENT,
        );
        let badge = Rect::from_min_size(
            pos2(title_rect.right() + 14.0, title_rect.center().y - 12.0),
            vec2(badge_text.size().x + 20.0, 24.0),
        );
        painter.rect_filled(badge, CornerRadius::same(12), theme::ACCENT_SOFT);
        painter.galley(badge.center() - badge_text.size() / 2.0, badge_text, theme::ACCENT);

        let subtitle = format!(
            "{} · {}",
            dates::range_label(self.monday),
            dates::relative_label(self.monday, self.today)
        );
        let sub_rect = painter.text(
            pos2(rect.left(), title_rect.bottom() + 4.0),
            Align2::LEFT_TOP,
            subtitle,
            theme::regular(15.5),
            theme::MUTED,
        );
        if self.save_error.is_some() {
            painter.text(
                pos2(sub_rect.right() + 12.0, sub_rect.top()),
                Align2::LEFT_TOP,
                "· Modifications non enregistrées",
                theme::bold(14.5),
                theme::DANGER,
            );
        }

        // Boutons, de droite à gauche.
        let btn_h = 46.0;
        let top = rect.top() + 4.0;
        let gear = Rect::from_min_size(pos2(rect.right() - btn_h, top), vec2(btn_h, btn_h));
        let resp = ui
            .interact(gear, Id::unique("parametres-bouton"), Sense::click())
            .on_hover_text("Paramètres");
        let fill = if resp.hovered() { Color32::from_rgb(0xF3, 0xF5, 0xF9) } else { theme::SURFACE };
        painter.rect(gear, CornerRadius::same(10), fill, Stroke::new(1.0, theme::BORDER), StrokeKind::Inside);
        paint_gear(&painter, gear.center(), theme::TEXT, fill);
        if resp.clicked() {
            self.adding = None;
            self.settings = Some(Settings::open());
        }

        let nav = Rect::from_min_size(pos2(gear.left() - 12.0 - 100.0, top), vec2(100.0, btn_h));
        let prev = Rect::from_min_max(nav.min, pos2(nav.center().x, nav.bottom()));
        let next = Rect::from_min_max(pos2(nav.center().x, nav.top()), nav.max);
        painter.rect(nav, CornerRadius::same(10), theme::SURFACE, Stroke::new(1.0, theme::BORDER), StrokeKind::Inside);
        for (half, dir, name) in [(prev, -1i64, "Semaine précédente"), (next, 1, "Semaine suivante")] {
            let resp = ui
                .interact(half, Id::unique(("nav", dir)), Sense::click())
                .on_hover_text(format!("{name} (flèche {})", if dir < 0 { "gauche" } else { "droite" }));
            if resp.hovered() && self.drag.is_none() {
                let corners = if dir < 0 {
                    CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }
                } else {
                    CornerRadius { nw: 0, sw: 0, ne: 10, se: 10 }
                };
                painter.rect_filled(half.shrink(1.0), corners, Color32::from_rgb(0xF3, 0xF5, 0xF9));
            }
            if self.drag.is_some() && self.nav_hover.is_some_and(|(d, _)| d == dir) {
                painter.rect_filled(half.shrink(1.0), CornerRadius::same(9), theme::ACCENT_SOFT);
            }
            paint_chevron(&painter, half.center(), dir < 0, theme::TEXT);
            if resp.clicked() {
                self.go_to_week(self.monday + Duration::days(7 * dir));
            }
            self.nav_targets.push((half, dir));
        }
        painter.vline(nav.center().x, nav.y_range().shrink(1.0), Stroke::new(1.0, theme::BORDER));

        let today_btn = Rect::from_min_size(pos2(nav.left() - 12.0 - 122.0, top), vec2(122.0, btn_h));
        let resp = ui
            .interact(today_btn, Id::unique("aujourdhui"), Sense::click())
            .on_hover_text("Revenir à la semaine en cours (touche Origine)");
        let fill = if resp.hovered() { Color32::from_rgb(0xF3, 0xF5, 0xF9) } else { theme::SURFACE };
        painter.rect(today_btn, CornerRadius::same(10), fill, Stroke::new(1.0, theme::BORDER), StrokeKind::Inside);
        painter.text(today_btn.center(), Align2::CENTER_CENTER, "Aujourd'hui", theme::bold(15.0), theme::TEXT);
        if resp.clicked() {
            self.go_to_week(monday_of(self.today));
        }

        // Avancement de la semaine.
        let (done, total) = self.planner.week_progress(self.monday);
        let bar = Rect::from_min_size(
            pos2(today_btn.left() - 24.0 - 128.0, today_btn.center().y - 3.0),
            vec2(128.0, 6.0),
        );
        if total > 0 {
            painter.rect_filled(bar, CornerRadius::same(3), theme::LINE);
            let ratio = done as f32 / total as f32;
            if ratio > 0.0 {
                let filled = Rect::from_min_size(bar.min, vec2((bar.width() * ratio).max(6.0), bar.height()));
                painter.rect_filled(filled, CornerRadius::same(3), theme::ACCENT);
            }
            let label = painter.layout_no_wrap(" terminées".into(), theme::regular(14.5), theme::MUTED);
            let count = painter.layout_no_wrap(format!("{done}/{total}"), theme::bold(14.5), theme::TEXT);
            let x = bar.left() - 14.0 - label.size().x;
            let y = bar.center().y - label.size().y / 2.0;
            painter.galley(pos2(x - count.size().x, y), count, theme::TEXT);
            painter.galley(pos2(x, y), label, theme::MUTED);
        } else {
            painter.text(
                pos2(today_btn.left() - 24.0, today_btn.center().y),
                Align2::RIGHT_CENTER,
                "Aucune tâche cette semaine",
                theme::regular(14.5),
                theme::MUTED,
            );
        }
    }

    // ----- Semaine -------------------------------------------------------------------

    fn week(&mut self, ui: &mut Ui, rect: Rect) {
        let days = dates::week_days(self.monday);
        let col_w = (rect.width() - 5.0 * GAP) / 6.0;
        let column = |i: usize| {
            Rect::from_min_size(
                pos2(rect.left() + i as f32 * (col_w + GAP), rect.top()),
                vec2(col_w, rect.height()),
            )
        };
        for (i, &date) in days.iter().enumerate().take(5) {
            let col = column(i);
            column_frame(ui.painter(), col, date == self.today);
            self.day_section(ui, col, date);
        }

        // Samedi et dimanche partagent la dernière colonne.
        let col = column(5);
        column_frame(ui.painter(), col, false);
        let mid = col.center().y;
        let sat = Rect::from_min_max(col.min, pos2(col.right(), mid));
        let sun = Rect::from_min_max(pos2(col.left(), mid), col.max);
        ui.painter().hline(col.x_range(), mid, Stroke::new(1.0, theme::BORDER));
        for (half, date) in [(sat, days[5]), (sun, days[6])] {
            if date == self.today {
                ui.painter().rect_stroke(
                    half,
                    CornerRadius::same(ROUND),
                    Stroke::new(1.5, theme::ACCENT),
                    StrokeKind::Inside,
                );
            }
            self.day_section(ui, half, date);
        }
    }

    fn day_section(&mut self, ui: &mut Ui, rect: Rect, date: NaiveDate) {
        let slot = Slot::Day(date);
        let header = Rect::from_min_size(rect.min, vec2(rect.width(), DAY_HEADER_H));
        self.day_header(ui.painter(), header, date);

        let add = Rect::from_min_max(
            pos2(rect.left() + COL_PAD, rect.bottom() - ADD_ROW_H - 8.0),
            pos2(rect.right() - COL_PAD, rect.bottom() - 8.0),
        );
        let list = Rect::from_min_max(
            pos2(rect.left() + COL_PAD, header.bottom()),
            pos2(rect.right() - COL_PAD, add.top() - 4.0),
        );
        let items = self.card_list(ui, list, slot);
        self.add_row(ui, add, slot);
        self.zones.push(DropZone { slot, hit: rect, list, items, horizontal: false });
    }

    fn day_header(&self, painter: &Painter, rect: Rect, date: NaiveDate) {
        let is_today = date == self.today;
        let weekend = weekday_index(date) >= 5;
        let x = rect.left() + 20.0;
        let cy = rect.top() + 33.0;
        let number = date.day().to_string();

        let after_number = if is_today {
            let center = pos2(x + 13.0, cy);
            painter.circle_filled(center, 17.0, theme::ACCENT);
            painter.text(center, Align2::CENTER_CENTER, number, theme::bold(19.0), Color32::WHITE);
            center.x + 17.0
        } else {
            let color = if weekend { Color32::from_rgb(0x55, 0x5C, 0x69) } else { theme::TEXT };
            painter.text(pos2(x, cy), Align2::LEFT_CENTER, number, theme::bold(21.0), color).right()
        };
        let day_color = if is_today { theme::ACCENT } else { theme::MUTED };
        let day = painter.text(
            pos2(after_number + 12.0, cy + 1.0),
            Align2::LEFT_CENTER,
            dates::WEEKDAY_SHORT[weekday_index(date) as usize],
            theme::bold(13.5),
            day_color,
        );
        if is_today {
            let text = painter.layout_no_wrap("Aujourd'hui".into(), theme::bold(13.0), theme::ACCENT);
            let badge = Rect::from_min_size(
                pos2(day.right() + 9.0, cy - 11.0),
                vec2(text.size().x + 16.0, 22.0),
            );
            if badge.right() < rect.right() - 6.0 {
                painter.rect_filled(badge, CornerRadius::same(11), theme::ACCENT_SOFT);
                painter.galley(badge.center() - text.size() / 2.0, text, theme::ACCENT);
            }
        }
    }

    /// Les cartes d'une colonne verticale, avec défilement si elles débordent.
    fn card_list(&mut self, ui: &mut Ui, list: Rect, slot: Slot) -> Vec<(OccKey, Rect)> {
        let cards: Vec<(OccKey, CardView)> = self
            .planner
            .items(slot)
            .into_iter()
            .filter_map(|k| {
                let task = self.planner.task(k.task_id)?;
                Some((k, CardView::new(task, self.planner.is_done(k))))
            })
            .collect();

        let mut placed = Vec::with_capacity(cards.len());
        // Bas du contenu (cartes et éventuelle saisie), là où commencent les lignes vides.
        let mut content_bottom = list.top();
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(("colonne", slot))
                .max_rect(list)
                .layout(Layout::top_down(Align::Min)),
        );
        ScrollArea::vertical()
            .id_salt(("defilement", slot))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.spacing_mut().item_spacing.y = CARD_GAP;
                let width = ui.available_width();
                for (key, view) in &cards {
                    let layout = card::layout(ui.painter(), view, width, 1.0);
                    let (rect, _) = ui.allocate_exact_size(vec2(width, layout.height), Sense::hover());
                    self.show_card(ui, rect, *key, view, &layout);
                    placed.push((*key, rect));
                    content_bottom = rect.bottom();
                }
                // Saisie ouverte par un clic sur une ligne vide : juste sous les cartes,
                // là où la nouvelle tâche apparaîtra.
                if self.adding.as_ref().is_some_and(|a| a.slot == slot && a.in_list) {
                    let (rect, _) = ui.allocate_exact_size(vec2(width, LINE_STEP - 6.0), Sense::hover());
                    self.adding_field(ui, rect, slot);
                    content_bottom = rect.bottom();
                }
            });

        // Lignes de cahier dans l'espace libre.
        let painter = ui.painter_at(list);
        let mut y = content_bottom + LINE_STEP;
        while y < list.bottom() - 2.0 {
            painter.hline(list.x_range(), y, Stroke::new(1.0, theme::LINE));
            y += LINE_STEP;
        }

        // Un clic sur une ligne vide ouvre la saisie à la suite des cartes.
        let adding_here = self.adding.as_ref().is_some_and(|a| a.slot == slot && a.in_list);
        let empty = Rect::from_min_max(pos2(list.left(), content_bottom.max(list.top())), list.max);
        if !adding_here && empty.height() > 12.0 {
            let resp = ui.interact(empty, Id::unique(("ligne-vide", slot)), Sense::click());
            if resp.hovered() && self.drag.is_none() {
                let hint = Rect::from_min_size(
                    pos2(list.left(), content_bottom.max(list.top()) + 4.0),
                    vec2(list.width(), LINE_STEP - 8.0),
                )
                .intersect(empty);
                painter.rect_filled(hint, CornerRadius::same(8), Color32::from_rgb(0xF4, 0xF6, 0xF9));
                painter.text(
                    hint.left_center() + vec2(6.0, 0.0),
                    Align2::LEFT_CENTER,
                    "+ Ajouter une tâche",
                    theme::regular(14.5),
                    theme::MUTED,
                );
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if resp.clicked() && self.drag.is_none() {
                self.start_adding(slot, true);
            }
        }
        placed
    }

    /// Ouvre une saisie ; celle déjà ouverte ailleurs est enregistrée si elle contient du texte.
    fn start_adding(&mut self, slot: Slot, in_list: bool) {
        if let Some(previous) = self.adding.take() {
            let text = previous.text.trim().to_owned();
            if !text.is_empty() {
                self.actions.push(Action::Add(previous.slot, text));
            }
        }
        self.adding = Some(Adding { slot, in_list, text: String::new(), focus: true });
    }

    fn show_card(&mut self, ui: &mut Ui, rect: Rect, key: OccKey, view: &CardView, layout: &CardLayout) {
        let id = Id::unique(("carte", key.task_id, key.date));
        let resp = ui.interact(rect, id, Sense::click_and_drag());
        let check = ui.interact(card::check_rect(rect), id.with("rond"), Sense::click());

        if self.drag.as_ref().is_some_and(|d| d.key == key) {
            card::paint_placeholder(ui.painter(), rect);
            return;
        }
        let dragging = self.drag.is_some();
        if resp.drag_started() {
            let grab = resp.interact_pointer_pos().unwrap_or(rect.center()) - rect.min;
            self.adding = None;
            self.drag = Some(DragState { key, view: view.clone(), grab, size: rect.size() });
        }
        if check.clicked() {
            self.actions.push(Action::Toggle(key));
        } else if resp.clicked() {
            self.actions.push(Action::Edit(key));
        }
        if !dragging {
            if check.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            } else if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
        resp.context_menu(|ui| self.card_menu(ui, key, view));

        card::paint(
            ui.painter(),
            rect,
            view,
            layout,
            PaintState {
                hovered: !dragging && (resp.hovered() || check.hovered()),
                check_hovered: !dragging && check.hovered(),
                alpha: 1.0,
            },
        );
    }

    fn card_menu(&mut self, ui: &mut Ui, key: OccKey, view: &CardView) {
        ui.set_min_width(190.0);
        let mut chosen = None;
        if ui.button("Modifier…").clicked() {
            chosen = Some(Action::Edit(key));
        }
        let toggle = if view.done { "Marquer comme à faire" } else { "Marquer comme terminée" };
        if ui.button(toggle).clicked() {
            chosen = Some(Action::Toggle(key));
        }
        ui.separator();
        if view.recurring {
            if let Some(date) = key.date
                && ui.button("Retirer ce jour-là").clicked() {
                    chosen = Some(Action::Skip(key.task_id, date));
                }
            if ui.button(RichText::new("Supprimer la série").color(theme::DANGER)).clicked() {
                chosen = Some(Action::Delete(key.task_id));
            }
        } else if ui.button(RichText::new("Supprimer").color(theme::DANGER)).clicked() {
            chosen = Some(Action::Delete(key.task_id));
        }
        if let Some(action) = chosen {
            self.actions.push(action);
            ui.close();
        }
    }

    /// « + Ajouter une tâche », ou le champ de saisie quand on est en train d'ajouter.
    fn add_row(&mut self, ui: &mut Ui, rect: Rect, slot: Slot) {
        if self.adding.as_ref().is_some_and(|a| a.slot == slot && !a.in_list) {
            self.adding_field(ui, rect, slot);
            return;
        }

        let resp = ui.interact(rect, Id::unique(("ajouter", slot)), Sense::click());
        let painter = ui.painter();
        let hovered = resp.hovered() && self.drag.is_none();
        if hovered {
            painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(0xF4, 0xF6, 0xF9));
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, theme::LINE));
        painter.text(
            rect.left_center() + vec2(6.0, 0.0),
            Align2::LEFT_CENTER,
            "+ Ajouter une tâche",
            theme::regular(14.5),
            if hovered { theme::MUTED } else { theme::FAINT },
        );
        if resp.clicked() {
            self.start_adding(slot, false);
        }
    }

    /// Champ de saisie d'une nouvelle tâche : Entrée ajoute et garde le champ ouvert,
    /// Échap annule, cliquer ailleurs ajoute le texte saisi puis ferme.
    fn adding_field(&mut self, ui: &mut Ui, rect: Rect, slot: Slot) {
        if let Some(adding) = self.adding.as_mut().filter(|a| a.slot == slot) {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .id_salt(("saisie", slot))
                    .max_rect(rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let response = TextEdit::singleline(&mut adding.text)
                .hint_text("Nouvelle tâche, puis Entrée")
                .desired_width(rect.width())
                .margin(vec2(10.0, 8.0))
                .show(&mut child)
                .response;
            if adding.focus {
                response.request_focus();
                response.scroll_to_me(None);
                adding.focus = false;
            }
            if response.lost_focus() {
                let (enter, escape) = ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
                let text = adding.text.trim().to_owned();
                if escape {
                    self.adding = None;
                } else if enter && !text.is_empty() {
                    // On reste en saisie pour enchaîner plusieurs tâches.
                    adding.text.clear();
                    adding.focus = true;
                    self.actions.push(Action::Add(slot, text));
                } else {
                    if !text.is_empty() {
                        self.actions.push(Action::Add(slot, text));
                    }
                    self.adding = None;
                }
            }
        }
    }

    // ----- Un jour -------------------------------------------------------------------

    fn someday_layout(&self, painter: &Painter, width: f32) -> SomedayLayout {
        let inner = width - 2.0 * SOMEDAY_PAD;
        let per_row = (((inner + SOMEDAY_GAP) / (SOMEDAY_MIN_CARD + SOMEDAY_GAP)).floor() as usize).max(1);
        let card_w = (inner - (per_row - 1) as f32 * SOMEDAY_GAP) / per_row as f32;

        let mut sized: Vec<(OccKey, CardView, CardLayout)> = Vec::new();
        for key in self.planner.items(Slot::Someday) {
            if let Some(task) = self.planner.task(key.task_id) {
                let view = CardView::new(task, self.planner.is_done(key));
                let layout = card::layout(painter, &view, card_w, 1.0);
                sized.push((key, view, layout));
            }
        }

        // Rangées : les cartes, puis le bouton d'ajout ; centrées verticalement dans leur rangée.
        let count = sized.len() + 1;
        let heights: Vec<f32> = sized.iter().map(|(_, _, l)| l.height).collect();
        let height_of = |i: usize| heights.get(i).copied().unwrap_or(card::MIN_HEIGHT);
        let mut rows_top = Vec::new();
        let mut rows_h = Vec::new();
        let mut y = 0.0;
        for row_start in (0..count).step_by(per_row) {
            let h = (row_start..(row_start + per_row).min(count))
                .map(height_of)
                .fold(0.0, f32::max);
            rows_top.push(y);
            rows_h.push(h);
            y += h + SOMEDAY_GAP;
        }
        let grid_h = y - SOMEDAY_GAP;
        let rect_of = |i: usize| {
            let row = i / per_row;
            let h = height_of(i);
            Rect::from_min_size(
                pos2(
                    (i % per_row) as f32 * (card_w + SOMEDAY_GAP),
                    rows_top[row] + (rows_h[row] - h) / 2.0,
                ),
                vec2(card_w, h),
            )
        };
        let add = rect_of(count - 1);
        let cards = sized
            .into_iter()
            .enumerate()
            .map(|(i, (k, v, l))| (k, v, l, rect_of(i)))
            .collect();
        SomedayLayout {
            cards,
            add,
            grid_h,
            total_h: SOMEDAY_PAD + SOMEDAY_HEAD + 14.0 + grid_h + SOMEDAY_PAD,
        }
    }

    fn someday(&mut self, ui: &mut Ui, rect: Rect, layout: SomedayLayout) {
        let painter = ui.painter().clone();
        column_frame(&painter, rect, false);
        painter.text(
            rect.left_top() + vec2(SOMEDAY_PAD, SOMEDAY_PAD),
            Align2::LEFT_TOP,
            "Un jour",
            theme::bold(17.0),
            theme::TEXT,
        );
        painter.text(
            pos2(rect.right() - SOMEDAY_PAD, rect.top() + SOMEDAY_PAD + 3.0),
            Align2::RIGHT_TOP,
            "Les tâches sans date, à caser quand vous aurez le temps",
            theme::regular(14.0),
            theme::MUTED,
        );

        let grid = Rect::from_min_max(
            pos2(rect.left() + SOMEDAY_PAD, rect.top() + SOMEDAY_PAD + SOMEDAY_HEAD + 14.0),
            pos2(rect.right() - SOMEDAY_PAD, rect.bottom() - SOMEDAY_PAD + 6.0),
        );
        let mut items = Vec::with_capacity(layout.cards.len());
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt("un-jour")
                .max_rect(grid)
                .layout(Layout::top_down(Align::Min)),
        );
        ScrollArea::vertical()
            .id_salt("defilement-un-jour")
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                let (area, _) = ui.allocate_exact_size(vec2(grid.width(), layout.grid_h), Sense::hover());
                let offset = area.min.to_vec2();
                for (key, view, card_layout, rel) in &layout.cards {
                    let rect = rel.translate(offset);
                    self.show_card(ui, rect, *key, view, card_layout);
                    items.push((*key, rect));
                }
                self.add_row(ui, layout.add.translate(offset), Slot::Someday);
            });
        self.zones.push(DropZone {
            slot: Slot::Someday,
            hit: rect,
            list: grid,
            items,
            horizontal: true,
        });
    }

    // ----- Glisser-déposer -----------------------------------------------------------

    fn drop_target(&self, p: Pos2, dragged: OccKey) -> Option<DropTarget> {
        let zone = self.zones.iter().find(|z| z.hit.contains(p))?;
        let items: Vec<Rect> = zone
            .items
            .iter()
            .filter(|(k, _)| k.task_id != dragged.task_id)
            .map(|(_, r)| *r)
            .collect();
        let (index, line) = if zone.horizontal {
            let index = items
                .iter()
                .filter(|r| p.y > r.bottom() || (p.y >= r.top() && p.x > r.center().x))
                .count();
            let x_line = |x: f32, r: &Rect| [pos2(x, r.top()), pos2(x, r.bottom())];
            let line = if let Some(r) = items.get(index) {
                x_line(r.left() - SOMEDAY_GAP / 2.0, r)
            } else if let Some(r) = items.last() {
                x_line(r.right() + SOMEDAY_GAP / 2.0, r)
            } else {
                let r = Rect::from_min_size(zone.list.min, vec2(1.0, card::MIN_HEIGHT));
                x_line(zone.list.left() + 1.0, &r)
            };
            (index, line)
        } else {
            let index = items.iter().filter(|r| r.center().y < p.y).count();
            let y = if let Some(r) = items.get(index) {
                r.top() - CARD_GAP / 2.0
            } else if let Some(r) = items.last() {
                r.bottom() + CARD_GAP / 2.0
            } else {
                zone.list.top() + 3.0
            };
            (index, [pos2(zone.list.left(), y), pos2(zone.list.right(), y)])
        };
        Some(DropTarget { slot: zone.slot, index, line, clip: zone.hit })
    }

    fn update_drag(&mut self, ctx: &egui::Context) {
        let Some(drag) = self.drag.as_ref() else {
            self.nav_hover = None;
            return;
        };
        let (pointer, down, escape, time) = ctx.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.primary_down(),
                i.key_pressed(Key::Escape),
                i.time,
            )
        });
        if escape {
            self.drag = None;
            self.nav_hover = None;
            return;
        }
        let target = pointer.and_then(|p| self.drop_target(p, drag.key));
        if !down {
            if let Some(t) = target {
                self.actions.push(Action::Move(drag.key, t.slot, t.index));
            }
            self.drag = None;
            self.nav_hover = None;
            return;
        }

        ctx.set_cursor_icon(CursorIcon::Grabbing);
        if let Some(p) = pointer {
            if let Some(t) = &target {
                let painter = ctx
                    .layer_painter(LayerId::new(Order::Foreground, Id::unique("indicateur-depot")))
                    .with_clip_rect(t.clip);
                painter.line_segment(t.line, Stroke::new(3.0, theme::ACCENT));
                painter.circle_filled(t.line[0], 4.0, theme::ACCENT);
            }

            // La carte suit le pointeur.
            let ghost = Rect::from_min_size(p - drag.grab, drag.size);
            let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::unique("carte-deplacee")));
            painter.add(Shape::rect_filled(
                ghost.translate(vec2(0.0, 6.0)).expand(1.0),
                CornerRadius::same(card::RADIUS + 2),
                Color32::from_black_alpha(22),
            ));
            let layout = card::layout(&painter, &drag.view, drag.size.x, 1.0);
            card::paint(
                &painter,
                ghost,
                &drag.view,
                &layout,
                PaintState { hovered: true, check_hovered: false, alpha: 1.0 },
            );

            // Survoler une flèche change de semaine.
            let over = self.nav_targets.iter().find(|(r, _)| r.contains(p)).map(|(_, d)| *d);
            match (over, self.nav_hover) {
                (Some(dir), Some((d, since))) if d == dir => {
                    if time - since > NAV_HOVER_SECS {
                        self.monday += Duration::days(7 * dir);
                        self.nav_hover = Some((dir, time));
                    }
                }
                (Some(dir), _) => self.nav_hover = Some((dir, time)),
                (None, _) => self.nav_hover = None,
            }
        }
        ctx.request_repaint();
    }

    // ----- Raccourcis, fenêtre d'édition, notifications ------------------------------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.editor.is_some()
            || self.settings.is_some()
            || self.drag.is_some()
            || ctx.memory(|m| m.focused().is_some())
        {
            return;
        }
        let (undo, left, right, home) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(Key::Z),
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::Home),
            )
        });
        if undo {
            self.actions.push(Action::Undo);
        }
        if left {
            self.go_to_week(self.monday - Duration::days(7));
        }
        if right {
            self.go_to_week(self.monday + Duration::days(7));
        }
        if home {
            self.go_to_week(monday_of(self.today));
        }
    }

    fn editor(&mut self, ctx: &egui::Context) {
        let Some(editor) = self.editor.as_mut() else { return };
        let id = editor.task_id;
        let occurrence = editor.occurrence;
        match editor.show(ctx, self.today) {
            Outcome::Open => {}
            Outcome::Cancel => self.editor = None,
            Outcome::Save(edit) => {
                self.editor = None;
                self.commit(|p| apply_edit(p, id, occurrence, edit));
            }
            Outcome::Delete => {
                self.editor = None;
                self.actions.push(Action::Delete(id));
            }
            Outcome::SkipOccurrence(date) => {
                self.editor = None;
                self.actions.push(Action::Skip(id, date));
            }
        }
    }

    fn show_toast(&mut self, ctx: &egui::Context) {
        let Some(toast) = self.toast.as_mut() else { return };
        let now = ctx.input(|i| i.time);
        let start = *toast.shown_at.get_or_insert(now);
        let lifetime = if toast.error { 12.0 } else { 5.0 };
        if now - start > lifetime {
            self.toast = None;
            return;
        }
        ctx.request_repaint_after(StdDuration::from_secs_f64(lifetime - (now - start) + 0.05));

        let mut undo = false;
        let fill = if toast.error { theme::DANGER } else { Color32::from_rgb(0x1F, 0x24, 0x30) };
        egui::Area::new(Id::unique("notification"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -24.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(fill)
                    .corner_radius(10)
                    .inner_margin(Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.set_max_width(640.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&toast.text).color(Color32::WHITE).size(14.5));
                            if toast.undo {
                                ui.add_space(8.0);
                                let button = Button::new(
                                    RichText::new("Annuler")
                                        .color(Color32::from_rgb(0x9D, 0xBC, 0xFF))
                                        .font(theme::bold(14.5)),
                                )
                                .frame(false);
                                if ui.add(button).on_hover_text("Ctrl+Z").clicked() {
                                    undo = true;
                                }
                            }
                        });
                    });
            });
        if undo {
            self.toast = None;
            self.actions.push(Action::Undo);
        }
    }
}

impl eframe::App for PlanifApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.today = Local::now().date_naive();
        // Le jour courant change à minuit même sans interaction.
        ctx.request_repaint_after(StdDuration::from_secs(30));
        self.zones.clear();
        self.nav_targets.clear();

        let full = ui.max_rect();
        ui.painter().rect_filled(full, CornerRadius::ZERO, theme::BG);
        let content = Rect::from_min_max(
            full.min + vec2(MARGIN_X, 26.0),
            full.max - vec2(MARGIN_X, 24.0),
        );

        let header = Rect::from_min_size(content.min, vec2(content.width(), HEADER_H));
        self.header(ui, header);

        let someday_layout = self.someday_layout(ui.painter(), content.width());
        let someday_h = someday_layout
            .total_h
            .min((content.height() * 0.38).max(140.0));
        let week_top = header.bottom() + 20.0;
        let week_bottom = (content.bottom() - someday_h - 22.0).max(week_top + 300.0);
        let week = Rect::from_min_max(pos2(content.left(), week_top), pos2(content.right(), week_bottom));
        self.week(ui, week);
        let someday = Rect::from_min_size(
            pos2(content.left(), week.bottom() + 22.0),
            vec2(content.width(), someday_h),
        );
        self.someday(ui, someday, someday_layout);

        self.update_drag(&ctx);
        self.shortcuts(&ctx);
        self.editor(&ctx);
        if let Some(settings) = self.settings.as_mut()
            && !settings.show(&ctx)
        {
            self.settings = None;
        }
        self.show_toast(&ctx);
        self.apply_actions();
        self.save_if_dirty();
    }
}

fn column_frame(painter: &Painter, rect: Rect, highlight: bool) {
    painter.rect_filled(
        rect.translate(vec2(0.0, 1.0)),
        CornerRadius::same(ROUND),
        Color32::from_black_alpha(6),
    );
    let stroke = if highlight {
        Stroke::new(1.5, theme::ACCENT)
    } else {
        Stroke::new(1.0, theme::BORDER)
    };
    painter.rect(rect, CornerRadius::same(ROUND), theme::SURFACE, stroke, StrokeKind::Inside);
}

/// Roue crantée du bouton Paramètres.
fn paint_gear(painter: &Painter, center: Pos2, color: Color32, background: Color32) {
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.0;
        let dir = vec2(a.cos(), a.sin());
        painter.line_segment([center + dir * 6.0, center + dir * 9.5], Stroke::new(3.2, color));
    }
    painter.circle_filled(center, 7.2, color);
    painter.circle_filled(center, 3.0, background);
}

fn paint_chevron(painter: &Painter, center: Pos2, left: bool, color: Color32) {
    let s = if left { 1.0 } else { -1.0 };
    painter.add(Shape::line(
        vec![
            center + vec2(3.0 * s, -6.0),
            center + vec2(-3.0 * s, 0.0),
            center + vec2(3.0 * s, 6.0),
        ],
        Stroke::new(2.0, color),
    ));
}

/// Reporte le formulaire d'édition sur la tâche.
fn apply_edit(p: &mut Planner, id: u64, occurrence: Option<NaiveDate>, edit: Edit) {
    let Some(task) = p.task_mut(id) else { return };
    task.title = edit.title;
    task.time = edit.time;
    task.color = edit.color;
    let days: BTreeSet<u8> = edit.days.into_iter().collect();
    match (edit.kind, task.is_recurring()) {
        (Kind::Weekly, true) => {
            if let Schedule::Weekly { days: current, .. } = &mut task.schedule {
                *current = days;
            }
        }
        (Kind::Weekly, false) => {
            task.schedule = Schedule::Weekly { days, since: monday_of(edit.date) };
            task.done = false;
            task.done_dates.clear();
            task.skipped_dates.clear();
        }
        (kind, was_recurring) => {
            if was_recurring {
                task.done = occurrence.is_some_and(|d| task.done_dates.contains(&d));
                task.done_dates.clear();
                task.skipped_dates.clear();
            }
            task.schedule = if kind == Kind::Date {
                Schedule::Date { date: edit.date }
            } else {
                Schedule::Someday
            };
        }
    }
}
