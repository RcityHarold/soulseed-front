use crate::hooks::workspace::use_workspace_overview;
use crate::models::{TenantWorkspace, WorkspaceSession};
use crate::state::{use_app_actions, use_app_state, AppActions};
use dioxus::prelude::*;

pub fn WorkspacePanel(cx: Scope) -> Element {
    use_workspace_overview(cx);

    let actions = use_app_actions(cx);
    let app_state = use_app_state(cx);
    let snapshot = app_state.read();
    let workspace = snapshot.workspace.clone();
    let current_tenant = snapshot.tenant_id.clone();
    let current_session = snapshot.session_id.clone();
    drop(snapshot);

    let tenants = workspace.tenants.clone();

    let active_tenant = current_tenant
        .as_ref()
        .and_then(|id| tenants.iter().find(|tenant| &tenant.tenant_id == id))
        .cloned()
        .or_else(|| tenants.first().cloned());

    let body = if workspace.is_loading {
        rsx! { p { class: "text-xs text-slate-500", "正在加载工作台..." } }
    } else if let Some(ref err) = workspace.error {
        rsx! { p { class: "text-xs text-red-500", "工作台加载失败: {err}" } }
    } else if tenants.is_empty() {
        rsx! { p { class: "text-xs text-slate-500 italic", "暂无租户数据" } }
    } else if let Some(tenant) = active_tenant.clone() {
        let tenant_id = tenant.tenant_id.clone();
        let manifest_level = tenant
            .manifest_level
            .clone()
            .unwrap_or_else(|| "未配置".into());
        let clarify_strategy = tenant
            .clarify_strategy
            .clone()
            .unwrap_or_else(|| "默认".into());

        rsx! {
            div { class: "space-y-4",
                render_tenant_selector(&tenants, current_tenant.clone(), actions.clone())
                div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-2 text-xs text-slate-600",
                    header { class: "flex flex-col gap-1",
                        h3 { class: "text-sm font-semibold text-slate-800", "{tenant.display_name}" }
                        if let Some(desc) = tenant.description.as_ref() {
                            p { class: "text-[11px] text-slate-500", desc }
                        }
                    }
                    div { class: "flex flex-wrap gap-4 text-[11px] text-slate-500",
                        span { format!("Manifest: {}", manifest_level) }
                        span { format!("Clarify 策略: {}", clarify_strategy) }
                        span { format!("收藏 {}", tenant.pinned_sessions.len()) }
                        span { format!("最近 {}", tenant.recent_sessions.len()) }
                    }
                }

                div { class: "space-y-2",
                    h4 { class: "text-sm font-semibold text-slate-800", "收藏会话" }
                    if tenant.pinned_sessions.is_empty() {
                        p { class: "text-xs text-slate-500", "尚无收藏，会话可通过下方列表收藏" }
                    } else {
                        div { class: "grid gap-2 md:grid-cols-2",
                            for session in tenant.pinned_sessions.iter() {
                                session_card(
                                    session,
                                    &tenant_id,
                                    current_session.as_deref(),
                                    true,
                                    actions.clone(),
                                )
                            }
                        }
                    }
                }

                div { class: "space-y-2",
                    h4 { class: "text-sm font-semibold text-slate-800", "最近会话" }
                    if tenant.recent_sessions.is_empty() {
                        p { class: "text-xs text-slate-500", "暂无最近会话" }
                    } else {
                        div { class: "grid gap-2 md:grid-cols-2 lg:grid-cols-3",
                            for session in tenant.recent_sessions.iter() {
                                session_card(
                                    session,
                                    &tenant_id,
                                    current_session.as_deref(),
                                    false,
                                    actions.clone(),
                                )
                            }
                        }
                    }
                }
            }
        }
    } else {
        rsx! { p { class: "text-xs text-slate-500 italic", "请选择一个租户" } }
    };

    cx.render(rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "多租户 / 多会话工作台" }
                p { class: "text-xs text-slate-500", "快速切换租户、收藏关键会话，并查看默认配置。" }
            }
            {body}
        }
    })
}

