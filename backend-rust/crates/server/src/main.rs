mod auth;
mod clients;
mod config;
mod db;
mod error;
mod routes;

use axum::{routing::{get, post, put}, Extension, Router};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Load .env if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = config::Config::from_env();

    // Connect to Postgres
    tracing::info!("Connecting to database...");
    let pool = db::pool::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Run schema migrations
    tracing::info!("Running migrations...");
    db::pool::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router — same paths as Python FastAPI
    let app = Router::new()
        // Health
        .route("/health", get(routes::health::health_check))
        // Auth
        .route("/api/auth/register", post(routes::auth::register))
        .route("/api/auth/login", post(routes::auth::login))
        .route("/api/auth/me", get(routes::auth::me))
        // Profile — must be before /api/users/{username}
        .route("/api/users/me", put(routes::profile::update_profile))
        .route("/api/users/me/games", get(routes::games::get_my_games))
        // Games — order matters: specific routes before parameterized
        .route("/api/games/sync", post(routes::games::sync_games))
        .route("/api/games/sync/lichess", post(routes::games::sync_lichess_games))
        .route("/api/games/stored", get(routes::games::get_stored_games))
        .route("/api/games/tags", get(routes::games::get_game_tags))
        .route("/api/games/stats", get(routes::dashboard::get_game_stats))
        .route("/api/games/analyze", post(routes::games::analyze_games))
        .route("/api/games/{game_id}", get(routes::games::get_game_by_id))
        .route(
            "/api/games/{game_id}/analysis",
            get(routes::games::get_game_analysis)
                .post(routes::games::save_game_analysis),
        )
        // Opening tree
        .route("/api/opening-tree", get(routes::opening_tree::get_opening_tree))
        // Posts
        .route("/api/posts", post(routes::posts::create_post).get(routes::posts::get_posts))
        // User profile + posts (parameterized — must be last)
        .route("/api/users/{username}", get(routes::profile::get_user_profile))
        .route("/api/users/{username}/posts", get(routes::posts::get_user_posts))
        // Shared state
        .layer(Extension(pool))
        .layer(Extension(config.clone()))
        .layer(cors);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Starting server on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}
