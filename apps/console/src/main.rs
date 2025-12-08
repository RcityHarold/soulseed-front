#![allow(non_snake_case)]

mod api;
mod config;
mod fixtures;
mod hooks;
mod models;
mod services;
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
use ui::autonomous::AutonomousPanel;
use ui::context::ContextPanel;
use ui::dfr::DfrPanel;
use ui::evolution::EvolutionPanel;
use ui::explain::ExplainDiagnosticPanel;
use ui::graph::GraphPanel;
use ui::graph_enhanced::GraphEnhancedPanel;
use ui::interaction::InteractionPanel;
use ui::metacognition::MetacognitionPanel;
use ui::notifications::NotificationCenter;
use ui::surreal::{TimeSeriesPanel, VectorSearchPanel};
use ui::timeline::TimelineView;
use ui::tools::ToolTracePanel;
use ui::version_chain::VersionChainPanel;
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
    let global_styles = include_str!("../../../public/tailwind.css");

    use_future({
        let mut app_state = app_state.clone();
        move || async move {
            if let Some(cfg) = APP_CONFIG.get() {
                let default_tenant = cfg.default_tenant_id.clone();
                let default_session = cfg.default_session_id.clone();
                let mut state = app_state.write();
                if state.tenant_id.is_none() {
                    state.tenant_id = default_tenant;
                }
                if state.session_id.is_none() {
                    state.session_id = default_session;
                }
            }
        }
    });

    use_context_provider(|| app_state.clone());

    rsx! {
        style { dangerous_inner_html: "{global_styles}" }
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
        div { class: "app-shell space-y-4 p-4",
            section { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                h1 { class: "text-xl font-semibold text-slate-900", "Soulseed 控制台" }
                p { class: "text-sm text-slate-600", "Thin-Waist API: {api_endpoint}" }
                p { class: "text-xs text-slate-500", "当前为基础骨架，后续将串联实时数据与操作入口。" }
            }
            // 原有面板
            WorkspacePanel {}
            TimelineView {}
            GraphPanel {}
            AcePanel {}
            ToolTracePanel {}
            ContextPanel {}
            ExplainDiagnosticPanel {}
            InteractionPanel {}

            // === 新增功能面板 ===
            section { class: "rounded-lg border-2 border-blue-200 bg-blue-50 p-4",
                h2 { class: "text-lg font-bold text-blue-800 mb-4", "新增功能模块" }
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                    // 元认知分析
                    div { class: "bg-white rounded-lg p-4 shadow",
                        MetacognitionPanel {}
                    }
                    // 自主延续
                    div { class: "bg-white rounded-lg p-4 shadow",
                        AutonomousPanel {}
                    }
                    // DFR 决策
                    div { class: "bg-white rounded-lg p-4 shadow",
                        DfrPanel {}
                    }
                    // 版本链
                    div { class: "bg-white rounded-lg p-4 shadow",
                        VersionChainPanel {}
                    }
                    // 图谱增强
                    div { class: "bg-white rounded-lg p-4 shadow",
                        GraphEnhancedPanel {}
                    }
                    // 演化事件
                    div { class: "bg-white rounded-lg p-4 shadow",
                        EvolutionPanel {}
                    }
                    // 时序分析
                    div { class: "bg-white rounded-lg p-4 shadow",
                        TimeSeriesPanel {}
                    }
                    // 向量搜索
                    div { class: "bg-white rounded-lg p-4 shadow",
                        VectorSearchPanel {}
                    }
                }
            }
        }
    }
}
