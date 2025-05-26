use autd3_core::{
    ethercat::{EC_INPUT_FRAME_SIZE, EC_OUTPUT_FRAME_SIZE},
    link::{RxMessage, TxMessage},
};

use derive_more::Deref;
use zerocopy::{FromBytes, IntoBytes};

#[derive(Deref)]
pub struct IOMap {
    #[deref]
    buf: Vec<u8>,
    num_devices: usize,
}

impl IOMap {
    pub fn new(num_devices: usize) -> Self {
        let size = (EC_OUTPUT_FRAME_SIZE + EC_INPUT_FRAME_SIZE) * num_devices;
        Self {
            buf: vec![0x00; size],
            num_devices,
        }
    }

    pub fn input(&self) -> &[RxMessage] {
        <[RxMessage]>::ref_from_bytes(&self.buf[self.num_devices * EC_OUTPUT_FRAME_SIZE..]).unwrap()
    }

    pub fn copy_from(&mut self, tx: &[TxMessage]) {
        self.buf[0..tx.as_bytes().len()].copy_from_slice(tx.as_bytes());
    }

    pub fn clear(&mut self) {
        self.buf.fill(0x00);
    }
}

#[cfg(test)]
mod tests {
    use zerocopy::FromZeros;

    use super::*;

    #[test]
    fn test_iomap() {
        let mut iomap = IOMap::new(1);
        let mut tx = vec![TxMessage::new_zeroed(); 1];
        let payload_size = tx[0].payload().len();
        tx[0].header.msg_id = autd3_core::link::MsgId::new(0x01);
        tx[0].header.slot_2_offset = 0x0302;
        tx[0].payload_mut()[0] = 0x04;
        tx[0].payload_mut()[payload_size - 1] = 5;

        iomap.copy_from(&tx);

        assert_eq!(iomap[0], 0x01);
        assert_eq!(iomap[1], 0x00);
        assert_eq!(iomap[2], 0x02);
        assert_eq!(iomap[3], 0x03);
        assert_eq!(iomap[3 + 1], 0x04);
        assert_eq!(iomap[3 + payload_size], 0x05);
    }
}
