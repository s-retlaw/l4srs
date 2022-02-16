use tokio::net::{TcpListener, TcpStream};
use futures::SinkExt;
use futures::StreamExt;
use std::convert::TryFrom;
use std::net;
use std::str::FromStr;
use tokio_util::codec::{FramedRead, FramedWrite};
use crate::build_java;
use ldap3_proto::simple::*;
use ldap3_proto::LdapCodec;

use crate::common::ServerCfg;

pub struct LdapSession {
    dn: String,
    cfg : ServerCfg,
}

impl LdapSession {
    pub fn do_bind(&mut self, sbr: &SimpleBindRequest) -> LdapMsg {
        if sbr.dn == "cn=Directory Manager" && sbr.pw == "password" {
            self.dn = sbr.dn.to_string();
            sbr.gen_success()
        } else if sbr.dn == "" && sbr.pw == "" {
            self.dn = "Anonymous".to_string();
            sbr.gen_success()
        } else {
            sbr.gen_invalid_cred()
        }
    }

    pub fn do_search(&mut self, lsr: &SearchRequest) -> Vec<LdapMsg> {
        info!("in do search {:?}", lsr);
        let mut base = lsr.base.to_string();
        base.remove(0);
        let parts : Vec<&str> = base.split(":").collect();
       
        let mut name = base.clone();
        if (parts.len() == 3) && (parts[0] =="MM") {
            let addr_str = parts[1].replace(".", "_");
            let name_parts : Vec<&str> = vec![&parts[0], &addr_str, &parts[2]]; 
            name = name_parts.join("_").to_string();
            match build_java::ensure_mm_class_exists(&self.cfg.web_root, &name,  &parts[1], parts[2]) {
                Err(e) => info!("Error creating MM class {}", e),
                _ => (),
            }
      }
        info!("the base is {}", name);
        vec![
            lsr.gen_result_entry(LdapSearchResultEntry {
                dn: "cn=hello,dc=example,dc=com".to_string(),
                attributes: vec![
                    LdapPartialAttribute {
                        atype: "javaClassName".to_string(),
                        vals: vec!["TheClass".to_string()],
                    },
                    LdapPartialAttribute {
                        atype: "objectClass".to_string(),
                        vals: vec!["javaNamingReference".to_string()],
                    },
                    LdapPartialAttribute {
                        atype: "javaCodeBase".to_string(),
                        vals: vec![format!("http://{}:{}/", self.cfg.http_addr, self.cfg.http_port)],
                    },
                    LdapPartialAttribute {
                        atype: "javaFactory".to_string(),
                        vals: vec![name],
                    },
                ],
            }),
            lsr.gen_success(),
        ]
    }

    pub fn do_whoami(&mut self, wr: &WhoamiRequest) -> LdapMsg {
        wr.gen_success(format!("dn: {}", self.dn).as_str())
    }
}

async fn handle_client(socket: TcpStream, cfg : ServerCfg) {
    info!("in handle client");
    // Configure the codec etc.
    let (r, w) = tokio::io::split(socket);
    let mut reqs = FramedRead::new(r, LdapCodec);
    let mut resp = FramedWrite::new(w, LdapCodec);

    let mut session = LdapSession {
        dn: "Anonymous".to_string(),
        cfg
    };

    while let Some(msg) = reqs.next().await {
        debug!(?msg, "ldap message");
        let server_op = match msg
            .map_err(|_e| ())
            .and_then(|msg| ServerOps::try_from(msg))
        {
            Ok(v) => v,
            Err(_) => {
                let _err = resp
                    .send(DisconnectionNotice::gen(
                        LdapResultCode::Other,
                        "Internal Server Error",
                    ))
                    .await;
                let _err = resp.flush().await;
                return;
            }
        };

        let result = match server_op {
            ServerOps::SimpleBind(sbr) => vec![session.do_bind(&sbr)],
            ServerOps::Search(sr) => session.do_search(&sr),
            ServerOps::Unbind(_) => {
                // No need to notify on unbind (per rfc4511)
                return;
            }
            ServerOps::Whoami(wr) => vec![session.do_whoami(&wr)],
        };

        for rmsg in result.into_iter() {
            if let Err(_) = resp.send(rmsg).await {
                return;
            }
        }

        if let Err(_) = resp.flush().await {
            return;
        }
    }
    // Client disconnected
}

async fn acceptor(listener: Box<TcpListener>, cfg : ServerCfg) {
    loop {
        match listener.accept().await {
            Ok((socket, _paddr)) => {
                tokio::spawn(handle_client(socket, cfg.clone()));
            }
            Err(_e) => {
                //pass
            }
        }
    }
}

pub async fn start_ldap_server(cfg : ServerCfg){
    let addr = net::SocketAddr::from_str(&format!("0.0.0.0:{}", cfg.ldap_port)).unwrap();
    let listener = Box::new(TcpListener::bind(&addr).await.unwrap());
    // Initiate the acceptor task.
    println!("startng ldap://0.0.0.0:{} ...", cfg.ldap_port);
    tokio::spawn(acceptor(listener, cfg.clone()));
}

