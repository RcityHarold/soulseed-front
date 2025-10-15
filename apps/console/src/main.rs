#![allow(non_snake_case)]

mod api;
mod config;
mod fixtures;
mod hooks;
mod models;
mod state;
mod ui;

use api::{ClientError, ThinWaistClient};
use config::AppConfig;
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use once_cell::sync::OnceCell;
use state::AppState;
use tracing::{error, info};
use ui::ace::AcePanel;
use ui::context::ContextPanel;
use ui::explain::ExplainDiagnosticPanel;
use ui::graph::GraphPanel;
use ui::interaction::InteractionPanel;
use ui::notifications::NotificationCenter;
use ui::timeline::TimelineView;
use ui::tools::ToolTracePanel;
use ui::workspace::WorkspacePanel;

pub(crate) static APP_CONFIG: OnceCell<AppConfig> = OnceCell::new();
pub(crate) static API_CLIENT: OnceCell<ThinWaistClient> = OnceCell::new();

fn main() {
    console_error_panic_hook::set_once();
    init_logging();
    bootstrap_infrastructure();
    launch(App);
}

fn init_logging() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = dioxus_logger::init(tracing::Level::INFO);
    });
}

fn bootstrap_infrastructure() {
    let config = AppConfig::from_env();
    let _ = APP_CONFIG.set(config.clone());

    match ThinWaistClient::new(config) {
        Ok(client) => {
            let _ = API_CLIENT.set(client);
            info!("Thin-Waist client initialized");
        }
        Err(err) => {
            report_client_error("初始化 Thin-Waist 客户端失败", &err);
        }
    }
}

fn report_client_error(context: &str, err: &ClientError) {
    error!(%context, ?err, status = ?err.status(), "api bootstrap error");
}

#[component]
fn App() -> Element {
    let app_state = use_signal(AppState::default);

    use_context_provider(|| app_state.clone());

    rsx! {
        div { class: "relative",
            Router::<Route> {}
            NotificationCenter {}
        }
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Dashboard {},
}

#[component]
fn Dashboard() -> Element {
    let api_endpoint = APP_CONFIG
        .get()
        .map(|c| c.api_base_url.clone())
        .unwrap_or_else(|| "未配置 API 地址".to_string());

    rsx! {
        div { class: "app-shell space-y-4",
            section { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                h1 { class: "text-xl font-semibold text-slate-900", "Soulseed 控制台" }
                p { class: "text-sm text-slate-600", "Thin-Waist API: {api_endpoint}" }
                p { class: "text-xs text-slate-500", "当前为基础骨架，后续将串联实时数据与操作入口。" }
            }
            WorkspacePanel {}
            TimelineView {}
            GraphPanel {}
            AcePanel {}
            ToolTracePanel {}
            ContextPanel {}
            ExplainDiagnosticPanel {}
            InteractionPanel {}
        }
    }
}
