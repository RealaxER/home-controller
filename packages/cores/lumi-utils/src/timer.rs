use std::time::{SystemTime, UNIX_EPOCH};



pub trait Timer {
    fn now_ms(&mut self) -> u64;
}

#[derive(Debug, Default)]
pub struct SystemTimer {

}

impl Timer for SystemTimer {
    fn now_ms(&mut self) -> u64 {
        let start = SystemTime::now();

        start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backward")
            .as_millis() as u64
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_system_timer() {
        let mut system_timer = SystemTimer::default();
        let now = system_timer.now_ms();

        assert!(0 < now);
    }
}

