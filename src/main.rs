use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::Deserialize;

#[derive(Default)]
struct MetarInfo {
    station: String,
    date_time: String,
    wind: String,
    visibility: String,
    weather: String,
    clouds: String,
    temperature: String,
    dewpoint: String,
    altimeter: String,
    remarks: String,
    raw: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/metar", get(fetch_metar_handler))
        .route("/privacy", get(privacy));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

// serves the home page
async fn index() -> Html<&'static str> {
    Html(include_str!("../templates/index.html"))
}

// serves the privacy page
async fn privacy() -> Html<&'static str> {
    Html(include_str!("../templates/privacy.html"))
}

#[derive(Deserialize)]
struct MetarQuery {
    icao: String,
}

// handles the metar search request
async fn fetch_metar_handler(Query(params): Query<MetarQuery>) -> impl IntoResponse {
    let icao = params.icao.trim().to_uppercase();

    if icao.len() != 4 {
        return (
            StatusCode::BAD_REQUEST,
            Html(include_str!("../templates/error.html").replace("{{ERROR}}", "ICAO codes should be 4 characters (e.g., KJFK, EGLL, YSSY)")),
        )
            .into_response();
    }

    match fetch_metar(&icao).await {
        Ok(metar) => {
            let info = parse_metar(&metar, &icao);
            let html = format_results_page(&info);
            Html(html).into_response()
        }
        Err(e) => {
            let html = include_str!("../templates/error.html").replace("{{ERROR}}", &format!("Error fetching METAR: {}", e));
            (StatusCode::INTERNAL_SERVER_ERROR, Html(html)).into_response()
        }
    }
}

