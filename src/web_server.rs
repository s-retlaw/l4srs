use crate::common::ServerCfg;

use tokio::net::TcpStream;
use hyper::server::conn::Http;

pub async fn process_http(s : TcpStream, cfg : ServerCfg){
    let static_ = hyper_staticfile::Static::new(cfg.web_root);
    if let Err(http_err) = Http::new()
        .http1_only(true)
        .serve_connection(s, static_)
        .await {
            eprintln!("Error while serving HTTP connection: {}", http_err);
        }
//    println!("Closing http connection from {}", &cfg.port);
}

