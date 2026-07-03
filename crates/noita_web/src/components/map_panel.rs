use leptos::prelude::*;
use noita_sim::search::SearchProgress;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapProjection {
    pub width: f64,
    pub height: f64,
    pub parallel_width: f64,
    pub parallel_left_edge_tile: f64,
}

pub fn projection(ng: u32) -> MapProjection {
    if ng > 0 {
        MapProjection {
            width: 1280.0,
            height: 960.0,
            parallel_width: 64.0 * 512.0,
            parallel_left_edge_tile: -32.0,
        }
    } else {
        MapProjection {
            width: 1280.0 * 70.0 / 64.0,
            height: 960.0,
            parallel_width: 70.0 * 512.0,
            parallel_left_edge_tile: -35.0,
        }
    }
}

pub fn world_to_canvas(proj: MapProjection, x: f64, y: f64) -> (f64, f64) {
    let world_width = 64.0 * 512.0;
    let world_height = world_width * 48.0 / 64.0;
    let scale = proj.height / 48.0;
    (
        proj.width * x / world_width - proj.parallel_left_edge_tile * scale,
        proj.height * y / world_height + 14.0 * scale,
    )
}

pub fn parallel_name(parallel_number: i32, show_main_world: bool) -> String {
    if parallel_number > 0 {
        format!("East {parallel_number}")
    } else if parallel_number < 0 {
        format!("West {}", -parallel_number)
    } else if show_main_world {
        "Main World".into()
    } else {
        String::new()
    }
}

#[component]
pub fn MapPanel(
    ng: ReadSignal<u32>,
    progress: ReadSignal<SearchProgress>,
    status: ReadSignal<String>,
) -> impl IntoView {
    let view_box = move || {
        let proj = projection(ng.get());
        format!("0 0 {} {}", proj.width, proj.height)
    };
    let frame_ratio = move || {
        let proj = projection(ng.get());
        format!("aspect-ratio: {} / {};", proj.width, proj.height)
    };
    let marker_transform = move || {
        let proj = projection(ng.get());
        let progress = progress.get();
        let (x, y) = world_to_canvas(proj, progress.x, progress.y);
        format!("translate({x} {y})")
    };
    view! {
        <section class="map-panel">
            <div class="map-frame" style=frame_ratio>
                <img id="newgame_map" src="public/biome_map.png" alt="NG Map" style=move || { if ng.get() == 0 { "display:block" } else { "display:none" } } />
                <img id="newgame_plus_map" src="public/biome_map_newgame_plus.png" alt="NG+ Map" style=move || { if ng.get() > 0 { "display:block" } else { "display:none" } } />
                <svg id="map_overlay" viewBox=view_box preserveAspectRatio="none" role="img" aria-label="Search map overlay">
                    <g class="marker" transform=marker_transform><line x1="-10" y1="0" x2="10" y2="0"/><line x1="0" y1="-10" x2="0" y2="10"/><circle cx="0" cy="0" r="7"/></g>
                </svg>
                <div class="map-progress"><span>{move || status.get()}</span><small>{move || { let p = progress.get(); format!("x {} · y {}", p.x, p.y) }}</small></div>
            </div>
        </section>
    }
}
