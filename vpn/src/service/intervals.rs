use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_std::stream::Stream;
use futures_timer::Delay;

pub fn interval<T>(dur: Duration, val: T) -> Interval<T> {
    Interval {
        delay: Delay::new(dur),
        interval: dur,
        value: Box::new(val),
    }
}

pub struct Interval<T> {
    delay: Delay,
    interval: Duration,
    value: Box<T>,
}

impl<T: std::clone::Clone> Stream for Interval<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Pin::new(&mut self.delay).poll(cx).is_pending() {
            return Poll::Pending;
        }
        let interval = self.interval;
        self.delay.reset(interval);
        Poll::Ready(Some(*self.value.clone()))
    }
}
