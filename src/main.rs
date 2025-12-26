mod handlers;
mod models;
mod routes;
mod services;
mod utils;

use std::sync::Arc;
use crate::models::AppState;
use crate::routes::create_router;

// start up the web server and listens for incoming requests
#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {});
    
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
