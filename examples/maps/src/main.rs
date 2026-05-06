//! CIA World Factbook-style interactive map explorer.
//!
//! Fetches Natural Earth 110m GeoJSON from GitHub and renders a
//! choropleth colored by selectable economic indicators (GDP,
//! GDP per capita, population, GDP growth).
//!
//! Run with: cargo run --package maps

mod icon;

use std::sync::Arc;

use iced::widget::{button, center, column, container, pick_list, row, text};
use iced::{Element, Fill, Font, Task, Theme};

use hyozu::ProjectionKind;
use hyozu::geo::{self, GeoData, MapScope};

// ── Constants ──────────────────────────────────────────────────────

const COUNTRIES_URL: &str =
    "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_0_countries.geojson";

const STATES_URL: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_1_states_provinces.geojson";

// ── Indicator ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Indicator {
    Gdp,
    GdpPerCapita,
    Population,
    GdpGrowth,
}

impl Indicator {
    const ALL: [Indicator; 4] = [
        Indicator::Gdp,
        Indicator::GdpPerCapita,
        Indicator::Population,
        Indicator::GdpGrowth,
    ];

    fn label(&self) -> &'static str {
        match self {
            Indicator::Gdp => "GDP (Trillions USD)",
            Indicator::GdpPerCapita => "GDP per Capita (Thousands USD)",
            Indicator::Population => "Population (Millions)",
            Indicator::GdpGrowth => "GDP Growth (%)",
        }
    }

    fn short_label(&self) -> &'static str {
        match self {
            Indicator::Gdp => "GDP",
            Indicator::GdpPerCapita => "GDP/cap",
            Indicator::Population => "Population",
            Indicator::GdpGrowth => "Growth",
        }
    }

    /// Compact value string for the hover annotation. The data
    /// values are stored in the indicator's natural units
    /// (trillions, thousands, millions, percent), so the suffix
    /// is constant per indicator.
    fn format_hover_value(&self, value: f64) -> String {
        match self {
            Indicator::Gdp => format!("${value:.1}T"),
            Indicator::GdpPerCapita => format!("${value:.0}K"),
            Indicator::Population => format!("{value:.0}M"),
            Indicator::GdpGrowth => format!("{value:.1}%"),
        }
    }

    fn data(&self) -> Vec<(&'static str, f64)> {
        match self {
            // GDP in trillions USD (2023 estimates, World Bank / IMF)
            Indicator::Gdp => vec![
                // ── Major economies (existing) ──
                ("USA", 25.5),
                ("CHN", 17.9),
                ("JPN", 4.2),
                ("DEU", 4.1),
                ("IND", 3.4),
                ("GBR", 3.1),
                ("FRA", 2.8),
                ("CAN", 2.1),
                ("ITA", 2.0),
                ("BRA", 1.9),
                ("RUS", 1.8),
                ("KOR", 1.7),
                ("AUS", 1.7),
                ("ESP", 1.4),
                ("MEX", 1.3),
                ("IDN", 1.3),
                ("SAU", 1.1),
                ("TUR", 0.9),
                ("NLD", 1.0),
                ("CHE", 0.8),
                ("POL", 0.7),
                ("SWE", 0.6),
                ("BEL", 0.6),
                ("THA", 0.5),
                ("AUT", 0.5),
                ("NOR", 0.5),
                ("ISR", 0.5),
                ("ARE", 0.5),
                ("NGA", 0.5),
                ("EGY", 0.4),
                ("ZAF", 0.4),
                ("ARG", 0.6),
                ("COL", 0.3),
                ("PHL", 0.4),
                ("PAK", 0.3),
                // ── Europe ──
                ("IRL", 0.53),
                ("DNK", 0.40),
                ("FIN", 0.30),
                ("PRT", 0.27),
                ("CZE", 0.33),
                ("ROU", 0.30),
                ("GRC", 0.22),
                ("HUN", 0.18),
                ("UKR", 0.15),
                ("SVK", 0.13),
                ("BGR", 0.10),
                ("HRV", 0.07),
                ("LTU", 0.07),
                ("SVN", 0.06),
                ("LVA", 0.04),
                ("EST", 0.04),
                ("SRB", 0.07),
                ("BIH", 0.02),
                ("ALB", 0.02),
                ("MKD", 0.01),
                ("MNE", 0.01),
                ("LUX", 0.08),
                ("ISL", 0.03),
                ("BLR", 0.07),
                ("MDA", 0.02),
                // ── Asia & Middle East ──
                ("VNM", 0.41),
                ("MYS", 0.41),
                ("BGD", 0.46),
                ("MMR", 0.06),
                ("KHM", 0.03),
                ("LAO", 0.02),
                ("LKA", 0.08),
                ("NPL", 0.04),
                ("UZB", 0.08),
                ("KAZ", 0.22),
                ("TKM", 0.05),
                ("AZE", 0.07),
                ("GEO", 0.02),
                ("ARM", 0.02),
                ("MNG", 0.02),
                ("AFG", 0.01),
                ("IRN", 0.37),
                ("IRQ", 0.26),
                ("SYR", 0.01),
                ("JOR", 0.05),
                ("LBN", 0.02),
                ("YEM", 0.02),
                ("OMN", 0.10),
                ("QAT", 0.22),
                ("KWT", 0.16),
                ("BHR", 0.04),
                ("PRK", 0.02),
                ("NZL", 0.25),
                ("PNG", 0.03),
                ("TWN", 0.79),
                // ── Africa ──
                ("DZA", 0.19),
                ("MAR", 0.13),
                ("ETH", 0.16),
                ("KEN", 0.11),
                ("TZA", 0.08),
                ("GHA", 0.07),
                ("CIV", 0.07),
                ("CMR", 0.04),
                ("AGO", 0.12),
                ("MOZ", 0.02),
                ("MDG", 0.01),
                ("SEN", 0.03),
                ("TUN", 0.05),
                ("UGA", 0.05),
                ("ZMB", 0.03),
                ("ZWE", 0.02),
                ("BWA", 0.02),
                ("NAM", 0.01),
                ("MUS", 0.01),
                ("GAB", 0.02),
                ("LBY", 0.04),
                ("SDN", 0.03),
                ("SSD", 0.01),
                ("MLI", 0.02),
                ("BFA", 0.02),
                ("NER", 0.02),
                ("TCD", 0.01),
                ("RWA", 0.01),
                ("MWI", 0.01),
                ("BEN", 0.02),
                ("TGO", 0.01),
                ("SLE", 0.004),
                ("GIN", 0.02),
                ("LBR", 0.004),
                ("MRT", 0.01),
                ("COD", 0.06),
                ("COG", 0.01),
                ("GNQ", 0.01),
                ("ERI", 0.003),
                ("SWZ", 0.005),
                ("LSO", 0.003),
                ("CAF", 0.003),
                ("GMB", 0.002),
                ("BDI", 0.003),
                ("DJI", 0.004),
                // ── Americas ──
                ("PER", 0.24),
                ("ECU", 0.12),
                ("VEN", 0.10),
                ("BOL", 0.04),
                ("PRY", 0.04),
                ("URY", 0.07),
                ("GUY", 0.01),
                ("SUR", 0.004),
                ("CRI", 0.07),
                ("PAN", 0.08),
                ("GTM", 0.10),
                ("HND", 0.03),
                ("SLV", 0.03),
                ("NIC", 0.02),
                ("CUB", 0.11),
                ("DOM", 0.11),
                ("HTI", 0.02),
                ("JAM", 0.02),
                ("TTO", 0.03),
                ("CHL", 0.34),
            ],
            // GDP per capita in thousands USD (2023 estimates)
            Indicator::GdpPerCapita => vec![
                // ── Existing ──
                ("CHE", 93.3),
                ("NOR", 82.8),
                ("USA", 76.4),
                ("AUS", 64.5),
                ("DNK", 61.1),
                ("NLD", 57.0),
                ("SWE", 55.6),
                ("AUT", 52.1),
                ("ISR", 52.2),
                ("FIN", 50.1),
                ("CAN", 52.0),
                ("DEU", 51.2),
                ("BEL", 49.5),
                ("GBR", 46.1),
                ("FRA", 44.4),
                ("JPN", 33.8),
                ("KOR", 32.4),
                ("ITA", 34.1),
                ("ESP", 30.1),
                ("SAU", 27.7),
                ("POL", 17.8),
                ("MEX", 10.9),
                ("ARG", 10.6),
                ("RUS", 11.3),
                ("BRA", 8.9),
                ("CHN", 12.7),
                ("TUR", 10.6),
                ("THA", 7.1),
                ("COL", 6.2),
                ("ZAF", 6.0),
                ("IDN", 4.8),
                ("EGY", 3.6),
                ("IND", 2.6),
                ("NGA", 2.2),
                ("PHL", 3.5),
                ("PAK", 1.5),
                // ── Europe ──
                ("IRL", 100.2),
                ("LUX", 126.6),
                ("ISL", 73.5),
                ("PRT", 26.0),
                ("CZE", 27.2),
                ("SVN", 31.0),
                ("EST", 28.3),
                ("LTU", 24.0),
                ("LVA", 20.7),
                ("SVK", 22.0),
                ("HUN", 18.5),
                ("HRV", 18.0),
                ("ROU", 15.8),
                ("BGR", 14.3),
                ("GRC", 20.7),
                ("SRB", 9.5),
                ("BIH", 7.3),
                ("ALB", 6.8),
                ("MKD", 6.9),
                ("MNE", 10.0),
                ("BLR", 7.2),
                ("UKR", 4.5),
                ("MDA", 5.6),
                // ── Asia & Middle East ──
                ("ARE", 46.9),
                ("QAT", 82.0),
                ("KWT", 38.0),
                ("BHR", 27.0),
                ("OMN", 20.8),
                ("MYS", 12.4),
                ("KAZ", 11.5),
                ("IRN", 4.4),
                ("IRQ", 5.9),
                ("VNM", 4.1),
                ("LKA", 3.5),
                ("BGD", 2.7),
                ("UZB", 2.2),
                ("GEO", 6.0),
                ("ARM", 7.0),
                ("AZE", 6.8),
                ("JOR", 4.4),
                ("LBN", 3.0),
                ("MNG", 5.0),
                ("TKM", 7.6),
                ("KHM", 1.8),
                ("LAO", 2.5),
                ("MMR", 1.1),
                ("NPL", 1.3),
                ("AFG", 0.4),
                ("SYR", 0.5),
                ("YEM", 0.6),
                ("PRK", 0.8),
                ("NZL", 48.0),
                ("PNG", 3.0),
                ("TWN", 33.0),
                // ── Africa ──
                ("MUS", 10.2),
                ("BWA", 7.7),
                ("GAB", 8.8),
                ("GNQ", 7.3),
                ("NAM", 4.9),
                ("LBY", 6.0),
                ("DZA", 4.3),
                ("TUN", 3.8),
                ("MAR", 3.5),
                ("AGO", 3.5),
                ("CIV", 2.5),
                ("GHA", 2.3),
                ("KEN", 2.1),
                ("SEN", 1.7),
                ("CMR", 1.6),
                ("TZA", 1.2),
                ("ZMB", 1.3),
                ("ETH", 1.3),
                ("UGA", 1.0),
                ("BEN", 1.4),
                ("RWA", 0.9),
                ("MLI", 0.9),
                ("BFA", 0.8),
                ("TGO", 0.9),
                ("MRT", 2.0),
                ("NER", 0.6),
                ("ZWE", 1.7),
                ("MDG", 0.5),
                ("MWI", 0.6),
                ("TCD", 0.7),
                ("SLE", 0.5),
                ("GIN", 1.2),
                ("LBR", 0.7),
                ("SDN", 0.7),
                ("SSD", 0.4),
                ("COD", 0.6),
                ("COG", 2.1),
                ("ERI", 0.6),
                ("CAF", 0.5),
                ("SWZ", 4.1),
                ("LSO", 1.1),
                ("GMB", 0.8),
                ("BDI", 0.3),
                ("DJI", 3.5),
                // ── Americas ──
                ("CHL", 16.3),
                ("URY", 20.5),
                ("PAN", 15.5),
                ("CRI", 13.0),
                ("DOM", 10.2),
                ("PER", 7.0),
                ("ECU", 6.3),
                ("GTM", 5.5),
                ("PRY", 5.4),
                ("BOL", 3.6),
                ("SLV", 5.0),
                ("JAM", 6.0),
                ("TTO", 18.0),
                ("GUY", 14.0),
                ("HND", 2.8),
                ("NIC", 2.2),
                ("CUB", 9.5),
                ("VEN", 3.7),
                ("HTI", 1.8),
                ("SUR", 6.0),
            ],
            // Population in millions (2023 estimates)
            Indicator::Population => vec![
                // ── Existing ──
                ("CHN", 1412.0),
                ("IND", 1408.0),
                ("USA", 331.0),
                ("IDN", 273.0),
                ("PAK", 221.0),
                ("BRA", 213.0),
                ("NGA", 211.0),
                ("RUS", 146.0),
                ("MEX", 129.0),
                ("JPN", 126.0),
                ("EGY", 102.0),
                ("PHL", 110.0),
                ("TUR", 84.0),
                ("DEU", 83.0),
                ("THA", 70.0),
                ("GBR", 67.0),
                ("FRA", 67.0),
                ("ITA", 60.0),
                ("ZAF", 59.0),
                ("KOR", 52.0),
                ("COL", 51.0),
                ("ESP", 47.0),
                ("ARG", 45.0),
                ("POL", 38.0),
                ("CAN", 38.0),
                ("SAU", 35.0),
                ("AUS", 26.0),
                ("NLD", 17.0),
                ("CHE", 8.7),
                ("SWE", 10.4),
                ("AUT", 9.0),
                ("ISR", 9.0),
                ("NOR", 5.4),
                ("ARE", 10.0),
                ("BEL", 11.5),
                // ── Europe ──
                ("PRT", 10.3),
                ("GRC", 10.4),
                ("CZE", 10.8),
                ("HUN", 9.7),
                ("ROU", 19.0),
                ("BGR", 6.5),
                ("SRB", 6.7),
                ("HRV", 3.9),
                ("SVK", 5.5),
                ("DNK", 5.9),
                ("FIN", 5.5),
                ("IRL", 5.1),
                ("LTU", 2.8),
                ("SVN", 2.1),
                ("LVA", 1.8),
                ("EST", 1.3),
                ("BIH", 3.3),
                ("ALB", 2.8),
                ("MKD", 1.8),
                ("MNE", 0.6),
                ("LUX", 0.66),
                ("ISL", 0.38),
                ("UKR", 37.0),
                ("BLR", 9.4),
                ("MDA", 2.6),
                // ── Asia & Middle East ──
                ("BGD", 170.0),
                ("VNM", 98.0),
                ("IRN", 87.0),
                ("MYS", 33.0),
                ("MMR", 54.0),
                ("IRQ", 43.0),
                ("AFG", 41.0),
                ("NPL", 30.0),
                ("UZB", 35.0),
                ("KAZ", 19.4),
                ("SYR", 22.0),
                ("KHM", 17.0),
                ("YEM", 33.0),
                ("LKA", 22.0),
                ("JOR", 11.0),
                ("AZE", 10.2),
                ("LAO", 7.5),
                ("TKM", 6.3),
                ("GEO", 3.7),
                ("MNG", 3.4),
                ("ARM", 3.0),
                ("LBN", 5.5),
                ("OMN", 4.6),
                ("KWT", 4.3),
                ("QAT", 2.7),
                ("BHR", 1.5),
                ("PRK", 26.0),
                ("NZL", 5.1),
                ("PNG", 10.0),
                ("TWN", 23.6),
                // ── Africa ──
                ("ETH", 123.0),
                ("COD", 99.0),
                ("DZA", 45.0),
                ("SDN", 46.0),
                ("MAR", 37.0),
                ("AGO", 34.0),
                ("MOZ", 33.0),
                ("GHA", 33.0),
                ("CIV", 28.0),
                ("CMR", 28.0),
                ("MDG", 29.0),
                ("NER", 26.0),
                ("BFA", 22.0),
                ("MLI", 22.0),
                ("MWI", 20.0),
                ("ZMB", 20.0),
                ("SEN", 17.0),
                ("TCD", 17.0),
                ("ZWE", 16.0),
                ("GIN", 14.0),
                ("RWA", 13.5),
                ("BEN", 13.0),
                ("TUN", 12.0),
                ("SSD", 11.0),
                ("SLE", 8.4),
                ("TGO", 8.6),
                ("LBY", 7.0),
                ("LBR", 5.3),
                ("MRT", 4.6),
                ("COG", 6.0),
                ("ERI", 3.6),
                ("CAF", 5.5),
                ("NAM", 2.6),
                ("BWA", 2.4),
                ("GAB", 2.3),
                ("LSO", 2.2),
                ("GMB", 2.5),
                ("SWZ", 1.2),
                ("GNQ", 1.6),
                ("MUS", 1.3),
                ("DJI", 1.1),
                ("UGA", 47.0),
                ("KEN", 54.0),
                ("TZA", 65.0),
                // ── Americas ──
                ("PER", 34.0),
                ("VEN", 28.0),
                ("CHL", 19.5),
                ("ECU", 18.0),
                ("GTM", 17.6),
                ("CUB", 11.3),
                ("DOM", 11.0),
                ("BOL", 12.0),
                ("HTI", 11.6),
                ("HND", 10.3),
                ("PRY", 7.1),
                ("SLV", 6.5),
                ("NIC", 6.9),
                ("CRI", 5.2),
                ("PAN", 4.4),
                ("URY", 3.4),
                ("JAM", 3.0),
                ("TTO", 1.4),
                ("GUY", 0.8),
                ("SUR", 0.6),
            ],
            // GDP growth % (2023 estimates)
            Indicator::GdpGrowth => vec![
                // ── Existing ──
                ("IND", 6.3),
                ("CHN", 5.2),
                ("IDN", 5.0),
                ("PHL", 5.6),
                ("EGY", 4.2),
                ("SAU", 3.4),
                ("TUR", 4.5),
                ("MEX", 3.2),
                ("BRA", 2.9),
                ("USA", 2.5),
                ("KOR", 1.4),
                ("AUS", 1.5),
                ("CAN", 1.1),
                ("POL", 0.2),
                ("GBR", 0.1),
                ("FRA", 0.9),
                ("ESP", 2.5),
                ("ITA", 0.7),
                ("NLD", 0.1),
                ("DEU", -0.3),
                ("JPN", 1.9),
                ("CHE", 0.8),
                ("SWE", 0.0),
                ("NOR", 0.5),
                ("BEL", 1.5),
                ("AUT", 0.0),
                ("ISR", 2.0),
                ("NGA", 2.9),
                ("ZAF", 0.6),
                ("COL", 1.2),
                ("ARG", -1.6),
                ("PAK", -0.5),
                ("THA", 1.9),
                ("RUS", -2.1),
                ("ARE", 3.6),
                // ── Europe ──
                ("IRL", -3.2),
                ("PRT", 2.3),
                ("GRC", 2.0),
                ("CZE", -0.4),
                ("ROU", 2.1),
                ("HUN", -0.9),
                ("BGR", 1.8),
                ("HRV", 2.8),
                ("SVK", 1.1),
                ("SVN", 1.6),
                ("LTU", -0.3),
                ("LVA", -0.3),
                ("EST", -3.0),
                ("DNK", 1.9),
                ("FIN", -1.0),
                ("SRB", 2.5),
                ("BIH", 1.7),
                ("ALB", 3.3),
                ("MKD", 1.0),
                ("MNE", 6.0),
                ("LUX", -1.1),
                ("ISL", 4.1),
                ("UKR", 5.3),
                ("BLR", 3.9),
                ("MDA", 2.0),
                // ── Asia & Middle East ──
                ("VNM", 5.1),
                ("MYS", 3.7),
                ("BGD", 5.8),
                ("MMR", 2.5),
                ("KHM", 5.6),
                ("LAO", 3.7),
                ("LKA", -2.3),
                ("NPL", 4.1),
                ("UZB", 5.6),
                ("KAZ", 5.1),
                ("TKM", 6.3),
                ("AZE", 1.1),
                ("GEO", 7.5),
                ("ARM", 8.7),
                ("MNG", 7.0),
                ("AFG", -6.2),
                ("IRN", 5.4),
                ("IRQ", -3.0),
                ("SYR", -0.8),
                ("JOR", 2.6),
                ("LBN", -0.2),
                ("YEM", -2.0),
                ("OMN", 1.3),
                ("QAT", 1.6),
                ("KWT", -1.5),
                ("BHR", 2.7),
                ("PRK", -1.0),
                ("NZL", 0.6),
                ("PNG", 3.0),
                ("TWN", 1.3),
                // ── Africa ──
                ("DZA", 4.1),
                ("MAR", 3.0),
                ("ETH", 7.2),
                ("KEN", 5.4),
                ("TZA", 5.1),
                ("GHA", 2.9),
                ("CIV", 6.5),
                ("CMR", 3.6),
                ("AGO", 0.5),
                ("MOZ", 5.0),
                ("MDG", 4.0),
                ("SEN", 4.6),
                ("TUN", 0.4),
                ("UGA", 5.3),
                ("ZMB", 4.3),
                ("ZWE", 3.5),
                ("BWA", 3.8),
                ("NAM", 3.2),
                ("MUS", 7.0),
                ("GAB", 2.3),
                ("LBY", 10.0),
                ("SDN", -12.0),
                ("SSD", -1.0),
                ("MLI", 5.0),
                ("BFA", 4.4),
                ("NER", 11.1),
                ("TCD", 3.7),
                ("RWA", 8.2),
                ("MWI", 1.6),
                ("BEN", 6.0),
                ("TGO", 5.3),
                ("SLE", 2.7),
                ("GIN", 5.6),
                ("LBR", 4.6),
                ("MRT", 4.8),
                ("COD", 6.2),
                ("COG", 1.5),
                ("ERI", 2.9),
                ("CAF", 1.0),
                ("SWZ", 3.4),
                ("LSO", 1.9),
                ("GMB", 5.6),
                ("BDI", 2.7),
                ("DJI", 7.0),
                ("GNQ", -5.4),
                // ── Americas ──
                ("PER", -0.6),
                ("ECU", 2.4),
                ("VEN", 5.0),
                ("BOL", 2.2),
                ("PRY", 4.5),
                ("URY", 0.4),
                ("GUY", 38.4),
                ("SUR", 2.4),
                ("CRI", 5.1),
                ("PAN", 7.3),
                ("GTM", 3.5),
                ("HND", 3.5),
                ("SLV", 3.5),
                ("NIC", 3.8),
                ("CUB", 1.8),
                ("DOM", 2.4),
                ("HTI", -1.9),
                ("JAM", 2.2),
                ("TTO", 2.1),
                ("CHL", 0.2),
            ],
        }
    }

    fn state_data(&self) -> Vec<(&'static str, f64)> {
        match self {
            // GDP in trillions USD (2023 estimates)
            Indicator::Gdp => vec![
                ("CA", 3.6),
                ("TX", 2.0),
                ("NY", 1.9),
                ("FL", 1.2),
                ("IL", 0.9),
                ("PA", 0.8),
                ("OH", 0.7),
                ("GA", 0.6),
                ("NJ", 0.6),
                ("WA", 0.6),
                ("MA", 0.6),
                ("NC", 0.6),
                ("VA", 0.5),
                ("MI", 0.5),
                ("TN", 0.4),
                ("MD", 0.4),
                ("MN", 0.4),
                ("IN", 0.4),
                ("WI", 0.3),
                ("CO", 0.4),
                ("AZ", 0.4),
                ("MO", 0.3),
                ("CT", 0.3),
                ("OR", 0.3),
                ("SC", 0.2),
                ("LA", 0.3),
                ("AL", 0.2),
                ("KY", 0.2),
                ("OK", 0.2),
                ("IA", 0.2),
                ("UT", 0.2),
                ("NV", 0.2),
                ("KS", 0.2),
                ("AR", 0.1),
                ("NE", 0.1),
                ("MS", 0.1),
                ("NM", 0.1),
                ("HI", 0.09),
                ("NH", 0.09),
                ("ID", 0.09),
                ("WV", 0.08),
                ("ME", 0.08),
                ("RI", 0.07),
                ("DE", 0.08),
                ("MT", 0.06),
                ("SD", 0.06),
                ("ND", 0.06),
                ("AK", 0.06),
                ("VT", 0.04),
                ("WY", 0.04),
                ("DC", 0.15),
            ],
            // GDP per capita in thousands USD (2023 estimates)
            Indicator::GdpPerCapita => vec![
                ("DC", 200.0),
                ("NY", 94.0),
                ("MA", 91.0),
                ("WA", 84.0),
                ("CT", 82.0),
                ("CA", 81.0),
                ("NJ", 73.0),
                ("ND", 72.0),
                ("AK", 70.0),
                ("IL", 69.0),
                ("WY", 68.0),
                ("CO", 67.0),
                ("MN", 66.0),
                ("NH", 65.0),
                ("DE", 64.0),
                ("NE", 63.0),
                ("MD", 62.0),
                ("VA", 61.0),
                ("OR", 60.0),
                ("TX", 59.0),
                ("PA", 58.0),
                ("UT", 57.0),
                ("HI", 56.0),
                ("GA", 55.0),
                ("WI", 55.0),
                ("OH", 54.0),
                ("IA", 54.0),
                ("NC", 53.0),
                ("TN", 53.0),
                ("NV", 52.0),
                ("IN", 52.0),
                ("MI", 52.0),
                ("KS", 52.0),
                ("AZ", 51.0),
                ("FL", 51.0),
                ("RI", 51.0),
                ("MO", 50.0),
                ("SD", 50.0),
                ("VT", 49.0),
                ("MT", 48.0),
                ("ME", 48.0),
                ("KY", 46.0),
                ("ID", 46.0),
                ("SC", 46.0),
                ("NM", 45.0),
                ("OK", 45.0),
                ("LA", 44.0),
                ("AL", 43.0),
                ("AR", 42.0),
                ("WV", 41.0),
                ("MS", 38.0),
            ],
            // Population in millions (2023 estimates)
            Indicator::Population => vec![
                ("CA", 39.0),
                ("TX", 30.0),
                ("FL", 22.2),
                ("NY", 19.7),
                ("PA", 13.0),
                ("IL", 12.6),
                ("OH", 11.8),
                ("GA", 10.9),
                ("NC", 10.7),
                ("MI", 10.0),
                ("NJ", 9.3),
                ("VA", 8.6),
                ("WA", 7.7),
                ("AZ", 7.4),
                ("TN", 7.1),
                ("MA", 7.0),
                ("IN", 6.8),
                ("MO", 6.2),
                ("MD", 6.2),
                ("WI", 5.9),
                ("CO", 5.8),
                ("MN", 5.7),
                ("SC", 5.3),
                ("AL", 5.0),
                ("LA", 4.6),
                ("KY", 4.5),
                ("OR", 4.2),
                ("OK", 4.0),
                ("CT", 3.6),
                ("UT", 3.4),
                ("IA", 3.2),
                ("NV", 3.2),
                ("AR", 3.0),
                ("MS", 2.9),
                ("KS", 2.9),
                ("NM", 2.1),
                ("NE", 2.0),
                ("ID", 1.9),
                ("WV", 1.8),
                ("HI", 1.4),
                ("NH", 1.4),
                ("ME", 1.4),
                ("MT", 1.1),
                ("RI", 1.1),
                ("DE", 1.0),
                ("SD", 0.9),
                ("ND", 0.8),
                ("AK", 0.7),
                ("DC", 0.7),
                ("VT", 0.6),
                ("WY", 0.6),
            ],
            // GDP growth % (2023 estimates)
            Indicator::GdpGrowth => vec![
                ("NV", 5.1),
                ("TX", 4.8),
                ("FL", 4.5),
                ("ID", 4.3),
                ("SC", 4.1),
                ("AZ", 4.0),
                ("UT", 3.9),
                ("GA", 3.8),
                ("WA", 3.7),
                ("MT", 3.7),
                ("NC", 3.6),
                ("TN", 3.5),
                ("CO", 3.4),
                ("NH", 3.3),
                ("SD", 3.2),
                ("DE", 3.1),
                ("IN", 3.0),
                ("NE", 2.9),
                ("ME", 2.8),
                ("OR", 2.8),
                ("MA", 2.7),
                ("VA", 2.7),
                ("CA", 2.6),
                ("MN", 2.6),
                ("ND", 2.5),
                ("NJ", 2.5),
                ("AL", 2.4),
                ("KY", 2.4),
                ("MD", 2.3),
                ("WI", 2.3),
                ("MI", 2.2),
                ("OH", 2.2),
                ("MO", 2.1),
                ("AR", 2.1),
                ("CT", 2.0),
                ("RI", 2.0),
                ("PA", 1.9),
                ("DC", 1.9),
                ("HI", 1.8),
                ("NY", 1.8),
                ("IA", 1.7),
                ("KS", 1.7),
                ("IL", 1.6),
                ("NM", 1.5),
                ("WV", 1.4),
                ("OK", 1.3),
                ("MS", 1.2),
                ("VT", 1.1),
                ("LA", 1.0),
                ("AK", 0.8),
                ("WY", 0.7),
            ],
        }
    }
}

