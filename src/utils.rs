// converts wind direction in degrees to a cardinal direction like N, NE, E, etc.
pub fn degrees_to_cardinal(degrees: u32) -> &'static str {
    match degrees {
        0..=22 | 338..=360 => "N",
        23..=67 => "NE",
        68..=112 => "E",
        113..=157 => "SE",
        158..=202 => "S",
        203..=247 => "SW",
        248..=292 => "W",
        293..=337 => "NW",
        _ => "",
    }
}

// checks if a string contains a weather code like RA for rain or SN for snow
pub fn is_weather_code(code: &str) -> bool {
    if code.starts_with("SKC") || code.starts_with("CLR") || code.starts_with("FEW") 
        || code.starts_with("SCT") || code.starts_with("BKN") || code.starts_with("OVC")
        || code.starts_with("VV") || code.starts_with("NSC") || code.starts_with("NCD")
        || code.starts_with("A") || code.starts_with("Q") || code.contains('/') {
        return false;
    }

    let weather_codes = [
        "RA", "SN", "DZ", "PL", "GR", "GS", "UP", "IC", "SG",
        "BR", "FG", "FU", "VA", "DU", "SA", "HZ", "PY",
        "SQ", "FC", "SS", "DS", "PO",
        "MI", "BC", "DR", "BL", "SH", "TS", "FZ", "PR",
    ];

    for wx in &weather_codes {
        if code.contains(wx) {
            return true;
        }
    }
    false
}

// takes a weather code like -sn or +ra and converts it to readable text like "Light snow" or "Heavy rain"
pub fn decode_weather(code: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = code.chars().collect();

    if i < chars.len() {
        match chars[i] {
            '-' => {
                result.push_str("Light ");
                i += 1;
            }
            '+' => {
                result.push_str("Heavy ");
                i += 1;
            }
            'V' if i + 1 < chars.len() && chars[i + 1] == 'C' => {
                result.push_str("In vicinity ");
                i += 2;
            }
            _ => {}
        }
    }

    while i < chars.len() && chars[i].is_alphabetic() {
        match &code[i..] {
            s if s.starts_with("MI") => { result.push_str("Shallow "); i += 2; }
            s if s.starts_with("BC") => { result.push_str("Patches "); i += 2; }
            s if s.starts_with("DR") => { result.push_str("Low drifting "); i += 2; }
            s if s.starts_with("BL") => { result.push_str("Blowing "); i += 2; }
            s if s.starts_with("SH") => { result.push_str("Showers "); i += 2; }
            s if s.starts_with("TS") => { result.push_str("Thunderstorm "); i += 2; }
            s if s.starts_with("FZ") => { result.push_str("Freezing "); i += 2; }
            s if s.starts_with("PR") => { result.push_str("Partial "); i += 2; }
            _ => break,
        }
    }

    while i < chars.len() && chars[i].is_alphabetic() {
        match chars[i] {
            'D' if i + 1 < chars.len() && chars[i + 1] == 'Z' => { result.push_str("drizzle"); i += 2; }
            'R' if i + 1 < chars.len() && chars[i + 1] == 'A' => { result.push_str("rain"); i += 2; }
            'S' if i + 1 < chars.len() && chars[i + 1] == 'N' => { result.push_str("snow"); i += 2; }
            'S' if i + 1 < chars.len() && chars[i + 1] == 'G' => { result.push_str("snow grains"); i += 2; }
            'I' if i + 1 < chars.len() && chars[i + 1] == 'C' => { result.push_str("ice crystals"); i += 2; }
            'G' if i + 1 < chars.len() && chars[i + 1] == 'R' => { result.push_str("hail"); i += 2; }
            'P' if i + 1 < chars.len() && chars[i + 1] == 'L' => { result.push_str("ice pellets"); i += 2; }
            'H' if i + 1 < chars.len() && chars[i + 1] == 'Z' => { result.push_str("haze"); i += 2; }
            'F' if i + 1 < chars.len() && chars[i + 1] == 'U' => { result.push_str("smoke"); i += 2; }
            'F' if i + 1 < chars.len() && chars[i + 1] == 'G' => { result.push_str("fog"); i += 2; }
            'B' if i + 1 < chars.len() && chars[i + 1] == 'R' => { result.push_str("mist"); i += 2; }
            'S' if i + 1 < chars.len() && chars[i + 1] == 'Q' => { result.push_str("squalls"); i += 2; }
            'F' if i + 1 < chars.len() && chars[i + 1] == 'C' => { result.push_str("funnel cloud"); i += 2; }
            'G' if i + 1 < chars.len() && chars[i + 1] == 'S' => { result.push_str("small hail/snow pellets"); i += 2; }
            'U' if i + 1 < chars.len() && chars[i + 1] == 'P' => { result.push_str("unknown precipitation"); i += 2; }
            'V' if i + 1 < chars.len() && chars[i + 1] == 'A' => { result.push_str("volcanic ash"); i += 2; }
            'D' if i + 1 < chars.len() && chars[i + 1] == 'U' => { result.push_str("widespread dust"); i += 2; }
            'S' if i + 1 < chars.len() && chars[i + 1] == 'A' => { result.push_str("sand"); i += 2; }
            'P' if i + 1 < chars.len() && chars[i + 1] == 'O' => { result.push_str("dust/sand whirls"); i += 2; }
            'S' if i + 1 < chars.len() && chars[i + 1] == 'S' => { result.push_str("sandstorm"); i += 2; }
            'D' if i + 1 < chars.len() && chars[i + 1] == 'S' => { result.push_str("dust storm"); i += 2; }
            'P' if i + 1 < chars.len() && chars[i + 1] == 'Y' => { result.push_str("spray"); i += 2; }
            _ => break,
        }
    }

    result.trim().to_string()
}

// converts temperature from celsius to fahrenheit
pub fn celsius_to_fahrenheit(celsius: i32) -> i32 {
    (celsius * 9 / 5) + 32
}

// formats a stat value for display, returns a css class and the value to show (or default if empty)
pub fn format_stat_value(value: &str, default: &str) -> (String, String) {
    if value.is_empty() {
        (" empty".to_string(), default.to_string())
    } else {
        ("".to_string(), value.to_string())
    }
}

