use std::{
    borrow::Cow, 
    collections::HashMap, 
    sync::{Arc, RwLock}, 
    time::Duration};
use axum::{
    error_handling::HandleErrorLayer, 
    extract::{Path, State}, 
    handler::Handler, 
    http::StatusCode, 
    response::IntoResponse, 
    routing::{get, post}, 
    Router};
use tower::{BoxError, ServiceBuilder};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let shared_state = SharedState::default();

    let app = Router::new()
        .route("/keys", get(list_keys))
        .route(
            "/:key", 
            get(kv_get),
        )
        .route(
            "/:key", 
            post(kv_set),
        )
        .layer(
            ServiceBuilder ::new()
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .timeout(Duration::from_secs(10))
        )
        .with_state(Arc::clone(&shared_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

type SharedState = Arc<RwLock<AppState>>;

#[derive(Default)]
struct AppState {
    db: HashMap<String, String>
}

async fn kv_get(
    Path(key): Path<String>,
    State(state): State<SharedState>,
) -> Result<String, StatusCode> {
    let db = &state.read().unwrap().db;

    if let Some(value) = db.get(&key) {
        Ok(value.clone())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn kv_set(Path(key): Path<String>, State(state): State<SharedState>, string: String) {
    state.write().unwrap().db.insert(key, string);
}

async fn list_keys(State(state): State<SharedState>) -> String {
    let db = &state.read().unwrap().db;
    let mut s: String = "".to_string();

    for (key, value) in db {
        s.insert_str(s.len(), key);
        s.insert_str(s.len(), value);
    }
    s
}

async fn handle_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, Cow::from("request timed out"));
    }

    if error.is::<tower::load_shed::error::Overloaded>() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Cow::from("service is overloaded, try again later"),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Cow::from(format!("Unhandled internal error: {error}")),
    )
}
