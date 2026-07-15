use anyhow::Result;
use clap::Parser;
use tairitsu_e2e::{run_all_tests, ShirabeDriver};
use tracing::info;

#[derive(Parser)]
#[command(name = "tairitsu-e2e")]
#[command(about = "E2E testing framework for Tairitsu (CDP-based via Shirabe)")]
struct Args {
    #[arg(short, long, default_value = "http://localhost:8080")]
    website_url: String,

    #[arg(short, long, default_value = "9222")]
    debug_port: u16,

    #[arg(short, long, default_value = "./screenshots")]
    screenshots_dir: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt().with_env_filter("info").init();

    std::env::set_var("WEBSITE_BASE_URL", &args.website_url);
    std::env::set_var("E2E_SCREENSHOTS_DIR", &args.screenshots_dir);

    info!("Starting E2E tests (Shirabe CDP driver)...");
    info!("Website URL: {}", args.website_url);

    let driver = ShirabeDriver::connect(&args.website_url, args.debug_port).await?;

    let results = run_all_tests(&driver).await?;

    let passed = results
        .iter()
        .filter(|r| matches!(r.status, tairitsu_e2e::tests::TestStatus::Success))
        .count();
    let total = results.len();

    info!("\nTest Summary: {}/{} passed", passed, total);

    driver.quit().await?;

    if passed < total {
        std::process::exit(1);
    }

    Ok(())
}
