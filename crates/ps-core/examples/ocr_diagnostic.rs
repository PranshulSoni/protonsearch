//! Read-only OCR/index status probe. Requires an explicit database path.
use std::path::PathBuf;

use rusqlite::{Connection, OpenFlags};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let mut db_path = None;
    let mut settings_path = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--db") => db_path = args.next().map(PathBuf::from),
            Some("--settings") => settings_path = args.next().map(PathBuf::from),
            _ => anyhow::bail!("usage: cargo run -p ps-core --example ocr_diagnostic -- --db <index.db> [--settings <settings.json>]"),
        }
    }
    let db_path =
        db_path.ok_or_else(|| anyhow::anyhow!("--db is required; no default user database is opened"))?;

    println!("database: {}", db_path.display());
    match Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => report_database(&conn),
        Err(error) => println!("database status: unavailable ({error})"),
    }

    if let Some(path) = settings_path {
        match std::fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<ps_ipc::Settings>(&raw) {
                Ok(settings) => println!(
                    "settings: valid; OCR {}; configured roots: {}",
                    if settings.indexing.ocr_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    settings.indexing.roots.len()
                ),
                Err(error) => println!("settings: invalid ({error})"),
            },
            Err(error) => println!("settings: unavailable ({error})"),
        }
    } else {
        println!("settings: not checked (pass --settings <settings.json>)");
    }

    let languages = ps_ocr::available_languages();
    println!(
        "OCR capability: {}; installed languages: {}",
        if ps_ocr::available() {
            "available"
        } else {
            "unavailable"
        },
        languages.len()
    );
    println!("capability note: this reports host OCR availability, not indexing completeness");
    Ok(())
}

fn report_database(conn: &Connection) {
    let has_table = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'ocr_documents'",
            [],
            |_| Ok(()),
        )
        .is_ok();
    if !has_table {
        println!("OCR index: table absent; stored OCR row count unavailable");
        return;
    }

    match conn.query_row("SELECT COUNT(*) FROM ocr_documents", [], |row| {
        row.get::<_, u64>(0)
    }) {
        Ok(count) => println!("stored OCR rows: {count}"),
        Err(error) => println!("stored OCR rows: unavailable ({error})"),
    }

    let columns = conn
        .prepare("PRAGMA table_info(ocr_documents)")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()
        });
    match columns {
        Ok(columns) if columns.iter().any(|name| name == "path_lower") => {
            match conn.query_row(
                "SELECT COUNT(*) FROM ocr_documents WHERE path_lower = ''",
                [],
                |row| row.get::<_, u64>(0),
            ) {
                Ok(pending) => println!("OCR metadata: present; rows pending path metadata: {pending}"),
                Err(error) => println!("OCR metadata: present; pending count unavailable ({error})"),
            }
        }
        Ok(_) => println!("OCR metadata: path metadata column absent"),
        Err(error) => println!("OCR metadata: unavailable ({error})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_database_is_never_created() {
        let path = std::env::temp_dir().join(format!("ocr-diagnostic-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).is_err());
        assert!(!path.exists());
    }
}
