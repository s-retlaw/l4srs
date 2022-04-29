use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use chrono::prelude::*;
use uuid::Uuid;

//#[derive(Debug, Clone)]
//pub struct ClassCache{
//    cache : Arc<Mutex<HashMap<String, Vec<u8>>>>,
//}
//
//impl ClassCache{
//    pub fn new() -> ClassCache{
//        ClassCache{
//            cache : Arc::new(Mutex::new(HashMap::new())),
//        }
//    }
//
//    pub fn get_class(&self, class_name : &String)->Option<Vec<u8>>{
//        let cache = self.cache.lock().unwrap();
//        match cache.get(class_name) {
//            Some(c) => Some(c.clone()),
//            None => None
//        }
//    }
//
//    pub fn set_class(&self, class_name : String, class_data : Vec<u8>){
//        let mut cache = self.cache.lock().unwrap();
//        cache.insert(class_name, class_data);
//    }
//}

#[derive(Debug, Clone)]
pub enum AccessType{
    LDAP,
    HTTP,
    PROXY,
    UNKNOWN
}


#[derive(Debug, Clone)]
pub struct Access{
    access_type : AccessType,
    host : String,
    port : u16,
    time : DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Caches{
    class : Arc<Mutex<HashMap<String, Vec<u8>>>>,
    access : Arc<Mutex<Vec<Access>>>,
    id_access : Arc<Mutex<HashMap<String, Vec<Access>>>>,
    //ids : Arc<Mutex<Uuid>>,
    open_ports : Arc<Mutex<Vec<u16>>>,
    failed_ports : Arc<Mutex<Vec<u16>>>,
}
impl Caches{
    pub fn new() -> Caches{
        Caches{
            class : Arc::new(Mutex::new(HashMap::new())),
            access : Arc::new(Mutex::new(Vec::new())),
            id_access : Arc::new(Mutex::new(HashMap::new())),
            //ids : Arc::new(Mutex::new(Uuid::new_v4())),
            open_ports : Arc::new(Mutex::new(Vec::new())),
            failed_ports : Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn get_class(&self, class_name : &String)->Option<Vec<u8>>{
        let cache = self.class.lock().unwrap();
        match cache.get(class_name) {
            Some(c) => Some(c.clone()),
            None => None
        }
    }

    pub fn set_class(&self, class_name : String, class_data : Vec<u8>){
        let mut cache = self.class.lock().unwrap();
        cache.insert(class_name, class_data);
    }

    pub fn get_access(&self)->Vec<Access>{
        let cache = self.access.lock().unwrap();
        cache.clone()
    }

    pub fn add_access(&self, access : Access) {
        let mut cache  = self.access.lock().unwrap();
        cache.push(access);
    }
    
    pub fn get_access_for_id(&self, id : &String)->Option<Vec<Access>>{
        let cache = self.id_access.lock().unwrap();
        match cache.get(id) {
            Some(access) => Some(access.clone()),
            None => None
        }
    }

    pub fn add_access_for_id(&self, id : &String, access : Access){
        let mut cache = self.id_access.lock().unwrap();
        match cache.get_mut(id) {
            Some(access_list) => access_list.push(access),
            None => {
                cache.insert(id.clone(), vec![access]);
                ()
            }
        }
    }

    pub fn get_open_ports(&self)->Vec<u16>{
        let cache = self.open_ports.lock().unwrap();
        cache.clone()
    }

    pub fn add_open_port(&self, port : u16) {
        let mut cache  = self.open_ports.lock().unwrap();
        cache.push(port);
    }

    pub fn get_failed_ports(&self)->Vec<u16>{
        let cache = self.failed_ports.lock().unwrap();
        cache.clone()
    }

    pub fn add_failed_port(&self, port : u16) {
        let mut cache  = self.failed_ports.lock().unwrap();
        cache.push(port);
    }

    //does this really belong here????
    ////Shoudl we store the IDs we issue
    pub fn get_next_id(&self) -> String{
        //let uuid = self.ids.lock().unwrap();
        Uuid::new_v4().to_string()
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
    pub allow_build_cmd : bool,
    pub no_fs : bool,
}

#[derive(Debug, Clone)]
pub struct ServerCfg{
    pub port : u16,
    pub rsc : Arc<RunServerCfg>,
    pub caches : Arc<Caches>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCmdCfg{
    pub class_name : String,
    pub l_cmd : Option<String>,
    pub w_cmd : Option<String>,
}

