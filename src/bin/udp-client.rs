use std::net::UdpSocket;
use std::io::{self, Write};
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::collections::VecDeque;

struct UdpClient{
    socket: Arc<Mutex<UdpSocket>>,
    tasks: Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>,
    handlers: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>, 
    messages_queue: Arc<Mutex<VecDeque<String>>>,
}

impl UdpClient {

    fn run() -> (){
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        //socket.set_nonblocking(true).expect("Errore nella impostazione della socket come non bloccante");
        let mut instance= Arc::new(UdpClient {
            socket: Arc::new(Mutex::new(socket)),
            tasks:  Arc::new(Mutex::new(Vec::new())),
            handlers: Arc::new(Mutex::new(Vec::new())),
            messages_queue: Arc::new(Mutex::new(VecDeque::new())),
        });
        instance.initialize();
        instance.main_loop();
    }



    fn add_to_messages_queue(&self,s: &str){
        let mut msg_queue = self.messages_queue.lock().unwrap();
        msg_queue.push_back(s.to_string());
    }

    fn filler(&self){
        self.add_to_messages_queue("HELLO");
        self.add_to_messages_queue("HELLO");
        self.add_to_messages_queue("HELLO");
        self.add_to_messages_queue("HELLO");
        self.add_to_messages_queue("HELLO");
        self.add_to_messages_queue("HELLO");
        self.add_to_messages_queue("HELLO");
    }



    fn initialize(&self) {
        let mut socket: std::sync::MutexGuard<'_, UdpSocket> = self.socket.lock().unwrap();
        socket.connect("127.0.0.1:8080").expect("Connessione fallita");
        println!("Connessione eseguita.");    
    }

    



    fn task_launcher(&self){
        let f;
        {
            let mut tasks = self.tasks.lock().unwrap();
            f = tasks.pop();
        }
        if let Some(task) = f {
            task();  
        }
    }

    fn main_loop(self: &Arc<Self>){
        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.message_handler_loop()
        }));

        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.fetch_and_send_loop();
        }));

        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.filler();
        }));

        while !self.tasks.lock().unwrap().is_empty() {
            let client_clone = Arc::clone(&self);
            let mut handlers = self.handlers.lock().unwrap();
            handlers.push(thread::spawn(move || { 
                client_clone.task_launcher(); 
            }));
        }

        /* DIVERSO COMPORTAMENTO CON IL C++ */
        let mut handlers = self.handlers.lock().unwrap();
        for task in handlers.drain(..) {
            if let Err(e) = task.join() {
                eprintln!("Il thread ha restituito un errore: {:?}", e);
            }
        }

    }




    fn fetch_and_send_loop(&self) {
        loop {
            let mut msg;
            
            if !self.messages_queue.lock().unwrap().is_empty() {
                let mut mq = self.messages_queue.lock().unwrap();
                msg = mq.pop_front().unwrap();
            }else{
                msg = String::new();
                msg.push_str("Alive !!");
            }

            {
                let mut socket = self.socket.lock().unwrap();
                socket.send(msg.as_bytes()).unwrap();
            }
            thread::sleep(Duration::from_secs(1));
        }
    }




    fn message_handler_loop(&self) {
        loop{
            let mut buffer = [0u8; 1024];

            {
                let socket = self.socket.lock().unwrap();
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
    UdpClient::run();
}

