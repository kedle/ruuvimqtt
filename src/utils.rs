use regex::Regex;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Alias {
    pub address: [u8; 6],
    pub name: String,
}

/// Parses a String representation from any format supported
pub fn str_to_mac(s: &str) -> Result<[u8; 6], &'static str> {
    let re = Regex::new("(0x)?([0-9a-fA-F]{1,2})[:.-]?").unwrap();
    let mut mac: [u8; 6] = [0; 6];

    match s.len() {
        11..=17 => {}
        _ => {
            return Err("Invalid mac address");
        }
    }

    let mut i = 0;
    for caps in re.captures_iter(s) {
        // Fill the array and keep counting for InvalidByteCount
        if i < 6 {
            let matched_byte = caps.get(2).unwrap().as_str();
            mac[i] = u8::from_str_radix(matched_byte, 16).unwrap();
        }
        i += 1;
    }

    if i != 6 {
        return Err("Invalid byte count");
    }

    Ok(mac)
}

pub fn parse_alias(src: &str) -> Result<Alias, String> {
    let index = src.find('=');
    match index {
        Some(i) => {
            let (address, name) = src.split_at(i);
            Ok(Alias {
                address: str_to_mac(address).unwrap(),
                name: name.get(1..).unwrap_or("").to_string(),
            })
        }
        None => Err("invalid alias".to_string()),
    }
}

pub fn alias_map(aliases: &Vec<Alias>) -> BTreeMap<[u8; 6], String> {
    let mut map = BTreeMap::new();
    for alias in aliases.iter() {
        map.insert(alias.address, alias.name.to_string());
    }
    map
}