// ── Model ──────────────────────────────────────────────────────────

struct App {
    countries: Option<Arc<GeoData>>,
    states: Option<Arc<GeoData>>,
    dark: bool,
    theme: Theme,
    scope: MapScope,
    indicator: Indicator,
    color_scale: hyozu::palette::Scheme,
    chart_data: hyozu::Data,
    loading: bool,
}

#[derive(Debug, Clone)]
enum GeoKind {
    Countries,
    States,
}

#[derive(Debug, Clone)]
enum Message {
    GeoLoaded(GeoKind, Result<Arc<GeoData>, String>),
    ToggleDark,
    SetScope(MapScope),
    Back,
    SetIndicator(Indicator),
    SetColorScale(hyozu::palette::Scheme),
    ChartAction(hyozu::Action),
}

// ── App ────────────────────────────────────────────────────────────

impl App {
    fn new() -> (Self, Task<Message>) {
        let task = Task::batch([
            Task::future(fetch_geo(COUNTRIES_URL))
                .map(|r| Message::GeoLoaded(GeoKind::Countries, r.map(Arc::new).map_err(|e| e.to_string()))),
            Task::future(fetch_geo(STATES_URL))
                .map(|r| Message::GeoLoaded(GeoKind::States, r.map(Arc::new).map_err(|e| e.to_string()))),
        ]);

        (
            Self {
                countries: None,
                states: None,
                dark: false,
                theme: Theme::Light,
                scope: MapScope::World,
                indicator: Indicator::Gdp,
                color_scale: hyozu::palette::Scheme::RedGreen,
                chart_data: hyozu::Data::default(),
                loading: true,
            },
            task,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::GeoLoaded(kind, Ok(geo)) => {
                match kind {
                    GeoKind::Countries => {
                        self.countries = Some(Arc::clone(&geo));
                        self.loading = false;
                    }
                    GeoKind::States => {
                        self.states = Some(Arc::clone(&geo));
                    }
                }
                self.chart_data = self.build_chart_data();
            }
            Message::GeoLoaded(kind, Err(err)) => {
                eprintln!("Failed to load {kind:?} geo data: {err}");
                if matches!(kind, GeoKind::Countries) {
                    self.loading = false;
                }
            }
            Message::ToggleDark => {
                self.dark = !self.dark;
                self.theme = if self.dark { Theme::Dark } else { Theme::Light };
            }
            Message::SetScope(scope) => {
                self.scope = scope;
                self.chart_data = self.build_chart_data();
            }
            Message::Back => {
                self.scope = MapScope::World;
                self.chart_data = self.build_chart_data();
            }
            Message::SetIndicator(indicator) => {
                self.indicator = indicator;
                self.chart_data = self.build_chart_data();
            }
            Message::SetColorScale(scale) => {
                self.color_scale = scale;
                self.chart_data = self.build_chart_data();
            }
            Message::ChartAction(hyozu::Action::Clicked(hyozu::Target::Feature { id, .. })) => {
                // Drill into the region containing this country
                if let Some(scope) = scope_for_country(id.as_str(), &self.countries) {
                    if scope != self.scope {
                        self.scope = scope;
                        self.chart_data = self.build_chart_data();
                    }
                }
            }
            Message::ChartAction(_) => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        if self.loading {
            return center(text("Loading map data...").size(20))
                .width(Fill)
                .height(Fill)
                .into();
        }

        if self.countries.is_none() {
            return center(text("Failed to load map data.").size(20))
                .width(Fill)
                .height(Fill)
                .into();
        }

        let toolbar = self.view_toolbar();
        let region_bar = self.view_region_bar();
        let indicator = self.indicator;
        let chart = hyozu::chart(&self.chart_data)
            .height(Fill)
            .on_action(Message::ChartAction)
            .hover(move |entry| match entry {
                hyozu::hover::Entry::Choropleth {
                    feature_name, value, ..
                } => {
                    let name = text(feature_name.to_string()).size(14).font(Font {
                        weight: iced::font::Weight::Semibold,
                        ..Font::DEFAULT
                    });
                    let detail = match value {
                        Some(v) => text(format!(
                            "{}: {}",
                            indicator.short_label(),
                            indicator.format_hover_value(*v)
                        ))
                        .size(12),
                        None => text("no data".to_string()).size(12),
                    };
                    hyozu::hover::Annotation::new(column![name, detail].spacing(2))
                }
                _ => hyozu::hover::Annotation::new(text("")),
            });

        column![toolbar, region_bar, chart].width(Fill).height(Fill).into()
    }

    // -- Toolbar -------------------------------------------------------

    fn view_toolbar(&self) -> Element<'_, Message> {
        let back_btn: Element<'_, Message> = if self.scope != MapScope::World {
            button(icon::arrow_left().size(16))
                .padding([4, 8])
                .on_press(Message::Back)
                .style(button::text)
                .into()
        } else {
            // Invisible placeholder to keep layout stable
            container(text("")).width(28).into()
        };

