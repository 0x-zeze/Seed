use cosmwasm_std::Addr;

/// Injective addresses are 20 bytes. `MockApi::addr_make` emits 32, which
/// rejects when a derivative order builds a subaccount id.
pub fn addr20(label: &str) -> Addr {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(label.as_bytes());
    let prefix = bech32::Hrp::parse("inj").unwrap();
    Addr::unchecked(bech32::encode::<bech32::Bech32>(prefix, &digest[..20]).unwrap())
}

/// Default Injective subaccount: `0x` plus the lowercase hex of the
/// bech32-decoded address, plus 24 zero hex chars (nonce 0).
pub fn default_subaccount_id(addr: &Addr) -> Result<String, bech32::DecodeError> {
    let (_, data) = bech32::decode(addr.as_str())?;
    Ok(format!("0x{}{}", hex::encode(data), "0".repeat(24)))
}

#[cfg(test)]
mod tests {
    use super::{addr20, default_subaccount_id};

    #[test]
    fn default_subaccount_is_address_hex_plus_24_zeroes() {
        let addr = addr20("vault");
        let id = default_subaccount_id(&addr).unwrap();
        let (_, data) = bech32::decode(addr.as_str()).unwrap();
        assert_eq!(id, format!("0x{}{}", hex::encode(data), "0".repeat(24)));
        assert!(id.ends_with(&"0".repeat(24)));
        assert_eq!(id.len(), 2 + 40 + 24);
    }
}
