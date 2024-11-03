use std::net::UdpSocket;
use std::io::{self, Write};


struct UdpClient{
    socket: UdpSocket,
}

impl UdpClient {

    fn connect(&mut self) {
        self.socket.connect("127.0.0.1:8080").unwrap();     
    }


    fn fetch_and_send_loop(&mut self) {
        print!("Inserisci un messaggio da inviare al server: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        self.socket.send(input.as_bytes()).unwrap();
    }


    fn allelse(&mut self) -> std::io::Result<()> {
        let mut buffer = [0u8; 1024];
        let bytes_received = self.socket.recv(&mut buffer).unwrap();
        let response = String::from_utf8_lossy(&buffer[..bytes_received]);
    
        println!("Risposta del server: {}", response);
    
        Ok(())
    }

}



fn main() {
    let mut client = UdpClient{socket: UdpSocket::bind("127.0.0.1:0").unwrap()};
    client.connect();
    client.fetch_and_send_loop();
    client.allelse();

}

