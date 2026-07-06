use crate::sync::LockExt;
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
                 result TEXT NOT NULL, detail TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS creds (
                 id INTEGER PRIMARY KEY CHECK (id=1),
                 pw_phc TEXT NOT NULL,
                 totp_secret_b32 TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    pub fn append_metric(&self, name: &str, ts_ms: i64, value: f64) -> Result<()> {
        let conn = self.conn.lock_ok();
        conn.execute("INSERT INTO metrics (name, ts_ms, value) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, ts_ms, value])?;
        Ok(())
    }

    pub fn query_window(&self, name: &str, since_ms: i64) -> Result<Vec<(i64, f64)>> {
        let conn = self.conn.lock_ok();
        let mut stmt = conn.prepare(
            "SELECT ts_ms, value FROM metrics WHERE name = ?1 AND ts_ms >= ?2 ORDER BY ts_ms ASC")?;
        let rows = stmt.query_map(rusqlite::params![name, since_ms], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn prune_metrics(&self, older_than_ms: i64) -> Result<usize> {
        let conn = self.conn.lock_ok();
        Ok(conn.execute("DELETE FROM metrics WHERE ts_ms < ?1", rusqlite::params![older_than_ms])?)
    }

    pub fn append_audit(&self, row: &AuditRow) -> Result<i64> {
        let conn = self.conn.lock_ok();
        conn.execute(
            "INSERT INTO audit (ts_ms, actor, op, params, result, detail) VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![row.ts_ms, row.actor, row.op, row.params, row.result, row.detail])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_audit(&self, limit: usize) -> Result<Vec<AuditRow>> {
        let conn = self.conn.lock_ok();
        let mut stmt = conn.prepare(
            "SELECT ts_ms, actor, op, params, result, detail FROM audit ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map(rusqlite::params![limit as i64], |r| Ok(AuditRow {
            ts_ms: r.get(0)?, actor: r.get(1)?, op: r.get(2)?, params: r.get(3)?,
            result: r.get(4)?, detail: r.get(5)?,
        }))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Return `(pw_phc, totp_secret_b32)` from the persisted credentials row, or `None` on a
    /// fresh database. The `creds` table enforces a single-row invariant via `CHECK (id=1)`.
    pub fn get_creds(&self) -> Result<Option<(String, String)>> {
        let conn = self.conn.lock_ok();
        let mut stmt = conn.prepare(
            "SELECT pw_phc, totp_secret_b32 FROM creds WHERE id = 1")?;
        let mut rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Persist (or replace) the single credentials row.
    pub fn set_creds(&self, pw_phc: &str, totp_secret_b32: &str) -> Result<()> {
        let conn = self.conn.lock_ok();
        conn.execute(
            "INSERT OR REPLACE INTO creds (id, pw_phc, totp_secret_b32) VALUES (1, ?1, ?2)",
            rusqlite::params![pw_phc, totp_secret_b32],
        )?;
        Ok(())
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock_ok();
        conn.execute("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value])?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock_ok();
        let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
        let mut rows = stmt.query(rusqlite::params![key])?;
        match rows.next()? {
            Some(r) => Ok(Some(r.get(0)?)),
            None => Ok(None),
        }
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
    fn creds_fresh_store_returns_none() {
        let s = Store::open(":memory:").unwrap();
        assert_eq!(s.get_creds().unwrap(), None);
    }

    #[test]
    fn creds_set_then_get_roundtrips() {
        let s = Store::open(":memory:").unwrap();
        s.set_creds("$argon2id$v=19$m=19456,t=2,p=1$fakehash", "JBSWY3DPEHPK3PXP").unwrap();
        let result = s.get_creds().unwrap();
        assert_eq!(
            result,
            Some(("$argon2id$v=19$m=19456,t=2,p=1$fakehash".to_string(), "JBSWY3DPEHPK3PXP".to_string()))
        );
    }

    #[test]
    fn creds_set_twice_replaces_in_place() {
        let s = Store::open(":memory:").unwrap();
        s.set_creds("hash_v1", "SECRET_V1").unwrap();
        s.set_creds("hash_v2", "SECRET_V2").unwrap();
        let result = s.get_creds().unwrap();
        assert_eq!(result, Some(("hash_v2".to_string(), "SECRET_V2".to_string())));
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

    #[test]
    fn meta_roundtrip_and_missing() {
        let s = Store::open(":memory:").unwrap();
        assert_eq!(s.get_meta("rollback_point").unwrap(), None);
        s.set_meta("rollback_point", "0.14.5").unwrap();
        assert_eq!(s.get_meta("rollback_point").unwrap().as_deref(), Some("0.14.5"));
        s.set_meta("rollback_point", "0.14.7").unwrap();
        assert_eq!(s.get_meta("rollback_point").unwrap().as_deref(), Some("0.14.7"));
    }
}
