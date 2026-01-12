// Define data modules
mod models; // Data structures (Task, Settings, Db, etc.)
mod store;  // Persistent storage (load/save db.json)
mod logic;  // Core scheduling and scoring logic
mod routes_tasks;   // HTTP handlers for task & settings APIs
mod routes_plan;    // HTTP handlers for today plan API

// Import axum routing utilities and Router
use axum::{
    routing::{get, post, put}, // HTTP method helpers
    Router, // Main router type
};
use tower_http::services::ServeDir; // Used to serve static files (HTML/CSS/JS)
use std::net::SocketAddr;   // ServeDir is used to serve static files (HTML/CSS/JS)


#[tokio::main]
async fn main() {
    let api = Router::new()
        // plan
        .route("/plan/today", get(routes_plan::get_today_plan))
        // tasks
        .route("/tasks", get(routes_tasks::get_tasks).post(routes_tasks::create_task))
        .route("/tasks/:id", put(routes_tasks::update_task).delete(routes_tasks::delete_task))
        .route("/tasks/:id/toggle", post(routes_tasks::toggle_task))
        // settings
        .route("/settings", get(routes_tasks::get_settings).put(routes_tasks::put_settings));

    let app = Router::new()
        .nest("/api", api)
        .nest_service("/", ServeDir::new("static"));

    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();

    // Print the link to the server 
    println!("  Server running at http://{}", addr);
    println!("  Static files: http://{}/", addr);
    println!("  API base:     http://{}/api", addr);

        
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind failed");

    axum::serve(listener, app).await.expect("server error");
}
