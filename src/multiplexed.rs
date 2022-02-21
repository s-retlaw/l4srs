use crate::common::{ServerCfg, RunServerCfg};
use crate::ldap_server;
use crate::web_server;
use crate::tcp_proxy;

use tokio::net::TcpListener;

use std::net;
use std::str::FromStr;
use std::fs::File;
use std::io::Write;

fn is_ldap(buf : &[u8]) -> bool{
    return buf == [48, 12, 2, 1, 1];
}

fn is_http(buf : &[u8]) -> bool{
    let peek = String::from_utf8_lossy(&buf).to_string();

    match peek.as_str() {
        "GET /" => true,
        "POST " => true,
        "HEAD " => true,
        "PATCH" => true,
        "OPTIO" => true,
        "PUT /" => true,
        _ => false,
    }
}

async fn acceptor(listener: Box<TcpListener>, cfg : ServerCfg) {
    loop {
        match listener.accept().await {
            Ok((stream, _paddr)) => {
                let mut buf = [0; 5];
                let _len = stream.peek(&mut buf).await.expect("peek failed");
//                println!("the bytes are {:?}", buf);
                if is_ldap(&buf) {
                    println!("LDAP request on port {} from {}", &cfg.port, _paddr);
                    tokio::spawn(ldap_server::handle_client(stream, cfg.clone()));
                } else if is_http(&buf){
                    println!("HTTP request on port {} from {}", &cfg.port, _paddr);
                    web_server::process_http(stream, cfg.clone()).await;
                } else {
                    match &cfg.proxxy_addr {
                        Some(addr) =>{
                            println!("Attempting to connect to proxy {}", &addr);
                            tokio::spawn(tcp_proxy::proxy(stream, addr.clone(), cfg.port));
                        }
                        _ => {
                            println!("Not an HTTP or LDAP request on port {} from {} and no proxy configured", &cfg.port, _paddr);
                        },
                    }
                }
            }
            Err(e) => {
                println!("Error with listener on port {} : {}", &cfg.port, e);
            }
        }
    }
}

pub async fn run_multiplexed_servers(rsc : RunServerCfg) {
    let mut tasks = Vec::new(); 
    let mut opened = Vec::new();
    let mut failed = Vec::new();
    for port in rsc.ports {
        let cfg = ServerCfg{
            port,
            addr : rsc.addr.clone(),
            web_root : rsc.web_root.clone(),
            proxxy_addr : rsc.proxy_addr.clone(),
        };
        let addr = net::SocketAddr::from_str(&format!("0.0.0.0:{}", &cfg.port)).unwrap();
        match TcpListener::bind(&addr).await {
            Ok(l) => {
                opened.push(cfg.port.to_string());
                tasks.push(tokio::spawn(acceptor(Box::new(l), cfg.clone())));
            },
            Err(e) => {
                failed.push(cfg.port.to_string());
                println!("Error opening port {} : {}", port, e);
            },
        }
    }
    println!("==================");
    println!("We failed to start on these ports ({})", failed.join(","));
    println!("==================");
    println!("We started servers on these ports ({})", opened.join(","));
    println!("==================");
    try_write(rsc.ports_file_name, &opened);
    try_write(rsc.failed_file_name, &failed);
    if !opened.is_empty() { futures::future::join_all(tasks).await; }
}

fn try_write(file_name : Option<String>, ports : &Vec<String>) {
    match file_name {
        Some(f) => {
            let mut output = File::create(&f).expect(&format!("Unable to open file {}", &f));
            let file_data = ports.join("\n");
            write!(output, "{}", file_data).expect(&format!("Error writing to {}", &f));
        },
        None => {}
    }
}
