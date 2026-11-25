//! Lancement automatique au démarrage de Windows.
//!
//! Même mécanisme que la plupart des applications : une valeur dans
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, pour l'utilisateur courant,
//! sans droits administrateur. Windows permet aussi de couper ce lancement depuis
//! Paramètres › Applications › Démarrage : ce choix est lu dans `StartupApproved`
//! et levé quand on réactive l'option depuis Planif'Week.

const VALUE_NAME: &str = "PlanifWeek";

#[cfg(windows)]
mod imp {
    use std::io;

    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    pub const APPROVED_KEY: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    fn hkcu() -> RegKey {
        RegKey::predef(HKEY_CURRENT_USER)
    }

    fn not_found(e: &io::Error) -> bool {
        e.kind() == io::ErrorKind::NotFound
    }

    /// La commande enregistrée, si l'entrée existe.
    pub fn registered_command(run_key: &str, name: &str) -> io::Result<Option<String>> {
        let key = match hkcu().open_subkey(run_key) {
            Ok(key) => key,
            Err(e) if not_found(&e) => return Ok(None),
            Err(e) => return Err(e),
        };
        match key.get_value::<String, _>(name) {
            Ok(command) => Ok(Some(command)),
            Err(e) if not_found(&e) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Vrai si l'utilisateur a coupé ce lancement dans les paramètres de Windows
    /// (premier octet impair dans `StartupApproved`).
    pub fn disabled_by_windows(approved_key: &str, name: &str) -> io::Result<bool> {
        let key = match hkcu().open_subkey(approved_key) {
            Ok(key) => key,
            Err(e) if not_found(&e) => return Ok(false),
            Err(e) => return Err(e),
        };
        match key.get_raw_value(name) {
            Ok(value) => Ok(value.bytes.first().is_some_and(|b| b & 1 == 1)),
            Err(e) if not_found(&e) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn is_enabled_in(run_key: &str, approved_key: &str, name: &str) -> io::Result<bool> {
        Ok(registered_command(run_key, name)?.is_some() && !disabled_by_windows(approved_key, name)?)
    }

    pub fn set_enabled_in(
        run_key: &str,
        approved_key: &str,
        name: &str,
        command: &str,
        enabled: bool,
    ) -> io::Result<()> {
        let (run, _) = hkcu().create_subkey(run_key)?;
        if enabled {
            run.set_value(name, &command)?;
        } else {
            match run.delete_value(name) {
                Ok(()) => {}
                Err(e) if not_found(&e) => {}
                Err(e) => return Err(e),
            }
        }
        // Dans les deux cas, on efface le choix « désactivé » mémorisé par Windows :
        // c'est désormais le réglage de Planif'Week qui décide.
        if let Ok(approved) = hkcu().open_subkey_with_flags(approved_key, winreg::enums::KEY_SET_VALUE) {
            match approved.delete_value(name) {
                Ok(()) => {}
                Err(e) if not_found(&e) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

/// Le lancement automatique est-il actif ?
#[cfg(windows)]
pub fn is_enabled() -> Result<bool, String> {
    imp::is_enabled_in(imp::RUN_KEY, imp::APPROVED_KEY, VALUE_NAME).map_err(|e| e.to_string())
}

/// Active ou coupe le lancement automatique de l'exécutable en cours.
#[cfg(windows)]
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("Chemin de l'application introuvable : {e}"))?;
    let command = format!("\"{}\"", exe.display());
    imp::set_enabled_in(imp::RUN_KEY, imp::APPROVED_KEY, VALUE_NAME, &command, enabled)
        .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
pub fn is_enabled() -> Result<bool, String> {
    Err("Disponible uniquement sous Windows.".into())
}

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) -> Result<(), String> {
    Err("Disponible uniquement sous Windows.".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::imp::*;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    /// Clés de test isolées : jamais la vraie clé `Run`.
    const BASE: &str = r"Software\PlanifWeekTests";

    fn keys(case: &str) -> (String, String) {
        (
            format!(r"{BASE}\{case}-{}\Run", std::process::id()),
            format!(r"{BASE}\{case}-{}\Approved", std::process::id()),
        )
    }

    fn cleanup(case: &str) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey_all(format!(r"{BASE}\{case}-{}", std::process::id()));
        // Ne réussit que si la clé de base est vide (l'autre test a fini).
        let _ = hkcu.delete_subkey(BASE);
    }

    #[test]
    fn enable_then_disable() {
        let (run, approved) = keys("toggle");
        assert!(!is_enabled_in(&run, &approved, "App").unwrap(), "absent au départ");

        set_enabled_in(&run, &approved, "App", r#""C:\app.exe""#, true).unwrap();
        assert!(is_enabled_in(&run, &approved, "App").unwrap());
        assert_eq!(registered_command(&run, "App").unwrap().as_deref(), Some(r#""C:\app.exe""#));

        set_enabled_in(&run, &approved, "App", "", false).unwrap();
        assert!(!is_enabled_in(&run, &approved, "App").unwrap());
        // Couper une entrée déjà absente n'est pas une erreur.
        set_enabled_in(&run, &approved, "App", "", false).unwrap();
        cleanup("toggle");
    }

    #[test]
    fn windows_startup_settings_are_respected_then_overridden() {
        let (run, approved) = keys("approved");
        set_enabled_in(&run, &approved, "App", "app.exe", true).unwrap();

        // L'utilisateur coupe l'application dans Paramètres › Démarrage.
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(&approved).unwrap();
        let disabled = winreg::RegValue {
            bytes: vec![3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0].into(),
            vtype: winreg::enums::RegType::REG_BINARY,
        };
        key.set_raw_value("App", &disabled).unwrap();
        assert!(!is_enabled_in(&run, &approved, "App").unwrap());

        // Réactiver depuis Planif'Week lève ce blocage.
        set_enabled_in(&run, &approved, "App", "app.exe", true).unwrap();
        assert!(is_enabled_in(&run, &approved, "App").unwrap());
        cleanup("approved");
    }
}
