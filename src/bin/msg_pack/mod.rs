
use asynchronous_messages_exchanger_rust::utilities::{MsgType};


pub fn msg_pack(seq: i32, msg_type: MsgType,s: String) -> String {
    let msg_type_str = format!("{:?}", msg_type);
    let mut complete_msg = String::from(seq.to_string());
    complete_msg.push_str("."); 
    complete_msg.push_str(msg_type_str.as_str());
    complete_msg.push_str("."); 
    complete_msg.push_str(s.as_str()); 
    complete_msg
}

pub fn msg_unpack(s: String) -> (i32, MsgType, String) {
    let mut parts = s.splitn(3, '.');
    if let (Some(seq_str), Some(msg_type_str), Some(message)) = (parts.next(), parts.next(), parts.next()) {
        let seq = seq_str.parse::<i32>().unwrap_or(0);
        let msg_type = match msg_type_str {
            "ACK" => MsgType::ACK,
            "MSG" => MsgType::MSG,
            _ => MsgType::UNKNOWN, 
        };
        (seq, msg_type, message.to_string())
    } else {
        panic!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_0() {
        assert_eq!(msg_pack(MsgType::ACK,String::from("Ciaooo")), String::from("ACK.Ciaooo"));
    }

    #[test]
    fn test_1() {
        assert_eq!(msg_unpack(String::from("ACK.Ciaooo")), (MsgType::ACK, String::from("Ciaooo")));
    }
}