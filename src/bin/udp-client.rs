use std::net::UdpSocket;
use std::io::{self, Write};

fn main() -> std::io::Result<()> {
    // Creiamo una socket UDP
    let socket = UdpSocket::bind("127.0.0.1:0")?;  // Porta casuale per il client
    socket.connect("127.0.0.1:8080")?;  // Collegamento al server

    // Lettura dell'input utente
    print!("Inserisci un messaggio da inviare al server: ");
    io::stdout().flush()?;  // Flush per mostrare il prompt
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Inviamo il messaggio al server
    socket.send(input.as_bytes())?;

    // Riceviamo la risposta dal server
    let mut buffer = [0u8; 1024];
    let bytes_received = socket.recv(&mut buffer)?;
    let response = String::from_utf8_lossy(&buffer[..bytes_received]);

    println!("Risposta del server: {}", response);

    Ok(())
}
