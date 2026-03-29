use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
    routing::any,
    Router,
};
use clap::{Parser, Subcommand};
use gate::{
    estimate_cost, AuditEvent, AuditStatus, AuditWriter, BudgetAction, BudgetStore, SpendResult,
};
use reqwest::Client;
use serde_json::Value;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(
    name = "llm-gate",
    about = "LLM cost-control proxy and budget manager",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Proxy {
        #[arg(long, default_value = "127.0.0.1:7777")]
        listen: SocketAddr,
        #[arg(long, default_value = "https://api.anthropic.com")]
        target: String,
        #[arg(long, default_value = "default")]
        label: String,
        #[arg(long, default_value_t = 0.0)]
        budget: f64,
        #[arg(long)]
        audit: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Budget {
        #[arg(long, default_value = "budgets.json")]
        config: PathBuf,
        #[command(subcommand)]
        action: BudgetCommands,
    },
    Audit {
        file: PathBuf,
        #[arg(long, default_value_t = 20)]
        tail: usize,
    },
}

#[derive(Subcommand)]
enum BudgetCommands {
    Add {
        label: String,
        limit_usd: f64,
        #[arg(long, default_value = "block")]
        action: String,
    },
    Status,
    Reset {
        label: String,
    },
}

#[derive(Clone)]
struct ProxyState {
    target: String,
    label: String,
    budget: Arc<Mutex<BudgetStore>>,
    audit: Arc<Mutex<AuditWriter>>,
    client: Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Proxy {
            listen,
            target,
            label,
            budget: budget_limit,
            audit,
            config,
        } => {
            let mut store = match config {
                Some(ref path) if path.exists() => BudgetStore::load(path)?,
                _ => BudgetStore::new(),
            };

            if budget_limit > 0.0 {
                let _ = store.add_budget(label.clone(), budget_limit, BudgetAction::Block);
            }

            let audit_writer = match audit {
                Some(ref path) => AuditWriter::file(path)?,
                None => AuditWriter::stdout(),
            };

            let state = ProxyState {
                target: target.trim_end_matches('/').to_string(),
                label,
                budget: Arc::new(Mutex::new(store)),
                audit: Arc::new(Mutex::new(audit_writer)),
                client: Client::new(),
            };

            let app = Router::new()
                .route("/{*path}", any(proxy_handler))
                .route("/", any(proxy_handler))
                .with_state(state);

            info!("llm-gate proxy listening on {}", listen);
            let listener = tokio::net::TcpListener::bind(listen).await?;
            axum::serve(listener, app).await?;
        }

        Commands::Budget { config, action } => {
            let mut store = if config.exists() {
                BudgetStore::load(&config)?
            } else {
                BudgetStore::new()
            };

            match action {
                BudgetCommands::Add {
                    label,
                    limit_usd,
                    action,
                } => {
                    let ba = if action == "warn" {
                        BudgetAction::Warn
                    } else {
                        BudgetAction::Block
                    };
                    store.add_budget(label.clone(), limit_usd, ba)?;
                    println!("Budget set: {} = ${:.2}", label, limit_usd);
                }
                BudgetCommands::Status => {
                    let mut budgets: Vec<_> = store.all().collect();
                    budgets.sort_by(|a, b| a.label.cmp(&b.label));
                    if budgets.is_empty() {
                        println!("No budgets configured.");
                    } else {
                        println!(
                            "{:<20} {:>10} {:>12} {:>8}",
                            "Label", "Limit", "Spent", "Action"
                        );
                        println!("{}", "-".repeat(54));
                        for b in budgets {
                            let action_str = match b.action {
                                BudgetAction::Block => "block",
                                BudgetAction::Warn => "warn",
                            };
                            println!(
                                "{:<20} {:>10.4} {:>12.6} {:>8}",
                                b.label, b.limit_usd, b.spent_usd, action_str
                            );
                        }
                    }
                }
                BudgetCommands::Reset { label } => {
                    store.reset(&label)?;
                    println!("Reset budget: {}", label);
                }
            }
        }

        Commands::Audit { file, tail } => {
            let content = std::fs::read_to_string(&file)?;
            let events: Vec<AuditEvent> = content
                .lines()
                .filter(|l| !l.is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();

            let start = events.len().saturating_sub(tail);
            let recent = &events[start..];

            println!(
                "{:<25} {:<8} {:<30} {:>8} {:>8} {:>10} {}",
                "Timestamp", "Status", "Model", "In Tok", "Out Tok", "Cost USD", "Label"
            );
            println!("{}", "-".repeat(100));
            for e in recent {
                let ts = e.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                let status = format!("{:?}", e.status).to_lowercase();
                println!(
                    "{:<25} {:<8} {:<30} {:>8} {:>8} {:>10.6} {}",
                    ts, status, e.model, e.input_tokens, e.output_tokens, e.cost_usd, e.label
                );
            }
        }
    }

    Ok(())
}

async fn proxy_handler(
    State(state): State<ProxyState>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    let method_str = req.method().as_str().to_owned();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    let target_url = format!("{}{}", state.target, path_and_query);

    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let model = serde_json::from_slice::<Value>(&body_bytes)
        .ok()
        .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from))
        .unwrap_or_else(|| "unknown".to_string());

    {
        let store = state.budget.lock().unwrap();
        if let Some(b) = store.get(&state.label) {
            if b.limit_usd > 0.0 && b.spent_usd >= b.limit_usd {
                if matches!(b.action, BudgetAction::Block) {
                    warn!("Budget pre-blocked: label={}", state.label);
                    let body = serde_json::json!({
                        "error": "budget_exceeded",
                        "spent": b.spent_usd,
                        "limit": b.limit_usd,
                        "label": state.label
                    });
                    return Ok(Response::builder()
                        .status(StatusCode::TOO_MANY_REQUESTS)
                        .header("content-type", "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap());
                }
            }
        }
    }

    // Convert axum/http-1.x types to reqwest/http-0.2 compatible strings
    let reqwest_method =
        reqwest::Method::from_bytes(method_str.as_bytes()).unwrap_or(reqwest::Method::POST);

    let mut req_builder = state.client.request(reqwest_method, &target_url);
    for (key, value) in &headers {
        let name = key.as_str();
        if name != "host" && name != "content-length" {
            if let Ok(rname) = reqwest::header::HeaderName::from_bytes(name.as_bytes()) {
                if let Ok(rvalue) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                    req_builder = req_builder.header(rname, rvalue);
                }
            }
        }
    }
    req_builder = req_builder.body(body_bytes.to_vec());

    let upstream_resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Upstream request failed: {}", e);
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    let status_u16 = upstream_resp.status().as_u16();
    let resp_headers = upstream_resp.headers().clone();
    let resp_bytes = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read upstream response: {}", e);
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    let axum_status = StatusCode::from_u16(status_u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let (input_tokens, output_tokens) = serde_json::from_slice::<Value>(&resp_bytes)
        .ok()
        .and_then(|v| {
            let u = v.get("usage")?;
            let i = u
                .get("input_tokens")
                .or_else(|| u.get("prompt_tokens"))
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            let o = u
                .get("output_tokens")
                .or_else(|| u.get("completion_tokens"))
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            Some((i, o))
        })
        .unwrap_or((0, 0));

    let cost_usd = estimate_cost(&model, input_tokens, output_tokens).unwrap_or(0.0);

    let provider = if state.target.contains("anthropic") {
        "anthropic"
    } else if state.target.contains("openai") {
        "openai"
    } else if state.target.contains("google") || state.target.contains("googleapis") {
        "google"
    } else {
        "unknown"
    };

    let audit_status = if cost_usd > 0.0 {
        match state
            .budget
            .lock()
            .unwrap()
            .record_spend(&state.label, cost_usd)
        {
            Ok(SpendResult::Ok) => AuditStatus::Ok,
            Ok(SpendResult::Warned { spent, limit }) => {
                warn!(
                    "Budget warning: label={} spent=${:.4} limit=${:.4}",
                    state.label, spent, limit
                );
                AuditStatus::Warned
            }
            Ok(SpendResult::Blocked { spent, limit }) => {
                warn!(
                    "Budget blocked: label={} spent=${:.4} limit=${:.4}",
                    state.label, spent, limit
                );
                AuditStatus::Blocked
            }
            Err(_) => AuditStatus::Ok,
        }
    } else {
        AuditStatus::Ok
    };

    let event = AuditEvent::new(
        &state.label,
        &model,
        provider,
        input_tokens,
        output_tokens,
        cost_usd,
        audit_status,
    );
    if let Err(e) = state.audit.lock().unwrap().write(&event) {
        error!("Audit write failed: {}", e);
    }

    let mut response = Response::builder().status(axum_status);
    for (k, v) in &resp_headers {
        if k.as_str() != "transfer-encoding" {
            if let Ok(aname) = axum::http::HeaderName::from_bytes(k.as_str().as_bytes()) {
                if let Ok(avalue) = axum::http::HeaderValue::from_bytes(v.as_bytes()) {
                    response = response.header(aname, avalue);
                }
            }
        }
    }
    Ok(response.body(Body::from(resp_bytes.to_vec())).unwrap())
}
