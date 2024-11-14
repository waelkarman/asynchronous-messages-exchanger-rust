use std::net::UdpSocket;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::collections::VecDeque;
use std::collections::HashMap;


use asynchronous_messages_exchanger_rust::utilities::MSG_TYPE;
mod msg_pack;

struct UdpClient{
    sent_sequence: Arc<Mutex<i32>>,
    sent_messages: Arc<Mutex<HashMap<i32,String>>>,
    recv_ack_queue: Arc<Mutex<VecDeque<i32>>>,
    socket: Arc<Mutex<UdpSocket>>,
    tasks: Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>,
    handlers: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>, 
    messages_queue: Arc<Mutex<VecDeque<String>>>,
    messages_to_print: Arc<Mutex<HashMap<i32,String>>>,
}

impl UdpClient {

    fn run() -> (){
        let socket= UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.set_nonblocking(true).expect("Errore nella impostazione della socket come non bloccante");
        let instance= Arc::new(UdpClient {
            sent_sequence: Arc::new(Mutex::new(0)),
            sent_messages: Arc::new(Mutex::new(HashMap::new())),
            recv_ack_queue: Arc::new(Mutex::new(VecDeque::new())),
            socket: Arc::new(Mutex::new(socket)),
            tasks:  Arc::new(Mutex::new(Vec::new())),
            handlers: Arc::new(Mutex::new(Vec::new())),
            messages_queue: Arc::new(Mutex::new(VecDeque::new())),
            messages_to_print: Arc::new(Mutex::new(HashMap::new())),
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
        let socket   = self.socket.lock().unwrap();
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

        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.received_message_loop();
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
                {
                    let mut mq = self.messages_queue.lock().unwrap();
                    msg = mq.pop_front().unwrap();
                }
                msg = msg_pack::msg_pack(*self.sent_sequence.lock().unwrap(), MSG_TYPE::MSG,msg);
            }else{
                msg = String::new();
                msg.push_str("Alive!!");
                msg = msg_pack::msg_pack(*self.sent_sequence.lock().unwrap(), MSG_TYPE::MSG,msg);
            }

            
            {
                let socket = self.socket.lock().unwrap();
                socket.send(msg.as_bytes()).unwrap();
            }
            println!("Messaggio inviato: {:?}", msg);
            {
                let mut sent_messages = self.sent_messages.lock().unwrap();
                sent_messages.insert(*self.sent_sequence.lock().unwrap(), msg);
            }
            {
                *self.sent_sequence.lock().unwrap() += 1;
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn received_message_loop(&self) {
        loop{
            thread::sleep(Duration::from_secs(1));
            while !self.recv_ack_queue.lock().unwrap().is_empty() {
                let ack_n;
                {
                    let mut recv_ack_queue = self.recv_ack_queue.lock().unwrap();
                    ack_n = recv_ack_queue.pop_front().unwrap();
                }
                {
                    let mut sent_messages = self.sent_messages.lock().unwrap();
                    match sent_messages.remove(&ack_n) {
                        Some(msg) => {
                            println!("Messaggio con sequence {} rimosso correttamente: {:?}", ack_n, msg);
                        }
                        None => {
                            println!("ACK duplicato ricevuto per sequence {}: messaggio già rimosso", ack_n);
                        }
                    }
                }
            }
        }
    }

    fn message_handler_loop(&self) {
        loop{
            let mut buffer = [0u8; 1024];            
            let out;
            {
                let socket = self.socket.lock().unwrap();
                out = socket.recv_from(&mut buffer);
            }

            match out {
                Ok((size, _)) => {
                    let message = String::from_utf8_lossy(&buffer[..size]).to_string();
                    let cleaned_message = message.replace("'", "");
                    let (seq, msg_t, s) = msg_pack::msg_unpack(cleaned_message);
                    match msg_t {
                        MSG_TYPE::MSG => {
                            println!("MSG ricevuto: {}", s);
                            {
                                let mut messages_to_print = self.messages_to_print.lock().unwrap();
                                messages_to_print.insert(seq,s);
                            }
                        }
                        MSG_TYPE::ACK =>{
                            println!("ACK ricevuto: {}", seq);
                            {
                                let mut recv_ack_queue = self.recv_ack_queue.lock().unwrap();
                                recv_ack_queue.push_back(seq);
                            }
                        }
                        MSG_TYPE::UNKNOWN => {
                            println!("Messaggio sconosciuto.");
                        }
                    }
                }
                Err(e) => {
                    continue;                    
                }
            }
        }
    }
}



fn main() {
    UdpClient::run();
}

