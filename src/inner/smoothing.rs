// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

#[derive(Debug, Clone, Copy)]
pub struct Smoothing {
    alpha: f32,
    current: Option<f32>,
}

impl Smoothing {
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha,
            current: None,
        }
    }

    pub fn push(&mut self, value: f32) -> f32 {
        let current = self.current.get_or_insert(value);
        *current = self.alpha * value + (1.0 - self.alpha) * *current;
        *current
    }
}

#[cfg(test)]
mod tests {
    use super::Smoothing;

    #[test]
    fn test_smoothing() {
        let mut smoothing = Smoothing::new(0.2);
        assert_eq!(10.0, smoothing.push(10.0));
        assert_eq!(12.0, smoothing.push(20.0));
        assert_eq!(15.6, smoothing.push(30.0));
        assert_eq!(20.48, smoothing.push(40.0));
    }
}
