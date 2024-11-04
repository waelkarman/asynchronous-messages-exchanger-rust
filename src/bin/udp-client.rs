use std::net::UdpSocket;
use std::io::{self, Write};
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct UdpClient{
    socket: Arc<Mutex<UdpSocket>>,
    tasks: Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>,
    handlers: Mutex<Vec<thread::JoinHandle<()>>>,
}

impl UdpClient {

    fn create() -> Arc<UdpClient>{
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.set_nonblocking(true).expect("Errore nella impostazione della socket come non bloccante");
        let instance= Arc::new(UdpClient {
            socket: Arc::new(Mutex::new(socket)),
            tasks:  Arc::new(Mutex::new(Vec::new())),
            handlers: Mutex::new(Vec::new()),
        });
        instance.connect();
        instance.main_loop();
        instance
    }

    fn connect(&self) {
        let mut socket = self.socket.lock().unwrap();
        socket.connect("127.0.0.1:8080").expect("Connessione fallita");
        println!("Connessione eseguita.");    
    }

    
    fn task_launcher(self: Arc<Self>){
        for task in self.tasks.lock().unwrap().iter() {
            let task_copy = task.clone();
            let handle = thread::spawn(move || {
                task_copy();
            });
            self.handlers.lock().unwrap().push(handle);
        }
        
        let mut handlers = self.handlers.lock().unwrap();
        for task in handlers.drain(..) {
            if let Err(e) = task.join() {
                eprintln!("Il thread ha restituito un errore: {:?}", e);
            }
        }
    }


    fn main_loop(self: &Arc<Self>){
        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.fetch_and_send_loop();
        }));

        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.message_handler_loop().unwrap();
        }));

        let client_clone = Arc::clone(&self);
        let handle = thread::spawn(move || { client_clone.task_launcher(); }); 
        handle.join().expect("Il thread è terminato con un errore");
    }

    fn fetch_and_send_loop(&self) {
        loop {
            let mut input = String::new();
            input.push_str("CIAO!");
            {
                let mut socket = self.socket.lock().unwrap();
                socket.send(input.as_bytes()).unwrap();
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    fn message_handler_loop(&self) -> std::io::Result<()> {
        loop{
            let mut buffer = [0u8; 1024];

            {
                let mut socket = self.socket.lock().unwrap();
                match socket.recv_from(&mut buffer) {
                    Ok((size, _)) => {
                        println!("Messaggio ricevuto: {}", String::from_utf8_lossy(&buffer[..size]));
                    }
                    Err(e) => {
                        println!("Errore durante la ricezione: {:?}", e);
                    }
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

}



fn main() {
    let mut client = UdpClient::create();
    
}

