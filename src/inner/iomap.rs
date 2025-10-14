// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use autd3_core::{
    ethercat::{EC_INPUT_FRAME_SIZE, EC_OUTPUT_FRAME_SIZE},
    link::{RxMessage, TxMessage},
};

pub struct IOMap {
    buf: Vec<u8>,
    num_devices: usize,
}

impl std::ops::Deref for IOMap {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl IOMap {
    pub fn new(num_devices: usize) -> Self {
        let size = (1 + EC_OUTPUT_FRAME_SIZE + EC_INPUT_FRAME_SIZE) * num_devices;
        Self {
            buf: vec![0x00; size],
            num_devices,
        }
    }

    pub fn input(&self) -> &[RxMessage] {
        unsafe {
            std::slice::from_raw_parts(
                self.buf[self.num_devices * EC_OUTPUT_FRAME_SIZE..].as_ptr() as *const RxMessage,
                self.num_devices,
            )
        }
    }

    pub fn copy_from(&mut self, tx: &[TxMessage]) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                tx.as_ptr() as *const u8,
                self.buf[0..].as_mut_ptr(),
                std::mem::size_of_val(tx),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iomap() {
        let mut iomap = IOMap::new(1);
        let mut tx = vec![TxMessage::new(); 1];
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
