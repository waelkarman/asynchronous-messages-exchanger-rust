
use asynchronous_messages_exchanger_rust::utilities::{MSG_TYPE};


pub fn msg_pack(msg_type: MSG_TYPE,s: String) -> String {
    let msg_type_str = format!("{:?}", msg_type);
    let mut complete_msg = String::from(msg_type_str.as_str());
    complete_msg.push_str("."); 
    complete_msg.push_str(s.as_str()); 
    complete_msg
}

pub fn msg_unpack(s: String) -> (MSG_TYPE, String) {
    if let Some((msg_type_str, message)) = s.split_once('.') {
        let msg_type = match msg_type_str {
            "ACK" => MSG_TYPE::ACK,
            "NACK" => MSG_TYPE::MSG,
            _ => MSG_TYPE::UNKNOWN, 
        };
        (msg_type, message.to_string())
    } else {
        (MSG_TYPE::UNKNOWN, String::from(""))
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