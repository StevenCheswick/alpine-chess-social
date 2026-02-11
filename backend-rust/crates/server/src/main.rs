use server::auth;
use server::clients;
use server::config;
use server::db;
use server::error;
use server::routes;

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

    // Load titled players cache (and seed from Chess.com API on first run)
    let titled_count = db::titled_players::load_cache(&pool)
        .await
        .expect("Failed to load titled players cache");
    if titled_count == 0 {
        tracing::info!("Titled players table is empty — seeding from Chess.com API...");
        tokio::spawn({
            let pool = pool.clone();
            async move {
                match db::titled_players::seed_from_chesscom(&pool).await {
                    Ok(count) => tracing::info!("Seeded {} titled players from Chess.com", count),
                    Err(e) => tracing::warn!("Failed to seed titled players: {}", e),
                }
            }
        });
    }

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router — same paths as Python FastAPI
    let app = Router::new()
        // Analysis WebSocket
        .route("/api/ws/analyze", get(routes::analysis_ws::ws_handler))
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
        .route("/api/games/endgame-stats", get(routes::endgame::get_endgame_stats))
        .route("/api/games/analyze", post(routes::games::analyze_games))
        .route("/api/games/{game_id}", get(routes::games::get_game_by_id))
        .route(
            "/api/games/{game_id}/analysis",
            get(routes::games::get_game_analysis)
                .post(routes::games::save_game_analysis),
        )
        // Puzzles
        .route("/api/puzzles", get(routes::puzzles::get_puzzles))
        // Admin — titled players
        .route("/api/admin/titled-players/refresh", post(routes::titled_players::refresh_titled_players))
        .route("/api/admin/backfill-titled-tags", post(routes::titled_players::backfill_titled_tags))
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
