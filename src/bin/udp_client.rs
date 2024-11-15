use std::net::UdpSocket;
use std::thread;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::sync::mpsc;

use rand::Rng;


use asynchronous_messages_exchanger_rust::utilities::MSG_TYPE;
mod msg_pack;

struct UdpClient{
    sent_sequence: Arc<Mutex<i32>>,
    sent_messages: Arc<Mutex<HashMap<i32,String>>>,
    sent_messages_condvar: Arc<Condvar>,
    recv_ack_queue: Arc<Mutex<VecDeque<i32>>>,
    recv_ack_queue_condvar: Arc<Condvar>,
    socket: Arc<Mutex<UdpSocket>>,
    tasks: Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>,
    handlers: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>,
    t_handlers: Arc<Mutex<HashMap<i32,std::thread::JoinHandle<()>>>>, 
    messages_queue: Arc<Mutex<VecDeque<String>>>,
    messages_queue_condvar: Arc<Condvar>,
    messages_to_print: Arc<Mutex<HashMap<i32,String>>>,
    messages_to_print_condvar: Arc<Condvar>,
    send_failure: Arc<Mutex<i32>>,
}

impl UdpClient {

    fn run() -> (){
        let socket= UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.set_nonblocking(true).expect("Error setting non blocking");
        let instance= Arc::new(UdpClient {
            sent_sequence: Arc::new(Mutex::new(0)),
            sent_messages: Arc::new(Mutex::new(HashMap::new())),
            sent_messages_condvar: Arc::new(Condvar::new()),
            recv_ack_queue: Arc::new(Mutex::new(VecDeque::new())),
            recv_ack_queue_condvar: Arc::new(Condvar::new()),
            socket: Arc::new(Mutex::new(socket)),
            tasks:  Arc::new(Mutex::new(Vec::new())),
            handlers: Arc::new(Mutex::new(Vec::new())),
            t_handlers: Arc::new(Mutex::new(HashMap::new())),
            messages_queue: Arc::new(Mutex::new(VecDeque::new())),
            messages_queue_condvar: Arc::new(Condvar::new()),
            messages_to_print: Arc::new(Mutex::new(HashMap::new())),
            messages_to_print_condvar: Arc::new(Condvar::new()),
            send_failure: Arc::new(Mutex::new(0)),
        });
        instance.initialize();
        instance.main_loop();
    }

    fn add_to_messages_queue(&self,s: &str){
        let mut msg_queue = self.messages_queue.lock().unwrap();
        msg_queue.push_back(s.to_string());
    }

    fn message_generator(&self){
        let mut rng = rand::thread_rng();
        loop{
            let random_number = rng.gen_range(1..=500);
            thread::sleep(Duration::from_millis(random_number));
            let mut message = String::from("HELLO ");
            message.push_str(random_number.to_string().as_str());
            message.push_str("ms");
            self.add_to_messages_queue(&message);
            self.messages_queue_condvar.notify_all();
        }
    }

