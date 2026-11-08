use axum::Router;
use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rust_embed::Embed;

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/", get(root))
        .route("/{*file}", get(static_file))
}

#[derive(Embed)]
#[folder = "../../frontend/build"]
struct Asset;

async fn root(headers: HeaderMap) -> impl IntoResponse {
    serve_asset("pages/index.html", &headers)
}

async fn static_file(Path(path): Path<String>, headers: HeaderMap) -> impl IntoResponse {
    serve_asset(&format!("static/{path}"), &headers)
}

fn serve_asset(path: &str, headers: &HeaderMap) -> Response {
    match Asset::get(path) {
        Some(content) => {
            let etag = hex::encode(content.metadata.sha256_hash());
            let cache_control = if path.starts_with("static/_app/immutable/") {
                "public, max-age=31536000, immutable"
            } else {
                "public, max-age=3600"
            };

            let res_headers = [
                (header::CONTENT_TYPE, content.metadata.mimetype()),
                (header::CACHE_CONTROL, cache_control),
                (header::ETAG, &etag),
            ];

            if headers
                .get_all(header::IF_NONE_MATCH)
                .iter()
                .any(|v| *v == *etag)
            {
                return (StatusCode::NOT_MODIFIED, res_headers).into_response();
            }

            (res_headers, content.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "404 not found").into_response(),
    }
}
