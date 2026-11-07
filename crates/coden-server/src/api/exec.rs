mod languages;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{Json, extract::State, http::StatusCode};
use coden_agent_api::schema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tokio::sync::SemaphorePermit;
use tracing::instrument;
use utoipa::{IntoResponses, ToSchema};

use crate::api::{ApiError, ApiResult, ApiState, ErrorResponse, ErrorResponses};

#[derive(IntoResponses)]
#[allow(unused)]
pub enum VmResponses<'a> {
    #[response(
        status = SERVICE_UNAVAILABLE,
        description = "No available VMs left.",
    )]
    VmUnavailable(ErrorResponse<'a>),
}

fn acquire_vm_permit(api_state: &ApiState) -> Result<SemaphorePermit<'_>, ApiError> {
    api_state.vm_semaphore.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "No available VMs left. Try again later.",
        )
    })
}

#[instrument(skip_all)]
async fn spawn_machine(api_state: &ApiState) -> Result<coden_vm::Machine> {
    let machine_config = coden_vm::MachineConfig {
        jailer_uid: api_state.vm_jailer_uid,
        jailer_gid: api_state.vm_jailer_gid,
        kernel_image: api_state.vm_kernel.clone(),
        rootfs: api_state.vm_rootfs.clone(),
        vcpu_count: api_state.vm_vcpu_count,
        mem_size_mib: api_state.vm_mem_size_mib,
    };

    api_state
        .vm_manager
        .spawn(&machine_config)
        .await
        .context("could not spawn VM")
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecBody {
    program: String,
    args: Vec<String>,
}

/// Result of executing a program.
#[serde_as]
#[derive(Debug, Serialize, ToSchema)]
pub struct ExecResponse {
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
    path = "/api/exec",
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
#[instrument(skip_all)]
pub async fn exec(
    State(api_state): State<Arc<ApiState>>,
    Json(body): Json<ExecBody>,
) -> ApiResult<Json<ExecResponse>> {
    let _permit = acquire_vm_permit(&api_state)?;

    let machine = spawn_machine(&api_state).await?;
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
    path = "/api/run",
    request_body(content = RunBody, example = json!({
        "code": "echo hello world",
        "language": "sh",
    })),
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
#[instrument(skip_all, fields(run.language = body.language))]
pub async fn run(
    State(api_state): State<Arc<ApiState>>,
    Json(body): Json<RunBody>,
) -> ApiResult<Json<RunResponse>> {
    let Some(language) = languages::get_by_id(&body.language) else {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Unknown language"));
    };

    let _permit = acquire_vm_permit(&api_state)?;

    let machine = spawn_machine(&api_state).await?;
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
