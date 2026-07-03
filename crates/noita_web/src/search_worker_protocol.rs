use noita_sim::search::{SearchHit, SearchProgress, SearchRequest};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SearchWorkerStart {
    pub token_id: u64,
    pub request: SearchRequest,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SearchWorkerEvent {
    Ready,
    Progress {
        token_id: u64,
        progress: SearchProgress,
        pixels_per_second: f64,
    },
    Hit {
        token_id: u64,
        progress: SearchProgress,
        pixels_per_second: f64,
        hit: SearchHit,
    },
    Error {
        token_id: Option<u64>,
        message: String,
    },
}
