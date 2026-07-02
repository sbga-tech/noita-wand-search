fn main() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        let thread_count = web_sys::window()
            .map(|window| window.navigator().hardware_concurrency())
            .unwrap_or(1.0)
            .max(1.0) as usize;
        if let Err(error) =
            wasm_bindgen_futures::JsFuture::from(noita_web::init_thread_pool(thread_count)).await
        {
            web_sys::console::error_1(&error);
            return;
        }
        leptos::mount::mount_to_body(noita_web::app::App);
    });
}
