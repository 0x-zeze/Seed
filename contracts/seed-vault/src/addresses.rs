use cosmwasm_std::Addr;

/// Injective addresses are 20 bytes. `MockApi::addr_make` emits 32, which
/// `H160` rejects when a derivative order builds a subaccount id.
pub fn addr20(label: &str) -> Addr {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(label.as_bytes());
    let prefix = bech32::Hrp::parse("inj").unwrap();
    Addr::unchecked(bech32::encode::<bech32::Bech32>(prefix, &digest[..20]).unwrap())
}
