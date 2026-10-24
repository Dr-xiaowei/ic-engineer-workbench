use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("icwb-upgrade-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn database(&self) -> PathBuf {
        self.0.join("中文 用户 data.sqlite3")
    }
    fn backups(&self) -> Vec<PathBuf> {
        fs::read_dir(self.0.join("migration-backups"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "icwb"))
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn legacy_database(path: &Path) -> Connection {
    let db = Connection::open(path).unwrap();
    initialize(&db).unwrap();
    db.execute_batch("INSERT INTO notes(id,title,body) VALUES('note-personal','个人评审','Iq=10 µA，TT，27°C');
        INSERT INTO conversations(id,title) VALUES('chat-personal','项目对话');
        INSERT INTO messages(id,conversation_id,position,role,text) VALUES('message-personal','chat-personal',0,'user','只使用合成数据');
        INSERT INTO app_settings(key,value) VALUES('personal-preference','dark');
        DROP TABLE note_reviews; DROP TABLE review_sources; DROP TABLE workspace_state;
        PRAGMA user_version=9;").unwrap();
    db
}

#[test]
fn upgrade_creates_verified_old_snapshot_and_keeps_personal_data() {
    let fixture = Fixture::new();
    drop(legacy_database(&fixture.database()));
    let upgraded = open_database_path(&fixture.database()).unwrap();
    assert_eq!(
        upgraded
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
    for (sql, expected) in [
        (
            "SELECT body FROM notes WHERE id='note-personal'",
            "Iq=10 µA，TT，27°C",
        ),
        (
            "SELECT text FROM messages WHERE id='message-personal'",
            "只使用合成数据",
        ),
        (
            "SELECT value FROM app_settings WHERE key='personal-preference'",
            "dark",
        ),
    ] {
        assert_eq!(
            upgraded
                .query_row(sql, [], |r| r.get::<_, String>(0))
                .unwrap(),
            expected
        );
    }
    assert_eq!(fixture.backups().len(), 1);
    let backup_path = &fixture.backups()[0];
    let bytes = fs::read(backup_path).unwrap();
    let backup =
        Connection::open_with_flags(backup_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        9
    );
    assert_eq!(
        backup
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        backup
            .query_row("SELECT body FROM notes", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Iq=10 µA，TT，27°C"
    );
    drop(backup);
    drop(upgraded);
    for _ in 0..5 {
        drop(open_database_path(&fixture.database()).unwrap());
    }
    assert_eq!(
        fixture.backups().len(),
        1,
        "reopening does not make duplicate backups"
    );
    assert_eq!(fs::read(backup_path).unwrap(), bytes);
}

#[test]
fn failed_migration_rolls_back_all_ddl_and_keeps_recovery_snapshot() {
    let fixture = Fixture::new();
    let db = Connection::open(fixture.database()).unwrap();
    // A deliberately malformed old database makes ALTER fail after CREATE TABLE.
    db.execute_batch(
        "CREATE TABLE skills(id TEXT PRIMARY KEY, version TEXT);
        INSERT INTO skills VALUES('old','keep-me'); PRAGMA user_version=6;",
    )
    .unwrap();
    drop(db);
    let before = fs::read(fixture.database()).unwrap();
    assert!(open_database_path(&fixture.database()).is_err());
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
    let db = Connection::open(fixture.database()).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT version FROM skills", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "keep-me"
    );
    assert_eq!(fixture.backups().len(), 1);
}

#[test]
fn unavailable_backup_prevents_any_upgrade_write() {
    let fixture = Fixture::new();
    drop(legacy_database(&fixture.database()));
    fs::write(fixture.0.join("migration-backups"), b"not a directory").unwrap();
    let before = fs::read(fixture.database()).unwrap();
    assert!(open_database_path(&fixture.database()).is_err());
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
}

#[test]
fn future_and_unversioned_databases_are_never_overwritten() {
    for version in [0, SCHEMA_VERSION + 1] {
        let fixture = Fixture::new();
        let db = Connection::open(fixture.database()).unwrap();
        db.execute_batch(
            "CREATE TABLE personal(value TEXT); INSERT INTO personal VALUES('retain');",
        )
        .unwrap();
        db.pragma_update(None, "user_version", version).unwrap();
        drop(db);
        let before = fs::read(fixture.database()).unwrap();
        assert!(open_database_path(&fixture.database()).is_err());
        assert_eq!(fs::read(fixture.database()).unwrap(), before);
        assert!(fixture.backups().is_empty());
    }
}

#[test]
fn upgrade_snapshot_includes_committed_wal_data_and_can_be_restored() {
    let fixture = Fixture::new();
    let writer = legacy_database(&fixture.database());
    writer.pragma_update(None, "journal_mode", "WAL").unwrap();
    writer
        .execute(
            "UPDATE notes SET body='WAL 中最新的 12 µA 合成记录' WHERE id='note-personal'",
            [],
        )
        .unwrap();
    let upgraded = open_database_path(&fixture.database()).unwrap();
    assert_eq!(
        upgraded
            .query_row("SELECT body FROM notes", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "WAL 中最新的 12 µA 合成记录"
    );
    let snapshot = Connection::open_with_flags(
        &fixture.backups()[0],
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut restored = Connection::open_in_memory().unwrap();
    restore_with_connections(&snapshot, &mut restored).unwrap();
    assert_eq!(
        restored
            .query_row("SELECT body FROM notes", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "WAL 中最新的 12 µA 合成记录"
    );
    assert_eq!(
        restored
            .query_row(
                "SELECT value FROM app_settings WHERE key='network_access_enabled'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "false"
    );
}

#[test]
#[ignore = "requires explicitly selected synthetic schema 9 export from the released application"]
fn actual_schema9_upgrade_preserves_every_existing_table_and_original_file() {
    use rusqlite::types::Value;
    let original =
        PathBuf::from(std::env::var("ICWB_QA_BACKUP").expect("synthetic schema 9 UI export"));
    let before = fs::read(&original).unwrap();
    let fixture = Fixture::new();
    fs::write(fixture.database(), &before).unwrap();
    let old = Connection::open_with_flags(
        fixture.database(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        old.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        9
    );
    let names = old.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    fn rows(db: &Connection, table: &str) -> Vec<Vec<Value>> {
        let mut statement = db
            .prepare(&format!("SELECT * FROM \"{}\"", table.replace('"', "\"\"")))
            .unwrap();
        let columns = statement.column_count();
        let mut values: Vec<Vec<Value>> = statement
            .query_map([], |row| {
                (0..columns).map(|i| row.get::<_, Value>(i)).collect()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        values.sort_by_cached_key(|row| format!("{row:?}"));
        values
    }
    let expected: Vec<_> = names.iter().map(|name| rows(&old, name)).collect();
    drop(old);
    let upgraded = open_database_path(&fixture.database()).unwrap();
    for (name, expected) in names.iter().zip(expected) {
        assert_eq!(rows(&upgraded, name), expected, "table {name}");
    }
    assert_eq!(
        fs::read(original).unwrap(),
        before,
        "selected source remains unchanged"
    );
    assert_eq!(fixture.backups().len(), 1);
}
