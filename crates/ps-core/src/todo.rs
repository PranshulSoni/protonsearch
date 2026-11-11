//! Shared local task storage. Each mutation is transactional across app windows.
use std::path::Path;

use anyhow::{bail, Context};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
    pub notes: Option<String>,
    pub due_date: Option<String>,
    pub timezone: Option<String>,
    pub priority: Option<String>,
    pub pinned: bool,
    pub position: i64,
    pub association: Option<String>,
}

pub struct TodoStore(Connection);

impl TodoStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let connection = Connection::open(path).context("opening tasks database")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY, payload TEXT NOT NULL
             );",
        )?;
        Ok(Self(connection))
    }

    pub fn list(&self) -> anyhow::Result<Vec<TodoItem>> {
        let mut statement = self.0.prepare("SELECT payload FROM todos")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut items = Vec::new();
        for row in rows {
            items.push(serde_json::from_str::<TodoItem>(&row?)?);
        }
        items.sort_by_key(|item| (!item.pinned, item.position, item.id.clone()));
        Ok(items)
    }

    pub fn save(&mut self, item: &TodoItem) -> anyhow::Result<()> {
        if item.id.trim().is_empty() || item.id.len() > 256 {
            bail!("A task needs a valid identifier");
        }
        if item.title.trim().is_empty() || item.title.len() > 4096 {
            bail!("A task title must contain between 1 and 4096 bytes");
        }
        if item.notes.as_ref().is_some_and(|notes| notes.len() > 65536) {
            bail!("Task notes are too large");
        }
        if item.priority.as_deref().is_some_and(|value| !matches!(value, "low" | "medium" | "high")) {
            bail!("Unknown task priority");
        }
        let payload = serde_json::to_string(item)?;
        self.0.execute(
            "INSERT INTO todos(id, payload) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",
            params![item.id, payload],
        )?;
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> anyhow::Result<()> {
        self.0.execute("DELETE FROM todos WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn import(&mut self, items: &[TodoItem], source: &str) -> anyhow::Result<()> {
        if items.len() > 10000 || source.len() > 256 {
            bail!("Task import exceeds its limit");
        }
        self.0.execute_batch("CREATE TABLE IF NOT EXISTS todo_imports(source TEXT PRIMARY KEY);")?;
        let transaction = self.0.transaction()?;
        let imported: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM todo_imports WHERE source=?1)", [source], |row| row.get(0),
        )?;
        if !imported {
            for item in items {
                if item.id.trim().is_empty() || item.title.trim().is_empty() || item.title.len() > 4096 {
                    bail!("Invalid legacy task");
                }
                transaction.execute(
                    "INSERT OR IGNORE INTO todos(id, payload) VALUES (?1, ?2)",
                    params![item.id, serde_json::to_string(item)?],
                )?;
            }
            transaction.execute("INSERT INTO todo_imports(source) VALUES (?1)", [source])?;
        }
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasks_survive_reopening_and_mutations() {
        let path = std::env::temp_dir().join(format!("ps-todo-{}.db", std::process::id()));
        let mut store = TodoStore::open(&path).unwrap();
        let mut item = TodoItem {
            id: "persisted-task".into(), title: "Read notes".into(), completed: false,
            created_at: 1, updated_at: 1, completed_at: None, notes: None,
            due_date: None, timezone: None, priority: Some("medium".into()),
            pinned: false, position: 0, association: None,
        };
        store.save(&item).unwrap();
        drop(store);
        let mut store = TodoStore::open(&path).unwrap();
        assert_eq!(store.list().unwrap(), vec![item.clone()]);
        item.completed = true;
        item.completed_at = Some(2);
        item.updated_at = 2;
        store.save(&item).unwrap();
        assert_eq!(store.list().unwrap(), vec![item.clone()]);
        store.import(&[item.clone()], "browser-v2").unwrap();
        store.delete(&item.id).unwrap();
        store.import(&[item.clone()], "browser-v2").unwrap();
        assert!(store.list().unwrap().is_empty(), "migration must not resurrect deleted tasks");
        store.save(&item).unwrap();
        item.title.clear();
        assert!(store.save(&item).is_err());
        store.delete(&item.id).unwrap();
        assert!(store.list().unwrap().is_empty());
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
}