// fetches raw metar data from aviationweather.gov
async fn fetch_metar(icao: &str) -> Result<String, Box<dyn std::error::Error>> {
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

// parses raw metar string into human readable info
fn parse_metar(metar: &str, icao: &str) -> MetarInfo {
    let mut info = MetarInfo {
        station: icao.to_string(),
        raw: metar.to_string(),
        ..Default::default()
    };

    let parts: Vec<&str> = metar.split_whitespace().collect();
    if parts.is_empty() {
        return info;
    }

    let mut i = 0;

    if i < parts.len() && (parts[i] == "METAR" || parts[i] == "SPECI") {
        i += 1;
    }

    if i < parts.len() {
        info.station = parts[i].to_string();
        i += 1;
    }

    if i < parts.len() && parts[i].len() == 7 && parts[i].ends_with('Z') {
        let dt = parts[i];
        if let (Ok(day), Ok(hour), Ok(min)) = (
            dt[0..2].parse::<u32>(),
            dt[2..4].parse::<u32>(),
            dt[4..6].parse::<u32>(),
        ) {
            info.date_time = format!("Day {}, {}:{:02} UTC", day, hour, min);
        }
        i += 1;
    }

    if i < parts.len() {
        let wind = parts[i];
        if wind.starts_with("VRB") {
            if let Ok(speed) = wind[3..5].parse::<u32>() {
                info.wind = format!("Variable at {} knots", speed);
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
            if let Ok(dir) = wind[0..3].parse::<u32>() {
                if let Ok(speed) = wind[3..5].parse::<u32>() {
                    let dir_cardinal = degrees_to_cardinal(dir);
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
                }
            }
        }
    }

    let mut cavok_found = false;
    if i < parts.len() && parts[i] == "CAVOK" {
        info.visibility = "10 kilometers or more".to_string();
        info.clouds = "No clouds below 5,000 feet".to_string();
        info.weather = "None significant".to_string();
        cavok_found = true;
        i += 1;
    }

    if !cavok_found && i < parts.len() {
        let vis = parts[i];
        if vis == "9999" || vis.ends_with("SM") {
            if vis == "9999" {
                info.visibility = "10 kilometers or more".to_string();
            } else if let Ok(miles) = vis[0..vis.len() - 2].parse::<f32>() {
                info.visibility = format!("{} statute miles", miles);
            }
            i += 1;
        } else if vis.parse::<u32>().is_ok() {
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

    if !cavok_found {
        while i < parts.len() {
            let part = parts[i];

            if part.starts_with("SKC") || part.starts_with("CLR") || part.starts_with("FEW") 
                || part.starts_with("SCT") || part.starts_with("BKN") || part.starts_with("OVC")
                || part.starts_with("VV") {
                break;
            }

            let is_weather = part.starts_with('-') || part.starts_with('+') || part.starts_with("VC")
                || is_weather_code(part);

            if is_weather {
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

    if !cavok_found {
        let mut cloud_layers = Vec::new();
        while i < parts.len() {
            let part = parts[i];
            if part.starts_with("SKC") {
                cloud_layers.push("Sky clear".to_string());
                i += 1;
                break;
            } else if part.starts_with("CLR") {
                cloud_layers.push("Clear below 12,000 feet".to_string());
                i += 1;
                break;
            } else if part.starts_with("NSC") {
                cloud_layers.push("No significant cloud".to_string());
                i += 1;
                break;
            } else if part.starts_with("NCD") {
                cloud_layers.push("No cloud detected".to_string());
                i += 1;
                break;
            } else if part.starts_with("VV") {
                if part.len() >= 5 {
                    if let Ok(alt) = part[2..5].parse::<u32>() {
                        let altitude = alt * 100;
                        cloud_layers.push(format!("Sky obscured, vertical visibility {} feet", altitude));
                    }
                } else {
                    cloud_layers.push("Sky obscured".to_string());
                }
                i += 1;
            } else if part.starts_with("FEW") || part.starts_with("SCT") 
                || part.starts_with("BKN") || part.starts_with("OVC") {
                let coverage = match &part[0..3] {
                    "FEW" => "Few",
                    "SCT" => "Scattered",
                    "BKN" => "Broken",
                    "OVC" => "Overcast",
                    _ => "",
                };
                if !coverage.is_empty() && part.len() >= 6 {
                    if let Ok(alt) = part[3..6].parse::<u32>() {
                        let altitude = alt * 100;
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

    while i < parts.len() {
        let part = parts[i];

        if part.contains('/') && part.len() <= 7 {
            let temp_parts: Vec<&str> = part.split('/').collect();
            if temp_parts.len() == 2 {
                let mut temp_str = temp_parts[0];
                let mut temp_neg = false;
                if temp_str.starts_with('M') {
                    temp_neg = true;
                    temp_str = &temp_str[1..];
                } else if temp_str.starts_with('T') {
                    temp_str = &temp_str[1..];
                }

                let mut dew_str = temp_parts[1];
                let mut dew_neg = false;
                if dew_str.starts_with('M') {
                    dew_neg = true;
                    dew_str = &dew_str[1..];
                }

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

        if part.starts_with("A") && part.len() == 5 {
            if let Ok(inches) = part[1..].parse::<f32>() {
                let inches_str = format!("{:.2}", inches / 100.0);
                info.altimeter = format!("{} inches of mercury", inches_str);
            }
            i += 1;
        } else if part.starts_with("Q") && part.len() == 5 {
            if let Ok(hpa) = part[1..].parse::<u32>() {
                info.altimeter = format!("{} hectopascals", hpa);
            }
            i += 1;
        } else if part.starts_with("RMK") {
            i += 1;
            let mut remark_parts = Vec::new();
            while i < parts.len() && parts[i] != "$" {
                let remark = parts[i];
                if remark.starts_with("AO") {
                    remark_parts.push("Automated station".to_string());
                } else if remark.starts_with("RAE") {
                    if let Ok(min) = remark[3..].parse::<u32>() {
                        remark_parts.push(format!("Rain ended at {} minutes past the hour", min));
                    }
                } else if remark.starts_with("P") && remark.len() > 1 {
                    if let Ok(precip) = remark[1..].parse::<f32>() {
                        if precip == 0.0 {
                            remark_parts.push("No precipitation in past hour".to_string());
                        } else {
                            remark_parts.push(format!("Precipitation: {} inches", precip / 100.0));
                        }
                    }
                } else if remark.starts_with("T") && remark.len() > 1 && remark.contains('/') {
                    let temp_parts: Vec<&str> = remark[1..].split('/').collect();
                    if temp_parts.len() == 2 {
                        if let (Ok(temp_int), Ok(dew_int)) = (temp_parts[0].parse::<i32>(), temp_parts[1].parse::<i32>()) {
                            let temp_c = temp_int as f32 / 10.0;
                            let dew_c = dew_int as f32 / 10.0;
                            remark_parts.push(format!("Precise temperature: {:.1}°C / {:.1}°C", temp_c, dew_c));
                        }
                    }
                } else if remark == "$" {
                    remark_parts.push("Maintenance needed on automated station".to_string());
                }
                i += 1;
            }
            if !remark_parts.is_empty() {
                info.remarks = remark_parts.join(". ");
            }
            break;
        } else if part.starts_with("NOSIG") || part == "$" {
            i += 1;
        } else {
            i += 1;
        }
    }

    info
}

// converts wind degrees to cardinal direction
fn degrees_to_cardinal(degrees: u32) -> &'static str {
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

// checks if a string contains a known weather code
fn is_weather_code(code: &str) -> bool {
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

// decodes weather codes like -SN or +RA into readable text
fn decode_weather(code: &str) -> String {
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

// converts celsius to fahrenheit
fn celsius_to_fahrenheit(celsius: i32) -> i32 {
    (celsius * 9 / 5) + 32
}

// generates the html results page
fn format_results_page(info: &MetarInfo) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>metarflow - METAR Weather Viewer</title>
    <style>
        body {{
            font-family: monospace;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            line-height: 1.6;
        }}
        h1 {{
            border-bottom: 1px solid #000;
            padding-bottom: 10px;
        }}
        .stats {{
            margin: 30px 0;
        }}
        .stat-row {{
            margin: 15px 0;
            padding: 10px 0;
            border-bottom: 1px solid #ddd;
        }}
        .stat-label {{
            font-weight: bold;
            display: inline-block;
            width: 150px;
        }}
        .stat-value {{
            display: inline-block;
        }}
        .raw-metar {{
            margin: 30px 0;
        }}
        pre {{
            background: #f5f5f5;
            padding: 15px;
            border: 1px solid #000;
            overflow-x: auto;
        }}
        a {{
            color: #000;
            text-decoration: underline;
        }}
        form {{
            margin: 30px 0;
        }}
        label {{
            display: block;
            margin-bottom: 5px;
        }}
        input[type="text"] {{
            padding: 8px;
            font-family: monospace;
            font-size: 14px;
            width: 200px;
            border: 1px solid #000;
        }}
        button {{
            padding: 8px 16px;
            font-family: monospace;
            font-size: 14px;
            border: 1px solid #000;
            background: #fff;
            cursor: pointer;
        }}
        button:hover {{
            background: #f0f0f0;
        }}
        footer {{
            margin-top: 50px;
            padding-top: 20px;
            border-top: 1px solid #ddd;
            text-align: center;
            font-size: 12px;
        }}
        footer .disclaimer {{
            color: #ff0000;
        }}
        footer .credits {{
            color: #000;
        }}
        footer a {{
            color: #000;
            text-decoration: underline;
        }}
        .github-icon {{
            width: 14px;
            height: 14px;
            vertical-align: -2px;
            margin-right: 4px;
            display: inline-block;
        }}
        @media (max-width: 600px) {{
            body {{
                padding: 10px;
                margin: 20px auto;
            }}
            input[type="text"] {{
                width: 100%;
                max-width: 200px;
            }}
            button {{
                width: 100%;
                max-width: 200px;
            }}
            h1 {{
                font-size: 1.5em;
            }}
            .stat-label {{
                width: 120px;
                font-size: 0.9em;
            }}
            .stat-value {{
                font-size: 0.9em;
                word-break: break-word;
            }}
            pre {{
                font-size: 11px;
                padding: 10px;
                overflow-x: auto;
            }}
            footer {{
                font-size: 11px;
            }}
        }}
    </style>
</head>
<body>
    <h1>metarflow</h1>
    
    <div class="stats">
        <div class="stat-row">
            <span class="stat-label">Airport:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Date/Time:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Wind:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Visibility:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Weather:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Clouds:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Temperature:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Dewpoint:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Altimeter:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Remarks:</span>
            <span class="stat-value">{}</span>
        </div>
    </div>

    <div class="raw-metar">
        <h2>Raw METAR</h2>
        <pre>{}</pre>
    </div>

    <form action="/metar" method="get">
        <label for="icao">Enter airport ICAO code:</label>
        <input type="text" id="icao" name="icao" placeholder="e.g., KJFK" maxlength="4" required>
        <button type="submit">Fetch METAR</button>
    </form>
    <footer>
        <p class="disclaimer">The information on this website is intended for educational purposes only. Not for operational use.</p>
        <p class="credits">Created by <a href="https://github.com/Balionelis/metarflow" target="_blank" rel="noopener noreferrer"><svg class="github-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.012 8.012 0 0 0 16 8c0-4.42-3.58-8-8-8z"/></svg>Balionelis</a>. Licensed under <a href="https://github.com/Balionelis/metarflow/blob/main/LICENSE" target="_blank" rel="noopener noreferrer">GPL-3.0</a>. <a href="/privacy">Privacy</a>.</p>
    </footer>
</body>
</html>
        "#,
        info.station,
        if info.date_time.is_empty() { "N/A" } else { &info.date_time },
        if info.wind.is_empty() { "N/A" } else { &info.wind },
        if info.visibility.is_empty() { "N/A" } else { &info.visibility },
        if info.weather.is_empty() { "N/A" } else { &info.weather },
        if info.clouds.is_empty() { "N/A" } else { &info.clouds },
        if info.temperature.is_empty() { "N/A" } else { &info.temperature },
        if info.dewpoint.is_empty() { "N/A" } else { &info.dewpoint },
        if info.altimeter.is_empty() { "N/A" } else { &info.altimeter },
        if info.remarks.is_empty() { "None" } else { &info.remarks },
        info.raw
    )
}
