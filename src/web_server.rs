use crate::common::ServerCfg;


use tokio::net::TcpStream;

//use hyper::{Body, Request, Response, Server};
//use hyper::service::{make_service_fn, service_fn};
use hyper::server::conn::Http;


//let make_svc = make_service_fn(|socket: &AddrStream| {
//    async move {
//        Ok::<_, Infallible>(service_fn(move |_: Request<Body>| async move {
//            Ok::<_, Infallible>(
//                Response::new(Body::from(format!("Hello, {}!", remote_addr)))
//            )
//        }))
//    }
//});

//async fn process_http(s : TcpStream, cfg : ServerCfg){
//    let dir = cfg.web_root.clone();
//    tokio::task::spawn(async move {
//        if let Err(http_err) = Http::new()
//            .http1_only(true)
//                .serve_connection(s, service_fn(move |req : Request<Body>| async move {
//                    let static_ = hyper_staticfile::Static::new(dir);
//                    println!("{}", &req.uri());
//                    static_.serve(req).await
//                }))
//        .await {
//            eprintln!("Error while serving HTTP connection: {}", http_err);
//        }
//    });
//}
pub async fn process_http(s : TcpStream, cfg : ServerCfg){
    println!("Entering process http on port {}", &cfg.port);
    let static_ = hyper_staticfile::Static::new(cfg.web_root);
    let r = tokio::task::spawn(async move {
        if let Err(http_err) = Http::new()
            .http1_only(true)
                .serve_connection(s, static_)
                .await {
                    eprintln!("Error while serving HTTP connection: {}", http_err);
        }
    }).await;
    match r {
        Ok(()) => (),
        Err(e) => println!("Error with http request {}", e),
    };
}

