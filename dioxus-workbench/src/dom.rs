//! Renderer-agnostic DOM plumbing.
//!
//! Everything here goes through `document::eval`, which Dioxus provides on
//! web, desktop, and liveview alike — so pointer capture, focus movement, and
//! the drag-start shim behave the same in a browser tab and in a webview.
//! Renderers without a document provider fail quietly, which degrades to the
//! pre-interaction state rather than a crash.

use dioxus::document;

/// Encode a Rust string as a JavaScript string literal. JSON string syntax is
/// a subset of JavaScript's, which makes this injection-safe for element ids
/// that applications choose freely.
fn js_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

pub(crate) async fn sleep_ms(ms: u32) {
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::TimeoutFuture::new(ms).await;
    #[cfg(not(target_arch = "wasm32"))]
    futures_timer::Delay::new(std::time::Duration::from_millis(u64::from(ms))).await;
}

/// Route subsequent pointer events to the element even when the pointer
/// leaves it mid-drag. Splitter drags stay live outside the window.
pub(crate) fn capture_pointer(element_id: &str, pointer_id: i32) {
    let script = format!(
        "const el = document.getElementById({id}); \
         if (el && el.setPointerCapture) {{ try {{ el.setPointerCapture({pointer_id}); }} catch (_) {{}} }}",
        id = js_string(element_id),
    );
    dioxus::dioxus_core::spawn_forever(async move {
        let _ = document::eval(&script).await;
    });
}

/// Write a split child's flex-basis directly while a splitter drag is live,
/// bypassing the VDOM: committing the layout per pointer move would re-render
/// the whole workspace at pointer frequency. The settled ratio is committed
/// (and rendered normally) when the gesture ends.
pub(crate) fn set_flex_basis(element_id: &str, percent: f64) {
    let script = format!(
        "const el = document.getElementById({id}); if (el) el.style.flexBasis = '{percent:.5}%';",
        id = js_string(element_id),
    );
    dioxus::dioxus_core::spawn_forever(async move {
        let _ = document::eval(&script).await;
    });
}

/// Scroll an element horizontally by a pixel delta — the wheel-over-tab-strip
/// affordance editors train: a vertical wheel walks an overflowed strip
/// sideways. A no-op when nothing overflows.
pub(crate) fn scroll_by_x(element_id: &str, delta: f64) {
    if !delta.is_finite() || delta == 0.0 {
        return;
    }
    let script = format!(
        "const el = document.getElementById({id}); if (el) el.scrollLeft += {delta:.2};",
        id = js_string(element_id),
    );
    dioxus::dioxus_core::spawn_forever(async move {
        let _ = document::eval(&script).await;
    });
}

/// Bring an element into view inside its scrollable ancestor — an activated
/// tab in a crowded, overflowed strip. `nearest` keeps it from scrolling the
/// page itself.
pub(crate) fn scroll_into_view(element_id: &str) {
    let script = format!(
        "const el = document.getElementById({id}); \
         if (el && el.scrollIntoView) el.scrollIntoView({{block: 'nearest', inline: 'nearest'}});",
        id = js_string(element_id),
    );
    dioxus::dioxus_core::spawn_forever(async move {
        // Yield a tick first: this runs from effect flushes (including the
        // very first mount), and evaluating script from that context can
        // re-enter the executor mid-poll — the same deferral
        // `focus_after_render` uses.
        sleep_ms(0).await;
        let _ = document::eval(&script).await;
    });
}

/// Move focus after Dioxus has committed a replacement subtree. Retries for a
/// few frames because the target may not exist until the next render.
pub fn focus_after_render(id: impl Into<String>) {
    let id = id.into();
    dioxus::dioxus_core::spawn_forever(async move {
        for attempt in 0..16 {
            sleep_ms(if attempt == 0 { 0 } else { 16 }).await;
            let script = format!(
                "const el = document.getElementById({id}); if (el) el.focus(); return el !== null;",
                id = js_string(&id),
            );
            match document::eval(&script).await {
                Ok(found) if found.as_bool() == Some(true) => break,
                Ok(_) => continue,
                // No document provider: retrying will not change anything.
                Err(_) => break,
            }
        }
    });
}

/// Firefox only starts an HTML5 drag once `dataTransfer` carries data, and
/// Dioxus events cannot reach the native `DataTransfer` object. One capturing
/// listener per document primes it for every workbench tab.
pub(crate) fn install_drag_shim() {
    const SHIM: &str = "\
        if (!window.__wbDragShim) {\
            window.__wbDragShim = true;\
            document.addEventListener('dragstart', (event) => {\
                const tab = event.target && event.target.closest ? event.target.closest('.wb-tab') : null;\
                if (tab && event.dataTransfer) {\
                    event.dataTransfer.setData('text/plain', tab.id || 'wb-tab');\
                    event.dataTransfer.effectAllowed = 'move';\
                }\
            }, true);\
        }";
    dioxus::dioxus_core::spawn_forever(async move {
        let _ = document::eval(SHIM).await;
    });
}
