use crate::common::ServerCfg;
use crate::ldap_server;
use crate::web_server;

use tokio::net::TcpListener;
use std::net;
use std::str::FromStr;

async fn acceptor(listener: Box<TcpListener>, cfg : ServerCfg) {
    loop {
        match listener.accept().await {
            Ok((stream, _paddr)) => {
                let mut buf = [0; 3];
                let _len = stream.peek(&mut buf).await.expect("peek failed");
                let peek = String::from_utf8_lossy(&buf);
                if peek == "GET" {
                    println!("HTTP GET request on port {} from {}", &cfg.port, _paddr);
                    web_server::process_http(stream, cfg.clone()).await;
                }else{
                    println!("Not a HTTP GET request sending to LDAP on port {} from {}", &cfg.port, _paddr);
                    tokio::spawn(ldap_server::handle_client(stream, cfg.clone()));
                }
            }
            Err(_e) => {
                //pass
            }
        }
    }
}


pub async fn start_multiplexed_server(cfg : ServerCfg){
    let addr = net::SocketAddr::from_str(&format!("0.0.0.0:{}", &cfg.port)).unwrap();
    let listener = Box::new(TcpListener::bind(&addr).await.unwrap());
    // Initiate the acceptor task.
    println!("starting multiplexed server on : 0.0.0.0:{} ...", &cfg.port);
    tokio::spawn(acceptor(listener, cfg.clone()));
}
