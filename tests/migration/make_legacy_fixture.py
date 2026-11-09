#!/usr/bin/env python3
"""Create a deterministic synthetic legacy v1 database for migration tests.

Never reads or writes the real %APPDATA% index. Output goes to a caller-supplied temp dir.
Schema mirrors the real legacy DB (see docs/audit/GATE0_BASELINE.md).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
import sys
import tempfile

FIXTURE_VERSION = 1


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def build_db(db_path: str) -> dict:
    if os.path.exists(db_path):
        os.remove(db_path)
    con = sqlite3.connect(db_path)
    try:
        con.executescript(
            """
            CREATE TABLE files (
                path TEXT PRIMARY KEY, name TEXT NOT NULL, extension TEXT NOT NULL,
                modified INTEGER NOT NULL, size INTEGER NOT NULL DEFAULT 0,
                is_dir INTEGER NOT NULL DEFAULT 0, last_seen_scan INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX idx_files_name ON files(name);
            CREATE VIRTUAL TABLE files_fts USING fts5(path UNINDEXED, content);
            CREATE TABLE browser_items (
                url TEXT PRIMARY KEY, title TEXT NOT NULL, source TEXT NOT NULL,
                visit_count INTEGER NOT NULL, last_visit_time INTEGER NOT NULL
            );
            CREATE TABLE quicklinks (name TEXT PRIMARY KEY, url TEXT NOT NULL, keyword TEXT NOT NULL);
            CREATE TABLE snippets (name TEXT PRIMARY KEY, content TEXT NOT NULL, keyword TEXT);
            CREATE TABLE ai_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE memory_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp INTEGER NOT NULL,
                source TEXT NOT NULL, event_type TEXT NOT NULL, title TEXT NOT NULL,
                detail TEXT, app_name TEXT, path TEXT, url TEXT
            );
            CREATE TABLE clipboard_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT UNIQUE,
                timestamp INTEGER NOT NULL, source_app TEXT NOT NULL,
                is_image INTEGER DEFAULT 0, pinned INTEGER DEFAULT 0, ocr_text TEXT
            );
            CREATE TABLE timeline_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp INTEGER NOT NULL,
                duration INTEGER NOT NULL, app_name TEXT NOT NULL, window_title TEXT NOT NULL
            );
            """
        )
        files = [
            ("C:\\Users\\Example\\Documents\\alpha-report.pdf", "alpha-report.pdf", "pdf", 1735689600, 245760, 0),
            ("C:\\Users\\Example\\Documents\\beta-notes.md", "beta-notes.md", "md", 1735689700, 4096, 0),
            ("C:\\Users\\Example\\Desktop\\invoice-2024.docx", "invoice-2024.docx", "docx", 1735689800, 98304, 0),
            ("C:\\Users\\Example\\Downloads\\setup.exe", "setup.exe", "exe", 1735689900, 1048576, 0),
            ("C:\\Users\\Example\\Pictures\\screenshot.png", "screenshot.png", "png", 1735690000, 524288, 0),
            ("C:\\Users\\Example\\Projects\\app\\main.rs", "main.rs", "rs", 1735690100, 8192, 0),
            ("C:\\Users\\Example\\Projects\\app\\index.ts", "index.ts", "ts", 1735690200, 6144, 0),
            ("C:\\Users\\Example\\Projects\\app", "app", "", 1735690300, 0, 1),
            ("C:\\Users\\Example\\Documents", "Documents", "", 1735690400, 0, 1),
            ("D:\\Archive\\legacy-dump.txt", "legacy-dump.txt", "txt", 1735690500, 2048, 0),
            ("D:\\Archive\\photo.jpg", "photo.jpg", "jpg", 1735690600, 734003, 0),
            ("E:\\Games\\demo\\readme.txt", "readme.txt", "txt", 1735690700, 512, 0),
        ]
        con.executemany(
            "INSERT INTO files(path,name,extension,modified,size,is_dir,last_seen_scan) VALUES (?,?,?,?,?,?,?)",
            [(*row, 1) for row in files],
        )
        con.executemany("INSERT INTO files_fts(path,content) VALUES (?,?)", [
            ("C:\\Users\\Example\\Documents\\alpha-report.pdf", "Quarterly revenue summary for project phoenix. Confidential synthetic content."),
            ("C:\\Users\\Example\\Documents\\beta-notes.md", "Design notes: tiered retrieval, cancellation, deterministic ranking."),
            ("C:\\Users\\Example\\Projects\\app\\main.rs", 'fn main() { println!("synthetic fixture"); }'),
        ])
        con.executemany("INSERT INTO browser_items(url,title,source,visit_count,last_visit_time) VALUES (?,?,?,?,?)", [
            ("https://example.com/docs/search", "Example Docs: search architecture", "chrome_history", 7, 1735691000000000),
            ("https://example.org/bookmark", "Example Bookmark", "chrome_bookmark", 1, 0),
            ("https://news.example.net/item?id=42", "Synthetic news item", "brave_history", 3, 1735691100000000),
            ("https://support.example.com/faq", "Support FAQ", "firefox_history", 2, 1735691200000000),
        ])
        con.executemany("INSERT INTO quicklinks(name,url,keyword) VALUES (?,?,?)", [
            ("Example Search", "https://example.com/search?q={query}", "ex"),
            ("Rust Docs", "https://docs.rs/releases/search?query={query}", "rs"),
        ])
        con.execute("INSERT INTO snippets(name,content,keyword) VALUES (?,?,?)",
                    ("Synthetic Snippet", "This is fixture text for migration tests.", "!fix"))
        con.executemany("INSERT INTO ai_settings(key,value) VALUES (?,?)", [
            ("endpoint", "https://api.example.invalid/chat/completions"),
            ("model", "synthetic-model-name"),
        ])
        con.executemany(
            "INSERT INTO memory_events(timestamp,source,event_type,title,detail,app_name,path,url) VALUES (?,?,?,?,?,?,?,?)",
            [
                (1735692000, "Browser", "Visited Page", "Example Docs: search architecture", "https://example.com/docs/search", "chrome_history", None, "https://example.com/docs/search"),
                (1735692100, "Browser", "Visited Page", "Synthetic news item", "https://news.example.net/item?id=42", "brave_history", None, "https://news.example.net/item?id=42"),
                (1735692200, "Browser", "Visited Page", "Support FAQ", "https://support.example.com/faq", "firefox_history", None, "https://support.example.com/faq"),
                (1735692300, "Git", "Commit", "fixture: synthetic commit", "deadbeefcafe synthetic commit message", "git", "C:\\Users\\Example\\Projects\\app", None),
                (1735692400, "Git", "Commit", "fixture: second commit", "cafed00d synthetic commit message", "git", "C:\\Users\\Example\\Projects\\app", None),
            ],
        )
        con.commit()
    finally:
        con.close()
    return {
        "fixture_version": FIXTURE_VERSION,
        "tables": {
            "files": 12, "files_fts": 3, "browser_items": 4, "quicklinks": 2,
            "snippets": 1, "ai_settings": 2, "memory_events": 5,
            "clipboard_history": 0, "timeline_events": 0,
        },
        "ai_settings_keys": ["endpoint", "model"],
        "sha256": sha256_file(db_path),
        "bytes": os.path.getsize(db_path),
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Create synthetic legacy v1 SQLite fixture")
    ap.add_argument("--out-dir", default=tempfile.mkdtemp(prefix="ps-migration-fixture-"))
    args = ap.parse_args(argv)
    os.makedirs(args.out_dir, exist_ok=True)
    db_path = os.path.join(args.out_dir, "legacy_fixture.db")
    manifest = build_db(db_path)
    manifest["db_path"] = db_path
    manifest_path = os.path.join(args.out_dir, "fixture_manifest.json")
    with open(manifest_path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(manifest, f, indent=2, sort_keys=True)
        f.write("\n")
    print(json.dumps(manifest, indent=2, sort_keys=True))
    print("fixture written to", args.out_dir)
    return 0


if __name__ == "__main__":
    sys.exit(main())
