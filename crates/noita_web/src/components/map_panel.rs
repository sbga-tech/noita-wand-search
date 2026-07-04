use super::search_runner::compact_number;
use leptos::prelude::*;
use noita_sim::search::SearchProgress;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapProjection {
    pub width: f64,
    pub height: f64,
    pub parallel_width: f64,
    pub parallel_left_edge_tile: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParallelPosition {
    pub number: i32,
    pub local_x: f64,
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

pub fn parallel_position(proj: MapProjection, x: f64) -> ParallelPosition {
    let left_edge = proj.parallel_left_edge_tile * 512.0;
    let number = ((x - left_edge) / proj.parallel_width).floor() as i32;
    ParallelPosition {
        number,
        local_x: x - number as f64 * proj.parallel_width,
    }
}

pub fn world_to_canvas(proj: MapProjection, x: f64, y: f64) -> (f64, f64) {
    let world_width = 64.0 * 512.0;
    let world_height = world_width * 48.0 / 64.0;
    let scale = proj.height / 48.0;
    let parallel = parallel_position(proj, x);
    (
        proj.width * parallel.local_x / world_width - proj.parallel_left_edge_tile * scale,
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

pub fn parallel_marker_label(parallel_number: i32) -> String {
    if parallel_number > 0 {
        format!("E{parallel_number}")
    } else if parallel_number < 0 {
        format!("W{}", -parallel_number)
    } else {
        "Main".into()
    }
}

#[component]
pub fn MapPanel(
    ng: ReadSignal<u32>,
    progress: ReadSignal<SearchProgress>,
    search_speed: ReadSignal<f64>,
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
    let marker_parallel_number = move || {
        let proj = projection(ng.get());
        let progress = progress.get();
        parallel_position(proj, progress.x).number
    };
    let marker_class = move || {
        if marker_parallel_number() == 0 {
            "marker"
        } else {
            "marker marker-parallel"
        }
    };
    let marker_label = move || parallel_marker_label(marker_parallel_number());
    let metrics_label = move || {
        let progress = progress.get();
        format!(
            "{}px · {}px/s",
            compact_number(progress.searched_pixels as f64),
            compact_number(search_speed.get())
        )
    };
    let coords_label = move || {
        let proj = projection(ng.get());
        let progress = progress.get();
        let world = parallel_name(parallel_position(proj, progress.x).number, true);
        format!("x {:.0} · y {:.0} · {world}", progress.x, progress.y)
    };
    view! {
        <section class="map-panel">
            <div class="map-frame" style=frame_ratio>
                <img id="newgame_map" src="public/biome_map.png" alt="NG Map" style=move || { if ng.get() == 0 { "display:block" } else { "display:none" } } />
                <img id="newgame_plus_map" src="public/biome_map_newgame_plus.png" alt="NG+ Map" style=move || { if ng.get() > 0 { "display:block" } else { "display:none" } } />
                <svg id="map_overlay" viewBox=view_box preserveAspectRatio="none" role="img" aria-label="Search map overlay">
                    <g class=marker_class transform=marker_transform>
                        <line x1="-10" y1="0" x2="10" y2="0"/>
                        <line x1="0" y1="-10" x2="0" y2="10"/>
                        <circle cx="0" cy="0" r="7"/>
                        <text class="marker-world-label" x="14" y="-14">{marker_label}</text>
                    </g>
                </svg>
                <div class="map-progress">
                    <span>{metrics_label}</span>
                    <small>{coords_label}</small>
                </div>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_position_wraps_world_x_for_new_game() {
        let proj = projection(0);
        assert_eq!(
            parallel_position(proj, 0.0),
            ParallelPosition {
                number: 0,
                local_x: 0.0,
            }
        );
        assert_eq!(
            parallel_position(proj, 18_000.0),
            ParallelPosition {
                number: 1,
                local_x: -17_840.0,
            }
        );
        assert_eq!(
            parallel_position(proj, -18_000.0),
            ParallelPosition {
                number: -1,
                local_x: 17_840.0,
            }
        );
    }

    #[test]
    fn parallel_position_uses_ng_plus_width() {
        let proj = projection(1);
        assert_eq!(
            parallel_position(proj, 16_384.0),
            ParallelPosition {
                number: 1,
                local_x: -16_384.0,
            }
        );
    }
}
