use std::net::UdpSocket;
use std::str;

struct UdpServer{
    socket: UdpSocket,
}

impl UdpServer {

    fn connect(&mut self) { 
        println!("Server in ascolto su 127.0.0.1:8080");
    }

    fn allelse(&mut self) -> std::io::Result<()> {
        let mut buffer = [0u8; 1024]; 

        loop {
            let (bytes_received, client_addr) = self.socket.recv_from(&mut buffer).unwrap();
            let msg = str::from_utf8(&buffer[..bytes_received]).expect("Messaggio non in formato UTF-8");
    
            println!("Ricevuto dal client {}: '{}'", client_addr, msg);
    
            let response = format!("Messaggio ricevuto: '{}'", msg);
            self.socket.send_to(response.as_bytes(), client_addr).unwrap();
        }
    }

}

fn main() {
    let mut server = UdpServer{
        socket: UdpSocket::bind("127.0.0.1:8080").unwrap()
    };

    server.connect();
    server.allelse();


}
