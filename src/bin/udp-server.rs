use std::net::UdpSocket;
use std::str;

fn main() -> std::io::Result<()> {
    // Creiamo una socket UDP in ascolto sull'indirizzo specificato
    let socket = UdpSocket::bind("127.0.0.1:8080")?;
    println!("Server in ascolto su 127.0.0.1:8080");

    let mut buffer = [0u8; 1024];  // Buffer per memorizzare i dati ricevuti

    loop {
        // Riceviamo i dati e otteniamo l'indirizzo del client
        let (bytes_received, client_addr) = socket.recv_from(&mut buffer)?;
        let msg = str::from_utf8(&buffer[..bytes_received])
            .expect("Messaggio non in formato UTF-8");

        println!("Ricevuto dal client {}: '{}'", client_addr, msg);

        // Inviamo una risposta al client
        let response = format!("Messaggio ricevuto: '{}'", msg);
        socket.send_to(response.as_bytes(), client_addr)?;
    }
}
