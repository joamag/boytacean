use std::collections::HashMap;

#[derive(Clone)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct GameShark {
    /// Hash map that contains the complete set of Game Shark
    /// codes that have been registered for the current ROM.
    /// These codes are going to apply a series of patches to
    /// the RAM effectively allowing the user to cheat.
    codes: HashMap<u16, GameSharkCode>,
}

#[derive(Clone)]
pub struct GameSharkCode {
    /// The Game Genie code that is going to be applied to the ROM.
    code: String,

    /// The RAM bank that the cheat code is going to be applied to,
    /// allowing advanced MBCs to be patched.
    ram_bank: u16,

    /// The new data that is going to be written to the address.
    new_data: u8,

    /// Address of the data that is going to be patched.
    addr: u16,
}
