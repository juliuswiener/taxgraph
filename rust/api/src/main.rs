//! `taxgraph-api [port]` — bindet 127.0.0.1 (Auflage B, nie 0.0.0.0) wie `server.py:main`;
//! `taxgraph-api --openapi` druckt das OpenAPI-Dokument und endet.
use std::io::Write;

use api::konfig::{lade_env_dateien, Konfig};
use api::{app, Zustand};
use utoipa::OpenApi;

const HOST: &str = "127.0.0.1";

async fn beendet() {
    use tokio::signal::unix::{signal, SignalKind};
    let (Ok(mut term), Ok(mut int)) = (
        signal(SignalKind::terminate()),
        signal(SignalKind::interrupt()),
    ) else {
        return std::future::pending().await;
    };
    let name = tokio::select! { _ = term.recv() => "SIGTERM", _ = int.recv() => "SIGINT" };
    eprintln!("\n{name} empfangen, fahre herunter...");
}

async fn dienen(port: u16, zustand: Zustand) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind((HOST, port)).await?;
    println!(
        "TaxGraph-Haut auf http://{HOST}:{}  (Ctrl-C zum Beenden)",
        listener.local_addr()?.port()
    );
    std::io::stdout().flush()?;
    axum::serve(
        listener,
        axum::ServiceExt::<axum::extract::Request>::into_make_service(app(zustand)),
    )
    .with_graceful_shutdown(beendet())
    .await?;
    println!("Server heruntergefahren.");
    Ok(())
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--openapi") {
        match api::openapi::ApiDoc::openapi().to_pretty_json() {
            Ok(text) => println!("{text}"),
            Err(e) => {
                eprintln!("OpenAPI nicht serialisierbar: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
        return std::process::ExitCode::SUCCESS;
    }
    // Wie `server.main`: gitignorierte Env-Dateien zuerst, das Prozess-Env gewinnt.
    // Vor dem Runtime-Start, damit `set_var` keinem anderen Thread begegnet.
    lade_env_dateien(&Konfig::aus_env().wurzel);
    let port = match args.get(1).map(|a| a.parse::<u16>()) {
        None => 8000,
        Some(Ok(p)) => p,
        Some(Err(e)) => {
            eprintln!("Port: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let zustand = match Zustand::aus_env() {
        Ok(z) => z,
        Err(e) => {
            eprintln!("Start: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let laufzeit = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Runtime: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match laufzeit.block_on(dienen(port, zustand)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Server: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