    fn initialize(&self) {
        let socket   = self.socket.lock().unwrap();
        socket.connect("127.0.0.1:8080").expect("Connection failed.");
        println!("Connection established.");    
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

    // fn handle_printing(&self){
    //     loop{
    //         {
    //             let mut messages_to_print = self.messages_to_print.lock().unwrap();
    //             while messages_to_print.is_empty() {
    //                 messages_to_print = self.messages_to_print_condvar.wait(messages_to_print).unwrap();
    //             }
    //         }

    //         // gestisci una finestra che tenga in conto dell'out-of-order
    //         // while !self.messages_to_print.lock().unwrap().is_empty() {
                
    //         // }
    //         thread::sleep(Duration::from_secs(1));

    //     }
    // }

    // fn connection_monitor(&self){

    // }

    fn timer_launcher(&self, n: i32, index: i32) {
        let mut attempt = 1;
        let mut spin = true;
        while attempt < 4 && spin {
            thread::sleep(Duration::from_millis(n as u64)); 
            let contains;
            {
                let sent_messages = self.sent_messages.lock().unwrap();
                contains = sent_messages.contains_key(&index);
            }   
             
            if contains {
                println!("Timeout:{} retry attempt: {}/3",index,attempt);
                attempt += 1;
                
                let sent_messages = self.sent_messages.lock().unwrap();
                if let Some(msg) = sent_messages.get(&index) {
                    let socket = self.socket.lock().unwrap();
                    socket.send(msg.as_bytes()).unwrap();
                    println!("Message resend: {:?}", msg);
                }
                
            } else {
                println!("Message {} delivered.",index);
                spin = false;
            }
        }

        if attempt == 4 || spin
        {
            println!("Message {} delivery failure.",index);
            *self.send_failure.lock().unwrap() += 1;
        }        
        
    }

    fn timers_loop(self: &Arc<Self>){
        let (tx, rx) = mpsc::channel();
        let mut index = 0;
        loop{
            let mut contains= false;
            {
                let sent_messages = self.sent_messages.lock().unwrap();
                contains = sent_messages.contains_key(&index);
            }
                
            while !contains {
                {
                    let mut sent_messages = self.sent_messages.lock().unwrap();
                    while sent_messages.is_empty() {
                        sent_messages = self.sent_messages_condvar.wait(sent_messages).unwrap();
                    }
                }
                {
                    let sent_messages = self.sent_messages.lock().unwrap();
                    contains = sent_messages.contains_key(&index);
                }
            }

            if contains{
                let thread_tx = tx.clone();
                let client_clone = Arc::clone(&self);
                let index_ref = index;

                let mut tasks = self.tasks.lock().unwrap();
                tasks.push(Arc::new(move || {
                    client_clone.timer_launcher(500,index);
                    thread_tx.send(format!("{}",index_ref)).unwrap();
                }));
                drop(tasks);
        
                while !self.tasks.lock().unwrap().is_empty() {
                    let client_clone = Arc::clone(&self);
                    let mut t_handlers = self.t_handlers.lock().unwrap();
                    t_handlers.insert(index,thread::spawn(move || { 
                        client_clone.task_launcher(); 
                    }));
                }
            }
            
            loop {
                match rx.try_recv() {
                    Ok(message) => {
                        println!("Timer for message {} ended!",message);
                        let mut t_handlers = self.t_handlers.lock().unwrap();
                        let key: i32 = message.parse().unwrap();

                        let task = t_handlers.get(&key).unwrap();

                        if let Some(task) = t_handlers.remove(&key) {
                            task.join().unwrap();
                            println!("Joined and removed element with key: {}", key);
                        } else {
                            println!("Key not found in the HashMap.");
                        }

                        break;
                    }
                    Err(err) => {
                        break;
                    }
                }
            }
            index += 1;         
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
            client_clone.message_generator();
        }));

        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.confirm_message_delivery();
        }));

        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.timers_loop();
        }));

        while !self.tasks.lock().unwrap().is_empty() {
            let client_clone = Arc::clone(&self);
            let mut handlers = self.handlers.lock().unwrap();
            handlers.push(thread::spawn(move || { 
                client_clone.task_launcher(); 
            }));
        }

        /* Rust & C++ main difference in thread handling*/
        let mut handlers = self.handlers.lock().unwrap();
        for task in handlers.drain(..) {
            if let Err(e) = task.join() {
                eprintln!("The thread returned an error: {:?}", e);
            }
        }
    }

    fn fetch_and_send_loop(&self) {
        loop {
            {
                let mut messages_queue = self.messages_queue.lock().unwrap();
                while messages_queue.is_empty() {
                    messages_queue = self.messages_queue_condvar.wait(messages_queue).unwrap();
                }
            }

            let mut msg;

            {
                let mut mq = self.messages_queue.lock().unwrap();
                msg = mq.pop_front().unwrap();
            }
            msg = msg_pack::msg_pack(*self.sent_sequence.lock().unwrap(), MSG_TYPE::MSG,msg);
            
            {
                let socket = self.socket.lock().unwrap();
                socket.send(msg.as_bytes()).unwrap();
            }
            println!("Message sent: {:?}", msg);
            {
                let mut sent_messages = self.sent_messages.lock().unwrap();
                sent_messages.insert(*self.sent_sequence.lock().unwrap(), msg);
                self.sent_messages_condvar.notify_all();
            }
            
            {
                *self.sent_sequence.lock().unwrap() += 1;
            }
        }
    }

    fn confirm_message_delivery(&self) {
        loop{
            {
                let mut recv_ack_queue = self.recv_ack_queue.lock().unwrap();
                while recv_ack_queue.is_empty() {
                    recv_ack_queue = self.recv_ack_queue_condvar.wait(recv_ack_queue).unwrap();
                }
            }

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
                            println!("Message with sequence {} successfully removed: {:?}.", ack_n, msg);
                        }
                        None => {
                            println!("Duplicate ACK received for sequence {}: message already removed.", ack_n);
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
                            println!("MSG received: {}", s);
                            {
                                let mut messages_to_print = self.messages_to_print.lock().unwrap();
                                messages_to_print.insert(seq,s);
                            }
                        }
                        MSG_TYPE::ACK =>{
                            println!("ACK received: {}", seq);
                            {
                                let mut recv_ack_queue = self.recv_ack_queue.lock().unwrap();
                                recv_ack_queue.push_back(seq);
                                self.recv_ack_queue_condvar.notify_all();
                            }
                        }
                        MSG_TYPE::UNKNOWN => {
                            println!("Message type unknown.");
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

