use crate::common::{ServerCfg, BuildCmdCfg};
use crate::build_java;

use tokio::net::TcpStream;
use hyper::server::conn::Http;

use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, StatusCode};
use std::io::Error as IoError;
use std::error::Error;
use core::fmt::Display;
use serde::Deserialize;

use serde_json;

#[derive(Debug)]
enum WSError{
    IoError(IoError),
    ReadBody,
    Json,
    Cache,
}
impl Display for WSError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WSError::IoError(io) => write!(f, "{}", io),
            WSError::ReadBody => write!(f, "Error reading request body"),
            WSError::Json => write!(f, "Error Converting request body to JSON"),
            WSError::Cache => write!(f, "Error createing or saving cached class"),
        }
    }
}

impl std::error::Error for WSError{}

impl From<std::io::Error> for WSError{
    fn from(err : std::io::Error) -> WSError{
        WSError::IoError(err)
    }
}

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
    pub async fn serve(self, req: Request<Body>) -> Result<Response<Body>, WSError> {
        println!("We have  request {} : {}", req.method(), req.uri().path());
        match (req.method(), req.uri().path()) {
            // Serve some instructions at /
            (&Method::GET, "/") => Ok(Response::new(Body::from(
                        "Hello there, try /echo",
            ))),
           (&Method::GET, "/echo") => Ok(Response::new(Body::from(
                        "try echo with a post",
            ))),
           (&Method::POST, "/build_cmd") => self.build_cmd(req).await, 
           //(&Method::GET, p) if p.starts_with("/MM_")  => self.build_mm(req).await, 
            
            _ => {  
                let path = req.uri().path()[1..].to_string();
                match self.cfg.class_cache.get_class(&path) {
                    Some(the_class) => {
                        println!("we have a cache match");
                        Ok(Response::new(Body::from(the_class)))
                    },
                    None => {
                        if path.starts_with("MM_") {
                            self.process_mm(req).await
                        }else{
                            match self.file_server.serve(req).await {
                                Ok(r) => Ok(r),
                                Err(ioe) => Err(ioe.into())
                            }
                        }
                    }
                }
            }
        }
    }

    async fn not_found(&self) -> Result<Response<Body>, WSError>{
        let mut not_found = Response::default();
        *not_found.status_mut() = StatusCode::NOT_FOUND;
        Ok(not_found)
    }

    async fn body_as_string(&self, req: Request<Body>) -> Result<String, WSError>{
        if let Ok(raw_body) =  hyper::body::to_bytes(req.into_body()).await {
            if let Ok(s) = String::from_utf8(raw_body.to_vec()){
                return Ok(s);
            }
        }
        Err(WSError::ReadBody)
    }

    async fn process_mm(&self, req : Request<Body>) -> Result<Response<Body>, WSError>{
        let class_name = &req.uri().path()[1..]; 
        let parts : Vec<&str> = class_name.split("_").collect();
        if parts.len() < 3 {
            println!("Error : unable to create MM class. {} is invalid.", req.uri().path());
            return self.not_found().await;
        }
        println!("we are processing a MM");
        let last = parts.len()-1;
        let host = &parts[1..last].join(".");
        let port = &parts[last];
        if let Ok(the_class) = build_java::build_mm_class(&class_name, &host, &port) { 
            self.cfg.class_cache.set_class(class_name.to_string(), the_class.clone());
            Ok(Response::new(Body::from(the_class)))
        } else {
            return Err(WSError::Cache);
        }
    }

    async fn build_cmd(&self, req: Request<Body>) -> Result<Response<Body>, WSError> {
        let body = self.body_as_string(req).await?;

        if let Ok(build_cmd) = serde_json::from_str::<BuildCmdCfg>(&body) {
            if let Ok(the_class) = build_java::build_cmd_class(build_cmd.clone()){
                let class_name = format!("{}.class", build_cmd.class_name);
                self.cfg.class_cache.set_class(class_name, the_class);
                return Ok(Response::new(Body::from(format!("Created new class for {:?} -- \n\n\n", build_cmd))));
            }else{
                return Err(WSError::Cache);
            }

        }
        return Ok(Response::new(Body::from("ERROR READING POST DATA  for build_cmd")));
    }
}

pub async fn process_http(s : TcpStream, cfg : ServerCfg){
    //let hw = WebService::new(cfg.clone());
    let service = |r| WebService::new(cfg.clone()).serve(r);

    if let Err(http_err) = Http::new()
        .http1_only(true)
        .serve_connection(s, service_fn(&service))
        .await {
            eprintln!("Error while serving HTTP connection: {}", http_err);
        }
}

