use std::sync::Arc;
use metarflow::models::AppState;
use metarflow::routes::create_router;
use tower_layer::Layer;
use vercel_runtime::{run, Error};
use vercel_runtime::axum::VercelLayer;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let state = Arc::new(AppState {});
    let app = create_router(state);
    let service = VercelLayer::new().layer(app.into_service());

    run(service).await
}