        let title = text("World Factbook Explorer").size(16);

        let mut indicator_row = row![].spacing(4);
        for &ind in &Indicator::ALL {
            let active = self.indicator == ind;
            let label: &'static str = ind.short_label();
            indicator_row = indicator_row.push(
                button(text(label).size(11))
                    .padding([4, 10])
                    .on_press(Message::SetIndicator(ind))
                    .style(move |theme, status| tab_button_style(theme, status, active)),
            );
        }

        let scale_picker = pick_list(
            Some(self.color_scale.clone()),
            hyozu::palette::Scheme::ALL,
            hyozu::palette::Scheme::to_string,
        )
        .on_select(Message::SetColorScale)
        .text_size(11)
        .padding([4, 8]);

        let theme_icon = if self.dark { icon::sun() } else { icon::moon() };

        let theme_btn = button(theme_icon.size(16))
            .padding([4, 8])
            .on_press(Message::ToggleDark)
            .style(button::text);

        let left = row![back_btn, title].spacing(8).align_y(iced::Alignment::Center);
        let right = row![indicator_row, scale_picker, theme_btn]
            .spacing(12)
            .align_y(iced::Alignment::Center);

        let spacer = container(text("")).width(Fill);

        row![left, spacer, right]
            .padding([8, 16])
            .align_y(iced::Alignment::Center)
            .width(Fill)
            .into()
    }

    // -- Region bar ----------------------------------------------------

    fn view_region_bar(&self) -> Element<'_, Message> {
        let regions: [(&str, MapScope); 7] = [
            ("World", MapScope::World),
            ("Europe", MapScope::Europe),
            ("Asia", MapScope::Asia),
            ("Africa", MapScope::Africa),
            ("N. America", MapScope::NorthAmerica),
            ("S. America", MapScope::LatinAmerica),
            ("U.S. States", MapScope::UnitedStates),
        ];

        let mut buttons = row![].spacing(4);
        for &(label, scope) in &regions {
            let active = self.scope == scope;
            buttons = buttons.push(
                button(text(label).size(11))
                    .padding([4, 10])
                    .on_press(Message::SetScope(scope))
                    .style(move |theme, status| tab_button_style(theme, status, active)),
            );
        }

        container(buttons)
            .padding([6, 16])
            .width(Fill)
            .style(|theme: &Theme| {
                let palette = theme.palette();
                container::Style {
                    background: Some(palette.background.weak.color.into()),
                    ..Default::default()
                }
            })
            .into()
    }

    // -- Chart data ----------------------------------------------------

    fn build_chart_data(&self) -> hyozu::Data {
        let is_states = self.scope == MapScope::UnitedStates;
        let geo = if is_states {
            self.states.as_ref()
        } else {
            self.countries.as_ref()
        };
        let Some(geo) = geo else {
            return hyozu::Data::default();
        };

        let entries = if is_states {
            self.indicator.state_data()
        } else {
            self.indicator.data()
        };

        let choropleth = hyozu::choropleth(entries)
            .log()
            .scheme(self.color_scale.clone())
            .legend_title(self.indicator.label());

        hyozu::data(choropleth)
            .geo(geo.clone(), self.scope, ProjectionKind::NaturalEarth)
            .title(self.indicator.label())
    }
}

