//! Nature's Thread server: one listener for REST, WebSocket and the built client.

mod auth;
mod config;
mod director;
mod error;
mod http;
mod hub;
mod meta;
mod persist;
mod ratelimit;
mod river;
mod room;
mod ws;

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use std::sync::Arc;

pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: config::Config,
    pub hub: Arc<hub::Hub>,
    pub limiter: ratelimit::RateLimiter,
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../database/migrations");

fn load_tls(cert: &str, key: &str) -> anyhow::Result<rustls::ServerConfig> {
    use std::{fs::File, io::BufReader};
    let certs = rustls_pemfile::certs(&mut BufReader::new(File::open(cert)?)).collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(File::open(key)?))?
        .ok_or_else(|| anyhow::anyhow!("no private key found in TLS_KEY"))?;
    Ok(rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key)?)
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,natures_thread_server=info".into()),
        )
        .init();

    let cfg = match config::Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("{e}");
            eprintln!("{e}\nSee .env.example for every variable.");
            std::process::exit(2);
        }
    };
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing::info!(config = ?cfg, "starting Nature's Thread server");
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await
        .map_err(|e| {
            anyhow::anyhow!("cannot connect to PostgreSQL at {}: {e}", config::redact_db_url(&cfg.database_url))
        })?;
    MIGRATOR.run(&db).await.map_err(|e| anyhow::anyhow!("database migration failed: {e}"))?;
    tracing::info!("database migrations up to date");
    if std::env::args().any(|a| a == "--migrate-only") {
        println!("Migrations applied; database is up to date.");
        return Ok(());
    }

    let hub = Arc::new(hub::Hub::default());
    let open = persist::load_open_rooms(&db).await?;
    tracing::info!(rooms = open.len(), "restored open rooms");
    hub.restore(open).await;

    // Expire idle rooms periodically.
    {
        let (db, hub, hours) = (db.clone(), hub.clone(), cfg.room_idle_expiry_hours);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                tick.tick().await;
                match persist::expire_idle_rooms(&db, hours).await {
                    Ok(ids) if !ids.is_empty() => {
                        tracing::info!(count = ids.len(), "expired idle rooms");
                        hub.expire(&ids).await;
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "room expiry failed"),
                }
            }
        });
    }

    let bind = (cfg.host.clone(), cfg.http_port);
    let tls = match (&cfg.tls_cert, &cfg.tls_key) {
        (Some(c), Some(k)) => Some(load_tls(c, k)?),
        _ => None,
    };
    let scheme = if tls.is_some() { "https" } else { "http" };
    tracing::info!("listening on {scheme}://{}:{} (WebSocket at /ws)", bind.0, bind.1);
    if tls.is_none() {
        tracing::warn!(
            "serving plain HTTP: browsers on other devices will use WebGL2 instead of WebGPU (run `make certs`)"
        );
    }

    let state = web::Data::new(AppState { db, config: cfg.clone(), hub, limiter: ratelimit::RateLimiter::default() });
    let static_dir = cfg.static_dir.clone();
    let client_built = std::path::Path::new(&static_dir).join("index.html").is_file();
    if !client_built {
        tracing::warn!(dir = %static_dir, "client build not found; run `make build-client` (API and WebSocket still work)");
    }
    let origins = cfg.allowed_origins.clone();
    let dev = cfg.is_dev();

    let server = HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![actix_web::http::header::AUTHORIZATION, actix_web::http::header::CONTENT_TYPE])
            .max_age(3600);
        for o in &origins {
            cors = cors.allowed_origin(o);
        }
        if dev {
            // Same-LAN dev clients (Vite on another device) are allowed in development only.
            cors = cors.allowed_origin_fn(|origin, _| {
                origin.to_str().is_ok_and(|o| o.starts_with("http://localhost:") || o.starts_with("https://localhost:"))
            });
        }
        let index = format!("{}/index.html", static_dir);
        App::new()
            .app_data(state.clone())
            .app_data(web::JsonConfig::default().limit(16 * 1024).error_handler(|err, _| {
                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::BadRequest().json(
                        serde_json::json!({ "code": "bad_json", "message": "The request body was not valid JSON." }),
                    ),
                )
                .into()
            }))
            .wrap(cors)
            .wrap(middleware::Compress::default())
            // %U = path without the query string, so nothing sensitive is logged.
            .wrap(middleware::Logger::new("%a \"%m %U\" %s %b %Dms"))
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("Referrer-Policy", "no-referrer"))
                    .add(("X-Frame-Options", "DENY")),
            )
            .route("/health", web::get().to(http::health))
            .route("/ready", web::get().to(http::ready))
            .route("/ws", web::get().to(ws::ws_handler))
            .service(
                web::scope("/api")
                    .route("/content", web::get().to(http::content))
                    .route("/auth/register", web::post().to(auth::register))
                    .route("/auth/login", web::post().to(auth::login))
                    .route("/auth/logout", web::post().to(auth::logout))
                    .route("/me", web::get().to(auth::me))
                    .route("/me/progress", web::get().to(http::progress))
                    .route("/me/rooms", web::get().to(http::my_rooms))
                    .route("/me/profile", web::get().to(meta::profile))
                    .route("/me/daily", web::post().to(meta::claim_daily))
                    .route("/me/road", web::post().to(meta::claim_road))
                    .route("/shop/buy", web::post().to(meta::buy))
                    .route("/shop/undo", web::post().to(meta::refund))
                    .route("/garage/equip", web::post().to(meta::equip))
                    .route("/rooms", web::post().to(http::create_room))
                    .route("/rooms/join", web::post().to(http::join_room))
                    .route("/rooms/{id}", web::get().to(http::get_room))
                    .route("/rooms/{id}/results", web::get().to(http::room_results)),
            )
            .configure(|cfg| {
                if client_built {
                    // Built client, with index.html as the fallback for client-side routes.
                    cfg.service(actix_files::Files::new("/", &static_dir).index_file("index.html").default_handler(
                        actix_web::dev::fn_service(move |req: actix_web::dev::ServiceRequest| {
                            let index = index.clone();
                            async move {
                                let (req, _) = req.into_parts();
                                let file = actix_files::NamedFile::open_async(&index).await?;
                                let res = file.into_response(&req);
                                Ok(actix_web::dev::ServiceResponse::new(req, res))
                            }
                        }),
                    ));
                } else {
                    cfg.default_service(web::to(|| async {
                        HttpResponse::NotFound().json(serde_json::json!({
                            "code": "client_not_built",
                            "message": "The game client has not been built. Run `make build-client`."
                        }))
                    }));
                }
            })
    })
    .shutdown_timeout(10);

    let server = match tls {
        Some(t) => server.bind_rustls_0_23(bind, t)?,
        None => server.bind(bind)?,
    };
    server.run().await?;
    Ok(())
}
