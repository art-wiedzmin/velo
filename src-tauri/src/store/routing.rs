use rusqlite::params;

use super::now_secs;
use super::types::{Result, RoutingRule, StoreError};
use super::Store;

impl Store {
    pub fn list_routing_rules(&self) -> Result<Vec<RoutingRule>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, app_path, app_name, enabled, created_at FROM routing_rules \
             ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], row_to_routing_rule)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn insert_routing_rule(
        &self,
        app_path: &str,
        app_name: Option<&str>,
    ) -> Result<RoutingRule> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let now = now_secs();
        conn.execute(
            "INSERT INTO routing_rules (app_path, app_name, enabled, created_at) \
             VALUES (?1, ?2, 1, ?3) \
             ON CONFLICT(app_path) DO UPDATE SET \
                 app_name = COALESCE(excluded.app_name, routing_rules.app_name), \
                 enabled = 1",
            params![app_path, app_name, now],
        )?;
        // Always look the row up post-write: ON CONFLICT path doesn't update
        // last_insert_rowid.
        let rule: RoutingRule = conn
            .query_row(
                "SELECT id, app_path, app_name, enabled, created_at FROM routing_rules WHERE app_path = ?1",
                params![app_path],
                row_to_routing_rule,
            )?;
        Ok(rule)
    }

    pub fn delete_routing_rule(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let n = conn.execute("DELETE FROM routing_rules WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn set_routing_rule_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let n = conn.execute(
            "UPDATE routing_rules SET enabled = ?1 WHERE id = ?2",
            params![enabled as i64, id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
}

fn row_to_routing_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<RoutingRule> {
    let enabled: i64 = row.get(3)?;
    Ok(RoutingRule {
        id: row.get(0)?,
        app_path: row.get(1)?,
        app_name: row.get(2)?,
        enabled: enabled != 0,
        created_at: row.get(4)?,
    })
}