// ── Async fetch ────────────────────────────────────────────────────

async fn fetch_geo(url: &str) -> Result<GeoData, Box<dyn std::error::Error + Send + Sync>> {
    let body = reqwest::get(url).await?.text().await?;
    let data = geo::parse_geojson(&body)?;
    Ok(data)
}

// ── Helpers ────────────────────────────────────────────────────────

/// Maps a country ISO_A3 code to a drill-down scope based on its continent.
fn scope_for_country(id: &str, geo: &Option<Arc<GeoData>>) -> Option<MapScope> {
    let geo = geo.as_ref()?;
    let feature = geo.get(id)?;
    let continent = feature.properties.get("CONTINENT")?;
    Some(match continent.as_str() {
        "Europe" => MapScope::Europe,
        "Asia" => MapScope::Asia,
        "Africa" => MapScope::Africa,
        "North America" => MapScope::NorthAmerica,
        "South America" => MapScope::LatinAmerica,
        "Oceania" => MapScope::Asia, // Include with Asia view
        _ => MapScope::World,
    })
}

fn tab_button_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let palette = theme.palette();
    let muted = palette.background.base.text.scale_alpha(0.6);
    let bright = palette.background.base.text;
    let fill = palette.background.strong.color.scale_alpha(0.4);

    button::Style {
        background: if active {
            Some(fill.into())
        } else if matches!(status, button::Status::Hovered) {
            Some(fill.scale_alpha(0.5).into())
        } else {
            None
        },
        text_color: if active { bright } else { muted },
        border: iced::Border {
            width: if active { 1.0 } else { 0.0 },
            radius: 4.0.into(),
            color: palette.background.strong.color.scale_alpha(0.3),
        },
        ..Default::default()
    }
}

// ── Main ───────────────────────────────────────────────────────────

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([1200.0, 800.0])
        .title("World Factbook Explorer")
        .settings(iced::Settings {
            default_text_size: 13.into(),
            default_font: Font::MONOSPACE,
            ..Default::default()
        })
        .font(icon::FONT)
        .theme(|state: &App| state.theme.clone())
        .run()
}
