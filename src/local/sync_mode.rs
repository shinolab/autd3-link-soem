/// Deprecated: This option is no longer used and has no effect.
#[deprecated(since = "33.1.0", note = "This setting no longer has any effect")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum SyncMode {
    /// DC
    DC = 0,
    /// FreeRun
    FreeRun = 1,
}
