use thiserror::Error;

#[derive(Clone, Copy)]
pub struct SseConnectOptions {
    pub heartbeat_timeout_ms: u64,
    pub retry_base_ms: u64,
    pub retry_max_ms: u64,
}

impl Default for SseConnectOptions {
    fn default() -> Self {
        Self {
            heartbeat_timeout_ms: 30_000,
            retry_base_ms: 1_000,
            retry_max_ms: 10_000,
        }
    }
}

pub struct SseCallbacks {
    pub on_open: Box<dyn Fn() + 'static>,
    pub on_message: Box<dyn Fn(SseMessage) + 'static>,
    pub on_error: Box<dyn Fn(String) + 'static>,
}

impl SseCallbacks {
    pub fn new(
        on_open: impl Fn() + 'static,
        on_message: impl Fn(SseMessage) + 'static,
        on_error: impl Fn(String) + 'static,
    ) -> Self {
        Self {
            on_open: Box::new(on_open),
            on_message: Box::new(on_message),
            on_error: Box::new(on_error),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SseMessage {
    pub event: Option<String>,
    pub data: String,
}

#[derive(Debug, Error)]
pub enum SseError {
    #[error("SSE 初始化失败: {0}")]
    EventSourceInit(String),
    #[error("SSE 在当前平台未实现: {0}")]
    Unsupported(String),
}

pub struct SseHandle {
    #[cfg(target_arch = "wasm32")]
    inner: std::rc::Rc<wasm::SseInner>,
}

impl SseHandle {
    pub fn close(&self) {
        #[cfg(target_arch = "wasm32")]
        self.inner.close();
    }
}

impl Drop for SseHandle {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct SseClient;

impl SseClient {
    pub fn connect(
        url: &str,
        callbacks: SseCallbacks,
        options: SseConnectOptions,
    ) -> Result<SseHandle, SseError> {
        #[cfg(target_arch = "wasm32")]
        {
            let inner = wasm::SseInner::new(url, callbacks, options)?;
            inner.connect();
            Ok(SseHandle { inner })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (url, callbacks, options);
            Err(SseError::Unsupported(
                "仅 wasm32 目标支持 EventSource".into(),
            ))
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::{SseCallbacks, SseConnectOptions, SseError, SseMessage};
    use gloo_timers::callback::{Interval, Timeout};
    use js_sys::Date;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use wasm_bindgen::{closure::Closure, JsCast};
    use web_sys::{Event, EventSource, MessageEvent};

    pub struct SseInner {
        url: String,
        callbacks: SseCallbacks,
        options: SseConnectOptions,
        event_source: RefCell<Option<EventSource>>,
        reconnect_timer: RefCell<Option<Timeout>>,
        heartbeat_timer: RefCell<Option<Interval>>,
        last_event_ms: Cell<f64>,
        backoff_ms: Cell<u64>,
        closed: Cell<bool>,
    }

    impl SseInner {
        pub fn new(
            url: &str,
            callbacks: SseCallbacks,
            options: SseConnectOptions,
        ) -> Result<Rc<Self>, SseError> {
            if url.trim().is_empty() {
                return Err(SseError::EventSourceInit("URL 为空".into()));
            }

            Ok(Rc::new(Self {
                url: url.to_string(),
                callbacks,
                options,
                event_source: RefCell::new(None),
                reconnect_timer: RefCell::new(None),
                heartbeat_timer: RefCell::new(None),
                last_event_ms: Cell::new(Date::now()),
                backoff_ms: Cell::new(options.retry_base_ms.max(500)),
                closed: Cell::new(false),
            }))
        }

        pub fn connect(self: &Rc<Self>) {
            if self.closed.get() {
                return;
            }

            match EventSource::new(&self.url) {
                Ok(es) => {
                    self.install_handlers(&es);
                    self.event_source.replace(Some(es));
                }
                Err(err) => {
                    let reason = js_value_to_string(&err);
                    (self.callbacks.on_error)(format!("SSE 连接失败: {reason}"));
                    self.schedule_reconnect();
                }
            }
        }

        pub fn close(&self) {
            self.closed.set(true);
            if let Some(es) = self.event_source.borrow_mut().take() {
                es.close();
            }
            if let Some(timer) = self.reconnect_timer.borrow_mut().take() {
                timer.cancel();
            }
            if let Some(interval) = self.heartbeat_timer.borrow_mut().take() {
                interval.cancel();
            }
        }

        fn install_handlers(self: &Rc<Self>, es: &EventSource) {
            self.backoff_ms.set(self.options.retry_base_ms.max(500));
            self.last_event_ms.set(Date::now());

            let inner = Rc::clone(self);
            let on_open = Closure::wrap(Box::new(move |_evt: Event| {
                inner.last_event_ms.set(Date::now());
                inner.start_heartbeat();
                (inner.callbacks.on_open)();
            }) as Box<dyn FnMut(_)>);
            es.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            on_open.forget();

            let inner = Rc::clone(self);
            let on_error = Closure::wrap(Box::new(move |_evt: Event| {
                (inner.callbacks.on_error)("SSE 连接中断，准备重试".into());
                inner.restart_event_source();
            }) as Box<dyn FnMut(_)>);
            es.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            on_error.forget();

            let inner = Rc::clone(self);
            let on_message = Closure::wrap(Box::new(move |evt: MessageEvent| {
                inner.handle_message(None, evt);
            }) as Box<dyn FnMut(_)>);
            es.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            on_message.forget();

            // 监听常见的自定义事件
            for event_name in ["dialogue_event", "awareness_event", "ping"] {
                let inner = Rc::clone(self);
                let event_type = event_name.to_string();
                let closure = Closure::wrap(Box::new(move |evt: MessageEvent| {
                    inner.handle_message(Some(event_type.clone()), evt);
                }) as Box<dyn FnMut(_)>);
                let _ = es
                    .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref());
                closure.forget();
            }
        }

        fn handle_message(&self, event_type: Option<String>, evt: MessageEvent) {
            if self.closed.get() {
                return;
            }
            self.last_event_ms.set(Date::now());
            let data = match evt.data() {
                val if val.is_string() => val.as_string().unwrap_or_default(),
                val => js_sys::JSON::stringify(&val)
                    .ok()
                    .and_then(|js| js.as_string())
                    .unwrap_or_else(|| String::from("null")),
            };

            (self.callbacks.on_message)(SseMessage {
                event: event_type,
                data,
            });
        }

        fn restart_event_source(self: &Rc<Self>) {
            if let Some(es) = self.event_source.borrow_mut().take() {
                es.close();
            }
            self.schedule_reconnect();
        }

        fn schedule_reconnect(self: &Rc<Self>) {
            if self.closed.get() {
                return;
            }

            if let Some(timer) = self.reconnect_timer.borrow_mut().take() {
                timer.cancel();
            }

            let delay = self
                .backoff_ms
                .get()
                .min(self.options.retry_max_ms.max(1_000));
            self.backoff_ms
                .set((delay * 2).min(self.options.retry_max_ms.max(1_000)));

            let inner = Rc::clone(self);
            let timer = Timeout::new(delay as u32, move || {
                inner.connect();
            });
            self.reconnect_timer.replace(Some(timer));
        }

        fn start_heartbeat(self: &Rc<Self>) {
            if let Some(interval) = self.heartbeat_timer.borrow_mut().take() {
                interval.cancel();
            }

            let timeout_ms = self.options.heartbeat_timeout_ms.max(5_000);
            let inner = Rc::clone(self);
            let interval = Interval::new((timeout_ms / 2) as u32, move || {
                if inner.closed.get() {
                    return;
                }
                let elapsed = Date::now() - inner.last_event_ms.get();
                if elapsed > timeout_ms as f64 {
                    (inner.callbacks.on_error)("SSE 心跳超时，尝试重新连接".into());
                    inner.restart_event_source();
                }
            });
            self.heartbeat_timer.replace(Some(interval));
        }
    }

    fn js_value_to_string(value: &wasm_bindgen::JsValue) -> String {
        if let Some(text) = value.as_string() {
            return text;
        }
        js_sys::JSON::stringify(value)
            .ok()
            .and_then(|js| js.as_string())
            .unwrap_or_else(|| "未知错误".into())
    }
}
