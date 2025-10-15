use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::timeline::sample_ace_cycles;
use crate::state::{use_app_actions, use_app_state};

pub fn use_ace_cycles() {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    let session = snapshot.session_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant, session)| {
        let actions = actions.clone();
        async move {
            tracing::info!("ACE loader triggered: tenant={:?}, session={:?}", tenant, session);
            if tenant.is_none() || session.is_none() {
                actions.set_ace_cycles(Vec::new());
                actions.select_ace_cycle(None);
                return;
            }

            TimeoutFuture::new(0).await;

            actions.set_ace_loading(true);
            actions.set_ace_error(None);

            TimeoutFuture::new(150).await;

            let cycles = sample_ace_cycles();
            let selected = cycles.first().map(|cycle| cycle.cycle_id.clone());
            actions.set_ace_cycles(cycles);
            actions.select_ace_cycle(selected);
        }
    }));
}
