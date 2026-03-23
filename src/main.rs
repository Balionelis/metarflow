use std::sync::Arc;
use metarflow::models::AppState;
use metarflow::routes::create_router;

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
