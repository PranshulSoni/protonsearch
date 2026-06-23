use rusqlite::Connection;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = match std::env::var("APPDATA") {
        Ok(d) => PathBuf::from(d).join("opensearch-os").join("file_index.db"),
        Err(_) => PathBuf::from("file_index.db"),
    };

    println!("Checking database at: {:?}", db_path);
    if !db_path.exists() {
        println!("Database file does not exist!");
        return Ok(());
    }

    let conn = Connection::open(&db_path)?;

    // 1. Total files
    let total_files: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
    println!("Total files in 'files' table: {}", total_files);

    // 2. Total FTS entries
    let total_fts: i64 = conn.query_row("SELECT COUNT(*) FROM files_fts", [], |row| row.get(0))?;
    println!("Total files in 'files_fts' table: {}", total_fts);

    // 3. Total images
    let total_images: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE extension IN ('png', 'jpg', 'jpeg', 'bmp', 'gif')",
        [],
        |row| row.get(0),
    )?;
    println!("Total images in 'files' table: {}", total_images);

    // Total files in Pictures
    let total_pictures_files: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE path LIKE '%Pictures%'",
        [],
        |row| row.get(0),
    )?;
    println!("Total files containing 'Pictures' in path: {}", total_pictures_files);

    // 4. Print latest 10 images in Pictures
    println!("\n--- Latest 10 Pictures in DB ---");
    let mut stmt = conn.prepare(
        "SELECT path, name, extension, modified, size FROM files \
         WHERE path LIKE '%Pictures%' \
         ORDER BY modified DESC LIMIT 10",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;

    for r in rows {
        if let Ok((path, name, ext, modified, size)) = r {
            // Check if it has FTS content
            let has_fts: i64 = conn.query_row(
                "SELECT COUNT(*) FROM files_fts WHERE path = ? AND length(content) > 0",
                [&path],
                |row| row.get(0),
            )?;
            let fts_content: String = if has_fts > 0 {
                conn.query_row(
                    "SELECT content FROM files_fts WHERE path = ?",
                    [&path],
                    |row| row.get(0),
                )?
            } else {
                "None".to_string()
            };

            println!(
                "Path: {}\nName: {}\nExt: {}\nModified: {}\nSize: {} bytes\nHas FTS: {} (Content snippet: {:?})\n",
                path, name, ext, modified, size, has_fts > 0, fts_content
            );
        }
    }

    // 5. Check if anything matches 'test native image ocr'
    println!("\n--- FTS Match test for 'test native image ocr' ---");
    let clean_query = "test* native* image* ocr*";
    let mut stmt = conn.prepare(
        "SELECT path, content FROM files_fts WHERE files_fts MATCH ?",
    )?;
    let fts_matches = stmt.query_map([clean_query], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut match_count = 0;
    for m in fts_matches {
        if let Ok((path, content)) = m {
            println!("Match path: {}\nContent: {:?}\n", path, content);
            match_count += 1;
        }
    }
    println!("Total FTS matches found: {}", match_count);

    Ok(())
}
