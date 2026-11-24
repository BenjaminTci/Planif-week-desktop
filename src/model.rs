//! Modèle de données : tâches ponctuelles, récurrentes et « un jour ».

use std::cmp::Ordering;
use std::collections::BTreeSet;

use chrono::{Duration, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::dates::weekday_index;

pub const DATA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskColor {
    #[default]
    Violet,
    Rose,
    Bleu,
    Orange,
    Vert,
}

impl TaskColor {
    pub const ALL: [TaskColor; 5] = [
        TaskColor::Violet,
        TaskColor::Rose,
        TaskColor::Bleu,
        TaskColor::Orange,
        TaskColor::Vert,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TaskColor::Violet => "Violet",
            TaskColor::Rose => "Rose",
            TaskColor::Bleu => "Bleu",
            TaskColor::Orange => "Orange",
            TaskColor::Vert => "Vert",
        }
    }
}

/// Quand une tâche apparaît.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Schedule {
    /// Sans date : zone « Un jour ».
    Someday,
    /// Un jour précis.
    Date { date: NaiveDate },
    /// Chaque semaine, les jours donnés (0 = lundi … 6 = dimanche),
    /// à partir de la semaine de `since`.
    Weekly { days: BTreeSet<u8>, since: NaiveDate },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub time: Option<NaiveTime>,
    #[serde(default)]
    pub color: TaskColor,
    pub schedule: Schedule,
    /// Terminée (tâches ponctuelles et « un jour »).
    #[serde(default)]
    pub done: bool,
    /// Occurrences terminées (tâches récurrentes).
    #[serde(default)]
    pub done_dates: BTreeSet<NaiveDate>,
    /// Occurrences retirées ou déplacées ailleurs (tâches récurrentes).
    #[serde(default)]
    pub skipped_dates: BTreeSet<NaiveDate>,
    /// Position dans la liste du jour (plus petit = plus haut).
    #[serde(default)]
    pub order: f64,
}

impl Task {
    pub fn is_recurring(&self) -> bool {
        matches!(self.schedule, Schedule::Weekly { .. })
    }

    pub fn occurs_on(&self, date: NaiveDate) -> bool {
        match &self.schedule {
            Schedule::Someday => false,
            Schedule::Date { date: d } => *d == date,
            Schedule::Weekly { days, since } => {
                date >= *since
                    && days.contains(&weekday_index(date))
                    && !self.skipped_dates.contains(&date)
            }
        }
    }
}

/// Une colonne où l'on peut ranger des tâches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Day(NaiveDate),
    Someday,
}

impl Slot {
    fn schedule(self) -> Schedule {
        match self {
            Slot::Day(date) => Schedule::Date { date },
            Slot::Someday => Schedule::Someday,
        }
    }
}

/// Une carte affichée : une tâche, et pour une récurrente, la date de l'occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccKey {
    pub task_id: u64,
    pub date: Option<NaiveDate>,
}

impl OccKey {
    pub fn slot(self) -> Slot {
        match self.date {
            Some(date) => Slot::Day(date),
            None => Slot::Someday,
        }
    }
}

/// Toutes les données de l'application, telles qu'enregistrées sur le disque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Planner {
    #[serde(default)]
    pub version: u32,
    pub next_id: u64,
    pub tasks: Vec<Task>,
}

impl Default for Planner {
    fn default() -> Self {
        Self {
            version: DATA_VERSION,
            next_id: 1,
            tasks: Vec::new(),
        }
    }
}

impl Planner {
    pub fn task(&self, id: u64) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn task_mut(&mut self, id: u64) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Les cartes d'une colonne, dans l'ordre d'affichage.
    pub fn items(&self, slot: Slot) -> Vec<OccKey> {
        let mut tasks: Vec<&Task> = self
            .tasks
            .iter()
            .filter(|t| match slot {
                Slot::Someday => t.schedule == Schedule::Someday,
                Slot::Day(date) => t.occurs_on(date),
            })
            .collect();
        tasks.sort_by(|a, b| {
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(Ordering::Equal)
                .then(a.id.cmp(&b.id))
        });
        let date = match slot {
            Slot::Day(date) => Some(date),
            Slot::Someday => None,
        };
        tasks
            .into_iter()
            .map(|t| OccKey { task_id: t.id, date })
            .collect()
    }

    pub fn is_done(&self, key: OccKey) -> bool {
        match self.task(key.task_id) {
            Some(t) if t.is_recurring() => key.date.is_some_and(|d| t.done_dates.contains(&d)),
            Some(t) => t.done,
            None => false,
        }
    }

    pub fn toggle_done(&mut self, key: OccKey) {
        let Some(task) = self.task_mut(key.task_id) else {
            return;
        };
        if task.is_recurring() {
            if let Some(date) = key.date
                && !task.done_dates.remove(&date) {
                    task.done_dates.insert(date);
                }
        } else {
            task.done = !task.done;
        }
    }

