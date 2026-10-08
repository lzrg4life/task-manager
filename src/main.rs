use std::{io::IsTerminal, sync::Arc};

use clap::{Parser, Subcommand};
use task_manager::{
    AppState, app,
    config::{self, Config},
    db,
};

#[derive(Parser)]
#[command(about = "Task manager web API and static client")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the web server (default)
    Serve,
    /// Create/migrate the database and create the first admin user, then exit
    InitDb {
        /// Email of the first admin user
        #[arg(long)]
        admin_email: String,
    },
}

fn admin_password() -> Result<String, String> {
    if let Ok(p) = std::env::var("ADMIN_PASSWORD") {
        return Ok(p);
    }
    if !std::io::stdin().is_terminal() {
        return Err("no terminal available; set ADMIN_PASSWORD".into());
    }
    let first = rpassword::prompt_password("Admin password: ").map_err(|e| e.to_string())?;
    let second = rpassword::prompt_password("Confirm password: ").map_err(|e| e.to_string())?;
    if first != second {
        return Err("passwords do not match".into());
    }
    Ok(first)
}

async fn run() -> Result<(), String> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve) {
        Command::InitDb { admin_email } => {
            let password = admin_password()?;
            let pool = db::connect(&config::database_url_from_env())
                .await
                .map_err(|e| e.to_string())?;
            db::migrate(&pool).await.map_err(|e| e.to_string())?;
            db::create_first_admin(&pool, &admin_email, &password)
                .await
                .map_err(|e| e.to_string())?;
            println!("Database initialized; admin user {admin_email} created.");
            Ok(())
        }
        Command::Serve => {
            let config = Config::from_env()?;
            let pool = db::connect(&config.database_url)
                .await
                .map_err(|e| e.to_string())?;
            db::migrate(&pool).await.map_err(|e| e.to_string())?;
            let listener = tokio::net::TcpListener::bind(&config.bind_addr)
                .await
                .map_err(|e| e.to_string())?;
            tracing::info!("listening on http://{}", config.bind_addr);
            let state = AppState {
                pool,
                config: Arc::new(config),
            };
            axum::serve(listener, app(state))
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await
                .map_err(|e| e.to_string())
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
