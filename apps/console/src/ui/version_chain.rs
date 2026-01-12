//! 版本链面板
//!
//! 展示实体版本历史、版本差异对比等信息

use dioxus::prelude::*;

use crate::models::{VersionChainSummary, VersionEntry};
use crate::state::use_app_state;
use crate::{API_CLIENT, APP_CONFIG};

/// 版本链查询状态
#[derive(Clone, Debug, Default)]
struct QueryState {
    loading: bool,
    error: Option<String>,
    chain: Option<VersionChainSummary>,
}

/// 版本链面板组件
#[component]
pub fn VersionChainPanel() -> Element {
    let state_store = use_app_state();
    let mut entity_type = use_signal(|| "dialogue_event".to_string());
    let mut entity_id_input = use_signal(|| String::new());
    let mut query_state = use_signal(QueryState::default);

    // 查询按钮点击处理
    let on_search = move |_| {
        let current_type = entity_type.read().clone();
        let current_id = entity_id_input.read().clone();

        if current_id.is_empty() {
            return;
        }

        // 获取 tenant_id
        let snapshot = state_store.read();
        let tenant_id = snapshot.tenant_id.clone();
        drop(snapshot);

        let tenant = tenant_id.or_else(|| {
            APP_CONFIG
                .get()
                .and_then(|cfg| cfg.default_tenant_id.clone())
        });

        let Some(tenant) = tenant else {
            query_state.write().error = Some("请先选择租户".into());
            return;
        };

        let Some(client) = API_CLIENT.get().cloned() else {
            query_state.write().error = Some("API 客户端未初始化".into());
            return;
        };

        // 设置加载状态
        query_state.write().loading = true;
        query_state.write().error = None;

        // 发起异步请求
        spawn(async move {
            tracing::info!("开始查询版本链: tenant={}, type={}, id={}", tenant, current_type, current_id);

            match client
                .get_version_chain_summary::<VersionChainSummary>(&tenant, &current_type, &current_id)
                .await
            {
                Ok(env) => {
                    tracing::info!("版本链查询成功: {:?}", env.data);
                    let mut state = query_state.write();
                    state.chain = env.data;
                    state.loading = false;
                }
                Err(err) => {
                    tracing::error!("版本链加载失败: {err}");
                    let mut state = query_state.write();
                    state.error = Some(format!("加载失败: {err}"));
                    state.loading = false;
                }
            }
        });
    };

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "版本链" }
                p { class: "text-xs text-slate-500", "查看实体的版本历史与变更记录" }
            }
            // 搜索面板
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                div { class: "flex gap-3 items-end",
                    div { class: "w-32",
                        label { class: "block text-xs text-slate-500 mb-1", "实体类型" }
                        select {
                            class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            value: "{entity_type}",
                            onchange: move |evt| entity_type.set(evt.value().clone()),
                            option { value: "dialogue_event", "对话事件" }
                            option { value: "session", "会话" }
                            option { value: "message", "消息" }
                            option { value: "artifact", "制品" }
                        }
                    }
                    div { class: "flex-1",
                        label { class: "block text-xs text-slate-500 mb-1", "实体 ID" }
                        input {
                            r#type: "text",
                            class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                            placeholder: "输入实体 ID（如：evt_123456）...",
                            value: "{entity_id_input}",
                            oninput: move |evt| entity_id_input.set(evt.value().clone())
                        }
                    }
                    div {
                        button {
                            class: "px-4 py-2 text-sm font-medium rounded-lg focus:outline-none",
                            style: "background-color: #2563eb; color: white; min-width: 80px;",
                            disabled: entity_id_input.read().is_empty(),
                            onclick: on_search,
                            "查询"
                        }
                    }
                }
            }
            // 版本链展示
            {
                let state = query_state.read();
                if state.loading {
                    rsx! { p { class: "text-xs text-slate-500", "正在加载版本链..." } }
                } else if let Some(ref err) = state.error {
                    rsx! { p { class: "text-xs text-red-500", "{err}" } }
                } else if let Some(ref chain) = state.chain {
                    rsx! { {render_version_chain(chain)} }
                } else {
                    rsx! {
                        div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                            p { class: "text-xs text-slate-500 italic", "输入实体 ID 以查看版本链" }
                        }
                    }
                }
            }
        }
    }
}

/// 渲染版本链
fn render_version_chain(chain: &VersionChainSummary) -> Element {
    let current_ver = chain.current_version.version_number;
    let root_ver = chain.root_version.version_number;

    rsx! {
        div { class: "space-y-4",
            // 概览
            div { class: "rounded-lg border border-slate-200 bg-slate-50 p-4 shadow-sm",
                div { class: "grid grid-cols-2 md:grid-cols-4 gap-4 text-center",
                    div {
                        p { class: "text-xl font-bold text-blue-600", "{chain.total_versions}" }
                        p { class: "text-xs text-slate-500", "总版本数" }
                    }
                    div {
                        p { class: "text-xl font-bold text-green-600", "v{current_ver}" }
                        p { class: "text-xs text-slate-500", "当前版本" }
                    }
                    div {
                        p { class: "text-xl font-bold text-purple-600", "v{root_ver}" }
                        p { class: "text-xs text-slate-500", "根版本" }
                    }
                    div {
                        p { class: "text-xl font-bold text-orange-600",
                            {format!("{}", chain.conflicts.len())}
                        }
                        p { class: "text-xs text-slate-500", "冲突数" }
                    }
                }
            }
            // 版本列表
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                h3 { class: "text-sm font-semibold text-slate-800 mb-3", "版本历史" }
                div { class: "space-y-2 max-h-96 overflow-y-auto",
                    for entry in chain.history.iter() {
                        {render_version_entry(entry, current_ver)}
                    }
                }
            }
        }
    }
}

fn render_version_entry(entry: &VersionEntry, current_version: u32) -> Element {
    let is_current = entry.version_number == current_version;

    rsx! {
        div {
            class: if is_current {
                "p-3 bg-blue-50 rounded-lg border-2 border-blue-300"
            } else {
                "p-3 bg-slate-50 rounded-lg border border-slate-100"
            },
            div { class: "flex items-center justify-between mb-2",
                div { class: "flex items-center gap-2",
                    span { class: "text-sm font-bold text-slate-700", "v{entry.version_number}" }
                    if is_current {
                        span { class: "text-xs px-2 py-0.5 bg-blue-100 text-blue-600 rounded", "当前" }
                    }
                }
                span { class: "text-xs text-slate-400", {format!("{}", entry.created_at_ms)} }
            }
            if let Some(ref desc) = entry.description {
                p { class: "text-xs text-slate-600 mb-2", "{desc}" }
            }
            div { class: "flex items-center gap-4 text-xs text-slate-500",
                span { "作者: {entry.created_by}" }
                span { class: "font-mono", "ID: {entry.version_id}" }
            }
        }
    }
}

/// 版本差异对比面板 - 简化版
#[component]
pub fn VersionDiffPanel() -> Element {
    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "版本差异" }
                p { class: "text-xs text-slate-500", "对比两个版本之间的变更" }
            }
            div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                p { class: "text-xs text-slate-500 italic", "版本差异对比功能开发中..." }
            }
        }
    }
}
