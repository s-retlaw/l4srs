
#[derive(Debug, Clone)]
pub struct ServerCfg{
    pub http_addr : String,
    pub http_port : u16,
    pub ldap_port : u16,
    pub web_root : String,
}

#[derive(Debug, Clone)]
pub struct BuildCmdCfg{
    pub build_path : String,
    pub class_name : String,
    pub l_cmd : String,
    pub w_cmd : String,
}


