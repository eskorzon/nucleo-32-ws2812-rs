use core::ops::Deref;
extern crate nb;
use embassy_stm32::Peripheral;
use embassy_stm32::spi::{Spi, Instance, SckPin, MosiPin, MisoPin, Config, Error};
use embedded_hal::spi::FullDuplex;


pub struct MySpi<'d, T: Instance, Tx, Rx>(Spi<'d, T, Tx, Rx>);

impl<'d, T: Instance, Tx, Rx> MySpi<'d, T, Tx, Rx> {
    pub fn new(
        peri: impl Peripheral<P = T> + 'd,
        sck: impl Peripheral<P = impl SckPin<T>> + 'd,
        mosi: impl Peripheral<P = impl MosiPin<T>> + 'd,
        miso: impl Peripheral<P = impl MisoPin<T>> + 'd,
        txdma: impl Peripheral<P = Tx> + 'd,
        rxdma: impl Peripheral<P = Rx> + 'd,
        config: Config,
    ) -> Self {
        MySpi(Spi::new(peri, sck, mosi, miso, txdma, rxdma, config))
    }
}

impl<'d, T: Instance, Tx, Rx> FullDuplex<u8> for MySpi<'d, T, Tx, Rx> {
    type Error = Error;

    fn read(&mut self) -> Result<u8, nb::Error<Error>> {
        let data: u8 = Default::default();
        self.0.blocking_read(&mut [data])?;

        Ok(data)
    }

    fn send(&mut self, word: u8) -> Result<(), nb::Error<Error>> {
        Ok(self.0.blocking_write(&[word])?)
    }
}


impl<'d, T: Instance, Tx, Rx> Deref for MySpi<'d, T, Tx, Rx> {
    type Target = Spi<'d, T, Tx, Rx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
