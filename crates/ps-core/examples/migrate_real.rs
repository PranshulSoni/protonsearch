//! Diagnostic: run the legacy migration against a COPY of a legacy database into
//! a fresh v2 database, printing the report. Never touches the real user data.
//!
//! Usage: cargo run -p ps-core --example migrate_real -- <legacy-copy.db> <out-v2.db>

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let legacy = PathBuf::from(args.get(1).expect("legacy db path"));
    let v2 = PathBuf::from(args.get(2).expect("v2 db path"));
    let options = ps_core::migration::MigrationOptions {
        import_clipboard: false,
        import_ai_keys: false,
        dry_run: std::env::var("DRY_RUN").is_ok(),
    };
    match ps_core::migration::migrate_legacy_db(&legacy, &v2, &options) {
        Ok(report) => {
            println!("{report:#?}");
        }
        Err(err) => {
            eprintln!("MIGRATION ERROR: {err:#}");
            std::process::exit(2);
        }
    }
}
