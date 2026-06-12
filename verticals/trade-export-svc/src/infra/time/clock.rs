//! System clock implementation of the domain `Clock` port.

use chrono::{DateTime, Utc};

use crate::domain::shared::types::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
