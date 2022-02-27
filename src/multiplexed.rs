use crate::common::{ServerCfg, RunServerCfg};
use crate::ldap_server;
use crate::web_server;
use crate::tcp_proxy;

use tokio::time::{timeout, Duration};
use tokio::net::TcpListener;

use std::net;
use std::str::FromStr;
use std::fs::File;
use std::io::Write;

fn is_ldap(buf : &[u8], num_bytes : usize) -> bool{
    if num_bytes < 4 {return false;}
    
    return buf == [48, 12, 2, 1]; //, 1];
}

fn is_http(buf : &[u8], num_bytes : usize) -> bool{
    if num_bytes < 4 {return false;}

    let peek = String::from_utf8_lossy(&buf).to_string();

    match peek.as_str() {
        "GET " => true,
        "POST" => true,
        "HEAD" => true,
        "PATC" => true,
        "OPTI" => true,
        "PUT " => true,
        _ => false,
    }
}

async fn acceptor(listener: Box<TcpListener>, cfg : ServerCfg) {
    loop {
        match listener.accept().await {
            Ok((stream, _paddr)) => {
                //println!("New connection lets peek");
                let mut buf = [0; 4];
                let num_bytes : usize;

                let peek_result = timeout(Duration::from_millis(100), stream.peek(&mut buf)).await;
                num_bytes = match peek_result {
                    Ok(r) => { 
                        match r {
                            Ok(b) => b,
                            Err(_e) => {
                                eprintln!("Error peeking for port {} : {}", &cfg.port, _e);
                                continue;
                            },
                        }
                    },
                    Err(_e) => 0,
                };
                if is_ldap(&buf, num_bytes) {
                    println!("Port {} | From {} | LDAP", &cfg.port, _paddr);
                    tokio::spawn(ldap_server::handle_client(stream, cfg.clone()));
                } else if is_http(&buf, num_bytes){
                    println!("Port {} | From {} | HTTP", &cfg.port, _paddr);
                    tokio::spawn(web_server::process_http(stream, cfg.clone()));
                } else {
                    match &cfg.proxxy_addr {
                        Some(addr) =>{
                            println!("Port {} | From {} | Proxy {}", &cfg.port, _paddr, &addr);
                            tokio::spawn(tcp_proxy::proxy(stream, addr.clone(), cfg.port));
                        }
                        _ => {
                            eprintln!("Not an HTTP or LDAP request on port {} from {} and no proxy configured", &cfg.port, _paddr);
                        },
                    }
                }
            }
            Err(e) => {
                eprintln!("Error with listener on port {} : {}", &cfg.port, e);
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
                eprintln!("Error opening port {} : {}", port, e);
            },
        }
    }
    println!("==================");
    println!("We failed to start on these ports ({})", failed.join(","));
    println!("==================");
    println!("We started servers on these ports ({})", opened.join(","));
    println!("==================");
    println!("Running");
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
