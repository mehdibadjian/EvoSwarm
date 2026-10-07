//! Web-based test triage interface (e5-5, AD-5).
//!
//! Provides an HTTP server strictly bound to 127.0.0.1 (rejecting external interfaces)
//! for reviewing candidate adversary tests, inspecting which candidates failed them,
//! and 1-click promoting or rejecting tests using the E2-6 workflow.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

use crate::approve_tests::{approve_candidate_tests, reject_candidate_tests};

#[derive(Debug, Error)]
pub enum TriageError {
    #[error("bind address '{0}' is not a local loopback address (127.0.0.1 required)")]
    NonLoopbackAddress(String),
}

/// Validates that a socket bind address binds strictly to 127.0.0.1.
pub fn validate_bind_address(addr_str: &str) -> Result<SocketAddr, TriageError> {
    let addr: SocketAddr = addr_str
        .parse()
        .map_err(|_| TriageError::NonLoopbackAddress(addr_str.to_string()))?;

    if addr.ip() != std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)) {
        return Err(TriageError::NonLoopbackAddress(addr_str.to_string()));
    }

    Ok(addr)
}

/// Shared state for the triage web server.
pub struct TriageServerState {
    pub repo_path: PathBuf,
    pub spool_dir: PathBuf,
}

#[derive(Deserialize)]
pub struct TriageActionParams {
    pub job_id: String,
    pub id: String,
}

/// Creates the axum router for the test triage page.
pub fn create_triage_router(state: Arc<TriageServerState>) -> Router {
    Router::new()
        .route("/triage", get(handle_get_triage))
        .route("/triage/approve", post(handle_post_approve))
        .route("/triage/reject", post(handle_post_reject))
        .with_state(state)
}

/// GET /triage: renders HTML page listing candidate adversary tests.
async fn handle_get_triage(State(state): State<Arc<TriageServerState>>) -> Response {
    let mut html = String::from(
        "<!DOCTYPE html><html><head><title>EvoSwarm Test Triage</title></head><body>\n\
        <h1>Candidate Adversary Tests</h1>\n<ul>\n",
    );

    // Scan jobs in spool_dir
    if state.spool_dir.exists() {
        if let Ok(entries) = fs::read_dir(&state.spool_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(record) = serde_json::from_str::<evoswarm_memory::falkordb::JobRecord>(&content) {
                            for ev in &record.evaluations {
                                for t in &ev.tests {
                                    if t.origin == "adversary" {
                                        let fail_note = if !t.passed {
                                            format!("(failed by candidate `{}`)", ev.candidate_id)
                                        } else {
                                            "(passed)".to_string()
                                        };
                                        html.push_str(&format!(
                                            "<li><strong>{}</strong> {} [Job: {}] \
                                            <form method=\"POST\" action=\"/triage/approve?job_id={}&id={}\" style=\"display:inline;\"><button>Approve</button></form> \
                                            <form method=\"POST\" action=\"/triage/reject?job_id={}&id={}\" style=\"display:inline;\"><button>Reject</button></form></li>\n",
                                            t.name, fail_note, record.task.id, record.task.id, t.name, record.task.id, t.name
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    html.push_str("</ul></body></html>");
    Html(html).into_response()
}

/// POST /triage/approve: promotes test using E2-6 workflow.
async fn handle_post_approve(
    State(state): State<Arc<TriageServerState>>,
    Query(params): Query<TriageActionParams>,
) -> Response {
    match approve_candidate_tests(
        &params.job_id,
        &[params.id],
        &state.repo_path,
        &state.spool_dir,
    ) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

/// POST /triage/reject: rejects test using E2-6 workflow.
async fn handle_post_reject(
    State(state): State<Arc<TriageServerState>>,
    Query(params): Query<TriageActionParams>,
) -> Response {
    match reject_candidate_tests(
        &params.job_id,
        &[params.id],
        &state.repo_path,
        &state.spool_dir,
    ) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
