use axum::{
    extract::Query,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse},
};
use crate::models::{MetarInfo, MetarQuery};
use crate::services::{fetch_metar, parse_metar};
use crate::utils::format_stat_value;

// serves the home page with the search form
pub async fn index() -> Html<String> {
    let template = include_str!("../templates/index.html");
    Html(template.to_string())
}

// serves the privacy policy page
pub async fn privacy() -> Html<String> {
    let template = include_str!("../templates/privacy.html");
    Html(template.to_string())
}

// serves the favicon svg file
pub async fn favicon() -> impl IntoResponse {
    let svg = include_str!("../metarflow.svg");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/svg+xml"),
    );
    (StatusCode::OK, headers, svg)
}

// handles requests to fetch and display metar data for an airport
pub async fn fetch_metar_handler(
    Query(params): Query<MetarQuery>,
) -> impl IntoResponse {
    let icao = params.icao.trim().to_uppercase();

    if icao.len() != 4 {
        let html = include_str!("../templates/error.html")
            .replace("{{ERROR}}", "ICAO codes should be 4 characters (e.g., KJFK, EGLL, YSSY)");
        return (StatusCode::BAD_REQUEST, Html(html)).into_response();
    }

    match fetch_metar(&icao).await {
        Ok(metar) => {
            let info = parse_metar(&metar, &icao);
            let html = format_results_page(&info);
            Html(html).into_response()
        }
        Err(e) => {
            let html = include_str!("../templates/error.html")
                .replace("{{ERROR}}", &format!("Error fetching METAR: {}", e));
            (StatusCode::INTERNAL_SERVER_ERROR, Html(html)).into_response()
        }
    }
}

