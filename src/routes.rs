use axum::Router;
use std::sync::Arc;
use crate::handlers;
use crate::models::AppState;

// sets up all the routes for the web server
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(handlers::index))
        .route("/metar", axum::routing::get(handlers::fetch_metar_handler))
        .route("/privacy", axum::routing::get(handlers::privacy))
        .route("/metarflow.svg", axum::routing::get(handlers::favicon))
        .with_state(state)
}

