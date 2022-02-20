
#[derive(Debug, Clone)]
pub struct RunServerCfg{
    pub addr : String,
    pub web_root : String,
    pub ports : Vec<u16>,
    pub ports_file_name : Option<String>,
    pub failed_file_name : Option<String>,
}


#[derive(Debug, Clone)]
pub struct ServerCfg{
    pub addr : String,
    pub port : u16,
    pub web_root : String,
}

#[derive(Debug, Clone)]
pub struct BuildCmdCfg{
    pub build_path : String,
    pub class_name : String,
    pub l_cmd : String,
    pub w_cmd : String,
}

