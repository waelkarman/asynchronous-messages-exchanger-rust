
use asynchronous_messages_exchanger_rust::utilities::{MSG_TYPE};


pub fn msg_pack(seq: i32, msg_type: MSG_TYPE,s: String) -> String {
    let msg_type_str = format!("{:?}", msg_type);
    let mut complete_msg = String::from(seq.to_string());
    complete_msg.push_str("."); 
    complete_msg.push_str(msg_type_str.as_str());
    complete_msg.push_str("."); 
    complete_msg.push_str(s.as_str()); 
    complete_msg
}

pub fn msg_unpack(s: String) -> (i32, MSG_TYPE, String) {
    let mut parts = s.splitn(3, '.');
    if let (Some(seq_str), Some(msg_type_str), Some(message)) = (parts.next(), parts.next(), parts.next()) {
        let seq = seq_str.parse::<i32>().unwrap_or(0);
        let msg_type = match msg_type_str {
            "ACK" => MSG_TYPE::ACK,
            "MSG" => MSG_TYPE::MSG,
            _ => MSG_TYPE::UNKNOWN, 
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
        assert_eq!(msg_pack(MSG_TYPE::ACK,String::from("Ciaooo")), String::from("ACK.Ciaooo"));
    }

    #[test]
    fn test_1() {
        assert_eq!(msg_unpack(String::from("ACK.Ciaooo")), (MSG_TYPE::ACK, String::from("Ciaooo")));
    }
}