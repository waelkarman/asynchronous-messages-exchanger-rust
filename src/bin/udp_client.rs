#![allow(unused_imports)]

use std::net::UdpSocket;
use std::thread;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::sync::mpsc;
use std::process;
use rand::Rng;

use std::mem;

use asynchronous_messages_exchanger_rust::utilities::MsgType;
use asynchronous_messages_exchanger_rust::utilities::Speed;
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
    timers_handlers: Arc<Mutex<HashMap<i32,std::thread::JoinHandle<()>>>>,
    messages_queue: Arc<Mutex<VecDeque<String>>>,
    messages_queue_condvar: Arc<Condvar>,
    messages_to_print: Arc<Mutex<HashMap<i32,String>>>,
    messages_to_print_condvar: Arc<Condvar>,
    send_failure: Arc<Mutex<i32>>,
    ordered_window_size: Arc<i32>,
    limit: Arc<i32>,
    speed: bool,
    current_speed: Arc<Mutex<u64>>,
    slow_down_messages_generation: Arc<Mutex<bool>>,
    slow_down_messages_generation_condvar: Arc<Condvar>,
    referee: Arc<Mutex<i32>>,
    referee_condvar: Arc<Condvar>,
}

impl UdpClient {

    fn run(mode: Speed) -> (){
        let socket= UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.set_nonblocking(true).expect("Error setting non blocking");
        let ordered_window_size = 10;
        let instance= Arc::new(UdpClient {
            sent_sequence: Arc::new(Mutex::new(0)),
            sent_messages: Arc::new(Mutex::new(HashMap::new())),
            sent_messages_condvar: Arc::new(Condvar::new()),
            recv_ack_queue: Arc::new(Mutex::new(VecDeque::new())),
            recv_ack_queue_condvar: Arc::new(Condvar::new()),
            socket: Arc::new(Mutex::new(socket)),
            tasks:  Arc::new(Mutex::new(Vec::new())),
            handlers: Arc::new(Mutex::new(Vec::new())),
            timers_handlers: Arc::new(Mutex::new(HashMap::new())),
            messages_queue: Arc::new(Mutex::new(VecDeque::new())),
            messages_queue_condvar: Arc::new(Condvar::new()),
            messages_to_print: Arc::new(Mutex::new(HashMap::new())),
            messages_to_print_condvar: Arc::new(Condvar::new()),
            send_failure: Arc::new(Mutex::new(0)),
            ordered_window_size: Arc::new(ordered_window_size),
            limit: Arc::new(200000000*ordered_window_size),
            speed: mode.into(),
            current_speed: Arc::new(Mutex::new(0)),
            slow_down_messages_generation: Arc::new(Mutex::new(false)),
            slow_down_messages_generation_condvar: Arc::new(Condvar::new()),
            referee: Arc::new(Mutex::new(0)),
            referee_condvar: Arc::new(Condvar::new()),
        });
        instance.initialize();
        instance.main_loop();
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

    fn handle_printing(&self){
        let mut message_processed = 0;
        let mut ordered_window = vec![String::from("invalid"); *self.ordered_window_size as usize];
        loop{
            {
                let mut messages_to_print = self.messages_to_print.lock().unwrap();
                while messages_to_print.is_empty() {
                    messages_to_print = self.messages_to_print_condvar.wait(messages_to_print).unwrap();
                }
            }

            loop {

                if !self.messages_to_print.lock().unwrap().contains_key(&message_processed) {
                    break;

                    /* DEBUGGING INFO */ 
                    // println!("Message missing in the flow! {} : len {}",message_processed,self.messages_to_print.lock().unwrap().len());
                    // if let Ok(map) = self.messages_to_print.lock() {
                    //     println!("{:?}", *map);
                    // }else {
                    //     println!("Unable to lock the Mutex.");
                    // }

                    /* BYPASS MISSING VALUE */
                    //self.messages_to_print.lock().unwrap().clear();
                }
                
                {
                    let position_int;
                    let messages_to_print = self.messages_to_print.lock().unwrap();
                    if let Some(element) = messages_to_print.get(&message_processed) {
                        let position = message_processed % *self.ordered_window_size;
                        position_int = position as usize;
                        if let Some(valore) = ordered_window.get_mut(position_int) {
                            *valore = element.clone();
                        }
                    }
                }
            
                {
                    let mut messages_to_print = self.messages_to_print.lock().unwrap();
                    messages_to_print.remove(&message_processed);
                }

                if (message_processed % *self.ordered_window_size) as i32 == *self.ordered_window_size - 1 {
                    println!("{:?}", ordered_window);
                }

                message_processed = (message_processed+1) % *self.limit;
            }
        }
    }

    fn connection_monitor(&self){
        let stress_factor = 100;
        loop{
            if self.speed == Speed::Dynamic.into() {
                let current_speed = *self.current_speed.lock().unwrap();
                thread::sleep(Duration::from_millis(current_speed*10));
            }else{
                thread::sleep(Duration::from_millis(1));
            }

            /* LOAD INFORMATIONS */
            // println!("Size of timers_handlers: {} bytes", mem::size_of_val(&self.timers_handlers.lock().unwrap()));
            // println!("Size of sent_messages: {} bytes", mem::size_of_val(&self.sent_messages.lock().unwrap()));
            // println!("Size of recv_ack_queue: {} bytes", mem::size_of_val(&self.recv_ack_queue.lock().unwrap()));
            // println!("Size of messages_queue: {} bytes", mem::size_of_val(&self.messages_queue.lock().unwrap()));
            // println!("Size of messages_to_print: {} bytes", mem::size_of_val(&self.messages_to_print.lock().unwrap()));
            
            // println!("timers_handlers size: {}", self.timers_handlers.lock().unwrap().len());
            // println!("sent_messages size: {}", self.sent_messages.lock().unwrap().len());
            // println!("recv_ack_queue size: {}", self.recv_ack_queue.lock().unwrap().len());
            // println!("messages_queue size: {}", self.messages_queue.lock().unwrap().len());
            // println!("messages_to_print size: {}", self.messages_to_print.lock().unwrap().len());
            
            let messages_queue_len= self.messages_queue.lock().unwrap().len();
            
            if  messages_queue_len > stress_factor 
            {
                {
                    let mut slow_down = self.slow_down_messages_generation.lock().unwrap();
                    *slow_down = true;
                }
                //println!("Speed Controller    - messages_queue size over the stress_factor: {} -> 100 ",messages_queue_len);
                self.messages_queue_condvar.notify_all();
                let mut referee = self.referee.lock().unwrap();
                *referee = 0;
                self.referee_condvar.notify_all();

            }else{
                {
                    let mut slow_down = self.slow_down_messages_generation.lock().unwrap();
                    *slow_down = false;
                }
                self.slow_down_messages_generation_condvar.notify_all();
            }

            let send_failure = self.send_failure.lock().unwrap();
            if *send_failure > 0 {
                println!("fatal error: broken pipe");
                process::exit(1);
            }
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

        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.connection_monitor();
        }));

