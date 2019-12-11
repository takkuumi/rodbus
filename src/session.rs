use crate::channel::Request;
use tokio::sync::{mpsc, oneshot};
use crate::error::{Error, InvalidRequestReason};
use crate::service::traits::Service;
use crate::service::services::{ReadCoils, ReadDiscreteInputs, ReadHoldingRegisters, ReadInputRegisters};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct UnitIdentifier {
    id: u8,
}

pub struct AddressRange {
    pub start: u16,
    pub count: u16
}

impl AddressRange {

    pub const MAX_REGISTERS : u16 = 125;
    pub const MAX_BINARY_BITS : u16 = 2000;

    pub fn new(start: u16, count: u16) -> Self {
        AddressRange { start, count }
    }

    fn check_validity(&self, max_count: u16) -> Result<(), InvalidRequestReason> {
        // a count of zero is never valid
        if self.count == 0 {
            return Err(InvalidRequestReason::CountOfZero);
        }

        // check that start/count don't overflow u16
        let last_address = (self.start as u32) + (self.count as u32 - 1);
        if last_address > (std::u16::MAX as u32) {
            return Err(InvalidRequestReason::AddressOverflow);
        }

        if self.count > max_count {
            return Err(InvalidRequestReason::CountTooBigForType);
        }

        Ok(())
    }

    pub fn check_validity_for_bits(&self) -> Result<(), InvalidRequestReason> {
        self.check_validity(Self::MAX_BINARY_BITS)
    }

    pub fn check_validity_for_registers(&self) -> Result<(), InvalidRequestReason> {
        self.check_validity(Self::MAX_REGISTERS)
    }
}

pub struct Indexed<T> {
    pub index: u16,
    pub value: T
}

impl<T> Indexed<T> {
    pub fn new(index: u16, value : T) -> Self {
        Indexed {  index, value }
    }
}

impl UnitIdentifier {
    pub fn new(unit_id: u8) -> Self {
        Self { id: unit_id }
    }

    pub fn default() -> Self {
        Self { id: 0xFF }
    }

    pub fn value(&self) -> u8 {
        self.id
    }
}

pub struct Session {
    id: UnitIdentifier,
    channel_tx: mpsc::Sender<Request>,
}

impl Session {
    pub(crate) fn new(id: UnitIdentifier, channel_tx: mpsc::Sender<Request>) -> Self {
        Session { id, channel_tx }
    }

    async fn make_service_call<S : Service>(&mut self, request: S::Request) -> Result<S::Response, Error> {
        S::check_request_validity(&request)?;
        let (tx, rx) = oneshot::channel::<Result<S::Response, Error>>();
        let request = S::create_request(self.id, request, tx);
        self.channel_tx.send(request).await.map_err(|_| Error::Shutdown)?;
        rx.await.map_err(|_| Error::Shutdown)?
    }

    pub async fn read_coils(&mut self, range: AddressRange) -> Result<Vec<Indexed<bool>>, Error> {
        self.make_service_call::<ReadCoils>(range).await
    }

    pub async fn read_discrete_inputs(&mut self, range: AddressRange) -> Result<Vec<Indexed<bool>>, Error> {
        self.make_service_call::<ReadDiscreteInputs>(range).await
    }

    pub async fn read_holding_registers(&mut self, range: AddressRange) -> Result<Vec<Indexed<u16>>, Error> {
        self.make_service_call::<ReadHoldingRegisters>(range).await
    }

    pub async fn read_input_registers(&mut self, range: AddressRange) -> Result<Vec<Indexed<u16>>, Error> {
        self.make_service_call::<ReadInputRegisters>(range).await
    }
}