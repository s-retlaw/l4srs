use crate::common::ServerCfg;

use tokio::net::TcpStream;
use hyper::server::conn::Http;

use hyper::service::Service;
use hyper::{Body, Method, Request, Response};
use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};
use std::io::Error as IoError;

//use std::collections::HashMap;
//use std::sync::{Arc, Mutex};

//let classes = Arc::new(Mutex::new(HashMap<String, Vec<u8>>));

#[derive(Clone)]
struct WebService{
    file_server : hyper_staticfile::Static,
    cfg : ServerCfg,
}

impl WebService{
    pub fn new(cfg: ServerCfg)->WebService{
        WebService{
            file_server : hyper_staticfile::Static::new(&cfg.web_root), 
            cfg,
        }
    }
    /// Serve a request.
    pub async fn serve<B>(self, req: Request<B>) -> Result<Response<Body>, IoError> {
        println!("We have  request {} : {}", req.method(), req.uri().path());
        match (req.method(), req.uri().path()) {
            // Serve some instructions at /
            (&Method::GET, "/") => Ok(Response::new(Body::from(
                        "Hello there, try /echo",
            ))),
           (&Method::GET, "/echo") => Ok(Response::new(Body::from(
                        "try echo with a post",
            ))),

            // Return the 404 Not Found for other routes.
            _ => {  
                let path = req.uri().path()[1..].to_string();
                match self.cfg.class_cache.get_class(path) {
                    Some(the_class) => {
                        println!("we have a cache match");
                        Ok(Response::new(Body::from(the_class)))
                    },
                    None => self.file_server.serve(req).await
                }
//                let mut not_found = Response::default();
//                *not_found.status_mut() = StatusCode::NOT_FOUND;
//                Ok(not_found)
            }
        }
    }
}

impl<B: Send + Sync + 'static> Service<Request<B>> for WebService{
    type Response = Response<Body>;
    type Error = IoError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        Box::pin(self.clone().serve(request))
    }
}


pub async fn process_http(s : TcpStream, cfg : ServerCfg){
    let hw = WebService::new(cfg.clone());

    if let Err(http_err) = Http::new()
        .http1_only(true)
        .serve_connection(s, hw)
        .await {
            eprintln!("Error while serving HTTP connection: {}", http_err);
        }
}

