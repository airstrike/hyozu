//! Live BTC price streaming example using hyper + iced Task::sip
//!
//! Fetches BTC/USD price from Kraken API every N seconds and displays
//! a rolling window of the last M prices as a line chart.
//!
//! Run with: cargo run --package btc_stream

use std::time::Duration;

use http_body_util::{BodyExt, Empty};
use hyper::Request;
use hyper::body::Bytes;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::Deserialize;

use hyozu::{Data, chart, line};
use iced::task::{self, sipper};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Border, Element, Fill, Font, Shrink, Task, Theme};

const WINDOW_SIZE: usize = 60;
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// A price point with timestamp
#[derive(Debug, Clone)]
struct PricePoint {
    timestamp: i64,
    price: f32,
}

/// Trading pair to display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TradingPair {
    #[default]
    BtcUsd,
    EthUsd,
}

impl TradingPair {
    fn label(&self) -> &'static str {
        match self {
            TradingPair::BtcUsd => "BTC/USD",
            TradingPair::EthUsd => "ETH/USD",
        }
    }

    /// Kraken API pair parameter
    fn api_pair(&self) -> &'static str {
        match self {
            TradingPair::BtcUsd => "XBTUSD",
            TradingPair::EthUsd => "ETHUSD",
        }
    }

    /// Kraken API result key (they use different naming)
    fn result_key(&self) -> &'static str {
        match self {
            TradingPair::BtcUsd => "XXBTZUSD",
            TradingPair::EthUsd => "XETHZUSD",
        }
    }
}

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([700.0, 400.0])
        .title("hyozu • btc_stream")
        .settings(iced::Settings {
            default_text_size: 12.into(),
            default_font: Font::MONOSPACE,
            ..Default::default()
        })
        .theme(hyozu::theme::hyozu_dark())
        .run()
}

/// Kraken ticker response (result is a map of pair -> ticker)
#[derive(Debug, Clone, Deserialize)]
struct KrakenResponse {
    error: Vec<String>,
    result: Option<std::collections::HashMap<String, KrakenTicker>>,
}

#[derive(Debug, Clone, Deserialize)]
struct KrakenTicker {
    #[allow(dead_code)]
    a: Vec<String>, // a = ask [price, whole lot volume, lot volume]
    #[allow(dead_code)]
    b: Vec<String>, // b = bid [price, whole lot volume, lot volume]
    c: Vec<String>, // c = last trade closed [price, lot volume]
    t: Vec<u64>,    // t = number of trades [today, last 24 hours]
}

/// Ticker data for a single pair
#[derive(Debug, Clone)]
struct TickerData {
    timestamp: i64,
    price: f32,       // last trade price
    trade_count: u64, // number of trades today
}

struct App {
    points: Vec<PricePoint>,
    latest_ticker: Option<TickerData>,
    trading_pair: TradingPair,
    min_price: f32,
    max_price: f32,
    error: Option<String>,
    data: Data,
    sip_handle: Option<task::Handle>,
    loading_pair: Option<TradingPair>, // pair we're loading historical for
    last_trade_count: u64,             // to detect new trades
}

#[derive(Debug, Clone)]
enum Message {
    TickerReceived(TickerData),
    HistoricalPrices(TradingPair, Vec<PricePoint>),
    SetTradingPair(TradingPair),
    SipFinished(Result<(), String>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let pair = TradingPair::default();

        // Fetch historical prices at startup - sip starts after historical arrives
        let historical_task = Task::future(fetch_historical_prices(pair)).map(move |result| match result {
            Ok(points) => Message::HistoricalPrices(pair, points),
            Err(e) => {
                eprintln!("Failed to fetch historical: {}", e);
                Message::HistoricalPrices(pair, vec![])
            }
        });

        (
            Self {
                points: Vec::with_capacity(WINDOW_SIZE),
                latest_ticker: None,
                trading_pair: pair,
                min_price: f32::MAX,
                max_price: f32::MIN,
                error: None,
                data: Data::default(),
                sip_handle: None,
                loading_pair: Some(pair),
                last_trade_count: 0,
            },
            historical_task,
        )
    }

    fn current_price(&self) -> Option<f32> {
        self.latest_ticker.as_ref().map(|t| t.price)
    }