fn render_tenant_selector(
    tenants: &[TenantWorkspace],
    current_tenant: Option<String>,
    actions: AppActions,
) -> LazyNodes {
    rsx! {
        div { class: "flex flex-wrap gap-2",
            for tenant in tenants.iter() {
                let tenant_id = tenant.tenant_id.clone();
                let label = tenant.display_name.clone();
                let is_active = current_tenant
                    .as_ref()
                    .map(|current| current == &tenant_id)
                    .unwrap_or(false);
                let actions = actions.clone();
                let session_seed = tenant
                    .pinned_sessions
                    .first()
                    .or_else(|| tenant.recent_sessions.first())
                    .map(|session| session.session_id.clone());
                let button_class = if is_active {
                    "rounded-full bg-slate-900 px-4 py-1.5 text-xs font-semibold text-white"
                } else {
                    "rounded-full border border-slate-300 px-4 py-1.5 text-xs text-slate-700 hover:border-slate-500"
                };

                button {
                    key: tenant_id.clone(),
                    class: button_class,
                    onclick: move |_| {
                        actions.set_tenant(Some(tenant_id.clone()));
                        if let Some(session_id) = session_seed.clone() {
                            actions.set_session(Some(session_id));
                        }
                    },
                    label
                }
            }
        }
    }
}

fn session_card(
    session: &WorkspaceSession,
    tenant_id: &str,
    current_session: Option<&str>,
    is_pinned_section: bool,
    actions: AppActions,
) -> LazyNodes {
    let session_id = session.session_id.clone();
    let title = session
        .title
        .clone()
        .unwrap_or_else(|| format!("会话 {}", session_id.clone()));
    let scenario_label = session
        .scenario
        .as_ref()
        .map(|scenario| format!("{:?}", scenario))
        .unwrap_or_else(|| "未知场景".into());
    let is_active = current_session
        .map(|current| current == session_id)
        .unwrap_or(false);
    let summary = session.summary.clone().unwrap_or_default();
    let display_summary = if summary.is_empty() {
        None
    } else {
        Some(summary)
    };

    let card_class = if is_active {
        "rounded-lg border border-slate-900 bg-slate-900/90 p-4 text-xs text-white shadow-sm space-y-2"
    } else {
        "rounded-lg border border-slate-200 bg-white p-4 text-xs text-slate-600 shadow-sm space-y-2"
    };

    let key = format!("{}-{}", tenant_id, session_id);

    let actions_open = actions.clone();
    let actions_pin = actions.clone();
    let tenant_id = tenant_id.to_string();
    let is_pinned = session.pinned;

    rsx! {
        div { key: key, class: card_class,
            header { class: "flex items-center justify-between",
                h3 { class: "text-sm font-semibold", title }
                span { class: "text-[11px] text-slate-500", scenario_label }
            }
            if let Some(text) = display_summary {
                p { class: "text-[11px]", text }
            }
            div { class: "flex flex-wrap gap-2",
                button {
                    class: if is_active {
                        "rounded bg-white/10 px-3 py-1 text-[11px] font-semibold text-white"
                    } else {
                        "rounded bg-slate-900 px-3 py-1 text-[11px] font-semibold text-white hover:bg-slate-800"
                    },
                    onclick: move |_| {
                        actions_open.set_tenant(Some(tenant_id.clone()));
                        actions_open.set_session(Some(session_id.clone()));
                    },
                    if is_active { "当前" } else { "打开" }
                }
                let toggle_label = if is_pinned { "取消收藏" } else { "收藏" };
                button {
                    class: if is_pinned {
                        "rounded border border-amber-400 px-3 py-1 text-[11px] text-amber-700 hover:bg-amber-50"
                    } else {
                        "rounded border border-slate-300 px-3 py-1 text-[11px] text-slate-600 hover:bg-slate-100"
                    },
                    onclick: move |_| {
                        actions_pin.set_session_pin(&tenant_id, &session_id, !is_pinned);
                    },
                    toggle_label
                }
            }
            if !is_pinned_section && session.pinned {
                span { class: "rounded bg-amber-100 px-2 py-0.5 text-[11px] text-amber-800", "已收藏" }
            }
        }
    }
}
