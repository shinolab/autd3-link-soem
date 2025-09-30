// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State(ec_state);

impl State {
    pub const NONE: Self = Self(ec_state_EC_STATE_NONE);
    pub const INIT: Self = Self(ec_state_EC_STATE_INIT);
    pub const PRE_OP: Self = Self(ec_state_EC_STATE_PRE_OP);
    pub const SAFE_OP: Self = Self(ec_state_EC_STATE_SAFE_OP);
    pub const OPERATIONAL: Self = Self(ec_state_EC_STATE_OPERATIONAL);

    pub const fn state(self) -> u16 {
        self.0 as _
    }

    pub fn is_none(self) -> bool {
        self.0 == ec_state_EC_STATE_NONE
    }

    pub fn is_some(self) -> bool {
        self.0 > ec_state_EC_STATE_NONE
    }

    pub fn is_safe_op(self) -> bool {
        (self.0 & !ec_state_EC_STATE_ERROR) == ec_state_EC_STATE_SAFE_OP
    }

    pub fn is_error(self) -> bool {
        (self.0 & ec_state_EC_STATE_ERROR) != 0
    }
}

impl From<u16> for State {
    fn from(state: u16) -> Self {
        Self(state as _)
    }
}

impl std::fmt::Display for State {
    #[allow(non_upper_case_globals)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 & !ec_state_EC_STATE_ERROR {
            ec_state_EC_STATE_NONE => write!(f, "None")?,
            ec_state_EC_STATE_INIT => write!(f, "Init")?,
            ec_state_EC_STATE_PRE_OP => write!(f, "Pre-op")?,
            ec_state_EC_STATE_SAFE_OP => write!(f, "Safe-op")?,
            ec_state_EC_STATE_OPERATIONAL => write!(f, "Operational")?,
            _ => {
                return write!(f, "Unknown ({})", self.0);
            }
        };
        if (self.0 & ec_state_EC_STATE_ERROR) != 0 {
            write!(f, " + Error")
        } else {
            Ok(())
        }
    }
}
