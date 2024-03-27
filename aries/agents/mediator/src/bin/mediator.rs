use aries_vcx_core::wallet::indy::IndySdkWallet;
use log::info;
use mediator::aries_agent::AgentBuilder;

#[tokio::main]
async fn main() {
    load_dot_env();
    setup_logging();
    info!("Starting up mediator! ⚙️⚙️");
    let mut endpoint_root = std::env::var("ENDPOINT_ROOT").unwrap_or("127.0.0.1:8005".into());
    let server_host = &endpoint_root.clone();

    if !endpoint_root.starts_with("http://") {
        endpoint_root = format!("http://{}", endpoint_root);
    }

    let service_endpoint = std::env::var("BASE_DOMAIN").unwrap_or(endpoint_root.clone());
    let format = format!("{}/didcomm", service_endpoint).parse().unwrap();

    info!("Mediator endpoint root address {}", format);

    let mut agent = AgentBuilder::<IndySdkWallet>::new_demo_agent()
        .await
        .unwrap();
    agent.init_service(vec![], format).await.unwrap();
    let app_router = mediator::http_routes::build_router(agent).await;
    info!("Starting server");
    axum::Server::bind(
        &server_host
            .parse()
            .expect("Pass an address to listen on like IP:PORT"),
    )
    .serve(app_router.into_make_service())
    .await
    .unwrap();
}

fn setup_logging() {
    let env = env_logger::Env::default().default_filter_or("info");
    env_logger::init_from_env(env);
}

fn load_dot_env() {
    let _ = dotenvy::dotenv();
}
