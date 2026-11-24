//! Enregistrement des données dans un fichier JSON local.
//!
//! Emplacement par défaut : `%APPDATA%\PlanifWeek\data\planif-week.json` sous Windows.
//! La variable d'environnement `PLANIFWEEK_DATA` permet de choisir un autre fichier.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::Planner;

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> PathBuf {
        if let Some(custom) = std::env::var_os("PLANIFWEEK_DATA") {
            return PathBuf::from(custom);
        }
        directories::ProjectDirs::from("", "", "PlanifWeek")
            .map(|dirs| dirs.data_dir().join("planif-week.json"))
            .unwrap_or_else(|| PathBuf::from("planif-week.json"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Charge les données. Un fichier absent donne un planning vide.
    /// Un fichier illisible est mis de côté (renommé) pour ne jamais être écrasé,
    /// et le message renvoyé explique où il se trouve.
    pub fn load(&self) -> (Planner, Option<String>) {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return (Planner::default(), None),
            Err(e) => {
                return (
                    Planner::default(),
                    Some(format!("Lecture impossible de {} : {e}", self.path.display())),
                );
            }
        };
        match serde_json::from_slice::<Planner>(&bytes) {
            Ok(planner) => (planner, None),
            Err(e) => {
                let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                let backup = self.path.with_extension(format!("corrompu-{stamp}.json"));
                let message = match fs::rename(&self.path, &backup) {
                    Ok(()) => format!(
                        "Fichier de données illisible ({e}). Il a été conservé sous {}.",
                        backup.display()
                    ),
                    Err(re) => format!("Fichier de données illisible ({e}), copie impossible : {re}"),
                };
                (Planner::default(), Some(message))
            }
        }
    }

    /// Écrit d'abord un fichier temporaire puis le renomme : une coupure
    /// en pleine écriture ne peut pas abîmer les données existantes.
    pub fn save(&self, planner: &Planner) -> io::Result<()> {
        if let Some(dir) = self.path.parent()
            && !dir.as_os_str().is_empty() {
                fs::create_dir_all(dir)?;
            }
        let json = serde_json::to_vec_pretty(planner).map_err(io::Error::other)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Schedule, TaskColor};

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("planifweek-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("sous-dossier").join("data.json")
    }

    #[test]
    fn save_then_load() {
        let path = temp_file("roundtrip");
        let storage = Storage::new(path.clone());
        let (empty, warning) = storage.load();
        assert!(empty.tasks.is_empty() && warning.is_none());

        let mut planner = Planner::default();
        planner.add("Relire le script".into(), None, TaskColor::Rose, Schedule::Someday);
        storage.save(&planner).unwrap();
        storage.save(&planner).unwrap(); // le remplacement d'un fichier existant fonctionne
        let (loaded, warning) = storage.load();
        assert!(warning.is_none());
        assert_eq!(loaded, planner);
        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn corrupt_file_is_kept_aside() {
        let path = temp_file("corrupt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{ pas du json").unwrap();
        let (planner, warning) = Storage::new(path.clone()).load();
        assert!(planner.tasks.is_empty());
        assert!(warning.unwrap().contains("conservé"));
        assert!(!path.exists());
        let kept = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().contains("corrompu"));
        assert!(kept);
        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }
}
