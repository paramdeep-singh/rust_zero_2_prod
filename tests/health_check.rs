use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::configuration;
use zero2prod::startup;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &test_app.address))
        .send()
        .await
        .expect("Failed to execute request!");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn test_200_for_valid_subscription_post_data() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=param&email=param%40gmail.com";
    let response = client
        .post(&format!("{}/subscription", &test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request!");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.name, "param");
    assert_eq!(saved.email, "param@gmail.com");
}

#[tokio::test]
async fn test_400_when_subscription_request_lacks_required_data() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("email=param%40gmail.com", "name missing"),
        ("name=param", "email missing"),
        ("", "both name, email missing"),
    ];

    for (bad_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscription", &test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(bad_body)
            .send()
            .await
            .expect("Failed to execute request!");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The subscription API did not return a 400 Bad Request response for the payload: {}",
            error_message,
        );
    }
}

// launch app in the background
async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let configuration = configuration::get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(
        &configuration.database.connection_string()
    )
    .await
    .expect("Failed to connect to Postgres.");

    let server = startup::run(listener, connection_pool.clone())
        .expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection_pool,
    }
}
