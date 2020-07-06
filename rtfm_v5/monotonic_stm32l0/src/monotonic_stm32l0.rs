//! Using STM32L0 as monotonic timer.
//!
//! Note: The STM32L0 only has 16 bit timers. But we can link together two such
//! timers to form a 32 bit timer.

// TODO: Correctness / bounds docs for Instant / Duration

use core::u32;
use core::{
    cmp::Ordering,
    convert::{Infallible, TryInto},
    fmt, ops,
};
use rtic::Monotonic;
use rtt_target::rprintln;
use stm32l0xx_hal::{pac, timer::LinkedTimerPair};

/// Implementor of the `rtic::Monotonic` traits and used to consume the timer
/// to not allow for erroneous configuration.
///
/// This uses TIM2/TIM3 internally as linked timers.
pub struct LinkedTim2Tim3;

impl LinkedTim2Tim3 {
    /// Initialize the timer instance.
    pub fn initialize(timer: LinkedTimerPair<pac::TIM2, pac::TIM3>) {
        // Explicitly drop timer instance so it cannot be reused or reconfigured.
        drop(timer);
    }
}

impl Monotonic for LinkedTim2Tim3 {
    type Instant = Instant;

    fn ratio() -> rtic::Fraction {
        // monotonic * fraction = sys clock
        // TODO: Assumes both timer and sysclock clock run at 16 MHz
        rtic::Fraction {
            numerator: 1,
            denominator: 1,
        }
    }

    /// Returns the current time
    ///
    /// # Correctness
    ///
    /// This function is *allowed* to return nonsensical values if called before `reset` is invoked
    /// by the runtime. Therefore application authors should *not* call this function during the
    /// `#[init]` phase.
    fn now() -> Self::Instant {
        Instant::now()
    }

    /// Resets the counter to *zero*
    ///
    /// # Safety
    ///
    /// This function will be called *exactly once* by the RTFM runtime after `#[init]` returns and
    /// before tasks can start; this is also the case in multi-core applications. User code must
    /// *never* call this function.
    unsafe fn reset() {
        rprintln!("LinkedTim2Tim3::reset()");

        let tim_msb = &*pac::TIM3::ptr();
        let tim_lsb = &*pac::TIM2::ptr();

        // Pause
        tim_msb.cr1.modify(|_, w| w.cen().clear_bit());
        tim_lsb.cr1.modify(|_, w| w.cen().clear_bit());
        // Reset counter
        tim_msb.cnt.reset();
        tim_msb.cnt.reset();
        // Continue
        tim_msb.cr1.modify(|_, w| w.cen().set_bit());
        tim_lsb.cr1.modify(|_, w| w.cen().set_bit());
    }

    fn zero() -> Self::Instant {
        Instant { inner: 0 }
    }
}

/// A measurement of the counter. Opaque and useful only with `Duration`.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Instant {
    inner: u32,
}

impl Instant {
    /// Returns an instant corresponding to "now".
    pub fn now() -> Self {
        loop {
            let tim_msb = unsafe { &*pac::TIM3::ptr() };
            let tim_lsb = unsafe { &*pac::TIM2::ptr() };

            let msb = tim_msb.cnt.read().cnt().bits() as u32;
            let lsb = tim_lsb.cnt.read().cnt().bits() as u32;
            let msb_again = tim_msb.cnt.read().cnt().bits() as u32;

            rprintln!("msb {} lsb {} msba {}", msb, lsb, msb_again);

            // Because the timer is still running at high frequency
            // between reading MSB and LSB, it's possible that LSB
            // has already overflowed. Therefore we read MSB again
            // to check that it hasn't changed.
            let msb_again = tim_msb.cnt.read().cnt().bits() as u32;
            if msb == msb_again {
                return Instant {
                    inner: (msb << 16) | lsb,
                };
            }
        }
    }

    /// Returns the amount of time elapsed since this instant was created.
    pub fn elapsed(&self) -> Duration {
        Instant::now() - *self
    }

    /// Returns the underlying count
    pub fn counts(&self) -> u32 {
        self.inner
    }

    /// Returns the amount of time elapsed from another instant to this one.
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        assert!(
            self.inner > earlier.inner,
            "second instant is later than self"
        );
        Duration {
            inner: self.inner - earlier.inner,
        }
    }
}

impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Instant")
            .field(&(self.inner as u32))
            .finish()
    }
}

impl ops::AddAssign<Duration> for Instant {
    fn add_assign(&mut self, dur: Duration) {
        self.inner = self.inner.wrapping_add(dur.inner);
    }
}

impl ops::Add<Duration> for Instant {
    type Output = Self;
    fn add(mut self, dur: Duration) -> Self {
        self += dur;
        self
    }
}

impl ops::SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, dur: Duration) {
        self.inner = self.inner.wrapping_sub(dur.inner);
    }
}

impl ops::Sub<Duration> for Instant {
    type Output = Self;
    fn sub(mut self, dur: Duration) -> Self {
        self -= dur;
        self
    }
}

impl ops::Sub for Instant {
    type Output = Duration;
    fn sub(self, other: Instant) -> Duration {
        self.duration_since(other)
    }
}

impl Ord for Instant {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.inner.cmp(&rhs.inner)
    }
}

impl PartialOrd for Instant {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

/// A `Duration` type to represent a span of time.
#[derive(Clone, Copy, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Duration {
    inner: u32,
}

impl Duration {
    /// Creates a new `Duration` from the specified number of clock cycles
    pub fn from_cycles(cycles: u32) -> Self {
        Duration { inner: cycles }
    }

    /// Returns the total number of clock cycles contained by this `Duration`
    pub fn as_cycles(&self) -> u32 {
        self.inner
    }
}

// Used internally by RTIC to convert the duration into a known type
impl TryInto<u32> for Duration {
    type Error = Infallible;

    fn try_into(self) -> Result<u32, Infallible> {
        Ok(self.as_cycles())
    }
}

impl ops::AddAssign for Duration {
    fn add_assign(&mut self, dur: Duration) {
        self.inner += dur.inner;
    }
}

impl ops::Add for Duration {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Duration {
            inner: self.inner + other.inner,
        }
    }
}

impl ops::Mul<u32> for Duration {
    type Output = Self;
    fn mul(self, other: u32) -> Self {
        Duration {
            inner: self.inner * other,
        }
    }
}

impl ops::MulAssign<u32> for Duration {
    fn mul_assign(&mut self, other: u32) {
        *self = *self * other;
    }
}

impl ops::SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        self.inner -= rhs.inner;
    }
}

impl ops::Sub for Duration {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Duration {
            inner: self.inner - rhs.inner,
        }
    }
}

///// Adds the `secs`, `millis` and `micros` methods to the `u32` type
/////
///// This trait is only available on ARMv7-M
//pub trait U32Ext {
//    /// Converts the `u32` value as seconds into ticks
//    fn secs(self) -> Duration;
//
//    /// Converts the `u32` value as milliseconds into ticks
//    fn millis(self) -> Duration;
//
//    /// Converts the `u32` value as microseconds into ticks
//    fn micros(self) -> Duration;
//}
//
//impl U32Ext for u32 {
//    fn secs(self) -> Duration {
//        self.millis() * 1_000
//    }
//
//    fn millis(self) -> Duration {
//        self.micros() * 1_000
//    }
//
//    fn micros(self) -> Duration {
//        let frac = Tim1::ratio();
//
//        // 64 MHz / fraction / 1_000_000
//        Duration {
//            inner: (64 * frac.denominator * self) / frac.numerator,
//        }
//    }
//}