    fn start_sip(&mut self) -> Task<Message> {
        // Abort existing sip if any
        self.sip_handle = None;

        let pair = self.trading_pair;
        let (sip_task, handle) =
            Task::sip(fetch_ticker(pair), Message::TickerReceived, Message::SipFinished).abortable();

        self.sip_handle = Some(handle.abort_on_drop());
        sip_task
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TickerReceived(ticker) => {
                self.error = None;

                // Only add a point if new trades have occurred
                if ticker.trade_count > self.last_trade_count {
                    self.last_trade_count = ticker.trade_count;

                    // Track min/max for session
                    self.min_price = self.min_price.min(ticker.price);
                    self.max_price = self.max_price.max(ticker.price);

                    // Add to rolling window
                    self.points.push(PricePoint {
                        timestamp: ticker.timestamp,
                        price: ticker.price,
                    });
                    if self.points.len() > WINDOW_SIZE {
                        self.points.remove(0);
                    }
                    self.rebuild_chart();
                }

                self.latest_ticker = Some(ticker);
            }
            Message::HistoricalPrices(pair, points) => {
                // Only apply if this is for our current/loading pair
                if self.loading_pair == Some(pair) {
                    self.loading_pair = None;
                    self.last_trade_count = 0; // reset for new pair
                    self.points = points;
                    self.min_price = f32::MAX;
                    self.max_price = f32::MIN;
                    for p in &self.points {
                        self.min_price = self.min_price.min(p.price);
                        self.max_price = self.max_price.max(p.price);
                    }
                    self.rebuild_chart();

                    // Now start the live sip
                    return self.start_sip();
                }
            }
            Message::SetTradingPair(pair) => {
                if self.trading_pair != pair {
                    self.trading_pair = pair;
                    self.loading_pair = Some(pair);

                    // Stop current sip and clear data while loading
                    self.sip_handle = None;
                    self.points.clear();
                    self.latest_ticker = None;
                    self.data = Data::default();

                    // Fetch historical for new pair
                    return Task::future(fetch_historical_prices(pair)).map(move |result| match result {
                        Ok(points) => Message::HistoricalPrices(pair, points),
                        Err(e) => {
                            eprintln!("Failed to fetch historical: {}", e);
                            Message::HistoricalPrices(pair, vec![])
                        }
                    });
                }
            }
            Message::SipFinished(result) => {
                if let Err(err) = result {
                    self.error = Some(err);
                }
            }
        }
        Task::none()
    }

    fn rebuild_chart(&mut self) {
        // Create (timestamp, price) points for proper time-based x-axis
        let points: Vec<(f32, f32)> = self.points.iter().map(|p| (p.timestamp as f32, p.price)).collect();

        self.data =
            Data::from(line(points).data_labels(line::label::Show::LastOnly + line::label::Position::Right + currency))
                .x_axis(|_| line::Line::time_axis())
                .y_axis_labels(currency);
    }

    fn view(&self) -> Element<'_, Message> {
        // Segmented picker for trading pair
        let picker = {
            let pairs = [TradingPair::BtcUsd, TradingPair::EthUsd];
            let buttons = pairs.into_iter().map(|pair| {
                let selected = self.trading_pair == pair;
                button(text(pair.label()).size(11))
                    .padding([4, 8])
                    .style(move |theme, status| segment_button(theme, status, selected))
                    .on_press(Message::SetTradingPair(pair))
                    .into()
            });
            container(row(buttons))
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        background: Some(palette.background.weak.color.scale_alpha(0.3).into()),
                        ..Default::default()
                    }
                })
                .padding(2)
        };

        // Status info (pair name and change %)
        let status = match (&self.current_price(), &self.error) {
            (Some(price), None) => {
                let change = if self.min_price < f32::MAX {
                    let mid = (self.min_price + self.max_price) / 2.0;
                    let pct = (price - mid) / mid * 100.0;
                    if pct >= 0.0 {
                        format!("+{:.3}%", pct)
                    } else {
                        format!("{:.3}%", pct)
                    }
                } else {
                    String::new()
                };
                row![text(self.trading_pair.label()), text(change)].spacing(20)
            }
            (_, Some(err)) => row![text!("error: {}", err)],
            (None, None) => row![text("connecting...")],
        };

        let header = row![container(status).width(Fill), Space::new().height(Shrink), picker]
            .spacing(20)
            .align_y(iced::Alignment::Center);

        let content = if self.loading_pair.is_some() {
            column![header, text("loading data...")].spacing(10)
        } else if self.points.len() >= 2 {
            column![
                header,
                chart(&self.data)
                    .padding(iced::padding::right(50))
                    .style(hyozu::chart::transparent)
            ]
            .spacing(20)
        } else {
            column![header, text("waiting for data...")].spacing(10)
        };

        container(content).padding(15).into()
    }
}

/// Custom button style for segment picker
fn segment_button(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = theme.extended_palette();
    let muted = palette.background.base.text.scale_alpha(0.5);
    let less_muted = palette.background.base.text.scale_alpha(0.8);
    let fill = palette.background.weak.color.scale_alpha(0.5);

    button::Style {
        background: if selected || matches!(status, button::Status::Hovered) {
            Some(fill.into())
        } else {
            None
        },
        text_color: if selected { less_muted } else { muted },
        border: Border::default(),
        ..Default::default()
    }
}