// builds the html page that shows all the parsed metar information
pub fn format_results_page(info: &MetarInfo) -> String {
    let (dt_class, dt_value) = format_stat_value(&info.date_time, "N/A");
    let (wind_class, wind_value) = format_stat_value(&info.wind, "N/A");
    let (vis_class, vis_value) = format_stat_value(&info.visibility, "N/A");
    let (wx_class, wx_value) = format_stat_value(&info.weather, "N/A");
    let (clouds_class, clouds_value) = format_stat_value(&info.clouds, "N/A");
    let (temp_class, temp_value) = format_stat_value(&info.temperature, "N/A");
    let (dew_class, dew_value) = format_stat_value(&info.dewpoint, "N/A");
    let (alt_class, alt_value) = format_stat_value(&info.altimeter, "N/A");
    let (rmk_class, rmk_value) = format_stat_value(&info.remarks, "None");
    
    let altimeter_hpa = if let Some(hpa) = info.altimeter_hpa {
        hpa.to_string()
    } else {
        "null".to_string()
    };
    let altimeter_inches = if let Some(inches) = info.altimeter_inches {
        format!("{:.2}", inches)
    } else {
        "null".to_string()
    };
    let altimeter_default = if info.altimeter_default_unit.is_empty() {
        "hpa".to_string()
    } else {
        info.altimeter_default_unit.clone()
    };
    
    let zulu_day = if let Some(day) = info.zulu_day {
        day.to_string()
    } else {
        "null".to_string()
    };
    let zulu_hour = if let Some(hour) = info.zulu_hour {
        hour.to_string()
    } else {
        "null".to_string()
    };
    let zulu_minute = if let Some(min) = info.zulu_minute {
        min.to_string()
    } else {
        "null".to_string()
    };
    
    format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>metarflow - METAR Weather Viewer</title>
    <link rel="icon" type="image/svg+xml" href="/metarflow.svg">
    <style>
        * {{
            transition: background-color 0.3s ease, color 0.3s ease, border-color 0.3s ease;
        }}
        body {{
            font-family: monospace;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            line-height: 1.6;
            background-color: #fff;
            color: #000;
        }}
        body.dark-mode {{
            background-color: #1a1a1a;
            color: #e0e0e0;
        }}
        h1 {{
            border-bottom: 1px solid #000;
            padding-bottom: 10px;
        }}
        body.dark-mode h1 {{
            border-bottom-color: #e0e0e0;
        }}
        .stats {{
            margin: 30px 0;
        }}
        .stat-row {{
            margin: 15px 0;
            padding: 10px 0;
            border-bottom: 1px solid #ddd;
        }}
        body.dark-mode .stat-row {{
            border-bottom-color: #444;
        }}
        .stat-label {{
            font-weight: bold;
            display: inline-block;
            width: 150px;
        }}
        .stat-value {{
            display: inline-block;
        }}
        .stat-value.empty {{
            color: #999;
            font-style: italic;
        }}
        body.dark-mode .stat-value.empty {{
            color: #999;
        }}
        .raw-metar {{
            margin: 30px 0;
        }}
        .raw-metar-header {{
            display: flex;
            align-items: center;
            gap: 10px;
        }}
        .raw-metar-header button {{
            font-size: 12px;
            padding: 4px 8px;
        }}
        .action-buttons {{
            margin: 20px 0;
            display: flex;
            gap: 10px;
            flex-wrap: wrap;
        }}
        .search-container {{
            position: relative;
            display: inline-block;
        }}
        .dropdown {{
            display: none;
            position: absolute;
            top: 100%;
            left: 0;
            margin-top: 5px;
            background: #fff;
            border: 1px solid #000;
            min-width: 200px;
            max-width: 400px;
            z-index: 1000;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        body.dark-mode .dropdown {{
            background: #2a2a2a;
            border-color: #e0e0e0;
        }}
        .dropdown.show {{
            display: block;
        }}
        .dropdown-section {{
            padding: 10px;
            border-bottom: 1px solid #ddd;
        }}
        body.dark-mode .dropdown-section {{
            border-bottom-color: #444;
        }}
        .dropdown-section:last-child {{
            border-bottom: none;
        }}
        .dropdown-section h4 {{
            font-size: 12px;
            margin: 0 0 8px 0;
            font-weight: bold;
            text-transform: uppercase;
        }}
        .dropdown-item {{
            display: block;
            padding: 8px;
            text-decoration: none;
            color: #000;
            border-bottom: 1px solid #f0f0f0;
            cursor: pointer;
        }}
        body.dark-mode .dropdown-item {{
            color: #e0e0e0;
            border-bottom-color: #444;
        }}
        .dropdown-item:last-child {{
            border-bottom: none;
        }}
        .dropdown-item:hover {{
            background: #f0f0f0;
        }}
        body.dark-mode .dropdown-item:hover {{
            background: #3a3a3a;
        }}
        .dropdown-item.empty {{
            color: #666;
            font-style: italic;
            cursor: default;
        }}
        body.dark-mode .dropdown-item.empty {{
            color: #999;
        }}
        .dropdown-item.empty:hover {{
            background: #fff;
        }}
        body.dark-mode .dropdown-item.empty:hover {{
            background: #2a2a2a;
        }}
        pre {{
            background: #f5f5f5;
            padding: 15px;
            border: 1px solid #000;
            overflow-x: auto;
        }}
        body.dark-mode pre {{
            background: #2a2a2a;
            border-color: #e0e0e0;
        }}
        a {{
            color: #000;
            text-decoration: underline;
        }}
        body.dark-mode a {{
            color: #e0e0e0;
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
            background-color: #fff;
            color: #000;
        }}
        body.dark-mode input[type="text"] {{
            border-color: #e0e0e0;
            background-color: #2a2a2a;
            color: #e0e0e0;
        }}
        button {{
            padding: 8px 16px;
            font-family: monospace;
            font-size: 14px;
            border: 1px solid #000;
            background: #fff;
            color: #000;
            cursor: pointer;
        }}
        body.dark-mode button {{
            border-color: #e0e0e0;
            background: #2a2a2a;
            color: #e0e0e0;
        }}
        button:hover {{
            background: #f0f0f0;
        }}
        body.dark-mode button:hover {{
            background: #3a3a3a;
        }}
        footer {{
            margin-top: 50px;
            padding-top: 20px;
            border-top: 1px solid #ddd;
            text-align: center;
            font-size: 12px;
        }}
        body.dark-mode footer {{
            border-top-color: #444;
        }}
        footer .disclaimer {{
            color: #ff0000;
        }}
        footer .credits {{
            color: #000;
        }}
        body.dark-mode footer .credits {{
            color: #e0e0e0;
        }}
        footer a {{
            color: #000;
            text-decoration: underline;
        }}
        body.dark-mode footer a {{
            color: #e0e0e0;
        }}
        .back-link {{
            margin-top: 30px;
        }}
        .github-icon {{
            width: 14px;
            height: 14px;
            vertical-align: -2px;
            margin-right: 4px;
            display: inline-block;
        }}
        .dark-mode-toggle {{
            position: absolute;
            top: 20px;
            right: 20px;
            background: none;
            border: 1px solid #000;
            padding: 6px 12px;
            font-family: monospace;
            font-size: 12px;
            cursor: pointer;
            background: #fff;
            color: #000;
        }}
        body.dark-mode .dark-mode-toggle {{
            border-color: #e0e0e0;
            background: #2a2a2a;
            color: #e0e0e0;
        }}
        .dark-mode-toggle:hover {{
            background: #f0f0f0;
        }}
        body.dark-mode .dark-mode-toggle:hover {{
            background: #3a3a3a;
        }}
        #home-link {{
            color: #000;
        }}
        body.dark-mode #home-link {{
            color: #e0e0e0;
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
            .dark-mode-toggle {{
                position: static;
                display: block;
                margin: 10px 0;
                width: auto;
            }}
        }}
    </style>
