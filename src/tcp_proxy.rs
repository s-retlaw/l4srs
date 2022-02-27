use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

//use std::error::Error;

pub async fn proxy(mut client: TcpStream, server_addr: String, port : u16){
    let mut server = match TcpStream::connect(&server_addr).await{
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error connecting to proxy {} : {}", &server_addr, e);
            return ;
        }
    };

    let (mut rc, mut wc) = client.split();
    let (mut rs, mut ws) = server.split();

    let client_to_server = async {
        io::copy(&mut rc, &mut ws).await?;
        ws.shutdown().await
    };

    let server_to_client = async {
        io::copy(&mut rs, &mut wc).await?;
        wc.shutdown().await
    };

    match tokio::try_join!(client_to_server, server_to_client){
        Ok(_) => {
            eprintln!("Closing proxy from port {}", port);
        },
        Err(e) => {
            eprintln!("Error with proxy from port {} : {}", port, e);
        },
    }
}
