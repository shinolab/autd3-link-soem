/// Synchronization mode.
///
/// See [Synchronization modes of an EtherCAT slave](https://infosys.beckhoff.com/english.php?content=../content/1033/ethercatsystem/2469118347.html&id=) for more information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum SyncMode {
    /// DC
    DC = 0,
    /// FreeRun
    FreeRun = 1,
}
