# metarflow

A simple web application built with Rust and Axum to display METAR weather information for airports.

## Features

- Clean, minimal web interface
- Parses raw METAR data into human-readable information
- Displays wind, visibility, weather, clouds, temperature, dewpoint, altimeter, and remarks
- Mobile responsive design
- Optional Google Analytics integration (via environment variable)

## Running locally

```bash
cargo run
```

Then open http://localhost:3000

## Example ICAO codes

- KJFK - New York JFK
- EGLL - London Heathrow
- EYVI - Vilnius
- KLAX - Los Angeles

## Tech stack

- Rust
- Axum web framework
- Data from aviationweather.gov

## Google Analytics Setup (Optional)

To enable Google Analytics:

1. Go to [Google Analytics](https://analytics.google.com/)
2. Create an account or sign in
3. Create a new property (GA4)
4. Get your **Measurement ID** (starts with `G-`)
5. Set the environment variable before running:

```bash
export GA_MEASUREMENT_ID=G-XXXXXXXXXX
cargo run
```

## License

[GPL-3.0](https://github.com/Balionelis/metarflow/blob/main/LICENSE)
