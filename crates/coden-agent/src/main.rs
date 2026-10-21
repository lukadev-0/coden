use std::{os::unix::process::ExitStatusExt, process::Stdio, time::Instant};

use anyhow::{Context, Result};
use coden_agent_api::VSOCK_PORT;
use coden_agent_api::schema::{self, ExecResponseStatus, Request};
use coden_agent_api::server::Server;
use nix::unistd::User;
use serde_json::to_value;
use tokio::{io::AsyncReadExt, net::unix::pipe::pipe, process::Command};
use tokio_vsock::{VsockAddr, VsockStream};
use tracing::{Instrument, Level, debug, error, info, info_span};

const RUNNER_USERNAME: &str = "runner";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_ansi(false)
        .init();

    let runner_user = User::from_name(RUNNER_USERNAME)
        .with_context(|| format!("could not lookup user '{RUNNER_USERNAME}'"))?
        .with_context(|| format!("no user named '{RUNNER_USERNAME}'"))?;

    let mut state = State { runner_user };

    let stream = VsockStream::connect(VsockAddr::new(2, VSOCK_PORT))
        .await
        .context("could not connect to vsock")?;

    info!("vsock connected");

    let mut server = Server::new(stream);
    while let Some(request) = server.read_request().await? {
        let span = info_span!("request", ?request);
        match handle_request(request, &mut state)
            .instrument(span.clone())
            .await
        {
            Ok(res) => {
                server
                    .send_response(&res)
                    .await
                    .context("could not send response")?;
            }
            Err(e) => error!(parent: span, err = ?e, "error"),
        };
    }

    info!("finished");

    Ok(())
}

struct State {
    runner_user: User,
}

async fn handle_request(request: Request, state: &mut State) -> Result<serde_json::Value> {
    debug!("request");
    match request {
        Request::Exec(exec) => {
            let (stdout_writer, mut output_reader) = pipe()?;
            let stdout_writer = stdout_writer.into_blocking_fd()?;
            let stderr_writer = stdout_writer.try_clone()?;

            let start = Instant::now();
            let mut child = Command::new(exec.program)
                .args(exec.args)
                .stdin(Stdio::null())
                .stdout(stdout_writer)
                .stderr(stderr_writer)
                .uid(state.runner_user.uid.as_raw())
                .uid(state.runner_user.gid.as_raw())
                .env_clear()
                .env("LOGNAME", &state.runner_user.name)
                .env("USER", &state.runner_user.name)
                .env("HOME", &state.runner_user.dir)
                .env("SHELL", &state.runner_user.shell)
                .env("PATH", "/bin")
                .current_dir(&state.runner_user.dir)
                .spawn()
                .context("could not spawn program")?;

            let mut output = Vec::new();
            output_reader.read_to_end(&mut output).await?;
            let output = String::from_utf8_lossy_owned(output);

            let status = child.wait().await.context("could not wait for child")?;
            let elapsed = start.elapsed();

            Ok(to_value(schema::ExecResponse {
                time: elapsed,
                output,
                status: None
                    .or(status.code().map(ExecResponseStatus::Code))
                    .or(status.signal().map(ExecResponseStatus::Signal))
                    .unwrap_or(ExecResponseStatus::Other),
            })?)
        }
    }
}
