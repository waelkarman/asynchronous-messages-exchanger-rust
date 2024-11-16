pub mod utilities {
    #[derive(Debug, PartialEq)]
    pub enum MSG_TYPE {
        MSG,
        ACK,
        UNKNOWN,
    }

    pub enum Speed {
        Max,
        Dynamic,
    }
    
    impl Into<bool> for Speed {
        fn into(self) -> bool {
            match self {
                Speed::Max => true,
                Speed::Dynamic => false,
            }
        }
    }
}