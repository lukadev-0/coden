use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
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
use tokio::sync::SemaphorePermit;
use tracing::error;
use utoipa::{IntoResponses, OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{AppState, languages};

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse<'a> {
    message: &'a str,
}

#[derive(IntoResponses)]
#[response(
    status = INTERNAL_SERVER_ERROR,
    description = "Internal server error.",
    example = json!({ "message": "Internal server error." }),
)]
#[allow(unused)]
pub struct ErrorResponses<'a>(ErrorResponse<'a>);

pub struct ApiError {
    pub message: String,
    pub status: StatusCode,
    pub source: Option<anyhow::Error>,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status,
            source: None,
        }
    }
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
            message: "Internal server error.".into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            source: Some(value),
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

pub fn router() -> Router<AppState> {
    let (mut router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(get_openapi))
        .routes(routes!(exec))
        .routes(routes!(run))
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

    router = router
        .route("/", get(root))
        .merge(scalar_api_reference::axum::router("/docs", &scalar_config).with_state(()))
        .layer(Extension(api))
        .layer(middleware::from_fn(log_errors));

    #[cfg(copyright_page)]
    {
        let html = include_str!(concat!(env!("OUT_DIR"), "/copyright.html"));
        router = router.route("/copyright", get(async move || axum::response::Html(html)));
    }

    router
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
    License: {}{}

    For API documentation, see `/docs`.
    For the OpenAPI document, see `/openapi.json`.

    -----------------------------------------------------
",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY"),
        env!("CARGO_PKG_LICENSE"),
        if cfg!(copyright_page) {
            " (also see /copyright)"
        } else {
            ""
        }
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

#[derive(IntoResponses)]
#[response(
    status = SERVICE_UNAVAILABLE,
    description = "No available VMs left.",
)]
#[allow(unused)]
pub struct VmResponses<'a>(ErrorResponse<'a>);

fn acquire_vm_permit(app_state: &AppState) -> Result<SemaphorePermit<'_>, ApiError> {
    app_state.vm_semaphore.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "No available VMs left. Try again later.",
        )
    })
}

async fn spawn_machine(app_state: &AppState) -> Result<coden_vm::Machine> {
    let machine_config = MachineConfig {
        jailer_uid: app_state.config.jailer_uid,
        jailer_gid: app_state.config.jailer_gid,
        kernel_image: app_state.vm_vmlinux.clone(),
        rootfs: app_state.vm_rootfs.clone(),
        vcpu_count: app_state.config.vm_vcpu_count,
        mem_size_mib: app_state.config.vm_mem_size_mib,
    };
    app_state
        .vm_manager
        .spawn(&machine_config)
        .await
        .context("could not spawn VM")
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
            description = "Program executed successfully.",
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
        VmResponses,
        ErrorResponses,
    ),
)]
async fn exec(app_state: AppState, Json(body): Json<ExecBody>) -> ApiResult<Json<ExecResponse>> {
    let _permit = acquire_vm_permit(&app_state)?;

    let machine = spawn_machine(&app_state).await?;
    let res = machine
        .request(schema::ExecRequest {
            program: body.program,
            args: body.args,
        })
        .await
        .context("could not execute program in VM")?;

    Ok(Json(res.into()))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RunBody {
    language: String,
    code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunResponse {
    /// Result after executing the compiler.
    /// This is `null` if the language does not require compilation.
    compiler: Option<ExecResponse>,
    /// Result after executing the (compiled) program.
    /// This is `null` if compilation failed.
    program: Option<ExecResponse>,
}

/// Compile & execute code
#[utoipa::path(
    post,
    path = "/run",
    responses(
        (
            status = OK,
            body = RunResponse,
            description = "Compiler and program output.",
        ),
        VmResponses,
        ErrorResponses,
    ),
)]
async fn run(app_state: AppState, Json(body): Json<RunBody>) -> ApiResult<Json<RunResponse>> {
    let Some(language) = languages::get_by_id(&body.language) else {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Unknown language"));
    };

    let _permit = acquire_vm_permit(&app_state)?;

    let machine = spawn_machine(&app_state).await?;
    machine
        .request(schema::WriteFileRequest {
            path: language.source_file.into(),
            contents: body.code.into(),
        })
        .await
        .context("could not write program code into VM")?;

    let compiler_res = if let Some(command) = language.compile {
        Some(
            machine
                .request(schema::ExecRequest {
                    program: command
                        .first()
                        .expect("compile command should have at least one item")
                        .to_string(),
                    args: command.iter().skip(1).map(|s| s.to_string()).collect(),
                })
                .await
                .context("could not execute compiler in VM")?,
        )
    } else {
        None
    };

    let program_res = if compiler_res
        .as_ref()
        .is_none_or(|c| c.status == schema::ExecResponseStatus::Code(0))
    {
        Some(
            machine
                .request(schema::ExecRequest {
                    program: language
                        .execute
                        .first()
                        .expect("execute command should have at least one item")
                        .to_string(),
                    args: language
                        .execute
                        .iter()
                        .skip(1)
                        .map(|s| s.to_string())
                        .collect(),
                })
                .await
                .context("could not execute program in VM")?,
        )
    } else {
        None
    };

    Ok(Json(RunResponse {
        compiler: compiler_res.map(Into::into),
        program: program_res.map(Into::into),
    }))
}
