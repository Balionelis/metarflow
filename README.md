# metarflow

A simple web application built with Rust and Axum to display METAR weather information for airports.

## Features

- Clean, minimal web interface
- Parses raw METAR data into human-readable information
- Displays wind, visibility, weather, clouds, temperature, dewpoint, altimeter, and remarks
- Mobile responsive design
- Dark mode support

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

## License

[GPL-3.0](https://github.com/Balionelis/metarflow/blob/main/LICENSE)
