use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// Turns a JSON newspaper issue into a typeset, Kindle-ready EPUB.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP API.
    Serve {
        /// Address to listen on.
        #[arg(long, env = "BIND", default_value = "0.0.0.0:8080")]
        bind: SocketAddr,
        /// Require `Authorization: Bearer <token>` on /v1/* routes.
        #[arg(long, env = "RENDER_TOKEN", hide_env_values = true)]
        token: Option<String>,
    },
    /// Render an issue file to EPUB.
    Render {
        /// Path to an issue.v1 JSON file (`-` for stdin).
        input: PathBuf,
        /// Output EPUB path. Defaults to `<name>-<date>.epub`.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Also write the cover JPEG to this path.
        #[arg(long)]
        cover: Option<PathBuf>,
    },
    /// Check an issue file without rendering it.
    Validate { input: PathBuf },
    /// Print the JSON Schema of the issue format.
    Schema,
    /// List built-in themes.
    Themes,
    /// Exit 0 if the server answers on /healthz (for container health checks,
    /// which have no curl in a distroless image).
    Healthcheck {
        #[arg(long, default_value = "127.0.0.1:8080")]
        addr: SocketAddr,
    },
}

fn read_issue(path: &PathBuf) -> Result<epub_gazette::Issue> {
    let text = if path.as_os_str() == "-" {
        std::io::read_to_string(std::io::stdin())?
    } else {
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?
    };
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn explain(e: epub_gazette::RenderError) -> anyhow::Error {
    match e {
        epub_gazette::RenderError::Invalid(errs) => {
            anyhow::anyhow!("the issue is invalid:\n  - {}", errs.join("\n  - "))
        }
        other => other.into(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info")),
        )
        .init();

    match Cli::parse().command {
        Command::Serve { bind, token } => {
            let token = token.filter(|t| !t.is_empty());
            if token.is_none() {
                tracing::warn!("RENDER_TOKEN is not set; the API accepts unauthenticated requests");
            }
            let app = epub_gazette::server::router(epub_gazette::server::Config { token });
            let listener = tokio::net::TcpListener::bind(bind)
                .await
                .with_context(|| format!("binding {bind}"))?;
            tracing::info!(%bind, version = env!("CARGO_PKG_VERSION"), "epub-gazette listening");
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown())
                .await?;
        }
        Command::Render {
            input,
            output,
            cover,
        } => {
            let issue = read_issue(&input)?;
            if let Some(cover_path) = &cover {
                let jpg = epub_gazette::render_cover(issue.clone()).map_err(explain)?;
                std::fs::write(cover_path, jpg)?;
            }
            let book = epub_gazette::render(issue).map_err(explain)?;
            let out = output.unwrap_or_else(|| PathBuf::from(&book.file_name));
            std::fs::write(&out, &book.epub)
                .with_context(|| format!("writing {}", out.display()))?;
            println!("{} ({} KiB)", out.display(), book.epub.len() / 1024);
        }
        Command::Validate { input } => {
            let mut issue = read_issue(&input)?;
            issue.sanitize();
            if let Err(errs) = issue.validate() {
                bail!("the issue is invalid:\n  - {}", errs.join("\n  - "));
            }
            println!("{} is valid", input.display());
        }
        Command::Schema => println!("{}", serde_json::to_string_pretty(&epub_gazette::schema())?),
        Command::Healthcheck { addr } => healthcheck(addr)?,
        Command::Themes => {
            for t in epub_gazette::theme::all() {
                println!("{:<16} {} – {}", t.id, t.name, t.description);
            }
        }
    }
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { () = ctrl_c => {}, () = term => {} }
    tracing::info!("shutting down");
}

fn healthcheck(addr: SocketAddr) -> Result<()> {
    use std::io::{Read, Write};
    use std::time::Duration;
    let mut stream = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
    let mut resp = String::new();
    stream.read_to_string(&mut resp)?;
    if resp.starts_with("HTTP/1.1 200") {
        Ok(())
    } else {
        bail!(
            "unhealthy: {}",
            resp.lines().next().unwrap_or("no response")
        )
    }
}
