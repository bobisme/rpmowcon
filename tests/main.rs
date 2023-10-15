#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use rp_pico as _;

#[cfg_attr(test, defmt_test::tests)]
mod tests {
    use defmt::assert_eq;

    #[init]
    fn init() {}

    #[test]
    fn it_works() {
        assert_eq!(1 + 1, 2);
    }
}
