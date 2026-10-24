-- Schema only from the synthetic v1.0.1 UI export; no user rows or paths.
-- Source export SHA256: 29dc2731ed97cc8a079549754165e209b3fe87a9c3d4ec32b92101acf7347732
CREATE TABLE conversations (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE messages (
               id TEXT PRIMARY KEY,
               conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
               position INTEGER NOT NULL,
               role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
               text TEXT NOT NULL,
               request_id TEXT
             );

CREATE TABLE attachments (
               id TEXT PRIMARY KEY,
               message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
               position INTEGER NOT NULL,
               name TEXT NOT NULL,
               mime_type TEXT NOT NULL,
               data_url TEXT NOT NULL,
               size_bytes INTEGER NOT NULL
             );

CREATE TABLE document_attachments (
               id TEXT PRIMARY KEY,
               message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
               position INTEGER NOT NULL,
               source_name TEXT NOT NULL,
               source_kind TEXT NOT NULL,
               markdown TEXT NOT NULL,
               warnings_json TEXT NOT NULL DEFAULT '[]'
             );

CREATE TABLE projects (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               root_path TEXT NOT NULL UNIQUE,
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE project_files (
               project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
               relative_path TEXT NOT NULL,
               source_name TEXT NOT NULL,
               source_kind TEXT NOT NULL,
               size_bytes INTEGER NOT NULL,
               modified_at INTEGER NOT NULL,
               document_id TEXT,
               markdown TEXT,
               page_count INTEGER,
               warnings_json TEXT NOT NULL DEFAULT '[]',
               index_error TEXT,
               indexed_at INTEGER NOT NULL DEFAULT (unixepoch()),
               PRIMARY KEY (project_id, relative_path)
             );

CREATE VIRTUAL TABLE project_search USING fts5(
               project_id UNINDEXED,
               relative_path UNINDEXED,
               source_name UNINDEXED,
               markdown,
               tokenize = 'unicode61 remove_diacritics 2'
             );

CREATE TABLE skills (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               description TEXT NOT NULL,
               instructions TEXT NOT NULL,
               source_name TEXT NOT NULL,
               enabled INTEGER NOT NULL DEFAULT 0,
               has_scripts INTEGER NOT NULL DEFAULT 0,
               version TEXT NOT NULL DEFAULT 'unspecified',
               source_type TEXT NOT NULL DEFAULT 'local-file',
               permissions_json TEXT NOT NULL DEFAULT '["prompt"]',
               has_restricted_permissions INTEGER NOT NULL DEFAULT 0,
               imported_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE mail_accounts (
               id TEXT PRIMARY KEY,
               address TEXT NOT NULL,
               display_name TEXT NOT NULL,
               username TEXT NOT NULL,
               imap_host TEXT NOT NULL,
               imap_port INTEGER NOT NULL,
               smtp_host TEXT NOT NULL,
               smtp_port INTEGER NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE mail_messages (
               account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
               folder TEXT NOT NULL,
               uid INTEGER NOT NULL,
               message_id TEXT,
               subject TEXT NOT NULL,
               sender TEXT NOT NULL,
               recipients TEXT NOT NULL,
               sent_at INTEGER,
               preview TEXT NOT NULL,
               body_text TEXT NOT NULL,
               unread INTEGER NOT NULL DEFAULT 0,
               raw_size INTEGER NOT NULL DEFAULT 0,
               synced_at INTEGER NOT NULL DEFAULT (unixepoch()),
               PRIMARY KEY (account_id, folder, uid)
             );

CREATE VIRTUAL TABLE mail_search USING fts5(
               account_id UNINDEXED,
               folder UNINDEXED,
               uid UNINDEXED,
               subject,
               sender,
               recipients,
               body_text,
               tokenize = 'unicode61 remove_diacritics 2'
             );

CREATE TABLE mail_attachments (
               account_id TEXT NOT NULL,
               folder TEXT NOT NULL,
               uid INTEGER NOT NULL,
               position INTEGER NOT NULL,
               name TEXT NOT NULL,
               mime_type TEXT NOT NULL,
               size_bytes INTEGER NOT NULL,
               data BLOB NOT NULL,
               PRIMARY KEY (account_id, folder, uid, position),
               FOREIGN KEY (account_id, folder, uid)
                 REFERENCES mail_messages(account_id, folder, uid) ON DELETE CASCADE
             );

CREATE TABLE app_settings (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL,
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE audit_events (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               action TEXT NOT NULL,
               outcome TEXT NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE project_progress (
               project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
               start_date TEXT NOT NULL DEFAULT '',
               end_date TEXT NOT NULL DEFAULT '',
               current_phase TEXT NOT NULL DEFAULT '',
               milestone TEXT NOT NULL DEFAULT '',
               weekly_focus TEXT NOT NULL DEFAULT '',
               current_focus TEXT NOT NULL DEFAULT '',
               risks TEXT NOT NULL DEFAULT '',
               progress_percent INTEGER NOT NULL DEFAULT 0 CHECK(progress_percent BETWEEN 0 AND 100),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE calendar_tasks (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               start_at INTEGER NOT NULL,
               end_at INTEGER,
               status TEXT NOT NULL CHECK(status IN ('todo','doing','done')),
               priority TEXT NOT NULL CHECK(priority IN ('normal','important')),
               remind_at INTEGER,
               project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
               notes TEXT NOT NULL DEFAULT '',
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE software_launchers (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               target_path TEXT NOT NULL UNIQUE,
               target_kind TEXT NOT NULL CHECK(target_kind IN ('application','executable')),
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE project_annotations (
               project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
               starred INTEGER NOT NULL DEFAULT 0 CHECK(starred IN (0,1)),
               note TEXT NOT NULL DEFAULT '',
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE project_progress_history (
               id TEXT PRIMARY KEY,
               project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
               snapshot_json TEXT NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE TABLE notes (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               body TEXT NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );

CREATE INDEX messages_conversation_position
               ON messages(conversation_id, position);

CREATE INDEX attachments_message_position
               ON attachments(message_id, position);

CREATE INDEX document_attachments_message_position
               ON document_attachments(message_id, position);

CREATE INDEX project_files_document_id
               ON project_files(document_id);

CREATE INDEX calendar_tasks_date ON calendar_tasks(start_at,status);

CREATE INDEX calendar_tasks_project ON calendar_tasks(project_id,start_at);

CREATE INDEX project_progress_history_project
               ON project_progress_history(project_id,created_at DESC);
PRAGMA user_version=9;