    /// Ajoute une tâche en bas de la colonne où elle apparaît et renvoie son identifiant.
    pub fn add(
        &mut self,
        title: String,
        time: Option<NaiveTime>,
        color: TaskColor,
        schedule: Schedule,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let order = self.order_at_end(&schedule);
        self.tasks.push(Task {
            id,
            title,
            time,
            color,
            schedule,
            done: false,
            done_dates: BTreeSet::new(),
            skipped_dates: BTreeSet::new(),
            order,
        });
        id
    }

    fn order_at_end(&self, schedule: &Schedule) -> f64 {
        let max_of = |keys: Vec<OccKey>| {
            keys.iter()
                .filter_map(|k| self.task(k.task_id))
                .map(|t| t.order)
                .fold(None, |acc: Option<f64>, o| Some(acc.map_or(o, |a| a.max(o))))
        };
        let max = match schedule {
            Schedule::Someday => max_of(self.items(Slot::Someday)),
            Schedule::Date { date } => max_of(self.items(Slot::Day(*date))),
            Schedule::Weekly { .. } => self
                .tasks
                .iter()
                .map(|t| t.order)
                .fold(None, |acc: Option<f64>, o| Some(acc.map_or(o, |a| a.max(o)))),
        };
        max.map_or(0.0, |m| m + 1.0)
    }

    pub fn remove(&mut self, id: u64) {
        self.tasks.retain(|t| t.id != id);
    }

    /// Retire une seule occurrence d'une tâche récurrente.
    pub fn skip_occurrence(&mut self, id: u64, date: NaiveDate) {
        if let Some(task) = self.task_mut(id) {
            task.skipped_dates.insert(date);
            task.done_dates.remove(&date);
        }
    }

    /// Déplace une carte dans `target`, à la position `index` (comptée sans la carte déplacée).
    ///
    /// Déplacer une occurrence récurrente vers un autre jour ne touche pas la série :
    /// l'occurrence est retirée de la série et devient une tâche ponctuelle.
    pub fn move_item(&mut self, key: OccKey, target: Slot, index: usize) {
        let Some(task) = self.task(key.task_id) else {
            return;
        };
        let recurring = task.is_recurring();
        let list: Vec<OccKey> = self
            .items(target)
            .into_iter()
            .filter(|k| k.task_id != key.task_id)
            .collect();
        let index = index.min(list.len());
        let order_of = |i: usize| list.get(i).and_then(|k| self.task(k.task_id)).map(|t| t.order);
        let before = if index == 0 { None } else { order_of(index - 1) };
        let after = order_of(index);
        let order = match (before, after) {
            (None, None) => 0.0,
            (Some(b), None) => b + 1.0,
            (None, Some(a)) => a - 1.0,
            (Some(b), Some(a)) => (b + a) / 2.0,
        };

        if recurring && target != key.slot() {
            let Some(date) = key.date else { return };
            let new_id = self.next_id;
            self.next_id += 1;
            let task = self.task_mut(key.task_id).expect("tâche vérifiée plus haut");
            task.skipped_dates.insert(date);
            let done = task.done_dates.remove(&date);
            let copy = Task {
                id: new_id,
                title: task.title.clone(),
                time: task.time,
                color: task.color,
                schedule: target.schedule(),
                done,
                done_dates: BTreeSet::new(),
                skipped_dates: BTreeSet::new(),
                order,
            };
            self.tasks.push(copy);
        } else {
            let task = self.task_mut(key.task_id).expect("tâche vérifiée plus haut");
            task.order = order;
            if !recurring {
                task.schedule = target.schedule();
            }
        }
    }

