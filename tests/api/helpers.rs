use once_cell::sync::Lazy;
use sqlx::{PgPool, Pool, Postgres};
use wiremock::MockServer;
use zero2prod::startup::Application;
use zero2prod::{
    configuration::Settings,
    telemetry::{get_subscriber, init_subscriber},
};

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // We cannot assign the output of `get_subscriber` to a variable based on the value of `TEST_LOG`
    // because the sink is part of the type returned by `get_subscriber`, therefore they are not the
    // same type. We could work around it, but this is the most straight-forward way of moving forward.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

pub async fn spawn_app(connection_pool: Pool<Postgres>) -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let mut configuration = Settings::new().expect("Failed to read configuration.");
    configuration.application.port = 0;
    configuration.email_client.base_url = email_server.uri();

    let application = Application::build_test(&configuration, connection_pool.clone())
        .await
        .expect("Failed to build application.");
    let address = format!("http://127.0.0.1:{}", application.port());

    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        db_pool: connection_pool,
        email_server,
    }
}

// async fn configure_database(config: &DatabaseSettings) -> PgPool {
//     // Create database
//     let mut connection = PgConnection::connect_with(&config.without_db())
//         .await
//         .expect("Failed to connect to Postgres");

//     connection
//         .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
//         .await
//         .expect("Failed to create database.");
//     // Migrate database
//     let connection_pool = PgPool::connect_with(config.with_db())
//         .await
//         .expect("Failed to connect to Postgres.");
//     sqlx::migrate!("./migrations")
//         .run(&connection_pool)
//         .await
//         .expect("Failed to migrate the database");
//     connection_pool
// }
