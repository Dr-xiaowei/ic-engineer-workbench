//! Opt-in local TLS protocol acceptance; no system trust or real accounts change.
use super::*;
use std::{
    io::{BufRead, BufReader},
    net::{TcpListener, TcpStream as StdTcpStream},
    thread,
};

fn fixtures() -> (native_tls::TlsAcceptor, Vec<u8>) {
    let root = std::path::PathBuf::from(
        std::env::var("ICWB_QA_TLS").expect("explicit synthetic TLS directory"),
    );
    let identity = native_tls::Identity::from_pkcs12(
        &fs::read(root.join("identity.p12")).unwrap(),
        "synthetic-only",
    )
    .unwrap();
    (
        native_tls::TlsAcceptor::new(identity).unwrap(),
        fs::read(root.join("cert.pem")).unwrap(),
    )
}

fn account(imap_port: u16, smtp_port: u16) -> MailAccount {
    MailAccount {
        id: "mail-acceptance".into(),
        address: "engineer@example.invalid".into(),
        display_name: "合成工程邮箱".into(),
        username: "synthetic-user".into(),
        imap_host: "127.0.0.1".into(),
        imap_port,
        smtp_host: "127.0.0.1".into(),
        smtp_port,
        updated_at: 0,
    }
}

fn accepted_tcp(listener: TcpListener) -> StdTcpStream {
    let (tcp, peer) = listener.accept().unwrap();
    assert!(peer.ip().is_loopback());
    tcp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    tcp.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
    tcp
}

