use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Json, Router};
use coden_agent_api::schema;
use coden_vm::MachineConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::serde_as;
use tracing::error;
use utoipa::{IntoResponses, OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;

#[derive(Serialize, ToSchema, IntoResponses)]
#[response(
    status = INTERNAL_SERVER_ERROR,
    description = "Internal server error",
    example = json!({ "message": "Internal server error" }),
)]
pub struct ErrorResponse<'a> {
    message: &'a str,
}

pub struct ApiError {
    pub message: String,
    pub status: StatusCode,
    pub source: Option<anyhow::Error>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let mut res = (
            self.status,
            Json(ErrorResponse {
                message: &self.message,
            }),
        )
            .into_response();

        if let Some(e) = self.source {
            res.extensions_mut().insert(Arc::new(e));
        }

        res
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self {
            message: "Internal server error".into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            source: Some(value),
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

pub fn router() -> Router<AppState> {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(get_openapi, exec))
        .split_for_parts();

    let scalar_config = json!({
        "url": "/openapi.json",
        "featuredClients": ["shell/curl", "rust/reqwest", "js/fetch"],
        "agent": {
            "disabled": true,
        },
        "mcp": {
            "disabled": true,
        },
        "hideClientButton": true,
    });

    router
        .route("/", get(root))
        .merge(scalar_api_reference::axum::router("/docs", &scalar_config).with_state(()))
        .layer(Extension(api))
        .layer(middleware::from_fn(log_errors))
}

#[derive(OpenApi)]
#[openapi(info(title = "Coden API"), components(schemas(ErrorResponse)))]
struct ApiDoc;

async fn log_errors(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    if let Some(err) = response.extensions().get::<Arc<anyhow::Error>>() {
        error!(?err, "unhandled error in handler");
    }
    response
}

async fn root() -> String {
    format!(
        r"
      _____          _ 
     / ____|        | |
    | |     ___   __| | ___ _ __ 
    | |    / _ \ / _` |/ _ \ '_ \
    | |___| (_) | (_| |  __/ | | |
     \_____\___/ \__,_|\___|_| |_|

    -----------------------------------------------------

    {}/{}

    Source: {}
    License: {}

    For API documentation, see `/docs`.
    For the OpenAPI document, see `/openapi.json`.

    -----------------------------------------------------
",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY"),
        env!("CARGO_PKG_LICENSE"),
    )
}

/// OpenAPI document (JSON)
#[utoipa::path(
    get,
    path = "/openapi.json",
    responses(
        (status = OK, body = utoipa::openapi::Object, description = "The OpenAPI document.")
    ),
)]
async fn get_openapi(
    Extension(openapi): Extension<utoipa::openapi::OpenApi>,
) -> Json<utoipa::openapi::OpenApi> {
    Json(openapi)
}

#[derive(Debug, Deserialize, ToSchema)]
struct ExecBody {
    program: String,
    args: Vec<String>,
}

/// Result of executing a program.
#[serde_as]
#[derive(Debug, Serialize, ToSchema)]
struct ExecResponse {
    /// Time (in ms) it took to execute the program.
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    #[schema(value_type = usize)]
    pub time: Duration,
    /// The output of the program.
    pub output: String,
    /// The exit status of the program.
    pub status: ExecResponseStatus,
}

impl From<schema::ExecResponse> for ExecResponse {
    fn from(value: schema::ExecResponse) -> Self {
        Self {
            time: value.time,
            output: value.output,
            status: value.status.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecResponseStatus {
    /// The program exited with a code.
    Code(i32),
    /// The program exited due to a signal.
    Signal(i32),
    /// Other exit reason.
    #[serde(untagged)]
    Other,
}

impl From<schema::ExecResponseStatus> for ExecResponseStatus {
    fn from(value: schema::ExecResponseStatus) -> Self {
        match value {
            schema::ExecResponseStatus::Code(v) => Self::Code(v),
            schema::ExecResponseStatus::Signal(v) => Self::Signal(v),
            schema::ExecResponseStatus::Other => Self::Other,
        }
    }
}

/// Execute program
#[utoipa::path(
    post,
    path = "/exec",
    request_body(content = ExecBody, examples(
        ("1. Basic" = (value = json!({
            "program": "echo",
            "args": ["Hello, world!"],
        }))),
        ("2. Shell" = (value = json!({
            "program": "sh",
            "args": ["-c", "echo \"Hello, $USER!\""],
        }))),
    )),
    responses(
        (
            status = OK,
            body = ExecResponse,
            description = "Program executed successfully",
            examples(
                ("1. Success" = (value = json!({
                    "time": 4,
                    "output": "Hello, world!\n",
                    "status": { "code": 0 },
                }))),
                ("2. Signal" = (value = json!({
                    "time": 5,
                    "output": "",
                    "status": { "signal": 15 },
                }))),
                ("3. Other" = (value = json!({
                    "time": 12,
                    "output": "",
                    "status": "other",
                }))),
            )
        ),
        ErrorResponse,
    ),
)]
async fn exec(app_state: AppState, Json(body): Json<ExecBody>) -> ApiResult<Json<ExecResponse>> {
    let machine_config = MachineConfig {
        jailer_uid: app_state.config.jailer_uid,
        jailer_gid: app_state.config.jailer_gid,
        kernel_image: app_state.vm_vmlinux.clone(),
        rootfs: app_state.vm_rootfs.clone(),
        vcpu_count: app_state.config.vm_vcpu_count,
        mem_size_mib: app_state.config.vm_mem_size_mib,
    };
    let machine = app_state
        .vm_manager
        .spawn(&machine_config)
        .await
        .context("could not spawn VM")?;

    let res = machine
        .request(schema::ExecRequest {
            program: body.program,
            args: body.args,
        })
        .await
        .context("could not execute program in VM")?;

    Ok(Json(res.into()))
}
