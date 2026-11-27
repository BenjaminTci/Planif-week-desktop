//! Fenêtre de modification d'une tâche.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, Frame, Id, Key, Layout, Margin, RichText, Sense,
    Stroke, TextEdit, Ui, pos2, vec2,
};

use crate::dates::{self, WEEKDAY_LETTER, WEEKDAY_LONG};
use crate::model::{MAX_EVERY, OccKey, Schedule, Task, TaskColor};
use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Date,
    Weekly,
    Monthly,
    Someday,
}

pub struct Editor {
    pub task_id: u64,
    /// Date de l'occurrence ouverte (tâches récurrentes).
    pub occurrence: Option<NaiveDate>,
    pub was_recurring: bool,
    title: String,
    time_text: String,
    color: TaskColor,
    kind: Kind,
    date: NaiveDate,
    days: [bool; 7],
    /// Jour du mois (1 à 31) des échéances mensuelles.
    month_day: u8,
    /// Intervalle des séries : toutes les X semaines, tous les X mois (1 à 12).
    every: u8,
    error: Option<String>,
    first_frame: bool,
}

pub enum Outcome {
    Open,
    Cancel,
    Save(Edit),
    Delete,
    SkipOccurrence(NaiveDate),
}

/// Les valeurs validées du formulaire.
pub struct Edit {
    pub title: String,
    pub time: Option<NaiveTime>,
    pub color: TaskColor,
    pub kind: Kind,
    pub date: NaiveDate,
    pub days: Vec<u8>,
    pub month_day: u8,
    pub every: u8,
}

impl Editor {
    pub fn new(task: &Task, key: OccKey, today: NaiveDate) -> Self {
        let (kind, date, days) = match &task.schedule {
            Schedule::Someday => (Kind::Someday, today, [false; 7]),
            Schedule::Date { date } => (Kind::Date, *date, [false; 7]),
            Schedule::Weekly { days, .. } => {
                let mut set = [false; 7];
                for &d in days {
                    if let Some(slot) = set.get_mut(d as usize) {
                        *slot = true;
                    }
                }
                (Kind::Weekly, key.date.unwrap_or(today), set)
            }
            Schedule::Monthly { .. } => (Kind::Monthly, key.date.unwrap_or(today), [false; 7]),
        };
        let month_day = match task.schedule {
            Schedule::Monthly { day, .. } => day,
            _ => date.day() as u8,
        };
        let every = match task.schedule {
            Schedule::Weekly { every, .. } | Schedule::Monthly { every, .. } => every.clamp(1, MAX_EVERY),
            _ => 1,
        };
        Self {
            task_id: task.id,
            occurrence: if task.is_recurring() { key.date } else { None },
            was_recurring: task.is_recurring(),
            title: task.title.clone(),
            time_text: task.time.map(dates::format_time).unwrap_or_default(),
            color: task.color,
            kind,
            date,
            days,
            month_day,
            every,
            error: None,
            first_frame: true,
        }
    }

