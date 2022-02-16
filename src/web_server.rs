use warp::Filter;
use std::future::Future;
use crate::common::ServerCfg;


pub fn start_web_server(cfg : ServerCfg)->impl Future<Output=()>{
    let files = warp::fs::dir(cfg.web_root).with(warp::log("all"));
    println!("Starting http server on 0.0.0.0:{}", &cfg.http_port);
    let w = warp::serve(files).run(([0, 0, 0, 0], cfg.http_port));
    w
}
