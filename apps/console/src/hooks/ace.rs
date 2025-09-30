use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::timeline::sample_ace_cycles;
use crate::state::{use_app_actions, use_app_state};

pub fn use_ace_cycles(cx: Scope) {
    let actions = use_app_actions(cx);
    let state = use_app_state(cx);

    let dependencies = {
        let snapshot = state.read();
        (snapshot.tenant_id.clone(), snapshot.session_id.clone())
    };

    use_effect(cx, dependencies, move |(tenant, session)| {
        let actions = actions.clone();
        async move {
            if tenant.is_none() || session.is_none() {
                actions.set_ace_cycles(Vec::new());
                actions.select_ace_cycle(None);
                return;
            }

            actions.set_ace_loading(true);
            actions.set_ace_error(None);

            TimeoutFuture::new(150).await;

            let cycles = sample_ace_cycles();
            let selected = cycles.first().map(|cycle| cycle.cycle_id.clone());
            actions.set_ace_cycles(cycles);
            actions.select_ace_cycle(selected);
        }
    });
}