    fn validate(&self) -> Result<Edit, String> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err("Le titre ne peut pas être vide.".into());
        }
        let time = dates::parse_time(&self.time_text)
            .map_err(|()| "Heure non reconnue. Exemples : 14:30, 9h, 18h45.".to_owned())?;
        let days: Vec<u8> = (0..7u8).filter(|&i| self.days[i as usize]).collect();
        if self.kind == Kind::Weekly && days.is_empty() {
            return Err("Choisissez au moins un jour de la semaine.".into());
        }
        Ok(Edit {
            title: title.to_owned(),
            time,
            color: self.color,
            kind: self.kind,
            date: self.date,
            days,
            month_day: self.month_day,
            every: self.every,
        })
    }

    pub fn show(&mut self, ctx: &egui::Context, today: NaiveDate) -> Outcome {
        let mut outcome = Outcome::Open;
        let frame = Frame::new()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(16)
            .inner_margin(Margin::same(24));
        let response = egui::Modal::new(Id::unique("editeur-tache"))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(70))
            .show(ctx, |ui| {
                ui.set_width(560.0);
                outcome = self.contents(ui, today);
            });
        if matches!(outcome, Outcome::Open) && response.should_close() {
            outcome = Outcome::Cancel;
        }
        outcome
    }

    fn contents(&mut self, ui: &mut Ui, today: NaiveDate) -> Outcome {
        let mut outcome = Outcome::Open;
        ui.label(RichText::new("Modifier la tâche").font(theme::bold(20.0)).color(theme::TEXT));
        ui.add_space(10.0);

        label(ui, "Titre");
        let title = ui.add(
            TextEdit::singleline(&mut self.title)
                .desired_width(f32::INFINITY)
                .margin(vec2(10.0, 8.0)),
        );
        if self.first_frame {
            title.request_focus();
            self.first_frame = false;
        }
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                label(ui, "Heure");
                ui.add(
                    TextEdit::singleline(&mut self.time_text)
                        .hint_text("Aucune")
                        .desired_width(110.0)
                        .margin(vec2(10.0, 8.0)),
                );
            });
            ui.add_space(24.0);
            ui.vertical(|ui| {
                label(ui, "Couleur");
                ui.horizontal(|ui| {
                    for color in TaskColor::ALL {
                        self.color_swatch(ui, color);
                    }
                });
            });
        });
        ui.add_space(10.0);

        label(ui, "Quand");
        ui.horizontal(|ui| {
            for (kind, text) in [
                (Kind::Date, "Un jour précis"),
                (Kind::Weekly, "Par semaine"),
                (Kind::Monthly, "Par mois"),
                (Kind::Someday, "Un jour (sans date)"),
            ] {
                if segment(ui, self.kind == kind, text).clicked() && self.kind != kind {
                    if kind == Kind::Weekly && !self.days.contains(&true) {
                        self.days[dates::weekday_index(self.date) as usize] = true;
                    }
                    self.kind = kind;
                }
            }
        });
        ui.add_space(6.0);

        match self.kind {
            Kind::Date => {
                ui.horizontal(|ui| {
                    if ui.add(Button::new("‹").min_size(vec2(32.0, 32.0))).clicked() {
                        self.date -= Duration::days(1);
                    }
                    let text = dates::capitalize(&dates::long_date(self.date, today));
                    ui.add_sized(
                        vec2(220.0, 32.0),
                        egui::Label::new(RichText::new(text).color(theme::TEXT)),
                    );
                    if ui.add(Button::new("›").min_size(vec2(32.0, 32.0))).clicked() {
                        self.date += Duration::days(1);
                    }
                    if self.date != today && ui.link("Aujourd'hui").clicked() {
                        self.date = today;
                    }
                });
            }
            Kind::Weekly => {
                self.every_picker(ui);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for i in 0..7 {
                        let on = self.days[i];
                        let resp = day_chip(ui, on, WEEKDAY_LETTER[i]).on_hover_text(WEEKDAY_LONG[i]);
                        if resp.clicked() {
                            self.days[i] = !on;
                        }
                    }
                });
                let mut note = String::new();
                if self.every > 1 {
                    note = format!(
                        "Une semaine sur {}, en comptant la semaine du {}.",
                        self.every,
                        dates::long_date(dates::monday_of(self.date), today)
                    );
                }
                if self.was_recurring {
                    if !note.is_empty() {
                        note.push(' ');
                    }
                    note.push_str("Les changements s'appliquent à toute la série.");
                }
                if !note.is_empty() {
                    ui.label(RichText::new(note).color(theme::MUTED).size(13.0));
                }
            }
            Kind::Monthly => {
                self.every_picker(ui);
                ui.add_space(4.0);
                // Petit calendrier des jours 1 à 31, sept par ligne.
                for row in 0..5u8 {
                    ui.horizontal(|ui| {
                        for day in (row * 7 + 1)..=(row * 7 + 7).min(31) {
                            let on = self.month_day == day;
                            if day_chip(ui, on, &day.to_string()).clicked() {
                                self.month_day = day;
                            }
                        }
                    });
                }
                let mut note = if self.every > 1 {
                    format!(
                        "Le {}, tous les {} mois, en comptant {}.",
                        self.month_day,
                        self.every,
                        dates::month_year(self.date, today)
                    )
                } else {
                    format!("Le {} de chaque mois.", self.month_day)
                };
                if self.month_day > 28 {
                    note.push_str(" Les mois plus courts, l'échéance tombe leur dernier jour.");
                }
                if self.was_recurring {
                    note.push_str(" Les changements s'appliquent à toute la série.");
                }
                ui.label(RichText::new(note).color(theme::MUTED).size(13.0));
            }
            Kind::Someday => {
                ui.label(
                    RichText::new("La tâche ira dans « Un jour », à caser quand vous aurez le temps.")
                        .color(theme::MUTED)
                        .size(13.0),
                );
            }
        }

        if let Some(error) = &self.error {
            ui.add_space(4.0);
            ui.label(RichText::new(error).color(theme::DANGER).size(13.5));
        }

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if self.was_recurring {
                if let Some(date) = self.occurrence
                    && danger_button(ui, "Retirer ce jour-là")
                        .on_hover_text("Seule cette occurrence disparaît, la série continue.")
                        .clicked()
                    {
                        outcome = Outcome::SkipOccurrence(date);
                    }
                if danger_button(ui, "Supprimer la série").clicked() {
                    outcome = Outcome::Delete;
                }
            } else if danger_button(ui, "Supprimer").clicked() {
                outcome = Outcome::Delete;
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let save = Button::new(RichText::new("Enregistrer").color(Color32::WHITE))
                    .fill(theme::ACCENT)
                    .stroke(Stroke::NONE);
                if ui.add(save).clicked() {
                    outcome = Outcome::Save(Edit::placeholder());
                }
                if ui.button("Annuler").clicked() {
                    outcome = Outcome::Cancel;
                }
            });
        });

        if ui.input(|i| i.key_pressed(Key::Enter)) && matches!(outcome, Outcome::Open) {
            outcome = Outcome::Save(Edit::placeholder());
        }
        if matches!(outcome, Outcome::Save(_)) {
            match self.validate() {
                Ok(edit) => outcome = Outcome::Save(edit),
                Err(message) => {
                    self.error = Some(message);
                    outcome = Outcome::Open;
                }
            }
        }
        outcome
    }

    /// « Toutes les [2 semaines] » / « Tous les [3 mois] », de 1 à 12.
    fn every_picker(&mut self, ui: &mut Ui) {
        let monthly = self.kind == Kind::Monthly;
        let phrase = |n: u8| every_label(monthly, n);
        ui.horizontal(|ui| {
            let less = ui.add_enabled(self.every > 1, Button::new("‹").min_size(vec2(32.0, 32.0)));
            if less.on_hover_text("Moins espacé").clicked() {
                self.every -= 1;
            }
            ui.add_sized(
                vec2(190.0, 32.0),
                egui::Label::new(RichText::new(phrase(self.every)).color(theme::TEXT)),
            );
            let more = ui.add_enabled(self.every < MAX_EVERY, Button::new("›").min_size(vec2(32.0, 32.0)));
            if more.on_hover_text("Plus espacé").clicked() {
                self.every += 1;
            }
            if self.every > 1 && ui.link(phrase(1)).clicked() {
                self.every = 1;
            }
        });
    }

    fn color_swatch(&mut self, ui: &mut Ui, color: TaskColor) {
        let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 32.0), Sense::click());
        let (bg, accent) = theme::task_colors(color);
        let c = rect.center();
        if self.color == color {
            ui.painter().circle_stroke(c, 13.0, Stroke::new(2.0, accent));
        }
        ui.painter().circle_filled(c, 10.0, bg);
        ui.painter().circle_stroke(c, 10.0, Stroke::new(1.5, accent));
        if resp.on_hover_text(color.name()).clicked() {
            self.color = color;
        }
    }
}

