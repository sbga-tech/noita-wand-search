use wasm_bindgen::JsCast;

pub(super) const CHEVRON_DOWN: &str = r#"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;

/// Keeps the highlighted option visible when navigating a scrollable list.
pub(super) fn scroll_active_into_view(list: &web_sys::HtmlDivElement) {
    let Ok(Some(item)) = list.query_selector(".atlas-menu-item.highlighted") else {
        return;
    };
    let Ok(item) = item.dyn_into::<web_sys::HtmlElement>() else {
        return;
    };
    let top = item.offset_top();
    let bottom = top + item.offset_height();
    let view_top = list.scroll_top();
    let view_bottom = view_top + list.client_height();
    if top < view_top {
        list.set_scroll_top(top);
    } else if bottom > view_bottom {
        list.set_scroll_top(bottom - list.client_height());
    }
}
