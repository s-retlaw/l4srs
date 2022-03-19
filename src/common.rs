use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ClassCache{
    cache : Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl ClassCache{
    pub fn new() -> ClassCache{
        ClassCache{
            cache : Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_class(&self, class_name : String)->Option<Vec<u8>>{
        let cache = self.cache.lock().unwrap();
        match cache.get(&class_name) {
            Some(c) => Some(c.clone()),
            None => None
        }
    }

    pub fn set_class(&self, class_name : String, class_data : Vec<u8>){
        let mut cache = self.cache.lock().unwrap();
        cache.insert(class_name, class_data);
    }
}

#[derive(Debug, Clone)]
pub struct RunServerCfg{
    pub addr : String,
    pub web_root : String,
    pub ports : Vec<u16>,
    pub ports_file_name : Option<String>,
    pub failed_file_name : Option<String>,
    pub proxy_addr : Option<String>,
    pub class_cache : ClassCache,
}


#[derive(Debug, Clone)]
pub struct ServerCfg{
    pub addr : String,
    pub port : u16,
    pub web_root : String,
    pub proxxy_addr : Option<String>,
    pub class_cache : ClassCache,
}

#[derive(Debug, Clone)]
pub struct BuildCmdCfg{
    pub build_path : String,
    pub class_name : String,
    pub l_cmd : String,
    pub w_cmd : String,
}

