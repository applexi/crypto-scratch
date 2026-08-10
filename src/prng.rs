use rand;

use crate::ArithShare;

pub struct PRNG {
    val: ArithShare,
    next: Option<Box<PRNG>>,
}

impl PRNG {
    pub fn new() -> Self {
        PRNG { val: rand::random(), next: None }
    }
    pub fn next(&mut self) -> &mut Self {
        if self.next.is_none() {
            self.next = Some(Box::new(PRNG::new()));
        }
        return self.next.as_mut().unwrap();
    }
}