    /// (terminées, total) sur les sept jours de la semaine.
    pub fn week_progress(&self, monday: NaiveDate) -> (usize, usize) {
        let mut done = 0;
        let mut total = 0;
        for i in 0..7 {
            let date = monday + Duration::days(i);
            for key in self.items(Slot::Day(date)) {
                total += 1;
                if self.is_done(key) {
                    done += 1;
                }
            }
        }
        (done, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn weekly(days: &[u8], since: NaiveDate) -> Schedule {
        Schedule::Weekly {
            days: days.iter().copied().collect(),
            since,
        }
    }

    fn titles(p: &Planner, slot: Slot) -> Vec<String> {
        p.items(slot)
            .iter()
            .map(|k| p.task(k.task_id).unwrap().title.clone())
            .collect()
    }

    #[test]
    fn recurring_occurrences() {
        let mut p = Planner::default();
        // Tous les jeudis à partir de la semaine du 23 novembre 2026.
        let id = p.add("Sport".into(), None, TaskColor::Vert, weekly(&[3], d(2026, 11, 23)));
        assert_eq!(p.items(Slot::Day(d(2026, 11, 26))).len(), 1);
        assert_eq!(p.items(Slot::Day(d(2026, 12, 3))).len(), 1);
        assert!(p.items(Slot::Day(d(2026, 11, 27))).is_empty());
        assert!(p.items(Slot::Day(d(2026, 11, 19))).is_empty(), "pas avant le début");

        // Cocher une occurrence ne coche pas les autres.
        let thu = OccKey { task_id: id, date: Some(d(2026, 11, 26)) };
        p.toggle_done(thu);
        assert!(p.is_done(thu));
        assert!(!p.is_done(OccKey { task_id: id, date: Some(d(2026, 12, 3)) }));
        p.toggle_done(thu);
        assert!(!p.is_done(thu));

        p.skip_occurrence(id, d(2026, 12, 3));
        assert!(p.items(Slot::Day(d(2026, 12, 3))).is_empty());
        assert_eq!(p.items(Slot::Day(d(2026, 12, 10))).len(), 1);
    }

    #[test]
    fn move_one_off_between_days_and_someday() {
        let mut p = Planner::default();
        let mon = d(2026, 11, 23);
        let tue = d(2026, 11, 24);
        let a = p.add("A".into(), None, TaskColor::Rose, Schedule::Date { date: mon });
        p.add("B".into(), None, TaskColor::Rose, Schedule::Date { date: tue });
        p.add("C".into(), None, TaskColor::Rose, Schedule::Date { date: tue });

        // A entre B et C.
        p.move_item(OccKey { task_id: a, date: Some(mon) }, Slot::Day(tue), 1);
        assert!(p.items(Slot::Day(mon)).is_empty());
        assert_eq!(titles(&p, Slot::Day(tue)), ["B", "A", "C"]);

        // Tout en haut, puis vers « Un jour ».
        p.move_item(OccKey { task_id: a, date: Some(tue) }, Slot::Day(tue), 0);
        assert_eq!(titles(&p, Slot::Day(tue)), ["A", "B", "C"]);
        p.move_item(OccKey { task_id: a, date: Some(tue) }, Slot::Someday, 0);
        assert_eq!(titles(&p, Slot::Day(tue)), ["B", "C"]);
        assert_eq!(titles(&p, Slot::Someday), ["A"]);

        // Retour sur un jour, en fin de liste (index trop grand = à la fin).
        p.move_item(OccKey { task_id: a, date: None }, Slot::Day(mon), 99);
        assert_eq!(titles(&p, Slot::Day(mon)), ["A"]);
    }

    #[test]
    fn move_recurring_occurrence_detaches_it() {
        let mut p = Planner::default();
        let thu = d(2026, 11, 26);
        let fri = d(2026, 11, 27);
        let id = p.add("Sport".into(), None, TaskColor::Vert, weekly(&[3], d(2026, 11, 23)));
        let key = OccKey { task_id: id, date: Some(thu) };
        p.toggle_done(key);
        p.move_item(key, Slot::Day(fri), 0);

        assert!(p.items(Slot::Day(thu)).is_empty(), "l'occurrence a quitté jeudi");
        let moved = p.items(Slot::Day(fri));
        assert_eq!(moved.len(), 1);
        let copy = p.task(moved[0].task_id).unwrap();
        assert!(!copy.is_recurring());
        assert!(copy.done, "l'état terminé suit la carte");
        assert_eq!(p.items(Slot::Day(d(2026, 12, 3))).len(), 1, "la série continue");

        // Réordonner une récurrente dans son propre jour garde la série intacte.
        let next = d(2026, 12, 3);
        p.add("Avant".into(), None, TaskColor::Bleu, Schedule::Date { date: next });
        p.move_item(OccKey { task_id: id, date: Some(next) }, Slot::Day(next), 0);
        assert_eq!(titles(&p, Slot::Day(next)), ["Sport", "Avant"]);
        assert!(p.task(id).unwrap().is_recurring());
    }

    #[test]
    fn week_progress_counts_occurrences() {
        let mut p = Planner::default();
        let mon = d(2026, 11, 23);
        let id = p.add("Stand-up".into(), None, TaskColor::Bleu, weekly(&[0, 2, 4], mon));
        let one = p.add("Facture".into(), None, TaskColor::Orange, Schedule::Date { date: mon });
        p.add("Plus tard".into(), None, TaskColor::Orange, Schedule::Someday);
        assert_eq!(p.week_progress(mon), (0, 4));
        p.toggle_done(OccKey { task_id: one, date: Some(mon) });
        p.toggle_done(OccKey { task_id: id, date: Some(d(2026, 11, 25)) });
        assert_eq!(p.week_progress(mon), (2, 4));
    }

    #[test]
    fn json_roundtrip() {
        let mut p = Planner::default();
        p.add("Marché".into(), NaiveTime::from_hms_opt(9, 0, 0), TaskColor::Vert, weekly(&[5], d(2026, 11, 23)));
        p.add("Portfolio".into(), None, TaskColor::Violet, Schedule::Someday);
        let json = serde_json::to_string(&p).unwrap();
        let back: Planner = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
