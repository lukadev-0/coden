mod exec;

use std::borrow::Cow;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum::{Router, extract::FromRef, http::StatusCode};
use serde::Serialize;
use serde_json::json;
use tokio::{fs, sync::Semaphore};
use tracing::instrument;
use utoipa::{IntoResponses, OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::config::Config;

pub struct ApiState {
    vm_semaphore: Semaphore,
    vm_manager: coden_vm::Manager,
    vm_kernel: coden_vm::Resource,
    vm_rootfs: coden_vm::Resource,
    vm_vcpu_count: usize,
    vm_mem_size_mib: usize,
    vm_jailer_uid: u32,
    vm_jailer_gid: u32,
}

impl ApiState {
    #[instrument(name = "api.init", skip_all)]
    pub async fn init(config: &Config) -> Result<Self> {
        fs::create_dir_all(&config.state_dir)
            .await
            .with_context(|| {
                format!(
                    "could not create directory '{}'",
                    config.state_dir.display()
                )
            })?;

        let vm_semaphore = Semaphore::new(config.max_running_vms);

        let vm_manager = coden_vm::Manager::new(&coden_vm::ManagerOptions {
            state_dir: config.state_dir.clone(),
            firecracker: config.firecracker.clone(),
            firecracker_jailer: config.firecracker_jailer.clone(),
        })
        .await
        .context("could not create VM manager")?;

        let vm_kernel = vm_manager
            .load_resource("vmlinux", &config.vm_kernel)
            .await
            .context("could not load vmlinux into VM manager")?;
        let vm_rootfs = vm_manager
            .load_resource("rootfs", &config.vm_rootfs)
            .await
            .context("could not load rootfs into VM manager")?;

        Ok(Self {
            vm_semaphore,
            vm_manager,
            vm_kernel,
            vm_rootfs,
            vm_vcpu_count: config.vm_vcpu_count,
            vm_mem_size_mib: config.vm_mem_size_mib,
            vm_jailer_uid: config.vm_jailer_uid,
            vm_jailer_gid: config.vm_jailer_gid,
        })
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.vm_manager
            .shutdown()
            .await
            .context("failed to shut down vm manager")?;
        Ok(())
    }
}

#[derive(OpenApi)]
#[openapi(info(title = "Coden API"), components(schemas(ErrorResponse)))]
struct ApiDoc;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<ApiState>: FromRef<S>,
{
    let (mut router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(get_openapi))
        .routes(routes!(exec::exec))
        .routes(routes!(exec::run))
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
        .merge(scalar_api_reference::axum::router("/docs", &scalar_config).with_state(()))
        .layer(Extension(api));

    router
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

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse<'a> {
    message: &'a str,
}

#[derive(IntoResponses)]
#[allow(unused)]
pub enum ErrorResponses<'a> {
    #[response(
        status = INTERNAL_SERVER_ERROR,
        description = "Internal server error.",
        example = json!({ "message": "Internal server error." }),
    )]
    InternalServerError(ErrorResponse<'a>),
}

pub struct ApiError {
    pub message: Cow<'static, str>,
    pub status: StatusCode,
    pub source: Option<anyhow::Error>,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<Cow<'static, str>>) -> Self {
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
