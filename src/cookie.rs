use std::fmt::{Display, Formatter, Result};

pub struct Cookie<'a> {
    name: &'a str,
    value: String
}

impl Cookie<'_> {
    pub fn new(name: &str, value: String) -> Cookie {
        Cookie {
            name,
            value
        }
    }
}

impl Display for Cookie<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}={}; Max-Age=86400; HttpOnly; Path=/; SameSite=Lax", self.name, self.value)
    }
}
