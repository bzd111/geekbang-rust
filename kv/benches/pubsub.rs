use std::time::Duration;

use anyhow::Result;
use criterion::{async_executor::FuturesExecutor, criterion_group, criterion_main, Criterion};
use futures::StreamExt;

use kv::{
    start_client_with_config, start_server_with_config, ClientConfig, CommandRequest, ServerConfig,
    StorageConfig, YamuxCtrl,
};

use rand::prelude::SliceRandom;
use tokio::net::TcpStream;
use tokio::runtime::Builder;
use tokio::time::sleep;
use tokio_rustls::client::TlsStream;

use opentelemetry::{trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::Config;
use opentelemetry_sdk::{runtime, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing::{info, instrument, span};
use tracing_subscriber::prelude::*;
use tracing_subscriber::Registry;

async fn start_server() -> Result<()> {
    let addr = "127.0.0.1:9999";
    let mut config: ServerConfig = toml::from_str(include_str!("../fixtures/server.conf"))?;
    config.general.addr = addr.into();
    config.storage = StorageConfig::MemTable;
    tokio::spawn(async move {
        start_server_with_config(&config).await.unwrap();
    });
    sleep(Duration::from_secs(2)).await;

    info!("Exiting start_server function");

    Ok(())
}

async fn connect() -> Result<YamuxCtrl<TlsStream<TcpStream>>> {
    let addr = "127.0.0.1:9999";
    let mut config: ClientConfig = toml::from_str(include_str!("../fixtures/client.conf"))?;
    config.general.addr = addr.into();
    start_client_with_config(&config).await
}

async fn start_subscribers(topic: &'static str) -> Result<()> {
    let mut ctrl = connect().await?;
    let stream = ctrl.open_stream().await?;
    info!("C(subscriber): stream opened");
    let cmd = CommandRequest::new_subscribe(topic.to_string());
    tokio::spawn(async move {
        let mut stream = stream.execute_streaming(&cmd).await.unwrap();
        while let Some(Ok(data)) = stream.next().await {
            drop(data);
        }
    });

    Ok(())
}

async fn start_publishers(topic: &'static str, values: &[&'static str]) -> Result<()> {
    let mut rng = rand::thread_rng();
    let v = values.choose(&mut rng).unwrap();
    let mut ctrl = connect().await.unwrap();
    let mut stream = ctrl.open_stream().await.unwrap();
    info!("C(publisher): stream opened");
    let cmd = CommandRequest::new_publish(topic.to_string(), vec![(*v).into()]);
    stream.execute_unary(&cmd).await.unwrap();

    Ok(())
}

async fn setup_opentelemetry() {
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                SERVICE_NAME,
                "kv-service1",
            )])),
        )
        .install_batch(runtime::Tokio)
        .unwrap();
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer_provider.tracer("tracer"));
    Registry::default()
        .with(tracing_subscriber::EnvFilter::new("INFO"))
        .with(telemetry)
        .init();
    // let root = span!(tracing::Level::INFO, "app_start", work_units = 2);
    // let _enter = root.enter();
}

async fn prepare_server_and_subscribers(topic: &'static str) {
    eprint!("preparing server and subscribers...");
    if let Err(e) = start_server().await {
        eprintln!("Failed to start server: {:?}", e);
        return;
    }
    for i in 0..100 {
        if let Err(e) = start_subscribers(topic).await {
            eprintln!("Failed to start subscriber {}: {:?}", i, e);
            return;
        }
        eprint!(".");
    }
    eprintln!("Done!");
}

fn pubsub(c: &mut Criterion) {
    let rt = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("pubsub")
        .enable_all()
        .build()
        .unwrap();

    let values = &["hello", "world", "foo", "bar"];
    let topic = "lobby";

    rt.block_on(async {
        let root = span!(tracing::Level::INFO, "app_start", work_units = 2);
        let _enter = root.enter();

        // 设置 OpenTelemetry
        setup_opentelemetry().await;

        // 准备服务器和订阅者
        prepare_server_and_subscribers(topic).await;

        // 执行基准测试
        c.bench_function("publishing", |b| {
            b.to_async(FuturesExecutor)
                .iter(|| async { start_publishers(topic, values).await })
        });
    });

    // rt.block_on(async {
    //     let tracer_provider =
    //         opentelemetry_otlp::new_pipeline()
    //             .tracing()
    //             .with_exporter(
    //                 opentelemetry_otlp::new_exporter()
    //                     .tonic()
    //                     .with_endpoint("http://localhost:4317"),
    //             )
    //             .with_trace_config(Config::default().with_resource(Resource::new(vec![
    //                 KeyValue::new(SERVICE_NAME, "kv-service1"),
    //             ])))
    //             .install_batch(runtime::Tokio)
    //             // .install_simple()
    //             .unwrap();
    //     opentelemetry::global::set_tracer_provider(tracer_provider.clone());
    //     let telemetry =
    //         tracing_opentelemetry::layer().with_tracer(tracer_provider.tracer("tracer"));
    //     Registry::default()
    //         .with(tracing_subscriber::EnvFilter::new("INFO"))
    //         .with(telemetry)
    //         .init();
    // });
    // 准备服务器和订阅者
    // let rt = tokio::runtime::Builder::new_current_thread()
    //     .enable_io()
    //     .enable_time()
    //     .build()
    //     .unwrap();
    //

    // rt.block_on(async {
    //     // eprint!("preparing server and subscribers...");
    //     // start_server().await.unwrap();
    //     eprint!("preparing server and subscribers...");
    //     if let Err(e) = start_server().await {
    //         eprintln!("Failed to start server: {:?}", e);
    //         return;
    //     }
    //     for i in 0..100 {
    //         if let Err(e) = start_subscribers(topic).await {
    //             eprintln!("Failed to start subscriber {}: {:?}", i, e);
    //             return;
    //         }
    //         eprint!(".");
    //     }
    //     eprintln!("Done!");
    //     for _ in 0..100 {
    //         start_subscribers(topic).await.unwrap();
    //         eprint!(".");
    //     }
    //     eprintln!("Done!");
    // });

    // c.bench_function("publishing", |b| {
    //     b.to_async(FuturesExecutor)
    //         .iter(|| async { start_publishers(topic, values).await })
    // });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(15));
    targets = pubsub
}
criterion_main!(benches);

#[instrument]
async fn do_work() {
    // 模拟一些工作
    info!("Starting work");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    info!("Work step 1111");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    info!("Work step 2222");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    info!("Finished work");
}
