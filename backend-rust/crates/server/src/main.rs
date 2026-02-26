use server::clients;
use server::config;
use server::db;
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

    // Initialize SQS client for server-side analysis (optional)
    let analysis_queue = clients::sqs::AnalysisQueue::new(&config).await;
    if analysis_queue.is_some() {
        tracing::info!("SQS analysis queue configured");
    } else {
        tracing::info!("SQS not configured - server-side analysis disabled");
    }

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

    // Backfill first_inaccuracy_move with mistake/blunder keys for old games
    tokio::spawn({
        let pool = pool.clone();
        async move {
            match db::analysis::backfill_first_bad_moves(&pool).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("Backfilled first_bad_move data for {} games", n),
                Err(e) => tracing::warn!("Failed to backfill first_bad_moves: {}", e),
            }
        }
    });

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
        .route("/api/users/me", put(routes::profile::update_profile).delete(routes::profile::delete_account))
        .route("/api/users/me/games", get(routes::games::get_my_games))
        // Games — order matters: specific routes before parameterized
        .route("/api/games/sync", post(routes::games::sync_games))
        .route("/api/games/stored", get(routes::games::get_stored_games))
        .route("/api/games/tags", get(routes::games::get_game_tags))
        .route("/api/games/analyze-server", post(routes::games::analyze_server))
        .route("/api/games/backfill", post(routes::games::backfill_games))
        .route("/api/games/backfill/status", get(routes::games::backfill_status))
        .route("/api/games/stats", get(routes::dashboard::get_game_stats))
        .route("/api/games/endgame-stats", get(routes::endgame::get_endgame_stats))
        .route("/api/games/{game_id}", get(routes::games::get_game_by_id))
        .route(
            "/api/games/{game_id}/analysis",
            get(routes::games::get_game_analysis)
                .post(routes::games::save_game_analysis),
        )
        // Puzzles
        .route("/api/puzzles/stats", get(routes::puzzles::get_puzzle_stats))
        .route("/api/puzzles", get(routes::puzzles::get_puzzles))
        // Admin — titled players
        .route("/api/admin/titled-players/refresh", post(routes::titled_players::refresh_titled_players))
        .route("/api/admin/backfill-titled-tags", post(routes::titled_players::backfill_titled_tags))
        // Opening tree
        .route("/api/opening-tree", get(routes::opening_tree::get_opening_tree))
        // Opening book (master games)
        .route("/api/opening-book/check", get(routes::opening_book::check_book_move))
        .route("/api/admin/opening-book/reclassify", post(routes::opening_book::reclassify_book_moves))
        // Trainer
        .route("/api/trainer/openings", get(routes::trainer::list_openings))
        .route("/api/trainer/puzzles", get(routes::trainer::get_puzzles))
        .route("/api/trainer/progress", post(routes::trainer::mark_complete))
        .route("/api/admin/trainer/list", get(routes::trainer::admin_list_openings))
        .route("/api/admin/trainer/upload", post(routes::trainer::upload_puzzles))
        .route("/api/admin/trainer/delete", post(routes::trainer::delete_opening))
        // User profile (parameterized — must be last)
        .route("/api/users/{username}", get(routes::profile::get_user_profile))
        // Shared state
        .layer(Extension(pool))
        .layer(Extension(config.clone()))
        .layer(Extension(analysis_queue))
        .layer(cors);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Starting server on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}
