# Planif'Week

Planning de la semaine façon Trello : une colonne par jour, des cartes que l'on coche et que l'on fait glisser d'un jour à l'autre. Application native en Rust ([egui/eframe](https://github.com/emilk/egui)), sans base de données ni connexion.

## Lancer

```powershell
cargo run --release
```

L'exécutable se trouve ensuite dans `target\release\planif-week.exe` : il fonctionne seul et peut être copié où vous voulez.

## Installeur Windows

Prérequis : [Inno Setup 6](https://jrsoftware.org/isinfo.php) (`winget install JRSoftware.InnoSetup`).

```powershell
powershell -ExecutionPolicy Bypass -File installer\build-installer.ps1
```

Le script compile la version finale et produit `target\installer\PlanifWeek-Setup-<version>.exe`. La version vient de `Cargo.toml` : augmentez-la avant de publier une mise à jour. Une nouvelle version s'installe par-dessus l'ancienne.

L'installeur est en français. Par défaut, il installe l'application pour l'utilisateur courant, sans droits administrateur, dans `%LOCALAPPDATA%\Programs\PlanifWeek`. Il ajoute un raccourci dans le menu Démarrer, en propose un sur le Bureau, et inscrit l'application dans « Applications installées ». La désinstallation conserve les tâches de l'utilisateur.

L'icône (`assets/planif-week.ico`) se régénère avec `python installer/make_icon.py`.

## Utilisation

- **Ajouter** : « + Ajouter une tâche » en bas d'un jour (ou de « Un jour »), tapez le titre puis Entrée. Vous pouvez enchaîner plusieurs tâches, et Échap ferme la saisie. Une heure en fin de titre est reconnue : « Sport 18h30 ».
- **Cocher** : cliquez sur le rond.
- **Déplacer** : glissez une carte vers un autre jour, vers « Un jour » ou à une autre place dans la même colonne. Pour changer de semaine pendant le glisser, maintenez la carte un instant sur une flèche ‹ ›.
- **Modifier** : cliquez sur la carte pour changer le titre, l'heure, la couleur, le jour, ou la rendre récurrente (« Chaque semaine », un ou plusieurs jours). Le clic droit donne les mêmes actions.
- **Tâches récurrentes** : chaque occurrence se coche séparément. En déplaçant une occurrence, seul ce jour-là bouge et la série continue. « Retirer ce jour-là » enlève une seule occurrence.
- **Raccourcis** : ← → pour changer de semaine, Origine pour revenir à aujourd'hui, Ctrl+Z pour annuler.

## Données

Tout est enregistré automatiquement à chaque modification dans un fichier JSON :

```
%APPDATA%\PlanifWeek\data\planif-week.json
```

La variable d'environnement `PLANIFWEEK_DATA` permet d'utiliser un autre fichier (par exemple pour faire des essais). Si le fichier est illisible, il n'est jamais écrasé : il est renommé en `planif-week.corrompu-<date>.json` et l'application repart d'un planning vide.

## Organisation du code

| Fichier | Rôle |
| --- | --- |
| `src/model.rs` | Tâches, récurrence, déplacements, avancement de la semaine |
| `src/dates.rs` | Noms français, libellés de semaine, lecture des heures |
| `src/storage.rs` | Lecture et écriture du fichier JSON |
| `src/app.rs` | Fenêtre principale et glisser-déposer |
| `src/card.rs` | Dessin des cartes |
| `src/editor.rs` | Fenêtre de modification d'une tâche |
| `src/theme.rs` | Couleurs, polices, style |

Tests : `cargo test`.
