use zero2prod::configuration::Settings;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let configuration = Settings::new().expect("Failed to read configuration.");
    let application = Application::build(&configuration, None).await?;
    application.run_until_stopped().await?;
    Ok(())
}
