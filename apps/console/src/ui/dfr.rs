//! DFR 决策面板
//!
//! 展示决策指纹与复现功能

use dioxus::prelude::*;

use crate::hooks::dfr::use_fingerprint_list;

/// DFR 决策面板组件
#[component]
pub fn DfrPanel() -> Element {
    let fingerprints = use_fingerprint_list();

    let body = {
        let fp_data = fingerprints.read();
        if let Some(ref data) = *fp_data {
            rsx! {
                div { class: "space-y-4",
                    div { class: "rounded-lg border border-slate-200 bg-slate-50 p-4 shadow-sm",
                        h3 { class: "text-sm font-semibold text-slate-800 mb-3", "决策指纹" }
                        p { class: "text-xs text-slate-600",
                            {format!("共 {} 个指纹记录", data.fingerprints.len())}
                        }
                    }
                }
            }
        } else {
            rsx! { p { class: "text-xs text-slate-500 italic", "暂无决策指纹数据" } }
        }
    };

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "决策指纹复现" }
                p { class: "text-xs text-slate-500", "决策路径记录与场景复现分析" }
            }
            {body}
        }
    }
}
