pub mod shirabe_driver;
pub mod tests;

use anyhow::Result;

pub use shirabe_driver::ShirabeDriver;
pub use tests::{Test, TestResult, TestStatus};
use tracing::info;

pub async fn run_all_tests(driver: &ShirabeDriver) -> Result<Vec<TestResult>> {
    info!("Running all Tairitsu E2E tests via Shirabe CDP...\n");

    let mut results = vec![];

    macro_rules! run_suite {
        ($suite:expr, $label:literal) => {
            info!($label);
            match tests::Test::run_with_driver($suite, driver).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    eprintln!("{}: {}", $label, e);
                    results.push(TestResult::error($label, e.to_string().as_str()));
                }
            }
        };
    }

    run_suite!(&tests::BasicComponentsTests, "Basic Components");
    run_suite!(&tests::LifecycleTests, "Component Lifecycle");
    run_suite!(&tests::EventTests, "Event Handling");
    run_suite!(&tests::BuildTests, "Build Process");
    run_suite!(&tests::DoctorTests, "Doctor Command");
    run_suite!(&tests::ErrorHandlingTests, "Error Handling");
    run_suite!(&tests::NavigationTests, "Navigation");
    run_suite!(&tests::StateManagementTests, "State Management");
    run_suite!(&tests::FormValidationTests, "Form Validation");
    run_suite!(&tests::AsyncOperationsTests, "Async Operations");
    run_suite!(&tests::SvgSafetyTests, "SVG Safety");
    run_suite!(&tests::SsrTests, "SSR");
    run_suite!(&tests::StyleIntegrationTests, "Style Integration");

    info!("\n=== E2E Test Results ===");
    for result in &results {
        info!("{}: {}", result.component, result.message);
        match &result.status {
            TestStatus::Success => info!("  Status: PASSED"),
            TestStatus::Warning => info!("  Status: WARNING"),
            TestStatus::Failure => info!("  Status: FAILED"),
            TestStatus::Error(msg) => info!("  Status: ERROR - {}", msg),
        }
    }
    info!("=== End of Test Results ===\n");

    Ok(results)
}
