use nfsserve::demofs::DemoFS;
use nfsserve::tcp::*;

const HOSTPORT: u32 = 11111;
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();
    let listener = NFSTcpListener::bind(&format!("127.0.0.1:{HOSTPORT}"), DemoFS::default())
        .await
        .unwrap();
    listener.handle_forever().await.unwrap();
}
// Test with
// mount -t nfs -o nolocks,vers=3,tcp,port=12000,mountport=12000,soft 127.0.0.1:/ mnt/
