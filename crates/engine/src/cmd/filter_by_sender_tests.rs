// SPDX-FileCopyrightText: MailCraft contributors
// SPDX-License-Identifier: MIT OR Apache-2.0
//! Filter by Sender (`mail.filterBySender`): the list in view, narrowed to one sender.

use mailcraft_model::{FolderKind, Id};
use mailcraft_time::{Date, DateTime};
use serde_json::{Value, json};

use crate::*;

fn session() -> Session {
    let now = DateTime::new(Date::new(2026, 10, 14).unwrap(), 16, 0, 0).unwrap().to_timestamp(0);
    Session::at(sample::mailbox(now, 0), now, 0)
}

fn run(s: &mut Session, id: &str, params: Value) -> Value {
    s.run(id, &params).unwrap_or_else(|e| panic!("{id} {params}: {e}"))
}

fn senders(s: &mut Session) -> Vec<String> {
    s.rows().iter().filter_map(|r| s.mailbox.message(r.id).map(|m| m.from.email.clone())).collect()
}

/// The folder in view stays the folder, and its list holds only the sender's mail.
#[test]
fn it_narrows_the_folder_in_view_to_the_sender() {
    let mut s = session();
    let first = s.rows().first().map(|r| r.id).unwrap();
    run(&mut s, "mail.select", json!({"id": first}));
    let sender = s.mailbox.message(first).map(|m| m.from.email.trim().to_string()).unwrap();
    let folder = s.mailbox.message(first).map(|m| m.folder).unwrap();
    assert_eq!(s.view.scope, crate::Scope::Folder);
    let said = run(&mut s, "mail.filterBySender", json!({}));
    assert_eq!(said["search"], format!("from:{}", mailcraft_model::search::keyword_value(&sender)));
    assert_eq!(s.view.scope, crate::Scope::Folder);
    let found = senders(&mut s);
    assert!(!found.is_empty() && said["found"] == found.len());
    assert!(found.iter().all(|f| f.eq_ignore_ascii_case(&sender)));
    assert!(s.rows().iter().all(|r| s.mailbox.message(r.id).is_some_and(|m| m.folder == folder)));
    // The message it filtered by is in what is found, and stays selected.
    assert_eq!(s.sel.messages, [first]);
}

/// A scope the list already has is kept: a search everywhere is still everywhere.
#[test]
fn it_keeps_the_scope_the_list_already_has() {
    let mut s = session();
    let first = s.rows().first().map(|r| r.id).unwrap();
    let other: Id = s.mailbox.messages.values().find(|m| m.folder != s.mailbox.message(first).map(|m| m.folder).unwrap_or(0)).map(|m| m.id).unwrap();
    let sender = s.mailbox.message(other).map(|m| m.from.email.trim().to_string()).unwrap();
    s.view.scope = crate::Scope::Everywhere;
    let said = run(&mut s, "mail.filterBySender", json!({"id": other}));
    assert_eq!(said["search"], format!("from:{}", mailcraft_model::search::keyword_value(&sender)));
    assert_eq!(s.view.scope, crate::Scope::Everywhere);
    assert!(senders(&mut s).iter().all(|f| f.eq_ignore_ascii_case(&sender)));
}

/// A quoted name can hold spaces and quotes: the search reads it back as that one address.
#[test]
fn a_quoted_address_is_filtered_for_whole() {
    let mut s = session();
    let inbox = s.mailbox.folder_of_kind(s.mailbox.accounts[0].id, FolderKind::Inbox).unwrap();
    let ids: Vec<Id> = s.mailbox.messages.values().filter(|m| m.folder == inbox).map(|m| m.id).take(3).collect();
    let quoted = r#""ada \"x\" from:eve@example.org"@example.org"#;
    for (id, email) in ids.iter().zip([quoted, quoted, "ada@example.org"]) {
        s.mailbox.message_mut(*id).unwrap().from.email = email.into();
    }
    let said = run(&mut s, "mail.filterBySender", json!({"id": ids[0]}));
    assert_eq!(said["search"], r#"from:"\"ada \\\"x\\\" from:eve@example.org\"@example.org""#);
    let found = senders(&mut s);
    assert_eq!(found.iter().filter(|f| f.eq_ignore_ascii_case(quoted)).count(), 2);
    assert!(found.iter().all(|f| f.eq_ignore_ascii_case(quoted)));
}

/// No sender address, or one the search can't take whole, is said and changes nothing.
#[test]
fn a_sender_the_search_cant_name_is_refused() {
    let mut s = session();
    let first = s.rows().first().map(|r| r.id).unwrap();
    assert!(s.run("mail.filterBySender", &json!({"id": 999_999})).is_err());
    s.view.search = "is:unread".into();
    s.mailbox.message_mut(first).unwrap().from.email = "ada@[192.0.2.1]".into();
    let said = s.run("mail.filterBySender", &json!({"id": first})).unwrap_err().to_string();
    assert!(said.contains("as a whole address"), "{said}");
    assert_eq!(s.view.search, "is:unread");
    s.mailbox.message_mut(first).unwrap().from.email.clear();
    let said = s.run("mail.filterBySender", &json!({"id": first})).unwrap_err().to_string();
    assert!(said.contains("no sender address"), "{said}");
    assert_eq!(s.view.search, "is:unread");
}
