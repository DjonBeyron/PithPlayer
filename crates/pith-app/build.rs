//! Иконка и сведения о версии в самом exe (только Windows).
//!
//! Ресурс собирается вручную вызовом `windres` и `ar`, без крейта
//! `winresource`: тот передаёт путь к проекту в `-I` без кавычек, и на
//! пути с пробелом («Pith_Player v5.0.0») препроцессор падает. Свой вызов
//! обходится вовсе без `-I` — в тексте ресурса путь к иконке абсолютный.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_WINDOWS").is_err() {
        return;
    }

    if let Err(e) = embed_resources() {
        // Ресурсы — украшение, а не условие работы: без них exe соберётся
        // со стандартным значком Windows.
        println!("cargo:warning=не удалось встроить иконку: {e}");
    }
}

#[cfg(windows)]
fn embed_resources() -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").map_err(|e| e.to_string())?);

    // Слэши прямые: обратные пришлось бы экранировать в тексте ресурса.
    let icon = manifest
        .join("assets/icon.ico")
        .display()
        .to_string()
        .replace('\\', "/");

    if !std::path::Path::new(&icon).exists() {
        return Err(format!("нет файла иконки: {icon}"));
    }

    let version = env!("CARGO_PKG_VERSION");
    let numeric = version.replace('.', ", ");

    let script = format!(
        r#"#pragma code_page(65001)
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numeric}, 0
PRODUCTVERSION {numeric}, 0
FILEOS 0x40004
FILETYPE 0x1
{{
BLOCK "StringFileInfo"
{{
BLOCK "000004b0"
{{
VALUE "FileDescription", "Pith Player"
VALUE "FileVersion", "{version}"
VALUE "LegalCopyright", "Pith"
VALUE "ProductName", "Pith Player"
VALUE "ProductVersion", "{version}"
}}
}}
BLOCK "VarFileInfo" {{ VALUE "Translation", 0x0, 0x04b0 }}
}}
"#
    );

    let script_path = out_dir.join("resource.rc");
    std::fs::write(&script_path, script).map_err(|e| e.to_string())?;

    let object = out_dir.join("resource.o");
    run(
        Command::new(tool("WINDRES", "windres"))
            .arg("--target")
            .arg("pe-x86-64")
            .arg(&script_path)
            .arg(&object),
        "windres",
    )?;

    // Объект отдаём линковщику напрямую, а не через статическую библиотеку:
    // из библиотеки ресурс выбрасывается как неиспользуемый, и exe остаётся
    // без иконки и сведений о версии.
    println!("cargo:rustc-link-arg-bins={}", object.display());
    Ok(())
}

#[cfg(not(windows))]
fn embed_resources() -> Result<(), String> {
    Ok(())
}

/// Имя инструмента: из переменной окружения либо стандартное.
#[cfg(windows)]
fn tool(variable: &str, default: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| default.to_string())
}

#[cfg(windows)]
fn run(command: &mut std::process::Command, name: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|e| format!("не удалось запустить {name}: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} завершился с ошибкой: {status}"))
    }
}
