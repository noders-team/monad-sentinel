use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

pub struct Store {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditRow {
    pub ts_ms: i64,
    pub actor: String,
    pub op: String,
    pub params: String,
    pub result: String,
    pub detail: String,
}

impl Store {
    pub fn open(path: &str) -> Result<Store> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS metrics (
                 name TEXT NOT NULL, ts_ms INTEGER NOT NULL, value REAL NOT NULL);
             CREATE INDEX IF NOT EXISTS idx_metrics_name_ts ON metrics(name, ts_ms);
             CREATE TABLE IF NOT EXISTS audit (
                 id INTEGER PRIMARY KEY AUTOINCREMENT, ts_ms INTEGER NOT NULL,
                 actor TEXT NOT NULL, op TEXT NOT NULL, params TEXT NOT NULL,
                 result TEXT NOT NULL, detail TEXT NOT NULL);",
        )?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    pub fn append_metric(&self, name: &str, ts_ms: i64, value: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO metrics (name, ts_ms, value) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, ts_ms, value])?;
        Ok(())
    }

    pub fn query_window(&self, name: &str, since_ms: i64) -> Result<Vec<(i64, f64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ts_ms, value FROM metrics WHERE name = ?1 AND ts_ms >= ?2 ORDER BY ts_ms ASC")?;
        let rows = stmt.query_map(rusqlite::params![name, since_ms], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn prune_metrics(&self, older_than_ms: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute("DELETE FROM metrics WHERE ts_ms < ?1", rusqlite::params![older_than_ms])?)
    }

    pub fn append_audit(&self, row: &AuditRow) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO audit (ts_ms, actor, op, params, result, detail) VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![row.ts_ms, row.actor, row.op, row.params, row.result, row.detail])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_audit(&self, limit: usize) -> Result<Vec<AuditRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ts_ms, actor, op, params, result, detail FROM audit ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map(rusqlite::params![limit as i64], |r| Ok(AuditRow {
            ts_ms: r.get(0)?, actor: r.get(1)?, op: r.get(2)?, params: r.get(3)?,
            result: r.get(4)?, detail: r.get(5)?,
        }))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_roundtrip_and_window() {
        let s = Store::open(":memory:").unwrap();
        s.append_metric("vote_rate", 1_000, 1.0).unwrap();
        s.append_metric("vote_rate", 2_000, 2.0).unwrap();
        s.append_metric("other", 2_000, 9.0).unwrap();
        let pts = s.query_window("vote_rate", 1_500).unwrap();
        assert_eq!(pts, vec![(2_000, 2.0)]);
        let all = s.query_window("vote_rate", 0).unwrap();
        assert_eq!(all, vec![(1_000, 1.0), (2_000, 2.0)]);
    }

    #[test]
    fn prune_removes_old() {
        let s = Store::open(":memory:").unwrap();
        s.append_metric("m", 1_000, 1.0).unwrap();
        s.append_metric("m", 5_000, 1.0).unwrap();
        let removed = s.prune_metrics(3_000).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(s.query_window("m", 0).unwrap().len(), 1);
    }

    #[test]
    fn audit_append_and_list_newest_first() {
        let s = Store::open(":memory:").unwrap();
        s.append_audit(&AuditRow { ts_ms: 1, actor: "admin".into(), op: "restart".into(),
            params: "monad-bft.service".into(), result: "ok".into(), detail: "".into() }).unwrap();
        s.append_audit(&AuditRow { ts_ms: 2, actor: "admin".into(), op: "restart".into(),
            params: "monad-rpc.service".into(), result: "error".into(), detail: "boom".into() }).unwrap();
        let rows = s.list_audit(10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ts_ms, 2);
        assert_eq!(rows[0].result, "error");
    }
}
