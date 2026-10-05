//! evensplit-web: one trip file, edited from a local page. Every change is
//! checked, then written straight back to the file.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use evensplit::{Report, Trip};
use serde_json::{Value, json};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Parser)]
#[command(version, about = "Split a trip's costs from a page in your browser")]
struct Cli {
    /// The trip file. It is created if it doesn't exist yet.
    #[arg(default_value = "trip.json")]
    trip: PathBuf,
    /// Currency for a new trip, like INR or EUR. Ignored if the file exists.
    #[arg(long, default_value = "EUR")]
    currency: String,
    /// Port on 127.0.0.1 to listen on.
    #[arg(long, default_value_t = 7676)]
    port: u16,
    /// Serve the UI from this directory instead of the copy built into the
    /// binary. Handy while working on the UI.
    #[arg(long)]
    static_dir: Option<PathBuf>,
}

struct App {
    path: PathBuf,
    // One lock around the trip and its file, so two saves never interleave.
    trip: Mutex<Trip>,
}

/// The trip and everything worked out from it. A trip edited by hand can
/// have mistakes; then `error` says what, and the page shows it.
fn view(trip: &Trip) -> Value {
    match Report::new(trip) {
        Ok(r) => json!({
            "trip": trip,
            "report": r,
            "summary": r.text(trip.expenses.len()),
            "error": null,
        }),
        Err(e) => json!({ "trip": trip, "report": null, "summary": null, "error": e.to_string() }),
    }
}

async fn get_trip(State(app): State<Arc<App>>) -> Json<Value> {
    Json(view(&app.trip.lock().unwrap()))
}

/// Replace the whole trip. The page sends all of it on every change; a trip
/// is a few kilobytes, and it keeps the file and the page from drifting.
async fn put_trip(State(app): State<Arc<App>>, Json(next): Json<Trip>) -> Response {
    if let Err(e) = Report::new(&next) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    let mut trip = app.trip.lock().unwrap();
    if let Err(e) = next.save(&app.path) {
        let msg = format!(
            "Couldn't save to {}: {e}. Your last change isn't saved; check the folder can be written to and try again.",
            app.path.display()
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": msg })),
        )
            .into_response();
    }
    *trip = next;
    Json(view(&trip)).into_response()
}

#[derive(rust_embed::RustEmbed)]
#[folder = "web/dist"]
#[allow_missing = true]
struct Ui;

async fn embedded_ui(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let (file, path) = match Ui::get(path) {
        Some(f) if !path.is_empty() => (f, path),
        _ => match Ui::get("index.html") {
            Some(f) => (f, "index.html"),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    "This build has no UI. Run `pnpm --dir web build` and rebuild, or pass --static-dir.",
                )
                    .into_response();
            }
        },
    };
    // Vite puts a hash in asset names, so those can be kept forever.
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [
            (header::CONTENT_TYPE, file.metadata.mimetype().to_string()),
            (header::CACHE_CONTROL, cache.to_string()),
        ],
        file.data,
    )
        .into_response()
}

fn router(app: Arc<App>, static_dir: Option<PathBuf>) -> Router {
    let api = Router::new()
        .route("/api/trip", get(get_trip).put(put_trip))
        .with_state(app);
    match static_dir {
        Some(dir) => api
            .fallback_service(ServeDir::new(&dir).fallback(ServeFile::new(dir.join("index.html")))),
        None => api.fallback(embedded_ui),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let trip = if cli.trip.exists() {
        Trip::load(&cli.trip)
            .map_err(|e| anyhow::anyhow!("can't read {}: {e}", cli.trip.display()))?
    } else {
        let name = cli
            .trip
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let t = Trip::new(&name, &cli.currency.to_uppercase());
        t.save(&cli.trip)
            .map_err(|e| anyhow::anyhow!("can't create {}: {e}", cli.trip.display()))?;
        t
    };
    let app = Arc::new(App {
        path: cli.trip.clone(),
        trip: Mutex::new(trip),
    });
    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!(
        "evensplit is running at http://{addr}, saving to {}",
        cli.trip.display()
    );
    axum::serve(listener, router(app, cli.static_dir))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_bad_change_is_refused_and_the_file_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trip.json");
        let mut trip = Trip::new("Test", "EUR");
        trip.people = vec!["Asha".into(), "Ben".into()];
        trip.save(&path).unwrap();
        let app = Arc::new(App {
            path: path.clone(),
            trip: Mutex::new(trip.clone()),
        });

        let mut bad = trip.clone();
        bad.people.push("Asha".into());
        let r = put_trip(State(app.clone()), Json(bad)).await;
        assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(Trip::load(&path).unwrap(), trip);

        let mut good = trip.clone();
        good.people.push("Chitra".into());
        let r = put_trip(State(app.clone()), Json(good.clone())).await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(Trip::load(&path).unwrap(), good);
        assert_eq!(*app.trip.lock().unwrap(), good);
    }
}
