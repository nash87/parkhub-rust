//! Structural guard: every `/api/v1/admin/*` route must be authorized.
//!
//! Admin routes are protected two different ways in this codebase.
//! `admin_core_routes()` is wrapped once with `admin_middleware`, and every
//! route registered there is covered by construction. Routes registered on
//! the other sub-routers are deliberately *not* wrapped (see the comment
//! above the merge in `api::mod`) and are expected to call `check_admin`
//! themselves.
//!
//! That second contract is invisible: nothing enforced it, and three
//! handlers silently lost it — `admin_update_operating_hours` and both
//! dynamic-pricing handlers took no `AuthUser` at all, so any authenticated
//! non-admin could rewrite a lot's operating hours (which gate booking
//! creation) or its pricing rules. Their OpenAPI docs even advertised
//! "403 Admin access required" while the handler never checked.
//!
//! This test makes the contract enforceable: add an admin route outside the
//! guarded core without an admin check and it fails, naming the route.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

/// Handlers that intentionally carry no `AuthUser`, with the reason.
///
/// Keep this list short and justified. Anything added here is an explicit
/// decision to authorize a route some other way, not a place to silence the
/// guard.
const AUTHORIZED_WITHOUT_AUTH_USER: &[(&str, &str)] = &[(
    "download_audit_export",
    "authorized by a single-use, expiring download token validated in the handler",
)];

fn api_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api")
}

/// Extract the body of `pub async fn <name>` by brace matching, so a scan
/// cannot bleed into the following function and report a neighbour's guard.
fn function_body(source: &str, name: &str) -> Option<String> {
    let needle = format!("pub async fn {name}(");
    let start = source.find(&needle)?;
    let open = source[start..].find('{')? + start;

    let mut depth = 0usize;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(source[start..open + offset + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Line span of `fn admin_core_routes`, whose routes are covered by the
/// `admin_middleware` layer rather than per-handler checks.
fn admin_core_span(mod_rs: &str) -> (usize, usize) {
    let start_byte = mod_rs
        .find("fn admin_core_routes")
        .expect("admin_core_routes() should exist in api::mod");
    let start_line = mod_rs[..start_byte].lines().count();

    let end_byte = mod_rs[start_byte..]
        .find("\n}")
        .map_or(mod_rs.len(), |o| start_byte + o);
    let end_line = mod_rs[..end_byte].lines().count();

    (start_line, end_line)
}

#[test]
fn every_admin_route_outside_the_guarded_core_checks_admin() {
    let api = api_dir();
    let mod_rs = fs::read_to_string(api.join("mod.rs")).expect("read api/mod.rs");
    let (core_start, core_end) = admin_core_span(&mod_rs);

    // All handler sources concatenated — a route in mod.rs may point at a
    // handler defined in any sibling module.
    let mut sources = String::new();
    for entry in fs::read_dir(&api).expect("read api dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            sources.push_str(&fs::read_to_string(&path).expect("read handler source"));
            sources.push('\n');
        }
    }

    let handler_re =
        Regex::new(r"(?:get|post|put|patch|delete)\(\s*([A-Za-z0-9_:]+)").expect("valid regex");

    let lines: Vec<&str> = mod_rs.lines().collect();
    let mut unguarded: BTreeSet<String> = BTreeSet::new();

    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        if !line.contains("\"/api/v1/admin/") {
            continue;
        }
        // Routes inside admin_core_routes() are covered by the middleware.
        if line_no >= core_start && line_no <= core_end {
            continue;
        }

        let route = line
            .split('"')
            .find(|s| s.starts_with("/api/v1/admin/"))
            .unwrap_or("<unknown>");

        // A registration commonly spans several lines: the path on one, the
        // method/handler pairs on the next few.
        let window = lines[idx..lines.len().min(idx + 4)].join("\n");

        for capture in handler_re.captures_iter(&window) {
            let full = &capture[1];
            let name = full.rsplit("::").next().unwrap_or(full);

            if AUTHORIZED_WITHOUT_AUTH_USER.iter().any(|(h, _)| *h == name) {
                continue;
            }

            let Some(body) = function_body(&sources, name) else {
                // Not a handler we can resolve (e.g. a closure or a
                // re-exported extern). Nothing to assert.
                continue;
            };

            let extracts_identity = body.contains("Extension<AuthUser>");
            let checks_admin = body.contains("check_admin");

            if !extracts_identity && !checks_admin {
                unguarded.insert(format!("mod.rs:{line_no}  {route}  ->  {name}"));
            }
        }
    }

    assert!(
        unguarded.is_empty(),
        "These /api/v1/admin routes are registered outside admin_core_routes() (so \
         admin_middleware does not cover them) and their handlers neither extract \
         Extension<AuthUser> nor call check_admin. Any authenticated user can reach \
         them:\n  {}\n\nEither move the route into admin_core_routes(), add a \
         check_admin call to the handler, or — if it is authorized some other way — \
         add it to AUTHORIZED_WITHOUT_AUTH_USER with the reason.",
        unguarded.into_iter().collect::<Vec<_>>().join("\n  "),
    );
}