#[test]
#[ignore = "requires generated synthetic TLS identity; never adds system trust"]
fn real_tls_imap_readonly_cache_and_smtp_delivery() {
    use lettre::transport::smtp::client::{Certificate, TlsParameters};
    use lettre::Transport;
    let (acceptor, certificate) = fixtures();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut account = account(listener.local_addr().unwrap().port(), 0);
    let imap_acceptor = acceptor.clone();
    let imap = thread::spawn(move || {
        let tls = imap_acceptor.accept(accepted_tcp(listener)).unwrap();
        let mut io = BufReader::new(tls);
        io.get_mut()
            .write_all(b"* OK synthetic IMAP4rev1\r\n")
            .unwrap();
        let raw = concat!("From: reviewer@example.invalid\r\nTo: engineer@example.invalid\r\n",
            "Subject: =?UTF-8?B?5YGP572u5aSN5qC4?=\r\nMIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=qa\r\n\r\n--qa\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n",
            "Synthetic ZXW-100 bias: 10 uA, TT 27 C.\r\n--qa\r\nContent-Type: text/markdown; charset=utf-8\r\n",
            "Content-Disposition: attachment; filename=review.md\r\n\r\n# Review\r\nVDD=1.8 V\r\n--qa--\r\n");
        let mut commands = Vec::new();
        loop {
            let mut line = String::new();
            if io.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let (tag, command) = line.trim_end().split_once(' ').unwrap();
            let upper = command.to_ascii_uppercase();
            commands.push(if upper.starts_with("LOGIN ") {
                "LOGIN".into()
            } else {
                upper.clone()
            });
            let response = if upper.starts_with("LOGIN ") {
                format!("{tag} OK logged in\r\n")
            } else if upper.starts_with("LIST ") {
                format!("* LIST () \"/\" \"INBOX\"\r\n* LIST () \"/\" \"&U,BTFw-\"\r\n{tag} OK list\r\n")
            } else if upper.starts_with("EXAMINE ") {
                format!("* FLAGS (\\Seen)\r\n* 1 EXISTS\r\n* OK [UIDVALIDITY 1] valid\r\n{tag} OK [READ-ONLY] examined\r\n")
            } else if upper.starts_with("UID SEARCH ") {
                format!("* SEARCH 7\r\n{tag} OK search\r\n")
            } else if upper.starts_with("UID FETCH ") {
                format!("* 1 FETCH (UID 7 FLAGS () RFC822.SIZE {} BODY[] {{{}}}\r\n{})\r\n{tag} OK fetch\r\n",raw.len(),raw.len(),raw)
            } else if upper == "LOGOUT" {
                format!("* BYE synthetic logout\r\n{tag} OK logout\r\n")
            } else {
                panic!("unexpected IMAP operation {upper}");
            };
            io.get_mut().write_all(response.as_bytes()).unwrap();
            if upper == "LOGOUT" {
                break;
            }
        }
        commands
    });
    let connector = native_tls::TlsConnector::builder()
        .add_root_certificate(native_tls::Certificate::from_pem(&certificate).unwrap())
        .build()
        .unwrap();
    let parsed = tauri::async_runtime::block_on(async {
        let mut session =
            open_imap_session_with_connector(&account, "synthetic-password", connector)
                .await
                .unwrap();
        assert_eq!(
            read_folder_names(&mut session).await.unwrap(),
            vec!["INBOX", "&U,BTFw-"]
        );
        let parsed = read_folder_messages(&mut session, &account.id, "INBOX")
            .await
            .unwrap();
        session.logout().await.unwrap();
        parsed
    });
    let commands = imap.join().unwrap();
    assert!(commands.iter().any(|s| s.starts_with("EXAMINE ")));
    assert!(commands.iter().any(|s| s.contains("BODY.PEEK[]")));
    assert!(!commands
        .iter()
        .any(|s| s.starts_with("SELECT ") || s.starts_with("STORE ")));
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].record.subject, "偏置复核");
    assert!(parsed[0].record.unread);
    assert_eq!(parsed[0].record.attachments[0].name, "review.md");
    assert!(
        convert_document_bytes("review.md", "md", &parsed[0].attachment_data[0])
            .unwrap()
            .markdown
            .contains("VDD=1.8 V")
    );
    let mut db = Connection::open_in_memory().unwrap();
    crate::storage::initialize(&db).unwrap();
    db.execute("INSERT INTO mail_accounts(id,address,display_name,username,imap_host,imap_port,smtp_host,smtp_port) VALUES('mail-acceptance','engineer@example.invalid','synthetic','qa','127.0.0.1',993,'127.0.0.1',465)",[]).unwrap();
    save_synced(&mut db, &parsed).unwrap();
    assert_eq!(
        local_messages(&db, &account.id, "INBOX", Some("ZXW-100"))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        load_attachment_summaries(&db, &account.id, "INBOX", 7).len(),
        1
    );

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    account.smtp_port = listener.local_addr().unwrap().port();
    let smtp = thread::spawn(move || {
        let mut plain = BufReader::new(accepted_tcp(listener));
        plain
            .get_mut()
            .write_all(b"220 localhost synthetic SMTP\r\n")
            .unwrap();
        let mut line = String::new();
        plain.read_line(&mut line).unwrap();
        assert!(line.starts_with("EHLO "));
        plain
            .get_mut()
            .write_all(b"250-localhost\r\n250 STARTTLS\r\n")
            .unwrap();
        line.clear();
        plain.read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "STARTTLS");
        plain.get_mut().write_all(b"220 begin TLS\r\n").unwrap();
        let mut io = BufReader::new(acceptor.accept(plain.into_inner()).unwrap());
        let mut data = String::new();
        let mut in_data = false;
        let mut authenticated = false;
        loop {
            let mut line = String::new();
            if io.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let response = if in_data {
                if line == ".\r\n" {
                    in_data = false;
                    "250 accepted locally\r\n"
                } else {
                    data.push_str(&line);
                    continue;
                }
            } else if line.starts_with("EHLO ") {
                "250-localhost\r\n250 AUTH PLAIN\r\n"
            } else if line.starts_with("AUTH PLAIN ") {
                authenticated = true;
                "235 authenticated\r\n"
            } else if line.starts_with("MAIL FROM:") || line.starts_with("RCPT TO:") {
                assert!(authenticated);
                "250 OK\r\n"
            } else if line == "DATA\r\n" {
                in_data = true;
                "354 send content\r\n"
            } else if line == "QUIT\r\n" {
                io.get_mut().write_all(b"221 bye\r\n").unwrap();
                break;
            } else {
                panic!("unexpected SMTP operation");
            };
            io.get_mut().write_all(response.as_bytes()).unwrap();
        }
        data
    });
    let parameters = TlsParameters::builder("127.0.0.1".into())
        .add_root_certificate(Certificate::from_pem(&certificate).unwrap())
        .build()
        .unwrap();
    let transport = smtp_transport_with_parameters(
        &account.smtp_host,
        account.smtp_port,
        account.username.clone(),
        "synthetic-password".into(),
        Duration::from_secs(3),
        parameters,
    )
    .unwrap();
    let draft = MailDraft {
        to: vec!["local-receiver@example.invalid".into()],
        cc: vec![],
        subject: "Synthetic local acceptance".into(),
        body: "ZXW-100: 10 uA at TT 27 C.".into(),
        attachment_paths: vec![],
    };
    transport
        .send(&build_outgoing_message(&account, &draft).unwrap())
        .unwrap();
    drop(transport);
    let delivered = smtp.join().unwrap();
    assert!(delivered.contains("ZXW-100: 10 uA at TT 27 C."));
    assert!(delivered.contains("local-receiver@example.invalid"));
}

#[test]
#[ignore = "requires generated untrusted synthetic TLS identity"]
fn production_imap_connector_rejects_untrusted_certificate() {
    let (acceptor, _) = fixtures();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let account = account(listener.local_addr().unwrap().port(), 0);
    let server = thread::spawn(move || {
        if let Ok(tls) = acceptor.accept(accepted_tcp(listener)) {
            let mut io = BufReader::new(tls);
            let _ = io.get_mut().write_all(b"* OK test\r\n");
            let mut line = String::new();
            let _ = io.read_line(&mut line);
            assert!(!line.contains("LOGIN"));
        }
    });
    let result = tauri::async_runtime::block_on(open_imap_session(&account, "synthetic-password"));
    assert!(result.err().unwrap().contains("证书"));
    server.join().unwrap();
}
