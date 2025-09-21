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
