use opentelemetry::global;
use tracing::instrument;

#[tokio::main]
async fn main() {
    // First, create a OTLP exporter builder. Configure it as you need.
    let otlp_exporter = opentelemetry_otlp::new_exporter().tonic();
    // Then pass it into pipeline builder
    let _ = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .install_simple()
        .unwrap();
    let _tracer = global::tracer("my_tracer");
    // // 初始化 OTLP 导出器，指向 Jaeger 或 OpenTelemetry Collector 的 OTLP 接口
    // let tracer = opentelemetry_otlp::new_exporter()
    //     .tonic() // 使用 gRPC 传输
    //     .with_endpoint("http://localhost:4317") // 指定 OTLP 接收器的地址
    //     .install_simple()
    //     .expect("Failed to install OTLP exporter");

    // 创建 OpenTelemetry 层
    // let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // 创建全局 tracing Subscriber
    // let subscriber = Registry::default()
    //     .with(EnvFilter::from_default_env())
    //     .with(telemetry);

    // 设置全局 Subscriber
    // tracing::subscriber::set_global_default(subscriber)
    //     .expect("Failed to set global tracing subscriber");
    //
    // 调用 instrument 标注的函数
    my_function();

    // 关闭 TracerProvider，确保所有的 Span 都已导出
    opentelemetry::global::shutdown_tracer_provider();
}

#[instrument(name = "tls_connector_new")]
fn my_function() {
    // 函数实现
    println!("Function is running");
}