/// Creates a sipper that continuously fetches ticker for a trading pair
fn fetch_ticker(pair: TradingPair) -> impl task::Straw<(), TickerData, String> {
    sipper(async move |mut progress| {
        // Build HTTPS client
        let https = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_only()
            .enable_http1()
            .build();

        let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(https);

        loop {
            match fetch_single_ticker(&client, pair).await {
                Ok(ticker) => {
                    let _ = progress.send(ticker).await;
                }
                Err(e) => {
                    // Log error but continue polling
                    eprintln!("Fetch error: {}", e);
                }
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
}

async fn fetch_single_ticker<C>(client: &Client<C, Empty<Bytes>>, pair: TradingPair) -> Result<TickerData, String>
where
    C: hyper_util::client::legacy::connect::Connect + Clone + Send + Sync + 'static,
{
    let url = format!("https://api.kraken.com/0/public/Ticker?pair={}", pair.api_pair());
    let uri: hyper::Uri = url.parse().map_err(|e| format!("Invalid URI: {}", e))?;

    let req = Request::builder()
        .method("GET")
        .uri(&uri)
        .header("Accept", "application/json")
        .body(Empty::<Bytes>::new())
        .map_err(|e| format!("Request build error: {}", e))?;

    let res = client
        .request(req)
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let body = res
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Body read error: {}", e))?
        .to_bytes();

    let kraken: KrakenResponse = serde_json::from_slice(&body).map_err(|e| {
        let body_str = String::from_utf8_lossy(&body);
        format!("JSON parse error: {} - body: {}", e, body_str)
    })?;

    if !kraken.error.is_empty() {
        return Err(format!("Kraken error: {:?}", kraken.error));
    }

    let result = kraken.result.ok_or("No result in response")?;
    let ticker = result
        .get(pair.result_key())
        .ok_or_else(|| format!("No data for {}", pair.result_key()))?;

    let price = ticker
        .c
        .first()
        .ok_or("No last")?
        .parse::<f32>()
        .map_err(|e| format!("Price parse error: {}", e))?;

    let trade_count = *ticker.t.first().ok_or("No trade count")?;

    // Current unix timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    Ok(TickerData {
        timestamp,
        price,
        trade_count,
    })
}

/// Fetch historical OHLC data from Kraken
async fn fetch_historical_prices(pair: TradingPair) -> Result<Vec<PricePoint>, String> {
    let https = HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_only()
        .enable_http1()
        .build();

    let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(https);

    // OHLC endpoint with 1-minute interval
    let url = format!(
        "https://api.kraken.com/0/public/OHLC?pair={}&interval=1",
        pair.api_pair()
    );
    let uri: hyper::Uri = url.parse().map_err(|e| format!("Invalid URI: {}", e))?;

    let req = Request::builder()
        .method("GET")
        .uri(&uri)
        .header("Accept", "application/json")
        .body(Empty::<Bytes>::new())
        .map_err(|e| format!("Request build error: {}", e))?;

    let res = client
        .request(req)
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let body = res
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Body read error: {}", e))?
        .to_bytes();

    // OHLC response format: {"error":[],"result":{"XXBTZUSD":[[time,open,high,low,close,vwap,volume,count],...]}}
    let json: serde_json::Value = serde_json::from_slice(&body).map_err(|e| format!("JSON parse error: {}", e))?;

    if let Some(errors) = json.get("error").and_then(|e| e.as_array())
        && !errors.is_empty()
    {
        return Err(format!("Kraken error: {:?}", errors));
    }

    let candles = json
        .get("result")
        .and_then(|r| r.get(pair.result_key()))
        .and_then(|d| d.as_array())
        .ok_or_else(|| format!("No OHLC data for {}", pair.result_key()))?;

    // Extract timestamp and close prices from the last WINDOW_SIZE candles
    let points: Vec<PricePoint> = candles
        .iter()
        .rev()
        .take(WINDOW_SIZE)
        .filter_map(|candle| {
            let arr = candle.as_array()?;
            let timestamp = arr.first()?.as_i64()?;
            let price = arr.get(4)?.as_str()?.parse::<f32>().ok()?;
            Some(PricePoint { timestamp, price })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(points)
}

// Helper function to format numbers with thousands separator
fn currency(value: f64) -> String {
    let whole = value as i32;
    let s = whole.to_string();
    let mut result = String::new();
    let chars: Vec<_> = s.chars().collect();

    for (i, ch) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, *ch);
    }

    format!("${}", result)
}
