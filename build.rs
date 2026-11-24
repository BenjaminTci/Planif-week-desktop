//! Intègre l'icône et les informations de version dans l'exécutable Windows.

use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/planif-week.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let version = env::var("CARGO_PKG_VERSION").expect("version du paquet");
    let mut parts: Vec<u16> = version
        .split(['.', '-', '+'])
        .take(3)
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    parts.resize(4, 0);
    let numeric = parts.iter().map(u16::to_string).collect::<Vec<_>>().join(",");

    let icon = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("dossier du paquet"))
        .join("assets")
        .join("planif-week.ico");
    let icon = icon.display().to_string().replace('\\', "\\\\");

    let rc = format!(
        r#"#pragma code_page(65001)
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040C04B0"
    BEGIN
      VALUE "CompanyName", "Planif'Week"
      VALUE "FileDescription", "Planif'Week"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "planif-week"
      VALUE "OriginalFilename", "planif-week.exe"
      VALUE "ProductName", "Planif'Week"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x40C, 1200
  END
END
"#
    );
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("planif-week.rc");
    fs::write(&out, rc).expect("écriture du fichier de ressources");
    embed_resource::compile(&out, embed_resource::NONE)
        .manifest_optional()
        .expect("compilation des ressources Windows");
}
