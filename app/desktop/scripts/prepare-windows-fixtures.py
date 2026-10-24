"""Create disposable test inputs from the released schema, never a personal DB."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
root = args.output.resolve()
root.mkdir(parents=True, exist_ok=False)
project = root / "中文 工程资料"
project.mkdir()
(project / "偏置 ZXW100.md").write_text("# 合成验证资料\nZXW100 静态电流 Iq=10 µA，VDD=1.8 V，TT，27°C。\n", encoding="utf-8")
schema = Path(__file__).resolve().parent.parent / "tests/fixtures/schema9.sql"
backup = root / "synthetic-schema9.icwb"
with sqlite3.connect(backup) as db:
    db.executescript(schema.read_text(encoding="utf-8"))
    db.execute("INSERT INTO projects(id,name,root_path) VALUES(?,?,?)", ("synthetic-project", "合成偏置项目", str(project)))
    db.execute("INSERT INTO notes(id,title,body) VALUES(?,?,?)", ("synthetic-note", "升级保留笔记", "Iq=10 µA，TT，27°C；仅测试数据"))
    db.execute("INSERT INTO conversations(id,title) VALUES(?,?)", ("synthetic-chat", "合成设计评审"))
    db.execute("INSERT INTO messages(id,conversation_id,position,role,text) VALUES(?,?,?,?,?)", ("synthetic-message", "synthetic-chat", 0, "user", "保留单位与测试条件"))
    db.execute("INSERT INTO calendar_tasks(id,title,start_at,status,priority,project_id) VALUES(?,?,?,?,?,?)", ("synthetic-task", "复核偏置", 1800000000, "todo", "important", "synthetic-project"))
    db.execute("INSERT INTO project_progress(project_id,progress_percent) VALUES(?,?)", ("synthetic-project", 42))
    db.execute("INSERT INTO project_annotations(project_id,starred,note) VALUES(?,?,?)", ("synthetic-project", 1, "合成星标备注"))
    for key, value in [("ui_backup_state", "{}"), ("network_access_enabled", "true"), ("personal-preference", "dark")]:
        db.execute("INSERT INTO app_settings(key,value) VALUES(?,?)", (key, value))
    assert db.execute("PRAGMA integrity_check").fetchone() == ("ok",)
tls = root / "tls"
tls.mkdir()
subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2", "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost,IP:127.0.0.1", "-keyout", str(tls / "key.pem"), "-out", str(tls / "cert.pem")], check=True, capture_output=True)
subprocess.run(["openssl", "pkcs12", "-export", "-inkey", str(tls / "key.pem"), "-in", str(tls / "cert.pem"), "-out", str(tls / "identity.p12"), "-passout", "pass:synthetic-only", "-keypbe", "PBE-SHA1-3DES", "-certpbe", "PBE-SHA1-3DES", "-macalg", "sha1"], check=True, capture_output=True)
manifest = {"synthetic": True, "schemaVersion": 9, "schemaSource": "v1.0.1 synthetic UI export (schema only)", "backupSha256": hashlib.sha256(backup.read_bytes()).hexdigest(), "schemaSha256": hashlib.sha256(schema.read_bytes()).hexdigest()}
(root / "provenance.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
print("Synthetic fixtures created:", root)