        let client_clone = Arc::clone(&self);
        self.tasks.lock().unwrap().push(Arc::new(move || {
            client_clone.handle_printing();
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

    fn message_generator(&self){
        let mut rng = rand::thread_rng();
        loop{
            let mut random_number = 0;
            if self.speed == Speed::Dynamic.into() {
                random_number = rng.gen_range(1..=5);
                thread::sleep(Duration::from_millis(random_number));
            }
            *self.current_speed.lock().unwrap() = random_number;
            let mut message = String::from("HELLO ");
            message.push_str(random_number.to_string().as_str());
            message.push_str("ms");

            let msg = msg_pack::msg_pack(*self.sent_sequence.lock().unwrap(), MsgType::MSG,message);
            {
                let mut msg_queue = self.messages_queue.lock().unwrap();
                msg_queue.push_back(msg);
            }

            {
                let updated_seq = *self.sent_sequence.lock().unwrap();
                *self.sent_sequence.lock().unwrap() = (updated_seq+1) % *self.limit;
            }

            self.messages_queue_condvar.notify_all();

            {
                let slow_down = *self.slow_down_messages_generation.lock().unwrap();
                if slow_down {
                    {
                        let mut slow_down_messages_generation = self.slow_down_messages_generation.lock().unwrap();
                        while *slow_down_messages_generation == true {
                            slow_down_messages_generation= self.slow_down_messages_generation_condvar.wait(slow_down_messages_generation).unwrap();
                        }
                    }
                }
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

            {
                let mut referee = self.referee.lock().unwrap();
                while *referee == 1 {
                    referee = self.referee_condvar.wait(referee).unwrap();
                }
                *referee = 1;
            }

            let mut msg;

            {
                let mut mq = self.messages_queue.lock().unwrap();
                msg = mq.pop_front().unwrap();
            }

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

            self.referee_condvar.notify_all();
        }
    }

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
                    {
                        let mut mq = self.messages_queue.lock().unwrap();
                        let msg = msg_pack::msg_pack(index, MsgType::MSG,msg.to_string());
                        mq.push_front(msg); 
                    }
                    println!("Message resend: {:?}", msg);
                }

            } else {
                println!("Message {} delivered on attempt {}.",index,attempt);
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

            {
                let mut referee = self.referee.lock().unwrap();
                while *referee == 0 {
                    referee = self.referee_condvar.wait(referee).unwrap();
                }
                *referee = 0;
            }
                        
            let mut contains;
            {
                let sent_messages = self.sent_messages.lock().unwrap();
                contains = if sent_messages.contains_key(&index) { true } else { false };
            }

            while contains == false {
                {
                    let mut sent_messages = self.sent_messages.lock().unwrap();
                    while sent_messages.is_empty() {
                        sent_messages = self.sent_messages_condvar.wait(sent_messages).unwrap();
                    }
                }
                {
                    let sent_messages = self.sent_messages.lock().unwrap();
                    contains = if sent_messages.contains_key(&index) { true } else { false };
                }
            }

            if contains == true {
                let thread_tx = tx.clone();
                let client_clone = Arc::clone(&self);
                let index_ref = index;

                let mut tasks = self.tasks.lock().unwrap();
                tasks.push(Arc::new(move || {
                    let time = *client_clone.current_speed.lock().unwrap()*2+10;
                    client_clone.timer_launcher(time as i32,index);
                    thread_tx.send(format!("{}",index_ref)).unwrap();
                }));
                drop(tasks);

                while !self.tasks.lock().unwrap().is_empty() {
                    let client_clone = Arc::clone(&self);
                    let mut timers_handlers = self.timers_handlers.lock().unwrap();
                    timers_handlers.insert(index,thread::spawn(move || {
                        client_clone.task_launcher();
                    }));
                }

                self.referee_condvar.notify_all();
            }

            loop {
                match rx.try_recv() {
                    Ok(message) => {
                        let mut timers_handlers = self.timers_handlers.lock().unwrap();
                        let key: i32 = message.parse().unwrap();

                        if let Some(task) = timers_handlers.remove(&key) {
                            task.join().unwrap();
                        } else {
                            println!("Key not found in the timers_handlers.");
                        }

                        break;
                    }
                    Err(_err) => {
                        break;
                    }
                }
            }
            index += 1;
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
            
            loop {
                {
                    let recv_ack_queue = self.recv_ack_queue.lock().unwrap();
                    let recv_ack_queue_empty = recv_ack_queue.is_empty();
                    if recv_ack_queue_empty {break;}
                }

                {
                    let mut recv_ack_queue = self.recv_ack_queue.lock().unwrap();
                    let mut sent_messages = self.sent_messages.lock().unwrap();
                    let ack_n = recv_ack_queue.front().unwrap();
                    if let Some(_) = sent_messages.remove(&ack_n) {
                        recv_ack_queue.pop_front().unwrap();
                    } else {
                        continue;
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
                        MsgType::MSG => {
                            //println!("Received MSG: {}",seq);
                            {
                                let mut messages_to_print = self.messages_to_print.lock().unwrap();
                                messages_to_print.insert(seq,s.clone());
                            }
                            self.messages_to_print_condvar.notify_all();
                            let ack_msg = msg_pack::msg_pack(seq, MsgType::ACK,s);
                            {
                                let mut msg_queue = self.messages_queue.lock().unwrap();
                                msg_queue.push_back(ack_msg);
                            }
                        }
                        MsgType::ACK =>{
                            //println!("Received ACK: {}",seq);
                            {
                                let mut recv_ack_queue = self.recv_ack_queue.lock().unwrap();
                                recv_ack_queue.push_back(seq);
                            }
                            self.recv_ack_queue_condvar.notify_all();
                        }
                        MsgType::UNKNOWN => {
                            println!("Message type unknown.");
                        }
                    }
                }
                Err(_e) => {
                    continue;
                }
            }
        }
    }
}



fn main() {
    UdpClient::run(Speed::Max);
}

