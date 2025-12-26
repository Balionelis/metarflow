use crate::models::MetarInfo;
use crate::utils::{degrees_to_cardinal, is_weather_code, decode_weather, celsius_to_fahrenheit};

// fetches the raw metar data from the aviation weather api for a given airport code
pub async fn fetch_metar(icao: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "https://aviationweather.gov/api/data/metar?ids={}&format=raw",
        icao
    );

    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch data: {}", response.status()).into());
    }

    let text = response.text().await?;

    if text.trim().is_empty() {
        return Err(format!("No METAR data found for airport {}", icao).into());
    }

    Ok(text.trim().to_string())
}

// parses a raw metar string and extracts all the weather information into a structured format
pub fn parse_metar(metar: &str, icao: &str) -> MetarInfo {
    // start with a default metar info struct, setting the station code and raw string
    let mut info = MetarInfo {
        station: icao.to_string(),
        raw: metar.to_string(),
        ..Default::default()
    };

    // split the metar string into individual parts by whitespace
    let parts: Vec<&str> = metar.split_whitespace().collect();
    if parts.is_empty() {
        return info;
    }

    // use this index to walk through the parts one by one
    let mut i = 0;

    // skip the "METAR" or "SPECI" prefix if it's there
    if i < parts.len() && (parts[i] == "METAR" || parts[i] == "SPECI") {
        i += 1;
    }

    // the next part should be the station code
    if i < parts.len() {
        info.station = parts[i].to_string();
        i += 1;
    }

    // look for the date/time stamp which is 7 characters ending with Z
    if i < parts.len() && parts[i].len() == 7 && parts[i].ends_with('Z') {
        let dt = parts[i];
        // parse out the day (first 2 digits), hour (next 2), and minute (next 2)
        if let (Ok(day), Ok(hour), Ok(min)) = (
            dt[0..2].parse::<u32>(),
            dt[2..4].parse::<u32>(),
            dt[4..6].parse::<u32>(),
        ) {
            info.zulu_day = Some(day);
            info.zulu_hour = Some(hour);
            info.zulu_minute = Some(min);
            info.date_time = format!("Day {}, {}:{:02}Z", day, hour, min);
        }
        i += 1;
    }

    // skip any modifiers like COR (corrected), AUTO (automatic), or NIL (no data)
    while i < parts.len() && (parts[i] == "COR" || parts[i] == "AUTO" || parts[i] == "NIL") {
        i += 1;
    }

    // parse the wind information
    if i < parts.len() {
        let wind = parts[i];
        // check if it's variable wind
        if wind.starts_with("VRB") {
            // extract the wind speed
            if let Ok(speed) = wind[3..5].parse::<u32>() {
                info.wind = format!("Variable at {} knots", speed);
                // check for gusts
                if wind.contains('G') {
                    if let Some(gust_pos) = wind.find('G') {
                        if let Ok(gust) = wind[gust_pos + 1..gust_pos + 3].parse::<u32>() {
                            info.wind = format!("Variable at {} knots, gusting to {} knots", speed, gust);
                        }
                    }
                }
                i += 1;
            }
        } else if wind.len() >= 7 && wind.ends_with("KT") {
            // normal wind format: direction (3 digits) + speed (2 digits) + "KT"
            // like "27015KT" means 270 degrees at 15 knots
            if let Ok(dir) = wind[0..3].parse::<u32>() {
                if let Ok(speed) = wind[3..5].parse::<u32>() {
                    let dir_cardinal = degrees_to_cardinal(dir);
                    // check for gusts
                    if wind.contains('G') {
                        if let Some(gust_pos) = wind.find('G') {
                            if let Ok(gust) = wind[gust_pos + 1..gust_pos + 3].parse::<u32>() {
                                info.wind = format!("{} degrees ({}) at {} knots, gusting to {} knots", dir, dir_cardinal, speed, gust);
                            } else {
                                info.wind = format!("{} degrees ({}) at {} knots", dir, dir_cardinal, speed);
                            }
                        }
                    } else {
                        info.wind = format!("{} degrees ({}) at {} knots", dir, dir_cardinal, speed);
                    }
                    i += 1;
                    
                    // check for variable wind direction (like "200V250" meaning wind varies between 200 and 250 degrees)
                    if i < parts.len() {
                        let var_wind = parts[i];
                        if var_wind.contains('V') && var_wind.len() >= 5 {
                            if let Some(v_pos) = var_wind.find('V') {
                                if let (Ok(from_dir), Ok(to_dir)) = (
                                    var_wind[0..v_pos].parse::<u32>(),
                                    var_wind[v_pos + 1..].parse::<u32>(),
                                ) {
                                    info.wind.push_str(&format!(", variable between {} and {} degrees", from_dir, to_dir));
                                    i += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // check for CAVOK (ceiling and visibility okay) this means perfect conditions
    let mut cavok_found = false;
    if i < parts.len() && parts[i] == "CAVOK" {
        info.visibility = "10 kilometers or more".to_string();
        info.clouds = "No clouds below 5,000 feet".to_string();
        info.weather = "None significant".to_string();
        cavok_found = true;
        i += 1;
    }

    // parse visibility if we didn't find CAVOK
    if !cavok_found && i < parts.len() {
        let vis = parts[i];
        // "9999" means 10km or more, "SM" means statute miles
        if vis == "9999" || vis.ends_with("SM") {
            if vis == "9999" {
                info.visibility = "10 kilometers or more".to_string();
            } else if let Ok(miles) = vis[0..vis.len() - 2].parse::<f32>() {
                info.visibility = format!("{} statute miles", miles);
            }
            i += 1;
        } else if vis.parse::<u32>().is_ok() {
            // visibility in meters
            if let Ok(meters) = vis.parse::<u32>() {
                if meters >= 1000 {
                    info.visibility = format!("{} kilometers", meters / 1000);
                } else {
                    info.visibility = format!("{} meters", meters);
                }
            }
            i += 1;
        }
    }

    // parse weather conditions
    if !cavok_found {
        while i < parts.len() {
            let part = parts[i];

            // stop if we hit cloud information
            if part.starts_with("SKC") || part.starts_with("CLR") || part.starts_with("FEW") 
                || part.starts_with("SCT") || part.starts_with("BKN") || part.starts_with("OVC")
                || part.starts_with("VV") {
                break;
            }

            // check if this part is a weather code (starts with +/- for intensity, or contains weather codes)
            let is_weather = part.starts_with('-') || part.starts_with('+') || part.starts_with("VC")
                || is_weather_code(part);

            if is_weather {
                // decode the weather code to readable text
                let weather_desc = decode_weather(part);
                if !weather_desc.is_empty() {
                    if !info.weather.is_empty() {
                        info.weather.push_str(", ");
                    }
                    info.weather.push_str(&weather_desc);
                }
                i += 1;
            } else {
                break;
            }
        }

        if info.weather.is_empty() {
            info.weather = "None".to_string();
        }
    }

    // parse cloud information
    if !cavok_found {
        let mut cloud_layers = Vec::new();
        while i < parts.len() {
            let part = parts[i];
            // sky clear
            if part.starts_with("SKC") {
                cloud_layers.push("Sky clear".to_string());
                i += 1;
                break;
            } else if part.starts_with("CLR") {
                // clear below 12,000 feet
                cloud_layers.push("Clear below 12,000 feet".to_string());
                i += 1;
                break;
            } else if part.starts_with("NSC") {
                // no significant cloud
                cloud_layers.push("No significant cloud".to_string());
                i += 1;
                break;
            } else if part.starts_with("NCD") {
                // no cloud detected
                cloud_layers.push("No cloud detected".to_string());
                i += 1;
                break;
            } else if part.starts_with("VV") {
                // vertical visibility (sky obscured)
                if part.len() >= 5 {
                    if let Ok(alt) = part[2..5].parse::<u32>() {
                        let altitude = alt * 100; // altitude is in hundreds of feet
                        cloud_layers.push(format!("Sky obscured, vertical visibility {} feet", altitude));
                    }
                } else {
                    cloud_layers.push("Sky obscured".to_string());
                }
                i += 1;
            } else if part.starts_with("FEW") || part.starts_with("SCT") 
                || part.starts_with("BKN") || part.starts_with("OVC") {
                // cloud coverage codes: FEW (few), SCT (scattered), BKN (broken), OVC (overcast)
                let coverage = match &part[0..3] {
                    "FEW" => "Few",
                    "SCT" => "Scattered",
                    "BKN" => "Broken",
                    "OVC" => "Overcast",
                    _ => "",
                };
                if !coverage.is_empty() && part.len() >= 6 {
                    // extract altitude (in hundreds of feet)
                    if let Ok(alt) = part[3..6].parse::<u32>() {
                        let altitude = alt * 100;
                        // check for special cloud types
                        let cloud_type = if part.ends_with("CB") {
                            " (cumulonimbus)"
                        } else if part.ends_with("TCU") {
                            " (towering cumulus)"
                        } else {
                            ""
                        };
                        cloud_layers.push(format!("{} at {} feet{}", coverage, altitude, cloud_type));
                    }
                }
                i += 1;
            } else if part.starts_with("A") || part.starts_with("Q") || part.starts_with("T") 
                || part.starts_with("M") || part.starts_with("RMK") || part.starts_with("NOSIG") {
                break;
            } else if part.contains('/') && part.len() <= 7 {
                break;
            } else {
                i += 1;
            }
        }

        if cloud_layers.is_empty() {
            info.clouds = "No cloud information".to_string();
        } else {
            info.clouds = cloud_layers.join(", ");
        }
    }

    // parse temperature, dewpoint, altimeter, and remarks
    while i < parts.len() {
        let part = parts[i];

        // temperature and dewpoint are in format like "15/10" or "M05/M10" (M means negative)
        if part.contains('/') && part.len() <= 7 {
            let temp_parts: Vec<&str> = part.split('/').collect();
            if temp_parts.len() == 2 {
                let mut temp_str = temp_parts[0];
                let mut temp_neg = false;
                // check for negative temperature indicator
                if temp_str.starts_with('M') {
                    temp_neg = true;
                    temp_str = &temp_str[1..];
                } else if temp_str.starts_with('T') {
                    // T means trace, just skip it
                    temp_str = &temp_str[1..];
                }

                let mut dew_str = temp_parts[1];
                let mut dew_neg = false;
                // check for negative dewpoint
                if dew_str.starts_with('M') {
                    dew_neg = true;
                    dew_str = &dew_str[1..];
                }

                // parse both temperatures
                if let (Ok(temp_c), Ok(dew_c)) = (temp_str.parse::<i32>(), dew_str.parse::<i32>()) {
                    let temp_c = if temp_neg { -temp_c } else { temp_c };
                    let temp_f = celsius_to_fahrenheit(temp_c);
                    info.temperature = format!("{}°C ({}°F)", temp_c, temp_f);

                    let dew_c = if dew_neg { -dew_c } else { dew_c };
                    let dew_f = celsius_to_fahrenheit(dew_c);
                    info.dewpoint = format!("{}°C ({}°F)", dew_c, dew_f);

                    i += 1;
                    continue;
                }
            }
        }

        // altimeter setting "A" prefix means inches of mercury
        if part.starts_with("A") && part.len() == 5 {
            if let Ok(inches_raw) = part[1..].parse::<f32>() {
                let inches = inches_raw / 100.0; // convert from hundredths
                let inches_str = format!("{:.2}", inches);
                let hpa = (inches * 33.8639) as u32; // convert to hectopascals
                info.altimeter = format!("{} inches of mercury", inches_str);
                info.altimeter_inches = Some(inches);
                info.altimeter_hpa = Some(hpa);
                info.altimeter_default_unit = "inches".to_string();
            }
            i += 1;
        } else if part.starts_with("Q") && part.len() == 5 {
            // "Q" prefix means hectopascals
            if let Ok(hpa) = part[1..].parse::<u32>() {
                let inches = hpa as f32 / 33.8639; // convert to inches
                info.altimeter = format!("{} hectopascals", hpa);
                info.altimeter_hpa = Some(hpa);
                info.altimeter_inches = Some(inches);
                info.altimeter_default_unit = "hpa".to_string();
            }
            i += 1;
        } else if part.starts_with("RMK") {
            // remarks section - contains additional information
            i += 1;
            let mut remark_parts = Vec::new();
            // parse remarks until we hit "$" or end of string
            while i < parts.len() && parts[i] != "$" {
                let remark = parts[i];
                if remark.starts_with("AO") {
                    remark_parts.push("Automated station".to_string());
                } else if remark.starts_with("RAE") {
                    // rain ended at X minutes past the hour
                    if let Ok(min) = remark[3..].parse::<u32>() {
                        remark_parts.push(format!("Rain ended at {} minutes past the hour", min));
                    }
                } else if remark.starts_with("P") && remark.len() > 1 {
                    // precipitation amount
                    if let Ok(precip) = remark[1..].parse::<f32>() {
                        if precip == 0.0 {
                            remark_parts.push("No precipitation in past hour".to_string());
                        } else {
                            remark_parts.push(format!("Precipitation: {} inches", precip / 100.0));
                        }
                    }
                } else if remark.starts_with("T") && remark.len() > 1 && remark.contains('/') {
                    // precise temperature/dewpoint (in tenths of degrees)
                    let temp_parts: Vec<&str> = remark[1..].split('/').collect();
                    if temp_parts.len() == 2 {
                        if let (Ok(temp_int), Ok(dew_int)) = (temp_parts[0].parse::<i32>(), temp_parts[1].parse::<i32>()) {
                            let temp_c = temp_int as f32 / 10.0;
                            let dew_c = dew_int as f32 / 10.0;
                            remark_parts.push(format!("Precise temperature: {:.1}°C / {:.1}°C", temp_c, dew_c));
                        }
                    }
                } else if remark == "$" {
                    // maintenance indicator
                    remark_parts.push("Maintenance needed on automated station".to_string());
                }
                i += 1;
            }
            if !remark_parts.is_empty() {
                info.remarks = remark_parts.join(". ");
            }
            break;
        } else if part.starts_with("NOSIG") || part == "$" {
            // no significant changes or maintenance indicator
            i += 1;
        } else {
            i += 1;
        }
    }

    info
}