impl Edit {
    /// Valeur provisoire remplacée par `validate` avant d'être renvoyée.
    fn placeholder() -> Self {
        Self {
            title: String::new(),
            time: None,
            color: TaskColor::Violet,
            kind: Kind::Someday,
            date: NaiveDate::MIN,
            days: Vec::new(),
            month_day: 1,
            every: 1,
        }
    }
}

/// « Toutes les semaines », « Toutes les 3 semaines », « Tous les mois », « Tous les 3 mois ».
fn every_label(monthly: bool, n: u8) -> String {
    match (monthly, n) {
        (false, 1) => "Toutes les semaines".into(),
        (false, n) => format!("Toutes les {n} semaines"),
        (true, 1) => "Tous les mois".into(),
        (true, n) => format!("Tous les {n} mois"),
    }
}

fn label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(theme::MUTED).size(13.0));
}

fn segment(ui: &mut Ui, selected: bool, text: &str) -> egui::Response {
    let (fill, color, stroke) = if selected {
        (theme::ACCENT_SOFT, theme::ACCENT, Stroke::new(1.0, theme::ACCENT))
    } else {
        (theme::SURFACE, theme::TEXT, Stroke::new(1.0, theme::BORDER))
    };
    ui.add(
        Button::new(RichText::new(text).color(color).size(14.0))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(8),
    )
}

fn day_chip(ui: &mut Ui, on: bool, letter: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click());
    let (fill, text, stroke) = if on {
        (theme::ACCENT, Color32::WHITE, Stroke::NONE)
    } else if resp.hovered() {
        (theme::LINE, theme::TEXT, Stroke::new(1.0, theme::BORDER))
    } else {
        (theme::SURFACE, theme::TEXT, Stroke::new(1.0, theme::BORDER))
    };
    let painter = ui.painter();
    painter.rect(rect, CornerRadius::same(10), fill, stroke, egui::StrokeKind::Inside);
    painter.text(
        pos2(rect.center().x, rect.center().y),
        egui::Align2::CENTER_CENTER,
        letter,
        theme::bold(14.0),
        text,
    );
    resp
}

fn danger_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(
        Button::new(RichText::new(text).color(theme::DANGER).size(14.0))
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, Color32::from_rgb(0xF0, 0xC9, 0xC9))),
    )
}