</head>
<body>
    <button class="dark-mode-toggle" id="dark-mode-toggle" onclick="toggleDarkMode()">Dark Mode</button>
    <h1><a href="/" id="home-link" style="text-decoration: none; cursor: pointer;">metarflow</a></h1>
    
    <div class="stats">
        <div class="stat-row">
            <span class="stat-label">Airport:</span>
            <span class="stat-value">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Date/Time:</span>
            <span class="stat-value{}" id="datetime-value">{}</span>
            <button id="datetime-toggle" onclick="toggleDateTime()" style="font-size: 11px; padding: 2px 6px; margin-left: 10px; display: none;">Show Local</button>
        </div>
        <div class="stat-row">
            <span class="stat-label">Wind:</span>
            <span class="stat-value{}">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Visibility:</span>
            <span class="stat-value{}">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Weather:</span>
            <span class="stat-value{}">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Clouds:</span>
            <span class="stat-value{}">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Temperature:</span>
            <span class="stat-value{}">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Dewpoint:</span>
            <span class="stat-value{}">{}</span>
        </div>
        <div class="stat-row">
            <span class="stat-label">Altimeter:</span>
            <span class="stat-value{}" id="altimeter-value">{}</span>
            <button id="altimeter-toggle" onclick="toggleAltimeter()" style="font-size: 11px; padding: 2px 6px; margin-left: 10px; display: none;">Toggle</button>
        </div>
        <div class="stat-row">
            <span class="stat-label">Remarks:</span>
            <span class="stat-value{}">{}</span>
        </div>
    </div>

    <div class="raw-metar">
        <div class="raw-metar-header">
            <h2>Raw METAR</h2>
            <button onclick="copyMetar()">Copy</button>
        </div>
        <pre id="raw-metar-text">{}</pre>
    </div>

    <div class="action-buttons">
        <button onclick="window.location.reload()">Refresh</button>
    </div>

    <p class="back-link"><a href="/">Back to home</a></p>

    <form action="/metar" method="get" id="metar-form" onsubmit="handleSubmit(event)">
        <label for="icao">Enter airport ICAO code:</label>
        <div class="search-container">
            <input type="text" id="icao" name="icao" placeholder="e.g., KJFK" maxlength="4" required 
                   oninput="this.value = this.value.toUpperCase()"
                   onfocus="showDropdown()"
                   onblur="setTimeout(() => hideDropdown(), 200)"
                   onkeypress="if(event.key === 'Enter') {{ event.preventDefault(); handleSubmit(event); }}">
            <div id="dropdown" class="dropdown">
                <div class="dropdown-section">
                    <h4>Popular Airports</h4>
                    <a href="/metar?icao=KJFK" class="dropdown-item" onclick="hideDropdown()">KJFK (JFK)</a>
                    <a href="/metar?icao=EGLL" class="dropdown-item" onclick="hideDropdown()">EGLL (Heathrow)</a>
                    <a href="/metar?icao=KLAX" class="dropdown-item" onclick="hideDropdown()">KLAX (LAX)</a>
                    <a href="/metar?icao=KORD" class="dropdown-item" onclick="hideDropdown()">KORD (O'Hare)</a>
                    <a href="/metar?icao=EDDF" class="dropdown-item" onclick="hideDropdown()">EDDF (Frankfurt)</a>
                </div>
                <div class="dropdown-section">
                    <h4>Recent Searches</h4>
                    <div id="recent-dropdown-list"></div>
                </div>
            </div>
        </div>
        <button type="submit" id="submit-btn">
            Fetch METAR
        </button>
    </form>
    <script>
        (function() {{
            const urlParams = new URLSearchParams(window.location.search);
            const icao = urlParams.get('icao');
            if (icao) {{
                let recent = JSON.parse(localStorage.getItem('metarflow_recent') || '[]');
                if (!recent.includes(icao)) {{
                    recent.unshift(icao);
                    recent = recent.slice(0, 5);
                    localStorage.setItem('metarflow_recent', JSON.stringify(recent));
                }} else {{
                    recent = recent.filter(r => r !== icao);
                    recent.unshift(icao);
                    recent = recent.slice(0, 5);
                    localStorage.setItem('metarflow_recent', JSON.stringify(recent));
                }}
            }}
        }})();
        
        function showDropdown() {{
            const dropdown = document.getElementById('dropdown');
            dropdown.classList.add('show');
            updateRecentSearches();
        }}
        
        function hideDropdown() {{
            const dropdown = document.getElementById('dropdown');
            dropdown.classList.remove('show');
        }}
        
        function updateRecentSearches() {{
            const recent = JSON.parse(localStorage.getItem('metarflow_recent') || '[]');
            const list = document.getElementById('recent-dropdown-list');
            list.innerHTML = '';
            
            const recentLimited = recent.slice(0, 5);
            
            if (recentLimited.length > 0) {{
                recentLimited.forEach(icao => {{
                    const link = document.createElement('a');
                    link.href = `/metar?icao=${{icao}}`;
                    link.className = 'dropdown-item';
                    link.textContent = icao;
                    link.onclick = () => hideDropdown();
                    list.appendChild(link);
                }});
            }} else {{
                const empty = document.createElement('div');
                empty.className = 'dropdown-item empty';
                empty.textContent = 'No recent searches';
                list.appendChild(empty);
            }}
        }}
        
        function handleSubmit(event) {{
            hideDropdown();
            const btn = document.getElementById('submit-btn');
            const originalText = btn.textContent;
            btn.textContent = 'Loading...';
            btn.disabled = true;
            
            setTimeout(() => {{
                btn.textContent = originalText;
                btn.disabled = false;
            }}, 5000);
            
            return true;
        }}
        
        updateRecentSearches();
        
        (function() {{
            const altimeterHpaStr = '{}';
            const altimeterInchesStr = '{}';
            const altimeterUnit = '{}';
            
            if (altimeterHpaStr !== 'null' && altimeterInchesStr !== 'null') {{
                const altimeterHpa = parseInt(altimeterHpaStr);
                const altimeterInches = parseFloat(altimeterInchesStr);
                let currentUnit = altimeterUnit;
            
                if (!isNaN(altimeterHpa) && !isNaN(altimeterInches)) {{
                    const toggleBtn = document.getElementById('altimeter-toggle');
                    if (toggleBtn) {{
                        toggleBtn.style.display = 'inline-block';
                        if (altimeterUnit === 'hpa') {{
                            toggleBtn.textContent = 'Show inHg';
                        }} else {{
                            toggleBtn.textContent = 'Show hPa';
                        }}
                    }}
                }}
                
                window.toggleAltimeter = function() {{
                    const valueEl = document.getElementById('altimeter-value');
                    const toggleBtn = document.getElementById('altimeter-toggle');
                    
                    if (currentUnit === 'hpa') {{
                        valueEl.textContent = altimeterInches.toFixed(2) + ' inches of mercury';
                        currentUnit = 'inches';
                        if (toggleBtn) toggleBtn.textContent = 'Show hPa';
                    }} else {{
                        valueEl.textContent = altimeterHpa + ' hectopascals';
                        currentUnit = 'hpa';
                        if (toggleBtn) toggleBtn.textContent = 'Show inHg';
                    }}
                }};
            }}
        }})();
        
        (function() {{
            const zuluDayStr = '{}';
            const zuluHourStr = '{}';
            const zuluMinuteStr = '{}';
            
            if (zuluDayStr !== 'null' && zuluHourStr !== 'null' && zuluMinuteStr !== 'null') {{
                const zuluDay = parseInt(zuluDayStr);
                const zuluHour = parseInt(zuluHourStr);
                const zuluMinute = parseInt(zuluMinuteStr);
                
                if (!isNaN(zuluDay) && !isNaN(zuluHour) && !isNaN(zuluMinute)) {{
                    const toggleBtn = document.getElementById('datetime-toggle');
                    if (toggleBtn) {{
                        toggleBtn.style.display = 'inline-block';
                    }}
                    
                    window.toggleDateTime = function() {{
                        const valueEl = document.getElementById('datetime-value');
                        const toggleBtn = document.getElementById('datetime-toggle');
                        
                        if (!valueEl || !toggleBtn) return;
                        
                        const currentText = valueEl.textContent;
                        const isZulu = currentText.includes('Z');
                        
                        if (isZulu) {{
                            const now = new Date();
                            const year = now.getFullYear();
                            const month = now.getMonth();
                            
                            const utcDate = new Date(Date.UTC(year, month, zuluDay, zuluHour, zuluMinute));
                            
                            const localDay = utcDate.getDate();
                            const localHour = utcDate.getHours();
                            const localMinute = utcDate.getMinutes();
                            
                            valueEl.textContent = `Day ${{localDay}}, ${{localHour}}:${{String(localMinute).padStart(2, '0')}} Local`;
                            toggleBtn.textContent = 'Show Zulu';
                        }} else {{
                            valueEl.textContent = `Day ${{zuluDay}}, ${{String(zuluHour).padStart(2, '0')}}:${{String(zuluMinute).padStart(2, '0')}}Z`;
                            toggleBtn.textContent = 'Show Local';
                        }}
                    }};
                }}
            }}
        }})();
        
        function copyMetar() {{
            const text = document.getElementById('raw-metar-text').textContent;
            navigator.clipboard.writeText(text).then(function() {{
                alert('Copied to clipboard!');
            }}, function() {{
                const textarea = document.createElement('textarea');
                textarea.value = text;
                document.body.appendChild(textarea);
                textarea.select();
                document.execCommand('copy');
                document.body.removeChild(textarea);
                alert('Copied to clipboard!');
            }});
        }}
        
        function toggleDarkMode() {{
            const body = document.body;
            const isDark = body.classList.toggle('dark-mode');
            const toggle = document.getElementById('dark-mode-toggle');
            
            if (isDark) {{
                localStorage.setItem('metarflow_dark_mode', 'true');
                toggle.textContent = 'Light Mode';
            }} else {{
                localStorage.setItem('metarflow_dark_mode', 'false');
                toggle.textContent = 'Dark Mode';
            }}
        }}
        
        function initDarkMode() {{
            const savedMode = localStorage.getItem('metarflow_dark_mode');
            const toggle = document.getElementById('dark-mode-toggle');
            
            if (savedMode === 'true') {{
                document.body.classList.add('dark-mode');
                toggle.textContent = 'Light Mode';
            }} else {{
                document.body.classList.remove('dark-mode');
                toggle.textContent = 'Dark Mode';
            }}
        }}
        
        initDarkMode();
        
    </script>
    <footer>
        <p class="disclaimer">The information on this website is intended for educational purposes only. Not for operational use.</p>
        <p class="credits">Created by <a href="https://github.com/Balionelis/metarflow" target="_blank" rel="noopener noreferrer"><svg class="github-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.012 8.012 0 0 0 16 8c0-4.42-3.58-8-8-8z"/></svg>Balionelis</a>. Licensed under <a href="https://github.com/Balionelis/metarflow/blob/main/LICENSE" target="_blank" rel="noopener noreferrer">GPL-3.0</a>. <a href="/privacy">Privacy</a>.</p>
    </footer>
</body>
</html>
        "#,
        info.station,
        dt_class, dt_value,
        wind_class, wind_value,
        vis_class, vis_value,
        wx_class, wx_value,
        clouds_class, clouds_value,
        temp_class, temp_value,
        dew_class, dew_value,
        alt_class, alt_value,
        rmk_class, rmk_value,
        info.raw,
        altimeter_hpa, altimeter_inches, altimeter_default,
        zulu_day, zulu_hour, zulu_minute
    )
}

