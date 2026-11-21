// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// What the Privacy notice and the software license agreement promise about the app, kept true by tests
// (the final legal documents, 2026-11-21): no network except the four things the person asks for, no identifiers in
// the update check, typed text never logged, diagnostics without typed text.
#![cfg(test)]

use std::fs;
use std::path::{Path, PathBuf};

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            if !matches!(p.file_name().and_then(|n| n.to_str()), Some("target" | "node_modules" | "dist")) {
                sources(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn all_sources() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for d in ["app/src", "tsf/src", "crates/likhon-engine/src", "crates/likhon-store/src", "crates/likhon-ffi/src"] {
        sources(&root.join(d), &mut files);
    }
    files.into_iter().filter(|p| !p.ends_with("privacy_tests.rs")).map(|p| { let t = fs::read_to_string(&p).unwrap(); (p, t) }).collect()
}

/// Only net.rs may open a connection, and no other crate links a network library.
#[test]
fn only_net_rs_talks_to_the_network() {
    for (p, text) in all_sources() {
        let name = p.file_name().unwrap().to_str().unwrap();
        if name == "net.rs" {
            continue;
        }
        for needle in ["WinHttp", "winhttp", "TcpStream", "UdpSocket", "reqwest::", "ureq::", "hyper::", "InternetOpen", "URLSession"] {
            assert!(!text.contains(needle), "{} uses {needle}: only app/src/net.rs may talk to the network", p.display());
        }
    }
}

/// No HTTP client library is in the dependency list at all (the only network code is net.rs, on WinHTTP).
#[test]
fn no_http_client_crate_is_linked() {
    let lock = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.lock")).unwrap();
    for krate in ["reqwest", "hyper", "ureq", "isahc", "surf", "attohttpc", "curl", "minreq"] {
        assert!(!lock.contains(&format!("name = \"{krate}\"")), "Cargo.lock has the HTTP client crate {krate}");
    }
}

/// The four user actions are the only endpoints: update check, library, dictionary download, word donation.
#[test]
fn the_only_endpoints_are_the_four_the_person_asks_for() {
    let allowed = ["/api/app/latest", "/api/app/library", "/api/app/dictionary", "/api/dictionary/donate"];
    for (p, text) in all_sources() {
        for part in text.split("\"{}/api/").skip(1).chain(text.split("}/api/").skip(1)) {
            let path: String = format!("/api/{}", part.split(['"', '?', '`']).next().unwrap_or(""));
            assert!(allowed.contains(&path.as_str()), "{} calls {path}, which is not one of the four user actions", p.display());
        }
    }
}

/// The update check sends the plain request: no query, no body, no identifiers or cookies of any kind.
#[test]
fn the_update_check_sends_no_identifiers() {
    let online = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/online.rs")).unwrap();
    assert!(online.contains("json_from(&format!(\"{}/api/app/latest\", net::site()))"), "the update check must be the bare GET");
    let net = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/net.rs")).unwrap();
    for needle in ["Cookie", "User-Agent", "Authorization", "X-", "machine_id", "MachineGuid", "GetComputerName", "hostname"] {
        assert!(!net.contains(needle), "net.rs must not send {needle}");
    }
}

/// Typed text is never written to a log: no printing, logging or debug-output calls in the typing paths, at any level.
#[test]
fn typed_text_is_never_logged() {
    for (p, text) in all_sources() {
        let name = p.file_name().unwrap().to_str().unwrap().to_string();
        let in_typing_path = p.components().any(|c| matches!(c.as_os_str().to_str(), Some("tsf" | "likhon-engine" | "likhon-store" | "likhon-ffi")))
            || matches!(name.as_str(), "agent.rs" | "suggest.rs" | "tsfbridge.rs" | "words.rs" | "legacy.rs" | "quick.rs" | "topbar.rs" | "osk.rs");
        if !in_typing_path {
            continue;
        }
        // The shipped code only: stop at the test module.
        let shipped = text.split("#[cfg(test)]").next().unwrap_or("");
        for line in shipped.lines() {
            let code = line.split("//").next().unwrap_or("");
            for needle in ["println!", "eprintln!", "print!(", "dbg!", "OutputDebugString", "log::", "tracing::", "write_log"] {
                assert!(!code.contains(needle), "{}: {needle} in a typing path: {line}", p.display());
            }
        }
    }
}

/// The diagnostics file lists versions, settings and recent errors only (Settings > Privacy).
#[test]
fn diagnostics_hold_no_typed_text() {
    let src = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/settings_app.rs")).unwrap();
    let start = src.find("fn diagnostics() -> String").expect("diagnostics fn");
    let body = &src[start..start + src[start..].find("\n}\n").unwrap()];
    for needle in ["learned", "user_db", "UserData", "words", "clipboard", "text"] {
        assert!(!body.to_lowercase().contains(needle), "diagnostics() must not read {needle}");
    }
    // It names changed settings, never their values.
    assert!(body.contains("stored.keys()"));
}
